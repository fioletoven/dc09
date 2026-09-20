use anyhow::Result;
use clap::Parser;
use server::{Server, ServerConfig, TcpServer, UdpServer};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::metrics::{AppState, RecorderHandle};

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
        recorder: RecorderHandle::new(),
    };

    metrics::register_all();
    let metrics_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = metrics::start_metrics_server(args.address, args.metrics, metrics_state).await {
            log::error!("metrics server failed: {e}");
        }
    });

    log::info!("start listening on {}:{}", args.address, args.port);
    tokio::select! {
        () = run_receiver::<TcpServer>(&args, state.clone(), "tcp") => (),
        () = run_receiver::<UdpServer>(&args, state.clone(), "udp") => (),
    }

    Ok(())
}

async fn run_receiver<T: Server>(args: &cli::Args, state: AppState, protocol: &str) {
    if let Err(error) = run_receiver_internal::<T>(args, state, protocol == "tcp").await {
        log::error!("{protocol}: {error}");
    }
}

async fn run_receiver_internal<T: Server>(args: &cli::Args, state: AppState, is_tcp: bool) -> Result<()> {
    let config = create_server_config(args, is_tcp)?;
    let mut server = T::new(format!("{}:{}", args.address, args.port), config, state).await?;
    server.run().await?;

    Ok(())
}

fn create_server_config(args: &cli::Args, is_tcp: bool) -> Result<ServerConfig> {
    let keys = args.build_keys_map();
    let diallers = args.scenarios.as_ref().map(|s| s.diallers.clone()).unwrap_or_default();
    let config = ServerConfig::new(&diallers, keys).with_msg_mode(args.show);

    if !is_tcp {
        return Ok(config);
    }

    match args.tls_files() {
        Some((cert_path, key_path)) => config.with_tls(cert_path, key_path),
        None => Ok(config),
    }
}
