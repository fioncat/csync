use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::{expandenv, CommonConfig, PathSet};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SqliteConfig {
    #[serde(default = "SqliteConfig::default_memory")]
    pub memory: bool,

    #[serde(default = "SqliteConfig::default_path")]
    pub path: String,
}

impl CommonConfig for SqliteConfig {
    fn default() -> Self {
        Self {
            memory: Self::default_memory(),
            path: Self::default_path(),
        }
    }

    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        if self.memory {
            return Ok(());
        }

        self.path = expandenv("path", &self.path)?;
        if self.path.is_empty() {
            let path = ps.data_path.join("server.db");
            self.path = format!("{}", path.display());
        }

        Ok(())
    }
}

impl SqliteConfig {
    fn default_memory() -> bool {
        false
    }

    fn default_path() -> String {
        String::new()
    }
}
