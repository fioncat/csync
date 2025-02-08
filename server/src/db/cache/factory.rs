use anyhow::Result;

use super::config::{CacheConfig, CacheType};
use super::memory::MemoryCache;
use super::UnionCache;

pub struct CacheFactory;

/// Factory for building cache instances
impl CacheFactory {
    /// Creates a new cache factory instance
    pub fn new() -> Self {
        Self
    }

    /// Builds a cache instance based on configuration.
    /// Returns None if caching is disabled.
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
