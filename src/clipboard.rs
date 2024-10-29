use core::str;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::time::Duration;
use std::{env, fs};

use anyhow::{bail, Context, Result};
use log::{debug, error, info};
use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::hash::get_hash;

#[derive(Debug, Clone, Copy)]
enum ClipboardType {
    Wayland,
    X11,
    Macos,
    Windows,
}

pub struct Clipboard {
    clip_type: ClipboardType,

    current_hash: String,

    read_interval: u64,
}

impl Clipboard {
    const MIN_READ_INTERVAL: u64 = 200;
    const MAX_READ_INTERVAL: u64 = 10000;

    pub fn build(read_interval: u64) -> Result<Self> {
        if read_interval < Self::MIN_READ_INTERVAL {
            bail!(
                "clipboard read interval should be at least {}ms",
                Self::MIN_READ_INTERVAL
            );
        }
        if read_interval > Self::MAX_READ_INTERVAL {
            bail!(
                "clipboard read interval should be at most {}ms",
                Self::MAX_READ_INTERVAL
            );
        }

        Ok(Self {
            clip_type: Self::get_clipboard_type()?,
            current_hash: String::new(),
            read_interval,
        })
    }

    pub fn start(mut self) -> (mpsc::Receiver<Vec<u8>>, mpsc::Sender<Vec<u8>>) {
        let (watch_tx, watch_rx) = mpsc::channel::<Vec<u8>>(500);
        let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(500);

        let mut read_intv =
            tokio::time::interval_at(Instant::now(), Duration::from_millis(self.read_interval));
        tokio::spawn(async move {
            info!(
                "[clipboard] start handling loop, with read interval {}ms, and clipboard type {:?}",
                self.read_interval, self.clip_type
            );
            loop {
                tokio::select! {
                    _ = read_intv.tick() => {
                        let current_data = match self.read_raw() {
                            Ok(data) => data,
                            Err(err) => {
                                error!("[clipboard] read clipboard error: {err:#}");
                                continue;
                            },
                        };
                        if current_data.is_empty() {
                            continue;
                        }
                        let hash = get_hash(&current_data);
                        if self.current_hash == hash {
                            continue;
                        }
                        debug!("[clipboard] send {} data to watch channel", current_data.len());
                        self.current_hash = hash;
                        watch_tx.send(current_data).await.unwrap();
                    },
                    Some(data) = write_rx.recv() => {
                        let hash = get_hash(&data);
                        if self.current_hash == hash {
                            continue;
                        }
                        debug!("[clipboard] write {} data to clipboard", data.len());
                        if let Err(err) = self.write_raw(&data) {
                                error!("[clipboard] write clipboard error: {err:#}");
                            continue;
                        }
                        self.current_hash = hash;
                    },
                }
            }
        });
        (watch_rx, write_tx)
    }

    fn write_raw(&self, data: &[u8]) -> Result<()> {
        if str::from_utf8(data).is_err() {
            // If the data is not encoded in UTF-8, we assume it is a PNG image
            // (usually, users cannot copy other binary data to the clipboard).
            // For PNG image data, different systems may require different handling
            // logic compared to text. For example, in MacOS, the `pbcopy` command can
            // only handle text data, so images need to be saved to a file and then
            // processed using other commands. But in Wayland, the `wl-copy` can handle
            // both text and image, so we don't need add extra method to handle Wayland
            // image data.
            // TODO: Of course, this could be a hasty judgment, and we may be able to
            // parse the data and determine it based on the PNG file header.
            if let ClipboardType::Macos = self.clip_type {
                return self.macos_write_image(data);
            }
        }

        let mut copy_cmd = self.new_copy_cmd();
        // Write the data to copy command's stdin
        copy_cmd.stdin(Stdio::piped());

        let mut child = match copy_cmd.spawn() {
            Ok(child) => child,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                let program = copy_cmd.get_program().to_string_lossy();
                bail!("cannot find clipboard command '{program}' in your system, please install it first");
            }
            Err(err) => return Err(err).context("launch clipboard copy command"),
        };

        let stdin = child.stdin.as_mut().unwrap();
        if let Err(err) = stdin.write_all(data) {
            return Err(err).context("write data to clipboard copy command");
        }
        drop(child.stdin.take());

        let status = child.wait().context("wait clipboard copy command done")?;
        if !status.success() {
            let code = status
                .code()
                .map(|code| code.to_string())
                .unwrap_or("<unknown>".to_string());
            bail!("clipboard copy command exited with bad code {code}");
        }

        Ok(())
    }

    pub fn read_raw(&self) -> Result<Vec<u8>> {
        // TODO: MacOS cannot read PNG image data by using `pbpaste` command.
        let mut paste_cmd = self.new_paste_cmd();
        // Read the data from paste command's stdout
        paste_cmd.stdout(Stdio::piped());

        let output = paste_cmd
            .output()
            .context("execute clipboard paste command")?;

        if !output.status.success() {
            let code = output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or("<unknown>".to_string());
            if let ClipboardType::Wayland = self.clip_type {
                if code == "1" {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if stderr.trim() == "Nothing is copied" {
                        return Ok(Vec::new());
                    }
                }
            }
            bail!("clipboard paste command exited with bad code {code}");
        }

        Ok(output.stdout)
    }

    fn new_copy_cmd(&self) -> Command {
        match self.clip_type {
            ClipboardType::Wayland => Command::new("wl-copy"),
            ClipboardType::X11 => {
                let mut cmd = Command::new("xclip");
                cmd.arg("-selection").arg("clipboard");
                cmd
            }
            ClipboardType::Macos => Command::new("pbcopy"),
            ClipboardType::Windows => Command::new("clip"),
        }
    }

    fn new_paste_cmd(&self) -> Command {
        match self.clip_type {
            ClipboardType::Wayland => {
                let mut cmd = Command::new("wl-paste");
                cmd.arg("--no-newline");
                cmd
            }
            ClipboardType::X11 => {
                let mut cmd = Command::new("xclip");
                cmd.arg("-selection").arg("clipboard");
                cmd.arg("-o");
                cmd
            }
            ClipboardType::Macos => Command::new("pbpaste"),
            ClipboardType::Windows => {
                let mut cmd = Command::new("powershell.exe");
                cmd.arg("Get-Clipboard");
                cmd
            }
        }
    }

    fn macos_write_image(&self, data: &[u8]) -> Result<()> {
        // Write image data to a temporary file, use `osascript` to copy it.
        // See: https://stackoverflow.com/questions/6919403/set-clipboard-to-image-pbcopy
        let home_dir = match dirs::home_dir() {
            Some(dir) => dir,
            None => bail!("cannot get home directory for macos"),
        };

        let tmp_path = home_dir.join(".csync_image.png");
        fs::write(&tmp_path, data).context("write image data to temporary file")?;

        let mut cmd = Command::new("osascript");
        cmd.arg("-e");
        cmd.arg(format!(
            "set the clipboard to (read (POSIX file \"{}\") as JPEG picture)",
            tmp_path.display()
        ));

        let status = cmd
            .status()
            .context("execute macos clipboard image copy command")?;

        fs::remove_file(tmp_path).context("remove temporary image file")?;

        if !status.success() {
            bail!("macos clipboard image copy command exited with bad code");
        }

        Ok(())
    }

    fn get_clipboard_type() -> Result<ClipboardType> {
        match env::consts::OS {
            "linux" => {
                if env::var("WAYLAND_DISPLAY").is_ok() {
                    Ok(ClipboardType::Wayland)
                } else {
                    Ok(ClipboardType::X11)
                }
            }
            "macos" => Ok(ClipboardType::Macos),
            "windows" => Ok(ClipboardType::Windows),
            _ => bail!("unsupported os {}", env::consts::OS),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// Start test case:
    ///
    /// ```
    /// TEST_CLIPBOARD="true" cargo test clipboard::tests::test_clipboard -- --nocapture
    /// echo -n "Some content" > .test_clipboard
    /// ```
    #[tokio::test]
    async fn test_clipboard() {
        if env::var("TEST_CLIPBOARD").is_err() {
            return;
        }

        let clipboard = Clipboard::build(300).unwrap();
        let (mut watch_rx, write_tx) = clipboard.start();

        let mut trigger_intv = tokio::time::interval_at(Instant::now(), Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = trigger_intv.tick() => {
                    let data = fs::read(".test_clipboard").unwrap_or_default();
                    if data.is_empty() {
                        continue;
                    }
                    println!(
                        "Write data to clipboard: {}",
                        String::from_utf8_lossy(&data)
                    );
                    write_tx.send(data).await.unwrap();

                    fs::remove_file(".test_clipboard").unwrap();
                },
                Some(data) = watch_rx.recv() => {
                    println!(
                        "Read data from clipboard: {}",
                        String::from_utf8_lossy(&data)
                    );
                },
            }
        }
    }
}
