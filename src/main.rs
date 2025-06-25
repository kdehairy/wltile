#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::integer_division,
    clippy::needless_lifetimes
)]

mod cli;
mod commons;
mod functions;
mod heads;
mod wlr_client;

use clap::Parser;
use cli::Cli;
use cli::Commands;
use cli::Property;
use functions::position::TargetSetup;
use tracing::level_filters::LevelFilter;
use tracing::trace;

#[cfg(debug_assertions)]
const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::DEBUG;

#[cfg(not(debug_assertions))]
const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::ERROR;

#[tracing::instrument()]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    cli::validate(&args)?;

    let log_level = match args.verbose {
        0 => DEFAULT_LOG_LEVEL,
        1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };

    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(log_level)
        .init();

    let client = wlr_client::Client::new()?;
    let configs = client.configurations();
    let heads = configs.heads()?;

    match args.command {
        Commands::List => {
            trace!("cli command: list");
            functions::list::exec(&heads);
            Ok(())
        }
        Commands::Show { output: Some(output) } => {
                trace!("cli command: show {output}");
                let head = heads.get(&output).ok_or("output does not exist")?;
                functions::show_output::exec(head, configs);
                Ok(())
        }
        Commands::Show { output: None } => {
                trace!("cli command: show");
                functions::show::exec(configs);
                Ok(())
        }
        Commands::Position {
            target,
            relation,
            reference,
            alignment,
        } => {
            trace!("cli command: position {target} {relation} {reference} {alignment}");
            let target_head = heads
                .get(&target)
                .cloned()
                .ok_or("target output does not exist")?;
            let reference_head = heads
                .get(&reference)
                .cloned()
                .ok_or("reference output does not exist")?;
            functions::position::exec(
                &TargetSetup {
                    target: target_head,
                    reference: reference_head,
                    relation,
                    alignment,
                },
                &client,
            )?;
            Ok(())
        }
        Commands::Set {
            target,
            property,
            value,
        } => {
            let target_head = heads
                .get(&target)
                .cloned()
                .ok_or("target output does not exist")?;
            match property {
                Property::Mode => {
                    if let Ok(desired) = value.trim().parse::<usize>() {
                        functions::set_mode::exec(&target_head, desired, &client)?;
                    } else {
                        Err("Expected integer identifier for the mode")?;
                    }
                }
                Property::Scale => {
                    if let Ok(desired) = value.trim().parse::<f64>() {
                        functions::set_scale::exec(&target_head, desired, &client)?;
                    } else {
                        Err("Expected number identifier for the scale")?;
                    }
                }
                Property::Rotation => {
                    if let Ok(desired) = value.trim().parse::<i32>() {
                        functions::set_rotation::exec(&target_head, desired, &client)?;
                    } else {
                        Err("Expected number identifier for the rotation")?;
                    }
                }
            }

            Ok(())
        }
    }
}
