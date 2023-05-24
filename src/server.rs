use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use log::debug;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc::Sender, Semaphore};
use tokio::time::{self, Duration};

use crate::net::{Connection, Frame};

use log::{error, info};

/// The main csync server. It includes a `run` method which performs the TCP
/// listening and initialization of per-connection state.
pub struct Server {
    /// TCP listener supplied by the `run` caller.
    listener: TcpListener,

    /// Limit the max number of connections.
    ///
    /// A `Semaphore` is used to limit the max number of connections. Before
    /// attempting to accept a new connection, a permit is acquired from the
    /// semaphore. If none are available, the listener waits for one.
    ///
    /// When handlers complete processing a connection, the permit is returned
    /// to the semaphore.
    conn_limit: Arc<Semaphore>,

    /// Use to send synchronization requests to clipboard synchronizer.
    sender: Sender<Frame>,

    /// The server bind address.
    bind: SocketAddr,
}

impl Server {
    const ACCEPT_TCP_MAX_BACKOFF: u64 = 64;

    pub async fn new(bind: &SocketAddr, sender: Sender<Frame>, max_conn: usize) -> Result<Server> {
        let listener = TcpListener::bind(bind)
            .await
            .with_context(|| format!(r#"Bind "{}""#, bind))?;

        let conn_limit = Arc::new(Semaphore::new(max_conn));

        Ok(Server {
            listener,
            conn_limit,
            sender,
            bind: bind.clone(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Start to listen `{}`", self.bind);
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

            // Accept a new socket. This will attempt to perform error handling.
            // The `accept` method internally attempts to recover errors, so an
            // error here is non-recoverable.
            let (socket, addr) = self.accept().await?;

            // The `mpsc` channel sender can be cloned for each task.
            let sender = self.sender.clone();

            tokio::spawn(async move {
                debug!("Accpect connection from {addr}");
                if let Err(err) = Self::handle(sender, socket, addr).await {
                    error!("Handle socket error: {err:#}");
                }
                // Move the permit into the task and drop it after completion.
                // This returns the permit back to the semaphore.
                drop(permit);
            });
        }
    }

    async fn accept(&mut self) -> Result<(TcpStream, SocketAddr)> {
        let mut backoff = 1;

        loop {
            match self.listener.accept().await {
                Ok((socket, addr)) => return Ok((socket, addr)),
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

    async fn handle(sender: Sender<Frame>, socket: TcpStream, addr: SocketAddr) -> Result<()> {
        let mut conn = Connection::new(socket);
        loop {
            let frame = conn.read_frame().await?;

            // If `None` is returned from `read_frame()` then the peer closed
            // the socket. There is no further work to do and the task can be
            // terminated.
            let frame = match frame {
                Some(frame) => frame,
                None => {
                    debug!("Connection {addr} closed");
                    return Ok(());
                }
            };

            debug!("Recv {frame} from {addr}");
            sender.send(frame).await.context("Send frame to channel")?;
        }
    }
}
