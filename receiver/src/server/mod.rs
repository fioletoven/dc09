pub use self::config::ServerConfig;
pub use self::tcp::TcpServer;
pub use self::udp::UdpServer;

mod config;
mod tcp;
mod udp;

use anyhow::Result;
use tokio::net::ToSocketAddrs;

/// Represents type that can be treated as a server.
pub trait Server: Sized {
    /// Creates new [`Server`] instance.  
    /// **Note** that `key` can be provided to decrypt encrypted DC09 messages.
    async fn new(address: impl ToSocketAddrs, config: ServerConfig) -> Result<Self>;

    /// Runs the server.
    async fn run(&mut self) -> Result<()>;
}
