use clap::Parser;
use common::{
    scenarios::Scenarios,
    utils::{parse_key, parse_scenarios_path},
};
use std::net::IpAddr;

/// Test client that sends DC09 messages.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// IP address of the receiver.
    #[arg(default_value = "127.0.0.1")]
    pub address: IpAddr,

    /// Port number of the receiver.
    #[arg(long, short, default_value_t = 8080)]
    pub port: u16,

    /// ID token.
    #[arg(long, short, default_value = "SIA-DCS")]
    pub token: String,

    /// Message to send.
    #[arg(long, short, default_value = "#1234|NRR|AStart of dialler")]
    pub message: String,

    /// Dialler account number (automatically incremented if possible).
    #[arg(long, short, default_value = "1234")]
    pub account: String,

    /// Ensure that the account number is fixed across all diallers.
    #[arg(long, short)]
    pub fixed: bool,

    /// Message sequence start number.
    #[arg(long, short, default_value_t = 1)]
    pub sequence: u16,

    /// Number of diallers to create.
    #[arg(long, short, default_value_t = 1)]
    pub diallers: u16,

    /// Repeat message the specified number of times per dialler.
    #[arg(long, short, default_value_t = 1)]
    pub repeat: u16,

    /// Key to encrypt DC09 messages (16, 24 or 32 bytes long).
    #[arg(long, short, value_parser = parse_key)]
    pub key: Option<String>,

    /// Use a UDP connection instead of a TCP one.
    #[arg(long, short)]
    pub udp: bool,

    /// File with defined scenarios for this run.
    #[arg(long, value_parser = parse_scenarios_path)]
    pub scenarios: Option<Scenarios>,
}
