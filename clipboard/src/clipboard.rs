use std::borrow::Cow;

use anyhow::{bail, Context, Error, Result};
use arboard::{Clipboard as ClipboardDriver, ImageData};
use clipboard_master::CallbackResult as ClipboardCallbackResult;
use clipboard_master::ClipboardHandler;
use clipboard_master::Master as ClipboardMaster;
use csync_proto::frame::{ClipboardFrame, ClipboardImage, DataFrame};
use sha256::digest;
use tokio::select;
use tokio::sync::mpsc::{self, Receiver, Sender};

const CHANNEL_SIZE: usize = 512;

pub struct Clipboard {
    pub read_rx: Option<Receiver<Option<DataFrame>>>,
    pub write_tx: Option<Sender<DataFrame>>,

    pub error_rx: Receiver<Error>,
}

impl Clipboard {
    pub fn new(readonly: bool, writeonly: bool) -> Result<Clipboard> {
        if readonly && writeonly {
            bail!("readonly and writeonly can not both be true");
        }

        let (read_tx, read_rx) = if writeonly {
            (None, None)
        } else {
            let (tx, rx) = mpsc::channel::<Option<DataFrame>>(CHANNEL_SIZE);
            (Some(tx), Some(rx))
        };
        let (write_tx, write_rx) = if readonly {
            (None, None)
        } else {
            let (tx, rx) = mpsc::channel::<DataFrame>(CHANNEL_SIZE);
            (Some(tx), Some(rx))
        };

        let (error_tx, error_rx) = mpsc::channel::<Error>(CHANNEL_SIZE);

        let driver = ClipboardDriver::new().context("init clipboard driver")?;

        let mut mgr = ClipboardManager {
            digest: None,
            driver,
            read_tx,
            write_rx,
            error_tx,
        };
        tokio::spawn(async move {
            mgr.main_loop().await;
        });

        Ok(Clipboard {
            read_rx,
            write_tx,
            error_rx,
        })
    }
}

struct ClipboardManager {
    digest: Option<String>,
    driver: ClipboardDriver,

    read_tx: Option<Sender<Option<DataFrame>>>,
    write_rx: Option<Receiver<DataFrame>>,

    error_tx: Sender<Error>,
}

impl ClipboardManager {
    /// Such error returns from `arboard` should be ignored.
    const INCORRECT_CLIPBOARD_TYPE_ERROR: &'static str = "incorrect type received from clipboard";

    async fn main_loop(&mut self) {
        if let None = self.read_tx {
            if let None = self.write_rx {
                unreachable!();
            }
            return self.writeonly_loop().await;
        }

        if let None = self.write_rx {
            return self.readonly_loop().await;
        }

        let mut notify = ClipboardNotify::start();
        let mut read_tx = self.read_tx.take().unwrap();
        let mut write_rx = self.write_rx.take().unwrap();

        loop {
            select! {
                Some(result) = notify.recv() => {
                    self.handle_read(result, &mut read_tx).await;
                }
                Some(data_frame) = write_rx.recv() => {
                    self.handle_write(data_frame).await;
                }
            }
        }
    }

    async fn readonly_loop(&mut self) {
        let mut notify = ClipboardNotify::start();
        let mut tx = self.read_tx.take().unwrap();

        loop {
            if let Some(result) = notify.recv().await {
                self.handle_read(result, &mut tx).await;
            }
        }
    }

    async fn writeonly_loop(&mut self) {
        let mut rx = self.write_rx.take().unwrap();

        loop {
            let data_frame = rx.recv().await;
            if let Some(data_frame) = data_frame {
                self.handle_write(data_frame).await;
            }
        }
    }

    async fn handle_read(&mut self, notify: Result<()>, tx: &mut Sender<Option<DataFrame>>) {
        if let Err(err) = notify {
            self.error_tx
                .send(err.context("watch clipboard failed"))
                .await
                .unwrap();
            return;
        }
        match self.read() {
            Ok(data_frame) => {
                tx.send(data_frame).await.unwrap();
            }
            Err(err) => {
                self.error_tx
                    .send(err.context("read clipboard failed"))
                    .await
                    .unwrap();
            }
        };
    }

    async fn handle_write(&mut self, data_frame: DataFrame) {
        match self.write(data_frame) {
            Ok(_) => {}
            Err(err) => {
                self.error_tx
                    .send(err.context("write clipboard failed"))
                    .await
                    .unwrap();
            }
        }
    }

    fn read(&mut self) -> Result<Option<DataFrame>> {
        match self.driver.get_text() {
            Ok(text) => {
                let digest = Some(digest(&text));
                if digest == self.digest {
                    return Ok(None);
                }
                self.digest = digest.clone();
                return Ok(Some(DataFrame {
                    from: None,
                    digest: digest.unwrap(),
                    data: ClipboardFrame::Text(text),
                }));
            }
            Err(err) => {
                if !Self::ignore_clipboard_error(&err) {
                    bail!("read clipboard text failed: {err}");
                }
            }
        }

        match self.driver.get_image() {
            Ok(image) => {
                let (width, height) = (image.width as u64, image.height as u64);
                let data = image.bytes.into_owned();
                let digest = Some(digest::<&[u8]>(&data));
                if digest == self.digest {
                    return Ok(None);
                }
                self.digest = digest.clone();
                Ok(Some(DataFrame {
                    from: None,
                    digest: digest.unwrap(),
                    data: ClipboardFrame::Image(ClipboardImage {
                        width,
                        height,
                        data,
                    }),
                }))
            }
            Err(err) => {
                if !Self::ignore_clipboard_error(&err) {
                    bail!("read clipboard image failed: {err}");
                }
                Ok(None)
            }
        }
    }

    fn write(&mut self, frame: DataFrame) -> Result<()> {
        let DataFrame {
            from: _,
            digest,
            data,
        } = frame;

        let digest = Some(digest);
        if digest == self.digest {
            return Ok(());
        }

        match data {
            ClipboardFrame::Text(text) => self
                .driver
                .set_text(text)
                .context("write text to clipboard driver"),
            ClipboardFrame::Image(image) => self
                .driver
                .set_image(ImageData {
                    width: image.width as usize,
                    height: image.height as usize,
                    bytes: Cow::Owned(image.data),
                })
                .context("write image to clipboard driver"),
        }?;
        self.digest = digest;
        Ok(())
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
                description == Self::INCORRECT_CLIPBOARD_TYPE_ERROR
            }
            arboard::Error::ContentNotAvailable => true,
            _ => false,
        }
    }
}

struct ClipboardNotify {
    tx: mpsc::Sender<Result<()>>,
}

impl ClipboardHandler for ClipboardNotify {
    fn on_clipboard_change(&mut self) -> ClipboardCallbackResult {
        self.tx.blocking_send(Ok(())).unwrap();
        ClipboardCallbackResult::Next
    }

    fn on_clipboard_error(&mut self, err: std::io::Error) -> ClipboardCallbackResult {
        let err = Err(err).context("watch clipboard failed");
        self.tx.blocking_send(err).unwrap();
        ClipboardCallbackResult::Next
    }
}

impl ClipboardNotify {
    fn start() -> mpsc::Receiver<Result<()>> {
        let (tx, rx) = mpsc::channel::<Result<()>>(CHANNEL_SIZE);
        let notify = ClipboardNotify { tx };

        let mut master = ClipboardMaster::new(notify);

        std::thread::spawn(move || {
            match master.run() {
                // We would never stop watching, so the function should never be returned.
                _ => unreachable!(),
            }
        });

        rx
    }
}
