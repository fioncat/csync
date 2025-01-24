use anyhow::Result;
use async_trait::async_trait;
use clap::Args;

use crate::client::config::ClientConfig;
use crate::config::CommonConfig;
use crate::daemon::config::DaemonConfig;
use crate::filelock::GlobalLock;
use crate::tray::factory::TrayFactory;
use crate::tray::ui::build_and_run_tray_ui;

use super::{ConfigArgs, LogArgs, RunCommand};

#[derive(Args)]
pub struct TrayArgs {
    #[command(flatten)]
    pub config: ConfigArgs,

    #[command(flatten)]
    pub log: LogArgs,
}

#[async_trait]
impl RunCommand for TrayArgs {
    async fn run(&self) -> Result<()> {
        self.log.init()?;
        let ps = self.config.build_path_set()?;

        let lock_path = ps.data_path.join("tray.lock");
        let lock = GlobalLock::acquire(lock_path)?;

        let daemon_cfg: DaemonConfig = ps.load_config("daemon", DaemonConfig::default)?;
        let client_cfg: ClientConfig = ps.load_config("client", ClientConfig::default)?;

        let factory = TrayFactory::new(client_cfg, daemon_cfg);
        let (mut daemon, menu_rx, write_tx) = factory.build_tray_daemon(20, 50).await?;

        let default_menu = daemon.build_menu().await?;

        tokio::spawn(async move {
            daemon.run().await;
        });

        build_and_run_tray_ui(default_menu, menu_rx, write_tx).await?;
        drop(lock);
        Ok(())
    }
}
