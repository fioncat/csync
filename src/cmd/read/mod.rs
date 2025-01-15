mod file;
mod image;
mod text;

use anyhow::Result;
use async_trait::async_trait;
use clap::{Args, Subcommand};

use super::RunCommand;

/// Read the data of a resource. Unlike the get command, this command will actually read the
/// data, whereas get only reads metadata.
#[derive(Args)]
pub struct ReadCommand {
    #[command(subcommand)]
    pub command: ReadCommands,
}

#[derive(Subcommand)]
pub enum ReadCommands {
    Text(text::TextArgs),
    Image(image::ImageArgs),
    File(file::FileArgs),
}

#[async_trait]
impl RunCommand for ReadCommand {
    async fn run(&self) -> Result<()> {
        match &self.command {
            ReadCommands::Text(args) => args.run().await,
            ReadCommands::Image(args) => args.run().await,
            ReadCommands::File(args) => args.run().await,
        }
    }
}
