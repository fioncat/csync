use std::fs;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

use crate::imghdr::{detect_data_image_type, ImageType};

use super::exec::{check_command, execute_read_command, execute_write_command};

/// Temporary file path for image clipboard operations
const TMP_IMAGE_PATH: &str = "/tmp/clipboard_image.png";

/// Checks if required clipboard utilities are available
///
/// Verifies both pbcopy and pbpaste commands are installed and working
pub fn check() -> Result<()> {
    check_command("pbcopy", &["-h"])?;
    check_command("pbpaste", &["-h"])?;
    Ok(())
}

/// Reads text content from macOS clipboard using pbpaste
///
/// # Returns
/// - `Ok(Some(String))` if non-empty text content is found
/// - `Ok(None)` if clipboard is empty or contains only whitespace
/// - `Err` if clipboard access fails
pub fn read_text() -> Result<Option<String>> {
    let data = execute_read_command("pbpaste", &[])?;
    let text = String::from_utf8(data).ok();
    if let Some(ref text) = text {
        if text.trim().is_empty() {
            return Ok(None);
        }
    }
    Ok(text)
}

/// Writes text content to macOS clipboard using pbcopy
pub fn write_text(text: String) -> Result<()> {
    execute_write_command("pbcopy", &[], text.as_bytes())
}

/// Reads image data from macOS clipboard
///
/// Note: This operation is NOT supported on macOS due to system API limitations.
/// Always returns `Ok(None)`.
pub fn read_image() -> Result<Option<Vec<u8>>> {
    Ok(None)
}

/// Writes image data to macOS clipboard using AppleScript
///
/// This function:
/// 1. Validates the image format (PNG/JPEG only)
/// 2. Temporarily saves the image to /tmp
/// 3. Uses AppleScript to write the image to clipboard
/// 4. Cleans up the temporary file
///
/// # Arguments
/// * `data` - Raw image data (PNG or JPEG format)
///
/// # Returns
/// - `Ok(())` if write succeeds
/// - `Err` if data is not valid image or clipboard write fails
pub fn write_image(data: Vec<u8>) -> Result<()> {
    let image_type = match detect_data_image_type(&data) {
        ImageType::Png | ImageType::Jpeg => "JPEG",
        ImageType::Unknown => bail!("data is not a valid image"),
    };

    fs::write(TMP_IMAGE_PATH, &data).context("write image to tmp file")?;
    let script = format!(
        "set the clipboard to (read (POSIX file \"{TMP_IMAGE_PATH}\") as {image_type} picture)"
    );
    let mut cmd = Command::new("osascript");
    cmd.args(["-e", &script]);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::piped());

    let status = cmd.status()?;
    if !status.success() {
        bail!("osascript command exited with bad code, you can try execute: `osascript -e '{script}'` to see what's wrong");
    }

    fs::remove_file(TMP_IMAGE_PATH).context("remove tmp image file")?;
    Ok(())
}

/// Checks if clipboard content is an image
///
/// Note: Always returns `false` since image reading is not supported on macOS
pub fn is_image() -> Result<bool> {
    Ok(false)
}

/// Checks if clipboard content is text
///
/// Note: Always returns `true` since we can only reliably detect text content on macOS
pub fn is_text() -> Result<bool> {
    Ok(true)
}
