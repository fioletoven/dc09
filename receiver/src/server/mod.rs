pub use self::tcp::TcpServer;
pub use self::udp::UdpServer;

mod tcp;
mod udp;

use anyhow::Result;
use common::scenarios::DiallerConfig;
use tokio::net::ToSocketAddrs;

/// Server configuration.
pub struct ServerConfig {
    pub diallers: Vec<DiallerConfig>,
    pub key: Option<String>,
    pub send_naks: bool,
}

/// Represents type that can be treated as a server.
pub trait Server: Sized {
    /// Creates new [`Server`] instance.  
    /// **Note** that `key` can be provided to decrypt encrypted DC09 messages.
    async fn new(address: impl ToSocketAddrs, config: ServerConfig) -> Result<Self>;

    /// Runs the server.
    async fn run(&mut self) -> Result<()>;
}
