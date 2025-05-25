use clap::Parser;
use common::{
    scenarios::Scenarios,
    utils::{parse_key, parse_scenarios_path},
};
use std::net::IpAddr;

/// Test server that handles DC09 dialler connections.
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// IP address to listen on.
    #[arg(default_value = "127.0.0.1")]
    pub address: IpAddr,

    /// Port number to listen on.
    #[arg(long, short, default_value = "8080")]
    pub port: u16,

    /// Key to decrypt DC09 messages (16, 24 or 32 bytes long).
    #[arg(long, short, value_parser = parse_key)]
    pub key: Option<String>,

    /// Send `NAK` instead of `ACK` for received messages.
    #[arg(long)]
    pub nak: bool,

    /// Configuration file specifying defined scenarios for the run.
    #[arg(long, value_parser = parse_scenarios_path)]
    pub scenarios: Option<Scenarios>,
}
