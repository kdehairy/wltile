#![warn(
    clippy::all,
    clippy::pedantic,
    //clippy::print_stdout,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::integer_division
)]

mod logging;
mod wlr_client;

use log::info;

fn main() {
    logging::setup();
    info!("===Started===");
    let mut client = wlr_client::Client::new();
    let _res = client.connect();
    let configs = client.configurations();
    let heads = configs.heads();
    if heads.len() > 0 {
        println!("Found {} display(s):", heads.len());
        for head in heads {
            println!("- {head}");
            for id in head.mode_ids() {
                if let Some(mode) = configs.get_mode(id) {
                    if mode.id() == head.current_mode_id() {
                        println!("  * {mode}");
                    }
                }
            }
        }
    }
}
