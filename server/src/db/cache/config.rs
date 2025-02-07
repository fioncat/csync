use anyhow::Result;
use csync_misc::config::{CommonConfig, PathSet};
use serde::{Deserialize, Serialize};

/// Cache configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CacheConfig {
    /// Whether to enable caching
    #[serde(default = "CacheConfig::default_enable")]
    pub enable: bool,

    /// Cache type
    #[serde(default = "CacheConfig::default_name")]
    pub name: CacheType,
}

/// Cache type
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum CacheType {
    /// Memory-based cache that stores hot data in memory.
    /// Cache data is lost after restart and does not expire automatically.
    /// Note: Do not use this cache type in distributed server deployments
    /// as it may cause cache inconsistency. For distributed setups,
    /// please use other distributed caching solutions.
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
