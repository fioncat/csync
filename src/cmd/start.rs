use anyhow::{Context, Result};
use clap::Args;

use crate::clipboard::Clipboard;
use crate::config::Config;
use crate::logs;
use crate::net::Client;
use crate::sync::Sync;

/// Watch clipboard and server
#[derive(Args)]
pub struct StartArgs {
    /// The config file to use.
    #[clap(long, short)]
    pub config: Option<String>,

    /// The log level
    #[clap(short, long, default_value = "info")]
    pub level: String,
}

impl StartArgs {
    pub async fn run(&self) -> Result<()> {
        logs::init(&self.level)?;
        let cfg = Config::load(self.config.as_deref())?;

        let clipboard = Clipboard::build(cfg.clipboard_interval).context("init clipboard")?;
        let client = Client::connect(cfg.addr.clone(), cfg.password.clone(), cfg.client_interval)
            .await
            .context("connect to server")?;

        let mut sync = Sync::new(clipboard, client, cfg.download_dir);
        sync.start().await;
        Ok(())
    }
}
