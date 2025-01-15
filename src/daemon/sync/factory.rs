use anyhow::{Context, Result};
use tokio::sync::mpsc;

use crate::client::config::ClientConfig;
use crate::client::factory::ClientFactory;
use crate::client::Client;
use crate::clipboard::Clipboard;

use super::config::SyncConfig;
use super::image::ImageSyncManager;
use super::text::TextSyncManager;
use super::{ResourceManager, SyncFlag, Synchronizer};

pub struct SyncFactory {
    client: Client,
    cfg: SyncConfig,

    cb: Clipboard,

    user: String,
    password: String,
}

impl SyncFactory {
    pub async fn new(client_cfg: ClientConfig, sync_cfg: SyncConfig) -> Result<Self> {
        let user = client_cfg.user.clone();
        let password = client_cfg.password.clone();

        let client_factory = ClientFactory::new(client_cfg);
        let client = client_factory.build_client().await.context("init client")?;

        let cb = Clipboard::load().context("init clipboard driver")?;

        Ok(Self {
            client,
            cfg: sync_cfg,
            cb,
            user,
            password,
        })
    }

    pub fn build_text_sync(
        &self,
    ) -> Option<(Synchronizer<TextSyncManager>, mpsc::Sender<Vec<u8>>)> {
        if !self.cfg.text.enable {
            return None;
        }

        Some(self.build_sync("text", TextSyncManager))
    }

    pub fn build_image_sync(
        &self,
    ) -> Option<(Synchronizer<ImageSyncManager>, mpsc::Sender<Vec<u8>>)> {
        if !self.cfg.image.enable {
            return None;
        }

        Some(self.build_sync("image", ImageSyncManager))
    }

    fn build_sync<M: ResourceManager>(
        &self,
        name: &'static str,
        mgr: M,
    ) -> (Synchronizer<M>, mpsc::Sender<Vec<u8>>) {
        let (cb_tx, cb_rx) = mpsc::channel(500);
        let sync = Synchronizer {
            name,
            mgr,
            flag: SyncFlag::None,
            bucket: None,
            server_hash: None,
            cb_hash: None,
            client: self.client.clone(),
            token: None,
            user: self.user.clone(),
            password: self.password.clone(),
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
