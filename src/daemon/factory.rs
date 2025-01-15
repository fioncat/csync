use std::sync::Arc;

use anyhow::Result;

use crate::client::config::ClientConfig;

use super::config::DaemonConfig;
use super::server::{DaemonContext, DaemonServer};
use super::sync::factory::SyncFactory;
use super::sync::image::ImageSyncManager;
use super::sync::text::TextSyncManager;
use super::sync::Synchronizer;

pub struct DaemonFactory {
    daemon_cfg: DaemonConfig,
    client_cfg: ClientConfig,
}

impl DaemonFactory {
    pub fn new(daemon_cfg: DaemonConfig, client_cfg: ClientConfig) -> Self {
        Self {
            daemon_cfg,
            client_cfg,
        }
    }

    pub async fn build_sync(
        &self,
    ) -> Result<(
        Option<Synchronizer<TextSyncManager>>,
        Option<Synchronizer<ImageSyncManager>>,
        DaemonContext,
    )> {
        let sync_factory =
            SyncFactory::new(self.client_cfg.clone(), self.daemon_cfg.sync.clone()).await?;

        let mut ctx = DaemonContext {
            text_tx: None,
            image_tx: None,
        };

        let mut ret_text = None;
        if let Some((text_sync, text_tx)) = sync_factory.build_text_sync() {
            ctx.text_tx = Some(text_tx);
            ret_text = Some(text_sync);
        }

        let mut ret_image = None;
        if let Some((image_sync, image_tx)) = sync_factory.build_image_sync() {
            ctx.image_tx = Some(image_tx);
            ret_image = Some(image_sync);
        }

        Ok((ret_text, ret_image, ctx))
    }

    pub fn build_server(&self, ctx: DaemonContext) -> DaemonServer {
        let ctx = Arc::new(ctx);
        DaemonServer::new(ctx, self.daemon_cfg.port)
    }
}
