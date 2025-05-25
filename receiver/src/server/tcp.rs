use anyhow::Result;
use common::dc09::DC09Message;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    task::JoinHandle,
};

use crate::utils::build_response_message;

use super::{Server, ServerConfig};

/// Represents DC09 messages TCP receiver.
pub struct TcpServer {
    listener: TcpListener,
    connections: Vec<JoinHandle<()>>,
    config: Arc<ServerConfig>,
}

impl Server for TcpServer {
    /// Creates new [`TcpServer`] instance.  
    /// **Note** that `key` can be provided to decrypt encrypted DC09 messages.
    async fn new(address: impl ToSocketAddrs, config: ServerConfig) -> Result<Self> {
        let listener = TcpListener::bind(address).await?;
        Ok(Self {
            listener,
            connections: Vec::new(),
            config: Arc::new(config),
        })
    }

    /// Starts listening on configured TCP address and port for incoming DC09 messages.
    async fn run(&mut self) -> Result<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    self.connections
                        .push(tokio::spawn(process_connection(stream, addr, Arc::clone(&self.config))))
                },
                Err(e) => log::error!("error accepting connection: {}", e),
            };

            if self.connections.len() > 1_000 {
                self.connections.retain(|t| !t.is_finished());
            }
        }
    }
}

async fn process_connection(mut socket: TcpStream, addr: SocketAddr, config: Arc<ServerConfig>) {
    log::debug!("accepted new connection from {}", addr);

    let mut buffer = [0; 1536];
    loop {
        match socket.read(&mut buffer).await {
            Ok(0) => {
                match socket.shutdown().await {
                    Ok(_) => log::debug!("connection closed by {}", addr),
                    Err(e) => log::warn!("error while socket shutdown: {}", e),
                }

                return;
            },
            Ok(n) => match str::from_utf8(&buffer[..n]) {
                Ok(msg) => {
                    if !process_message(&mut socket, &addr, msg, &config).await {
                        break;
                    }
                },
                Err(err) => {
                    log::error!("received invalid UTF-8 sequence: {}", err);
                    break;
                },
            },
            Err(e) => {
                log::error!("failed to read from socket: {}", e);
                break;
            },
        }
    }

    match socket.shutdown().await {
        Ok(_) => log::debug!("connection closed for {}", addr),
        Err(e) => log::warn!("error while socket shutdown: {}", e),
    }
}

async fn process_message(socket: &mut TcpStream, addr: &SocketAddr, received_message: &str, config: &ServerConfig) -> bool {
    let key = config.get_key_for_message(received_message);
    match DC09Message::try_from(received_message, key) {
        Ok(msg) => {
            log::info!("{} -> {}", addr, received_message.trim());
            let response = build_response_message(msg, key, config.send_naks);

            log::info!("{} <- {}", addr, response.trim());
            let _ = socket.write_all(response.as_bytes()).await;

            true
        },
        Err(e) => {
            log::error!("{} -> {}: {}", addr, e, received_message.trim());

            false
        },
    }
}
