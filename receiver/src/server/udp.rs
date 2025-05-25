use anyhow::Result;
use common::dc09::DC09Message;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::{ToSocketAddrs, UdpSocket},
    sync::mpsc::{UnboundedSender, unbounded_channel},
};

use crate::utils::build_response_message;

use super::{Server, ServerConfig};

/// Represents DC09 messages UDP receiver.
pub struct UdpServer {
    socket: Arc<UdpSocket>,
    config: ServerConfig,
}

impl Server for UdpServer {
    /// Creates new [`UdpServer`] instance.  
    /// **Note** that `key` can be provided to decrypt encrypted DC09 messages.
    async fn new(address: impl ToSocketAddrs, config: ServerConfig) -> Result<Self> {
        let socket = UdpSocket::bind(address).await?;
        Ok(Self {
            socket: Arc::new(socket),
            config,
        })
    }

    /// Starts listening on configured UDP address and port for incoming DC09 messages.
    async fn run(&mut self) -> Result<()> {
        let (tx, mut _rx) = unbounded_channel::<(String, SocketAddr)>();
        let _s = Arc::clone(&self.socket);

        tokio::spawn(async move {
            while let Some((response, addr)) = _rx.recv().await {
                if let Err(error) = _s.send_to(response.as_bytes(), &addr).await {
                    log::error!("{}: {}", addr, error);
                }
            }
        });

        let mut buffer = [0; 1536];
        loop {
            let (n, addr) = self.socket.recv_from(&mut buffer).await?;
            match str::from_utf8(&buffer[..n]) {
                Ok(msg) => process_message(&tx, addr, msg, &self.config),
                Err(err) => {
                    log::error!("received invalid UTF-8 sequence: {}", err);
                },
            }
        }
    }
}

fn process_message(tx: &UnboundedSender<(String, SocketAddr)>, addr: SocketAddr, received_message: &str, config: &ServerConfig) {
    let key = config.get_key_for_message(received_message);
    match DC09Message::try_from(received_message, key) {
        Ok(msg) => {
            log::info!("{} -> {}", addr, received_message.trim());
            let response = build_response_message(msg, key, config.send_naks);

            log::info!("{} <- {}", addr, response.trim());
            let _ = tx.send((response, addr));
        },
        Err(e) => {
            log::error!("{} -> {}: {}", addr, e, received_message.trim());
        },
    }
}
