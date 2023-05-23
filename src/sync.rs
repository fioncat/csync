use std::borrow::Cow;
use std::collections::HashMap;
use std::net::SocketAddr;

use anyhow::{anyhow, Context, Result};
use arboard::Clipboard;
use log::error;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::{self, Duration, Instant, Interval};

use crate::config::Config;
use crate::net::{Client, Frame};

const INCORRECT_CLIPBOARD_TYPE_ERROR: &str = "incorrect type received from clipboard";

pub struct Synchronizer {
    conn_pool: HashMap<String, Client>,
    conn_expire: HashMap<String, Instant>,

    current_text_hash: Option<String>,
    current_image_hash: Option<String>,

    clipboard: Clipboard,

    receiver: Receiver<Frame>,

    clipboard_intv: Interval,
    expire_intv: Interval,
    expire_duration: Duration,
}

impl Synchronizer {
    pub async fn new(cfg: &Config) -> Result<(Synchronizer, Sender<Frame>)> {
        let conn_pool = HashMap::with_capacity(cfg.targets.len());
        let conn_expire = HashMap::with_capacity(cfg.targets.len());

        let mut clipboard = Clipboard::new().context("Init clipboard driver")?;
        let (sender, receiver) = mpsc::channel::<Frame>(cfg.conn_max as usize);

        let current_text = ClipboardText::read(&mut clipboard)?;
        let current_text_hash = match current_text {
            Some(text) => Some(text.get_hash()),
            None => None,
        };

        let current_image = ClipboardImage::read(&mut clipboard)?;
        let current_image_hash = match current_image {
            Some(image) => Some(image.hash),
            None => None,
        };

        let start = Instant::now();
        let clipboard_duration = Duration::from_millis(cfg.interval);
        let expire_duration = Duration::from_secs(cfg.conn_live as u64);

        let clipboard_intv = time::interval_at(start, clipboard_duration);
        let expire_intv = time::interval_at(start, expire_duration);

        let syncer = Synchronizer {
            conn_pool,
            conn_expire,

            current_text_hash,
            current_image_hash,

            clipboard,

            receiver,

            clipboard_intv,
            expire_intv,
            expire_duration,
        };

        Ok((syncer, sender))
    }

    pub async fn run(&mut self, targets: &[SocketAddr]) {
        use tokio::select;

        loop {
            select! {
                _ = self.clipboard_intv.tick() => {
                    if let Err(err) = self.send_clipboard_text(targets).await {
                        error!("Send clipboard text error: {err:#}");
                        continue;
                    }
                    if let Err(err) = self.send_clipboard_image(targets).await {
                        error!("Send clipboard image error: {err:#}");
                    }
                }
                _ = self.expire_intv.tick() => {
                    self.flush_conn();
                }
                frame = self.receiver.recv() => {
                    if let Some(frame) = frame {
                        if let Err(err) = self.recv_clipboard(frame) {
                            error!("Recv clipboard error: {err:#}");
                        }
                    }
                }
            }
        }
    }

    async fn get_conn(&mut self, target: &SocketAddr) -> Result<Client> {
        let addr = target.to_string();
        if let Some(conn) = self.conn_pool.remove(&addr) {
            return Ok(conn);
        }

        Client::dial(target).await
    }

    fn save_conn(&mut self, target: &SocketAddr, conn: Client) {
        let addr = target.to_string();
        self.conn_pool.insert(addr.clone(), conn);
        let expire = Instant::now().checked_add(self.expire_duration).unwrap();
        self.conn_expire.insert(addr, expire);
    }

    fn flush_conn(&mut self) {
        let now = Instant::now();
        for (addr, expire) in &self.conn_expire {
            if now >= *expire {
                self.conn_pool.remove(addr);
            }
        }
    }

    async fn send_clipboard_text(&mut self, targets: &[SocketAddr]) -> Result<()> {
        let text = match ClipboardText::read(&mut self.clipboard).context("Read clipboard text")? {
            Some(text) => text,
            None => return Ok(()),
        };
        let hash = text.get_hash();

        if let Some(current_text_hash) = &self.current_text_hash {
            if current_text_hash.eq(&hash) {
                return Ok(());
            }
        }

        let frame = Frame::Text(text.text);
        for target in targets {
            let mut conn = self.get_conn(target).await?;
            conn.write_frame(&frame).await?;
            self.save_conn(target, conn);
        }

        self.current_text_hash = Some(hash);

        Ok(())
    }

    async fn send_clipboard_image(&mut self, targets: &[SocketAddr]) -> Result<()> {
        let image =
            match ClipboardImage::read(&mut self.clipboard).context("Read clipboard image")? {
                Some(image) => image,
                None => return Ok(()),
            };

        if let Some(current_image_hash) = &self.current_image_hash {
            if current_image_hash.eq(&image.hash) {
                return Ok(());
            }
        }

        let ClipboardImage {
            width,
            height,
            data,
            hash,
        } = image;

        let frame = Frame::Image(width as u64, height as u64, data.into());
        for target in targets {
            let mut conn = self.get_conn(target).await?;
            conn.write_frame(&frame).await?;
            self.save_conn(target, conn);
        }

        self.current_image_hash = Some(hash);

        Ok(())
    }

    fn recv_clipboard(&mut self, frame: Frame) -> Result<()> {
        match frame {
            Frame::Text(text) => {
                let text = ClipboardText { text };
                let hash = text.get_hash();
                if let Some(current_text_hash) = &self.current_text_hash {
                    if current_text_hash.eq(&hash) {
                        return Ok(());
                    }
                }
                text.save(&mut self.clipboard)?;
                self.current_text_hash = Some(hash);
            }
            Frame::Image(width, height, data) => {
                use sha256::digest;

                let width = width as usize;
                let height = height as usize;
                let data = data.to_vec();
                let hash = digest::<&[u8]>(&data);
                let image = ClipboardImage {
                    width,
                    height,
                    data,
                    hash,
                };

                if let Some(current_image_hash) = &self.current_image_hash {
                    if current_image_hash.eq(&image.hash) {
                        return Ok(());
                    }
                }

                image.save(&mut self.clipboard)?;
                self.current_image_hash = Some(image.hash);
            }
            Frame::File(_name, _mode, _data) => {}
        }
        Ok(())
    }
}

/// If an encoding error occurs while reading the clipboard, it is ignored.
/// This is because `arboard` does not provide a universal method for reading
/// the clipboard, and we need to read both images and text simultaneously.
/// This can result in situations where we try to read text when the data in the
/// clipboard is actually an image.
/// Once the https://github.com/1Password/arboard/issues/11 is resolved, we will
/// have a more elegant way to handle this.
fn ignore_clipboard_error(err: &arboard::Error) -> bool {
    match &err {
        arboard::Error::Unknown { description } => description == INCORRECT_CLIPBOARD_TYPE_ERROR,
        arboard::Error::ContentNotAvailable => true,
        _ => false,
    }
}

struct ClipboardText {
    text: String,
}

impl ClipboardText {
    // If the size of the text exceeds this value, it will be compared using SHA256
    // hash calculation to reduce memory pressure.
    const SHA256_TEXT_SIZE: usize = 1024 * 10;

    fn read(cb: &mut Clipboard) -> Result<Option<ClipboardText>> {
        match cb.get_text() {
            Ok(text) => Ok(Some(ClipboardText { text })),
            Err(err) => {
                if ignore_clipboard_error(&err) {
                    return Ok(None);
                }
                Err(anyhow!(err))
            }
        }
    }

    fn get_hash(&self) -> String {
        use sha256::digest;
        if self.text.len() < Self::SHA256_TEXT_SIZE {
            return self.text.clone();
        }
        digest(self.text.as_str())
    }

    fn save(&self, cb: &mut Clipboard) -> Result<()> {
        cb.set_text(&self.text).context("Write text to clipboard")
    }
}

struct ClipboardImage {
    width: usize,
    height: usize,
    data: Vec<u8>,

    hash: String,
}

impl ClipboardImage {
    fn read(cb: &mut Clipboard) -> Result<Option<ClipboardImage>> {
        use sha256::digest;

        match cb.get_image() {
            Ok(image) => {
                let (width, height) = (image.width, image.height);
                let data = image.bytes.into_owned();
                let hash = digest::<&[u8]>(&data);
                Ok(Some(ClipboardImage {
                    width,
                    height,
                    data,
                    hash,
                }))
            }
            Err(err) => {
                if ignore_clipboard_error(&err) {
                    return Ok(None);
                }
                Err(anyhow!(err))
            }
        }
    }

    fn save(&self, cb: &mut Clipboard) -> Result<()> {
        let cb_image = arboard::ImageData {
            width: self.width,
            height: self.height,
            bytes: Cow::from(&self.data),
        };
        cb.set_image(cb_image).context("Write image to clipboard")
    }
}
