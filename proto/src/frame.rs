use std::borrow::Cow;
use std::fmt::Display;
use std::io::Cursor;

use anyhow::Result;
use bytes::{Buf, Bytes};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;

use crate::auth::{Auth, AuthError};

pub const PROTOCOL_VERSION: u32 = 1;

const PROTOCOL_REGISTER: u8 = b'0';
const PROTOCOL_ACCEPT: u8 = b'1';
const PROTOCOL_PUSH_TEXT: u8 = b'2';
const PROTOCOL_PUSH_IMAGE: u8 = b'3';
const PROTOCOL_PULL: u8 = b'4';
const PROTOCOL_NONE: u8 = b'5';
const PROTOCOL_ERROR: u8 = b'!';

#[derive(Debug)]
pub enum Frame {
    Register(RegisterFrame),
    Accept(AcceptFrame),

    Push(DataFrame),
    Pull,

    None,

    Error(String),
}

impl Frame {
    pub fn expect_register(self) -> Result<RegisterFrame, FrameError> {
        match self {
            Frame::Register(frame) => Ok(frame),
            _ => Err(FrameError::Unexpect("register")),
        }
    }

    pub fn expect_accept(self) -> Result<AcceptFrame, FrameError> {
        match self {
            Frame::Accept(frame) => Ok(frame),
            _ => Err(FrameError::Unexpect("accept")),
        }
    }

    pub fn expect_none(self) -> Result<(), FrameError> {
        match self {
            Frame::None => Ok(()),
            _ => Err(FrameError::Unexpect("none")),
        }
    }

    pub fn expect_data(self) -> Result<Option<DataFrame>, FrameError> {
        match self {
            Frame::Push(data) => Ok(Some(data)),
            Frame::None => Ok(None),
            _ => Err(FrameError::Unexpect("data/none")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterFrame {
    pub publish: Option<String>,

    pub subs: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AcceptFrame {
    pub version: u32,
    pub auth: Option<AuthFrame>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthFrame {
    pub nonce: Vec<u8>,
    pub salt: Vec<u8>,
    pub check: Vec<u8>,
    pub check_plain: Vec<u8>,
}

#[derive(Debug)]
pub struct DataFrame {
    pub from: Option<String>,
    pub digest: String,

    pub data: ClipboardFrame,
}

impl Display for DataFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let from = match self.from.as_ref() {
            Some(from) => from.as_str(),
            None => "<none>",
        };
        let digest = hex::encode(&self.digest);
        let data = match &self.data {
            ClipboardFrame::Text(s) => format!("{} length text", s.len()),
            ClipboardFrame::Image(image) => format!(
                "{} length image, width {}, height {}",
                image.data.len(),
                image.width,
                image.height
            ),
        };
        write!(
            f,
            "DataFrame: {{ from: {from}, digest: {digest}, data: {data} }}"
        )
    }
}

#[derive(Debug)]
pub enum ClipboardFrame {
    Text(String),
    Image(ClipboardImage),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClipboardImage {
    pub width: u64,
    pub height: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("not enough data is available to parse a message")]
    Incomplete,

    #[error("invalid frame protocol: {0}")]
    Protocol(&'static str),

    #[error("auth data frame failed")]
    Auth(#[from] AuthError),

    #[error("server error: {0}")]
    Server(String),

    #[error("unexpect next frame, expect to be `{0}`")]
    Unexpect(&'static str),
}

impl Frame {
    pub(crate) fn parse(
        src: &mut Cursor<&[u8]>,
        auth: &Option<Auth>,
    ) -> Result<Option<(Frame, usize)>, FrameError> {
        // The first step is to check if enough data has been buffered to parse
        // a single frame. This step is usually much faster than doing a full
        // parse of the frame, and allows us to skip allocating data structures
        // to hold the frame data unless we know the full frame has been
        // received.
        match Self::check(src) {
            Ok(_) => {
                // The `check` function will have advanced the cursor until the
                // end of the frame. Since the cursor had position set to zero
                // before `Frame::check` was called, we obtain the length of the
                // frame by checking the cursor position.
                let len = src.position() as usize;

                // Reset the position to zero before passing the cursor to
                // `Frame::parse`.
                src.set_position(0);

                // Parse the frame from the buffer. This allocates the necessary
                // structures to represent the frame and returns the frame
                // value.
                //
                // If the encoded frame representation is invalid, an error is
                // returned. This should terminate the **current** connection
                // but should not impact any other connected client.
                let frame = Self::parse_frame(src, auth)?;

                Ok(Some((frame, len)))
            }

            // There is not enough data present in the read buffer to parse a
            // single frame. We must wait for more data to be received from the
            // socket. Reading from the socket will be done in the statement
            // after this `match`.
            //
            // We do not want to return `Err` from here as this "error" is an
            // expected runtime condition.
            Err(FrameError::Incomplete) => Ok(None),

            Err(err) => Err(err),
        }
    }

    pub(crate) async fn send(
        &self,
        stream: &mut BufWriter<TcpStream>,
        auth: &Option<Auth>,
    ) -> Result<()> {
        match self {
            Frame::Register(frame) => {
                stream.write_u8(PROTOCOL_REGISTER).await?;
                let data = encode_object(frame)?;
                write_data(stream, &None, &data).await
            }
            Frame::Accept(frame) => {
                stream.write_u8(PROTOCOL_ACCEPT).await?;
                let data = encode_object(frame)?;
                write_data(stream, &None, &data).await
            }
            Frame::Push(frame) => {
                let flag = match &frame.data {
                    ClipboardFrame::Image(_) => PROTOCOL_PUSH_IMAGE,
                    ClipboardFrame::Text(_) => PROTOCOL_PUSH_TEXT,
                };
                stream.write_u8(flag).await?;

                match frame.from.as_ref() {
                    Some(from) => write_data(stream, auth, from.as_bytes()).await?,
                    None => stream.write_u64(0).await?,
                };
                write_data(stream, auth, frame.digest.as_bytes()).await?;

                match &frame.data {
                    ClipboardFrame::Text(text) => write_data(stream, auth, text.as_bytes()).await,
                    ClipboardFrame::Image(image) => {
                        stream.write_u64(image.width).await?;
                        stream.write_u64(image.height).await?;
                        write_data(stream, auth, &image.data).await
                    }
                }
            }
            Frame::Pull => {
                stream.write_u8(PROTOCOL_PULL).await?;
                Ok(())
            }
            Frame::None => {
                stream.write_u8(PROTOCOL_NONE).await?;
                Ok(())
            }

            Frame::Error(msg) => {
                stream.write_u8(PROTOCOL_ERROR).await?;
                write_data(stream, &None, msg.as_bytes()).await?;
                Ok(())
            }
        }
    }

    fn parse_frame(src: &mut Cursor<&[u8]>, auth: &Option<Auth>) -> Result<Frame, FrameError> {
        let flag = get_u8(src)?;
        match flag {
            PROTOCOL_REGISTER => {
                let data = get_data(src, &None)?;
                let frame = decode_object::<RegisterFrame>(&data)?;
                return Ok(Frame::Register(frame));
            }

            PROTOCOL_ACCEPT => {
                let data = get_data(src, &None)?;
                let frame = decode_object::<AcceptFrame>(&data)?;
                return Ok(Frame::Accept(frame));
            }

            PROTOCOL_PULL => return Ok(Frame::Pull),
            PROTOCOL_NONE => return Ok(Frame::None),

            PROTOCOL_ERROR => {
                let data = get_data(src, &None)?;
                let msg = parse_string(&data)?;
                return Err(FrameError::Server(msg));
            }

            _ => {}
        }

        let from = match get_data_option(src, auth)? {
            Some(data) => Some(parse_string(&data.to_vec())?),
            None => None,
        };

        let digest = get_data(src, auth)?.to_vec();
        let digest = parse_string(&digest)?;

        let data = match flag {
            PROTOCOL_PUSH_TEXT => {
                let data = get_data(src, auth)?;
                ClipboardFrame::Text(parse_string(&data)?)
            }
            PROTOCOL_PUSH_IMAGE => {
                let width = get_decimal(src)?;
                let height = get_decimal(src)?;
                let data = get_data(src, auth)?;
                ClipboardFrame::Image(ClipboardImage {
                    width,
                    height,
                    data: data.to_vec(),
                })
            }
            _ => return Err(Self::invalid_flag()),
        };

        let data_frame = DataFrame { from, digest, data };
        Ok(Frame::Push(data_frame))
    }

    fn check(src: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
        let flag = get_u8(src)?;
        match flag {
            PROTOCOL_REGISTER | PROTOCOL_ACCEPT | PROTOCOL_ERROR => {
                check_data(src)?;
                return Ok(());
            }
            PROTOCOL_PULL | PROTOCOL_NONE => return Ok(()),
            _ => {}
        }

        check_data(src)?;
        check_data(src)?;

        match flag {
            PROTOCOL_PUSH_TEXT => check_data(src),
            PROTOCOL_PUSH_IMAGE => {
                get_decimal(src)?;
                get_decimal(src)?;
                check_data(src)
            }
            _ => Err(Self::invalid_flag()),
        }
    }

    fn invalid_flag() -> FrameError {
        FrameError::Protocol("invalid frame flag")
    }
}

fn check_data(src: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
    let len = get_decimal(src)? as usize;
    skip(src, len)?;
    Ok(())
}

fn get_u8(src: &mut Cursor<&[u8]>) -> Result<u8, FrameError> {
    if !src.has_remaining() {
        return Err(FrameError::Incomplete);
    }
    Ok(src.get_u8())
}

fn get_decimal(src: &mut Cursor<&[u8]>) -> Result<u64, FrameError> {
    if src.remaining() < 8 {
        return Err(FrameError::Incomplete);
    }
    Ok(src.get_u64())
}

fn get_data(src: &mut Cursor<&[u8]>, auth: &Option<Auth>) -> Result<Bytes, FrameError> {
    let len = get_decimal(src)? as usize;
    get_data_len(src, auth, len)
}

fn get_data_option(
    src: &mut Cursor<&[u8]>,
    auth: &Option<Auth>,
) -> Result<Option<Bytes>, FrameError> {
    let len = get_decimal(src)? as usize;
    if len == 0 {
        return Ok(None);
    }
    let data = get_data_len(src, auth, len)?;
    Ok(Some(data))
}

fn get_data_len(
    src: &mut Cursor<&[u8]>,
    auth: &Option<Auth>,
    len: usize,
) -> Result<Bytes, FrameError> {
    if src.remaining() < len {
        return Err(FrameError::Incomplete);
    }

    let mut data = Bytes::copy_from_slice(&src.chunk()[..len]);
    if let Some(auth) = auth.as_ref() {
        data = auth.decrypt(&data)?.into();
    }
    skip(src, len)?;

    Ok(data)
}

fn skip(src: &mut Cursor<&[u8]>, n: usize) -> Result<(), FrameError> {
    if src.remaining() < n {
        return Err(FrameError::Incomplete);
    }
    src.advance(n);
    Ok(())
}

fn parse_string(data: &[u8]) -> Result<String, FrameError> {
    match String::from_utf8(data.to_vec()) {
        Ok(text) => Ok(text),
        Err(_) => return Err(FrameError::Protocol("invalid text, not uft-8 encoded")),
    }
}

async fn write_data(
    stream: &mut BufWriter<TcpStream>,
    auth: &Option<Auth>,
    data: &[u8],
) -> Result<()> {
    let data: Cow<[u8]> = match auth.as_ref() {
        Some(auth) => Cow::Owned(auth.encrypt(data)?),
        None => Cow::Borrowed(data),
    };

    stream.write_u64(data.len() as u64).await?;
    stream.write_all(data.as_ref()).await?;

    Ok(())
}

fn decode_object<T: DeserializeOwned>(data: &[u8]) -> Result<T, FrameError> {
    match bincode::deserialize(data) {
        Ok(v) => Ok(v),
        Err(_) => Err(FrameError::Protocol("decode frame failed, invalid object")),
    }
}

fn encode_object<T: Serialize>(value: &T) -> Result<Vec<u8>, FrameError> {
    match bincode::serialize(value) {
        Ok(data) => Ok(data),
        Err(_) => return Err(FrameError::Protocol("encode frame failed")),
    }
}
