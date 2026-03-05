use anyhow::Result;
use common::dc09::DC09Message;
use std::sync::atomic::{AtomicU8, Ordering};
use std::{net::SocketAddr, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::task::JoinHandle;

use crate::metrics::AppState;
use crate::server::ResponseMode;
use crate::utils::{build_response_message, get_received_message};
use crate::utils::{decrease_active_connections, increase_active_connections, increase_total_connections};
use crate::utils::{process_invalid_message_metrics, process_valid_message_metrics};
use crate::{Server, ServerConfig};

static TRANSPORT_NAME: &str = "TCP";

/// Represents DC09 messages TCP receiver.
pub struct TcpServer {
    listener: TcpListener,
    connections: Vec<JoinHandle<()>>,
    config: Arc<ServerConfig>,
    state: AppState,
}

impl Server for TcpServer {
    /// Creates new [`TcpServer`] instance.\
    /// **Note** that `key` can be provided to decrypt encrypted DC09 messages.
    async fn new(address: impl ToSocketAddrs, config: ServerConfig, state: AppState) -> Result<Self> {
        let listener = TcpListener::bind(address).await?;
        Ok(Self {
            listener,
            connections: Vec::new(),
            config: Arc::new(config),
            state,
        })
    }

    /// Starts listening on configured TCP address and port for incoming DC09 messages.
    async fn run(&mut self) -> Result<()> {
        self.state.tcp_ready.store(true, Ordering::Relaxed);

        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    let task = tokio::spawn(process_connection(
                        stream,
                        addr,
                        Arc::clone(&self.config),
                        Arc::clone(&self.state.response_mode),
                    ));
                    self.connections.push(task);
                },
                Err(e) => log::error!("error accepting connection: {e}"),
            }

            if self.connections.len() > 1_000 {
                self.connections.retain(|t| !t.is_finished());
            }
        }
    }
}

async fn process_connection(mut socket: TcpStream, addr: SocketAddr, config: Arc<ServerConfig>, mode: Arc<AtomicU8>) {
    log::debug!("accepted new connection from {addr}");
    increase_total_connections(TRANSPORT_NAME);
    increase_active_connections();

    let mut buffer = [0; 2048];
    loop {
        match socket.read(&mut buffer).await {
            Ok(0) => {
                match socket.shutdown().await {
                    Ok(()) => log::debug!("connection closed by {addr}"),
                    Err(e) => log::warn!("error while socket shutdown: {e}"),
                }

                decrease_active_connections();
                return;
            },
            Ok(n) => match str::from_utf8(&buffer[..n]) {
                Ok(msg) => {
                    let mode = mode.load(Ordering::Relaxed).into();
                    if !process_message(&mut socket, &addr, msg, &config, mode).await {
                        break;
                    }
                },
                Err(err) => {
                    log::error!("received invalid UTF-8 sequence: {err}");
                    break;
                },
            },
            Err(e) => {
                log::error!("failed to read from socket: {e}");
                break;
            },
        }
    }

    decrease_active_connections();
    match socket.shutdown().await {
        Ok(()) => log::debug!("connection closed for {addr}"),
        Err(e) => log::warn!("error while socket shutdown: {e}"),
    }
}

async fn process_message(
    socket: &mut TcpStream,
    addr: &SocketAddr,
    received_message: &str,
    config: &ServerConfig,
    response_mode: ResponseMode,
) -> bool {
    let key = config.get_key_for_message(received_message);
    match DC09Message::try_from(received_message, key) {
        Ok(msg) => {
            log::info!("{} -> {}", addr, get_received_message(received_message, &msg, config.mode));
            process_valid_message_metrics(TRANSPORT_NAME, received_message, &msg);

            if response_mode != ResponseMode::None {
                let response = build_response_message(msg, key, response_mode);
                log::info!("{} <- {}", addr, response.trim());
                let _ = socket.write_all(response.as_bytes()).await;
            }

            true
        },
        Err(e) => {
            log::error!("{} -> {}: {}", addr, e, received_message.trim());
            process_invalid_message_metrics(TRANSPORT_NAME, received_message, &e);

            false
        },
    }
}
