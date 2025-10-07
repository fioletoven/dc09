use anyhow::Result;
use clap::Parser;

use crate::utils::{create_diallers, setup_message_queues};

mod cli;
mod dialler;
mod utils;

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
    let keys = args.build_keys_map();
    let signals = args.build_signals_map();
    let mut diallers = create_diallers(&args, &signals, &keys);
    setup_message_queues(&mut diallers, &args);

    let mut tasks = Vec::new();
    for mut dialler in diallers {
        tasks.push(tokio::spawn(async move { dialler.run_sequence().await }));
    }

    for task in tasks {
        task.await?;
    }

    Ok(())
}
