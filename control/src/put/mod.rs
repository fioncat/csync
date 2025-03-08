mod blob;
mod user;

use anyhow::Result;
use async_trait::async_trait;
use clap::{Args, Subcommand};

use super::RunCommand;

/// Put commands
#[derive(Args)]
pub struct PutCommand {
    #[command(subcommand)]
    pub command: PutCommands,
}

#[derive(Subcommand)]
pub enum PutCommands {
    User(user::UserArgs),
    Blob(blob::BlobArgs),
}

#[async_trait]
impl RunCommand for PutCommand {
    async fn run(&self) -> Result<()> {
        match &self.command {
            PutCommands::User(args) => args.run().await,
            PutCommands::Blob(args) => args.run().await,
        }
    }
}
