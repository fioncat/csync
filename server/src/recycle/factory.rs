use std::sync::Arc;

use anyhow::Result;
use log::warn;

use crate::db::Database;
use crate::recycle::RecycleResource;
use crate::revision::Revision;

use super::config::RecycleConfig;
use super::Recycler;

pub struct RecyclerFactory;

impl RecyclerFactory {
    pub fn new() -> Self {
        Self
    }

    pub fn build_recycler(
        &self,
        cfg: &RecycleConfig,
        db: Arc<Database>,
        revision: Arc<Revision>,
    ) -> Result<Option<Recycler>> {
        let mut resources = Vec::new();

        if cfg.text.enable {
            let rsc = RecycleResource::Text(cfg.text);
            resources.push(rsc);
        } else {
            warn!("Recycle text is disabled");
        }

        if cfg.image.enable {
            let rsc = RecycleResource::Image(cfg.image);
            resources.push(rsc);
        } else {
            warn!("Recycle image is disabled");
        }

        if cfg.file.enable {
            let rsc = RecycleResource::File(cfg.file);
            resources.push(rsc);
        } else {
            warn!("Recycle file is disabled");
        }

        if resources.is_empty() {
            return Ok(None);
        }

        let recycler = Recycler::new(db, resources, revision);
        Ok(Some(recycler))
    }
}
