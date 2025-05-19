use anyhow::Result;
use clap::Parser;
use tcp::TcpServer;
use tokio::net::{TcpListener, UdpSocket};
use udp::UdpServer;

mod cli;
mod tcp;
mod udp;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let _logging_guard = common::logging::initialize("receiver")?;

    let args = cli::Args::parse();

    log::info!("start listening on {}:{}", args.address, args.port);
    let (tcp, udp) = tokio::join!(run_tcp_receiver(args.clone()), run_udp_receiver(args));

    if let Err(error) = tcp {
        log::error!("tcp: {}", error);
    }

    if let Err(error) = udp {
        log::error!("udp: {}", error);
    }

    Ok(())
}

async fn run_udp_receiver(args: cli::Args) -> Result<()> {
    let listener = UdpSocket::bind(format!("{}:{}", args.address, args.port)).await?;
    let mut server = UdpServer::new(listener, args.key, args.nak);

    server.run().await?;

    Ok(())
}

async fn run_tcp_receiver(args: cli::Args) -> Result<()> {
    let listener = TcpListener::bind(format!("{}:{}", args.address, args.port)).await?;
    let mut server = TcpServer::new(listener, args.key, args.nak);

    server.run().await?;

    Ok(())
}
