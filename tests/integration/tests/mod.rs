mod daemon;
mod list;
mod position;
mod set;
mod show;

fn find_output<'a>(outputs: &'a [crate::swaymsg::Output], name: &str) -> &'a crate::swaymsg::Output {
    outputs
        .iter()
        .find(|o| o.name == name)
        .unwrap_or_else(|| panic!("output '{name}' not found in compositor state"))
}
