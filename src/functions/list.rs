use std::ops::Div;

use colored::{Color, Colorize};

use crate::wlr_client::output::heads::{Head, Heads};

const GRAY_COLOR: Color = Color::TrueColor {
    r: 88,
    g: 88,
    b: 88,
};
const LIGHT_GRAY_COLOR: Color = Color::TrueColor {
    r: 160,
    g: 160,
    b: 160,
};

#[allow(clippy::print_stdout)]
pub fn exec(heads: &Heads) {
    for head in heads.heads() {
        let suffix = if head.enabled() { "" } else { " (disabled)" };
        println!("{}{suffix}:", head.name().bold());
        print_serial_number(head);
        print_make(head);
        print_size(head);
        print_physical_size(head);
        print_refresh(head);
        print_position(head);
    }
}

#[allow(clippy::print_stdout)]
fn print_serial_number(head: &Head) {
    println!("\tSerial Number: {}", head.serial_number());
}

#[allow(clippy::print_stdout)]
fn print_physical_size(head: &Head) {
    println!(
        "\tPhysical Size: {} x {} mm",
        head.physical_size().0,
        head.physical_size().1,
    );
}

#[allow(clippy::print_stdout)]
fn print_refresh(head: &Head) {
    match head.mode() {
        Some(mode) => println!(
            "\tRefresh Rate: {} kHz",
            f64::from(mode.refresh()).div(1000_f64).round()
        ),
        None => println!("\tRefresh Rate: N/A"),
    }
}

#[allow(clippy::print_stdout)]
fn print_make(head: &Head) {
    println!("\tMake: {} {}", head.make(), head.model().color(GRAY_COLOR));
}

#[allow(clippy::print_stdout)]
fn print_size(head: &Head) {
    match head.mode() {
        Some(mode) => println!(
            "\tSize: {} x {} {}",
            mode.size().0,
            mode.size().1,
            format!("scale: {}", head.scale()).color(LIGHT_GRAY_COLOR),
        ),
        None => println!("\tSize: N/A"),
    }
}

#[allow(clippy::print_stdout)]
fn print_position(head: &Head) {
    println!("\tPosition: {}", head.position());
}
