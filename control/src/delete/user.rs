use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;

use crate::RunCommand;

/// Delete a user from the server
#[derive(Args)]
pub struct UserArgs {
    /// The name of the user to delete
    pub name: String,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for UserArgs {
    async fn run(&self) -> Result<()> {
        let cfg: ClientConfig = self.config.load("client")?;
        let mut client = cfg.connect_restful(false).await?;

        client.delete_user(self.name.clone()).await?;

        Ok(())
    }
}
