mod config;
mod net;
mod server;
mod sync;

use std::io::{self, Write};
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use config::Arg;
use log::debug;

use crate::server::Server;
use crate::sync::Synchronizer;

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
    debug!("Use config: {:?}", cfg);

    let (mut syncer, sender) = Synchronizer::new(&cfg).await?;
    let mut server = Server::new(&cfg.bind, sender, cfg.conn_max as usize).await?;
    if let Some(auth_key) = &cfg.auth_key {
        server.with_auth(auth_key.clone());
        syncer.with_auth(auth_key.clone());
    }

    tokio::spawn(async move { syncer.run(&cfg).await });

    server.run().await
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            _ = writeln!(io::stderr(), "Fatal: {:#}", err);
            ExitCode::FAILURE
        }
    }
}
