use std::net::SocketAddr;

use anyhow::bail;
use anyhow::{Context, Result};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::{self, sync::mpsc};

use log::{error, info};

use crate::config;
use crate::sync::Packet;

pub async fn start(cfg: &config::Config, sender: mpsc::Sender<Packet>) -> Result<()> {
    let listener = TcpListener::bind(&cfg.bind).await;
    if let Err(err) = listener {
        bail!(r#"Bind "{}" error: {}"#, cfg.bind, err);
    }
    let listener = listener.unwrap();

    info!(r#"Begin to listen on "{}""#, cfg.bind);
    loop {
        match listener.accept().await {
            Ok((client, addr)) => {
                let sender = sender.clone();
                tokio::spawn(async move {
                    if let Err(err) = handle(client, addr, sender).await {
                        error!("Handle error: {:#}", err);
                    }
                });
            }
            Err(err) => error!("Accept connection error: {:#}", err),
        }
    }
}

async fn handle(
    mut client: TcpStream,
    addr: SocketAddr,
    sender: mpsc::Sender<Packet>,
) -> Result<()> {
    let mut raw_data = Vec::with_capacity(512);
    client
        .read_to_end(&mut raw_data)
        .await
        .context("read data from client")?;

    let packet = Packet::decode(&raw_data).context("decode data")?;

    info!("Recv {} from {}", packet, addr);
    sender
        .send(packet)
        .await
        .context("send packet to channel")?;
    Ok(())
}
