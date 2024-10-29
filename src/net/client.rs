use std::borrow::Cow;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use log::{debug, error, info};
use tokio::net::{lookup_host, TcpSocket, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{self, Instant, Timeout};

use super::conn::Connection;
use super::frame::{DataFrame, DataInfo, FileInfo, Frame};

pub struct Client {
    addr: SocketAddr,
    password: String,

    conn: Option<Connection<TcpStream>>,

    read_interval: u64,
}

pub enum DataItem {
    File(FileItem),
    Clipboard(Vec<u8>),
}

pub struct FileItem {
    pub name: String,
    pub mode: u64,
    pub data: Vec<u8>,
}

impl Client {
    const INIT_SEND_TIMEOUT: Duration = Duration::from_millis(200);
    const RETRY_SEND_TIMEOUT: Duration = Duration::from_secs(2);
    const RETRY_INETRVAL: Duration = Duration::from_secs(1);

    const SEND_MAX_RETRY: usize = 5;

    const MIN_READ_INTERVAL: u64 = 200;
    const MAX_READ_INTERVAL: u64 = 5000;

    pub async fn connect(addr: String, password: String, read_interval: u64) -> Result<Self> {
        let addr = parse_addr(addr).await?;
        if read_interval < Self::MIN_READ_INTERVAL {
            bail!(
                "client read interval should be at least {}ms",
                Self::MIN_READ_INTERVAL
            );
        }
        if read_interval > Self::MAX_READ_INTERVAL {
            bail!(
                "client read interval should be at most {}ms",
                Self::MAX_READ_INTERVAL
            );
        }
        Ok(Self {
            addr,
            password,
            conn: None,
            read_interval,
        })
    }

    pub async fn send_data(&mut self, data_item: DataItem) -> Result<()> {
        let data_frame = Self::convert_data_item(data_item);
        let frame = Frame::Post(data_frame);

        let mut conn = self.get_conn().await?;
        conn.write_frame(&frame).await?;
        conn.must_read_frame().await?;

        Ok(())
    }

    pub fn start(mut self) -> (mpsc::Receiver<DataItem>, mpsc::Sender<DataItem>) {
        let (watch_tx, watch_rx) = mpsc::channel::<DataItem>(500);
        let (write_tx, mut write_rx) = mpsc::channel::<DataItem>(500);

        let mut read_intv =
            tokio::time::interval_at(Instant::now(), Duration::from_millis(self.read_interval));

        tokio::spawn(async move {
            info!(
                "[client] start handling loop, with read interval {}ms",
                self.read_interval
            );
            loop {
                tokio::select! {
                    _ = read_intv.tick() => {
                        self.read_loop(&watch_tx).await;
                    },
                    Some(data_item) = write_rx.recv() => {
                        self.write_loop(data_item).await;
                    },
                }
            }
        });
        (watch_rx, write_tx)
    }

    async fn read_loop(&mut self, watch_tx: &mpsc::Sender<DataItem>) {
        if !self.send_frame(Frame::Get).await {
            return;
        };

        let frame = match self.read_frame().await {
            Some(frame) => frame,
            None => return,
        };

        if let Frame::Post(data_frame) = frame {
            debug!("[client] received post response from server: {data_frame}");
            let data_item = if let Some(file_info) = data_frame.info.file {
                DataItem::File(FileItem {
                    name: file_info.name,
                    mode: file_info.mode,
                    data: data_frame.data,
                })
            } else {
                DataItem::Clipboard(data_frame.data)
            };
            watch_tx.send(data_item).await.unwrap();
        }
    }

    async fn write_loop(&mut self, data_item: DataItem) {
        let data_frame = Self::convert_data_item(data_item);
        debug!("[client] send post request to server: {data_frame}");
        let frame = Frame::Post(data_frame);
        if !self.send_frame(frame).await {
            return;
        };

        self.read_frame().await;
    }

    fn convert_data_item(data_item: DataItem) -> DataFrame {
        match data_item {
            DataItem::File(file_item) => DataFrame {
                info: DataInfo {
                    file: Some(FileInfo {
                        name: file_item.name,
                        mode: file_item.mode,
                    }),
                },
                data: file_item.data,
            },
            DataItem::Clipboard(data) => DataFrame {
                info: DataInfo { file: None },
                data,
            },
        }
    }

    async fn send_frame(&mut self, frame: Frame) -> bool {
        let frame = Arc::new(frame);
        for retry in 0..Self::SEND_MAX_RETRY {
            let mut conn = match self.get_conn().await {
                Ok(conn) => conn,
                Err(err) => {
                    if retry > 0 {
                        error!(
                            "[client] get connection for sending error: {err:#}; retry: {retry}"
                        );
                        time::sleep(Self::RETRY_INETRVAL).await;
                    }
                    continue;
                }
            };
            let timeout = if retry == 0 {
                Self::INIT_SEND_TIMEOUT
            } else {
                Self::RETRY_SEND_TIMEOUT
            };

            let (done_tx, done_rx) = oneshot::channel::<Result<Connection<TcpStream>>>();
            let frame = Arc::clone(&frame);
            tokio::spawn(async move {
                let result = conn.write_frame(frame.as_ref()).await;
                let _ = match result {
                    Ok(_) => done_tx.send(Ok(conn)),
                    Err(err) => done_tx.send(Err(err)),
                };
            });

            let err_msg = match run_with_timeout(done_rx, timeout).await {
                Ok(result) => {
                    let result = result.unwrap();
                    match result {
                        Ok(conn) => {
                            self.conn = Some(conn);
                            return true;
                        }
                        Err(err) => Cow::Owned(format!("send frame error: {err:#}")),
                    }
                }
                Err(_) => Cow::Borrowed("send frame timeout"),
            };
            if retry > 0 {
                error!("[client] send frame to server error: {err_msg}; retry: {retry}");
            }
        }
        error!("[client] sending frame exceeded max retries, operation has not been completed");
        false
    }

    async fn read_frame(&mut self) -> Option<Frame> {
        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(err) => {
                error!("[client] get connection for reading error: {err:#}");
                return None;
            }
        };

        match conn.must_read_frame().await {
            Ok(frame) => {
                self.conn = Some(conn);
                Some(frame)
            }
            Err(err) => {
                error!("[client] read frame error: {err:#}");
                None
            }
        }
    }

    async fn get_conn(&mut self) -> Result<Connection<TcpStream>> {
        if let Some(conn) = self.conn.take() {
            return Ok(conn);
        }

        let socket = if self.addr.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        }
        .context("create tcp socket")?;

        let stream = socket
            .connect(self.addr)
            .await
            .context("connect to server")?;
        Ok(Connection::new(stream, self.password.clone()))
    }
}

async fn parse_addr<S: AsRef<str>>(addr: S) -> Result<SocketAddr> {
    if let Ok(addr) = addr.as_ref().parse::<SocketAddr>() {
        return Ok(addr);
    }

    let addrs: Vec<SocketAddr> = lookup_host(addr.as_ref())
        .await
        .with_context(|| format!("lookup host '{}'", addr.as_ref()))?
        .collect();

    let mut lookup_result: Option<SocketAddr> = None;
    for addr in addrs {
        if addr.is_ipv4() {
            lookup_result = Some(addr);
            break;
        }
        lookup_result = Some(addr);
    }
    match lookup_result {
        Some(addr) => Ok(addr),
        None => bail!("lookup host '{}' did not have result", addr.as_ref()),
    }
}

fn run_with_timeout<F>(future: F, timeout: Duration) -> Timeout<F>
where
    F: Future,
{
    time::timeout_at(Instant::now() + timeout, future)
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;

    use crate::logs;

    /// Start test case:
    ///
    /// ```
    /// TEST_SERVER="true" cargo test net::server::tests::test_server -- --nocapture
    /// TEST_CLIENT="0" cargo test net::client::tests::test_client -- --nocapture
    /// echo -n "Some content" > .test_client_0
    /// ```
    #[tokio::test]
    async fn test_client() {
        let client_name = match env::var("TEST_CLIENT") {
            Ok(name) => name,
            Err(_) => return,
        };
        logs::init("debug").unwrap();

        let file_name = format!(".test_client_{client_name}");

        let addr = String::from("127.0.0.1:9988");
        let password = String::from("password123");

        let client = Client::connect(addr, password, 500).await.unwrap();
        let (mut watch_rx, write_tx) = client.start();

        let mut trigger_intv = tokio::time::interval_at(Instant::now(), Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = trigger_intv.tick() => {
                    let data = fs::read(&file_name).unwrap_or_default();
                    if data.is_empty() {
                        continue;
                    }
                    println!(
                        "Write data to server: {}",
                        String::from_utf8_lossy(&data)
                    );
                    write_tx.send(DataItem::Clipboard(data)).await.unwrap();

                    fs::remove_file(&file_name).unwrap();
                }
                Some(data_item) = watch_rx.recv() => {
                    match data_item {
                        DataItem::File(file_item) => {
                            println!(
                                "Read file from server: {}, name: {}, mode: {}",
                                String::from_utf8_lossy(&file_item.data),
                                file_item.name,
                                file_item.mode,
                            );
                        }
                        DataItem::Clipboard(data) => {
                            println!(
                                "Read data from server: {}",
                                String::from_utf8_lossy(&data)
                            );
                        }
                    }
                }
            }
        }
    }
}
