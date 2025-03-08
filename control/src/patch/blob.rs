use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::api::blob::PatchBlobRequest;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;

use crate::RunCommand;

/// Patch a blob
#[derive(Args)]
pub struct BlobArgs {
    /// The blob id to patch
    pub id: u64,

    /// Pin the blob
    #[arg(long)]
    pub pin: bool,

    /// Unpin the blob
    #[arg(long)]
    pub unpin: bool,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for BlobArgs {
    async fn run(&self) -> Result<()> {
        let cfg: ClientConfig = self.config.load("client")?;
        let mut client = cfg.connect_restful(false).await?;

        let pin = if self.pin {
            Some(true)
        } else if self.unpin {
            Some(false)
        } else {
            None
        };

        let patch = PatchBlobRequest { id: self.id, pin };

        client.patch_blob(patch).await?;

        Ok(())
    }
}
