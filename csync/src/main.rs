#![allow(dead_code)]

mod config;
mod output;
mod sync;

use std::process;

use anyhow::Result;
use clap::Parser;
use console::style;
use csync_utils::build_info;

use crate::config::Config;
use crate::sync::Sync;

#[tokio::main]
async fn main() {
    match run().await {
        Ok(()) => {}
        Err(err) => {
            eprintln!("{}: {err:#}", style("error").red().bold());
            process::exit(12);
        }
    }
}

async fn run() -> Result<()> {
    let cfg = Config::parse();
    if cfg.build_info {
        build_info!("csync");
        return Ok(());
    }

    let mut sync = Sync::new(&cfg).await?;
    sync.start().await
}
