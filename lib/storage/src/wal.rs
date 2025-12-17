use anyhow::Result;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::Mutex;

/// Write-Ahead Log for durability
pub struct WriteAheadLog {
    file: Arc<Mutex<BufWriter<File>>>,
    path: PathBuf,
}

impl WriteAheadLog {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        Ok(Self {
            file: Arc::new(Mutex::new(BufWriter::new(file))),
            path,
        })
    }

    pub fn append(&self, data: &[u8]) -> Result<()> {
        let mut writer = self.file.lock();
        writer.write_all(data)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }

    pub fn sync(&self) -> Result<()> {
        let mut writer = self.file.lock();
        writer.flush()?;
        Ok(())
    }
}

