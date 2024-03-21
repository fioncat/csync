use std::borrow::Cow;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use log::info;
use tokio::net::TcpStream;
use tokio::select;
use tokio::time::{self, Instant};

use crate::net::conn::Connection;
use crate::net::frame::{DataFrame, Frame};
use crate::net::server::channel::ChannelRequest;

pub struct Worker {
    channel: ChannelRequest,

    conn: Connection<TcpStream>,

    addr: String,
}

impl Worker {
    #[inline]
    pub fn new(channel: ChannelRequest, conn: Connection<TcpStream>, addr: SocketAddr) -> Self {
        let addr = addr.to_string();

        Self {
            channel,
            conn,
            addr,
        }
    }

    pub async fn run_sub(&mut self, devices: Vec<String>) {
        let log_header = format!("[Worker] Sub '{}' <- {:?}:", self.addr, devices);
        info!("{log_header} Loop begin");

        let ping_frame = Frame::Ping;
        let mut ping_intv = time::interval_at(Instant::now(), Duration::from_secs(1));

        let devices = Arc::new(devices);

        let (uuid, mut data_rx) = self.channel.subscribe(Arc::clone(&devices)).await;
        loop {
            select! {
                data = data_rx.recv() => {
                    let data = data.unwrap();
                    let frame = Frame::Data(Cow::Borrowed(&data));
                    if let Err(err) = self.conn.write_frame(&frame).await {
                        info!("{log_header} Send data to client close: {:#}", err);
                        break;
                    }

                    info!("{log_header} {} data", data.body.len());
                },

                _ = ping_intv.tick() => {
                    if let Err(err) = self.conn.write_frame(&ping_frame).await {
                        info!("{log_header} Ping client close: {:#}", err);
                        break;
                    }
                },
            }
        }

        self.channel.close(uuid).await;
    }

    pub async fn run_pub(&mut self, device: String) {
        let log_header = format!("[Worker] Pub '{}' -> '{device}': ", self.addr);
        info!("{log_header} Loop begin");

        let device = Arc::new(device);

        let ok_frame = Frame::Ok;

        // TODO: Oneday we will use this...
        let _ = Frame::Error(Cow::Borrowed(""));

        loop {
            let result = self.recv_data().await;
            if let Err(err) = result {
                info!("{log_header} Receive data from client close: {:#} ", err);
                break;
            }

            let data = result.unwrap();
            let data_len = data.body.len();
            self.channel.publish(Arc::clone(&device), data).await;

            if let Err(err) = self.conn.write_frame(&ok_frame).await {
                info!("{log_header} Write ok frame to client close: {:#}", err);
                break;
            }

            info!("{log_header} {data_len} data");
        }
    }

    async fn recv_data(&mut self) -> Result<DataFrame> {
        match self.conn.must_read_frame().await {
            Ok(frame) => Ok(frame.expect_data()?),
            Err(err) => Err(err),
        }
    }
}
