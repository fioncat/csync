use std::borrow::Cow;

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

use crate::net::client::Client;
use crate::net::frame::{DataFrame, Frame};

#[cfg(test)]
use crate::net::conn::Connection;

pub struct SendClient<S: AsyncWrite + AsyncRead + Unpin>(Client<S>);

impl SendClient<TcpStream> {
    #[inline]
    pub async fn connect<S, D, P>(addr: S, device: D, password: Option<P>) -> Result<Self>
    where
        S: AsRef<str>,
        D: AsRef<str>,
        P: AsRef<str>,
    {
        let client =
            Client::connect(addr, &Frame::Pub(Cow::Borrowed(device.as_ref())), password).await?;
        Ok(Self(client))
    }
}

impl<S: AsyncWrite + AsyncRead + Unpin> SendClient<S> {
    #[inline]
    #[cfg(test)]
    pub async fn new<D, P>(conn: Connection<S>, device: D, password: Option<P>) -> Result<Self>
    where
        D: AsRef<str>,
        P: AsRef<str>,
    {
        let client =
            Client::new(conn, &Frame::Pub(Cow::Borrowed(device.as_ref())), password).await?;
        Ok(Self(client))
    }

    #[inline]
    pub async fn send(&mut self, data: &DataFrame) -> Result<()> {
        self.0.send(data).await
    }
}
