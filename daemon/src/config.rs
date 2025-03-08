use anyhow::{bail, Result};
use csync_misc::client::config::ClientConfig;
use csync_misc::config::{CommonConfig, PathSet};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::clipboard::ClipboardSync;
use crate::remote::Remote;
use crate::server::DaemonServer;
use crate::tray::SystemTray;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaemonConfig {
    #[serde(default = "DaemonConfig::default_cache_secs")]
    pub cache_secs: u64,

    #[serde(default = "DaemonConfig::default_refresh_tray_secs")]
    pub refresh_tray_secs: u64,

    #[serde(default = "DaemonConfig::default_tray_limit")]
    pub tray_limit: u64,

    #[serde(default = "DaemonConfig::default_clipboard_secs")]
    pub clipboard_secs: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        DaemonConfig {
            cache_secs: Self::default_cache_secs(),
            refresh_tray_secs: Self::default_refresh_tray_secs(),
            tray_limit: Self::default_tray_limit(),
            clipboard_secs: Self::default_clipboard_secs(),
        }
    }
}

impl CommonConfig for DaemonConfig {
    fn complete(&mut self, _ps: &PathSet) -> Result<()> {
        if self.cache_secs == 0 {
            bail!("cache_secs is required");
        }

        if self.cache_secs < Self::MIN_CACHE_SECS || self.cache_secs > Self::MAX_CACHE_SECS {
            bail!(
                "cache_secs must be in range [{}, {}]",
                Self::MIN_CACHE_SECS,
                Self::MAX_CACHE_SECS
            );
        }

        if self.refresh_tray_secs == 0 {
            bail!("refresh_tray_secs is required");
        }
        if self.refresh_tray_secs > Self::MAX_REFRESH_TRAY_SECS {
            bail!(
                "refresh_tray_secs must be less than or equal to {}",
                Self::MAX_REFRESH_TRAY_SECS
            );
        }

        if self.tray_limit == 0 {
            bail!("tray_limit is required");
        }
        if self.tray_limit < Self::MIN_TRAY_LIMIT || self.tray_limit > Self::MAX_TRAY_LIMIT {
            bail!(
                "tray_limit must be in range [{}, {}]",
                Self::MIN_TRAY_LIMIT,
                Self::MAX_TRAY_LIMIT
            );
        }

        if self.clipboard_secs == 0 {
            bail!("clipboard_secs is required");
        }
        if self.clipboard_secs < Self::MIN_CLIPBOARD_SECS
            || self.clipboard_secs > Self::MAX_CLIPBOARD_SECS
        {
            bail!(
                "clipboard_secs must be in range [{}, {}]",
                Self::MIN_CLIPBOARD_SECS,
                Self::MAX_CLIPBOARD_SECS
            );
        }

        Ok(())
    }
}

impl DaemonConfig {
    const MIN_CACHE_SECS: u64 = 60;
    const MAX_CACHE_SECS: u64 = 86400;

    const MAX_REFRESH_TRAY_SECS: u64 = 60;

    const MIN_TRAY_LIMIT: u64 = 5;
    const MAX_TRAY_LIMIT: u64 = 100;

    const MIN_CLIPBOARD_SECS: u64 = 1;
    const MAX_CLIPBOARD_SECS: u64 = 60;

    pub async fn build_remote(&self, client_cfg: &ClientConfig) -> Result<Remote> {
        let client = client_cfg.connect_restful(true).await?;
        let events_sub = client_cfg.subscribe_events().await?;

        let remote = Remote::start(client, self.cache_secs, events_sub);
        Ok(remote)
    }

    pub fn start_clipboard(&self, remote: Remote) -> Result<mpsc::Sender<Vec<u8>>> {
        let copy_tx = ClipboardSync::start(remote, self.clipboard_secs)?;
        Ok(copy_tx)
    }

    pub fn build_tray(
        &self,
        remote: Remote,
        ps: PathSet,
        copy_tx: mpsc::Sender<Vec<u8>>,
    ) -> SystemTray {
        SystemTray::new(remote, ps, copy_tx, self.tray_limit, self.refresh_tray_secs)
    }

    pub fn build_server(
        &self,
        client_cfg: &ClientConfig,
        copy_tx: mpsc::Sender<Vec<u8>>,
    ) -> DaemonServer {
        let addr = format!("127.0.0.1:{}", client_cfg.daemon_port);
        DaemonServer::new(addr, copy_tx)
    }

    fn default_cache_secs() -> u64 {
        3600 // 1hour
    }

    fn default_refresh_tray_secs() -> u64 {
        1
    }

    fn default_tray_limit() -> u64 {
        20
    }

    fn default_clipboard_secs() -> u64 {
        1
    }
}
