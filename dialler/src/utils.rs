use common::{
    scenarios::{DiallerConfig, Scenarios},
    utils::{SharedKeysMap, get_account_name},
};
use std::{net::IpAddr, sync::Arc};

use crate::{
    cli::{Args, SharedSignalsMap},
    dialler::Dialler,
};

/// Creates all diallers from the scenarios file and command line parameters.
pub fn create_diallers(args: &Args, signals: &SharedSignalsMap, keys: &SharedKeysMap) -> Vec<Dialler> {
    let mut result = Vec::new();

    if let Some(scenarios) = &args.scenarios {
        for (index, dialler) in scenarios.diallers.iter().enumerate() {
            result.extend(build_diallers(
                args.address,
                args.port,
                dialler,
                signals,
                keys,
                (index + 1) as u16,
                args.fixed,
            ));
        }
    }

    if result.is_empty() {
        let dialler = DiallerConfig::new(args.account.clone(), args.sequence, args.udp, args.diallers);
        result.extend(build_diallers(
            args.address,
            args.port,
            &dialler,
            signals,
            keys,
            0,
            args.fixed,
        ));
    }

    result
}

/// Assigns messages from the scenarios to the particular dialler queues.
pub fn setup_message_queues(diallers: &mut Vec<Dialler>, args: &Args) {
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

fn build_diallers(
    address: IpAddr,
    port: u16,
    config: &DiallerConfig,
    signals: &SharedSignalsMap,
    keys: &SharedKeysMap,
    index: u16,
    fixed: bool,
) -> Vec<Dialler> {
    let mut result = Vec::with_capacity(config.count.max(1).into());
    let account = config.name.parse::<u32>().ok();

    for i in 0..config.count.max(1) {
        let account = get_account_name(i, account, &config.name, fixed);

        result.push(
            Dialler::new(address, port, account, Arc::clone(signals), config.udp)
                .with_receiver_number(config.receiver.clone())
                .with_line_prefix(config.prefix.clone())
                .with_key(Arc::clone(keys), index)
                .with_start_sequence(config.sequence.saturating_sub(1)),
        );
    }

    result
}
