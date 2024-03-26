use std::io::{self, Read};

use anyhow::{Context, Result};
use clap::Args;

use crate::config::Config;
use crate::sync::notify;

/// Notify to send content
#[derive(Args)]
pub struct NotifyArgs {
    /// The config file to use.
    #[clap(long, short)]
    pub config: Option<String>,
}

impl NotifyArgs {
    pub async fn run(&self) -> Result<()> {
        let cfg = Config::load(self.config.as_deref())?;

        let mut buf = Vec::with_capacity(512);
        io::stdin()
            .lock()
            .read_to_end(&mut buf)
            .context("read data from stdin")?;

        let path = cfg.get_notify_path();
        notify::write(path, &buf)
    }
}
