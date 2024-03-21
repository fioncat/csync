mod cmd;
mod config;
mod net;
mod utils;

use std::{env, process};

use anyhow::{bail, Result};
use clap::Parser;
use log::error;

use crate::cmd::App;
use crate::net::client::{SendClient, WatchClient};
use crate::net::frame::{DataFrame, DataFrameInfo};
use crate::net::server;

fn init_log() {
    let log_env = env_logger::Env::default()
        .filter_or("CSYNC_LOG_LEVEL", "info")
        .write_style_or("CSYNC_LOG_STYLE", "always");
    env_logger::Builder::from_env(log_env)
        .format_timestamp_millis()
        .format_module_path(false)
        .format_target(false)
        .init();
}

async fn _debug(args: Vec<String>) -> Result<()> {
    const TEST_PASSWORD: &str = "test password 123";

    if args.is_empty() {
        bail!("missing debug action");
    }

    let act = args.first().unwrap();
    let args = args[1..].to_vec();

    match act.as_str() {
        "serve" => _debug_server(None).await,
        "serve-auth" => _debug_server(Some(TEST_PASSWORD)).await,
        "send" => _debug_send_text(args, None).await,
        "send-auth" => _debug_send_text(args, Some(TEST_PASSWORD)).await,
        "watch" => _debug_watch(args, None).await,
        "watch-auth" => _debug_watch(args, Some(TEST_PASSWORD)).await,
        _ => bail!("unknown debug '{act}'"),
    }
}

async fn _debug_server(password: Option<&str>) -> Result<()> {
    server::start(String::from("127.0.0.1:7703"), password).await
}

async fn _debug_send_text(args: Vec<String>, password: Option<&str>) -> Result<()> {
    if args.len() < 2 {
        bail!("require content to send");
    }
    let device = args.first().unwrap();
    let text = args.get(1).unwrap();

    let mut client = SendClient::connect("127.0.0.1:7703", &device, password).await?;
    client
        .send(&DataFrame {
            info: DataFrameInfo {
                device: Some(device.to_string()),
                digest: String::from("fake digest"),
                file: None,
            },
            body: text.as_bytes().to_vec(),
        })
        .await?;

    Ok(())
}

async fn _debug_watch(args: Vec<String>, password: Option<&str>) -> Result<()> {
    if args.is_empty() {
        bail!("require watch devices");
    }
    let devices = args;

    println!("Begin to sub {:?}", devices);
    let mut client = WatchClient::connect("127.0.0.1:7703", &devices, password)
        .await
        .unwrap();

    loop {
        let data = client.recv().await.unwrap();
        let from = data
            .info
            .device
            .clone()
            .unwrap_or(String::from("<unknown>"));
        let data_size = data.body.len();
        let str = match String::from_utf8(data.body) {
            Ok(s) => s,
            Err(_) => format!("<{data_size} binary data>"),
        };

        println!("From '{from}': {str}");
        println!();
    }
}

async fn _main() -> Result<()> {
    init_log();

    let args: Vec<_> = env::args().collect();
    if let Some(act) = args.get(1) {
        if act == "_debug" {
            return _debug(args[2..].to_vec()).await;
        }
    }

    let app = App::parse();
    app.run().await
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(err) = _main().await {
        error!("Fatal: {:#}", err);
        process::exit(1);
    }

    Ok(())
}
