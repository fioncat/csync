use anyhow::Result;
use async_trait::async_trait;
use clap::{Args, ValueEnum};

use crate::client::config::ClientConfig;
use crate::config::CommonConfig;
use crate::daemon::config::DaemonConfig;
use crate::display::display_json;
use crate::server::config::ServerConfig;

use super::{ConfigArgs, RunCommand};

/// Display the configuration information used in JSON format.
#[derive(Args)]
pub struct ShowConfigArgs {
    /// Name of the configuration to display.
    pub name: ConfigType,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ConfigType {
    Client,
    Server,
    Daemon,
}

#[async_trait]
impl RunCommand for ShowConfigArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        match self.name {
            ConfigType::Client => {
                let cfg = ps.load_config("client", ClientConfig::default)?;
                display_json(cfg)
            }
            ConfigType::Server => {
                let cfg = ps.load_config("server", ServerConfig::default)?;
                display_json(cfg)
            }
            ConfigType::Daemon => {
                let cfg = ps.load_config("daemon", DaemonConfig::default)?;
                display_json(cfg)
            }
        }
    }
}
