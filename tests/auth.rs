use std::net::SocketAddr;

use tokio::net::TcpListener;
use tokio::sync::oneshot;

use csync::net::{Auth, Client, Connection, Frame};

#[tokio::test]
async fn auth() {
    const LOOP_LEN: usize = 200;
    let addr = "0.0.0.0:9830";
    let password = "Test password 123".to_string();
    let auth_key = Auth::digest(password);
    let auth_key_client = auth_key.clone();

    let bind: SocketAddr = addr.parse().unwrap();
    let listener = TcpListener::bind(&bind).await.unwrap();
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut conn = Connection::new(socket);
        conn.with_auth(Auth::new(&auth_key));

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

    let mut client = Client::dial_string("127.0.0.1:9830").await.unwrap();
    client.with_auth(Auth::new(&auth_key_client));
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

#[tokio::test]
async fn auth_failed_text() {
    const LOOP_LEN: usize = 200;
    let addr = "0.0.0.0:9831";
    let password = "Test password 123".to_string();
    let auth_key = Auth::digest(password);

    let client_password = "Test client password 123".to_string();
    let auth_key_client = Auth::digest(client_password);

    let bind: SocketAddr = addr.parse().unwrap();
    let listener = TcpListener::bind(&bind).await.unwrap();
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut conn = Connection::new(socket);
        conn.with_auth(Auth::new(&auth_key));

        match conn.read_frame().await {
            Ok(_) => panic!("unexpected success"),
            Err(err) => {
                let msg = format!("{err}");
                assert_eq!("Parse frame", msg);
            }
        }
        tx.send(()).unwrap();
    });

    let mut client = Client::dial_string("127.0.0.1:9831").await.unwrap();
    client.with_auth(Auth::new(&auth_key_client));
    for i in 0..LOOP_LEN {
        let str = format!("Hello world, frame {i}\nThis is a new line\r\nThis is the final line");
        // Ignore error, since the client will be closed by server.
        let _ = client.send_text(str).await;
    }

    rx.await.unwrap();
}

#[tokio::test]
async fn auth_failed_image() {
    const LOOP_LEN: usize = 200;
    let addr = "0.0.0.0:9832";
    let password = "Test password 000".to_string();
    let auth_key = Auth::digest(password);

    let client_password = "Test client password 123".to_string();
    let auth_key_client = Auth::digest(client_password);

    let bind: SocketAddr = addr.parse().unwrap();
    let listener = TcpListener::bind(&bind).await.unwrap();
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut conn = Connection::new(socket);
        conn.with_auth(Auth::new(&auth_key));

        match conn.read_frame().await {
            Ok(_) => panic!("unexpected success"),
            Err(err) => {
                let msg = format!("{err}");
                assert_eq!("Parse frame", msg);
            }
        }
        tx.send(()).unwrap();
    });

    let mut client = Client::dial_string("127.0.0.1:9832").await.unwrap();
    client.with_auth(Auth::new(&auth_key_client));
    for i in 0..LOOP_LEN {
        let data = format!("Hello image\r\nindex={i}");
        // Ignore error, since the client will be closed by server.
        let _ = client.send_image(12, 64, data.into()).await;
    }

    rx.await.unwrap();
}
