use anyhow::Result;
use log::info;

use crate::daemon::server::DaemonServer;
use crate::server::restful::RestfulServer;

/// Union type for different server implementations
pub enum Server {
    /// Restful API server
    Server(RestfulServer),
    /// Daemon API server
    Daemon(DaemonServer),
}

impl Server {
    /// Starts the server based on its type
    pub async fn run(self) -> Result<()> {
        info!(
            "Build info: version: {}, buildType: {}, commit: {}",
            env!("CSYNC_VERSION"),
            env!("CSYNC_BUILD_TYPE"),
            env!("CSYNC_SHA")
        );
        match self {
            Server::Server(server) => server.run().await,
            Server::Daemon(server) => server.run().await,
        }
    }
}
