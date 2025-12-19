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
    #[allow(dead_code)]
    data_dir: PathBuf,  // Stored for potential future use
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
    /// Handles corruption gracefully:
    /// - Validates data integrity
    /// - Backs up corrupt files
    /// - Returns None instead of crashing
    /// - Logs detailed warnings
    pub fn load_snapshot(&self) -> Result<Option<SnapshotData>> {
        if !self.rdb_filename.exists() {
            eprintln!("[DistX] No snapshot file found, starting with empty database");
            return Ok(None);
        }
        
        // Check for version/marker file (indicates complete save)
        let version_file = self.rdb_filename.with_extension("version");
        if self.rdb_filename.exists() && !version_file.exists() {
            // Snapshot exists but no version file - incomplete save (crash recovery)
            eprintln!("[DistX] Warning: Snapshot file exists but version marker missing.");
            eprintln!("[DistX] This indicates an incomplete save. Starting fresh.");
            self.backup_and_remove_corrupt_file("incomplete");
            return Ok(None);
        }

        // Read the file
        let data = match std::fs::read(&self.rdb_filename) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[DistX] Warning: Could not read snapshot file: {}", e);
                eprintln!("[DistX] Starting with empty database.");
                return Ok(None);
            }
        };
        
        // Check minimum size (basic integrity check like Redis)
        if data.len() < 16 {
            eprintln!("[DistX] Warning: Snapshot file too small ({} bytes), likely corrupt", data.len());
            self.backup_and_remove_corrupt_file("too_small");
            return Ok(None);
        }
        
        // Deserialize with error handling (Redis: skip corrupt entries where possible)
        match bincode::deserialize(&data) {
            Ok(snapshot) => {
                eprintln!("[DistX] Successfully loaded snapshot ({} bytes)", data.len());
                Ok(Some(snapshot))
            }
            Err(e) => {
                // Data is corrupted - backup and start fresh
                eprintln!("[DistX] Warning: Snapshot data is corrupted: {}", e);
                eprintln!("[DistX] Starting with empty database.");
                self.backup_and_remove_corrupt_file("corrupt");
                Ok(None)
            }
        }
    }
    
    /// Backup corrupt file and remove original (Redis pattern: preserve for debugging)
    fn backup_and_remove_corrupt_file(&self, reason: &str) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        let backup_name = format!("dump.{}.{}.bak", reason, timestamp);
        let backup_path = self.rdb_filename.with_file_name(backup_name);
        
        if let Err(e) = std::fs::rename(&self.rdb_filename, &backup_path) {
            eprintln!("[DistX] Could not backup corrupt file: {}", e);
            // Try to delete it instead
            if let Err(del_err) = std::fs::remove_file(&self.rdb_filename) {
                eprintln!("[DistX] Could not delete corrupt file: {}", del_err);
            }
        } else {
            eprintln!("[DistX] Corrupt snapshot backed up to: {:?}", backup_path);
        }
        
        // Also remove version file if it exists
        let version_file = self.rdb_filename.with_extension("version");
        let _ = std::fs::remove_file(&version_file);
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
    /// Uses atomic rename pattern and version markers for data integrity
    pub fn save(&self, collections: &std::collections::HashMap<String, Arc<distx_core::Collection>>) -> Result<()> {
        let snapshot = self.create_snapshot(collections)?;
        let temp_file = self.rdb_filename.with_extension("tmp");
        let version_file = self.rdb_filename.with_extension("version");
        
        // Serialize data
        let data = bincode::serialize(&snapshot)
            .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?;
        
        // Write to temp file first (atomic write pattern from Redis)
        std::fs::write(&temp_file, &data)?;
        
        // Atomic rename (Redis pattern - prevents partial writes)
        std::fs::rename(&temp_file, &self.rdb_filename)?;
        
        // Write version marker (indicates complete save)
        let version_data = format!("distx:0.1.0:{}", data.len());
        std::fs::write(&version_file, version_data)?;
        
        eprintln!("[DistX] Snapshot saved ({} bytes)", data.len());
        Ok(())
    }
}

