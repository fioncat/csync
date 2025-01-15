use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{CommonConfig, PathSet};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SyncConfig {
    #[serde(default = "SyncConfig::default_server_intv_millis")]
    pub server_intv_millis: u64,

    #[serde(default = "SyncConfig::default_resource_config")]
    pub text: ResourceConfig,
    #[serde(default = "SyncConfig::default_resource_config")]
    pub image: ResourceConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResourceConfig {
    #[serde(default = "SyncConfig::default_enable")]
    pub enable: bool,

    #[serde(default = "SyncConfig::default_cb_intv_millis")]
    pub cb_intv_millis: u64,

    #[serde(default = "SyncConfig::default_readonly")]
    pub server_readonly: bool,
    #[serde(default = "SyncConfig::default_readonly")]
    pub cb_readonly: bool,
}

impl CommonConfig for SyncConfig {
    fn default() -> Self {
        Self {
            server_intv_millis: Self::default_server_intv_millis(),
            text: Self::default_resource_config(),
            image: Self::default_resource_config(),
        }
    }

    fn complete(&mut self, _ps: &PathSet) -> Result<()> {
        if !self.text.enable && !self.image.enable {
            bail!("both text and image are disabled");
        }

        Self::validate_intv(self.server_intv_millis).context("validate server interval")?;
        Self::validate_resource(&self.text).context("validate text")?;
        Self::validate_resource(&self.image).context("validate image")?;

        Ok(())
    }
}

impl SyncConfig {
    const MAX_INTV_MILLIS: u64 = 1000 * 5; // 5s
    const MIN_INTV_MILLIS: u64 = 100;

    fn default_resource_config() -> ResourceConfig {
        ResourceConfig {
            enable: Self::default_enable(),
            cb_intv_millis: Self::default_cb_intv_millis(),
            server_readonly: Self::default_readonly(),
            cb_readonly: Self::default_readonly(),
        }
    }

    fn default_server_intv_millis() -> u64 {
        1000
    }

    fn default_cb_intv_millis() -> u64 {
        500
    }

    fn default_enable() -> bool {
        true
    }

    fn default_readonly() -> bool {
        false
    }

    fn validate_resource(cfg: &ResourceConfig) -> Result<()> {
        if !cfg.enable {
            return Ok(());
        }

        Self::validate_intv(cfg.cb_intv_millis).context("validate cb interval")?;

        if cfg.server_readonly && cfg.cb_readonly {
            bail!("both server and clipboard are readonly");
        }

        Ok(())
    }

    fn validate_intv(intv: u64) -> Result<()> {
        if intv < Self::MIN_INTV_MILLIS {
            bail!(
                "interval too short, should be at least {}ms",
                Self::MIN_INTV_MILLIS
            );
        }
        if intv > Self::MAX_INTV_MILLIS {
            bail!(
                "interval too large, should be at most {}ms",
                Self::MAX_INTV_MILLIS
            );
        }
        Ok(())
    }
}
