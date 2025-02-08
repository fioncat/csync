use anyhow::Result;
use csync_misc::config::{expandenv, CommonConfig, PathSet};
use serde::{Deserialize, Serialize};

/// SQLite configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SqliteConfig {
    /// Whether to use in-memory database. All data will be lost when the program exits.
    /// Recommended for testing only.
    #[serde(default = "SqliteConfig::default_memory")]
    pub memory: bool,

    /// Database file path, only valid when memory is false.
    /// Default: {data_path}/server.db
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
