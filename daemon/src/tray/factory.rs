use anyhow::Result;
use csync_misc::client::config::ClientConfig;
use csync_misc::client::factory::ClientFactory;
use tokio::sync::mpsc;

use crate::sync::send::SyncSender;

use super::daemon::{MenuData, TrayDaemon, WriteRequest};

pub struct TrayFactory {
    cfg: ClientConfig,
}

impl TrayFactory {
    pub fn new(cfg: ClientConfig) -> Self {
        Self { cfg }
    }

    pub async fn build_tray_daemon(
        self,
        limit: u64,
        truncate_size: usize,
        sync_tx: SyncSender,
    ) -> Result<(
        TrayDaemon,
        mpsc::Receiver<MenuData>,
        mpsc::Sender<WriteRequest>,
    )> {
        let user = self.cfg.user.clone();
        let password = self.cfg.password.clone();
        let client_factory = ClientFactory::new(self.cfg);
        let client = client_factory.build_client().await?;

        let (menu_tx, menu_rx) = mpsc::channel(500);
        let (write_tx, write_rx) = mpsc::channel(500);

        let tray_daemon = TrayDaemon {
            latest_text_id: None,
            latest_image_id: None,
            latest_file_id: None,
            client,
            sync_tx,
            token: None,
            user,
            password,
            menu_tx,
            write_rx,
            limit,
            truncate_size,
        };

        Ok((tray_daemon, menu_rx, write_tx))
    }
}
