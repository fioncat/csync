use std::env;
use std::time::Duration;

use csync_proto::client::{Client, TerminalPassword};
use csync_proto::frame::ClipboardFrame;
use tokio::time::{self, Instant};

#[tokio::main]
async fn main() {
    let addr = "localhost:7703";

    let args: Vec<String> = env::args().collect();
    let sub = if args.len() >= 2 {
        let sub = args[1].as_str();
        sub.to_string()
    } else {
        "test".to_string()
    };

    let mut client = Client::dial(addr, None, Some(vec![sub]), TerminalPassword::new(false))
        .await
        .unwrap();

    println!("Connect ok");

    let intv = Duration::from_secs(1);
    let start = Instant::now();
    let mut intv = time::interval_at(start, intv);

    loop {
        intv.tick().await;

        let data = client.pull().await.unwrap();
        match data {
            Some(data) => match &data.data {
                ClipboardFrame::Text(text) => {
                    println!("Recv from {}: {}", data.from.unwrap(), text)
                }
                ClipboardFrame::Image(_) => {
                    panic!("unexpect image");
                }
            },
            None => {}
        }
    }
}
