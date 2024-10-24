use log::{debug, info};
use tokio::sync::mpsc;

use crate::clipboard::Clipboard;
use crate::net::{Client, DataItem};

pub struct Sync {
    clipboard_watch_rx: mpsc::Receiver<Vec<u8>>,
    clipboard_write_tx: mpsc::Sender<Vec<u8>>,

    server_watch_rx: mpsc::Receiver<DataItem>,
    server_write_tx: mpsc::Sender<DataItem>,
}

impl Sync {
    pub fn new(clipboard: Clipboard, client: Client) -> Self {
        let (clipboard_watch_rx, clipboard_write_tx) = clipboard.start();
        let (server_watch_rx, server_write_tx) = client.start();
        Self {
            clipboard_watch_rx,
            clipboard_write_tx,
            server_watch_rx,
            server_write_tx,
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
            DataItem::File(_file_item) => {
                unimplemented!()
            }
        }
    }
}
