use std::{fs, io};

use anyhow::{bail, Result};
use tokio::net::UnixStream;

use crate::config::PathSet;
use crate::daemon::frame::Frame;

use super::conn::Connection;

pub struct DaemonClient {
    conn: Connection<UnixStream>,
}

impl DaemonClient {
    pub async fn connect(ps: &PathSet) -> Result<Self> {
        let path = ps.data_path.join("daemon.sock");
        match fs::metadata(&path) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                bail!("daemon is not running");
            }
            Err(e) => return Err(e.into()),
        };

        let stream = match UnixStream::connect(&path).await {
            Ok(stream) => stream,
            Err(e) => {
                bail!("connect to daemon socket: {:#}", e);
            }
        };

        Ok(Self {
            conn: Connection::new(stream),
        })
    }

    pub async fn send(&mut self, data: Vec<u8>) -> Result<()> {
        let frame = Frame { data };
        self.conn.write_frame(&frame).await?;
        Ok(())
    }
}
