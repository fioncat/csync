mod channel;
mod config;
mod server;
mod worker;

#[cfg(test)]
mod tests;

use std::{env, process};

use anyhow::Result;
use clap::Parser;
use csync_utils::build_info;
use log::{debug, error};

use crate::config::Config;

#[tokio::main]
async fn main() {
    match run().await {
        Ok(()) => {}
        Err(err) => {
            error!("fatal: {:#}", err);
            process::exit(12);
        }
    }
}

async fn run() -> Result<()> {
    let cfg = Config::parse();
    if cfg.build_info {
        build_info!("csyncd");
        return Ok(());
    }

    if cfg.debug {
        env::set_var("CSYNC_LOG_LEVEL", "debug");
    }
    let log_env = env_logger::Env::default()
        .filter_or("CSYNC_LOG_LEVEL", "info")
        .write_style_or("CSYNC_LOG_STYLE", "always");
    env_logger::Builder::from_env(log_env)
        .format_timestamp_millis()
        .format_target(false)
        .format_module_path(false)
        .init();

    debug!("Use config: {:?}", cfg);
    server::start(&cfg).await
}
