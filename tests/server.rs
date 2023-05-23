use std::net::SocketAddr;

use csync::net::{Client, Frame};
use csync::server::Server;
use tokio::sync::{mpsc, oneshot};

#[tokio::test]
async fn server() {
    let addr: SocketAddr = String::from("0.0.0.0:9908").parse().unwrap();
    let (sender, mut receiver) = mpsc::channel::<Frame>(512);
    let mut srv = Server::new(&addr, sender, 100).await.unwrap();
    tokio::spawn(async move { srv.run().await.unwrap() });

    const LOOP_LEN: usize = 500;

    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        for i in 0..LOOP_LEN {
            let frame = receiver.recv().await.unwrap();
            match frame {
                Frame::Text(text) => {
                    let expect = format!("{i}: Test text info\r\nThis is next line\r\ndone!\r\n");
                    assert_eq!(text, expect);
                }
                Frame::Image(width, height, data) => {
                    let expect_width = (i * 10 + 2) as u64;
                    let expect_height = (i * 20 + 10) as u64;
                    assert_eq!(width, expect_width);
                    assert_eq!(height, expect_height);

                    let expect_data = format!("{i}: Test image info\r\nNext line");
                    let expect_data = expect_data.as_bytes();
                    assert_eq!(data, expect_data);
                }
                _ => panic!("unexpected frame type"),
            }
        }
        tx.send(()).unwrap();
    });

    let mut client = Client::dial_string("127.0.0.1:9908").await.unwrap();
    for i in 0..LOOP_LEN {
        if let 0 = i % 2 {
            let width = (i * 10 + 2) as u64;
            let height = (i * 20 + 10) as u64;
            let data = format!("{i}: Test image info\r\nNext line");
            client.send_image(width, height, data.into()).await.unwrap();
        } else {
            let text = format!("{i}: Test text info\r\nThis is next line\r\ndone!\r\n");
            client.send_text(text).await.unwrap();
        }
    }

    rx.await.unwrap();
}
