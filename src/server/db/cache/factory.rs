use anyhow::Result;

use super::config::{CacheConfig, CacheType};
use super::memory::MemoryCache;
use super::UnionCache;

pub struct CacheFactory;

impl CacheFactory {
    pub fn new() -> Self {
        Self
    }

    pub fn build_cache(&self, cfg: &CacheConfig) -> Result<Option<UnionCache>> {
        if !cfg.enable {
            return Ok(None);
        }

        let cache = match cfg.name {
            CacheType::Memory => UnionCache::Memory(MemoryCache::new()),
        };
        Ok(Some(cache))
    }
}
