use std::path::PathBuf;

use anyhow::Result;
use csync_misc::config::{CommonConfig, PathSet};
use serde::{Deserialize, Serialize};

use super::SqliteConnection;

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct SqliteConfig {
    #[serde(default)]
    pub memory: bool,

    #[serde(skip)]
    path: PathBuf,
}

impl CommonConfig for SqliteConfig {
    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        if self.memory {
            return Ok(());
        }

        self.path = ps.data_dir.join("sqlite.db");

        Ok(())
    }
}

impl SqliteConfig {
    pub fn build(&self) -> Result<SqliteConnection> {
        if self.memory {
            SqliteConnection::memory()
        } else {
            SqliteConnection::open(&self.path)
        }
    }
}
