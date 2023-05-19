use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpSocket;

use crate::config;
use crate::sync::Packet;

use log::info;

pub async fn send(cfg: &config::Config, packet: &Packet) -> Result<()> {
    let data = packet.encode().context("Encode packet")?;

    for addr in &cfg.targets {
        let socket = if addr.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        }
        .context("Create tcp socket")?;

        let mut stream = socket
            .connect(addr.clone())
            .await
            .with_context(|| format!(r#"Connect to "{}""#, addr))?;
        stream
            .write_all(&data)
            .await
            .with_context(|| format!(r#"Write to "{}""#, addr))?;

        info!(r#"Send {} to "{}""#, packet, addr);
    }
    Ok(())
}
