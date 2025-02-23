#![warn(
    clippy::all,
    clippy::pedantic,
    //clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::integer_division
)]

mod heads;
mod logging;
mod wlr_client;

use heads::Heads;

use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    logging::setup();
    info!("===Started===");
    let mut client = wlr_client::Client::new();
    let _res = client.connect();
    let configs = client.configurations();
    let heads = Heads::new(configs)?;
    for head in heads.heads() {
        if head.enabled() {
            println!("{head}");
        }
    }

    Ok(())
}
