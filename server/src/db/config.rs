use anyhow::{Context, Result};
use csync_misc::config::{CommonConfig, PathSet};
use serde::{Deserialize, Serialize};

use crate::db::UnionConnection;

use super::{sqlite::config::SqliteConfig, Database};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DbConfig {
    #[serde(default)]
    #[serde(rename = "type")]
    pub db_type: DbType,

    #[serde(default)]
    pub sqlite: SqliteConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub enum DbType {
    #[serde(rename = "sqlite")]
    #[default]
    Sqlite,
}

impl CommonConfig for DbConfig {
    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        match self.db_type {
            DbType::Sqlite => self.sqlite.complete(ps).context("sqlite"),
        }
    }
}

impl DbConfig {
    pub fn build(&self) -> Result<Database> {
        let conn = match self.db_type {
            DbType::Sqlite => UnionConnection::Sqlite(
                self.sqlite
                    .build()
                    .context("build sqlite database connection")?,
            ),
        };
        Ok(Database::new(conn))
    }
}
