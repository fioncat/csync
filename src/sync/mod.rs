use anyhow::{Context, Error, Result};
use tokio::select;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::net::client::{SendClient, WatchClient};
use crate::net::frame::{DataFrame, DataFrameInfo};

mod read;
mod write;

pub async fn start(mut cfg: Config) -> Result<()> {
    let mut current_device = Some(cfg.device.clone());

    let (err_tx, mut err_rx) = mpsc::channel::<Error>(512);

    let mut read_req = read::start(&mut cfg, err_tx.clone())?;
    let write_tx = write::start(&mut cfg, err_tx)?;

    let mut send_client = SendClient::connect(&cfg.server, &cfg.device, cfg.password.as_deref())
        .await
        .context("connect to send client")?;
    let mut watch_client = WatchClient::connect(&cfg.server, &cfg.watch, cfg.password.as_deref())
        .await
        .context("connect to write client")?;

    loop {
        select! {
            data_frame = watch_client.recv() => {
                let data_frame = data_frame.context("receive data from server")?;
                read_req.update_digest(data_frame.info.digest.clone()).await;
                write_tx.send(data_frame).await.unwrap();
            },

            Some((data, digest)) = read_req.recv_data() => {
                let data_frame = DataFrame{
                    body: data,
                    info: DataFrameInfo {
                        device: current_device.take(),
                        digest,
                        file: None,
                    },
                };
                send_client.send(&data_frame).await.context("send data to server")?;

                current_device = data_frame.info.device;
            },

            Some(err) = err_rx.recv() => {
                return Err(err).context("read write error");
            },
        }
    }
}
