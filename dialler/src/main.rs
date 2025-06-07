use anyhow::Result;
use clap::Parser;
use common::scenarios::Scenarios;
use dialler::Dialler;
use std::sync::Arc;

use crate::cli::SharedSignalsMap;

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
    let signals = args.build_signals_map();
    let mut diallers = build_diallers(&args, signals);
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
    for dialler in diallers {
        if let Some(scenarios) = &args.scenarios {
            if scenarios.scenarios.is_empty() {
                dialler.add_default_signal();
            } else {
                assign_scenarios(dialler, scenarios);
            }
        } else {
            dialler.add_default_signal();
        }
    }
}

fn assign_scenarios(dialler: &mut Dialler, scenarios: &Scenarios) {
    if let Some(scenario_ids) = scenarios.get_scenario_ids(dialler.account()) {
        for scenario_id in scenario_ids {
            if let Some(sequence) = scenarios.get_sequence(*scenario_id) {
                let id = *scenario_id + 1;
                dialler
                    .queue()
                    .extend(sequence.iter().enumerate().map(|(i, _)| (id, i as u16)));
            }
        }
    } else {
        for scenario in &scenarios.scenarios {
            let id = scenario.id + 1;
            dialler
                .queue()
                .extend(scenario.sequence.iter().enumerate().map(|(i, _)| (id, i as u16)));
        }
    }
}

fn build_diallers(args: &cli::Args, signals: SharedSignalsMap) -> Vec<Dialler> {
    let mut result = Vec::new();

    if let Some(scenarios) = &args.scenarios {
        for dialler in &scenarios.diallers {
            result.push(
                Dialler::new(args.address, args.port, dialler.name.clone(), Arc::clone(&signals), args.udp)
                    .with_receiver_number(dialler.receiver.clone())
                    .with_line_prefix(dialler.prefix.clone())
                    .with_key(dialler.key.clone())
                    .with_start_sequence(args.sequence.saturating_sub(1)),
            );
        }
    }

    if result.is_empty() {
        let account = args.account.parse::<u32>().ok();
        let dialler = Dialler::new(args.address, args.port, args.account.clone(), Arc::clone(&signals), args.udp)
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
