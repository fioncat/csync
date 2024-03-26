use crate::net::client::{SendClient, WatchClient};
use crate::net::server;
use crate::net::tests::get_test_data_frames;

#[tokio::test]
async fn test_server() {
    _test_server(7709, None).await;
}

#[tokio::test]
async fn test_server_auth() {
    _test_server(7710, Some("test password 123")).await;
}

async fn _test_server(port: u32, password: Option<&'static str>) {
    let addr = format!("127.0.0.1:{port}");
    let listener = server::bind(&addr).await.unwrap();

    tokio::spawn(async move {
        server::run(listener, password).await.unwrap();
    });

    let mut frames = get_test_data_frames();

    // Spawn a client to publish data
    let pub_frames = frames.clone();
    let mut send_client = SendClient::connect(&addr, "test-device", password)
        .await
        .unwrap();
    tokio::spawn(async move {
        loop {
            for frame in pub_frames.iter() {
                send_client.send(frame).await.unwrap();
            }
        }
    });

    // Watch data
    let mut watch_client = WatchClient::connect(addr, &["test-device".to_string()], password)
        .await
        .unwrap();
    while !frames.is_empty() {
        let frame = watch_client.recv().await.unwrap();

        let pos = frames.iter().position(|expect| expect.body == frame.body);
        if pos.is_none() {
            continue;
        }
        let expect_frame = frames.remove(pos.unwrap());

        assert_eq!(frame, expect_frame);
    }
}
