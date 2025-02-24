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

use std::fmt::Display;

use clap::Parser;
use functions::position::TargetSetup;
use heads::Heads;
use cli::Cli;
use cli::Commands;

use log::info;

#[derive(Debug)]
enum ParameterError {
    InvalidArgument(String),
}
impl Display for ParameterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParameterError::InvalidArgument(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ParameterError {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    logging::setup();
    info!("===Started===");

    let args = Cli::parse();

    let mut client = wlr_client::Client::new();
    client.connect()?;
    let configs = client.configurations().unwrap();
    let heads = Heads::new(configs)?;

    match args.command {
        Commands::List {} => functions::list::exec(&heads),
        Commands::Position { target, relation, reference, alignment } => {
            let target_head = heads.get(&target).cloned()
                .ok_or(ParameterError::InvalidArgument(String::from("target output does not exist")))?;
            let reference_head = heads.get(&reference).cloned()
                .ok_or(ParameterError::InvalidArgument(String::from("reference output does not exist")))?;
            functions::position::exec(&TargetSetup{
                target: target_head,
                reference: reference_head,
                relation,
                alignment,
            }, &client);
        },
    }

    info!("===Finished===");
    Ok(())
}
