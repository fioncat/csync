use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use clap::Args;

use crate::client::factory::ClientFactory;
use crate::client::Client;
use crate::clipboard::Clipboard;
use crate::cmd::{ConfigArgs, QueryArgs, RunCommand};
use crate::types::text::truncate_text;

/// Read text content from the server. This command allows reading multiple text contents
/// for further filtering and other operations.
#[derive(Args)]
pub struct TextArgs {
    /// Specify the text ID to read.
    pub id: Option<u64>,

    /// Write the text data to a file.
    #[arg(short, long)]
    pub file: Option<String>,

    /// Indicate reading multiple text contents, each occupying a line, formatted as
    /// "{ID}. {CONTENT}". The CONTENT will be truncated according to the --length option,
    /// and newline characters will be replaced with spaces. This output is generally
    /// used for other programs, such as fzf or rofi, for further filtering and selection.
    /// Refer to the documentation for specific usage regarding fzf and rofi integration.
    #[arg(short, long)]
    pub list: bool,

    /// When the -l option is specified, it indicates the text will be truncated if it
    /// exceeds a certain length.
    #[arg(long)]
    #[clap(default_value = "100")]
    pub length: usize,

    /// Write the text directly to the clipboard.
    #[arg(short, long)]
    pub cb: bool,

    /// Read "{ID}. {CONTENT}" content from stdin and download the complete original text
    /// content from the server. The content output by the -l option is specially processed
    /// and formatted for filtering purposes only. After filtering with fzf or rofi, this
    /// parameter is needed to restore the real content of the text for further operations.
    /// This option is generally used to read the output of other programs like fzf and rofi.
    /// Refer to the documentation for specific usage regarding fzf and rofi integration.
    #[arg(long)]
    pub from_selected_stdin: bool,

    #[command(flatten)]
    pub query: QueryArgs,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for TextArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        if self.list {
            return self.list(client).await;
        }

        let text = match self.id {
            Some(id) => client.read_text(id).await?,
            None => {
                if self.from_selected_stdin {
                    let id = self.get_id_from_selected_stdin()?;
                    client.read_text(id).await?
                } else {
                    client.read_latest_text().await?
                }
            }
        };
        let content = text.content.unwrap();

        if self.cb {
            let cb = Clipboard::load()?;
            cb.write_text(content).context("write text to clipboard")?;
            return Ok(());
        }

        print!("{content}");
        Ok(())
    }
}

impl TextArgs {
    const MAX_LENGTH: usize = 200;

    async fn list(&self, client: Client) -> Result<()> {
        if self.length == 0 {
            bail!("Length must be greater than 0");
        }
        if self.length > Self::MAX_LENGTH {
            bail!("Length must be less than {}", Self::MAX_LENGTH);
        }

        let texts = client.read_texts(self.query.build_query()?).await?;

        for text in texts {
            let id = text.id;
            let content = text.content.unwrap();
            let line = truncate_text(content, self.length);
            println!("{id}. {line}");
        }

        Ok(())
    }

    fn get_id_from_selected_stdin(&self) -> Result<u64> {
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .context("read stdin")?;

        let input = input.trim();

        let num_end = match input.find(|c: char| !c.is_ascii_digit()) {
            Some(num_end) => num_end,
            None => bail!("cannot find number in input"),
        };

        input[..num_end].parse().context("parse id number")
    }
}
