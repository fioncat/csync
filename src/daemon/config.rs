use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{CommonConfig, PathSet};

use super::sync::config::SyncConfig;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DaemonConfig {
    #[serde(default = "DaemonConfig::default_port")]
    pub port: u16,

    #[serde(default = "SyncConfig::default")]
    pub sync: SyncConfig,
}

impl CommonConfig for DaemonConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            sync: SyncConfig::default(),
        }
    }

    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        self.sync.complete(ps).context("sync")?;
        Ok(())
    }
}

impl DaemonConfig {
    pub fn default_port() -> u16 {
        7882
    }
}
