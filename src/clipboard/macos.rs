use std::fs;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

use crate::imghdr::{detect_data_image_type, ImageType};

use super::exec::{check_command, execute_read_command, execute_write_command};

const TMP_IMAGE_PATH: &str = "/tmp/clipboard_image.png";

pub fn check() -> Result<()> {
    check_command("pbcopy", &["-h"])?;
    check_command("pbpaste", &["-h"])?;
    Ok(())
}

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

pub fn write_text(text: String) -> Result<()> {
    execute_write_command("pbcopy", &[], text.as_bytes())
}

pub fn read_image() -> Result<Option<Vec<u8>>> {
    Ok(None)
}

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

pub fn is_image() -> Result<bool> {
    Ok(false)
}

pub fn is_text() -> Result<bool> {
    Ok(true)
}
