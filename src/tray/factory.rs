use anyhow::Result;
use tokio::sync::mpsc;

use crate::client::config::ClientConfig;
use crate::client::factory::ClientFactory;
use crate::daemon::config::DaemonConfig;

use super::daemon::TrayDaemon;

pub struct TrayFactory {
    client_cfg: ClientConfig,
    daemon_cfg: DaemonConfig,
}

impl TrayFactory {
    pub fn new(client_cfg: ClientConfig, daemon_cfg: DaemonConfig) -> Self {
        Self {
            client_cfg,
            daemon_cfg,
        }
    }

    pub async fn build_tray_daemon(
        self,
        limit: u64,
        truncate_size: usize,
    ) -> Result<(
        TrayDaemon,
        mpsc::Receiver<Vec<(String, String)>>,
        mpsc::Sender<u64>,
    )> {
        let user = self.client_cfg.user.clone();
        let password = self.client_cfg.password.clone();
        let client_factory = ClientFactory::new(self.client_cfg);
        let client = client_factory.build_client().await?;

        let (menu_tx, menu_rx) = mpsc::channel(500);
        let (write_tx, write_rx) = mpsc::channel(500);

        let tray_daemon = TrayDaemon {
            latest_id: None,
            client,
            daemon_port: self.daemon_cfg.port,
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
