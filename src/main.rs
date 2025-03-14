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
mod functions;
mod heads;
mod logging;
mod wlr_client;

use clap::Parser;
use cli::Cli;
use cli::Commands;
use functions::position::TargetSetup;
use wlr_client::wlr_mode::OutputMode;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    {
        logging::setup();
    }

    let args = Cli::parse();

    let mut client = wlr_client::Client::new();
    client.connect()?;
    let configs = client.configurations()?;
    let heads = configs.heads()?;

    match args.command {
        Commands::List {} => {
            functions::list::exec(&heads);
            Ok(())
        }
        Commands::Show { output } => {
            let head = heads.get(&output).ok_or("output does not exist")?;
            functions::show::exec(head, configs);
            Ok(())
        }
        Commands::Position {
            target,
            relation,
            reference,
            alignment,
        } => {
            let target_head = heads
                .get(&target)
                .cloned()
                .ok_or("target output does not exist")?;
            let reference_head = heads
                .get(&reference)
                .cloned()
                .ok_or("reference output does not exist")?;
            Ok(functions::position::exec(
                &TargetSetup {
                    target: target_head,
                    reference: reference_head,
                    relation,
                    alignment,
                },
                &client,
            )?)
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
            let mut modes: Vec<&OutputMode> = target_head.mode_ids().iter()
                .map(|id| configs.get_mode(id).expect("Unexpected error"))
                .collect();
            modes.sort_by(|a, b| b.cmp(a));

            let desired: usize = value.trim().parse().expect("Expected integer identifier for the mode");
            if let Some(target_mode) = modes.get(desired) {
                println!("You selected {} @ {} mode", target_mode.size(), target_mode.refresh());
            } else {
                return Err("Invalid mode identifier")?;
            }

            Ok(())
        }
    }
}
