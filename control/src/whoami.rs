use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::factory::ClientFactory;
use csync_misc::config::ConfigArgs;

use super::RunCommand;

/// Display the name of the currently authenticated user.
#[derive(Args)]
pub struct WhoamiArgs {
    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for WhoamiArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let name = client.whoami().await?;
        println!("{name}");

        Ok(())
    }
}
