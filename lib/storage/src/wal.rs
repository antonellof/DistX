use anyhow::Result;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::Mutex;

/// Write-Ahead Log for durability
/// Inspired by Redis AOF (Append Only File) patterns
pub struct WriteAheadLog {
    file: Arc<Mutex<BufWriter<File>>>,
    raw_file: Arc<Mutex<File>>,  // For fsync operations
    #[allow(dead_code)]
    path: PathBuf,  // Stored for potential future use (e.g., log rotation)
}

impl WriteAheadLog {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        
        // Clone the file for fsync operations (Redis pattern)
        let raw_file = file.try_clone()?;

        Ok(Self {
            file: Arc::new(Mutex::new(BufWriter::new(file))),
            raw_file: Arc::new(Mutex::new(raw_file)),
            path,
        })
    }

    /// Append data to the WAL
    #[inline]
    pub fn append(&self, data: &[u8]) -> Result<()> {
        let mut writer = self.file.lock();
        writer.write_all(data)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }

    /// Sync WAL to disk (like Redis fsync)
    /// Uses sync_data() which is equivalent to fdatasync on Unix
    #[inline]
    pub fn sync(&self) -> Result<()> {
        let mut writer = self.file.lock();
        writer.flush()?;
        
        // Ensure data is persisted to disk (Redis-style durability)
        let raw = self.raw_file.lock();
        raw.sync_data()?;  // fdatasync equivalent - faster than sync_all()
        Ok(())
    }

    /// Full sync including metadata (for critical operations)
    #[inline]
    pub fn sync_all(&self) -> Result<()> {
        let mut writer = self.file.lock();
        writer.flush()?;
        
        let raw = self.raw_file.lock();
        raw.sync_all()?;
        Ok(())
    }
}

