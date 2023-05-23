use std::net::SocketAddr;

use csync::{net::Frame, server::Server};
use tokio::sync::mpsc;

#[tokio::test]
async fn server() {
    let addr: SocketAddr = String::from("0.0.0.0:9908").parse().unwrap();
    let (sender, _receiver) = mpsc::channel::<Frame>(512);
    let mut srv = Server::new(&addr, sender, 100).await.unwrap();
    tokio::spawn(async move { srv.run().await.unwrap() });
}
