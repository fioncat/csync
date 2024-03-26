use anyhow::Result;
use clap::Args;

use crate::{config::Config, sync};

/// Watch clipboard and server
#[derive(Args)]
pub struct WatchArgs {
    /// The config file to use.
    #[clap(long, short)]
    pub config: Option<String>,
}

impl WatchArgs {
    pub async fn run(&self) -> Result<()> {
        let cfg = Config::load(self.config.as_deref())?;
        sync::start(cfg).await
    }
}
