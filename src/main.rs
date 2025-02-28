#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::integer_division
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
use heads::Heads;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    {
        logging::setup();
    }

    let args = Cli::parse();

    let mut client = wlr_client::Client::new();
    client.connect()?;
    let configs = client.configurations()?;
    let heads = Heads::new(configs)?;

    match args.command {
        Commands::List {} => {
            functions::list::exec(&heads);
            Ok(())
        }
        Commands::Position {
            target,
            relation,
            reference,
            alignment,
        } => {
            let target_head =
                heads
                    .get(&target)
                    .cloned()
                    .ok_or("target output does not exist")?;
            let reference_head =
                heads
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
    }
}
