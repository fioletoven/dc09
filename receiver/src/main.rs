use anyhow::Result;
use clap::Parser;
use server::{Server, TcpServer, UdpServer};

mod cli;
mod server;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let _logging_guard = common::logging::initialize("receiver")?;

    let args = cli::Args::parse();

    log::info!("start listening on {}:{}", args.address, args.port);
    let (tcp, udp) = tokio::join!(run_receiver::<TcpServer>(args.clone()), run_receiver::<UdpServer>(args));

    if let Err(error) = tcp {
        log::error!("tcp: {}", error);
    }

    if let Err(error) = udp {
        log::error!("udp: {}", error);
    }

    Ok(())
}

async fn run_receiver<T: Server>(args: cli::Args) -> Result<()> {
    let mut server = T::new(format!("{}:{}", args.address, args.port), args.key, args.nak).await?;
    server.run().await?;

    Ok(())
}
