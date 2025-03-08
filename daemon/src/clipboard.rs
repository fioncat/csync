use std::time::Duration;

use anyhow::{Context, Result};
use csync_misc::api::blob::Blob;
use csync_misc::api::metadata::{BlobType, Event, EventType};
use csync_misc::clipboard::Clipboard;
use csync_misc::{code, imghdr};
use log::{debug, error, info, warn};
use tokio::select;
use tokio::sync::mpsc;

use crate::remote::Remote;

pub struct ClipboardSync {
    remote: Remote,

    dirty_text: Option<Blob>,
    dirty_image: Option<Blob>,

    clipboard: Clipboard,

    clipboard_text_sha256: Option<String>,
    clipboard_image_sha256: Option<String>,

    copy_rx: mpsc::Receiver<Vec<u8>>,

    clipboard_secs: u64,
}

impl ClipboardSync {
    pub fn start(remote: Remote, clipboard_secs: u64) -> Result<mpsc::Sender<Vec<u8>>> {
        let cb = Clipboard::load().context("load clipboard")?;

        let (copy_tx, copy_rx) = mpsc::channel(200);

        let cs = Self {
            remote,
            dirty_text: None,
            dirty_image: None,
            clipboard: cb,
            clipboard_text_sha256: None,
            clipboard_image_sha256: None,
            copy_rx,
            clipboard_secs,
        };
        tokio::spawn(async move {
            cs.main_loop().await;
        });

        Ok(copy_tx)
    }

    async fn main_loop(mut self) {
        let mut clipboard_intv = tokio::time::interval(Duration::from_secs(self.clipboard_secs));
        let mut events_sub = self.remote.subscribe().await;

        info!("Begin to sync clipboard and server");
        loop {
            select! {
                event = events_sub.events.recv() => {
                    if let Err(e) = self.handle_event(event.unwrap()).await {
                        error!("Handle event error: {:?}", e);
                    }
                }

                _ = clipboard_intv.tick() => {
                    if let Err(e) = self.sync_clipboard().await {
                        error!("Sync clipboard error: {:?}", e);
                    }
                }

                Some(data) = self.copy_rx.recv() => {
                    if let Err(e) = self.handle_copy(data) {
                        error!("Handle copy error: {:?}", e);
                    }
                }

                state = events_sub.states.recv() => {
                    debug!("Clipboard sync: event state updated to {}", state.unwrap());
                },
            }
        }
    }

    async fn handle_event(&mut self, mut event: Event) -> Result<()> {
        if !matches!(event.event_type, EventType::Put) {
            return Ok(());
        }

        let item = match event.items.pop() {
            Some(item) => item,
            None => return Ok(()),
        };

        match item.blob_type {
            BlobType::Text => {
                if let Some(ref last_sha256) = self.clipboard_text_sha256 {
                    if last_sha256 == &item.blob_sha256 {
                        debug!(
                            "Text with sha256 {} is equals to clipboard, skip",
                            item.blob_sha256
                        );
                        return Ok(());
                    }
                }
            }
            BlobType::Image => {
                if let Some(ref last_sha256) = self.clipboard_image_sha256 {
                    if last_sha256 == &item.blob_sha256 {
                        debug!(
                            "Image with sha256 {} is equals to clipboard, skip",
                            item.blob_sha256
                        );
                        return Ok(());
                    }
                }
            }
            BlobType::File => return Ok(()),
        }

        let blob = self.remote.get_blob(item.id).await?;

        match blob.blob_type {
            BlobType::Text => {
                self.dirty_text = Some(blob);
            }
            BlobType::Image => {
                self.dirty_image = Some(blob);
            }
            BlobType::File => {
                warn!("Received file blob, ignore it");
            }
        }

        Ok(())
    }

    async fn sync_clipboard(&mut self) -> Result<()> {
        let mut dirty = false;

        if let Some(blob) = self.dirty_text.take() {
            let text = String::from_utf8(blob.data)?;
            self.clipboard.write_text(text)?;

            let new_text = self.clipboard.read_text()?;
            self.clipboard_text_sha256 = new_text.map(code::sha256);

            info!(
                "Write dirty text to clipboard done, new sha256: {:?}",
                self.clipboard_text_sha256
            );
            dirty = true;
        }

        if let Some(blob) = self.dirty_image.take() {
            let data = blob.data;
            if !imghdr::is_data_image(&data) {
                warn!("Sync: dirty image from server is not valid, ignore it");
                return Ok(());
            }

            self.clipboard.write_image(data)?;

            let new_image = self.clipboard.read_image()?;
            self.clipboard_image_sha256 = new_image.map(code::sha256);

            info!(
                "Write dirty image to clipboard done, new sha256: {:?}",
                self.clipboard_image_sha256
            );
            dirty = true;
        }

        if dirty {
            return Ok(());
        }

        let current_text = self.clipboard.read_text()?;
        let current_sha256 = current_text.as_ref().map(code::sha256);
        if self.clipboard_text_sha256 != current_sha256 {
            self.clipboard_text_sha256 = current_sha256;
            if let Some(text) = current_text {
                let blob = Blob::new_text(text);
                info!(
                    "Clipboard text updated, sha256: {}, push to server",
                    blob.sha256
                );
                self.remote.put_blob(blob).await?;
            } else {
                info!("Clipboard text set to empty");
            }
        }

        let current_image = self.clipboard.read_image()?;
        let current_sha256 = current_image.as_ref().map(code::sha256);
        if self.clipboard_image_sha256 != current_sha256 {
            self.clipboard_image_sha256 = current_sha256;
            if let Some(image) = current_image {
                let blob = Blob::new_image(image);
                info!(
                    "Clipboard image updated, sha256: {}, push to server",
                    blob.sha256
                );
                self.remote.put_blob(blob).await?;
            } else {
                info!("Clipboard image set to empty");
            }
        }

        Ok(())
    }

    fn handle_copy(&mut self, data: Vec<u8>) -> Result<()> {
        if imghdr::is_data_image(&data) {
            self.clipboard.write_image(data)?;

            let new_image = self.clipboard.read_image()?;
            let new_sha256 = new_image.as_ref().map(code::sha256);
            self.clipboard_image_sha256 = new_sha256;

            info!(
                "Copy image to clipboard done, with sha256: {:?}",
                self.clipboard_image_sha256
            );

            return Ok(());
        }

        if let Ok(text) = String::from_utf8(data) {
            self.clipboard.write_text(text)?;

            let new_text = self.clipboard.read_text()?;
            let new_sha256 = new_text.as_ref().map(code::sha256);
            self.clipboard_text_sha256 = new_sha256;

            info!(
                "Copy text to clipboard done, with sha256: {:?}",
                self.clipboard_text_sha256
            );

            return Ok(());
        }

        warn!("Copy data is not valid text or image, ignore it");
        Ok(())
    }
}
