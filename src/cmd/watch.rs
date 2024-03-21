use std::fs::File;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

use anyhow::{Context, Result};
use clap::Args;
use log::info;

use crate::config::Config;
use crate::net::client::WatchClient;
use crate::net::frame::DataFrame;
use crate::utils::Cmd;

/// Watch and receive content from server
#[derive(Args)]
pub struct WatchArgs {
    /// The config file to use. Default is `~/.config/csync.toml`
    #[clap(long, short)]
    pub config: Option<String>,

    /// The command to execute when recive data from server, the data will be sent to
    /// the stdin of this command.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub exec: Vec<String>,
}

impl WatchArgs {
    pub async fn run(&self) -> Result<()> {
        let cfg = Config::load(self.config.as_deref()).context("load config")?;

        let addr = cfg.get_server();
        let devices = cfg.get_watch()?;
        let password = cfg.get_password();
        let download_dir = cfg.get_download_dir()?;

        let mut watch_client = WatchClient::connect(addr, devices, password)
            .await
            .context("connect to server")?;

        loop {
            let frame = watch_client.recv().await.context("recv data from server")?;
            let data_size = frame.body.len();

            let from = frame.info.device.clone().unwrap_or_default();
            info!(
                "Receive {} data from server, from '{}', is_file: {}",
                frame.body.len(),
                from,
                frame.info.file.is_some(),
            );

            if let Some(file_info) = frame.info.file.as_ref() {
                let path = download_dir.join(&file_info.name);
                let mut opts = File::options();
                opts.create(true).truncate(true).write(true);

                // TODO: Support Windows
                opts.mode(file_info.mode as u32);

                println!(
                    "From '{from}': Download {data_size} data to file {}",
                    path.display()
                );
                let mut file = opts
                    .open(&path)
                    .with_context(|| format!("open file to download '{}'", path.display()))?;
                file.write_all(&frame.body)
                    .with_context(|| format!("write data to file '{}'", path.display()))?;
                drop(file);
                println!();

                continue;
            }

            if !self.exec.is_empty() {
                self.execute_cmd(frame).context("execute command")?;
                continue;
            }

            let show =
                String::from_utf8(frame.body).unwrap_or(format!("<{data_size} binary data>"));
            println!("From '{from}': {show}");
            println!();
        }
    }

    fn execute_cmd(&self, frame: DataFrame) -> Result<()> {
        let DataFrame { info, body } = frame;
        let data_len = body.len();

        println!(
            "From '{}': Send {data_len} data to command",
            info.device.unwrap_or_default()
        );

        let mut cmd = Cmd::new(&self.exec, Some(body), false);
        cmd.execute()?;
        println!();

        Ok(())
    }
}
