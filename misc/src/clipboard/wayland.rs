use anyhow::{bail, Result};

use crate::imghdr::is_data_image;

use super::exec::{check_command, execute_read_command, execute_write_command};

/// MIME type for plain text content
const MIME_TEXT: &str = "text/plain";
/// MIME type for PNG image content
const MIME_IMAGE: &str = "image/png";

/// Checks if wl-clipboard tools are available in the system
///
/// Verifies both wl-copy and wl-paste commands are installed and working
pub fn check() -> Result<()> {
    check_command("wl-copy", &["-v"])?;
    check_command("wl-paste", &["-v"])?;
    Ok(())
}

/// Reads text content from Wayland clipboard
///
/// Uses wl-paste to get clipboard content, only returns text if MIME type matches text/plain
///
/// # Returns
/// - `Ok(Some(String))` if text content is found
/// - `Ok(None)` if clipboard is empty or content is not text
/// - `Err` if clipboard access fails
pub fn read_text() -> Result<Option<String>> {
    let mime = get_mime()?;
    if !mime.contains(MIME_TEXT) {
        return Ok(None);
    }

    let data = execute_read_command("wl-paste", &["--no-newline"])?;
    let text = String::from_utf8(data).ok();
    Ok(text)
}

/// Writes text content to Wayland clipboard using wl-copy
pub fn write_text(text: String) -> Result<()> {
    execute_write_command("wl-copy", &[], text.as_bytes())
}

/// Reads image data from Wayland clipboard
///
/// Uses wl-paste to get clipboard content, validates both MIME type and image data format
///
/// # Returns
/// - `Ok(Some(Vec<u8>))` if valid image data is found
/// - `Ok(None)` if clipboard is empty or content is not an image
/// - `Err` if clipboard access fails
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

/// Writes image data to Wayland clipboard using wl-copy
///
/// # Arguments
/// * `data` - Raw image data to write
///
/// # Returns
/// - `Ok(())` if write succeeds
/// - `Err` if data is not valid image or clipboard write fails
pub fn write_image(data: Vec<u8>) -> Result<()> {
    if !is_data_image(&data) {
        bail!("data is not a valid image");
    }

    execute_write_command("wl-copy", &[], &data)
}

/// Checks if clipboard content is an image by examining MIME type
pub fn is_image() -> Result<bool> {
    let mime = get_mime()?;
    Ok(mime.contains(MIME_IMAGE))
}

/// Checks if clipboard content is text by examining MIME type
pub fn is_text() -> Result<bool> {
    let mime = get_mime()?;
    Ok(mime.contains(MIME_TEXT))
}

/// Gets MIME type of current clipboard content using wl-paste --list-types
fn get_mime() -> Result<String> {
    let mime = execute_read_command("wl-paste", &["--list-types"])?;
    let mime = String::from_utf8(mime).unwrap_or_default();
    Ok(mime)
}
