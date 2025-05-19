use anyhow::Result;
use common::dc09::DC09Message;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::UdpSocket,
    sync::mpsc::{UnboundedSender, unbounded_channel},
};

use crate::utils::build_response_message;

/// Represents DC09 messages UDP receiver.
pub struct UdpServer {
    socket: Arc<UdpSocket>,
    key: Option<String>,
    send_nak: bool,
}

impl UdpServer {
    /// Creates new [`UdpServer`] instance.  
    /// **Note** that `key` can be provided to decrypt encrypted DC09 messages.
    pub fn new(socket: UdpSocket, key: Option<String>, send_nak: bool) -> Self {
        Self {
            socket: Arc::new(socket),
            key,
            send_nak,
        }
    }

    /// Starts listening on configured UDP address and port for incoming DC09 messages.
    pub async fn run(&mut self) -> Result<()> {
        let (tx, mut _rx) = unbounded_channel::<(String, SocketAddr)>();
        let _s = self.socket.clone();

        tokio::spawn(async move {
            while let Some((response, addr)) = _rx.recv().await {
                if let Err(error) = _s.send_to(response.as_bytes(), &addr).await {
                    log::error!("{}: {}", addr, error);
                }
            }
        });

        let mut buffer = [0; 1024];
        loop {
            let (n, addr) = self.socket.recv_from(&mut buffer).await?;
            match str::from_utf8(&buffer[..n]) {
                Ok(msg) => process_message(&tx, addr, msg, self.key.as_deref(), self.send_nak),
                Err(err) => {
                    log::error!("received invalid UTF-8 sequence: {}", err);
                },
            }
        }
    }
}

fn process_message(
    tx: &UnboundedSender<(String, SocketAddr)>,
    addr: SocketAddr,
    received_message: &str,
    key: Option<&str>,
    nak: bool,
) {
    match DC09Message::try_from(received_message, key) {
        Ok(msg) => {
            log::info!("{} -> {}", addr, received_message.trim());
            let response = build_response_message(msg, key, nak);

            log::info!("{} <- {}", addr, response.trim());
            let _ = tx.send((response, addr));
        },
        Err(e) => {
            log::error!("{} -> {}: {}", addr, e, received_message.trim());
        },
    }
}
