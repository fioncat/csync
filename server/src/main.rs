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
mod revision;

use std::process;

use anyhow::Result;
use clap::Parser;
use config::ServerConfig;
use csync_misc::config::{CommonConfig, ConfigArgs};
use csync_misc::display::display_json;
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
}

async fn build_server(args: ServerArgs) -> Result<RestfulServer> {
    let ps = args.config.build_path_set()?;
    let cfg: ServerConfig = ps.load_config("server", ServerConfig::default)?;
    if args.print_config {
        display_json(&cfg)?;
        process::exit(0);
    }

    ps.init_logger("server", &cfg.log)?;

    let factory = ServerFactory::new(cfg)?;

    let revision = factory.build_revision()?;

    let recycler = factory.build_recycler(revision.clone())?;
    if let Some(recycler) = recycler {
        recycler.start();
    }

    let srv = factory.build_server(revision)?;
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
