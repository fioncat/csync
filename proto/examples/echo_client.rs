use std::io::{stdout, Write};

use csync_proto::client::{Client, TerminalPassword};
use csync_proto::frame::{ClipboardFrame, DataFrame};
use scanf::scanf;

#[tokio::main]
async fn main() {
    let addr = "localhost:7703";

    let mut client = Client::dial(
        addr,
        Some("test".to_string()),
        None,
        TerminalPassword::new(false),
    )
    .await
    .unwrap();

    println!("Connect ok");

    loop {
        stdout().write_all(b"Please input something: ").unwrap();
        stdout().flush().unwrap();

        let mut input = String::new();
        scanf!("{}", input).unwrap();

        client
            .push(DataFrame {
                from: None,
                digest: String::new(),
                data: ClipboardFrame::Text(input),
            })
            .await
            .unwrap();

        let resp = client.pull().await.unwrap().unwrap();
        match resp.data {
            ClipboardFrame::Text(text) => println!("resp: '{text}'"),
            _ => {}
        }
    }
}
