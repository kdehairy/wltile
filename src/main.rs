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

use colored::{Color, Colorize};

const GRAY_COLOR: Color = Color::TrueColor { r: 88, g: 88, b: 88 };

fn main() -> Result<(), Box<dyn std::error::Error>> {
    logging::setup();
    info!("===Started===");
    let mut client = wlr_client::Client::new();
    client.connect()?;
    let configs = client.configurations();
    let heads = Heads::new(configs)?;
    for head in heads.heads() {
        if head.enabled() {
            println!("{}:", head.name().bold());
            println!("\tMake: {} {}", head.make(), head.model().color(GRAY_COLOR));
            println!("\tSize: {} x {}", head.mode().size().0, head.mode().size().1);
            println!("\tPosition: {}", head.position());
        }
    }

    Ok(())
}
