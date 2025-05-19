use anyhow::Result;
use common::dc09::DC09Message;
use std::net::IpAddr;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
};

/// Represents DC09 dialler.
#[derive(Clone)]
pub struct Dialler {
    address: IpAddr,
    port: u16,
    account: String,
    sequence: u16,
    key: Option<String>,
    udp: bool,
}

impl Dialler {
    /// Creates new [`Dialler`] instance.
    pub fn new(address: IpAddr, port: u16, account: String, use_udp: bool) -> Self {
        Self {
            address,
            port,
            account,
            sequence: 0,
            key: None,
            udp: use_udp,
        }
    }

    /// Sets sequence number to the provided value.
    pub fn with_start_sequence(mut self, sequence: u16) -> Self {
        self.sequence = sequence;
        self
    }

    /// Sets key that is used to decrypt and encrypt DC09 messages.
    pub fn with_key(mut self, key: Option<String>) -> Self {
        self.key = key;
        self
    }

    /// Sets new account for the dialler.
    pub fn set_account(&mut self, new_account: String) {
        self.account = new_account;
    }

    /// Sends DC09 message with specified ID token.
    pub async fn send_message(&mut self, token: String, message: String) -> Result<()> {
        self.sequence += 1;
        if self.sequence > 9999 {
            self.sequence = 1;
        }

        let message = DC09Message::new(token, self.account.clone(), self.sequence, Some(message));
        let message = if let Some(key) = self.key.as_deref() {
            message
                .to_encrypted(key)
                .expect("Cannot encrypt DC09 message with provided key")
        } else {
            message.to_string()
        };

        log::info!("{} connecting to {}:{}", self.account, self.address, self.port);
        if self.udp {
            self.send_message_udp(message).await?;
        } else {
            self.send_message_tcp(message).await?;
        }

        Ok(())
    }

    async fn send_message_tcp(&mut self, message: String) -> Result<()> {
        let mut stream = TcpStream::connect((self.address, self.port)).await?;
        stream.write_all(message.as_bytes()).await?;
        log::info!("{} >> {}", self.account, message.trim());

        let mut buffer = [0; 1024];
        match stream.read(&mut buffer).await {
            Ok(0) => log::error!("{}, connection closed by receiver", self.account),
            Ok(n) => self.process_ack_buffer(buffer, n),
            Err(e) => log::error!("{}, failed to read response: {}", self.account, e),
        }

        stream.shutdown().await?;
        Ok(())
    }

    async fn send_message_udp(&self, message: String) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect((self.address, self.port)).await?;

        let _ = socket.send(message.as_bytes()).await?;
        log::info!("{} >> {}", self.account, message.trim());

        let mut buffer = [0; 1024];
        match socket.recv(&mut buffer).await {
            Ok(n) => self.process_ack_buffer(buffer, n),
            Err(e) => log::error!("{}, failed to read response: {}", self.account, e),
        };

        Ok(())
    }

    fn process_ack_buffer(&self, buffer: [u8; 1024], n: usize) {
        match core::str::from_utf8(&buffer[..n]) {
            Ok(ack) => self.process_ack_message(ack),
            Err(e) => log::error!("{}, received invalid UTF-8 sequence: {}", self.account, e),
        }
    }

    fn process_ack_message(&self, message: &str) {
        match DC09Message::try_from(message, self.key.as_deref()) {
            Ok(msg) => match msg.validate(&self.account, self.sequence) {
                Ok(_) => log::info!("{} << {}", self.account, message.trim()),
                Err(e) => log::error!("{} << ({}) {}", self.account, e, message.trim()),
            },
            Err(e) => log::error!("{} << ({}) {}", self.account, e, message.trim()),
        };
    }
}
