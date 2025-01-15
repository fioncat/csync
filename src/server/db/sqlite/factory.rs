use anyhow::Result;
use log::{info, warn};

use super::config::SqliteConfig;
use super::Sqlite;

pub struct SqliteFactory;

impl SqliteFactory {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build_sqlite(&self, cfg: &SqliteConfig) -> Result<Sqlite> {
        if cfg.memory {
            warn!("Using in-memory sqlite database, the data will be lost when the server stops");
            return Sqlite::memory();
        }

        info!("Using sqlite database: {}", cfg.path);
        Sqlite::open(cfg.path.as_ref())
    }
}
