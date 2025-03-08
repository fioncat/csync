mod blob;
mod user;

use anyhow::Result;
use async_trait::async_trait;
use clap::{Args, Subcommand};

use super::RunCommand;

/// Patch commands
#[derive(Args)]
pub struct PatchCommand {
    #[command(subcommand)]
    pub command: PatchCommands,
}

#[derive(Subcommand)]
pub enum PatchCommands {
    Blob(blob::BlobArgs),
    User(user::UserArgs),
}

#[async_trait]
impl RunCommand for PatchCommand {
    async fn run(&self) -> Result<()> {
        match &self.command {
            PatchCommands::Blob(args) => args.run().await,
            PatchCommands::User(args) => args.run().await,
        }
    }
}
