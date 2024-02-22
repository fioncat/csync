use std::fs;

use csync_clipboard::Clipboard;
use csync_proto::frame::{ClipboardFrame, DataFrame};
use tokio::select;

#[tokio::main]
async fn main() {
    let mut cw = Clipboard::new(true, false).unwrap();
    let mut read_rx = cw.read_rx.take().unwrap();

    loop {
        select! {
            Some(data_frame) = read_rx.recv() => {
                handle_read(data_frame);
            }
            Some(err) = cw.error_rx.recv() => {
                panic!("clipboard watch error: {:#}", err);
            }
        }
    }
}

fn handle_read(data_frame: Option<DataFrame>) {
    println!("Clipboard changed");
    if data_frame.is_none() {
        println!();
        return;
    }
    let data_frame = data_frame.unwrap();
    println!("Digest: {}", data_frame.digest);

    match &data_frame.data {
        ClipboardFrame::Text(text) => println!("{text}"),
        ClipboardFrame::Image(image) => {
            let data = bincode::serialize(&image).unwrap();
            fs::write("test-image-data", data).unwrap();
            println!("Write image to 'test-image-data'");
        }
    }
    println!();
}
