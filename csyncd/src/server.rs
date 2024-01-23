use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use csync_proto::auth::Auth;
use csync_proto::frame::{self, AcceptFrame, Frame};
use log::{error, info, warn};
use tokio::net::TcpListener;

use crate::channel::ChannelHandler;
use crate::config::Config;
use crate::worker::Worker;

pub async fn start(cfg: &Config) -> Result<()> {
    let bind: SocketAddr = cfg
        .addr
        .parse()
        .with_context(|| format!("parse bind addr '{}'", cfg.addr))?;
    let listener = TcpListener::bind(&bind)
        .await
        .with_context(|| format!("bind to '{}'", cfg.addr))?;
    info!("Start to listen on '{}'", cfg.addr);

    let ch = ChannelHandler::new().await;
    let auth = match cfg.password.as_ref() {
        Some(password) => {
            info!("Generate auth key according to provided password");
            Some(Auth::new(password))
        }
        None => {
            warn!("Not providing a password is not recommended. This will expose your clipboard data to the network");
            None
        }
    };

    let auth_frame = match auth.as_ref() {
        Some(auth) => Some(auth.build_frame().context("build auth fame")?),
        None => None,
    };
    let accept_frame = Frame::Accept(AcceptFrame {
        version: frame::PROTOCOL_VERSION,
        auth: auth_frame,
    });

    let auth = Arc::new(auth);
    let accept_frame = Arc::new(accept_frame);

    info!("Begin to accept connections");
    loop {
        let (stream, addr) = match listener.accept().await {
            Ok((stream, addr)) => (stream, addr),
            Err(err) => {
                error!("Accept tcp connection failed: {:#}", err);
                continue;
            }
        };
        Worker::start(&ch, stream, addr, &accept_frame, &auth).await;
    }
}
