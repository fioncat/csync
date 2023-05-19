mod client;
mod config;
mod server;
mod sync;

use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::{Context, Result};

use arboard::Clipboard;
use clap::Parser;
use config::Arg;
use log::info;
use tokio::{self, sync::mpsc};

async fn run() -> Result<()> {
    let env = env_logger::Env::default()
        .filter_or("CSYNC_LOG_LEVEL", "info")
        .write_style_or("CSYNC_LOG_STYLE", "always");
    env_logger::Builder::from_env(env)
        .format_timestamp_millis()
        .format_target(false)
        .format_module_path(false)
        .init();

    let mut arg = Arg::parse();
    let cfg = arg.normalize()?;
    info!("{:?}", cfg);

    let cb = Clipboard::new().context("unable to init clipboard driver")?;

    let (sender, receiver) = mpsc::channel::<sync::Packet>(sync::CHANNEL_SIZE);
    tokio::spawn(sync::start(cfg.clone(), cb, receiver));

    server::start(&cfg, sender).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            _ = writeln!(io::stderr(), "fatal: {:#}", err);
            ExitCode::FAILURE
        }
    }
}
