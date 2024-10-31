use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::PathBuf;
use std::time::Duration;
use std::{fs, io};

use anyhow::{Context, Result};
use log::{debug, error, info};
use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::clipboard::Clipboard;
use crate::net::{Client, DataItem, FileItem};

pub struct Sync {
    clipboard_watch_rx: mpsc::Receiver<Vec<u8>>,
    clipboard_write_tx: mpsc::Sender<Vec<u8>>,

    server_watch_rx: mpsc::Receiver<DataItem>,
    server_write_tx: mpsc::Sender<DataItem>,

    download_dir: String,
    upload_dir: String,
}

impl Sync {
    const WATCH_UPLOAD_DIR_INTERVAL: Duration = Duration::from_secs(1);

    pub fn new(
        clipboard: Clipboard,
        client: Client,
        download_dir: String,
        upload_dir: String,
    ) -> Self {
        let (clipboard_watch_rx, clipboard_write_tx) = clipboard.start();
        let (server_watch_rx, server_write_tx) = client.start();
        Self {
            clipboard_watch_rx,
            clipboard_write_tx,
            server_watch_rx,
            server_write_tx,
            download_dir,
            upload_dir,
        }
    }

    pub async fn start(&mut self) {
        let mut watch_upload_intv =
            tokio::time::interval_at(Instant::now(), Self::WATCH_UPLOAD_DIR_INTERVAL);
        info!("[sync] begin to sync clipboard and server");
        loop {
            tokio::select! {
                Some(data) = self.clipboard_watch_rx.recv() => {
                    self.handle_clipboard(data).await;
                },
                Some(data_item) = self.server_watch_rx.recv() => {
                    self.handle_server(data_item).await;
                },
                _ = watch_upload_intv.tick() => {
                    self.watch_upload_dir().await;
                }
            }
        }
    }

    async fn handle_clipboard(&self, data: Vec<u8>) {
        debug!(
            "[sync] get {} bytes data from from clipboard, send to server",
            data.len()
        );
        let data_item = DataItem::Clipboard(data);
        self.server_write_tx.send(data_item).await.unwrap();
    }

    async fn handle_server(&self, data_item: DataItem) {
        match data_item {
            DataItem::Clipboard(data) => {
                debug!(
                    "[sync] get {} bytes data from server, write to clipboard",
                    data.len()
                );
                self.clipboard_write_tx.send(data).await.unwrap();
            }
            DataItem::File(file_item) => {
                let path = PathBuf::from(&self.download_dir).join(&file_item.name);
                debug!(
                    "[sync] get {} bytes data from server, write to file {}, with mode {}",
                    file_item.data.len(),
                    path.display(),
                    file_item.mode
                );

                // Ensure download dir is exists and it is a directory.
                match fs::metadata(&self.download_dir) {
                    Ok(meta) if !meta.is_dir() => {
                        error!(
                            "[sync] download dir {} is not a directory",
                            self.download_dir
                        );
                        return;
                    }
                    Err(err) if err.kind() == io::ErrorKind::NotFound => {
                        if let Err(err) = fs::create_dir_all(&self.download_dir) {
                            error!("[sync] create download dir error: {err:#}");
                            return;
                        }
                    }
                    _ => {}
                }

                let mut file = match fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    // TODO: Support Windows
                    .mode(file_item.mode as u32)
                    .open(&path)
                {
                    Ok(file) => file,
                    Err(err) => {
                        error!("[sync] open file {} error: {err:#}", path.display());
                        return;
                    }
                };

                if let Err(err) = file.write_all(&file_item.data) {
                    error!("[sync] write file {} error: {err:#}", path.display());
                }
            }
        }
    }

    async fn watch_upload_dir(&self) {
        if let Err(err) = self.watch_upload_dir_raw().await {
            error!("[sync] watch upload dir error: {err:#}");
        }
    }

    async fn watch_upload_dir_raw(&self) -> Result<()> {
        let (file_item, path) = match fs::read_dir(&self.upload_dir) {
            Ok(entries) => {
                let mut result = None;
                for entry in entries {
                    let entry = entry.context("read dir entry")?;
                    let metadata = entry.metadata().context("read dir entry metadata")?;
                    if !metadata.is_file() {
                        continue;
                    }
                    let name = match entry.file_name().to_str() {
                        Some(name) => String::from(name),
                        None => continue,
                    };
                    let path = PathBuf::from(&self.upload_dir).join(&name);
                    let data = fs::read(&path)
                        .with_context(|| format!("read file '{}'", path.display()))?;
                    result = Some((
                        FileItem {
                            name,
                            mode: metadata.mode() as u64,
                            data,
                        },
                        path,
                    ));
                }
                match result {
                    Some((data_item, path)) => (data_item, path),
                    None => return Ok(()),
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(err).context("read dir"),
        };

        debug!("[sync] get upload file from dir, with {} bytes data, name '{}', mode {}, send to server", file_item.data.len(), file_item.name, file_item.mode);
        self.server_write_tx
            .send(DataItem::File(file_item))
            .await
            .unwrap();

        fs::remove_file(path).context("remove file")?;

        Ok(())
    }
}
