use anyhow::bail;
use anyhow::Result;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpSocket;

use crate::config;
use crate::sync::Packet;

use log::info;

pub async fn send(cfg: &config::Config, packet: &Packet) -> Result<()> {
    let data = packet.encode();
    if let Err(err) = data {
        bail!("unable to encode packet: {}", err)
    }
    let data = data.unwrap();

    for addr in &cfg.targets {
        let socket = if addr.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        };
        if let Err(err) = socket {
            bail!("failed to create tcp socket: {}", err)
        }
        let socket = socket.unwrap();
        let stream = socket.connect(addr.clone()).await;
        if let Err(err) = stream {
            bail!("failed to connect to \"{}\": {}", addr, err)
        }
        let mut stream = stream.unwrap();
        if let Err(err) = stream.write_all(&data).await {
            bail!("failed to write data to \"{}\": {}", addr, err)
        }

        info!("Send {} to {}", packet, addr);
    }
    Ok(())
}
