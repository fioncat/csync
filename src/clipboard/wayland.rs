use anyhow::{bail, Result};

use crate::imghdr::is_data_image;

use super::exec::{check_command, execute_read_command, execute_write_command};

const MIME_TEXT: &str = "text/plain";
const MIME_IMAGE: &str = "image/png";

pub fn check() -> Result<()> {
    check_command("wl-copy", &["-v"])?;
    check_command("wl-paste", &["-v"])?;
    Ok(())
}

pub fn read_text() -> Result<Option<String>> {
    let mime = get_mime()?;
    if !mime.contains(MIME_TEXT) {
        return Ok(None);
    }

    let data = execute_read_command("wl-paste", &["--no-newline"])?;
    let text = String::from_utf8(data).ok();
    Ok(text)
}

pub fn write_text(text: String) -> Result<()> {
    execute_write_command("wl-copy", &[], text.as_bytes())
}

pub fn read_image() -> Result<Option<Vec<u8>>> {
    let mime = get_mime()?;
    if !mime.contains(MIME_IMAGE) {
        return Ok(None);
    }

    let data = execute_read_command("wl-paste", &[])?;
    if !is_data_image(&data) {
        return Ok(None);
    }

    Ok(Some(data))
}

pub fn write_image(data: Vec<u8>) -> Result<()> {
    if !is_data_image(&data) {
        bail!("data is not a valid image");
    }

    execute_write_command("wl-copy", &[], &data)
}

pub fn is_image() -> Result<bool> {
    let mime = get_mime()?;
    Ok(mime.contains(MIME_IMAGE))
}

pub fn is_text() -> Result<bool> {
    let mime = get_mime()?;
    Ok(mime.contains(MIME_TEXT))
}

fn get_mime() -> Result<String> {
    let mime = execute_read_command("wl-paste", &["--list-types"])?;
    let mime = String::from_utf8(mime).unwrap_or_default();
    Ok(mime)
}
