use anyhow::Result;
use async_trait::async_trait;
use clap::Args;

use crate::config::CommonConfig;
use crate::server::config::ServerConfig;
use crate::server::factory::ServerFactory;
use crate::types::server::Server;

use super::{ConfigArgs, LogArgs, ServerCommand};

/// Start the server to save clipboard data from different devices for persistence and
/// synchronization. This command will start an HTTP/HTTPS server.
#[derive(Args)]
pub struct ServerArgs {
    #[command(flatten)]
    pub config: ConfigArgs,

    #[command(flatten)]
    pub log: LogArgs,
}

#[async_trait]
impl ServerCommand for ServerArgs {
    async fn build_server(&self) -> Result<Server> {
        self.log.init()?;
        let ps = self.config.build_path_set()?;
        let cfg: ServerConfig = ps.load_config("server", ServerConfig::default)?;
        let factory = ServerFactory::new(cfg)?;

        let recycler = factory.build_recycler()?;
        if let Some(recycler) = recycler {
            recycler.start();
        }

        let srv = factory.build_server()?;
        Ok(Server::Server(srv))
    }
}
