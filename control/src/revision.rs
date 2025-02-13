use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::factory::ClientFactory;
use csync_misc::types::cmd::ConfigArgs;

use super::RunCommand;

/// Print the server's current revision
#[derive(Args)]
pub struct RevisionArgs {
    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for RevisionArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let rev = client.revision().await?;
        println!("{rev}");

        Ok(())
    }
}
