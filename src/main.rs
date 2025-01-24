mod client;
mod clipboard;
mod cmd;
mod config;
mod daemon;
mod dirs;
mod display;
mod filelock;
mod humanize;
mod imghdr;
mod logs;
mod rsa;
mod secret;
mod server;
mod table;
mod time;
#[cfg(feature = "tray")]
mod tray;
mod types;

use std::process;

use anyhow::Result;
use clap::error::ErrorKind as ArgsErrorKind;
use clap::Parser;
use cmd::{App, Commands, RunCommand, ServerCommand};
use log::{error, info};
use types::server::Server;

async fn run_cmd() -> Result<Option<Server>> {
    let app = match App::try_parse() {
        Ok(app) => app,
        Err(err) => {
            err.use_stderr();
            err.print().expect("write help message to stderr");
            if matches!(
                err.kind(),
                ArgsErrorKind::DisplayHelp
                    | ArgsErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
                    | ArgsErrorKind::DisplayVersion
            ) {
                return Ok(None);
            }
            process::exit(3);
        }
    };

    if let Commands::Server(ref args) = app.command {
        let srv = args.build_server().await?;
        return Ok(Some(srv));
    }

    if let Commands::Daemon(ref args) = app.command {
        let srv = args.build_server().await?;
        return Ok(Some(srv));
    }

    app.run().await.map(|_| None)
}

#[tokio::main]
async fn main() {
    match run_cmd().await {
        Ok(Some(srv)) => match srv.run().await {
            Ok(_) => {
                info!("Server exited successfully");
            }
            Err(e) => {
                error!("Run server failed with fatal error: {e:#}, exit now");
                process::exit(2);
            }
        },
        Ok(None) => {}
        Err(e) => {
            eprintln!("Command error: {e:#}");
            process::exit(1);
        }
    }
}
