use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::{CommonConfig, PathSet};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CacheConfig {
    #[serde(default = "CacheConfig::default_enable")]
    pub enable: bool,

    #[serde(default = "CacheConfig::default_name")]
    pub name: CacheType,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum CacheType {
    #[serde(rename = "memory")]
    Memory,
}

impl CommonConfig for CacheConfig {
    fn default() -> Self {
        Self {
            enable: Self::default_enable(),
            name: Self::default_name(),
        }
    }
    fn complete(&mut self, _ps: &PathSet) -> Result<()> {
        Ok(())
    }
}

impl CacheConfig {
    fn default_enable() -> bool {
        true
    }

    fn default_name() -> CacheType {
        CacheType::Memory
    }
}
