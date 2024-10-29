use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::{fs, io};

use log::{debug, error, info};
use tokio::sync::mpsc;

use crate::clipboard::Clipboard;
use crate::net::{Client, DataItem};

pub struct Sync {
    clipboard_watch_rx: mpsc::Receiver<Vec<u8>>,
    clipboard_write_tx: mpsc::Sender<Vec<u8>>,

    server_watch_rx: mpsc::Receiver<DataItem>,
    server_write_tx: mpsc::Sender<DataItem>,

    download_dir: String,
}

impl Sync {
    pub fn new(clipboard: Clipboard, client: Client, download_dir: String) -> Self {
        let (clipboard_watch_rx, clipboard_write_tx) = clipboard.start();
        let (server_watch_rx, server_write_tx) = client.start();
        Self {
            clipboard_watch_rx,
            clipboard_write_tx,
            server_watch_rx,
            server_write_tx,
            download_dir,
        }
    }

    pub async fn start(&mut self) {
        info!("[sync] begin to sync clipboard and server");
        loop {
            tokio::select! {
                Some(data) = self.clipboard_watch_rx.recv() => {
                    self.handle_clipboard(data).await;
                },
                Some(data_item) = self.server_watch_rx.recv() => {
                    self.handle_server(data_item).await;
                },
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
}
