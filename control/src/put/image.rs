use std::fs;
use std::io::Read;

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::factory::ClientFactory;
use csync_misc::types::cmd::ConfigArgs;

use crate::RunCommand;

/// Upload an image to the server to provide synchronization for other devices. If the
/// daemon is running, this step will be performed automatically, and it is not recommended
/// to execute this command manually. The storage duration of images on the server is shorter
/// than that of text, so they should be used more quickly to prevent automatic deletion by
/// the server.
#[derive(Args)]
pub struct ImageArgs {
    /// Read an image from a file and upload it. If not provided, the image data is read
    /// from stdin by default.
    #[arg(short, long)]
    pub file: Option<String>,

    /// Delete the image after reading the image file and uploading it; this needs to be
    /// used in conjunction with the --file option.
    #[arg(short, long)]
    pub rm: bool,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for ImageArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let data = self.get_data()?;
        let image = client.put_image(data).await?;

        if self.rm {
            if let Some(ref file) = self.file {
                fs::remove_file(file).context("failed to remove file")?;
            }
        }

        println!("Image id {}", image.id);
        Ok(())
    }
}

impl ImageArgs {
    fn get_data(&self) -> Result<Vec<u8>> {
        if let Some(ref file) = self.file {
            return fs::read(file).context("failed to read file");
        }

        let mut input = Vec::new();
        std::io::stdin()
            .read_to_end(&mut input)
            .context("read stdin")?;

        Ok(input)
    }
}
