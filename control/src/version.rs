use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::factory::ClientFactory;
use csync_misc::types::cmd::ConfigArgs;

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
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let resp = client.healthz().await?;

        println!("Client version: {}", env!("CSYNC_VERSION"));
        println!(
            "Server version: {}",
            resp.version.unwrap_or(String::from("Unknown"))
        );

        Ok(())
    }
}
