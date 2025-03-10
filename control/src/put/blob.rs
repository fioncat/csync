use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Args;
use csync_misc::api::blob::Blob;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;
use csync_misc::imghdr;

use crate::RunCommand;

/// Put a new blob to the server
#[derive(Args)]
pub struct BlobArgs {
    /// Put text to the server
    pub content: Option<String>,

    /// Read content from file
    #[arg(short, long)]
    pub file: Option<String>,

    /// Upload file to the server
    #[arg(short, long)]
    pub upload_file: bool,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for BlobArgs {
    async fn run(&self) -> Result<()> {
        let cfg: ClientConfig = self.config.load("client")?;
        let mut client = cfg.connect_restful(false).await?;

        let blob = self.get_blob()?;
        client.put_blob(blob).await?;

        Ok(())
    }
}

impl BlobArgs {
    fn get_blob(&self) -> Result<Blob> {
        if let Some(ref content) = self.content {
            return Ok(Blob::new_text(content.clone()));
        }

        let data = match self.file {
            Some(ref file) => {
                let path = PathBuf::from(file);
                if self.upload_file {
                    return Blob::read_from_file(&path);
                }

                fs::read(&path)?
            }
            None => self.read_stdin()?,
        };

        if imghdr::is_data_image(&data) {
            return Ok(Blob::new_image(data));
        }

        let s = String::from_utf8(data).context("invalid utf8 text")?;
        Ok(Blob::new_text(s))
    }

    fn read_stdin(&self) -> Result<Vec<u8>> {
        let mut content = Vec::new();
        std::io::stdin().read_to_end(&mut content)?;
        Ok(content)
    }
}
