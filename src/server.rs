use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio::sync::Semaphore;
use tokio::time;
use tokio::time::Duration;

use crate::config::Config;
use crate::net::Connection;
use crate::net::Frame;

use log::{error, info};

pub struct Server {
    listener: TcpListener,
    conn_limit: Arc<Semaphore>,

    sender: Sender<Frame>,
}

impl Server {
    const ACCEPT_TCP_MAX_BACKOFF: u64 = 64;

    pub async fn new(cfg: &Config, sender: Sender<Frame>) -> Result<Server> {
        let listener = TcpListener::bind(&cfg.bind)
            .await
            .with_context(|| format!(r#"Bind "{}""#, cfg.bind))?;

        let max_conn = cfg.conn_max as usize;
        let conn_limit = Arc::new(Semaphore::new(max_conn));

        Ok(Server {
            listener,
            conn_limit,
            sender,
        })
    }

    pub async fn run(&mut self, cfg: &Config) -> Result<()> {
        info!(r#"Start to listen "{}""#, cfg.bind);
        loop {
            // Wait for a permit to become available
            //
            // `acquire_owned` returns a permit that is bound to the semaphore.
            // When the permit value is dropped, it is automatically returned
            // to the semaphore.
            //
            // `acquire_owned()` returns `Err` when the semaphore has been
            // closed. We don't ever close the semaphore, so `unwrap()` is safe.
            let permit = self.conn_limit.clone().acquire_owned().await.unwrap();

            let socket = self.accept().await?;

            let sender = self.sender.clone();

            tokio::spawn(async move {
                if let Err(err) = Self::handle(sender, socket).await {
                    error!("Handle socket error: {err:#}");
                }
                // Move the permit into the task and drop it after completion.
                // This returns the permit back to the semaphore.
                drop(permit);
            });
        }
    }

    async fn accept(&mut self) -> Result<TcpStream> {
        let mut backoff = 1;

        loop {
            match self.listener.accept().await {
                Ok((socket, _)) => return Ok(socket),
                Err(err) => {
                    if backoff > Self::ACCEPT_TCP_MAX_BACKOFF {
                        return Err(err).context("Accept tcp socket exceeded max backoff");
                    }

                    error!("Accept tcp socket error: {err:#}, will retry after {backoff} seconds");
                    time::sleep(Duration::from_secs(backoff)).await;
                    backoff *= 2;
                }
            }
        }
    }

    async fn handle(sender: Sender<Frame>, socket: TcpStream) -> Result<()> {
        let mut conn = Connection::new(socket);
        loop {
            let frame = conn.read_frame().await?;

            // If `None` is returned from `read_frame()` then the peer closed
            // the socket. There is no further work to do and the task can be
            // terminated.
            let frame = match frame {
                Some(frame) => frame,
                None => return Ok(()),
            };

            sender.send(frame).await.context("Send frame to channel")?;
        }
    }
}
