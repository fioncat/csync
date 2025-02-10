use anyhow::Result;
use csync_misc::client::config::ClientConfig;
use csync_misc::client::factory::ClientFactory;
use tokio::sync::mpsc;

use crate::sync::send::SyncSender;

use super::config::TrayConfig;
use super::daemon::{MenuData, TrayDaemon, WriteRequest};

pub struct TrayFactory {
    tray_cfg: TrayConfig,
    client_cfg: ClientConfig,
}

impl TrayFactory {
    pub fn new(tray_cfg: TrayConfig, client_cfg: ClientConfig) -> Self {
        Self {
            tray_cfg,
            client_cfg,
        }
    }

    pub async fn build_tray_daemon(
        self,
        sync_tx: SyncSender,
    ) -> Result<(
        TrayDaemon,
        mpsc::Receiver<MenuData>,
        mpsc::Sender<WriteRequest>,
    )> {
        let user = self.client_cfg.user.clone();
        let password = self.client_cfg.password.clone();
        let client_factory = ClientFactory::new(self.client_cfg);
        let client = client_factory.build_client().await?;

        let (menu_tx, menu_rx) = mpsc::channel(500);
        let (write_tx, write_rx) = mpsc::channel(500);

        let tray_daemon = TrayDaemon {
            latest_text_id: None,
            latest_image_id: None,
            latest_file_id: None,
            enable_text: self.tray_cfg.text.enable,
            text_limit: self.tray_cfg.text.limit,
            enable_image: self.tray_cfg.image.enable,
            image_limit: self.tray_cfg.image.limit,
            enable_file: self.tray_cfg.file.enable,
            file_limit: self.tray_cfg.file.limit,
            client,
            sync_tx,
            token: None,
            user,
            password,
            menu_tx,
            write_rx,
            truncate_size: self.tray_cfg.truncate_text,
        };

        Ok((tray_daemon, menu_rx, write_tx))
    }
}
