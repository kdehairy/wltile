use crate::heads::Heads;
use colored::{Color, Colorize};

const GRAY_COLOR: Color = Color::TrueColor {
    r: 88,
    g: 88,
    b: 88,
};

pub fn exec(heads: &Heads) {
    for head in heads.heads() {
        if head.enabled() {
            println!("{}:", head.name().bold());
            println!("\tMake: {} {}", head.make(), head.model().color(GRAY_COLOR));
            println!(
                "\tSize: {} x {}",
                head.mode().size().0,
                head.mode().size().1
            );
            println!("\tPosition: {}", head.position());
        }
    }
}
