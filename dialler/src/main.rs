use anyhow::Result;
use clap::Parser;
use std::process::ExitCode;

use crate::utils::{create_diallers, setup_message_queues};

mod cli;
mod dialler;
mod utils;

#[tokio::main]
async fn main() -> ExitCode {
    let args = cli::Args::parse();

    let _logging_guard = match common::logging::initialize("dialler") {
        Ok(guard) => guard,
        Err(error) => {
            eprintln!("failed to initialize logging: {error}");
            return ExitCode::FAILURE;
        },
    };

    match run_diallers(&args).await {
        Ok(had_signal_error) => {
            if args.fail_on_signal_error && had_signal_error {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        },
        Err(error) => {
            log::error!("{error}");
            ExitCode::FAILURE
        },
    }
}

async fn run_diallers(args: &cli::Args) -> Result<bool> {
    let keys = args.build_keys_map();
    let signals = args.build_signals_map();
    let mut diallers = create_diallers(args, &signals, &keys)?;
    setup_message_queues(&mut diallers, args);

    let mut tasks = Vec::new();
    for mut dialler in diallers {
        tasks.push(tokio::spawn(async move { dialler.run_sequence().await }));
    }

    let mut had_signal_error = false;
    for task in tasks {
        let success = task.await?;
        had_signal_error |= !success;
    }

    Ok(had_signal_error)
}
