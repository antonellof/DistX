// Snapshot support for persistence
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
// Snapshot support for persistence

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub timestamp: u64,
    pub collections: Vec<String>,
}

pub struct SnapshotManager {
    #[allow(dead_code)]
    snapshot_dir: PathBuf,
}

impl SnapshotManager {
    pub fn new<P: AsRef<Path>>(snapshot_dir: P) -> Result<Self> {
        let snapshot_dir = snapshot_dir.as_ref().to_path_buf();
        fs::create_dir_all(&snapshot_dir)?;
        Ok(Self { snapshot_dir })
    }

    pub fn create_snapshot(&self, collections: &[String]) -> Result<PathBuf> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        let snapshot_path = self.snapshot_dir.join(format!("snapshot_{}.json", timestamp));
        
        let metadata = SnapshotMetadata {
            timestamp,
            collections: collections.to_vec(),
        };
        
        let data = serde_json::to_string_pretty(&metadata)?;
        fs::write(&snapshot_path, data)?;
        
        Ok(snapshot_path)
    }

    pub fn list_snapshots(&self) -> Result<Vec<PathBuf>> {
        let mut snapshots = Vec::new();
        for entry in fs::read_dir(&self.snapshot_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                snapshots.push(path);
            }
        }
        snapshots.sort();
        Ok(snapshots)
    }

    pub fn load_snapshot(&self, path: &Path) -> Result<SnapshotMetadata> {
        let data = fs::read_to_string(path)?;
        let metadata: SnapshotMetadata = serde_json::from_str(&data)?;
        Ok(metadata)
    }
}

