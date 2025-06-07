use anyhow::Result;
use clap::Parser;
use server::{Server, ServerConfig, TcpServer, UdpServer};

mod cli;
mod server;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let _logging_guard = common::logging::initialize("receiver")?;

    let args = cli::Args::parse();

    log::info!("start listening on {}:{}", args.address, args.port);
    let (tcp, udp) = tokio::join!(run_receiver::<TcpServer>(&args), run_receiver::<UdpServer>(&args));

    if let Err(error) = tcp {
        log::error!("tcp: {}", error);
    }

    if let Err(error) = udp {
        log::error!("udp: {}", error);
    }

    Ok(())
}

async fn run_receiver<T: Server>(args: &cli::Args) -> Result<()> {
    let config = create_server_config(args);
    let mut server = T::new(format!("{}:{}", args.address, args.port), config).await?;
    server.run().await?;

    Ok(())
}

fn create_server_config(args: &cli::Args) -> ServerConfig {
    let keys = args.build_keys_map();
    let diallers = args.scenarios.as_ref().map(|s| s.diallers.clone()).unwrap_or_default();
    ServerConfig::new(diallers, keys, args.nak)
}
