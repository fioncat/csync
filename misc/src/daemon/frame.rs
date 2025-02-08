use std::io::Cursor;

use anyhow::Result;
use bytes::{Buf, Bytes};
use thiserror::Error;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub data: Vec<u8>,
}

impl Frame {
    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Option<(Self, usize)>, FrameError> {
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
                let frame = Self::parse_raw(src)?;

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

    fn parse_raw(src: &mut Cursor<&[u8]>) -> Result<Self, FrameError> {
        let data = get_data(src)?;
        Ok(Self { data })
    }

    fn check(src: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
        check_data(src)?;
        Ok(())
    }

    pub async fn write<W>(&self, stream: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        write_data(stream, &self.data).await?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("not enough data is available to parse a message")]
    Incomplete,

    #[error("invalid frame protocol: {0}")]
    Protocol(&'static str),
}

fn get_decimal(src: &mut Cursor<&[u8]>) -> Result<u64, FrameError> {
    if src.remaining() < 8 {
        return Err(FrameError::Incomplete);
    }
    Ok(src.get_u64())
}

fn skip(src: &mut Cursor<&[u8]>, n: usize) -> Result<(), FrameError> {
    if src.remaining() < n {
        return Err(FrameError::Incomplete);
    }
    src.advance(n);
    Ok(())
}

fn check_data(src: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
    let len = get_decimal(src)? as usize;
    skip(src, len)?;
    Ok(())
}

fn get_data(src: &mut Cursor<&[u8]>) -> Result<Vec<u8>, FrameError> {
    let len = get_decimal(src)? as usize;
    if src.remaining() < len {
        return Err(FrameError::Incomplete);
    }
    let data = Bytes::copy_from_slice(&src.chunk()[..len]);
    skip(src, len)?;
    Ok(data.to_vec())
}

async fn write_data<W>(w: &mut W, data: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    w.write_u64(data.len() as u64).await?;
    w.write_all(data.as_ref()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::io::BufWriter;

    #[tokio::test]
    async fn test_frame() {
        let mut buffer: Vec<u8> = Vec::new();
        let mut stream = BufWriter::new(&mut buffer);

        // Write: GET, GET, POST, ERROR, GET
        let frames = [
            Frame {
                data: b"test data01".to_vec(),
            },
            Frame {
                data: b"test data02".to_vec(),
            },
            Frame {
                data: b"hello world".to_vec(),
            },
            Frame { data: b"".to_vec() },
            Frame {
                data: b"This is a new data".to_vec(),
            },
        ];
        for frame in frames.iter() {
            frame.write(&mut stream).await.unwrap();
        }
        stream.flush().await.unwrap();
        drop(stream);

        for expect in frames {
            let mut cursor = Cursor::new(&buffer[..]);
            let (frame, len) = Frame::parse(&mut cursor).unwrap().unwrap();
            assert_eq!(frame, expect);
            buffer.drain(..len);
        }
    }
}
