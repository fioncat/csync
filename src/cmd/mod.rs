mod send;
mod serve;
mod start;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version = env!("CSYNC_VERSION"), about)]
pub struct App {
    #[command(subcommand)]
    pub commands: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Start(start::StartArgs),
    Serve(serve::ServeArgs),
    Send(send::SendArgs),
}

impl App {
    pub async fn run(&self) -> Result<()> {
        match &self.commands {
            Commands::Start(args) => args.run().await,
            Commands::Serve(args) => args.run().await,
            Commands::Send(args) => args.run().await,
        }
    }
}
