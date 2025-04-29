use anyhow::Result;
use clap::Parser;
use dialler::Dialler;

mod cli;
mod dialler;

#[tokio::main]
async fn main() -> Result<()> {
    let _logging_guard = common::logging::initialize("dialler")?;

    let args = cli::Args::parse();

    if let Err(error) = run_diallers(args).await {
        log::error!("{}", error);
    }

    Ok(())
}

async fn run_diallers(args: cli::Args) -> Result<()> {
    let mut dialler = Dialler::new(args.address, args.port, args.account).with_start_sequence(args.sequence.saturating_sub(1));

    for _ in 0..args.repeat {
        dialler.send_message(args.token.clone(), args.message.clone()).await?;
    }

    Ok(())
}
