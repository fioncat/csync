use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;

use super::RunCommand;

/// Display client and server versions.
#[derive(Args)]
pub struct VersionArgs {
    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for VersionArgs {
    async fn run(&self) -> Result<()> {
        let cfg: ClientConfig = self.config.load("client")?;
        let client = cfg.connect_restful(false).await?;

        println!("Client version: {}", env!("CSYNC_VERSION"));
        println!("Server version: {}", client.get_server_version());

        Ok(())
    }
}
