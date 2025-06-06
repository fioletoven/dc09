use anyhow::Result;
use clap::Parser;
use common::scenarios::{Scenarios, SignalConfig};
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
    let mut diallers = create_diallers(&args);
    setup_message_queues(&mut diallers, &args);

    let mut tasks = Vec::new();
    for mut _dialler in diallers.into_iter() {
        tasks.push(tokio::spawn(async move { _dialler.run_sequence().await }));
    }

    for task in tasks {
        task.await?;
    }

    Ok(())
}

fn setup_message_queues(diallers: &mut Vec<Dialler>, args: &cli::Args) {
    let default_signal = SignalConfig::new(args.token.clone(), args.message.clone(), args.repeat);
    for dialler in diallers {
        if let Some(scenarios) = &args.scenarios {
            if scenarios.scenarios.is_empty() {
                dialler.add_sequence(vec![default_signal.clone()]);
            } else {
                assign_scenarios(dialler, scenarios);
            }
        } else {
            dialler.add_sequence(vec![default_signal.clone()]);
        }
    }
}

fn assign_scenarios(dialler: &mut Dialler, scenarios: &Scenarios) {
    if let Some(scenario_ids) = scenarios.get_scenario_ids(dialler.account()) {
        for scenario_id in scenario_ids {
            if let Some(sequence) = scenarios.get_sequence(*scenario_id) {
                dialler.add_sequence(sequence.clone());
            }
        }
    } else {
        for scenario in &scenarios.scenarios {
            dialler.add_sequence(scenario.sequence.clone());
        }
    }
}

fn create_diallers(args: &cli::Args) -> Vec<Dialler> {
    let mut result = Vec::new();

    if let Some(scenarios) = &args.scenarios {
        for dialler in &scenarios.diallers {
            result.push(
                Dialler::new(args.address, args.port, dialler.name.clone(), args.udp)
                    .with_receiver_number(dialler.receiver.clone())
                    .with_line_prefix(dialler.prefix.clone())
                    .with_key(dialler.key.clone())
                    .with_start_sequence(args.sequence.saturating_sub(1)),
            );
        }
    }

    if result.is_empty() {
        let account = args.account.parse::<u32>().ok();
        let dialler = Dialler::new(args.address, args.port, args.account.clone(), args.udp)
            .with_key(args.key.clone())
            .with_start_sequence(args.sequence.saturating_sub(1));

        for i in 0..args.diallers {
            let mut dialler = dialler.clone();
            if let Some(account) = account {
                if !args.fixed {
                    dialler.set_account((account + i as u32).to_string());
                }
            }

            result.push(dialler);
        }
    }

    result
}
