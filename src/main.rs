#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::integer_division
)]

mod logging;
mod wlr_client;

use logging::setup_logging;

use log::info;

fn main() {
    setup_logging();
    info!("===Started===");
    let mut client = wlr_client::Client::new();
    let _res = client.connect();
    let configs = client.configurations();
    let heads = configs.heads();
    if heads.len() > 0 {
        println!("Found {} display(s):", heads.len());
        for head in heads {
            println!("- {}", head);
        }
    }
}
