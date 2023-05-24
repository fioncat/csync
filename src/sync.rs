use core::fmt;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use arboard::Clipboard;
use human_bytes::human_bytes;
use log::{debug, error, info};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::{self, Duration, Instant, Interval};

use crate::config::Config;
use crate::net::{Client, Frame};

const INCORRECT_CLIPBOARD_TYPE_ERROR: &str = "incorrect type received from clipboard";

pub struct Synchronizer {
    conn_pool: HashMap<String, Client>,
    conn_expire: HashMap<String, Instant>,

    current_hash: Option<String>,

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

        let current = ClipboardData::read(&mut clipboard).context("Read clipboard")?;
        let current_hash = match current {
            Some(data) => Some(data.get_hash()),
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

            current_hash,

            clipboard,

            receiver,

            clipboard_intv,
            expire_intv,
            expire_duration,
        };

        Ok((syncer, sender))
    }

    pub async fn run(&mut self, cfg: &Config) {
        use tokio::select;

        info!("Start to sync clipboard");
        loop {
            select! {
                _ = self.clipboard_intv.tick() => {
                    if let Err(err) = self.send_clipboard_data(&cfg.targets).await {
                        error!("Send clipboard error: {err:#}");
                    }
                }
                _ = self.expire_intv.tick() => {
                    self.flush_conn();
                }
                frame = self.receiver.recv() => {
                    if let Some(frame) = frame {
                        if let Frame::File(name, mode, data) = &frame {
                            if let Err(err) = self.recv_file(&cfg.dir, name, *mode, data).await {
                                error!("Recv data error: {err:#}");
                            }
                        }
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

        debug!("Create connection to {target}");
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
        let mut clean = Vec::new();
        for (addr, expire) in &self.conn_expire {
            if now >= *expire {
                debug!("Drop expired connection {addr}");
                self.conn_pool.remove(addr);
                clean.push(addr.clone());
            }
        }
        for addr in clean {
            self.conn_expire.remove(&addr);
        }
    }

    async fn send_clipboard_data(&mut self, targets: &[SocketAddr]) -> Result<()> {
        let data = match ClipboardData::read(&mut self.clipboard)? {
            Some(data) => data,
            None => return Ok(()),
        };
        let hash = data.get_hash();
        if let Some(current_hash) = &self.current_hash {
            if current_hash.eq(&hash) {
                return Ok(());
            }
        }
        self.current_hash = Some(hash);
        debug!("Clipboard changed: {data}");

        let frame = data.to_frame();
        for target in targets {
            debug!("Send {frame} to {target}");
            let mut conn = self.get_conn(target).await?;
            conn.write_frame(&frame).await?;
            self.save_conn(target, conn);
        }

        Ok(())
    }

    fn recv_clipboard(&mut self, frame: Frame) -> Result<()> {
        let data = ClipboardData::from_frame(frame);
        let hash = data.get_hash();
        if let Some(current_hash) = &self.current_hash {
            if current_hash.eq(&hash) {
                return Ok(());
            }
        }
        debug!("Write {data} to clipboard");
        data.save(&mut self.clipboard).context("Save clipboard")?;
        Ok(())
    }

    async fn recv_file(
        &mut self,
        dir: &PathBuf,
        name: &String,
        mode: u32,
        data: &[u8],
    ) -> Result<()> {
        let path = dir.join(name);
        let dir = path.parent();
        debug!(
            "Write {} data to file {}, mode {}",
            human_bytes(data.len() as u32),
            path.display(),
            mode
        );

        if let Some(dir) = dir {
            match fs::read_dir(dir).await {
                Err(err) if err.kind() == io::ErrorKind::NotFound => fs::create_dir_all(&dir)
                    .await
                    .with_context(|| format!("Create directory {}", dir.display()))?,
                Err(err) => {
                    return Err(err).with_context(|| format!("Read directory {}", dir.display()))
                }

                Ok(_) => {}
            }
        }

        let mut opts = OpenOptions::new();
        opts.create(true).write(true).truncate(true).mode(mode);
        let mut file = opts
            .open(&path)
            .await
            .with_context(|| format!("Open file {}", path.display()))?;
        file.write_all(data)
            .await
            .with_context(|| format!("Write file {}", path.display()))?;

        Ok(())
    }
}

pub enum ClipboardData {
    Text(String),
    Image(u64, u64, Vec<u8>),
}

impl ClipboardData {
    // If the size of the text exceeds this value, it will be compared using SHA256
    // hash calculation to reduce memory pressure.
    const SHA256_TEXT_SIZE: usize = 1024 * 10;

    pub fn read(cb: &mut Clipboard) -> Result<Option<ClipboardData>> {
        match cb.get_text() {
            Ok(text) => return Ok(Some(ClipboardData::Text(text))),
            Err(err) => {
                if !Self::ignore_clipboard_error(&err) {
                    return Err(anyhow!(err));
                }
            }
        }
        match cb.get_image() {
            Ok(image) => {
                let (width, height) = (image.width as u64, image.height as u64);
                let data = image.bytes.into_owned();
                return Ok(Some(ClipboardData::Image(width, height, data)));
            }
            Err(err) => {
                if !Self::ignore_clipboard_error(&err) {
                    return Err(anyhow!(err));
                }
            }
        }
        Ok(None)
    }

    pub fn save(&self, cb: &mut Clipboard) -> Result<()> {
        match self {
            ClipboardData::Text(text) => cb.set_text(text).context("Write text to clipboard"),
            ClipboardData::Image(width, height, data) => {
                let cb_image = arboard::ImageData {
                    width: *width as usize,
                    height: *height as usize,
                    bytes: Cow::from(data),
                };
                cb.set_image(cb_image).context("Write image to clipboard")
            }
        }
    }

    pub fn get_hash(&self) -> String {
        use sha256::digest;

        match self {
            ClipboardData::Text(text) => {
                if text.len() < Self::SHA256_TEXT_SIZE {
                    text.clone()
                } else {
                    digest(text.as_str())
                }
            }
            ClipboardData::Image(_, _, data) => digest::<&[u8]>(data),
        }
    }

    pub fn from_frame(frame: Frame) -> ClipboardData {
        match frame {
            Frame::Text(text) => ClipboardData::Text(text),
            Frame::Image(width, height, data) => ClipboardData::Image(width, height, data.to_vec()),
            _ => unreachable!(),
        }
    }

    pub fn to_frame(self) -> Frame {
        match self {
            ClipboardData::Text(text) => Frame::Text(text),
            ClipboardData::Image(width, height, data) => Frame::Image(width, height, data.into()),
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
            arboard::Error::Unknown { description } => {
                description == INCORRECT_CLIPBOARD_TYPE_ERROR
            }
            arboard::Error::ContentNotAvailable => true,
            _ => false,
        }
    }

    /// Converts text with all the special characters escape with a backslash
    fn escape_string<'a>(text: &'a str) -> Cow<'a, str> {
        let bytes = text.as_bytes();

        let mut owned = None;

        for pos in 0..bytes.len() {
            let special = match bytes[pos] {
                0x07 => Some(b'a'),
                0x08 => Some(b'b'),
                b'\t' => Some(b't'),
                b'\n' => Some(b'n'),
                0x0b => Some(b'v'),
                0x0c => Some(b'f'),
                b'\r' => Some(b'r'),
                b' ' => Some(b' '),
                b'\\' => Some(b'\\'),
                _ => None,
            };
            if let Some(s) = special {
                if owned.is_none() {
                    owned = Some(bytes[0..pos].to_owned());
                }
                owned.as_mut().unwrap().push(b'\\');
                owned.as_mut().unwrap().push(s);
            } else if let Some(owned) = owned.as_mut() {
                owned.push(bytes[pos]);
            }
        }

        if let Some(owned) = owned {
            unsafe { Cow::Owned(String::from_utf8_unchecked(owned)) }
        } else {
            unsafe { Cow::Borrowed(std::str::from_utf8_unchecked(bytes)) }
        }
    }
}

impl fmt::Display for ClipboardData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipboardData::Text(text) => write!(f, "Text `{}`", Self::escape_string(text.as_str())),
            ClipboardData::Image(width, height, data) => {
                let size = human_bytes(data.len() as u32);
                write!(f, "Image {size}, {width}, {height}")
            }
        }
    }
}
