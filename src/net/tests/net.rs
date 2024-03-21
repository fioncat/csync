use std::borrow::Cow;
use std::sync::Arc;

use tokio::fs::File;
use tokio::sync::oneshot;

use crate::net::auth::Auth;
use crate::net::frame::*;
use crate::net::tests::*;

fn get_test_frames() -> Vec<Frame<'static>> {
    vec![
        Frame::Pub(Cow::Borrowed("hello")),
        Frame::Sub(Cow::Owned(vec![
            String::from("hello0"),
            String::from("hello1"),
        ])),
        Frame::Accept(Cow::Owned(AcceptFrame {
            version: 123,
            auth: Some(AuthFrame {
                check: b"test check".to_vec(),
                check_plain: b"test check plain".to_vec(),
                nonce: b"test nonce".to_vec(),
                salt: b"test salt".to_vec(),
            }),
        })),
        Frame::Data(Cow::Owned(DataFrame {
            info: DataFrameInfo {
                device: Some(String::from("test device")),
                digest: String::from("test digest"),
                file: None,
            },

            body: b"test data".to_vec(),
        })),
        Frame::Data(Cow::Owned(DataFrame {
            info: DataFrameInfo {
                device: Some(String::from("test device")),
                digest: String::from("test digest"),
                file: Some(FileInfo {
                    name: String::from("test_file.png"),
                    mode: 123,
                }),
            },

            body: b"test file data".to_vec(),
        })),
        Frame::Ok,
        Frame::Error(Cow::Borrowed("Test error")),
        Frame::Ping,
    ]
}

#[tokio::test]
async fn test_frame() {
    _test_frame("test_frame", None).await
}

#[tokio::test]
async fn test_frame_auth() {
    _test_frame("test_frame_auth", Some(Auth::new("test password 333"))).await
}

async fn _test_frame(name: &str, auth: Option<Auth>) {
    let path = format!("_test_data/files/{name}");

    _test_frame_write(&path, auth.clone()).await;

    let file = File::open(path).await.unwrap();
    let mut conn = Connection::new(file);
    conn.with_auth(Arc::new(auth));

    let expect_frames = get_test_frames();
    for expect_frame in expect_frames {
        let frame = conn.must_read_frame().await.unwrap();
        assert_eq!(frame, expect_frame);
    }
}

async fn _test_frame_write(path: &str, auth: Option<Auth>) {
    utils::ensure_dir(PathBuf::from(path).parent().unwrap()).unwrap();

    let file = File::create(&path).await.unwrap();
    let mut conn = Connection::new(file);
    conn.with_auth(Arc::new(auth));

    let frames = get_test_frames();
    for frame in frames.iter() {
        conn.write_frame(frame).await.unwrap();
    }
}

#[tokio::test]
async fn test_conn() {
    _test_conn("test_conn", None).await
}

#[tokio::test]
async fn test_conn_auth() {
    _test_conn("test_conn_auth", Some(Auth::new("test password 123"))).await
}

async fn _test_conn(name: &str, auth: Option<Auth>) {
    let frames = get_test_frames();

    let auth = Arc::new(auth);

    let expect_frames = frames.clone();
    let mut rx = start_server(name).await;
    let (done_tx, done_rx) = oneshot::channel::<()>();
    let server_auth = Arc::clone(&auth);
    tokio::spawn(async move {
        for expect_frame in expect_frames {
            let mut conn = rx.recv().await.expect("recv from mpsc");
            conn.with_auth(Arc::clone(&server_auth));
            let frame = conn.must_read_frame().await.expect("read frame");
            assert_eq!(frame, expect_frame);
        }

        done_tx.send(()).unwrap();
    });

    for frame in frames {
        let mut conn = connect_server(name).await;
        conn.with_auth(Arc::clone(&auth));
        conn.write_frame(&frame).await.expect("write frame");
    }

    done_rx.await.unwrap();
}
