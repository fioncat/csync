use anyhow::Result;
use async_trait::async_trait;
use clap::Args;

use crate::client::config::ClientConfig;
use crate::config::CommonConfig;
use crate::daemon::config::DaemonConfig;
use crate::daemon::factory::DaemonFactory;
use crate::filelock::GlobalLock;
use crate::types::server::Server;

use super::{ConfigArgs, LogArgs, ServerCommand};

/// Start the daemon service. This service will synchronize the system clipboard with the
/// server. Note that only one daemon process is allowed to run on a machine, and it is
/// recommended to clear the clipboard data before execution.
#[derive(Args)]
pub struct DaemonArgs {
    #[command(flatten)]
    pub config: ConfigArgs,

    #[command(flatten)]
    pub log: LogArgs,
}

#[async_trait]
impl ServerCommand for DaemonArgs {
    async fn build_server(&self) -> Result<Server> {
        self.log.init()?;
        let ps = self.config.build_path_set()?;

        let lock_path = ps.data_path.join("daemon.lock");
        let lock = GlobalLock::acquire(lock_path)?;

        let daemon_cfg: DaemonConfig = ps.load_config("daemon", DaemonConfig::default)?;
        let client_cfg: ClientConfig = ps.load_config("client", ClientConfig::default)?;

        let factory = DaemonFactory::new(daemon_cfg, client_cfg);

        let (text_sync, image_sync, ctx) = factory.build_sync().await?;
        if let Some(text_sync) = text_sync {
            text_sync.start();
        }
        if let Some(image_sync) = image_sync {
            image_sync.start();
        }

        let mut srv = factory.build_server(ctx);
        srv.set_global_lock(lock);

        Ok(Server::Daemon(srv))
    }
}
