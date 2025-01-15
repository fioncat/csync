use std::sync::Arc;

use anyhow::Result;

use super::cache::factory::CacheFactory;
use super::config::{DbConfig, DbType};
use super::sqlite::factory::SqliteFactory;
use super::{Database, UnionConnection};

pub struct DbFactory;

impl DbFactory {
    pub fn new() -> Self {
        Self
    }

    pub fn build_db(&self, cfg: &DbConfig) -> Result<Arc<Database>> {
        let conn = match cfg.name {
            DbType::Sqlite => {
                let sqlite_factory = SqliteFactory::new();
                let sqlite = sqlite_factory.build_sqlite(&cfg.sqlite)?;
                UnionConnection::Sqlite(sqlite)
            }
        };

        let cache_factory = CacheFactory::new();
        let cache = cache_factory.build_cache(&cfg.cache)?;
        let db = Database::new(conn, cache);
        Ok(Arc::new(db))
    }
}
