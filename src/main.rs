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
use cli::Property;
use functions::position::TargetSetup;

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
                    let desired: usize = value
                        .trim()
                        .parse()
                        .expect("Expected integer identifier for the mode");
                    functions::set_mode::exec(&target_head, desired, &client)?;
                }
            }

            Ok(())
        }
    }
}
