mod client;
mod net;
mod server;

use std::fs;
use std::path::PathBuf;

use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc::{self, Receiver};

use crate::net::conn::Connection;
use crate::net::frame::{DataFrame, DataFrameInfo, FileInfo};
use crate::utils;

fn get_test_data_frames() -> Vec<DataFrame> {
    vec![
        DataFrame {
            info: DataFrameInfo {
                device: Some(String::from("test device")),
                digest: String::from("test digest"),
                file: None,
            },

            body: b"test data".to_vec(),
        },
        DataFrame {
            info: DataFrameInfo {
                device: Some(String::from("test device")),
                digest: String::from("test digest"),
                file: Some(FileInfo {
                    name: String::from("test_file.png"),
                    mode: 123,
                }),
            },

            body: b"test file data".to_vec(),
        },
    ]
}

async fn start_server(name: &str) -> Receiver<Connection<UnixStream>> {
    let path = format!("_test_data/socks/{name}.sock");
    let _ = fs::remove_file(&path);
    utils::ensure_dir(PathBuf::from(&path).parent().unwrap()).expect("mkdir for sock file");
    let listener = UnixListener::bind(path).expect("bind server");

    let (tx, rx) = mpsc::channel::<Connection<UnixStream>>(1);
    tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.expect("accept socket");
            let conn = Connection::new(socket);
            tx.send(conn).await.expect("send mpsc");
        }
    });

    rx
}

async fn connect_server(name: &str) -> Connection<UnixStream> {
    let path = format!("_test_data/socks/{name}.sock");
    let socket = UnixStream::connect(path)
        .await
        .expect("connect to unix socket");
    Connection::new(socket)
}
