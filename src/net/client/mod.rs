mod send;
mod watch;

pub use send::SendClient;
pub use watch::WatchClient;

use std::borrow::Cow;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{lookup_host, TcpSocket, TcpStream};
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use tokio::time::{self, Instant};

use crate::net::auth::Auth;
use crate::net::conn::Connection;
use crate::net::frame::{self, DataFrame, Frame};

struct Client<S: AsyncWrite + AsyncRead + Unpin + Send> {
    conn: Arc<Mutex<Connection<S>>>,
}

impl Client<TcpStream> {
    async fn connect<S, P>(addr: S, head: &Frame<'_>, password: Option<P>) -> Result<Self>
    where
        S: AsRef<str>,
        P: AsRef<str>,
    {
        let addr = parse_addr(addr).await?;
        let socket = if addr.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        }
        .context("create tcp socket")?;
        let stream = socket
            .connect(addr)
            .await
            .with_context(|| format!("connect to '{}'", addr))?;

        let conn = Connection::new(stream);
        Self::new(conn, head, password).await
    }
}

impl<S: AsyncWrite + AsyncRead + Unpin + Send + 'static> Client<S> {
    async fn new<P>(mut conn: Connection<S>, head: &Frame<'_>, password: Option<P>) -> Result<Self>
    where
        P: AsRef<str>,
    {
        conn.write_frame(head)
            .await
            .context("send head frame to server")?;

        let mut accept = conn
            .must_read_frame()
            .await
            .context("read accept frame from server")?
            .expect_accept()?;

        if accept.version != frame::PROTOCOL_VERSION {
            bail!(
                "protocol version mismatched, client is: {}, server is: {}",
                frame::PROTOCOL_VERSION,
                accept.version
            );
        }

        let auth = accept.auth.take();
        if let Some(auth_frame) = auth {
            if password.is_none() {
                bail!(
                    "the server require auth, but you donot have a password, please conigure one"
                );
            }

            let auth = Auth::from_frame(password.unwrap(), auth_frame).context("auth server")?;
            conn.with_auth(Arc::new(Some(auth)));
        }

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    async fn send(&mut self, data: Arc<DataFrame>) -> Result<()> {
        let conn = Arc::clone(&self.conn);
        let (done_tx, done_rx) = oneshot::channel::<Result<()>>();
        tokio::spawn(async move {
            let mut conn = conn.lock().await;
            let result = Self::_send(&mut conn, data).await;
            done_tx.send(result).unwrap();
        });

        match time::timeout_at(Instant::now() + Duration::from_secs(1), done_rx).await {
            Ok(result) => result.unwrap(),
            Err(_) => bail!("send data timeout after 1s"),
        }
    }

    #[inline]
    async fn _send(conn: &mut Connection<S>, data: Arc<DataFrame>) -> Result<()> {
        let frame = Frame::Data(Cow::Borrowed(data.as_ref()));

        conn.write_frame(&frame).await.context("send data frame")?;

        conn.must_read_frame()
            .await
            .context("read data frame resp")?
            .expect_ok()?;

        Ok(())
    }

    async fn recv(&mut self) -> Result<DataFrame> {
        loop {
            let mut conn = self.conn.lock().await;
            let frame = conn.must_read_frame().await.context("recv data frame")?;
            if Frame::Ping == frame {
                continue;
            }
            let data = frame.expect_data()?;
            return Ok(data);
        }
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
