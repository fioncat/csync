mod blob;
mod user;

use anyhow::Result;
use async_trait::async_trait;
use clap::{Args, Subcommand};

use super::RunCommand;

/// Delete a resource from the server
#[derive(Args)]
pub struct DeleteCommand {
    #[command(subcommand)]
    pub command: DeleteCommands,
}

#[derive(Subcommand)]
pub enum DeleteCommands {
    Blob(blob::BlobArgs),
    User(user::UserArgs),
}

#[async_trait]
impl RunCommand for DeleteCommand {
    async fn run(&self) -> Result<()> {
        match &self.command {
            DeleteCommands::Blob(args) => args.run().await,
            DeleteCommands::User(args) => args.run().await,
        }
    }
}
