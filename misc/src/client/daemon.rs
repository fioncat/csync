use anyhow::{Context, Result};
use tokio::net::TcpStream;

use crate::stream::Stream;

pub struct DaemonClient {
    stream: Stream<TcpStream>,
}

impl DaemonClient {
    pub async fn connect(port: u32) -> Result<Self> {
        let addr = format!("127.0.0.1:{port}");
        let stream = TcpStream::connect(addr)
            .await
            .context("connect to daemon server")?;

        Ok(Self {
            stream: Stream::new(stream),
        })
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.stream.write(data).await
    }
}
