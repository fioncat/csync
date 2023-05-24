use std::net::SocketAddr;

use bytes::Bytes;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use csync::net::{Client, Connection, Frame};

#[tokio::test]
async fn frame_text() {
    const LOOP_LEN: usize = 200;
    let addr = "0.0.0.0:9823";

    let bind: SocketAddr = addr.parse().unwrap();
    let listener = TcpListener::bind(&bind).await.unwrap();
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut conn = Connection::new(socket);

        for i in 0..LOOP_LEN {
            let frame = conn.read_frame().await.unwrap().unwrap();
            let str =
                format!("Hello world, frame {i}\nThis is a new line\r\nThis is the final line");
            match frame {
                Frame::Text(text) => {
                    assert_eq!(str, text);
                }
                _ => panic!("unexpected frame type"),
            }
        }
        tx.send(()).unwrap();
    });

    let mut client = Client::dial_string("127.0.0.1:9823").await.unwrap();
    for i in 0..LOOP_LEN {
        let str = format!("Hello world, frame {i}\nThis is a new line\r\nThis is the final line");
        client.send_text(str).await.unwrap();
    }

    rx.await.unwrap();
}

#[tokio::test]
async fn frame_image() {
    const LOOP_LEN: usize = 150;
    let addr = "0.0.0.0:9824";

    let image_data0 = Bytes::from_static(b"Hello, image\r\nThis is a new line");
    let image_data1 = Bytes::from_static(b"\x01\x02\x03\x04\x05\x06\x12\x13\x14\x15");

    let expect_data0 = image_data0.clone();
    let expect_data1 = image_data1.clone();

    let bind: SocketAddr = addr.parse().unwrap();
    let listener = TcpListener::bind(&bind).await.unwrap();
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut conn = Connection::new(socket);

        for i in 0..LOOP_LEN {
            let frame = conn.read_frame().await.unwrap().unwrap();
            match frame {
                Frame::Image(width, height, data) => {
                    let expect_width = (i * 2) as u64;
                    let expect_height = (i * 5) as u64;
                    assert_eq!(width, expect_width);
                    assert_eq!(height, expect_height);

                    if let 0 = i % 2 {
                        assert_eq!(data, expect_data0);
                    } else {
                        assert_eq!(data, expect_data1);
                    }
                }
                _ => panic!("unexpected frame type"),
            }
        }
        tx.send(()).unwrap();
    });

    let mut client = Client::dial_string("127.0.0.1:9824").await.unwrap();
    for i in 0..LOOP_LEN {
        let width = (i * 2) as u64;
        let height = (i * 5) as u64;
        let data = if let 0 = i % 2 {
            image_data0.clone()
        } else {
            image_data1.clone()
        };
        client.send_image(width, height, data).await.unwrap();
    }

    rx.await.unwrap();
}

#[tokio::test]
async fn frame_mix() {
    const LOOP_LEN: usize = 200;
    let addr = "0.0.0.0:9825";

    let bind: SocketAddr = addr.parse().unwrap();
    let listener = TcpListener::bind(&bind).await.unwrap();
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut conn = Connection::new(socket);

        for i in 0..LOOP_LEN {
            let frame = conn.read_frame().await.unwrap().unwrap();
            match frame {
                Frame::Image(width, height, data) => {
                    let expect_width = (i * 10) as u64;
                    let expect_height = (i * 50) as u64;
                    assert_eq!(width, expect_width);
                    assert_eq!(height, expect_height);

                    let expect_data = format!("Hello image\r\nindex={i}");
                    let expect_data = expect_data.as_bytes();
                    assert_eq!(data, expect_data);
                }
                Frame::Text(text) => {
                    let expect_text = format!("Hello text\r\nindex={i}");
                    assert_eq!(text, expect_text);
                }
                _ => panic!("unexpected frame type"),
            }
        }
        tx.send(()).unwrap();
    });

    let mut client = Client::dial_string("127.0.0.1:9825").await.unwrap();
    for i in 0..LOOP_LEN {
        if let 0 = i % 2 {
            let width = (i * 10) as u64;
            let height = (i * 50) as u64;
            let data = format!("Hello image\r\nindex={i}");
            client.send_image(width, height, data.into()).await.unwrap();
        } else {
            let text = format!("Hello text\r\nindex={i}");
            client.send_text(text).await.unwrap();
        }
    }

    rx.await.unwrap();
}
