use std::{fs, io};

use anyhow::{Context, Result};

// TODO: Support Windows
const IGNORE_PATH: &str = "/tmp/csync_ignore";

#[inline]
pub fn save<S: AsRef<str>>(digest: S) -> Result<()> {
    fs::write(IGNORE_PATH, digest.as_ref().as_bytes())
        .with_context(|| format!("write digest to file '{IGNORE_PATH}'"))
}

#[inline]
pub fn load() -> Result<Option<String>> {
    match fs::read(IGNORE_PATH) {
        Ok(data) => {
            let digest = String::from_utf8(data).context("decode digest in ignore file")?;
            Ok(Some(digest))
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("read digest from file '{IGNORE_PATH}'")),
    }
}

#[inline]
pub fn remove() -> Result<()> {
    fs::remove_file(IGNORE_PATH).with_context(|| format!("remove ignore file '{IGNORE_PATH}'"))
}
