use anyhow::Result;
use clap::Parser;
use common::dc09::DC09Message;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    let _logging_guard = common::logging::initialize("dialler")?;

    let args = cli::Args::parse();
    log::info!("connecting to {}:{}", args.address, args.port);

    let mut stream = TcpStream::connect((args.address, args.port)).await?;

    let message = DC09Message::new(args.token, args.account, args.sequence, args.message).to_string();
    stream.write_all(message.as_bytes()).await?;
    log::info!(">> {}", message.trim());

    let mut buffer = [0; 1024];
    match stream.read(&mut buffer).await {
        Ok(0) => log::error!("connection closed by receiver"),
        Ok(n) => match core::str::from_utf8(&buffer[..n]) {
            Ok(response) => {
                match DC09Message::try_from(response) {
                    Ok(message) => {
                        if message.sequence == args.sequence {
                            log::info!("<< {}", response.trim())
                        } else {
                            log::error!("<< (incorrect sequence in ACK) {}", response.trim())
                        }
                    },
                    Err(e) => log::error!("<< ({}) {}", e, response.trim()),
                };
            },
            Err(e) => {
                log::error!("received invalid UTF-8 sequence: {}", e);
            },
        },
        Err(e) => log::error!("failed to read response: {}", e),
    }

    Ok(())
}
