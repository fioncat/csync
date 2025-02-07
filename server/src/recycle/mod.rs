pub mod config;
pub mod factory;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use config::RecycleResourceConfig;
use csync_misc::time::get_time_before_hours;
use log::{error, info};
use tokio::time::{interval_at, Instant};

use super::db::cache::Cache;
use super::db::{Database, Transaction};

pub struct Recycler {
    db: Arc<Database>,
    resources: Vec<RecycleResource>,
}

#[derive(Debug, Clone, Copy)]
pub enum RecycleResource {
    Text(RecycleResourceConfig),
    Image(RecycleResourceConfig),
    File(RecycleResourceConfig),
}

impl Recycler {
    const HANDLE_INTERVAL_SECS: u64 = 60 * 60; // 1 hour
    const DELETE_LIMIT: usize = 20;

    pub fn new(db: Arc<Database>, resources: Vec<RecycleResource>) -> Self {
        Self { db, resources }
    }

    pub fn start(self) {
        tokio::spawn(async move {
            self.main_loop().await;
        });
    }

    async fn main_loop(&self) {
        let mut intv = interval_at(
            Instant::now(),
            Duration::from_secs(Self::HANDLE_INTERVAL_SECS),
        );

        info!("Starting recycling resources");
        loop {
            intv.tick().await;

            for rsc in self.resources.iter() {
                let name = rsc.get_name();
                let result = self.db.with_transaction(|tx, cache| {
                    let need_clear_cache = self.handle(name, tx, *rsc)?;
                    if need_clear_cache {
                        rsc.clear_cache(cache)?;
                    }
                    Ok(())
                });
                if let Err(e) = result {
                    error!("Failed to recycle '{name}': {e:#}");
                }
            }
        }
    }

    fn handle(&self, name: &str, tx: &dyn Transaction, rsc: RecycleResource) -> Result<bool> {
        let cfg = rsc.get_config();

        let mut need_clear_cache = false;
        let count = rsc.count(tx)?;
        if count > cfg.max as usize {
            let mut delta = count - cfg.max as usize;
            if delta > Self::DELETE_LIMIT {
                delta = Self::DELETE_LIMIT;
            }
            info!(
                "Resource '{name}' count {count} exceeds the limit {}, start to recycle with limit {}",
                cfg.max,  delta
            );
            let to_delete = rsc.get_oldest_ids(tx, delta)?;
            let deleted = rsc.delete(tx, &to_delete)?;
            info!("Recycled {deleted} '{name}' items");
            need_clear_cache = true;
        }

        let outdated_time = get_time_before_hours(cfg.keep_hours);
        let deleted = rsc.delete_before_time(tx, outdated_time)?;
        if deleted > 0 {
            info!("Deleted {deleted} outdated '{name}' items created before {outdated_time}");
            need_clear_cache = true;
        }

        Ok(need_clear_cache)
    }
}

impl RecycleResource {
    fn get_name(&self) -> &'static str {
        match self {
            Self::Text(_) => "text",
            Self::Image(_) => "image",
            Self::File(_) => "file",
        }
    }

    fn get_config(&self) -> RecycleResourceConfig {
        match self {
            Self::Text(config) => *config,
            Self::Image(config) => *config,
            Self::File(config) => *config,
        }
    }

    fn count(&self, tx: &dyn Transaction) -> Result<usize> {
        match self {
            Self::Text(_) => tx.count_texts(None),
            Self::Image(_) => tx.count_images(None),
            Self::File(_) => tx.count_files(None),
        }
    }

    fn get_oldest_ids(&self, tx: &dyn Transaction, limit: usize) -> Result<Vec<u64>> {
        match self {
            Self::Text(_) => tx.get_oldest_text_ids(limit),
            Self::Image(_) => tx.get_oldest_image_ids(limit),
            Self::File(_) => tx.get_oldest_file_ids(limit),
        }
    }

    fn delete(&self, tx: &dyn Transaction, ids: &[u64]) -> Result<usize> {
        match self {
            Self::Text(_) => tx.delete_texts_batch(ids),
            Self::Image(_) => tx.delete_images_batch(ids),
            Self::File(_) => tx.delete_files_batch(ids),
        }
    }

    fn delete_before_time(&self, tx: &dyn Transaction, time: u64) -> Result<usize> {
        match self {
            Self::Text(_) => tx.delete_texts_before_time(time),
            Self::Image(_) => tx.delete_images_before_time(time),
            Self::File(_) => tx.delete_files_before_time(time),
        }
    }

    fn clear_cache(&self, cache: &dyn Cache) -> Result<()> {
        match self {
            Self::Text(_) => cache.clear_text(),
            Self::Image(_) => cache.clear_image(),
            Self::File(_) => cache.clear_file(),
        }
    }
}
