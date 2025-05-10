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
    let account = args.account.parse::<u32>().ok();
    let dialler = Dialler::new(args.address, args.port, args.account)
        .with_key(args.key)
        .with_start_sequence(args.sequence.saturating_sub(1));

    let mut tasks = Vec::new();
    for i in 0..args.diallers {
        let mut _dialler = dialler.clone();
        if let Some(account) = account {
            if !args.fixed {
                _dialler.set_account((account + i as u32).to_string());
            }
        }

        let _token = args.token.clone();
        let _message = args.message.clone();

        let task = tokio::spawn(async move {
            for _ in 0..args.repeat {
                if let Err(error) = _dialler.send_message(_token.clone(), _message.clone()).await {
                    log::error!("{}", error);
                    break;
                }
            }
        });

        tasks.push(task);
    }

    for task in tasks {
        task.await?;
    }

    Ok(())
}
