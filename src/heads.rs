use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::fmt::Display;

use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_output::Transform;

use crate::wlr_client::wlr_mode::OutputMode;

use super::wlr_client::wlr_head::OutputHead;
use super::wlr_client::Point;

#[derive(Default)]
pub struct Heads<'a> {
    heads: HashMap<String, Head<'a>>,
}

impl<'a> Heads<'a> {
    pub fn insert(&mut self, name: String, head: Head<'a>) {
        self.heads.insert(name, head);
    }

    pub fn heads(&self) -> Values<'_, String, Head> {
        self.heads.values()
    }

    pub fn get(&self, name: &str) -> Option<&Head> {
        self.heads.get(name)
    }
}

#[derive(Clone)]
pub struct Head<'a> {
    pub output_head: &'a OutputHead,
    pub current_mode: &'a OutputMode,
}

impl Head<'_> {
    pub fn mode(&self) -> &OutputMode {
        self.current_mode
    }

    pub fn name(&self) -> &str {
        self.output_head.name()
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
        self.output_head
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
}

impl Display for Head<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let head = self.output_head;
        write!(
            f,
            "{} => {} {} {} @ {}",
            head.name(), head.make(), head.model(), self.current_mode.size(), head.position()
        )
    }
}
