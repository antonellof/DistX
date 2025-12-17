// LMDB-based storage for fast persistence (like helix-db)
use anyhow::Result;
use heed::{Database, Env, EnvOpenOptions};
use std::path::Path;
use std::sync::Arc;

const DB_COLLECTIONS: &str = "collections";
const DB_POINTS: &str = "points";
const DB_GRAPH_NODES: &str = "graph_nodes";
const DB_GRAPH_EDGES: &str = "graph_edges";

pub struct LmdbStorage {
    env: Arc<Env>,
    collections_db: Database<heed::types::Str, heed::types::Bytes>,
    points_db: Database<heed::types::Str, heed::types::Bytes>,
    nodes_db: Database<heed::types::U64<heed::byteorder::BE>, heed::types::Bytes>,
    edges_db: Database<heed::types::U64<heed::byteorder::BE>, heed::types::Bytes>,
}

impl LmdbStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        std::fs::create_dir_all(&path)?;

        let env = Arc::new(
            unsafe {
                EnvOpenOptions::new()
                    .map_size(100 * 1024 * 1024 * 1024) // 100GB default
                    .max_dbs(10)
                    .open(path)?
            }
        );

        let mut wtxn = env.write_txn()?;

        let collections_db = env
            .create_database(&mut wtxn, Some(DB_COLLECTIONS))?;

        let points_db = env
            .create_database(&mut wtxn, Some(DB_POINTS))?;

        let nodes_db = env
            .create_database(&mut wtxn, Some(DB_GRAPH_NODES))?;

        let edges_db = env
            .create_database(&mut wtxn, Some(DB_GRAPH_EDGES))?;

        wtxn.commit()?;

        Ok(Self {
            env,
            collections_db,
            points_db,
            nodes_db,
            edges_db,
        })
    }

    pub fn save_point(&self, collection: &str, point_id: &str, data: &[u8]) -> Result<()> {
        let mut wtxn = self.env.write_txn()?;
        let key = format!("{}:{}", collection, point_id);
        self.points_db.put(&mut wtxn, &key, data)?;
        wtxn.commit()?;
        Ok(())
    }

    pub fn get_point(&self, collection: &str, point_id: &str) -> Result<Option<Vec<u8>>> {
        let rtxn = self.env.read_txn()?;
        let key = format!("{}:{}", collection, point_id);
        match self.points_db.get(&rtxn, &key)? {
            Some(data) => Ok(Some(data.to_vec())),
            None => Ok(None),
        }
    }

    pub fn delete_point(&self, collection: &str, point_id: &str) -> Result<bool> {
        let mut wtxn = self.env.write_txn()?;
        let key = format!("{}:{}", collection, point_id);
        let existed = self.points_db.delete(&mut wtxn, &key)?;
        wtxn.commit()?;
        Ok(existed)
    }

    pub fn save_collection(&self, name: &str, data: &[u8]) -> Result<()> {
        let mut wtxn = self.env.write_txn()?;
        self.collections_db.put(&mut wtxn, name, data)?;
        wtxn.commit()?;
        Ok(())
    }

    pub fn get_collection(&self, name: &str) -> Result<Option<Vec<u8>>> {
        let rtxn = self.env.read_txn()?;
        match self.collections_db.get(&rtxn, name)? {
            Some(data) => Ok(Some(data.to_vec())),
            None => Ok(None),
        }
    }

    pub fn list_collections(&self) -> Result<Vec<String>> {
        let rtxn = self.env.read_txn()?;
        let mut collections = Vec::new();
        for result in self.collections_db.iter(&rtxn)? {
            let (key, _) = result?;
            collections.push(key.to_string());
        }
        Ok(collections)
    }

    pub fn save_node(&self, node_id: u64, data: &[u8]) -> Result<()> {
        let mut wtxn = self.env.write_txn()?;
        self.nodes_db.put(&mut wtxn, &node_id, data)?;
        wtxn.commit()?;
        Ok(())
    }

    pub fn get_node(&self, node_id: u64) -> Result<Option<Vec<u8>>> {
        let rtxn = self.env.read_txn()?;
        match self.nodes_db.get(&rtxn, &node_id)? {
            Some(data) => Ok(Some(data.to_vec())),
            None => Ok(None),
        }
    }

    pub fn save_edge(&self, edge_id: u64, data: &[u8]) -> Result<()> {
        let mut wtxn = self.env.write_txn()?;
        self.edges_db.put(&mut wtxn, &edge_id, data)?;
        wtxn.commit()?;
        Ok(())
    }

    pub fn get_edge(&self, edge_id: u64) -> Result<Option<Vec<u8>>> {
        let rtxn = self.env.read_txn()?;
        match self.edges_db.get(&rtxn, &edge_id)? {
            Some(data) => Ok(Some(data.to_vec())),
            None => Ok(None),
        }
    }
}

