use distx_core::{Collection, CollectionConfig, Distance, Error, Result, Point, PointId, Vector};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use crate::lmdb_storage::LmdbStorage;
use crate::wal::WriteAheadLog;
use crate::snapshot::SnapshotManager;
use crate::persistence::ForkBasedPersistence;

/// Manages collections and persistence
pub struct StorageManager {
    collections: Arc<RwLock<HashMap<String, Arc<Collection>>>>,
    #[allow(dead_code)]
    data_dir: PathBuf,
    #[allow(dead_code)]
    lmdb: Option<Arc<LmdbStorage>>,
    #[allow(dead_code)]
    wal: Option<Arc<WriteAheadLog>>,
    #[allow(dead_code)]
    snapshots: Option<Arc<SnapshotManager>>,
    persistence: Arc<ForkBasedPersistence>,
    #[allow(dead_code)]
    save_interval: Option<Duration>,
}

impl StorageManager {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;

        let lmdb_path = data_dir.join("lmdb");
        let lmdb = Arc::new(LmdbStorage::new(&lmdb_path)
            .map_err(|e| Error::Storage(e.to_string()))?);

        let wal_path = data_dir.join("wal.log");
        let wal = Arc::new(WriteAheadLog::new(&wal_path)
            .map_err(|e| Error::Storage(e.to_string()))?);

        let snapshot_dir = data_dir.join("snapshots");
        let snapshots = Arc::new(SnapshotManager::new(&snapshot_dir)
            .map_err(|e| Error::Storage(e.to_string()))?);

        let persistence = Arc::new(ForkBasedPersistence::new(&data_dir));

        let collections = Arc::new(RwLock::new(HashMap::new()));
        
        if let Some(snapshot) = persistence.load_snapshot()
            .map_err(|e| Error::Persistence(e.to_string()))? {
            eprintln!("Loading snapshot from disk...");
            let mut collections_map = HashMap::new();
            
            for col_snapshot in snapshot.collections {
                let config = CollectionConfig {
                    name: col_snapshot.name.clone(),
                    vector_dim: col_snapshot.config.vector_dim,
                    distance: match col_snapshot.config.distance.as_str() {
                        "Cosine" => Distance::Cosine,
                        "Euclidean" => Distance::Euclidean,
                        "Dot" => Distance::Dot,
                        _ => Distance::Cosine,
                    },
                    use_hnsw: col_snapshot.config.use_hnsw,
                    enable_bm25: col_snapshot.config.enable_bm25,
                };
                
                let collection = Arc::new(Collection::new(config));
                
                for point_snapshot in col_snapshot.points {
                    let point = Point::new(
                        PointId::String(point_snapshot.id.clone()),
                        Vector::new(point_snapshot.vector),
                        point_snapshot.payload,
                    );
                    if let Err(e) = collection.upsert(point) {
                        eprintln!("Warning: Failed to restore point {}: {}", point_snapshot.id, e);
                    }
                }
                
                collections_map.insert(col_snapshot.name, collection);
            }
            
            *collections.write() = collections_map;
            eprintln!("Snapshot loaded: {} collections", collections.read().len());
        }

        let manager = Self {
            collections,
            data_dir,
            lmdb: Some(lmdb),
            wal: Some(wal),
            snapshots: Some(snapshots),
            persistence,
            save_interval: Some(Duration::from_secs(300)),
        };

        manager.start_background_save();

        Ok(manager)
    }

    /// Start background save thread
    fn start_background_save(&self) {
        let collections = self.collections.clone();
        let persistence = self.persistence.clone();
        let interval = self.save_interval.unwrap_or(Duration::from_secs(300));

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(interval);
                
                if !ForkBasedPersistence::is_bgsave_in_progress() {
                    let collections_map = collections.read();
                    if let Err(e) = persistence.bgsave(&collections_map) {
                        eprintln!("Background save error: {}", e);
                    }
                }
            }
        });
    }

    pub fn create_collection(&self, config: CollectionConfig) -> Result<Arc<Collection>> {
        let name = config.name.clone();
        let mut collections = self.collections.write();

        if collections.contains_key(&name) {
            return Err(Error::CollectionExists(name));
        }

        let collection = Arc::new(Collection::new(config));
        collections.insert(name.clone(), collection.clone());
        Ok(collection)
    }

    pub fn get_collection(&self, name: &str) -> Option<Arc<Collection>> {
        self.collections.read().get(name).cloned()
    }

    pub fn delete_collection(&self, name: &str) -> Result<bool> {
        let mut collections = self.collections.write();
        Ok(collections.remove(name).is_some())
    }

    pub fn list_collections(&self) -> Vec<String> {
        self.collections.read().keys().cloned().collect()
    }

    pub fn collection_exists(&self, name: &str) -> bool {
        self.collections.read().contains_key(name)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Trigger background save
    pub fn bgsave(&self) -> Result<bool> {
        let collections = self.collections.read();
        self.persistence.bgsave(&collections)
            .map_err(|e| Error::Storage(e.to_string()))
    }

    /// Force save
    pub fn save(&self) -> Result<()> {
        let collections = self.collections.read();
        self.persistence.save(&collections)
            .map_err(|e| Error::Storage(e.to_string()))
    }

    /// Get last save time
    pub fn last_save_time(&self) -> u64 {
        ForkBasedPersistence::last_save_time()
    }

    /// Check if background save is in progress
    pub fn is_bgsave_in_progress(&self) -> bool {
        ForkBasedPersistence::is_bgsave_in_progress()
    }
}

