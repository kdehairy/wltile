use std::ops::Div;

use crate::{heads::Head, wlr_client::{configs::Configurations, wlr_mode::OutputMode}};


pub(crate) fn exec(head: &Head, configs: &Configurations) {
    print_make(head);
    print_size(head);
    print_physical_size(head);
    print_refresh(head);
    print_position(head);
    print_modes(head, configs);
}

#[allow(clippy::print_stdout)]
fn print_modes(head: &Head, configs: &Configurations) {
    println!("Modes:");
    for id in head.mode_ids() {
        let mode = configs.get_mode(id).expect("Something went wrong with mode configurations");
        print_mode(mode);
    }
}

#[allow(clippy::print_stdout)]
fn print_mode(mode: &OutputMode) {
    let prefered = if mode.prefered() {
        "\t>"
    } else {
        "\t "
    };
    println!("{} {} x {} @ {} kHz", prefered, mode.size().0, mode.size().1, f64::from(mode.refresh()).div(1000_f64).round());
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
    println!(
        "Refresh Rate: {} kHz",
        f64::from(head.mode().refresh()).div(1000_f64).round()
    );
}

#[allow(clippy::print_stdout)]
fn print_make(head: &Head) {
    println!("Make: {}", head.make());
    println!("Model: {}", head.model());
}

#[allow(clippy::print_stdout)]
fn print_size(head: &Head) {
    println!(
        "Size: {} x {}",
        head.mode().size().0,
        head.mode().size().1,
    );
    println!("Scale: {}", head.scale());
}

#[allow(clippy::print_stdout)]
fn print_position(head: &Head) {
    println!("Position: {}", head.position());
}
