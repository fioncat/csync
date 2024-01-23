use core::panic;
use std::net::SocketAddr;
use std::sync::Arc;

use csync_proto::auth::Auth;
use csync_proto::conn::Connection;
use csync_proto::frame::*;
use tokio::net::TcpListener;
use tokio::sync::oneshot::{self, Receiver};

#[tokio::test]
async fn conn_register() {
    const ROUNDS: usize = 300;

    let (rx, mut client) = spawn_server(9820, ROUNDS, None, |idx, frame| {
        let register_frame = match frame {
            Frame::Register(frame) => frame,
            _ => panic!("unexpect frame"),
        };

        let publish = Some(format!("register-{idx}"));
        assert_eq!(publish, register_frame.publish);

        let mut subs = Vec::with_capacity(idx);
        for i in 0..idx {
            let sub = format!("sub-{i}");
            subs.push(sub);
        }
        assert_eq!(Some(subs), register_frame.subs);

        Some(Frame::Accept(AcceptFrame {
            auth: None,
            version: idx as u32,
        }))
    })
    .await;

    for idx in 0..ROUNDS {
        let publish = Some(format!("register-{idx}"));
        let mut subs = Vec::with_capacity(idx);
        for i in 0..idx {
            let sub = format!("sub-{i}");
            subs.push(sub);
        }
        let frame = Frame::Register(RegisterFrame {
            publish,
            subs: Some(subs),
        });
        client.write_frame(&frame).await.unwrap();

        let ret = client.read_frame().await.unwrap().unwrap();
        let ret = match ret {
            Frame::Accept(frame) => frame,
            _ => panic!("unexpect frame"),
        };
        assert_eq!(ret.version as usize, idx);
    }

    rx.await.unwrap();
}

#[tokio::test]
async fn conn_text() {
    text_inner(9821, None).await
}

#[tokio::test]
async fn conn_text_auth() {
    let auth = Auth::new("test-password");
    text_inner(9822, Some(auth)).await
}

async fn text_inner(port: u32, auth: Option<Auth>) {
    const ROUNDS: usize = 5000;

    let (rx, mut client) = spawn_server(port, ROUNDS, auth, |idx, frame| {
        let data_frame = match frame {
            Frame::Push(frame) => frame,
            _ => panic!("unexpect frame"),
        };

        let from = Some(format!("text-{idx}"));
        assert_eq!(from, data_frame.from);

        let digest = format!("digest-{idx}");
        assert_eq!(digest, data_frame.digest);

        let text = match data_frame.data {
            ClipboardFrame::Text(text) => text,
            _ => panic!("unexpect data frame"),
        };
        assert_eq!(text, format!("clipboard-{idx}"));

        None
    })
    .await;

    for idx in 0..ROUNDS {
        let from = Some(format!("text-{idx}"));
        let digest = format!("digest-{idx}");
        let text = format!("clipboard-{idx}");
        let frame = Frame::Push(DataFrame {
            from,
            digest,
            data: ClipboardFrame::Text(text),
        });

        client.write_frame(&frame).await.unwrap();
    }

    rx.await.unwrap();
}

#[tokio::test]
async fn conn_image() {
    image_inner(9823, None).await
}

#[tokio::test]
async fn conn_image_auth() {
    let auth = Auth::new("test-password");
    image_inner(9824, Some(auth)).await
}

async fn image_inner(port: u32, auth: Option<Auth>) {
    const ROUNDS: usize = 5000;
    let (rx, mut client) = spawn_server(port, ROUNDS, auth, |idx, frame| {
        let data_frame = match frame {
            Frame::Push(frame) => frame,
            _ => panic!("unexpect frame"),
        };

        let from = Some(format!("image-{idx}"));
        assert_eq!(from, data_frame.from);

        let digest = format!("digest-{idx}");
        assert_eq!(digest, data_frame.digest);

        let image = match data_frame.data {
            ClipboardFrame::Image(image) => image,
            _ => panic!("unexpect data frame"),
        };
        assert_eq!(image.width as usize, idx * 2);
        assert_eq!(image.height as usize, idx * 3);

        let image_data = format!("test-image-{idx}\n\rnew line\nlast line").into_bytes();
        assert_eq!(image.data, image_data);

        None
    })
    .await;

    for idx in 0..ROUNDS {
        let from = Some(format!("image-{idx}"));
        let digest = format!("digest-{idx}");
        let image_data = format!("test-image-{idx}\n\rnew line\nlast line").into_bytes();
        let frame = Frame::Push(DataFrame {
            from,
            digest,
            data: ClipboardFrame::Image(ClipboardImage {
                width: idx as u64 * 2,
                height: idx as u64 * 3,
                data: image_data,
            }),
        });

        client.write_frame(&frame).await.unwrap();
    }

    rx.await.unwrap();
}

async fn spawn_server<F>(
    port: u32,
    rounds: usize,
    auth: Option<Auth>,
    f: F,
) -> (Receiver<()>, Connection)
where
    F: Fn(usize, Frame) -> Option<Frame> + Send + 'static,
{
    let addr = format!("0.0.0.0:{port}");
    let bind: SocketAddr = addr.parse().unwrap();

    let auth = Arc::new(auth);
    let client_auth = Arc::clone(&auth);
    let listener = TcpListener::bind(&bind).await.unwrap();

    let (tx, rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut conn = Connection::new(socket, auth);
        for idx in 0..rounds {
            let frame = conn.read_frame().await.unwrap();
            let ret = f(idx, frame.unwrap());
            if let Some(ret) = ret {
                conn.write_frame(&ret).await.unwrap();
            }
        }
        tx.send(()).unwrap();
    });

    let addr = format!("127.0.0.1:{port}");
    let bind: SocketAddr = addr.parse().unwrap();
    let client_conn = Connection::dial(&bind, client_auth).await.unwrap();
    (rx, client_conn)
}
