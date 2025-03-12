mod auth;
mod config;
mod context;
mod db;
mod handlers;
mod recycle;
mod request;
mod restful;

use std::process;

use anyhow::{Context, Result};
use clap::Parser;
use config::ServerConfig;
use csync_misc::config::ConfigArgs;
use csync_misc::display;
use log::{error, info};

#[derive(Parser, Debug)]
#[command(author, version = env!("CSYNC_VERSION"), about)]
struct ServerArgs {
    /// Print server configuration data (JSON) and exit.
    #[arg(long)]
    pub print_config: bool,

    #[command(flatten)]
    pub config: ConfigArgs,
}

async fn run(args: ServerArgs) -> Result<()> {
    let cfg: ServerConfig = args.config.load("server")?;

    if args.print_config {
        return display::pretty_json(cfg);
    }

    cfg.logs.init("server")?;

    let ctx = cfg.build_ctx()?;

    let resftul_server = cfg.build_restful_server(ctx.clone())?;

    tokio::spawn(async move {
        recycle::start_recycle(ctx).await;
    });

    resftul_server.run().await.context("run restful server")?;

    info!("Server exited by user");
    Ok(())
}

#[tokio::main]
async fn main() {
    let args = ServerArgs::parse();
    match run(args).await {
        Ok(()) => {}
        Err(e) => {
            error!("Error: {:#}", e);
            process::exit(1);
        }
    }
}
