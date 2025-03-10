use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use log::{error, info};
use tokio::net::{lookup_host, TcpStream};
use tokio::sync::mpsc;

use crate::api::metadata::{Event, EventEstablished};
use crate::code;
use crate::stream::cipher::Cipher;
use crate::stream::Stream;

pub struct EventsChannel {
    pub events: mpsc::Receiver<Event>,
    pub states: mpsc::Receiver<bool>,
}

pub async fn subscribe(addr: String, username: String, password: String) -> Result<EventsChannel> {
    let (notify_tx, notify_rx) = mpsc::channel(500);
    let (error_tx, error_rx) = mpsc::channel(100);

    let client = EventListener {
        addr,
        username,
        password,
        notify: notify_tx,
        states_tx: error_tx,
    };
    tokio::spawn(async move {
        client.main_loop().await;
    });

    Ok(EventsChannel {
        events: notify_rx,
        states: error_rx,
    })
}

struct EventListener {
    addr: String,
    notify: mpsc::Sender<Event>,
    username: String,
    password: String,
    states_tx: mpsc::Sender<bool>,
}

impl EventListener {
    const MAX_RETRY_CONNECT_SECS: u64 = 30;

    async fn main_loop(&self) {
        let mut retry_connect_secs = 3;
        info!("Begin to listen server events");
        loop {
            let stream = match self.connect_server().await {
                Ok(conn) => {
                    self.states_tx.send(true).await.unwrap();
                    conn
                }
                Err(e) => {
                    self.states_tx.send(false).await.unwrap();
                    error!("Connect to events server error: {e:#}, we will retry in {retry_connect_secs} seconds");
                    tokio::time::sleep(Duration::from_secs(retry_connect_secs)).await;
                    if retry_connect_secs < Self::MAX_RETRY_CONNECT_SECS {
                        retry_connect_secs += 1;
                    }
                    continue;
                }
            };
            retry_connect_secs = 3;

            info!(
                "Connected to events server {}, begin to receive events",
                self.addr
            );
            if let Err(e) = self.listen_event(stream).await {
                error!("Listen events error: {e:#}");
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }

    async fn listen_event(&self, mut stream: Stream<TcpStream>) -> Result<()> {
        let userdata = self.username.clone().into_bytes();
        stream.write(&userdata).await?;

        let established_text = stream.next_string().await?;
        let established: EventEstablished =
            serde_json::from_str(&established_text).context("parse established data")?;
        if !established.ok {
            bail!(
                "failed to establish events connection with server: {}",
                established.message.unwrap_or_default()
            );
        }

        let password = format!("{}{}", self.password, established.salt);
        let password = code::sha256(password);
        let cipher = Cipher::new(password.into_bytes());

        stream.set_cipher(cipher);

        info!("Established events stream with server {}", self.addr);
        loop {
            let frame = stream.next_string().await?;

            let event: Event =
                serde_json::from_str(&frame).context("parse event data from server")?;
            info!("Received event from server: {:?}", event);

            self.notify
                .send(event)
                .await
                .context("send event to notify channel")?;
        }
    }

    async fn connect_server(&self) -> Result<Stream<TcpStream>> {
        let addr = self
            .parse_addr()
            .await
            .context("parse events server addr")?;

        let stream = TcpStream::connect(addr)
            .await
            .context("connect to events server")?;

        Ok(Stream::new(stream))
    }

    async fn parse_addr(&self) -> Result<SocketAddr> {
        if let Ok(addr) = self.addr.parse::<SocketAddr>() {
            return Ok(addr);
        }

        let addrs: Vec<SocketAddr> = lookup_host(&self.addr)
            .await
            .with_context(|| format!("lookup host '{}'", self.addr))?
            .collect();

        let mut lookup_result: Option<SocketAddr> = None;
        for addr in addrs {
            if addr.is_ipv4() {
                lookup_result = Some(addr);
                break;
            }
            lookup_result = Some(addr);
        }
        match lookup_result {
            Some(addr) => Ok(addr),
            None => bail!("lookup host '{}' did not have result", self.addr),
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::net::TcpListener;

    use crate::api::metadata::{EventType, Metadata};

    use super::*;

    #[tokio::test]
    async fn test_subscribe() {
        let addr = "127.0.0.1:23451";
        let events = vec![
            Event {
                event_type: EventType::Put,
                items: vec![Metadata {
                    id: 1,
                    summary: "test".to_string(),
                    owner: "test_user".to_string(),
                    ..Metadata::default()
                }],
            },
            Event {
                event_type: EventType::Update,
                items: vec![Metadata {
                    id: 1,
                    summary: "test222".to_string(),
                    ..Metadata::default()
                }],
            },
            Event {
                event_type: EventType::Delete,
                items: vec![
                    Metadata {
                        id: 1,
                        ..Metadata::default()
                    },
                    Metadata {
                        id: 2,
                        ..Metadata::default()
                    },
                    Metadata {
                        id: 3,
                        ..Metadata::default()
                    },
                ],
            },
        ];
        let server_events = events.clone();
        let username = "hello";
        let password = "test123";

        let listener = TcpListener::bind(addr).await.unwrap();
        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut stream = Stream::new(socket);
            let read_user = stream.next_string().await.unwrap();
            assert_eq!(read_user, username);

            let established = EventEstablished {
                ok: true,
                salt: String::new(),
                message: None,
            };
            let data = serde_json::to_vec(&established).unwrap();
            stream.write(&data).await.unwrap();

            let password = code::sha256(password);
            let cipher = Cipher::new(password.into_bytes());
            stream.set_cipher(cipher);

            for event in server_events {
                let data = serde_json::to_vec(&event).unwrap();
                stream.write(&data).await.unwrap();
            }
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        let mut events_sub =
            subscribe(addr.to_string(), username.to_string(), password.to_string())
                .await
                .unwrap();

        for expect_event in events {
            let event = events_sub.events.recv().await.unwrap();
            assert_eq!(event, expect_event);
        }
    }
}
