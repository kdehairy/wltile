use std::ops::Div;

use crate::{
    commons::ToString,
    heads::Head,
    wlr_client::{output::configs::Configurations, output::wlr_mode::OutputMode},
};

pub(crate) fn exec(head: &Head, configs: &Configurations) {
    print_name(head);
    print_serial_number(head);
    print_make(head);
    print_size(head);
    print_transform(head);
    print_physical_size(head);
    print_refresh(head);
    print_position(head);
    print_modes(head, configs);
}

#[allow(clippy::print_stdout)]
fn print_transform(head: &Head) {
    println!("Rotation: {}", head.transform().to_string());
}

#[allow(clippy::print_stdout)]
fn print_modes(head: &Head, configs: &Configurations) {
    let current_mode = head.current_mode;
    println!("Modes:");
    let mut modes: Vec<&OutputMode> = head
        .mode_ids()
        .iter()
        .map(|id| configs.get_mode(id).expect("Unexpected failure"))
        .collect();
    modes.sort_by(|a, b| b.cmp(a));
    for (i, mode) in modes.iter().enumerate() {
        print_mode(mode, i, current_mode == *mode);
    }
}

#[allow(clippy::print_stdout)]
fn print_mode(mode: &OutputMode, index: usize, current: bool) {
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
    println!(
        "Refresh Rate: {} kHz",
        f64::from(head.mode().refresh()).div(1000_f64).round()
    );
}

#[allow(clippy::print_stdout)]
fn print_name(head: &Head) {
    println!("Name: {}", head.name());
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
    println!("Size: {} x {}", head.mode().size().0, head.mode().size().1,);
    println!("Scale: {}", head.scale());
}

#[allow(clippy::print_stdout)]
fn print_position(head: &Head) {
    println!("Position: {}", head.position());
}
