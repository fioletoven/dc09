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

    if let Err(error) = run_receiver(args).await {
        log::error!("{}", error);
    }

    Ok(())
}

async fn run_receiver(args: cli::Args) -> Result<()> {
    log::info!("start listening on {}:{}", args.address, args.port);
    let listener = TcpListener::bind(format!("{}:{}", args.address, args.port)).await?;
    let mut server = Server::new(listener);

    server.run(args.key).await?;

    Ok(())
}
