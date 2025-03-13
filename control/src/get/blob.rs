use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;

use crate::RunCommand;

/// Download blob from server, write to the stdout
#[derive(Args)]
pub struct BlobArgs {
    /// The id of the blob to download
    pub id: u64,

    /// Write content to daemon
    #[arg(short, long)]
    pub daemon: bool,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for BlobArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;
        let cfg: ClientConfig = self.config.load_from_path_set("client", &ps)?;
        let mut client = cfg.connect_restful(false).await?;

        let blob = client.get_blob(self.id).await?;

        if self.daemon {
            let mut daemon = cfg.connect_daemon().await?;
            daemon.send(&blob.data).await?;

            return Ok(());
        }

        blob.write(&ps)?;

        Ok(())
    }
}
