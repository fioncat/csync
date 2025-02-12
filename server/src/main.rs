mod authn;
mod authz;
mod config;
mod db;
mod factory;
mod handlers;
mod now;
mod recycle;
mod response;
mod restful;

use std::process;

use anyhow::Result;
use clap::Parser;
use config::ServerConfig;
use csync_misc::config::CommonConfig;
use csync_misc::display::display_json;
use csync_misc::types::cmd::{ConfigArgs, LogArgs};
use factory::ServerFactory;
use log::info;
use restful::RestfulServer;

#[derive(Parser, Debug)]
#[command(author, version = env!("CSYNC_VERSION"), about)]
struct ServerArgs {
    /// Print server configuration data (JSON) and exit.
    #[arg(long)]
    pub print_config: bool,

    #[command(flatten)]
    pub config: ConfigArgs,

    #[command(flatten)]
    pub log: LogArgs,
}

async fn build_server(args: ServerArgs) -> Result<RestfulServer> {
    args.log.init()?;
    let ps = args.config.build_path_set()?;
    let cfg: ServerConfig = ps.load_config("server", ServerConfig::default)?;
    if args.print_config {
        display_json(&cfg)?;
        process::exit(0);
    }

    let factory = ServerFactory::new(cfg)?;

    let recycler = factory.build_recycler()?;
    if let Some(recycler) = recycler {
        recycler.start();
    }

    let srv = factory.build_server()?;
    Ok(srv)
}

#[tokio::main]
async fn main() {
    let args = ServerArgs::parse();

    let srv = match build_server(args).await {
        Ok(srv) => srv,
        Err(e) => {
            eprintln!("Failed to build server: {:#}", e);
            process::exit(2);
        }
    };

    match srv.run().await {
        Ok(_) => info!("Server exited successfully"),
        Err(e) => {
            eprintln!("Failed to run server: {:#}", e);
            process::exit(1);
        }
    }
}
