mod client;
mod config;
mod server;
mod sync;

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::prelude::PermissionsExt;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};

use arboard::Clipboard;
use clap::Parser;
use config::Arg;
use log::info;
use tokio::{self, sync::mpsc};

use crate::sync::{FileData, Packet};

async fn run() -> Result<()> {
    let env = env_logger::Env::default()
        .filter_or("CSYNC_LOG_LEVEL", "info")
        .write_style_or("CSYNC_LOG_STYLE", "always");
    env_logger::Builder::from_env(env)
        .format_timestamp_millis()
        .format_target(false)
        .format_module_path(false)
        .init();

    let mut arg = Arg::parse();
    let cfg = arg.normalize()?;
    if let Some(file) = arg.file.as_ref() {
        return send_file(&cfg, file).await;
    }
    info!("{:?}", cfg);

    let cb = Clipboard::new().context("Could not init clipboard driver")?;

    let (sender, receiver) = mpsc::channel::<sync::Packet>(sync::CHANNEL_SIZE);
    tokio::spawn(sync::start(cfg.clone(), cb, receiver));

    server::start(&cfg, sender).await?;
    Ok(())
}

async fn send_file(cfg: &config::Config, path: &String) -> Result<()> {
    let path = PathBuf::from(path);
    let file_name = match path.file_name() {
        Some(name) => match name.to_str() {
            Some(s) => s.to_string(),
            None => bail!("Invalid file name"),
        },
        None => bail!(r#"Invalid path "{}""#, path.display()),
    };

    let meta = fs::metadata(&path)
        .with_context(|| format!(r#"Get meta for file "{}""#, path.display()))?;
    if meta.is_dir() {
        bail!(r#""{}" is a directory"#, path.display());
    }
    let mode = meta.permissions().mode();
    let mut file =
        File::open(&path).with_context(|| format!(r#"Open file "{}""#, path.display()))?;

    let mut data: Vec<u8> = Vec::with_capacity(512);
    file.read_to_end(&mut data)
        .with_context(|| format!(r#"Read file "{}""#, path.display()))?;

    let packet = Packet {
        file: Some(FileData {
            name: file_name,
            mode,
            data,
        }),
        image: None,
        text: None,
    };

    client::send(cfg, &packet).await
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            _ = writeln!(io::stderr(), "Fatal: {:#}", err);
            ExitCode::FAILURE
        }
    }
}
