use std::io::{stdout, Write};

use csync_clipboard::{get_digest, Clipboard};
use csync_proto::frame::{ClipboardFrame, DataFrame};
use scanf::scanf;

#[tokio::main]
async fn main() {
    let mut cb = Clipboard::new(false, true).unwrap();
    let write_tx = cb.write_tx.take().unwrap();

    loop {
        stdout().write_all(b"Please input something: ").unwrap();
        stdout().flush().unwrap();

        let mut input = String::new();
        scanf!("{}", input).unwrap();

        let data = ClipboardFrame::Text(input);
        let digest = get_digest(&data);
        write_tx
            .send(DataFrame {
                from: None,
                digest,
                data,
            })
            .await
            .unwrap();
        println!("Write text to clipboard!");
    }
}
