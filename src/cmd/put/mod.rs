mod cb;
mod file;
mod image;
mod role;
mod text;
mod user;

use anyhow::Result;
use async_trait::async_trait;
use clap::{Args, Subcommand};

use super::RunCommand;

/// Perform a put operation on the resource; this command can be used for create and update.
#[derive(Args)]
pub struct PutCommand {
    #[command(subcommand)]
    pub command: PutCommands,
}

#[derive(Subcommand)]
pub enum PutCommands {
    User(user::UserArgs),
    Role(role::RoleArgs),
    Text(text::TextArgs),
    Image(image::ImageArgs),
    File(file::FileArgs),
    Cb(cb::CbArgs),
}

#[async_trait]
impl RunCommand for PutCommand {
    async fn run(&self) -> Result<()> {
        match &self.command {
            PutCommands::User(args) => args.run().await,
            PutCommands::Role(args) => args.run().await,
            PutCommands::Text(args) => args.run().await,
            PutCommands::Image(args) => args.run().await,
            PutCommands::File(args) => args.run().await,
            PutCommands::Cb(args) => args.run().await,
        }
    }
}
