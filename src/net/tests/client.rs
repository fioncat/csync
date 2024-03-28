use std::borrow::Cow;
use std::sync::Arc;

use tokio::sync::oneshot;

use crate::net::auth::Auth;
use crate::net::client::{SendClient, WatchClient};
use crate::net::frame;
use crate::net::frame::*;
use crate::net::tests::*;

#[tokio::test]
async fn test_send_client() {
    _test_send_client("test_send_client", None, None).await
}

#[tokio::test]
async fn test_send_client_auth() {
    let password = "test send client password 123";
    _test_send_client(
        "test_send_client_auth",
        Some(Auth::new(password)),
        Some(password),
    )
    .await
}

async fn _test_send_client(name: &str, auth: Option<Auth>, password: Option<&str>) {
    let frames = get_test_data_frames();

    let auth = Arc::new(auth);

    let expect_frames = frames.clone();
    let mut rx = start_server(name).await;
    let (done_tx, done_rx) = oneshot::channel::<()>();
    let server_auth = Arc::clone(&auth);
    tokio::spawn(async move {
        let mut conn = rx.recv().await.expect("recv connect");

        let pub_frame = conn.must_read_frame().await.unwrap();
        let expect_pub = Frame::Pub(Cow::Borrowed("test-send"));
        assert_eq!(pub_frame, expect_pub);

        let auth_frame = server_auth
            .as_ref()
            .as_ref()
            .map(|auth| auth.build_frame().unwrap());
        let accept = AcceptFrame {
            version: frame::PROTOCOL_VERSION,
            auth: auth_frame,
        };
        conn.write_frame(&Frame::Accept(Cow::Owned(accept)))
            .await
            .unwrap();

        conn.with_auth(Arc::clone(&server_auth));
        for expect_frame in expect_frames {
            let frame = conn.must_read_frame().await.unwrap().expect_data().unwrap();
            assert_eq!(frame, expect_frame);
            conn.write_frame(&Frame::Ok).await.unwrap();
        }

        done_tx.send(()).unwrap();
    });

    let conn = connect_server(name).await;
    let mut client = SendClient::new(conn, "test-send", password).await.unwrap();

    for frame in frames {
        client.send(Arc::new(frame)).await.unwrap();
    }

    done_rx.await.unwrap();
}

#[tokio::test]
async fn test_watch_client() {
    _test_watch_client("test_watch_client", None, None).await
}

#[tokio::test]
async fn test_watch_client_auth() {
    let password = "test watch client password 123";
    _test_watch_client(
        "test_watch_client_auth",
        Some(Auth::new(password)),
        Some(password),
    )
    .await
}

async fn _test_watch_client(name: &str, auth: Option<Auth>, password: Option<&str>) {
    let devices = vec![String::from("test-device")];
    let expect_devices = devices.clone();

    let frames = get_test_data_frames();
    let expect_frames = frames.clone();

    let auth = Arc::new(auth);

    let mut rx = start_server(name).await;
    let server_auth = Arc::clone(&auth);
    tokio::spawn(async move {
        let mut conn = rx.recv().await.expect("recv connect");

        let sub_frame = conn.must_read_frame().await.unwrap();
        let expect_sub = Frame::Sub(Cow::Owned(expect_devices));
        assert_eq!(sub_frame, expect_sub);

        let auth_frame = server_auth
            .as_ref()
            .as_ref()
            .map(|auth| auth.build_frame().unwrap());
        let accept = AcceptFrame {
            version: frame::PROTOCOL_VERSION,
            auth: auth_frame,
        };
        conn.write_frame(&Frame::Accept(Cow::Owned(accept)))
            .await
            .unwrap();

        conn.with_auth(Arc::clone(&server_auth));
        for frame in frames {
            let frame = Frame::Data(Cow::Owned(frame));
            conn.write_frame(&frame).await.unwrap();
        }
    });

    let conn = connect_server(name).await;
    let mut client = WatchClient::new(conn, &devices, password).await.unwrap();

    for expect_frame in expect_frames {
        let frame = client.recv().await.unwrap();
        assert_eq!(frame, expect_frame);
    }
}
