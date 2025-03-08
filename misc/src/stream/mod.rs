/// Provides encryption and framing for secure data streaming.
pub mod cipher;
/// Provides frame encoding and decoding for data streaming.
pub mod frame;

use std::io::Cursor;

use anyhow::{bail, Context, Result};
use bytes::{Buf, BytesMut};
use cipher::Cipher;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter};

/// A stream wrapper that provides framing and optional encryption for data.
///
/// This struct wraps an AsyncRead + AsyncWrite stream (like a TCP connection)
/// and provides methods to send and receive framed data with optional encryption.
pub struct Stream<S: AsyncWrite + AsyncRead + Unpin> {
    /// Buffered writer for the underlying stream
    bw: BufWriter<S>,

    /// Buffer for reading data
    buffer: BytesMut,

    /// Optional cipher for encryption/decryption
    cipher: Option<Cipher>,
}

impl<S: AsyncWrite + AsyncRead + Unpin> Stream<S> {
    /// The read buffer size, default is 32KiB.
    /// For most scenes, the clipboard stores text, and this value is appropriate.
    /// But for images, the buffer needs to be expanded.
    const BUFFER_SIZE: usize = 32 << 10;

    /// Creates a new Stream instance wrapping the provided socket.
    ///
    /// # Arguments
    ///
    /// * `socket` - The underlying stream to wrap
    ///
    /// # Returns
    ///
    /// A new Stream instance
    pub fn new(socket: S) -> Self {
        Self {
            bw: BufWriter::new(socket),
            buffer: BytesMut::with_capacity(Self::BUFFER_SIZE),
            cipher: None,
        }
    }

    /// Sets the cipher for encryption and decryption.
    ///
    /// # Arguments
    ///
    /// * `cipher` - The cipher to use
    pub fn set_cipher(&mut self, cipher: Cipher) {
        self.cipher = Some(cipher);
    }

    /// Reads the next frame and converts it to a UTF-8 string.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The next frame as a UTF-8 string
    /// * `Err` - If an error occurred during reading or UTF-8 conversion
    pub async fn next_string(&mut self) -> Result<String> {
        let data = self.next().await?;
        Ok(String::from_utf8(data)?)
    }

    /// Reads the next frame of data.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - The next frame of data
    /// * `Err` - If an error occurred during reading or the connection was closed
    pub async fn next(&mut self) -> Result<Vec<u8>> {
        match self.next_raw().await? {
            Some(frame) => Ok(frame),
            None => bail!("connection was closed by peer"),
        }
    }

    /// Reads the next frame of data, returning None if the connection is closed.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Vec<u8>))` - The next frame of data
    /// * `Ok(None)` - If the connection was cleanly closed
    /// * `Err` - If an error occurred during reading
    pub async fn next_raw(&mut self) -> Result<Option<Vec<u8>>> {
        loop {
            // Cursor is used to track the "current" location in the
            // buffer. Cursor also implements `Buf` from the `bytes` crate
            // which provides a number of helpful utilities for working
            // with bytes.
            let mut buf = Cursor::new(&self.buffer[..]);

            if let Some((frame, len)) = frame::next_data(&mut buf, &self.cipher)? {
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
                .bw
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

    /// Writes a frame of data to the stream.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to write
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the data was successfully written
    /// * `Err` - If an error occurred during writing
    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        frame::write_data(&mut self.bw, data, &self.cipher).await?;
        self.bw.flush().await.context("flush tcp stream")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::fs::{File, OpenOptions};

    async fn run_test(path: &str, cipher: Option<Cipher>) {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .await
            .unwrap();
        let mut stream = Stream::new(file);
        if let Some(cipher) = cipher.clone() {
            stream.set_cipher(cipher);
        }

        let frames = [
            b"test data01".to_vec(),
            b"test data02".to_vec(),
            b"hello world".to_vec(),
            b"".to_vec(),
            b"This is a new data".to_vec(),
        ];
        for frame in frames.iter() {
            stream.write(frame).await.unwrap();
        }

        let file = File::open(path).await.unwrap();
        let mut stream = Stream::new(file);
        if let Some(cipher) = cipher {
            stream.set_cipher(cipher);
        }
        for expect in frames {
            let frame = stream.next().await.unwrap();
            assert_eq!(expect, frame);
        }
    }

    #[tokio::test]
    async fn test_no_auth() {
        run_test("testdata/conn_no_auth", None).await;
    }

    #[tokio::test]
    async fn test_with_auth() {
        let passwords = vec!["test123", "password", "hello", ""];
        for password in passwords {
            let cipher = Some(Cipher::new(password.as_bytes().to_vec()));
            run_test("testdata/conn_auth", cipher).await;
        }
    }
}
