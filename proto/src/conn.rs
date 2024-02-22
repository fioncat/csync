use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::{TcpSocket, TcpStream};

use crate::auth::Auth;
use crate::frame::Frame;

pub struct Connection {
    stream: BufWriter<TcpStream>,

    buffer: BytesMut,

    auth: Arc<Option<Auth>>,
}

impl Connection {
    /// The read buffer size, default is 32KiB.
    /// For most scenes, the clipboard stores text, and this value is appropriate.
    /// But for images, the buffer needs to be expanded.
    const BUFFER_SIZE: usize = 32 << 10;

    pub fn new(socket: TcpStream, auth: Arc<Option<Auth>>) -> Connection {
        Connection {
            stream: BufWriter::new(socket),
            buffer: BytesMut::with_capacity(Self::BUFFER_SIZE),
            auth,
        }
    }

    pub async fn dial(addr: &SocketAddr, auth: Arc<Option<Auth>>) -> Result<Connection> {
        let socket = if addr.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        }
        .context("create tcp socket")?;
        let stream = socket
            .connect(*addr)
            .await
            .with_context(|| format!("connect to '{}'", addr))?;
        Ok(Self::new(stream, auth))
    }

    pub(crate) fn set_auth(&mut self, auth: Auth) {
        self.auth = Arc::new(Some(auth));
    }

    pub async fn must_read_frame(&mut self) -> Result<Frame> {
        match self.read_frame().await? {
            Some(frame) => Ok(frame),
            None => bail!("connection was closed by peer"),
        }
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            // Cursor is used to track the "current" location in the
            // buffer. Cursor also implements `Buf` from the `bytes` crate
            // which provides a number of helpful utilities for working
            // with bytes.
            let mut buf = Cursor::new(&self.buffer[..]);

            if let Some((frame, len)) = Frame::parse(&mut buf, &self.auth)? {
                // Discard the parsed data from the read buffer.
                //
                // When `advance` is called on the read buffer, all of the data
                // up to `len` is discarded. The details of how this works is
                // left to `BytesMut`. This is often done by moving an internal
                // cursor, but it may be done by reallocating and copying data.
                self.buffer.advance(len);
                return Ok(Some(frame));
            }

            let read = self
                .stream
                .read_buf(&mut self.buffer)
                .await
                .context("read data from peer")?;
            if read == 0 {
                // The remote closed the connection. For this to be a clean
                // shutdown, there should be no data in the read buffer. If
                // there is, this means that the peer closed the socket while
                // sending a frame.
                if self.buffer.is_empty() {
                    return Ok(None);
                }
                bail!("connection reset by peer");
            }
        }
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> Result<()> {
        frame.send(&mut self.stream, &self.auth).await?;
        self.stream.flush().await.context("flush tcp stream")
    }
}
