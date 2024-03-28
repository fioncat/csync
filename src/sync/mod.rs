pub mod notify;

mod read;
mod write;

use std::sync::Arc;

use anyhow::{bail, Context, Error, Result};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::config::Config;
use crate::net::client::{SendClient, WatchClient};
use crate::net::frame::{DataFrame, DataFrameInfo};

pub async fn start(mut cfg: Config) -> Result<()> {
    let (err_tx, mut err_rx) = mpsc::channel::<Error>(512);
    let read_req = read::start(&mut cfg, err_tx.clone())?;
    let write_tx = write::start(&mut cfg, err_tx)?;

    if read_req.is_none() && cfg.watch.is_empty() {
        bail!("no data to send and no devices to watch, please check your config");
    }

    if read_req.is_none() {
        // No read clipboard, watch only
        return watch_only(cfg, write_tx, err_rx).await;
    }

    let mut read_req = read_req.unwrap();
    if cfg.watch.is_empty() {
        // No watch server, send only
        return send_only(cfg, read_req, err_rx).await;
    }

    let mut send_client = connect_send(&cfg).await?;
    let mut watch_client = connect_watch(&cfg).await?;

    let mut device = Some(cfg.device);

    loop {
        select! {
            data_frame = watch_client.recv() => {
                let data_frame = data_frame.context("receive data from server")?;
                read_req.update_digest(data_frame.info.digest.clone()).await;
                write_tx.send(data_frame).await.unwrap();
            },

            Some((data, digest)) = read_req.recv_data() => {
                send_data(&mut send_client, data, digest, &mut device).await?;
            },

            Some(err) = err_rx.recv() => {
                return Err(err);
            },
        }
    }
}

#[inline]
async fn send_only(
    cfg: Config,
    mut read_req: read::ReadRequest,
    mut err_rx: Receiver<Error>,
) -> Result<()> {
    let mut client = connect_send(&cfg).await?;
    let mut device = Some(cfg.device);
    loop {
        select! {
            Some((data, digest)) = read_req.recv_data() => {
                send_data(&mut client, data, digest, &mut device).await?;
            },
            Some(err) = err_rx.recv() => {
                return Err(err);
            },
        }
    }
}

#[inline]
async fn watch_only(
    cfg: Config,
    write_tx: Sender<DataFrame>,
    mut err_rx: Receiver<Error>,
) -> Result<()> {
    let mut client = connect_watch(&cfg).await?;
    loop {
        select! {
            data_frame = client.recv() => {
                let data_frame = data_frame.context("receive data from server")?;
                write_tx.send(data_frame).await.unwrap();
            },

            Some(err) = err_rx.recv() => {
                return Err(err);
            },
        }
    }
}

#[inline]
async fn connect_send(cfg: &Config) -> Result<SendClient<TcpStream>> {
    SendClient::connect(&cfg.server, &cfg.device, cfg.password.as_deref())
        .await
        .context("connect to send client")
}

#[inline]
async fn connect_watch(cfg: &Config) -> Result<WatchClient<TcpStream>> {
    WatchClient::connect(&cfg.server, &cfg.watch, cfg.password.as_deref())
        .await
        .context("connect to write client")
}

#[inline]
async fn send_data(
    client: &mut SendClient<TcpStream>,
    data: Vec<u8>,
    digest: String,
    device: &mut Option<String>,
) -> Result<()> {
    println!();
    println!("[Send {} data]", data.len());

    let data_frame = DataFrame {
        body: data,
        info: DataFrameInfo {
            device: device.take(),
            digest,
            file: None,
        },
    };
    let data_frame = Arc::new(data_frame);

    client
        .send(Arc::clone(&data_frame))
        .await
        .context("send data to server")?;

    *device = Arc::try_unwrap(data_frame).unwrap().info.device;

    Ok(())
}
