use anyhow::Result;
use clap::Parser;
use server::{Server, ServerConfig, TcpServer, UdpServer};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::metrics::AppState;

mod cli;
mod metrics;
mod server;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let _logging_guard = common::logging::initialize("receiver")?;

    let args = cli::Args::parse();
    let state = AppState {
        tcp_ready: Arc::new(AtomicBool::new(false)),
        udp_ready: Arc::new(AtomicBool::new(false)),
        response_modes: Arc::new(args.response_modes()),
    };

    metrics::register_all();
    let metrics_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = metrics::start_metrics_server(args.address, args.metrics, metrics_state).await {
            log::error!("metrics server failed: {e}");
        }
    });

    log::info!("start listening on {}:{}", args.address, args.port);
    let (tcp, udp) = tokio::join!(
        run_receiver::<TcpServer>(&args, state.clone()),
        run_receiver::<UdpServer>(&args, state.clone())
    );

    if let Err(error) = tcp {
        log::error!("tcp: {error}");
    }

    if let Err(error) = udp {
        log::error!("udp: {error}");
    }

    Ok(())
}

async fn run_receiver<T: Server>(args: &cli::Args, state: AppState) -> Result<()> {
    let config = create_server_config(args);
    let mut server = T::new(format!("{}:{}", args.address, args.port), config, state).await?;
    server.run().await?;

    Ok(())
}

fn create_server_config(args: &cli::Args) -> ServerConfig {
    let keys = args.build_keys_map();
    let diallers = args.scenarios.as_ref().map(|s| s.diallers.clone()).unwrap_or_default();
    ServerConfig::new(&diallers, keys).with_msg_mode(args.show)
}
