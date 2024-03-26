use std::borrow::Cow;
use std::io::Cursor;

use anyhow::Result;
use bytes::{Buf, Bytes};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::net::auth::{Auth, AuthError};

pub const PROTOCOL_VERSION: u32 = 1;

const PROTOCOL_PUB: u8 = b'0';
const PROTOCOL_SUB: u8 = b'1';
const PROTOCOL_ACCEPT: u8 = b'2';
const PROTOCOL_DATA: u8 = b'3';
const PROTOCOL_OK: u8 = b'4';
const PROTOCOL_ERROR: u8 = b'5';
const PROTOCOL_PING: u8 = b'6';

#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    Pub(Cow<'a, str>),
    Sub(Cow<'a, [String]>),

    Accept(Cow<'a, AcceptFrame>),

    Data(Cow<'a, DataFrame>),

    Ok,
    Error(Cow<'a, str>),

    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AcceptFrame {
    pub version: u32,
    pub auth: Option<AuthFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthFrame {
    pub nonce: Vec<u8>,
    pub salt: Vec<u8>,
    pub check: Vec<u8>,
    pub check_plain: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataFrame {
    pub info: DataFrameInfo,

    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataFrameInfo {
    pub device: Option<String>,
    pub digest: String,

    pub file: Option<FileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileInfo {
    pub name: String,
    pub mode: u64,
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

impl<'a> Frame<'a> {
    pub(crate) fn parse(
        src: &mut Cursor<&[u8]>,
        auth: &Option<Auth>,
    ) -> Result<Option<(Frame<'a>, usize)>, FrameError> {
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
                let frame = Self::_parse(src, auth)?;

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

    fn _parse(src: &mut Cursor<&[u8]>, auth: &Option<Auth>) -> Result<Self, FrameError> {
        let flag = get_u8(src)?;

        match flag {
            PROTOCOL_PUB => {
                let device = parse_string(&get_data(src, auth)?)?;
                Ok(Self::Pub(Cow::Owned(device)))
            }

            PROTOCOL_SUB => {
                let devices = decode_object(&get_data(src, auth)?)?;
                Ok(Self::Sub(devices))
            }

            PROTOCOL_ACCEPT => {
                let accept = decode_object(&get_data(src, auth)?)?;
                Ok(Self::Accept(accept))
            }

            PROTOCOL_DATA => {
                let info: DataFrameInfo = decode_object(&get_data(src, auth)?)?;
                let data = get_data(src, auth)?.to_vec();
                Ok(Self::Data(Cow::Owned(DataFrame { info, body: data })))
            }

            PROTOCOL_OK => Ok(Self::Ok),

            PROTOCOL_ERROR => {
                let err = parse_string(&get_data(src, auth)?)?;
                Ok(Self::Error(Cow::Owned(err)))
            }

            PROTOCOL_PING => Ok(Self::Ping),

            _ => Err(Self::invalid_flag()),
        }
    }

    fn check(src: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
        let flag = get_u8(src)?;

        let check_rounds = match flag {
            PROTOCOL_OK | PROTOCOL_PING => 0,
            PROTOCOL_PUB | PROTOCOL_SUB | PROTOCOL_ACCEPT | PROTOCOL_ERROR => 1,
            PROTOCOL_DATA => 2,
            _ => return Err(Self::invalid_flag()),
        };

        for _ in 0..check_rounds {
            check_data(src)?;
        }

        Ok(())
    }

    pub(crate) async fn write<W>(&self, stream: &mut W, auth: &Option<Auth>) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        match self {
            Self::Pub(device) => {
                stream.write_u8(PROTOCOL_PUB).await?;
                write_data(stream, auth, device.as_bytes()).await?;
            }

            Self::Sub(devices) => {
                stream.write_u8(PROTOCOL_SUB).await?;
                write_data(stream, auth, &encode_object(devices)?).await?;
            }

            Self::Accept(accept) => {
                stream.write_u8(PROTOCOL_ACCEPT).await?;
                write_data(stream, auth, &encode_object(accept)?).await?;
            }

            Self::Data(data) => {
                stream.write_u8(PROTOCOL_DATA).await?;
                write_data(stream, auth, &encode_object(&data.info)?).await?;
                write_data(stream, auth, &data.body).await?;
            }

            Self::Ok => stream.write_u8(PROTOCOL_OK).await?,

            Self::Error(err) => {
                stream.write_u8(PROTOCOL_ERROR).await?;
                write_data(stream, auth, err.as_bytes()).await?;
            }

            Self::Ping => stream.write_u8(PROTOCOL_PING).await?,
        }
        Ok(())
    }

    #[inline]
    fn invalid_flag() -> FrameError {
        FrameError::Protocol("invalid frame flag")
    }

    pub fn expect_accept(self) -> Result<AcceptFrame, FrameError> {
        self.check_result()?;
        if let Self::Accept(accept) = self {
            return Ok(accept.into_owned());
        }

        Err(FrameError::Unexpect("accept"))
    }

    pub fn expect_ok(self) -> Result<(), FrameError> {
        self.check_result()?;
        if let Self::Ok = self {
            return Ok(());
        }

        Err(FrameError::Unexpect("ok"))
    }

    pub fn expect_data(self) -> Result<DataFrame, FrameError> {
        self.check_result()?;
        if let Self::Data(data) = self {
            return Ok(data.into_owned());
        }

        Err(FrameError::Unexpect("data"))
    }

    #[inline]
    fn check_result(&self) -> Result<(), FrameError> {
        if let Self::Error(err) = self {
            return Err(FrameError::Server(err.to_string()));
        }
        Ok(())
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

fn get_data(src: &mut Cursor<&[u8]>, auth: &Option<Auth>) -> Result<Bytes, FrameError> {
    let len = get_decimal(src)? as usize;
    get_data_len(src, auth, len)
}

fn get_decimal(src: &mut Cursor<&[u8]>) -> Result<u64, FrameError> {
    if src.remaining() < 8 {
        return Err(FrameError::Incomplete);
    }
    Ok(src.get_u64())
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

#[inline]
fn parse_string(data: &[u8]) -> Result<String, FrameError> {
    match String::from_utf8(data.to_vec()) {
        Ok(text) => Ok(text),
        Err(_) => Err(FrameError::Protocol("invalid text, not uft-8 encoded")),
    }
}

async fn write_data<W>(w: &mut W, auth: &Option<Auth>, data: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let data: Cow<[u8]> = match auth.as_ref() {
        Some(auth) => Cow::Owned(auth.encrypt(data)?),
        None => Cow::Borrowed(data),
    };

    w.write_u64(data.len() as u64).await?;
    w.write_all(data.as_ref()).await?;

    Ok(())
}

#[inline]
fn decode_object<T: DeserializeOwned>(data: &[u8]) -> Result<T, FrameError> {
    match bincode::deserialize(data) {
        Ok(v) => Ok(v),
        Err(_) => Err(FrameError::Protocol("decode frame failed, invalid object")),
    }
}

#[inline]
fn encode_object<T: Serialize>(value: &T) -> Result<Vec<u8>, FrameError> {
    match bincode::serialize(value) {
        Ok(data) => Ok(data),
        Err(_) => Err(FrameError::Protocol("encode frame failed")),
    }
}
