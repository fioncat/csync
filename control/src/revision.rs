use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;
use csync_misc::display;

use super::RunCommand;

/// Display current revision and latest sha256
#[derive(Args)]
pub struct RevisionArgs {
    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for RevisionArgs {
    async fn run(&self) -> Result<()> {
        let cfg: ClientConfig = self.config.load("client")?;
        let mut client = cfg.connect_restful(false).await?;

        let rev = client.get_revision().await?;

        display::pretty_json(rev)?;

        Ok(())
    }
}
