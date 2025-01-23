use anyhow::{bail, Result};

use crate::imghdr::is_data_image;

use super::exec::{check_command, execute_read_command, execute_write_command};

const MIME_TEXT: &str = "text/plain";
const MIME_IMAGE: &str = "image/png";

/// X11 clipboard selection to use (-selection clipboard)
const SELECTION: &str = "-selection";
const CLIPBOARD: &str = "clipboard";

/// Target type argument (-t)
const TARGET: &str = "-t";

/// Checks if xclip command is available
pub fn check() -> Result<()> {
    check_command("xclip", &["-version"])?;
    Ok(())
}

/// Reads text content from X11 clipboard using xclip
pub fn read_text() -> Result<Option<String>> {
    let mime = get_mime()?;
    if !mime.contains(MIME_TEXT) {
        return Ok(None);
    }

    let data = execute_read_command("xclip", &[SELECTION, CLIPBOARD, TARGET, MIME_TEXT, "-o"])?;
    let text = String::from_utf8(data).ok();
    Ok(text)
}

/// Writes text content to X11 clipboard using xclip
pub fn write_text(text: String) -> Result<()> {
    execute_write_command(
        "xclip",
        &[SELECTION, CLIPBOARD, TARGET, MIME_TEXT],
        text.as_bytes(),
    )
}

/// Reads image data from X11 clipboard
pub fn read_image() -> Result<Option<Vec<u8>>> {
    let mime = get_mime()?;
    if !mime.contains(MIME_IMAGE) {
        return Ok(None);
    }

    let data = execute_read_command("xclip", &[SELECTION, CLIPBOARD, TARGET, MIME_IMAGE, "-o"])?;
    if !is_data_image(&data) {
        return Ok(None);
    }

    Ok(Some(data))
}

/// Writes image data to X11 clipboard using xclip
pub fn write_image(data: Vec<u8>) -> Result<()> {
    if !is_data_image(&data) {
        bail!("data is not a valid image");
    }

    execute_write_command("xclip", &[SELECTION, CLIPBOARD, TARGET, MIME_IMAGE], &data)
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

/// Gets MIME type of current clipboard content
fn get_mime() -> Result<String> {
    let data = execute_read_command("xclip", &[SELECTION, CLIPBOARD, "-t", "TARGETS", "-o"])?;
    let mime = String::from_utf8(data).unwrap_or_default();
    Ok(mime)
}
