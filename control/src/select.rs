use std::io::{Read, Write};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use clap::Args;
use csync_misc::api::metadata::{BlobType, GetMetadataRequest};
use csync_misc::api::QueryRequest;
use csync_misc::client::config::ClientConfig;
use csync_misc::client::restful::RestfulClient;
use csync_misc::clipboard::Clipboard;
use csync_misc::config::ConfigArgs;

use super::RunCommand;

/// Read the latest blobs and use an external command (e.g., fzf) for searching and selecting.
/// The selected result can be processed in different ways (clipboard, daemon, or stdout).
#[derive(Args)]
pub struct SelectArgs {
    /// The external selection command to execute. Text content will be written to stdin,
    /// and results will be read from stdout. stderr is inherited directly.
    #[arg(short, long, default_value = "fzf")]
    pub run: String,

    /// Send the selected text to the daemon server
    #[arg(short, long)]
    pub daemon: bool,

    /// Write the selected text to clipboard
    #[arg(short, long)]
    pub clipboard: bool,

    /// The query limit
    #[arg(short, long, default_value = "20")]
    pub limit: u64,

    /// Don't trim the trailing newline from the selection command output
    #[arg(long)]
    pub no_trim_break: bool,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for SelectArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;
        let cfg: ClientConfig = self.config.load_from_path_set("client", &ps)?;
        let mut client = cfg.connect_restful(false).await?;

        let id = self.select_blob(&mut client).await?;
        let blob = client.get_blob(id).await?;

        if !matches!(blob.blob_type, BlobType::File) {
            if self.daemon {
                let mut daemon = cfg.connect_daemon().await?;
                daemon.send(&blob.data).await?;

                return Ok(());
            }
            if self.clipboard {
                let cb = Clipboard::load()?;
                match blob.blob_type {
                    BlobType::Text => {
                        let text = String::from_utf8(blob.data)?;
                        cb.write_text(text)?;
                    }
                    BlobType::Image => {
                        cb.write_image(blob.data)?;
                    }
                    _ => unreachable!(),
                }

                return Ok(());
            }
        }

        blob.write(&ps)?;
        Ok(())
    }
}

impl SelectArgs {
    async fn select_blob(&self, client: &mut RestfulClient) -> Result<u64> {
        if self.limit == 0 {
            bail!("limit must be greater than 0");
        }

        let list = client
            .get_metadatas(GetMetadataRequest {
                query: QueryRequest {
                    limit: Some(self.limit),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await?;
        if list.items.is_empty() {
            bail!("no blobs to select");
        }

        let mut items = Vec::with_capacity(list.items.len());
        let mut ids = Vec::with_capacity(list.items.len());
        for item in list.items {
            items.push(item.summary);
            ids.push(item.id);
        }

        let idx = self.execute_cmd(&items)?;
        Ok(ids[idx])
    }

    fn execute_cmd(&self, items: &[String]) -> Result<usize> {
        let mut input = String::with_capacity(items.len());
        for item in items.iter() {
            input.push_str(item);
            input.push('\n');
        }

        let mut c = Command::new("bash");
        c.args(["-c", &self.run]);

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
