use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};

use crate::dirs;
use crate::server::store::cache::Cache;
use crate::server::store::sqlite::SqliteStore;

use super::config::StoreConfig;
use super::Storage;

#[derive(Copy, Clone)]
pub struct StoreFactory;

impl StoreFactory {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build_store(&self, cfg: &StoreConfig) -> Result<Box<dyn Storage>> {
        let db = match cfg.name.as_str() {
            "sqlite" => {
                let data_path = PathBuf::from(&cfg.data_path);
                dirs::ensure_dir_exists(&data_path).context("ensure data dir")?;
                let path = data_path.join("server.db");
                SqliteStore::open(&path)?
            }
            "memory" => SqliteStore::memory()?,
            _ => bail!("unsupported storage type: '{}'", cfg.name),
        };
        let store = Box::new(db) as Box<dyn Storage>;

        if cfg.cache.enable {
            let cache = Cache::new(Arc::new(store), cfg.cache.expiry);
            return Ok(Box::new(cache));
        }

        Ok(store)
    }
}
