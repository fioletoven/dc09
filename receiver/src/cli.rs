use clap::Parser;
use std::net::IpAddr;

/// Test server that handles DC09 dialler connections.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// IP address to listen on.
    #[arg(default_value = "127.0.0.1")]
    pub address: IpAddr,

    /// Port number to listen on.
    #[arg(long, short, default_value = "8080")]
    pub port: u16,
}
