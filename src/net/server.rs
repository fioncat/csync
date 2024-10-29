use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use log::{debug, error, info};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

use super::conn::Connection;
use super::frame::Frame;

use crate::hash::get_hash;

pub struct Server {
    addr: String,
    password: String,

    listener: TcpListener,

    write_tx: mpsc::Sender<WriteRequest>,
    read_tx: mpsc::Sender<ReadRequest>,
}

impl Server {
    pub async fn bind(addr: String, password: String) -> Result<Self> {
        let bind: SocketAddr = addr.parse().context("parse listen address")?;
        let listener = TcpListener::bind(&bind)
            .await
            .context("bind tcp listener")?;
        let (write_tx, read_tx) = ServerState::start();
        Ok(Self {
            addr,
            password,
            listener,
            write_tx,
            read_tx,
        })
    }

    pub async fn listen_and_serve(&self) -> Result<()> {
        info!(
            "[server] begin to accept connection on address '{}'",
            self.addr
        );
        loop {
            let (stream, addr) = match self.listener.accept().await {
                Ok((stream, addr)) => (stream, addr),
                Err(err) => {
                    error!("[server] accept tcp connection error: {err:#}");
                    continue;
                }
            };

            let (write_tx, read_tx) = (self.write_tx.clone(), self.read_tx.clone());
            let conn = Connection::new(stream, self.password.clone());
            let mut worker = ServerWorker {
                addr: addr.to_string(),
                conn,
                write_tx,
                read_tx,
                hash: Arc::new(String::new()),
            };
            tokio::spawn(async move {
                worker.main_loop().await;
            });
        }
    }
}

struct ServerWorker {
    addr: String,

    conn: Connection<TcpStream>,

    write_tx: mpsc::Sender<WriteRequest>,
    read_tx: mpsc::Sender<ReadRequest>,

    hash: Arc<String>,
}

impl ServerWorker {
    async fn main_loop(&mut self) {
        info!("[{}] start handling frame", self.addr);
        loop {
            let frame = match self.conn.read_frame().await {
                Ok(frame) => match frame {
                    Some(frame) => frame,
                    None => break,
                },
                Err(err) => {
                    error!("[{}] read frame from client error: {err:#}", self.addr);
                    break;
                }
            };
            let resp_frame = match frame {
                Frame::Post(data_frame) => {
                    debug!("[{}] get post request: {data_frame}", self.addr);
                    let hash = get_hash(&data_frame.data);
                    let (resp_tx, resp_rx) = oneshot::channel();
                    let req = WriteRequest {
                        frame: Arc::new(Frame::Post(data_frame)),
                        hash: hash.clone(),
                        resp: resp_tx,
                    };
                    self.write_tx.send(req).await.unwrap();
                    resp_rx.await.unwrap();
                    self.hash = Arc::new(hash);
                    Arc::new(Frame::Get)
                }
                Frame::Get => {
                    let hash = self.hash.clone();
                    let (resp_tx, resp_rx) = oneshot::channel();
                    let req = ReadRequest {
                        hash,
                        resp: resp_tx,
                    };
                    self.read_tx.send(req).await.unwrap();
                    let resp = resp_rx.await.unwrap();
                    match resp {
                        Some((frame, hash)) => {
                            debug!("[{}] send post response: {}", self.addr, frame.as_data());
                            self.hash = Arc::new(hash);
                            frame
                        }
                        None => Arc::new(Frame::Get),
                    }
                }
                Frame::Error(message) => {
                    error!(
                        "[{}] error from client: {message}, close connection",
                        self.addr
                    );
                    break;
                }
            };

            if let Err(err) = self.conn.write_frame(&resp_frame).await {
                error!("[{}] send data to client error: {err:#}", self.addr);
                break;
            }
        }

        info!("[{}] close connection", self.addr);
    }
}

struct ServerState {
    write_rx: mpsc::Receiver<WriteRequest>,
    read_rx: mpsc::Receiver<ReadRequest>,

    data: Option<Arc<Frame>>,
    hash: String,
}

struct WriteRequest {
    frame: Arc<Frame>,
    hash: String,
    resp: oneshot::Sender<()>,
}

struct ReadRequest {
    hash: Arc<String>,
    resp: oneshot::Sender<Option<(Arc<Frame>, String)>>,
}

impl ServerState {
    fn start() -> (mpsc::Sender<WriteRequest>, mpsc::Sender<ReadRequest>) {
        let (write_tx, write_rx) = mpsc::channel(500);
        let (read_tx, read_rx) = mpsc::channel(500);

        let mut state = ServerState {
            write_rx,
            read_rx,
            data: None,
            hash: String::new(),
        };
        tokio::spawn(async move {
            state.main_loop().await;
        });

        (write_tx, read_tx)
    }

    async fn main_loop(&mut self) {
        info!("[server] start state sync main loop");
        loop {
            tokio::select! {
                Some(req) = self.write_rx.recv() => {
                    self.handle_write(req);
                },
                Some(req) = self.read_rx.recv() => {
                    self.handle_read(req);
                },
            }
        }
    }

    fn handle_write(&mut self, req: WriteRequest) {
        let WriteRequest { frame, hash, resp } = req;

        self.data = Some(frame);
        self.hash = hash;

        resp.send(()).unwrap();
    }

    fn handle_read(&mut self, req: ReadRequest) {
        let ReadRequest { hash, resp } = req;
        let data = if self.hash == hash.as_str() {
            None
        } else {
            match self.data {
                Some(ref data) => Some((data.clone(), self.hash.clone())),
                None => None,
            }
        };
        resp.send(data).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    use crate::logs;

    /// Start test case:
    ///
    /// ```
    /// TEST_SERVER="true" cargo test net::server::tests::test_server -- --nocapture
    /// ```
    #[tokio::test]
    async fn test_server() {
        if env::var("TEST_SERVER").is_err() {
            return;
        }
        logs::init("debug").unwrap();
        let addr = String::from("127.0.0.1:9988");
        let password = String::from("password123");

        let server = Server::bind(addr, password).await.unwrap();
        server.listen_and_serve().await.unwrap();
    }
}
