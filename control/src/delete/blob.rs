use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;

use crate::RunCommand;

/// Delete a blob from server
#[derive(Args)]
pub struct BlobArgs {
    /// The id of the blob to delete
    pub id: u64,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for BlobArgs {
    async fn run(&self) -> Result<()> {
        let cfg: ClientConfig = self.config.load("client")?;
        let mut client = cfg.connect_restful(false).await?;

        client.delete_blob(self.id).await?;

        Ok(())
    }
}
