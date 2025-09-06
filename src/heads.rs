use std::collections::HashMap;
use std::fmt::Display;

use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_output::Transform;

use crate::wlr_client::output::wlr_mode::OutputMode;

use super::wlr_client::output::wlr_head::OutputHead;
use super::wlr_client::point::Point;

#[derive(Default)]
pub struct Heads {
    heads: HashMap<String, Head>,
}

impl Heads {
    pub fn insert(&mut self, name: String, head: Head) {
        self.heads.insert(name, head);
    }

    pub fn heads(&self) -> Vec<&Head> {
        self.heads.values().collect()
    }

    pub fn find(&self, expr: &str) -> Option<&Head> {
        self.heads().into_iter().find(|&head| {
            head.serial_number() == expr
                || head.name() == expr
                || head.make().to_lowercase().contains(&expr.to_lowercase())
        })
    }
}

#[derive(PartialEq, Clone)]
pub struct Head {
    pub output_head: OutputHead,
    pub current_mode: OutputMode,
}

impl Head {
    pub fn mode(&self) -> &OutputMode {
        &self.current_mode
    }

    pub fn name(&self) -> &str {
        self.output_head.name()
    }

    pub fn serial_number(&self) -> &str {
        self.output_head().serial_number()
    }

    pub fn enabled(&self) -> bool {
        self.output_head.enabled()
    }

    pub fn make(&self) -> &str {
        self.output_head.make()
    }

    pub fn model(&self) -> &str {
        self.output_head.model()
    }

    pub fn position(&self) -> &Point {
        self.output_head.position()
    }

    pub fn physical_size(&self) -> &Point {
        self.output_head.physical_size()
    }

    pub fn output_head(&self) -> &OutputHead {
        &self.output_head
    }

    pub fn mode_ids(&self) -> &Vec<ObjectId> {
        self.output_head.mode_ids()
    }

    pub fn scale(&self) -> f64 {
        self.output_head.scale()
    }

    pub fn transform(&self) -> Transform {
        self.output_head.transform()
    }

    #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
    pub fn scaled_corrected_size(&self) -> Point {
        let hight = f64::from(self.mode().size().1) / self.scale();
        let hight = hight.round() as i32;

        let width = f64::from(self.mode().size().0) / self.scale();
        let width = width.round() as i32;

        if self.is_vertical() {
            Point(hight, width)
        } else {
            Point(width, hight)
        }
    }

    fn is_vertical(&self) -> bool {
        match self.transform() {
            Transform::Normal | Transform::_180 | Transform::Flipped | Transform::Flipped180 => {
                false
            }
            Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270 => true,
            _ => panic!("Unexpected value for transform"),
        }
    }
}

impl Display for Head {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let head = &self.output_head;
        write!(
            f,
            "{} => {} {} {} @ {}",
            head.name(),
            head.make(),
            head.model(),
            self.current_mode.size(),
            head.position()
        )
    }
}
