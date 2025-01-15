use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{CommonConfig, PathSet};

use super::cache::config::CacheConfig;
use super::sqlite::config::SqliteConfig;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DbConfig {
    #[serde(default = "DbConfig::default_name")]
    pub name: DbType,

    #[serde(default = "SqliteConfig::default")]
    pub sqlite: SqliteConfig,

    #[serde(default = "CacheConfig::default")]
    pub cache: CacheConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum DbType {
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
