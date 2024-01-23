use std::env;
use std::io::{stdout, Write};

use csync_proto::client::{Client, TerminalPassword};
use csync_proto::frame::{ClipboardFrame, DataFrame};
use scanf::scanf;

#[tokio::main]
async fn main() {
    let addr = "localhost:7703";

    let args: Vec<String> = env::args().collect();
    let publish = if args.len() >= 2 {
        let publish = args[1].as_str();
        publish.to_string()
    } else {
        "test".to_string()
    };

    let mut client = Client::dial(addr, Some(publish), None, TerminalPassword::new(false))
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
    }
}
