use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::fmt::Display;

use crate::wlr_client::wlr_mode::OutputMode;

use super::wlr_client::configs::Configurations;
use super::wlr_client::wlr_head::OutputHead;
use super::wlr_client::Point;

pub struct Heads<'a> {
    heads: HashMap<String, Head<'a>>,
}

impl<'a> Heads<'a> {
    pub fn new(configs: &'a Configurations) -> Result<Self, String> {
        let mut heads = Self {
            heads: HashMap::default(),
        };
        for output_head in configs.heads() {
            let head = Head {
                output_head,
                current_mode: {
                    if let Some(mode) = Self::find_current_mode(output_head, configs) {
                        mode
                    } else {
                        return Err(String::from("failed to find current mode"));
                    }
                },
            };
            heads.heads.insert(output_head.name().to_string(), head);
        }
        Ok(heads)
    }

    fn find_current_mode(wlr_head: &'a OutputHead, configs: &'a Configurations) -> Option<&'a OutputMode> {
        wlr_head.mode_ids().iter()
            .find(|&id| id == wlr_head.current_mode_id())
            .map(|id| configs.get_mode(id))?
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
    output_head: &'a OutputHead,
    current_mode: &'a OutputMode,
}

impl<'a> Head<'a> {
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

    pub fn output_head(&self) -> &OutputHead {
        self.output_head
    }

    pub fn scale(&self) -> f64 {
        self.output_head.scale()
    }
}

impl<'a> Display for Head<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let head = self.output_head;
        write!(
            f,
            "{} => {} {} {} @ {}",
            head.name(), head.make(), head.model(), self.current_mode.size(), head.position()
        )
    }
}
