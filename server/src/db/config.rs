use anyhow::{Context, Result};
use csync_misc::config::{CommonConfig, PathSet};
use serde::{Deserialize, Serialize};

use super::cache::config::CacheConfig;
use super::sqlite::config::SqliteConfig;

/// Database configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DbConfig {
    /// Database type to use
    #[serde(default = "DbConfig::default_name")]
    pub name: DbType,

    /// SQLite configuration, only valid when database type is sqlite
    #[serde(default = "SqliteConfig::default")]
    pub sqlite: SqliteConfig,

    /// Cache configuration
    #[serde(default = "CacheConfig::default")]
    pub cache: CacheConfig,
}

/// Database type
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum DbType {
    /// Use SQLite database
    #[serde(rename = "sqlite")]
    Sqlite,
}

impl CommonConfig for DbConfig {
    fn default() -> Self {
        Self {
            name: Self::default_name(),
            sqlite: SqliteConfig::default(),
            cache: CacheConfig::default(),
        }
    }

    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        self.sqlite.complete(ps).context("sqlite")?;
        self.cache.complete(ps).context("cache")?;
        Ok(())
    }
}

impl DbConfig {
    fn default_name() -> DbType {
        DbType::Sqlite
    }
}
