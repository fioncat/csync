use std::io::Cursor;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter};

use crate::net::auth::Auth;
use crate::net::frame::Frame;

pub struct Connection<S: AsyncWrite + AsyncRead + Unpin> {
    stream: BufWriter<S>,

    buffer: BytesMut,

    auth: Arc<Option<Auth>>,
}

impl<S: AsyncWrite + AsyncRead + Unpin> Connection<S> {
    /// The read buffer size, default is 32KiB.
    /// For most scenes, the clipboard stores text, and this value is appropriate.
    /// But for images, the buffer needs to be expanded.
    const BUFFER_SIZE: usize = 32 << 10;

    #[inline]
    pub fn new(socket: S) -> Connection<S> {
        Connection {
            stream: BufWriter::new(socket),
            buffer: BytesMut::with_capacity(Self::BUFFER_SIZE),
            auth: Arc::new(None),
        }
    }

    #[inline]
    pub fn with_auth(&mut self, auth: Arc<Option<Auth>>) {
        self.auth = auth;
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

    pub async fn write_frame(&mut self, frame: &Frame<'_>) -> Result<()> {
        frame.write(&mut self.stream, &self.auth).await?;
        self.stream.flush().await.context("flush tcp stream")
    }
}
