use std::sync::Arc;

use anyhow::{Context, Result};
use csync_misc::client::share::ShareClient;
use csync_misc::clipboard::Clipboard;
use tokio::sync::mpsc;

use super::config::SyncConfig;
use super::image::ImageSyncManager;
use super::text::TextSyncManager;
use super::{ResourceManager, SyncFlag, Synchronizer};

pub struct SyncFactory {
    cfg: SyncConfig,

    cb: Clipboard,
}

impl SyncFactory {
    pub async fn new(cfg: SyncConfig) -> Result<Self> {
        let cb = Clipboard::load().context("init clipboard driver")?;

        Ok(Self { cfg, cb })
    }

    pub fn build_text_sync(
        &self,
        share_client: Arc<ShareClient>,
    ) -> Option<(Synchronizer<TextSyncManager>, mpsc::Sender<Vec<u8>>)> {
        if !self.cfg.text.enable {
            return None;
        }

        Some(self.build_sync("text", TextSyncManager, share_client))
    }

    pub fn build_image_sync(
        &self,
        share_client: Arc<ShareClient>,
    ) -> Option<(Synchronizer<ImageSyncManager>, mpsc::Sender<Vec<u8>>)> {
        if !self.cfg.image.enable {
            return None;
        }

        Some(self.build_sync("image", ImageSyncManager, share_client))
    }

    fn build_sync<M: ResourceManager>(
        &self,
        name: &'static str,
        mgr: M,
        share_client: Arc<ShareClient>,
    ) -> (Synchronizer<M>, mpsc::Sender<Vec<u8>>) {
        let (cb_tx, cb_rx) = mpsc::channel(500);
        let sync = Synchronizer {
            name,
            mgr,
            flag: SyncFlag::None,
            bucket: None,
            server_hash: None,
            cb_hash: None,
            share_client,
            cb: self.cb,
            cb_request_rx: cb_rx,
            server_intv: self.cfg.server_intv_millis,
            cb_intv: self.cfg.image.cb_intv_millis,
            server_readonly: self.cfg.image.server_readonly,
            cb_readonly: self.cfg.image.cb_readonly,
            first_server: true,
        };
        (sync, cb_tx)
    }
}
