use anyhow::Result;
use common::dc09::DC09Message;
use std::sync::atomic::Ordering;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::{ToSocketAddrs, UdpSocket};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use crate::metrics::AppState;
use crate::server::ResponseMode;
use crate::utils::{build_response_message, get_received_message};
use crate::utils::{increase_total_connections, process_invalid_message_metrics, process_valid_message_metrics};
use crate::{Server, ServerConfig};

static TRANSPORT_NAME: &str = "UDP";

/// Represents DC09 messages UDP receiver.
pub struct UdpServer {
    socket: Arc<UdpSocket>,
    config: ServerConfig,
    state: AppState,
}

impl Server for UdpServer {
    /// Creates new [`UdpServer`] instance.\
    /// **Note** that `key` can be provided to decrypt encrypted DC09 messages.
    async fn new(address: impl ToSocketAddrs, config: ServerConfig, state: AppState) -> Result<Self> {
        let socket = UdpSocket::bind(address).await?;
        Ok(Self {
            socket: Arc::new(socket),
            config,
            state,
        })
    }

    /// Starts listening on configured UDP address and port for incoming DC09 messages.
    async fn run(&mut self) -> Result<()> {
        let (tx, mut _rx) = unbounded_channel::<(String, SocketAddr)>();
        let _s = Arc::clone(&self.socket);

        tokio::spawn(async move {
            while let Some((response, addr)) = _rx.recv().await {
                if let Err(error) = _s.send_to(response.as_bytes(), &addr).await {
                    log::error!("{addr}: {error}");
                }
            }
        });

        self.state.udp_ready.store(true, Ordering::Relaxed);

        let mut buffer = [0; 2048];
        loop {
            let (n, addr) = self.socket.recv_from(&mut buffer).await?;
            increase_total_connections(TRANSPORT_NAME);

            match str::from_utf8(&buffer[..n]) {
                Ok(msg) => {
                    let mode = self.state.response_mode.load(Ordering::Relaxed).into();
                    process_message(&tx, addr, msg, &self.config, mode);
                },
                Err(err) => {
                    log::error!("received invalid UTF-8 sequence: {err}");
                },
            }
        }
    }
}

fn process_message(
    tx: &UnboundedSender<(String, SocketAddr)>,
    addr: SocketAddr,
    received_message: &str,
    config: &ServerConfig,
    response_mode: ResponseMode,
) {
    let key = config.get_key_for_message(received_message);
    match DC09Message::try_from(received_message, key) {
        Ok(msg) => {
            log::info!("{} -> {}", addr, get_received_message(received_message, &msg, config.mode));
            process_valid_message_metrics(TRANSPORT_NAME, received_message, &msg);

            if response_mode != ResponseMode::None {
                let response = build_response_message(msg, key, response_mode);
                log::info!("{} <- {}", addr, response.trim());
                let _ = tx.send((response, addr));
            }
        },
        Err(e) => {
            log::error!("{} -> {}: {}", addr, e, received_message.trim());
            process_invalid_message_metrics(TRANSPORT_NAME, received_message, &e);
        },
    }
}
