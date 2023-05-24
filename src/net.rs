use std::fmt;
use std::io::Cursor;
use std::net::SocketAddr;

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key};
use anyhow::{bail, Context, Result};
use bytes::{Buf, Bytes, BytesMut};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::{TcpSocket, TcpStream};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Not enough data is available to parse a message")]
    Incomplete,

    /// Invalid message encoding
    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Auth message error")]
    Auth,
}

#[derive(Clone)]
pub struct Auth {
    cipher: Aes256Gcm,
}

impl Auth {
    pub fn new(auth_key: &[u8]) -> Auth {
        let key = Key::<Aes256Gcm>::from_slice(auth_key);
        let cipher = Aes256Gcm::new(key);

        Auth { cipher }
    }

    fn encrypt(&self, plain: &[u8]) -> Result<Vec<u8>, Error> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let mut data = match self.cipher.encrypt(&nonce, plain) {
            Ok(data) => data,
            Err(_) => return Err(Error::Auth),
        };
        data.splice(..0, nonce);
        Ok(data)
    }

    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        if data.len() <= 12 {
            return Err(Error::Auth);
        }
        let nonce = &data[..12];
        let cipher_data = &data[12..];

        match self.cipher.decrypt(nonce.into(), cipher_data) {
            Ok(plain) => Ok(plain),
            Err(_) => Err(Error::Auth),
        }
    }
}

/// A frame in the csync protocol.
#[derive(Debug)]
pub enum Frame {
    Text(String),
    Image(u64, u64, Bytes),
    File(String, u32, Bytes),
}

struct FrameParser<'a> {
    cursor: Cursor<&'a [u8]>,
    auth: Option<&'a Auth>,
}

impl<'a> FrameParser<'a> {
    pub const PROTOCOL_TEXT: u8 = b't';
    pub const PROTOCOL_IMAGE: u8 = b'i';
    pub const PROTOCOL_FILE: u8 = b'f';

    fn new(buffer: &BytesMut) -> FrameParser {
        FrameParser {
            cursor: Cursor::new(&buffer[..]),
            auth: None,
        }
    }

    fn with_auth(&mut self, auth: &'a Auth) {
        self.auth = Some(auth);
    }

    fn parse(&mut self) -> Result<Option<(Frame, usize)>, Error> {
        // The first step is to check if enough data has been buffered to parse
        // a single frame. This step is usually much faster than doing a full
        // parse of the frame, and allows us to skip allocating data structures
        // to hold the frame data unless we know the full frame has been
        // received.
        match self.check() {
            Ok(_) => {
                // The `check` function will have advanced the cursor until the
                // end of the frame. Since the cursor had position set to zero
                // before `Frame::check` was called, we obtain the length of the
                // frame by checking the cursor position.
                let len = self.cursor.position() as usize;

                // Reset the position to zero before passing the cursor to
                // `Frame::parse`.
                self.cursor.set_position(0);

                // Parse the frame from the buffer. This allocates the necessary
                // structures to represent the frame and returns the frame
                // value.
                //
                // If the encoded frame representation is invalid, an error is
                // returned. This should terminate the **current** connection
                // but should not impact any other connected client.
                let frame = self.parse_frame()?;

                Ok(Some((frame, len)))
            }
            // There is not enough data present in the read buffer to parse a
            // single frame. We must wait for more data to be received from the
            // socket. Reading from the socket will be done in the statement
            // after this `match`.
            //
            // We do not want to return `Err` from here as this "error" is an
            // expected runtime condition.
            Err(Error::Incomplete) => Ok(None),

            Err(err) => Err(err),
        }
    }

    fn check(&mut self) -> Result<(), Error> {
        match self.get_u8()? {
            Self::PROTOCOL_TEXT => self.check_data(),
            Self::PROTOCOL_IMAGE => {
                self.get_decimal()?; // width
                self.get_decimal()?; // height
                self.check_data()
            }
            Self::PROTOCOL_FILE => {
                self.get_line()?; // file name
                self.get_decimal()?; // file mode
                self.check_data()
            }
            actual => Err(Error::Protocol(format!("invalid frame type `{actual}`"))),
        }
    }

    fn parse_frame(&mut self) -> Result<Frame, Error> {
        match self.get_u8()? {
            Self::PROTOCOL_TEXT => {
                let data = self.get_data()?;
                let text = self.parse_string(&data)?;
                Ok(Frame::Text(text))
            }
            Self::PROTOCOL_IMAGE => {
                let width = self.get_decimal()?;
                let height = self.get_decimal()?;
                let data = self.get_data()?;
                Ok(Frame::Image(width, height, data))
            }
            Self::PROTOCOL_FILE => {
                let name_data = self.get_line()?;
                let name = self.parse_string(name_data)?;
                let mode = self.get_decimal()? as u32;
                let data = self.get_data()?;

                Ok(Frame::File(name, mode, data))
            }
            _ => unreachable!(),
        }
    }

    fn get_u8(&mut self) -> Result<u8, Error> {
        if !self.cursor.has_remaining() {
            return Err(Error::Incomplete);
        }
        Ok(self.cursor.get_u8())
    }

    fn check_data(&mut self) -> Result<(), Error> {
        let len = self.get_decimal()? as usize;
        self.skip(len + 2)?;
        Ok(())
    }

    fn get_line(&mut self) -> Result<&'a [u8], Error> {
        let start = self.cursor.position() as usize;
        let end = self.cursor.get_ref().len() - 1;

        for i in start..end {
            if self.cursor.get_ref()[i] == b'\r' && self.cursor.get_ref()[i + 1] == b'\n' {
                self.cursor.set_position((i + 2) as u64);
                return Ok(&self.cursor.get_ref()[start..i]);
            }
        }
        Err(Error::Incomplete)
    }

    fn get_decimal(&mut self) -> Result<u64, Error> {
        use atoi::atoi;
        let line = self.get_line()?;
        match atoi::<u64>(line) {
            Some(num) => Ok(num),
            None => Err(Error::Protocol("invalid decimal".into())),
        }
    }

    fn get_data(&mut self) -> Result<Bytes, Error> {
        let len = self.get_decimal()? as usize;
        let n = len + 2 as usize;

        if self.cursor.remaining() < len {
            return Err(Error::Incomplete);
        }

        let mut data = Bytes::copy_from_slice(&self.cursor.chunk()[..len]);
        if let Some(auth) = self.auth {
            data = auth.decrypt(&data)?.into();
        }

        // skip that number of bytes + 2 (\r\n)
        self.skip(n)?;

        Ok(data)
    }

    fn skip(&mut self, n: usize) -> Result<(), Error> {
        if self.cursor.remaining() < n {
            return Err(Error::Incomplete);
        }
        self.cursor.advance(n);
        Ok(())
    }

    fn parse_string(&self, data: &[u8]) -> Result<String, Error> {
        let data = match self.auth {
            Some(auth) => auth.decrypt(data)?,
            None => data.to_vec(),
        };
        match String::from_utf8(data) {
            Ok(text) => Ok(text),
            Err(_) => return Err(Error::Protocol("invalid text, not uft-8 string".into())),
        }
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use human_bytes::human_bytes;

        match self {
            Frame::Text(text) => {
                let size = human_bytes(text.len() as u32);
                write!(f, "{{{size} Text}}")
            }
            Frame::Image(width, height, data) => {
                let size = human_bytes(data.len() as u32);
                write!(f, "{{{size} Image, width={width}, height={height}}}")
            }
            Frame::File(name, mode, data) => {
                let size = human_bytes(data.len() as u32);
                write!(f, "{{{size} File, name={name}, mode={mode}}}")
            }
        }
    }
}

/// Send and receive `Frame` values from a remote peer.
///
/// To read frames, the `Connection` uses an internal buffer, which is filled
/// up until there are enough bytes to create a full frame. Once this happens,
/// the `Connection` creates the frame and returns it to the caller.
///
/// When sending frames, the frame is first encoded into the write buffer.
/// The contents of the write buffer are then written to the socket.
pub struct Connection {
    /// TCP stream to read or write data.
    stream: BufWriter<TcpStream>,

    /// The read buffer
    buffer: BytesMut,

    auth: Option<Auth>,
}

impl Connection {
    /// The read buffer size, default is 32KiB.
    /// For most scenes, the clipboard stores text, and this value is appropriate.
    /// But for images, the buffer needs to be expanded.
    const BUFFER_SIZE: usize = 32 << 10;

    /// Create a new `Connection`, backed by `socket`. Read and write buffers
    /// are initialized.
    pub fn new(socket: TcpStream) -> Connection {
        Connection {
            stream: BufWriter::new(socket),
            buffer: BytesMut::with_capacity(Self::BUFFER_SIZE),
            auth: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_auth(&mut self, auth: Auth) {
        self.auth = Some(auth);
    }

    /// Read a single `Frame` value from the underlying stream.
    ///
    /// The function waits until it has retrieved enough data to parse a frame.
    /// Any data remaining in the read buffer after the frame has been parsed is
    /// kept there for the next call to `read_frame`.
    ///
    /// # Returns
    ///
    /// On success, the received frame is returned. If the `TcpStream`
    /// is closed in a way that doesn't break a frame in half, it returns
    /// `None`. Otherwise, an error is returned.
    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            let mut parser = FrameParser::new(&self.buffer);
            if let Some(auth) = &self.auth {
                parser.with_auth(auth);
            }
            // Attempt to parse a frame from the buffered data. If enough data
            // has been buffered, the frame is returned.
            if let Some((frame, len)) = parser.parse().context("Parse frame")? {
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
                .context("Read data from peer")?;
            if read == 0 {
                // The remote closed the connection. For this to be a clean
                // shutdown, there should be no data in the read buffer. If
                // there is, this means that the peer closed the socket while
                // sending a frame.
                if self.buffer.is_empty() {
                    return Ok(None);
                }
                bail!("Connection reset by peer");
            }
        }
    }
}

pub struct Client {
    stream: BufWriter<TcpStream>,
    auth: Option<Auth>,
}

impl Client {
    pub async fn dial(addr: &SocketAddr) -> Result<Client> {
        let socket = if addr.is_ipv4() {
            TcpSocket::new_v4()
        } else {
            TcpSocket::new_v6()
        }
        .context("Create tcp socket")?;
        let stream = socket
            .connect(addr.clone())
            .await
            .with_context(|| format!(r#"Connect to "{}""#, addr))?;
        Ok(Client {
            stream: BufWriter::new(stream),
            auth: None,
        })
    }

    pub fn with_auth(&mut self, auth: Auth) {
        self.auth = Some(auth);
    }

    #[allow(dead_code)]
    pub async fn dial_string<S: AsRef<str>>(addr: S) -> Result<Client> {
        let addr: SocketAddr = addr
            .as_ref()
            .parse()
            .with_context(|| format!(r#"Invalid address"{}""#, addr.as_ref()))?;
        Self::dial(&addr).await
    }

    #[allow(dead_code)]
    pub async fn send_text(&mut self, text: String) -> Result<()> {
        self.write_frame(&Frame::Text(text)).await
    }

    #[allow(dead_code)]
    pub async fn send_image(&mut self, width: u64, height: u64, data: Bytes) -> Result<()> {
        self.write_frame(&Frame::Image(width, height, data)).await
    }

    /// Write a frame literal to the stream
    pub async fn write_frame(&mut self, frame: &Frame) -> Result<()> {
        match frame {
            Frame::Text(text) => {
                self.stream.write_u8(FrameParser::PROTOCOL_TEXT).await?;
                self.write_data(text.as_bytes()).await?;
            }
            Frame::Image(width, height, data) => {
                self.stream.write_u8(FrameParser::PROTOCOL_IMAGE).await?;
                self.write_decimal(*width).await?;
                self.write_decimal(*height).await?;
                self.write_data(&data).await?;
            }
            Frame::File(name, mode, data) => {
                self.stream.write_u8(FrameParser::PROTOCOL_FILE).await?;
                self.write_line(&name).await?;
                self.write_decimal(*mode as u64).await?;
                self.write_data(&data).await?;
            }
        };

        // Ensure the encoded frame is written to the socket. The calls above
        // are to the buffered stream and writes. Calling `flush` writes the
        // remaining contents of the buffer to the socket.
        self.stream.flush().await.context("Flush stream")
    }

    async fn write_line(&mut self, line: &String) -> Result<()> {
        let data = line.as_bytes();
        if let Some(auth) = &self.auth {
            let cipher_data = auth.encrypt(data)?;
            self.stream.write_all(&cipher_data).await?;
        } else {
            self.stream.write_all(data).await?;
        }
        self.stream.write_all(b"\r\n").await?;
        Ok(())
    }

    async fn write_data(&mut self, data: &[u8]) -> Result<()> {
        if let Some(auth) = &self.auth {
            let cipher_data = auth.encrypt(data)?;
            self.write_decimal(cipher_data.len() as u64).await?;
            self.stream.write_all(&cipher_data).await?;
        } else {
            self.write_decimal(data.len() as u64).await?;
            self.stream.write_all(data).await?;
        }
        self.stream.write_all(b"\r\n").await?;
        Ok(())
    }

    async fn write_decimal(&mut self, val: u64) -> Result<()> {
        use std::io::Write;

        // Convert the value to a string
        let mut buf = [0u8; 20];
        let mut buf = Cursor::new(&mut buf[..]);
        write!(&mut buf, "{}", val)?;

        let pos = buf.position() as usize;
        self.stream.write_all(&buf.get_ref()[..pos]).await?;
        self.stream.write_all(b"\r\n").await?;

        Ok(())
    }
}
