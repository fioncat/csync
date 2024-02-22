use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use tokio::net::lookup_host;

use crate::auth::Auth;
use crate::conn::Connection;
use crate::frame::{self, DataFrame, Frame, RegisterFrame};

pub trait Password {
    fn get(self) -> Result<String>;
}

pub struct StaticPassword(String);

impl Password for StaticPassword {
    fn get(self) -> Result<String> {
        Ok(self.0)
    }
}

impl StaticPassword {
    pub fn new<S: ToString>(s: S) -> StaticPassword {
        StaticPassword(s.to_string())
    }
}

pub struct TerminalPassword {
    no_auth: bool,
}

impl Password for TerminalPassword {
    fn get(self) -> Result<String> {
        if self.no_auth {
            bail!("require password to auth server");
        }
        let password =
            rpassword::prompt_password("[csync] password: ").context("input password from tty")?;
        if password.is_empty() {
            bail!("password cannot be empty");
        }
        Ok(password)
    }
}

impl TerminalPassword {
    pub fn new(no_auth: bool) -> TerminalPassword {
        TerminalPassword { no_auth }
    }
}

pub struct Client<P: Password> {
    publish: Option<String>,
    subs: Option<Vec<String>>,

    password: Option<P>,

    conn: Connection,
}

impl<P: Password> Client<P> {
    pub async fn dial<S: AsRef<str>>(
        addr: S,
        publish: Option<String>,
        subs: Option<Vec<String>>,
        password: P,
    ) -> Result<Client<P>> {
        if publish.is_none() && subs.is_none() {
            bail!("publish and subs cannot be empty at the same time");
        }

        let addr = Self::parse_addr(addr).await?;
        let conn = Connection::dial(&addr, Arc::new(None)).await?;
        let mut client = Client {
            publish,
            subs,
            password: Some(password),
            conn,
        };
        client.register().await.context("register to server")?;
        Ok(client)
    }

    async fn parse_addr<S: AsRef<str>>(addr: S) -> Result<SocketAddr> {
        if let Ok(addr) = addr.as_ref().parse::<SocketAddr>() {
            return Ok(addr);
        }

        let addrs: Vec<SocketAddr> = lookup_host(addr.as_ref())
            .await
            .with_context(|| format!("lookup host '{}'", addr.as_ref()))?
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
            None => bail!("lookup host '{}' did not have result", addr.as_ref()),
        }
    }

    pub async fn pull(&mut self) -> Result<Option<DataFrame>> {
        self.conn.write_frame(&Frame::Pull).await?;

        let resp = self.conn.must_read_frame().await?;
        let data = resp.expect_data()?;
        Ok(data)
    }

    pub async fn push(&mut self, frame: DataFrame) -> Result<()> {
        let frame = Frame::Push(frame);
        self.conn.write_frame(&frame).await?;

        let resp = self.conn.must_read_frame().await?;
        resp.expect_none()?;

        Ok(())
    }

    async fn register(&mut self) -> Result<()> {
        let frame = Frame::Register(RegisterFrame {
            publish: self.publish.clone(),
            subs: self.subs.clone(),
        });

        self.conn.write_frame(&frame).await?;

        let resp = self.conn.must_read_frame().await?;
        let mut accept = resp.expect_accept()?;

        if accept.version != frame::PROTOCOL_VERSION {
            bail!(
                "protocol version mismatched, client is: {}, server is: {}",
                frame::PROTOCOL_VERSION,
                accept.version
            );
        }

        let auth = accept.auth.take();
        if let Some(auth_frame) = auth {
            let password = self.password.take().unwrap();
            let password = password.get()?;

            let auth = Auth::from_frame(password, auth_frame).context("auth failed")?;
            self.conn.set_auth(auth);
        }

        Ok(())
    }
}
