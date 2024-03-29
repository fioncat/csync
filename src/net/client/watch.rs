use std::borrow::Cow;

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

use crate::net::client::Client;
use crate::net::frame::{DataFrame, Frame};

#[cfg(test)]
use crate::net::conn::Connection;

pub struct WatchClient<S: AsyncWrite + AsyncRead + Unpin + Send + 'static>(Client<S>);

impl WatchClient<TcpStream> {
    #[inline]
    pub async fn connect<S, P>(addr: S, devices: &[String], password: Option<P>) -> Result<Self>
    where
        S: AsRef<str>,
        P: AsRef<str>,
    {
        let client = Client::connect(addr, &Frame::Sub(Cow::Borrowed(devices)), password).await?;
        Ok(Self(client))
    }
}

impl<S: AsyncWrite + AsyncRead + Unpin + Send + 'static> WatchClient<S> {
    #[inline]
    #[cfg(test)]
    pub async fn new<P>(
        conn: Connection<S>,
        devices: &[String],
        password: Option<P>,
    ) -> Result<Self>
    where
        P: AsRef<str>,
    {
        let client = Client::new(conn, &Frame::Sub(Cow::Borrowed(devices)), password).await?;
        Ok(Self(client))
    }

    #[inline]
    pub async fn recv(&mut self) -> Result<DataFrame> {
        self.0.recv().await
    }
}
