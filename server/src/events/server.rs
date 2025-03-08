use std::sync::Arc;

use anyhow::{Context, Result};
use csync_misc::api::metadata::EventEstablished;
use csync_misc::stream::cipher::Cipher;
use csync_misc::stream::Stream;
use log::{debug, error, info};
use tokio::net::{TcpListener, TcpStream};

use crate::context::ServerContext;
use crate::db::types::UserPassword;

use super::notify::Notifier;

pub struct EventsServer {
    addr: String,

    notifier: Notifier,

    ctx: Arc<ServerContext>,
}

impl EventsServer {
    pub fn new(addr: String, notifier: Notifier, ctx: Arc<ServerContext>) -> Self {
        Self {
            addr,
            notifier,
            ctx,
        }
    }

    pub async fn run(self) -> Result<()> {
        info!("Binding to events server: {}", self.addr);
        let listener = TcpListener::bind(&self.addr)
            .await
            .context("bind events server")?;

        info!("Begin to accept events connections");
        loop {
            let (socket, addr) = match listener.accept().await {
                Ok((socket, addr)) => (socket, addr),
                Err(e) => {
                    error!("Accept events connection error: {e:#}");
                    continue;
                }
            };

            debug!("Accept events connection from: {addr}");
            let notifier = self.notifier.clone();
            let ctx = self.ctx.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_request(socket, notifier, ctx).await {
                    error!("Handle events connection from {addr} error: {e:#}");
                }
            });
        }
    }
}

async fn handle_request(
    socket: TcpStream,
    notifier: Notifier,
    ctx: Arc<ServerContext>,
) -> Result<()> {
    let mut stream = Stream::new(socket);

    let user = stream.next_string().await?;
    let result = ctx.db.with_transaction(|tx| {
        if user == "admin" {
            return Ok(Some(UserPassword {
                name: "admin".to_string(),
                password: ctx.cfg.admin_password.clone(),
                ..Default::default()
            }));
        }
        if !tx.has_user(user.clone())? {
            return Ok(None);
        }

        let user = tx.get_user_password(user)?;
        Ok(Some(user))
    });

    let mut failed_message = "";
    let user = match result {
        Ok(Some(user)) => Some(user),
        Ok(None) => {
            failed_message = "User not found";
            None
        }
        Err(e) => {
            error!("Database get user error from events server: {e:#}");
            failed_message = "Database error";
            None
        }
    };

    let established = if user.is_some() {
        EventEstablished {
            ok: true,
            message: None,
        }
    } else {
        EventEstablished {
            ok: false,
            message: Some(failed_message.to_string()),
        }
    };

    let established = serde_json::to_vec(&established)?;
    stream.write(&established).await?;

    let user = match user {
        Some(user) => user,
        None => return Ok(()),
    };

    debug!("Established events connection for {user:?}");
    let cipher = Cipher::new(user.password.into_bytes());
    stream.set_cipher(cipher);

    let mut events_rx = notifier.subscribe(user.name).await;

    loop {
        let event = events_rx.recv().await.unwrap();

        let data = serde_json::to_vec(&event)?;
        stream.write(&data).await?;
    }
}
