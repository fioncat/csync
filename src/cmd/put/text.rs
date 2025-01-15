use std::fs;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use clap::Args;

use crate::client::factory::ClientFactory;
use crate::cmd::{ConfigArgs, RunCommand};

/// Upload text to the server to provide synchronization for other devices. If the daemon is
/// running, this step will be performed automatically, and it is not recommended to execute
/// this command manually. Text has the longest storage duration on the server, but it will
/// still be automatically deleted after a certain period.
#[derive(Args)]
pub struct TextArgs {
    /// Specify the text content to upload.
    pub content: Option<String>,

    /// Read text from a file and upload it. If this option and the file are not specified,
    /// the text data will be read from stdin.
    #[arg(short, long)]
    pub file: Option<String>,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for TextArgs {
    async fn run(&self) -> Result<()> {
        let text = self.get_text()?;

        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let ret = client.put_text(text).await?;

        println!("Text id {}", ret.id);
        Ok(())
    }
}

impl TextArgs {
    fn get_text(&self) -> Result<String> {
        if let Some(ref content) = self.content {
            return Ok(content.clone());
        }

        if let Some(ref file) = self.file {
            return fs::read_to_string(file).context("failed to read file");
        }

        bail!("either content or file must be provided")
    }
}
