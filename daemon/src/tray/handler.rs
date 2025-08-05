use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Result};
use csync_misc::api::blob::Blob;
use csync_misc::api::metadata::BlobType;
use csync_misc::config::PathSet;
use csync_misc::imghdr;
use log::{debug, info};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::mpsc;

use crate::remote::Remote;

#[derive(Debug, Clone)]
pub struct TrayHandler {
    remote: Remote,
    ps: PathSet,
    copy_tx: mpsc::Sender<Vec<u8>>,
}

impl TrayHandler {
    pub fn new(remote: Remote, ps: PathSet, copy_tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self {
            remote,
            ps,
            copy_tx,
        }
    }

    pub async fn handle_event(&self, app: AppHandle, id: &str) -> Result<Option<&'static str>> {
        match id {
            "quit" => {
                info!("Quit application");
                app.exit(0);
                Ok(None)
            }
            "client_config" => self.handle_open_config(app, "client"),
            "daemon_config" => self.handle_open_config(app, "daemon"),
            "logs" => self.handle_open_logs(app),
            _ => {
                self.handle_event_innr(app, id).await?;
                Ok(None)
            }
        }
    }

    async fn handle_event_innr(&self, app: AppHandle, id: &str) -> Result<()> {
        debug!("Handle tray event: {id}");

        if let Some(("copy", id)) = self.parse_action_id(id) {
            return self.handle_copy(id).await;
        }

        if let Some(("open", id)) = self.parse_action_id(id) {
            return self.handle_open(app, id).await;
        }

        if let Some(("save", id)) = self.parse_action_id(id) {
            return self.handle_save(app, id).await;
        }

        if let Some(("pin", id)) = self.parse_action_id(id) {
            self.remote.pin_blob(id, true).await?;
            return Ok(());
        }

        if let Some(("unpin", id)) = self.parse_action_id(id) {
            self.remote.pin_blob(id, false).await?;
            return Ok(());
        }

        if let Some(("delete", id)) = self.parse_action_id(id) {
            self.remote.delete_blob(id).await?;
            return Ok(());
        }

        if id.starts_with("upload_") {
            let kind = id.strip_prefix("upload_").unwrap();
            return self.handle_upload(app, kind).await;
        }

        bail!("{id}: no handler");
    }

    async fn handle_copy(&self, id: u64) -> Result<()> {
        let blob = self.remote.get_blob(id).await?;
        self.copy_tx.send(blob.data).await?;

        Ok(())
    }

    async fn handle_open(&self, app: AppHandle, id: u64) -> Result<()> {
        let blob = self.remote.get_blob(id).await?;

        let path = match blob.blob_type {
            BlobType::Text => {
                let path = self.ps.tmp_dir.join("text.txt");
                fs::write(&path, &blob.data)?;
                path
            }
            BlobType::Image => {
                let path = self.ps.tmp_dir.join("image.png");
                fs::write(&path, &blob.data)?;
                path
            }
            BlobType::File => blob.write_file_to_dir(&self.ps.tmp_dir)?,
        };

        let opener = app.opener();
        let path = format!("{}", path.display());
        opener.open_path(&path, None::<&str>)?;

        Ok(())
    }

    async fn handle_save(&self, app: AppHandle, id: u64) -> Result<()> {
        let blob = self.remote.get_blob(id).await?;

        let path = match blob.blob_type {
            BlobType::Text => app
                .dialog()
                .file()
                .set_title("Save Text")
                .blocking_save_file(),
            BlobType::Image => app
                .dialog()
                .file()
                .set_title("Save Image")
                .add_filter("Image", &["png", "jpg", "jpeg"])
                .blocking_save_file(),
            BlobType::File => app
                .dialog()
                .file()
                .set_title("Save File")
                .blocking_pick_folder(),
        };

        if let Some(path) = path {
            let path = PathBuf::from(path.to_string());
            if matches!(blob.blob_type, BlobType::File) {
                blob.write_file_to_dir(&path)?;
            } else {
                fs::write(&path, &blob.data)?;
            }
        }

        Ok(())
    }

    async fn handle_upload(&self, app: AppHandle, kind: &str) -> Result<()> {
        let (path, blob_type) = match kind {
            // FIXME: Currently, Tauri's dialog doesn't support customizing the popup
            // position. This poses a problem for pure system tray applications like ours.
            // Specifically, on macOS, the dialog pops up above the menu bar, which isn't
            // user-friendly. We currently have no effective solution for this issue.
            // We need to wait for official support, see:
            //     <https://github.com/tauri-apps/plugins-workspace/issues/1306>
            "text" => (
                app.dialog()
                    .file()
                    .set_title("Upload Text")
                    .blocking_pick_file(),
                BlobType::Text,
            ),
            "image" => (
                app.dialog()
                    .file()
                    .set_title("Upload Image")
                    .add_filter("Image", &["png", "jpg", "jpeg"])
                    .blocking_pick_file(),
                BlobType::Image,
            ),
            "file" => (
                app.dialog()
                    .file()
                    .set_title("Upload File")
                    .blocking_pick_file(),
                BlobType::File,
            ),
            _ => unreachable!(),
        };

        if let Some(path) = path {
            let path = PathBuf::from(path.to_string());
            let blob = if matches!(blob_type, BlobType::File) {
                Blob::read_from_file(&path)?
            } else {
                let data = fs::read(&path)?;
                match blob_type {
                    BlobType::Text => {
                        let text = match String::from_utf8(data) {
                            Ok(text) => text,
                            Err(_) => bail!("file content is not utf8 text"),
                        };
                        Blob::new_text(text)
                    }
                    BlobType::Image => {
                        if !imghdr::is_data_image(&data) {
                            bail!("file content is not an image");
                        }
                        Blob::new_image(data)
                    }
                    _ => unreachable!(),
                }
            };

            self.remote.put_blob(blob).await?;
        }

        Ok(())
    }

    fn handle_open_config(&self, app: AppHandle, kind: &str) -> Result<Option<&'static str>> {
        let path = self.ps.config_dir.join(format!("{kind}.toml"));
        if !path.exists() {
            return Ok(Some("No config file found"));
        }

        let opener = app.opener();
        let path = format!("{}", path.display());
        opener.open_path(&path, None::<&str>)?;
        Ok(None)
    }

    fn handle_open_logs(&self, app: AppHandle) -> Result<Option<&'static str>> {
        let path = self.ps.data_dir.join("logs").join("daemon.log");
        if !path.exists() {
            return Ok(Some("No log file found"));
        }

        let opener = app.opener();
        let path = format!("{}", path.display());
        opener.open_path(&path, None::<&str>)?;
        Ok(None)
    }

    fn parse_action_id<'a>(&self, id: &'a str) -> Option<(&'a str, u64)> {
        let fields = id.split('_').collect::<Vec<_>>();
        if fields.len() != 2 {
            return None;
        }

        let id: u64 = fields[1].parse().ok()?;
        Some((fields[0], id))
    }
}
