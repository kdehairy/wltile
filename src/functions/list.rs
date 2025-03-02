use crate::heads::Heads;
use colored::{Color, Colorize};

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
        if head.enabled() {
            println!("{}:", head.name().bold());
            println!("\tMake: {} {}", head.make(), head.model().color(GRAY_COLOR));
            print!(
                "\tSize: {} x {}",
                head.mode().size().0,
                head.mode().size().1,
            );
            let scale = format!(" scale: {}", head.scale()).color(LIGHT_GRAY_COLOR);
            println!("{scale}");
            println!("\tPosition: {}", head.position());
        }
    }
}
