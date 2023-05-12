use std::net::SocketAddr;

use anyhow::bail;
use anyhow::Result;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::{self, sync::mpsc};

use log::{error, info};

use crate::config;
use crate::sync::Packet;

pub async fn start(cfg: &config::Config, sender: mpsc::Sender<Packet>) -> Result<()> {
    let listener = TcpListener::bind(&cfg.bind).await;
    if let Err(err) = listener {
        bail!("unable to bind \"{}\": {}", cfg.bind, err)
    }
    let listener = listener.unwrap();

    info!("Begin to listen on \"{}\"", cfg.bind);
    loop {
        match listener.accept().await {
            Ok((client, addr)) => {
                let sender = sender.clone();
                tokio::spawn(async move {
                    if let Err(err) = handle(client, addr, sender).await {
                        error!("Handle error: {}", err);
                    }
                });
            }
            Err(err) => error!("Accept connection error: {}", err),
        }
    }
}

async fn handle(
    mut client: TcpStream,
    addr: SocketAddr,
    sender: mpsc::Sender<Packet>,
) -> Result<()> {
    let mut raw_data = Vec::with_capacity(512);
    if let Err(err) = client.read_to_end(&mut raw_data).await {
        bail!("failed to read data from client: {}", err)
    }

    let packet = Packet::decode(&raw_data);
    if let Err(err) = packet {
        bail!("failed to decode data: {}", err)
    }
    let packet = packet.unwrap();

    info!("Recv {} from {}", packet, addr);
    if let Err(err) = sender.send(packet).await {
        bail!("failed to send packet to inner channel: {}", err)
    }
    Ok(())
}
