use anyhow::{Context, Result};
use csync_misc::config::{CommonConfig, PathSet};
use serde::{Deserialize, Serialize};

use crate::sync::config::SyncConfig;
use crate::tray::config::TrayConfig;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DaemonConfig {
    #[serde(default = "SyncConfig::default")]
    pub sync: SyncConfig,

    #[serde(default = "TrayConfig::default")]
    pub tray: TrayConfig,
}

impl CommonConfig for DaemonConfig {
    fn default() -> Self {
        Self {
            sync: SyncConfig::default(),
            tray: TrayConfig::default(),
        }
    }

    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        self.sync.complete(ps).context("sync")?;
        self.tray.complete(ps).context("tray")?;

        Ok(())
    }
}
