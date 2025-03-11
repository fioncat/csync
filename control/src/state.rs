use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;
use csync_misc::display;

use super::RunCommand;

/// Display current server state
#[derive(Args)]
pub struct StateArgs {
    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for StateArgs {
    async fn run(&self) -> Result<()> {
        let cfg: ClientConfig = self.config.load("client")?;
        let mut client = cfg.connect_restful(false).await?;

        let state = client.get_state().await?;

        display::pretty_json(state)?;

        Ok(())
    }
}
