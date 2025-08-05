use anyhow::{bail, Result};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};

#[inline(always)]
pub fn base64_encode<T>(input: T) -> String
where
    T: AsRef<[u8]>,
{
    BASE64_STANDARD.encode(input)
}

#[inline(always)]
pub fn base64_decode<T>(input: T) -> Result<Vec<u8>>
where
    T: AsRef<[u8]>,
{
    match BASE64_STANDARD.decode(input) {
        Ok(data) => Ok(data),
        Err(_) => bail!("invalid base64 string"),
    }
}

#[inline(always)]
pub fn base64_decode_string<T>(input: T) -> Result<String>
where
    T: AsRef<[u8]>,
{
    let data = base64_decode(input)?;
    match String::from_utf8(data) {
        Ok(s) => Ok(s),
        Err(_) => bail!("invalid utf8 string"),
    }
}

#[inline(always)]
pub fn sha256<T>(input: T) -> String
where
    T: AsRef<[u8]>,
{
    let hash = Sha256::digest(input);
    format!("{hash:x}")
}
