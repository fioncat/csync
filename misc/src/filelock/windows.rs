use std::path::PathBuf;

use anyhow::Result;

pub struct FileLock {}

impl FileLock {
    pub fn try_acquire(_path: PathBuf) -> Result<Option<Self>> {
        Ok(Some(Self {}))
    }

    pub fn acquire(path: PathBuf) -> Result<Self> {
        Ok(Self {})
    }
}
