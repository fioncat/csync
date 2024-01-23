use std::fs;
use std::time::Duration;

use csync_clipboard::{get_digest, Clipboard};
use csync_proto::frame::{ClipboardFrame, ClipboardImage, DataFrame};
use tokio::time;

#[tokio::main]
async fn main() {
    let mut cb = Clipboard::new(false, true).unwrap();
    let write_tx = cb.write_tx.take().unwrap();

    let data = fs::read("test-image-data").unwrap();
    let data: ClipboardImage = bincode::deserialize(&data).unwrap();
    let data = ClipboardFrame::Image(data);

    write_tx
        .send(DataFrame {
            from: None,
            digest: get_digest(&data),
            data,
        })
        .await
        .unwrap();
    println!("Write image to clipboard");

    time::sleep(Duration::from_secs(5)).await;
}
