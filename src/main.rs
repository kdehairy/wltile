#![warn(
    clippy::all,
    clippy::pedantic,
    //clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::integer_division
)]

mod cli;
mod heads;
mod logging;
mod wlr_client;
mod functions;

use clap::Parser;
use heads::Heads;
use cli::Cli;
use cli::Commands;

use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    logging::setup();
    info!("===Started===");

    let args = Cli::parse();

    let mut client = wlr_client::Client::new();
    client.connect()?;
    let configs = client.configurations();
    let heads = Heads::new(configs)?;

    match args.command {
        Commands::List {} => functions::list::exec(&heads),
    }

    info!("===Finished===");
    Ok(())
}
