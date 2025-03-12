mod delete;
mod get;
mod patch;
mod put;
mod select;
mod state;
mod version;
mod whoami;

use std::process;

use anyhow::Result;
use async_trait::async_trait;
use clap::error::ErrorKind as ArgsErrorKind;
use clap::{Parser, Subcommand};

#[async_trait]
pub trait RunCommand {
    async fn run(&self) -> Result<()>;
}

#[derive(Parser)]
#[command(author, about, version = env!("CSYNC_VERSION"))]
pub struct App {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Delete(delete::DeleteCommand),
    Get(get::GetCommand),
    Patch(patch::PatchCommand),
    Put(put::PutCommand),
    Select(select::SelectArgs),
    State(state::StateArgs),
    Version(version::VersionArgs),
    Whoami(whoami::WhoamiArgs),
}

#[async_trait]
impl RunCommand for App {
    async fn run(&self) -> Result<()> {
        match &self.command {
            Commands::Delete(args) => args.run().await,
            Commands::Get(args) => args.run().await,
            Commands::Patch(args) => args.run().await,
            Commands::Put(args) => args.run().await,
            Commands::Select(args) => args.run().await,
            Commands::State(args) => args.run().await,
            Commands::Version(args) => args.run().await,
            Commands::Whoami(args) => args.run().await,
        }
    }
}

async fn run_cmd() -> Result<()> {
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
                return Ok(());
            }
            process::exit(3);
        }
    };

    app.run().await
}

#[tokio::main]
async fn main() {
    match run_cmd().await {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Command error: {e:#}");
            process::exit(1);
        }
    }
}
