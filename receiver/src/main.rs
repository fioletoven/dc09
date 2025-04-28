use anyhow::Result;
use clap::Parser;
use server::Server;
use tokio::net::TcpListener;

mod cli;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    let _logging_guard = common::logging::initialize("receiver")?;

    let args = cli::Args::parse();

    log::info!("start listening on {}:{}", args.address, args.port);
    let listener = TcpListener::bind(format!("{}:{}", args.address, args.port)).await?;
    let mut server = Server::new(listener);

    let _ = server.run().await;

    Ok(())
}
