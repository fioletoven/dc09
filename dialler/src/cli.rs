use clap::Parser;
use std::net::IpAddr;

/// Test client that sends DC09 messages.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// IP address of the receiver.
    #[arg(default_value = "127.0.0.1")]
    pub address: IpAddr,

    /// Port number of the receiver.
    #[arg(long, short, default_value = "8080")]
    pub port: u16,

    /// ID token.
    #[arg(long, short, default_value = "SIA-DCS")]
    pub token: String,

    /// Message to send.
    #[arg(long, short, default_value = "#1234|NRR|AStart of dialler")]
    pub message: String,

    /// Dialler account number.
    #[arg(long, short, default_value = "1234")]
    pub account: String,

    /// Message sequence start number.
    #[arg(long, short, default_value = "1")]
    pub sequence: u16,

    /// Repeat message the specified number of times.
    #[arg(long, short, default_value = "1")]
    pub repeat: u16,
}
