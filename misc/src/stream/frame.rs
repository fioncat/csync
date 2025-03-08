use std::io::Cursor;

use anyhow::Result;
use bytes::{Buf, Bytes};
use thiserror::Error;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::cipher::Cipher;

/// Writes data to an asynchronous writer, optionally encrypting it first.
///
/// The data is written in a frame format consisting of:
/// 1. A 64-bit length prefix indicating the size of the data
/// 2. The actual data (optionally encrypted)
///
/// # Arguments
///
/// * `w` - The asynchronous writer to write to
/// * `data` - The data to write
/// * `cipher` - Optional cipher for encrypting the data
///
/// # Returns
///
/// * `Ok(())` - If the data was successfully written
/// * `Err` - If an error occurred during writing or encryption
pub async fn write_data<W>(w: &mut W, data: &[u8], cipher: &Option<Cipher>) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    if let Some(ref cipher) = cipher {
        let data = match cipher.encrypt(data) {
            Ok(data) => data,
            Err(_) => return Err(FrameError::Auth.into()),
        };

        w.write_u64(data.len() as u64).await?;
        w.write_all(data.as_ref()).await?;
        return Ok(());
    }

    w.write_u64(data.len() as u64).await?;
    w.write_all(data.as_ref()).await?;
    Ok(())
}

/// Attempts to read the next frame of data from a buffer.
///
/// This function checks if a complete frame is available in the buffer,
/// and if so, parses and returns it along with the total length consumed.
///
/// # Arguments
///
/// * `src` - Cursor over the buffer to read from
/// * `cipher` - Optional cipher for decrypting the data
///
/// # Returns
///
/// * `Ok(Some((Vec<u8>, usize)))` - The parsed data and the number of bytes consumed
/// * `Ok(None)` - If there is not enough data to parse a complete frame
/// * `Err` - If an error occurred during parsing or decryption
pub fn next_data(
    src: &mut Cursor<&[u8]>,
    cipher: &Option<Cipher>,
) -> Result<Option<(Vec<u8>, usize)>, FrameError> {
    match check_data(src) {
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
            let frame = parse_data(src, cipher)?;

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

/// Errors that can occur when working with frames.
#[derive(Debug, Error)]
pub enum FrameError {
    /// Indicates that there is not enough data available to parse a complete frame.
    #[error("not enough data is available to parse a message")]
    Incomplete,

    /// Indicates an authentication error during encryption or decryption.
    #[error("incorrect password to encode or decode data")]
    Auth,
}

/// Parses a frame of data from a buffer, optionally decrypting it.
///
/// # Arguments
///
/// * `src` - Cursor over the buffer to read from
/// * `cipher` - Optional cipher for decrypting the data
///
/// # Returns
///
/// * `Ok(Vec<u8>)` - The parsed and optionally decrypted data
/// * `Err` - If an error occurred during parsing or decryption
fn parse_data(src: &mut Cursor<&[u8]>, cipher: &Option<Cipher>) -> Result<Vec<u8>, FrameError> {
    let data = get_data(src)?;
    if let Some(ref cipher) = cipher {
        return match cipher.decrypt(&data) {
            Ok(data) => Ok(data),
            Err(_) => Err(FrameError::Auth),
        };
    }

    Ok(data)
}

/// Checks if a complete frame is available in the buffer.
///
/// This function advances the cursor to the end of the frame if it's complete.
///
/// # Arguments
///
/// * `src` - Cursor over the buffer to check
///
/// # Returns
///
/// * `Ok(())` - If a complete frame is available
/// * `Err` - If there is not enough data for a complete frame
fn check_data(src: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
    let len = get_decimal(src)? as usize;
    skip(src, len)?;
    Ok(())
}

/// Reads a frame of data from a buffer.
///
/// # Arguments
///
/// * `src` - Cursor over the buffer to read from
///
/// # Returns
///
/// * `Ok(Vec<u8>)` - The data read from the buffer
/// * `Err` - If there is not enough data available
fn get_data(src: &mut Cursor<&[u8]>) -> Result<Vec<u8>, FrameError> {
    let len = get_decimal(src)? as usize;
    if src.remaining() < len {
        return Err(FrameError::Incomplete);
    }
    let data = Bytes::copy_from_slice(&src.chunk()[..len]);
    skip(src, len)?;
    Ok(data.to_vec())
}

/// Reads a 64-bit unsigned integer from the buffer.
///
/// # Arguments
///
/// * `src` - Cursor over the buffer to read from
///
/// # Returns
///
/// * `Ok(u64)` - The 64-bit unsigned integer read from the buffer
/// * `Err` - If there is not enough data available
fn get_decimal(src: &mut Cursor<&[u8]>) -> Result<u64, FrameError> {
    if src.remaining() < 8 {
        return Err(FrameError::Incomplete);
    }
    Ok(src.get_u64())
}

/// Advances the cursor by `n` bytes.
///
/// # Arguments
///
/// * `src` - Cursor over the buffer to advance
/// * `n` - Number of bytes to advance
///
/// # Returns
///
/// * `Ok(())` - If the cursor was successfully advanced
/// * `Err` - If there is not enough data available
fn skip(src: &mut Cursor<&[u8]>, n: usize) -> Result<(), FrameError> {
    if src.remaining() < n {
        return Err(FrameError::Incomplete);
    }
    src.advance(n);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::io::BufWriter;

    async fn run_test(cipher: Option<Cipher>) {
        let mut buffer: Vec<u8> = Vec::new();
        let mut stream = BufWriter::new(&mut buffer);

        let datas = [
            b"test data01".to_vec(),
            b"test data02".to_vec(),
            b"hello world".to_vec(),
            b"".to_vec(),
            b"This is a new data".to_vec(),
        ];
        for data in datas.iter() {
            write_data(&mut stream, data, &cipher).await.unwrap();
        }
        stream.flush().await.unwrap();
        drop(stream);

        for expect in datas {
            let mut cursor = Cursor::new(&buffer[..]);
            let (frame, len) = next_data(&mut cursor, &cipher).unwrap().unwrap();
            assert_eq!(frame, expect);
            buffer.drain(..len);
        }
    }

    #[tokio::test]
    async fn test_no_auth() {
        run_test(None).await;
    }

    #[tokio::test]
    async fn test_with_auth() {
        let passwords = vec!["test123", "password", "hello", ""];
        for password in passwords {
            let cipher = Some(Cipher::new(password.as_bytes().to_vec()));
            run_test(cipher).await;
        }
    }
}
