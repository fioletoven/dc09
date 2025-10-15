use anyhow::Result;
use common::{dc09::DC09Message, scenarios::SignalConfig, time::OffsetDateTime, utils::SharedKeysMap};
use std::{collections::VecDeque, net::IpAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
};

use crate::cli::SharedSignalsMap;

/// Represents DC09 dialler.
#[derive(Clone)]
pub struct Dialler {
    address: IpAddr,
    port: u16,
    receiver: Option<String>,
    line_prefix: Option<String>,
    account: String,
    sequence: u16,
    key: Option<(SharedKeysMap, u16)>,
    udp: bool,
    signals: SharedSignalsMap,
    queue: VecDeque<(u16, u16)>,
    timeout: Option<Duration>,
}

impl Dialler {
    /// Creates new [`Dialler`] instance.
    pub fn new(address: IpAddr, port: u16, account: String, signals: SharedSignalsMap, use_udp: bool) -> Self {
        Self {
            address,
            port,
            receiver: None,
            line_prefix: None,
            account,
            sequence: 0,
            key: None,
            udp: use_udp,
            signals,
            queue: VecDeque::new(),
            timeout: None,
        }
    }

    /// Sets receiver number to the provided value.
    pub fn with_receiver_number(mut self, receiver: Option<String>) -> Self {
        self.receiver = receiver;
        self
    }

    /// Sets line prefix to the provided value.
    pub fn with_line_prefix(mut self, prefix: Option<String>) -> Self {
        self.line_prefix = prefix;
        self
    }

    /// Sets sequence number to the provided value.
    pub fn with_start_sequence(mut self, sequence: u16) -> Self {
        self.sequence = sequence;
        self
    }

    /// Sets key that is used to decrypt and encrypt DC09 messages.
    pub fn with_key(mut self, keys: SharedKeysMap, index: u16) -> Self {
        self.key = Some((keys, index));
        self
    }

    /// Sets the optional timeout duration for receiving a message.
    pub fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }

    /// Returns key that can be used to decrypt and encrypt DC09 messages.
    pub fn key(&self) -> Option<&str> {
        self.key
            .as_ref()
            .and_then(|(keys, index)| keys.get(index))
            .map(String::as_str)
    }

    /// Adds default signal to the queue.
    pub fn add_default_signal(&mut self) {
        self.queue.push_back((0, 0));
    }

    /// Gets dialler's account.
    pub fn account(&self) -> &str {
        &self.account
    }

    /// Gets dialler's signals queue.
    pub fn queue(&mut self) -> &mut VecDeque<(u16, u16)> {
        &mut self.queue
    }

    /// Sends sequence of messages from the queue.\
    /// **Note** that it will stop draining the queue on error.
    pub async fn run_sequence(&mut self) {
        log::info!("{}    start sending signals", self.account);
        'outer: while let Some(item) = self.queue.pop_front() {
            if let Some(signal) = self.signals.get(&item).cloned() {
                let repeat = signal.repeat.max(1) - 1;
                for _ in 0..repeat {
                    if !self.send_signal(signal.clone()).await {
                        break 'outer;
                    }
                }

                if !self.send_signal(signal).await {
                    break 'outer;
                }
            }
        }
    }

    /// Sends DC09 message with specified ID token.
    pub async fn send_message(&mut self, token: String, message: String) -> Result<()> {
        self.sequence += 1;
        if self.sequence > 9999 {
            self.sequence = 1;
        }

        let message = DC09Message::new(token, self.account.clone(), self.sequence, Some(message))
            .with_receiver(self.receiver.clone())
            .with_line_prefix(self.line_prefix.clone());
        let message = if let Some(key) = self.key() {
            message
                .with_timestamp(OffsetDateTime::now_utc())
                .to_encrypted(key)
                .expect("Cannot encrypt DC09 message with provided key")
        } else {
            message.to_string()
        };

        log::info!("{}    connecting to {}:{}", self.account, self.address, self.port);
        if self.udp {
            self.send_message_udp(message, self.timeout).await?;
        } else {
            self.send_message_tcp(message, self.timeout).await?;
        }

        Ok(())
    }

    async fn send_signal(&mut self, signal: SignalConfig) -> bool {
        if signal.delay > 50 {
            tokio::time::sleep(Duration::from_millis(signal.delay.into())).await;
        }

        let message = signal.message.map(|m| format!("#{}|{}", self.account, m)).unwrap_or_default();
        if let Err(error) = self.send_message(signal.token, message).await {
            log::error!("{}    {}", self.account, error);
            return false;
        }

        true
    }

    async fn send_message_tcp(&mut self, message: String, timeout: Option<Duration>) -> Result<()> {
        let mut stream = TcpStream::connect((self.address, self.port)).await?;
        stream.write_all(message.as_bytes()).await?;
        log::info!("{} >> {}", self.account, message.trim());

        let mut buffer = [0; 1024];
        let read_future = async {
            match stream.read(&mut buffer).await {
                Ok(0) => log::error!("{}    connection closed by receiver", self.account),
                Ok(n) => self.process_ack_buffer(&buffer, n),
                Err(e) => log::error!("{}    failed to read response: {}", self.account, e),
            }
        };

        match timeout {
            Some(timeout) => {
                if (tokio::time::timeout(timeout, read_future).await).is_err() {
                    log::warn!("{}    response timed out after {:?}", self.account, timeout);
                }
            },
            None => read_future.await,
        }

        stream.shutdown().await?;
        Ok(())
    }

    async fn send_message_udp(&self, message: String, timeout: Option<Duration>) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect((self.address, self.port)).await?;

        let _ = socket.send(message.as_bytes()).await?;
        log::info!("{} >> {}", self.account, message.trim());

        let mut buffer = [0; 1024];
        let recv_future = async {
            match socket.recv(&mut buffer).await {
                Ok(n) => self.process_ack_buffer(&buffer, n),
                Err(e) => log::error!("{}    failed to read response: {}", self.account, e),
            }
        };

        match timeout {
            Some(timeout) => {
                if (tokio::time::timeout(timeout, recv_future).await).is_err() {
                    log::warn!("{}    response timed out after {:?}", self.account, timeout);
                }
            },
            None => recv_future.await,
        }

        Ok(())
    }

    fn process_ack_buffer(&self, buffer: &[u8; 1024], n: usize) {
        match core::str::from_utf8(&buffer[..n]) {
            Ok(ack) => self.process_ack_message(ack),
            Err(e) => log::error!("{}    received invalid UTF-8 sequence: {}", self.account, e),
        }
    }

    fn process_ack_message(&self, message: &str) {
        match DC09Message::try_from(message, self.key()) {
            Ok(msg) => match msg.validate(&self.account, self.sequence) {
                Ok(()) => log::info!("{} << {}", self.account, message.trim()),
                Err(e) => log::error!("{} << ({}) {}", self.account, e, message.trim()),
            },
            Err(e) => log::error!("{} << ({}) {}", self.account, e, message.trim()),
        }
    }
}
