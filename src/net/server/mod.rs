mod channel;
mod worker;

use std::borrow::Cow;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use log::{error, info, warn};
use tokio::net::{TcpListener, TcpStream};

use crate::net::auth::Auth;
use crate::net::conn::Connection;
use crate::net::frame::{self, AcceptFrame, Frame};
use crate::net::server::channel::ChannelRequest;
use crate::net::server::worker::Worker;
use crate::utils::BuildInfo;

pub async fn bind<S: AsRef<str>>(addr: S) -> Result<TcpListener> {
    let bind: SocketAddr = addr
        .as_ref()
        .parse()
        .with_context(|| format!("parse bind addr '{}'", addr.as_ref()))?;
    TcpListener::bind(&bind)
        .await
        .with_context(|| format!("bind to '{}'", addr.as_ref()))
}

pub async fn run<P: AsRef<str>>(listener: TcpListener, password: Option<P>) -> Result<()> {
    BuildInfo::new().log();

    let channel = ChannelRequest::new().await;

    let auth = password.map(|password| {
        info!("[Main] Generate auth key from your password");
        Auth::new(password)
    });
    if auth.is_none() {
        warn!("[Main] Not providing a password is not recommended. This will expose your clipboard data to the network");
    }
    let auth = Arc::new(auth);

    let auth_frame = match auth.as_ref() {
        Some(auth) => Some(auth.build_frame().context("build auth frame")?),
        None => None,
    };

    let accept_frame = Cow::Owned(AcceptFrame {
        version: frame::PROTOCOL_VERSION,
        auth: auth_frame,
    });
    let accept_frame = Frame::Accept(accept_frame);
    let accept_frame = Arc::new(accept_frame);

    info!("[Main] Begin to accept connections");
    loop {
        let (stream, addr) = match listener.accept().await {
            Ok((stream, addr)) => (stream, addr),
            Err(err) => {
                error!("[Main] Accept tcp connection failed: {:#}", err);
                continue;
            }
        };

        let auth = Arc::clone(&auth);

        let accept_frame = Arc::clone(&accept_frame);
        let channel = channel.clone();
        tokio::spawn(async move {
            let addr_str = addr.to_string();
            if let Err(err) = accept_connection(stream, addr, channel, auth, accept_frame).await {
                error!("[Main] Handle client '{addr_str}' error: {:#}", err);
            }
        });
    }
}

pub async fn start<A, P>(addr: A, password: Option<P>) -> Result<()>
where
    A: AsRef<str>,
    P: AsRef<str>,
{
    let listener = bind(addr).await?;
    run(listener, password).await
}

enum WorkerType {
    Sub(Vec<String>),
    Pub(String),
}

async fn accept_connection(
    stream: TcpStream,
    addr: SocketAddr,
    channel: ChannelRequest,
    auth: Arc<Option<Auth>>,
    accept_frame: Arc<Frame<'_>>,
) -> Result<()> {
    let mut conn = Connection::new(stream);
    let head_frame = conn
        .must_read_frame()
        .await
        .context("read head frame from client")?;

    let worker_type = match head_frame {
        Frame::Pub(device) => WorkerType::Pub(device.into_owned()),
        Frame::Sub(devices) => WorkerType::Sub(devices.into_owned()),
        _ => bail!("unexpect head frame from client, expect pub or sub"),
    };

    conn.write_frame(&accept_frame)
        .await
        .context("write accept frame to client")?;
    conn.with_auth(auth);

    let mut worker = Worker::new(channel, conn, addr);
    match worker_type {
        WorkerType::Sub(devices) => worker.run_sub(devices).await,
        WorkerType::Pub(device) => worker.run_pub(device).await,
    }

    Ok(())
}
