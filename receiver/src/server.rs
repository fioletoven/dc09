use anyhow::Result;
use common::dc09::DC09Message;
use std::net::SocketAddr;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

/// Represents DC09 messages receiver.
pub struct Server {
    listener: TcpListener,
    connections: Vec<JoinHandle<()>>,
}

impl Server {
    /// Creates new [`Server`] instance.
    pub fn new(listener: TcpListener) -> Self {
        Self {
            listener,
            connections: Vec::new(),
        }
    }

    /// Starts listening on configured address and port for incoming DC09 messages.  
    /// **Note** that `key` can be provided that will be used to decrypt encrypted DC09 messages.
    pub async fn run(&mut self, key: Option<String>) -> Result<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => self
                    .connections
                    .push(tokio::spawn(process_connection(stream, addr, key.clone()))),
                Err(e) => log::error!("error accepting connection: {}", e),
            };

            if self.connections.len() > 1_000 {
                self.connections.retain(|t| !t.is_finished());
            }
        }
    }
}

async fn process_connection(mut socket: TcpStream, addr: SocketAddr, key: Option<String>) {
    log::debug!("accepted new connection from {}", addr);

    let mut buffer = [0; 1024];
    loop {
        match socket.read(&mut buffer).await {
            Ok(0) => {
                log::debug!("connection closed by {}", addr);
                return;
            },
            Ok(n) => match core::str::from_utf8(&buffer[..n]) {
                Ok(msg) => {
                    if !process_message(&mut socket, &addr, msg, key.as_deref()).await {
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

    log::debug!("connection closed for {}", addr);
}

async fn process_message(socket: &mut TcpStream, addr: &SocketAddr, received_message: &str, key: Option<&str>) -> bool {
    match DC09Message::try_from(received_message, key) {
        Ok(msg) => {
            log::info!("{} -> {}", addr, received_message.trim());
            let ack = DC09Message::ack(msg.account, msg.sequence).to_string();

            log::info!("{} <- {}", addr, ack.trim());
            let _ = socket.write_all(ack.as_bytes()).await;

            true
        },
        Err(e) => {
            log::error!("{} -> {}: {}", addr, e, received_message.trim());

            false
        },
    }
}
