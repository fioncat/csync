mod serve;
mod start;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, about)]
pub struct App {
    #[command(subcommand)]
    pub commands: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Start(start::StartArgs),
    Serve(serve::ServeArgs),
}

impl App {
    pub async fn run(&self) -> Result<()> {
        match &self.commands {
            Commands::Start(args) => args.run().await,
            Commands::Serve(args) => args.run().await,
        }
    }
}
