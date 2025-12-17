use anyhow::Result;
use nix::unistd::{fork, ForkResult};
use nix::sys::wait::waitpid;
use nix::sys::wait::WaitStatus;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

static BGSAVE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static LAST_SAVE_TIME: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotData {
    pub collections: Vec<CollectionSnapshot>,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionSnapshot {
    pub name: String,
    pub config: CollectionConfigSnapshot,
    pub points: Vec<PointSnapshot>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionConfigSnapshot {
    pub vector_dim: usize,
    pub distance: String,
    pub use_hnsw: bool,
    pub enable_bm25: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PointSnapshot {
    pub id: String,
    pub vector: Vec<f32>,
    pub payload: Option<serde_json::Value>,
}

/// Fork-based background save
pub struct ForkBasedPersistence {
    data_dir: PathBuf,
    rdb_filename: PathBuf,
}

impl ForkBasedPersistence {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref().to_path_buf();
        let rdb_filename = data_dir.join("dump.rdb");
        
        Self {
            data_dir,
            rdb_filename,
        }
    }

    /// Start background save (bgsave) - forks a child process
    pub fn bgsave(&self, collections: &std::collections::HashMap<String, Arc<distx_core::Collection>>) -> Result<bool> {
        // Check if already in progress
        if BGSAVE_IN_PROGRESS.swap(true, Ordering::Acquire) {
            return Ok(false); // Already in progress
        }

        // Fork the process
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child, .. }) => {
                // Parent process - continue serving requests
                // The child will handle the snapshot
                eprintln!("Background save started by pid {}", child);
                
                // Spawn a thread to wait for child and reset flag
                std::thread::spawn(move || {
                    match waitpid(child, None) {
                        Ok(WaitStatus::Exited(_, code)) => {
                            if code == 0 {
                                eprintln!("Background save completed successfully");
                                LAST_SAVE_TIME.store(
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                    Ordering::Release,
                                );
                            } else {
                                eprintln!("Background save failed with exit code {}", code);
                            }
                        }
                        Ok(status) => {
                            eprintln!("Background save child process: {:?}", status);
                        }
                        Err(e) => {
                            eprintln!("Error waiting for background save: {}", e);
                        }
                    }
                    BGSAVE_IN_PROGRESS.store(false, Ordering::Release);
                });
                
                Ok(true)
            }
            Ok(ForkResult::Child) => {
                // Child process - perform the snapshot
                // Set process title (if possible)
                eprintln!("Child process: Starting snapshot...");
                
                // Create snapshot data
                let snapshot = self.create_snapshot(collections)?;
                
                // Write to temporary file first (atomic write)
                let temp_file = self.rdb_filename.with_extension("tmp");
                let data = bincode::serialize(&snapshot)
                    .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?;
                std::fs::write(&temp_file, &data)?;
                
                // Atomic rename
                std::fs::rename(&temp_file, &self.rdb_filename)?;
                
                eprintln!("Child process: Snapshot saved to {:?}", self.rdb_filename);
                
                // Exit child process
                process::exit(0);
            }
            Err(e) => {
                BGSAVE_IN_PROGRESS.store(false, Ordering::Release);
                Err(anyhow::anyhow!("Failed to fork: {}", e))
            }
        }
    }

    /// Create snapshot from collections (called in child process)
    fn create_snapshot(
        &self,
        collections: &std::collections::HashMap<String, Arc<distx_core::Collection>>,
    ) -> Result<SnapshotData> {
        let mut collection_snapshots = Vec::new();

        for (name, collection) in collections {
            let mut points = Vec::new();
            
            // Iterate through all points
            for point in collection.iter() {
                points.push(PointSnapshot {
                    id: point.id.to_string(),
                    vector: point.vector.as_slice().to_vec(),
                    payload: point.payload.clone(),
                });
            }

            collection_snapshots.push(CollectionSnapshot {
                name: name.clone(),
                config: CollectionConfigSnapshot {
                    vector_dim: collection.vector_dim(),
                    distance: format!("{:?}", collection.distance()),
                    use_hnsw: true, // TODO: get from config
                    enable_bm25: false, // TODO: get from config
                },
                points,
            });
        }

        Ok(SnapshotData {
            collections: collection_snapshots,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        })
    }

    /// Load snapshot from disk (on startup)
    pub fn load_snapshot(&self) -> Result<Option<SnapshotData>> {
        if !self.rdb_filename.exists() {
            return Ok(None);
        }

        let data = std::fs::read(&self.rdb_filename)?;
        let snapshot: SnapshotData = bincode::deserialize(&data)
            .map_err(|e| anyhow::anyhow!("Deserialization error: {}", e))?;
        Ok(Some(snapshot))
    }

    /// Check if background save is in progress
    pub fn is_bgsave_in_progress() -> bool {
        BGSAVE_IN_PROGRESS.load(Ordering::Acquire)
    }

    /// Get last save time
    pub fn last_save_time() -> u64 {
        LAST_SAVE_TIME.load(Ordering::Acquire)
    }

    /// Force save (synchronous, blocks until complete)
    pub fn save(&self, collections: &std::collections::HashMap<String, Arc<distx_core::Collection>>) -> Result<()> {
        let snapshot = self.create_snapshot(collections)?;
        let temp_file = self.rdb_filename.with_extension("tmp");
        let data = bincode::serialize(&snapshot)
            .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?;
        std::fs::write(&temp_file, &data)?;
        std::fs::rename(&temp_file, &self.rdb_filename)?;
        Ok(())
    }
}

