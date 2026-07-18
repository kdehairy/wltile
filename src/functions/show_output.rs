use std::ops::Div;

use crate::{
    commons::ToString,
    wlr_client::output::heads::{Head, Mode},
};

pub(crate) fn exec(head: &Head) {
    print_name(head);
    print_serial_number(head);
    print_make(head);
    print_size(head);
    print_transform(head);
    print_physical_size(head);
    print_refresh(head);
    print_position(head);
    print_modes(head);
}

#[allow(clippy::print_stdout)]
fn print_transform(head: &Head) {
    println!("Rotation: {}", head.transform().to_string());
}

#[allow(clippy::print_stdout)]
fn print_modes(head: &Head) {
    let current_mode = head.mode();
    println!("Modes:");
    let mut sorted = head.modes().clone();
    sorted.sort_by(|a, b| b.cmp(a));
    for (i, mode) in sorted.iter().enumerate() {
        print_mode(mode, i, current_mode == Some(mode));
    }
}

#[allow(clippy::print_stdout)]
fn print_mode(mode: &Mode, index: usize, current: bool) {
    let current = if current { "\t>" } else { "\t " };
    let prefered = if mode.prefered() { "(*)" } else { "" };
    println!(
        "{} {}. {} x {} @ {} kHz {}",
        current,
        index,
        mode.size().0,
        mode.size().1,
        f64::from(mode.refresh()).div(1000_f64).round(),
        prefered
    );
}

#[allow(clippy::print_stdout)]
fn print_physical_size(head: &Head) {
    println!(
        "Physical Size: {} x {} mm",
        head.physical_size().0,
        head.physical_size().1,
    );
}

#[allow(clippy::print_stdout)]
fn print_refresh(head: &Head) {
    match head.mode() {
        Some(mode) => println!("Refresh Rate: {} kHz", f64::from(mode.refresh()).div(1000_f64).round()),
        None => println!("Refresh Rate: N/A"),
    }
}

#[allow(clippy::print_stdout)]
fn print_name(head: &Head) {
    let suffix = if head.enabled() { "" } else { " (disabled)" };
    println!("Name: {}{suffix}", head.name());
}

#[allow(clippy::print_stdout)]
fn print_serial_number(head: &Head) {
    println!("Serial Number: {}", head.serial_number());
}

#[allow(clippy::print_stdout)]
fn print_make(head: &Head) {
    println!("Make: {}", head.make());
    println!("Model: {}", head.model());
}

#[allow(clippy::print_stdout)]
fn print_size(head: &Head) {
    match head.mode() {
        Some(mode) => println!("Size: {} x {}", mode.size().0, mode.size().1),
        None => println!("Size: N/A"),
    }
    println!("Scale: {}", head.scale());
}

#[allow(clippy::print_stdout)]
fn print_position(head: &Head) {
    println!("Position: {}", head.position());
}
