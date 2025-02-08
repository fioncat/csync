use anyhow::Result;
use log::{info, warn};

use super::config::SqliteConfig;
use super::Sqlite;

/// Factory for building SQLite instances
pub struct SqliteFactory;

impl SqliteFactory {
    /// Creates a new SQLite factory instance
    pub fn new() -> Self {
        Self {}
    }

    /// Builds a SQLite instance based on configuration
    pub fn build_sqlite(&self, cfg: &SqliteConfig) -> Result<Sqlite> {
        if cfg.memory {
            warn!("Using in-memory sqlite database, the data will be lost when the server stops");
            return Sqlite::memory();
        }

        info!("Using sqlite database: {}", cfg.path);
        Sqlite::open(cfg.path.as_ref())
    }
}
