use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::factory::ClientFactory;
use csync_misc::clipboard::Clipboard;
use csync_misc::config::ConfigArgs;
use csync_misc::humanize::human_bytes;

use crate::RunCommand;

/// This is a generic put command that reads data from the clipboard and pushes it to the
/// server. The command will automatically determine if the clipboard data is text or an
/// image and call the corresponding put api.
#[derive(Args)]
pub struct CbArgs {
    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for CbArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let cb = Clipboard::load().context("load clipboard driver")?;

        if cb.is_image()? {
            let image = cb.read_image().context("read image")?;
            let data = match image {
                Some(data) => data,
                None => bail!("no image data in clipboard"),
            };
            let size = human_bytes(data.len() as u64);
            client.put_image(data).await?;
            println!("Send {size} image data to server");
        } else if cb.is_text()? {
            let text = cb.read_text().context("read text")?;
            let text = match text {
                Some(text) => text,
                None => bail!("no text data in clipboard"),
            };
            let size = human_bytes(text.len() as u64);
            client.put_text(text).await?;
            println!("Send {size} text data to server");
        } else {
            bail!("clipboard does not contain image or text data");
        }

        Ok(())
    }
}
