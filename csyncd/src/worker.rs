use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use csync_proto::auth::Auth;
use csync_proto::conn::Connection;
use csync_proto::frame::{Frame, RegisterFrame};
use log::{debug, error, warn};
use tokio::net::TcpStream;

use crate::channel::ChannelHandler;

pub struct Worker {
    publish: Option<Arc<String>>,
    subs: Option<Arc<Vec<String>>>,

    addr: Arc<String>,

    ch: ChannelHandler,

    conn: Connection,

    accept: Arc<Frame>,
}

impl Worker {
    pub async fn start(
        ch: &ChannelHandler,
        stream: TcpStream,
        addr: SocketAddr,
        accept: &Arc<Frame>,
        auth: &Arc<Option<Auth>>,
    ) {
        let addr = Arc::new(format!("{}", addr));
        let ch = ch.clone();
        let accept = Arc::clone(accept);
        let conn = Connection::new(stream, Arc::clone(auth));

        let mut worker = Worker {
            publish: None,
            subs: None,
            addr,
            ch,
            conn,
            accept,
        };
        tokio::spawn(async move {
            worker.run().await;
        });
    }

    async fn run(&mut self) {
        debug!("Begin to handle request for {}", self.addr);
        match self.main_loop().await {
            Ok(()) => {}
            Err(err) => {
                let msg = format!("{:#}", err);
                if msg.ends_with("connection was closed by peer") {
                    debug!("Connection {} was closed by peer", self.addr);
                    return;
                }

                error!("Server handle client {} error: {:#}", self.addr, err);
                if let Err(err) = self.conn.write_frame(&Frame::Error(msg)).await {
                    warn!("Send error msg to client {} failed: {:#}", self.addr, err);
                }
            }
        }

        let publish = self.publish.as_ref().map(|p| Arc::clone(p));
        let subs = self.subs.as_ref().map(|s| Arc::clone(s));

        if let Err(err) = self
            .ch
            .close(Arc::clone(&self.addr), publish.clone(), subs.clone())
            .await
        {
            warn!(
                "Close channel failed, addr: {}, publish: {:?}, subs: {:?}, error: {:#}",
                self.addr, publish, subs, err
            );
        }
    }

    async fn main_loop(&mut self) -> Result<()> {
        let register = self
            .conn
            .must_read_frame()
            .await
            .context("read register frame")?
            .expect_register()
            .context("unwrap register frame")?;
        debug!("Register({}): {:?}", self.addr, register);

        let RegisterFrame { publish, subs } = register;
        if let Some(publish) = publish {
            let publish = Arc::new(publish);
            self.ch
                .register(Arc::clone(&publish))
                .await
                .context("register to channel")?;
            self.publish = Some(publish);
        }
        if let Some(subs) = subs {
            self.subs = Some(Arc::new(subs));
        }

        self.conn
            .write_frame(&self.accept)
            .await
            .context("accept connection")?;

        loop {
            let frame = self
                .conn
                .must_read_frame()
                .await
                .context("recv data frame")?;

            if let Frame::Push(data) = &frame {
                debug!("Push({}): {data}", self.addr);
                self.handle_push(frame).await.context("handle push frame")?;
                continue;
            }

            if let Frame::Pull = frame {
                self.handle_pull().await.context("handle pull frame")?;
                continue;
            }

            warn!(
                "Recv unexpect frame from client {}, this will be ignored",
                self.addr
            );
        }
    }

    async fn handle_push(&mut self, frame: Frame) -> Result<()> {
        if let Some(publish) = self.publish.as_ref() {
            let mut data = frame.expect_data().unwrap().unwrap();
            if let None = data.from {
                data.from = Some(publish.to_string());
            }

            self.ch.push(Arc::clone(publish), Frame::Push(data)).await?;
        }
        self.conn.write_frame(&Frame::None).await
    }

    async fn handle_pull(&mut self) -> Result<()> {
        if let Some(subs) = self.subs.as_ref() {
            let frame = self
                .ch
                .pull(Arc::clone(&self.addr), Arc::clone(subs))
                .await?;
            if let Some(frame) = frame {
                if let Frame::Push(data) = frame.as_ref() {
                    debug!("Pull({}): {data}", self.addr);
                }
                self.conn.write_frame(frame.as_ref()).await?;
                return Ok(());
            }
        }
        self.conn.write_frame(&Frame::None).await
    }
}
