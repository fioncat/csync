use std::net::SocketAddr;
use std::sync::Arc;

use csync_proto::auth::Auth;
use csync_proto::client::{Client, StaticPassword, TerminalPassword};
use csync_proto::conn::Connection;
use csync_proto::frame::*;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[tokio::test]
async fn client_push() {
    let port = 8879;
    let password = "test-password";
    let rounds = 2000;

    let addr = format!("0.0.0.0:{port}");
    let bind: SocketAddr = addr.parse().unwrap();
    let listener = TcpListener::bind(&bind).await.unwrap();

    let auth = Auth::new(password);
    let auth = Arc::new(Some(auth));

    let (tx, rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut conn = Connection::new(socket, Arc::clone(&auth));

        let register_frame = conn.must_read_frame().await.unwrap();
        let register = register_frame.expect_register().unwrap();
        assert_eq!(register.publish, Some("test".to_string()));
        assert_eq!(
            register.subs,
            Some(vec!["test0".to_string(), "test1".to_string()])
        );
        let auth_frame = auth
            .as_ref()
            .as_ref()
            .map(|auth| auth.build_frame().unwrap());
        conn.write_frame(&Frame::Accept(AcceptFrame {
            version: PROTOCOL_VERSION,
            auth: auth_frame,
        }))
        .await
        .unwrap();

        for i in 0..rounds {
            let frame = conn
                .must_read_frame()
                .await
                .unwrap()
                .expect_data()
                .unwrap()
                .unwrap();

            assert_eq!(frame.from, None);
            assert_eq!(frame.digest, format!("data-digest-{i}"));

            if i % 2 == 0 {
                let text = match frame.data {
                    ClipboardFrame::Text(text) => text,
                    _ => panic!("unexpect image frame"),
                };
                assert_eq!(text, format!("clipboard data {i}\nnew line\nlast line"));
            } else {
                let image = match frame.data {
                    ClipboardFrame::Image(image) => image,
                    _ => panic!("unexpect text frame"),
                };
                assert_eq!(image.width, i * 2);
                assert_eq!(image.height, i * 3);
                assert_eq!(image.data, format!("image-data-{i}").into_bytes());
            }
            conn.write_frame(&Frame::None).await.unwrap();
        }

        tx.send(()).unwrap();
    });

    let addr = format!("localhost:{port}");
    let mut client = Client::dial(
        addr,
        Some("test".to_string()),
        Some(vec!["test0".to_string(), "test1".to_string()]),
        StaticPassword::new(password),
    )
    .await
    .unwrap();

    for i in 0..rounds {
        let data = if i % 2 == 0 {
            let text = format!("clipboard data {i}\nnew line\nlast line");
            ClipboardFrame::Text(text)
        } else {
            let image = ClipboardImage {
                width: i * 2,
                height: i * 3,
                data: format!("image-data-{i}").into_bytes(),
            };
            ClipboardFrame::Image(image)
        };
        let frame = DataFrame {
            from: None,
            digest: format!("data-digest-{i}"),
            data,
        };
        client.push(frame).await.unwrap();
    }

    rx.await.unwrap();
}

#[tokio::test]
async fn client_pull() {
    let port = 8880;
    let rounds = 2000;

    let addr = format!("0.0.0.0:{port}");
    let bind: SocketAddr = addr.parse().unwrap();
    let listener = TcpListener::bind(&bind).await.unwrap();

    tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut conn = Connection::new(socket, Arc::new(None));

        let register_frame = conn.must_read_frame().await.unwrap();
        let register = register_frame.expect_register().unwrap();
        assert_eq!(register.publish, Some("test".to_string()));
        assert_eq!(register.subs, None);
        conn.write_frame(&Frame::Accept(AcceptFrame {
            version: PROTOCOL_VERSION,
            auth: None,
        }))
        .await
        .unwrap();

        for i in 0..rounds {
            let frame = conn.must_read_frame().await.unwrap();
            match frame {
                Frame::Pull => {}
                _ => panic!("unexpect frame"),
            }

            let from = format!("frame-{i}");
            let digest = format!("{from}-digest");
            let text = format!("{from}-text\n\rnext line");

            conn.write_frame(&Frame::Push(DataFrame {
                from: Some(from),
                digest,
                data: ClipboardFrame::Text(text),
            }))
            .await
            .unwrap();
        }

        loop {
            let frame = conn.must_read_frame().await.unwrap();
            match frame {
                Frame::Pull => {}
                _ => panic!("unexpect frame"),
            }
            conn.write_frame(&Frame::None).await.unwrap();
        }
    });

    let addr = format!("localhost:{port}");
    let mut client = Client::dial(
        addr,
        Some("test".to_string()),
        None,
        TerminalPassword::new(true),
    )
    .await
    .unwrap();

    let mut count = 0;
    loop {
        let data = client.pull().await.unwrap();
        match data {
            Some(data) => {
                let from = data.from.clone().unwrap();
                let digest = format!("{from}-digest");
                let text = format!("{from}-text\n\rnext line");

                assert_eq!(data.from, Some(from));
                assert_eq!(data.digest, digest);
                match data.data {
                    ClipboardFrame::Text(s) => assert_eq!(s, text),
                    _ => panic!("unexpect image"),
                }
                count += 1;
            }
            None => break,
        }
    }

    assert_eq!(count, rounds);
}
