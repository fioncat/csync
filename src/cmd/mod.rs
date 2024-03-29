use anyhow::Result;
use clap::{Parser, Subcommand};

mod notify;
mod send;
mod serve;
mod watch;

#[derive(Parser)]
#[command(author, version = env!("CSYNC_VERSION"), about)]
pub struct App {
    #[command(subcommand)]
    pub commands: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Notify(notify::NotifyArgs),
    Send(send::SendArgs),
    Serve(serve::ServeArgs),
    Watch(watch::WatchArgs),
}

impl App {
    pub async fn run(&self) -> Result<()> {
        match &self.commands {
            Commands::Notify(args) => args.run().await,
            Commands::Send(args) => args.run().await,
            Commands::Serve(args) => args.run().await,
            Commands::Watch(args) => args.run().await,
        }
    }
}
