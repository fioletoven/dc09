use anyhow::Result;
use common::dc09::DC09Message;
use std::net::SocketAddr;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

use crate::utils::build_response_message;

/// Represents DC09 messages TCP receiver.
pub struct TcpServer {
    listener: TcpListener,
    connections: Vec<JoinHandle<()>>,
    key: Option<String>,
    send_nak: bool,
}

impl TcpServer {
    /// Creates new [`TcpServer`] instance.  
    /// **Note** that `key` can be provided to decrypt encrypted DC09 messages.
    pub fn new(listener: TcpListener, key: Option<String>, send_nak: bool) -> Self {
        Self {
            listener,
            connections: Vec::new(),
            key,
            send_nak,
        }
    }

    /// Starts listening on configured TCP address and port for incoming DC09 messages.
    pub async fn run(&mut self) -> Result<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => self.connections.push(tokio::spawn(process_connection(
                    stream,
                    addr,
                    self.key.clone(),
                    self.send_nak,
                ))),
                Err(e) => log::error!("error accepting connection: {}", e),
            };

            if self.connections.len() > 1_000 {
                self.connections.retain(|t| !t.is_finished());
            }
        }
    }
}

async fn process_connection(mut socket: TcpStream, addr: SocketAddr, key: Option<String>, nak: bool) {
    log::debug!("accepted new connection from {}", addr);

    let mut buffer = [0; 1024];
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
                    if !process_message(&mut socket, &addr, msg, key.as_deref(), nak).await {
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

async fn process_message(
    socket: &mut TcpStream,
    addr: &SocketAddr,
    received_message: &str,
    key: Option<&str>,
    nak: bool,
) -> bool {
    match DC09Message::try_from(received_message, key) {
        Ok(msg) => {
            log::info!("{} -> {}", addr, received_message.trim());
            let response = build_response_message(msg, key, nak);

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
