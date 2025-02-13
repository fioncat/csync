use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::CommonConfig;
use csync_misc::config::ConfigArgs;
use csync_misc::display::display_json;

use super::RunCommand;

/// Display the configuration information used in JSON format.
#[derive(Args)]
pub struct ShowConfigArgs {
    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for ShowConfigArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let cfg = ps.load_config("client", ClientConfig::default)?;
        display_json(cfg)
    }
}
