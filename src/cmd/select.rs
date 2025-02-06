use std::io::{Read, Write};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use clap::Args;

use crate::client::factory::ClientFactory;
use crate::client::Client;
use crate::clipboard::Clipboard;
use crate::config::CommonConfig;
use crate::daemon::client::DaemonClient;
use crate::daemon::config::DaemonConfig;
use crate::humanize::human_bytes;
use crate::types::text::truncate_text;

use super::{ConfigArgs, QueryArgs, RunCommand};

/// Read the latest text content list and use an external command (e.g., fzf) for searching and selecting.
/// The selected result can be processed in different ways (clipboard, daemon, or stdout).
#[derive(Args)]
pub struct SelectArgs {
    /// The external selection command to execute. Text content will be written to stdin,
    /// and results will be read from stdout. stderr is inherited directly.
    /// Each candidate will be written as a line to stdin, truncated according to --length option.
    #[arg(short, long, default_value = "fzf")]
    pub cmd: String,

    /// Maximum length of displayed text. Text longer than this will be truncated
    #[arg(short, long, default_value = "100")]
    pub length: usize,

    /// Send the selected text to the daemon server
    #[arg(short, long)]
    pub daemon: bool,

    /// Write the selected text to clipboard
    #[arg(long)]
    pub cb: bool,

    /// Don't trim the trailing newline from the selection command output
    #[arg(long)]
    pub no_trim_break: bool,

    #[command(flatten)]
    pub query: QueryArgs,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for SelectArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let text = self.select_text(client).await?;

        if self.daemon {
            let cfg: DaemonConfig = ps.load_config("daemon", DaemonConfig::default)?;
            let client = DaemonClient::new(cfg.port);

            let data = text.into_bytes();
            let size = human_bytes(data.len() as u64);
            client
                .send_data(data)
                .await
                .context("send data to daemon")?;
            println!("Send {size} data to daemon server");
            return Ok(());
        }

        if self.cb {
            let cb = Clipboard::load()?;
            let size = human_bytes(text.len() as u64);
            cb.write_text(text).context("write text to clipboard")?;
            println!("Write {size} text to clipboard");
            return Ok(());
        }

        print!("{text}");
        Ok(())
    }
}

impl SelectArgs {
    const MAX_LENGTH: usize = 200;

    async fn select_text(&self, client: Client) -> Result<String> {
        if self.length == 0 {
            bail!("Length must be greater than 0");
        }
        if self.length > Self::MAX_LENGTH {
            bail!("Length must be less than {}", Self::MAX_LENGTH);
        }

        let mut texts = client.read_texts(self.query.build_query()?).await?;

        if texts.is_empty() {
            bail!("No text found");
        }

        let mut items = Vec::with_capacity(texts.len());
        for text in texts.iter() {
            let line = truncate_text(text.content.clone().unwrap(), self.length);
            items.push(line);
        }

        let idx = self.execute_cmd(&items)?;
        let text = texts.remove(idx).content.unwrap();
        Ok(text)
    }

    fn execute_cmd(&self, items: &[String]) -> Result<usize> {
        let mut input = String::with_capacity(items.len());
        for item in items.iter() {
            input.push_str(item);
            input.push('\n');
        }

        let mut c = Command::new("bash");
        c.args(["-c", &self.cmd]);

        c.stdin(Stdio::piped());
        c.stdout(Stdio::piped());
        c.stderr(Stdio::inherit());

        let mut child = c.spawn().context("launch select command")?;

        let handle = child.stdin.as_mut().unwrap();
        if let Err(err) = write!(handle, "{}", input) {
            return Err(err).context("write data to select command");
        }

        drop(child.stdin.take());

        let mut stdout = child.stdout.take().unwrap();

        let mut out = String::new();
        stdout
            .read_to_string(&mut out)
            .context("read select command output")?;

        let result = if self.no_trim_break {
            &out
        } else {
            if out.is_empty() {
                bail!("select command output is empty");
            }
            &out[..out.len() - 1]
        };

        let status = child.wait().context("wait select command done")?;
        match status.code() {
            Some(0) => match items.iter().position(|s| s == result) {
                Some(idx) => Ok(idx),
                None => bail!("could not find item '{result}'"),
            },
            Some(code) => bail!("select command exited with code {code}"),
            None => bail!("select command  returned an unknown error"),
        }
    }
}
