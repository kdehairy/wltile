use std::fmt::Display;

use super::wlr_client::configs::Configurations;
use super::wlr_client::wlr_head::OutputHead;
use super::wlr_client::Point;

pub struct Heads<'a> {
    heads: Vec<Head<'a>>,
}

impl<'a> Heads<'a> {
    pub fn new(configs: &'a Configurations) -> Result<Self, String> {
        let mut heads = Self {
            heads: Vec::default(),
        };
        for wlr_head in configs.heads() {
            let head = Head {
                name: wlr_head.name(),
                _description: wlr_head.description(),
                _physical_size: wlr_head.physical_size(),
                enabled: wlr_head.enabled(),
                position: wlr_head.position(),
                make: wlr_head.make(),
                model: wlr_head.model(),
                _serial_number: wlr_head.serial_number(),
                mode: {
                    if let Some(mode) = Self::find_current_mode(wlr_head, configs) {
                        mode
                    } else {
                        return Err(String::from("failed to find current mode"));
                    }
                },
            };
            heads.heads.push(head);
        }
        Ok(heads)
    }

    fn find_current_mode(wlr_head: &OutputHead, configs: &'a Configurations) -> Option<Mode<'a>> {
        for id in wlr_head.mode_ids() {
            if let Some(mode) = configs.get_mode(id) {
                if mode.id() == wlr_head.current_mode_id() {
                    return Some(Mode {
                        size: mode.size(),
                        _refresh: mode.refresh(),
                        _prefered: mode.prefered(),
                    });
                }
                return None;
            }
        }
        None
    }

    pub fn heads(&self) -> &[Head<'a>] {
        &self.heads
    }
}

pub struct Mode<'a> {
    size: &'a Point,
    _refresh: i32,
    _prefered: bool,
}

impl<'a> Mode<'a> {
    pub fn size(&self) -> &Point {
        self.size
    }
}

pub struct Head<'a> {
    name: &'a str,
    _description: &'a str,
    _physical_size: &'a Point,
    mode: Mode<'a>,
    enabled: bool,
    position: &'a Point,
    make: &'a str,
    model: &'a str,
    _serial_number: &'a str,
}

impl<'a> Head<'a> {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn position(&self) -> &Point {
        self.position
    }

    pub fn make(&self) -> &str {
        self.make
    }

    pub fn model(&self) -> &str {
        self.model
    }

    pub fn mode(&self) -> &Mode<'a> {
        &self.mode
    }
}

impl<'a> Display for Head<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} => {} {} {} @ {}",
            self.name, self.make, self.model, self.mode.size, self.position
        )
    }
}
