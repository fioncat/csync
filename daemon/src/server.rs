use anyhow::{Context, Result};
use csync_misc::stream::Stream;
use log::{debug, error, info};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

pub struct DaemonServer {
    addr: String,
    copy_tx: mpsc::Sender<Vec<u8>>,
}

impl DaemonServer {
    pub fn new(addr: String, copy_tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self { addr, copy_tx }
    }

    pub async fn run(self) -> Result<()> {
        let listener = TcpListener::bind(&self.addr)
            .await
            .context("bind daemon address")?;

        info!("Begin to handle copy request for {}", self.addr);
        loop {
            let (socket, addr) = listener.accept().await.context("accept new connection")?;
            debug!("Accept new copy connection: {addr}");

            let mut stream = Stream::new(socket);
            let copy_tx = self.copy_tx.clone();
            tokio::spawn(async move {
                loop {
                    let data = match stream.next_raw().await {
                        Ok(Some(data)) => data,
                        Ok(None) => return,
                        Err(e) => {
                            error!("Failed to read data from copy connection: {e:#}");
                            return;
                        }
                    };
                    debug!("Received {} data from copy connection", data.len());
                    copy_tx.send(data).await.unwrap();
                }
            });
        }
    }
}
