// Background I/O system inspired by Redis's BIO (Background I/O)
// Implements job queues with worker threads for async operations

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Background job types (inspired by Redis BIO)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundJobType {
    HnswRebuild = 0,  // HNSW index rebuild
    LazyFree = 1,     // Lazy memory freeing
}

/// Background job trait
pub trait BackgroundJob: Send + 'static {
    fn execute(self: Box<Self>);
    fn job_type(&self) -> BackgroundJobType;
}

/// Background worker thread
struct BackgroundWorker {
    #[allow(dead_code)]
    worker_id: usize,
    #[allow(dead_code)]
    job_type: BackgroundJobType,
    jobs: Arc<Mutex<VecDeque<Box<dyn BackgroundJob>>>>,
    condvar: Arc<Condvar>,
    running: Arc<AtomicBool>,
}

impl BackgroundWorker {
    fn new(worker_id: usize, job_type: BackgroundJobType) -> Self {
        Self {
            worker_id,
            job_type,
            jobs: Arc::new(Mutex::new(VecDeque::new())),
            condvar: Arc::new(Condvar::new()),
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    fn submit(&self, job: Box<dyn BackgroundJob>) {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.push_back(job);
        self.condvar.notify_one();
    }

    fn pending_jobs(&self) -> usize {
        self.jobs.lock().unwrap().len()
    }

    fn shutdown(&self) {
        self.running.store(false, Ordering::Release);
        self.condvar.notify_all();
    }
}

/// Background job system (Redis-style BIO)
pub struct BackgroundJobSystem {
    workers: Vec<Arc<BackgroundWorker>>,
    job_counters: Arc<[AtomicU64; 2]>, // One counter per job type
}

impl BackgroundJobSystem {
    /// Create a new background job system
    pub fn new() -> Self {
        let mut workers = Vec::new();
        let mut handles = Vec::new();

        // Create workers for each job type
        for job_type in [BackgroundJobType::HnswRebuild, BackgroundJobType::LazyFree] {
            let worker = BackgroundWorker::new(0, job_type);
            let worker_arc = Arc::new(worker);
            let worker_for_thread = worker_arc.clone();
            let handle = thread::Builder::new()
                .name(format!("bg-worker-{:?}-0", job_type))
                .spawn(move || {
                    let jobs = worker_for_thread.jobs.clone();
                    let condvar = worker_for_thread.condvar.clone();
                    let running = worker_for_thread.running.clone();
                    
                    loop {
                        let mut jobs_guard = jobs.lock().unwrap();
                        
                        // Wait for jobs or shutdown signal
                        while jobs_guard.is_empty() && running.load(Ordering::Acquire) {
                            jobs_guard = condvar.wait(jobs_guard).unwrap();
                        }

                        // Check if we should shutdown
                        if !running.load(Ordering::Acquire) && jobs_guard.is_empty() {
                            break;
                        }

                        // Process jobs in FIFO order
                        while let Some(job) = jobs_guard.pop_front() {
                            drop(jobs_guard); // Release lock before executing
                            job.execute();
                            jobs_guard = jobs.lock().unwrap();
                        }
                    }
                })
                .expect("Failed to spawn background worker thread");
            
            handles.push(handle);
            workers.push(worker_arc);
        }

        // Don't wait for handles - let them run in background
        std::mem::forget(handles);

        Self {
            workers,
            job_counters: Arc::new([
                AtomicU64::new(0), // HnswRebuild
                AtomicU64::new(0), // LazyFree
            ]),
        }
    }

    /// Submit a background job
    pub fn submit(&self, job: Box<dyn BackgroundJob>) {
        let job_type = job.job_type();
        let worker = &self.workers[job_type as usize];
        
        self.job_counters[job_type as usize].fetch_add(1, Ordering::Relaxed);
        worker.submit(job);
    }

    /// Get pending jobs count for a job type
    pub fn pending_jobs(&self, job_type: BackgroundJobType) -> usize {
        self.workers[job_type as usize].pending_jobs()
    }

    /// Get total jobs processed for a job type
    pub fn jobs_processed(&self, job_type: BackgroundJobType) -> u64 {
        self.job_counters[job_type as usize].load(Ordering::Relaxed)
    }

    /// Shutdown all workers
    pub fn shutdown(&self) {
        for worker in &self.workers {
            worker.shutdown();
        }
    }
}

impl Default for BackgroundJobSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// HNSW rebuild job
pub struct HnswRebuildJob {
    points: Vec<crate::Point>,
    hnsw: Arc<parking_lot::RwLock<crate::HnswIndex>>,
    built_flag: Arc<parking_lot::RwLock<bool>>,
    rebuilding_flag: Arc<AtomicBool>,
}

impl HnswRebuildJob {
    pub fn new(
        points: Vec<crate::Point>,
        hnsw: Arc<parking_lot::RwLock<crate::HnswIndex>>,
        built_flag: Arc<parking_lot::RwLock<bool>>,
        rebuilding_flag: Arc<AtomicBool>,
    ) -> Self {
        Self {
            points,
            hnsw,
            built_flag,
            rebuilding_flag,
        }
    }
}

impl BackgroundJob for HnswRebuildJob {
    fn execute(self: Box<Self>) {
        // Rebuild HNSW index from all points
        let mut new_index = crate::HnswIndex::new(16, 3);
        for point in self.points {
            new_index.insert(point);
        }

        // Swap in the new index
        *self.hnsw.write() = new_index;
        *self.built_flag.write() = true;
        self.rebuilding_flag.store(false, Ordering::Release);
    }

    fn job_type(&self) -> BackgroundJobType {
        BackgroundJobType::HnswRebuild
    }
}

/// Global background job system (initialized on first use)
static BACKGROUND_SYSTEM: std::sync::OnceLock<Arc<BackgroundJobSystem>> = std::sync::OnceLock::new();

/// Get the global background job system
pub fn get_background_system() -> Arc<BackgroundJobSystem> {
    BACKGROUND_SYSTEM.get_or_init(|| {
        Arc::new(BackgroundJobSystem::new())
    }).clone()
}

