use std::collections::HashMap;

use crate::heads::{Head, Heads};

use super::wlr_head::OutputHead;
use super::wlr_mode::OutputMode;
use wayland_client::{backend::ObjectId, Proxy};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::ZwlrOutputHeadV1, zwlr_output_mode_v1::ZwlrOutputModeV1,
};
use wayland_client::protocol::wl_shm::Format;

#[derive(Default)]
pub struct Configurations {
    heads: HashMap<ObjectId, OutputHead>,
    modes: HashMap<ObjectId, OutputMode>,
    pixel_format: Option<Format>,
    serial: u32,
}

impl Configurations {
    pub fn heads(&self) -> Result<Heads, String> {
        let mut heads = Heads::default();
        for output_head in self.output_heads() {
            let head = Head {
                output_head,
                current_mode: {
                    if let Some(mode) = self.find_current_mode(output_head) {
                        mode
                    } else {
                        return Err(String::from("failed to find current mode"));
                    }
                },
            };
            heads.insert(output_head.name().to_string(), head);
        }
        Ok(heads)
    }

    fn find_current_mode(&self, wlr_head: &OutputHead) -> Option<&OutputMode> {
        wlr_head.mode_ids().iter()
            .find(|&id| id == wlr_head.current_mode_id())
            .map(|id| self.get_mode(id))?
    }

    pub fn add_head(&mut self, head: ZwlrOutputHeadV1) {
        self.heads.insert(head.id(), OutputHead::new(head));
    }

    pub fn add_mode(&mut self, mode: ZwlrOutputModeV1) {
        self.modes.insert(mode.id(), OutputMode::new(mode));
    }

    pub fn remove_head(&mut self, id: &ObjectId) -> Option<OutputHead> {
        self.heads.remove(id)
    }

    pub fn set_serial(&mut self, serial: u32) {
        self.serial = serial;
    }

    pub fn set_pixel_format(&mut self, format: Format) {
        self.pixel_format = Some(format);
    }

    pub fn get_head_mut(&mut self, id: &ObjectId) -> Option<&mut OutputHead> {
        self.heads.get_mut(id)
    }

    pub(crate) fn get_mode_mut(&mut self, id: &ObjectId) -> Option<&mut OutputMode> {
        self.modes.get_mut(id)
    }

    pub(crate) fn get_mode(&self, id: &ObjectId) -> Option<&OutputMode> {
        self.modes.get(id)
    }

    pub fn output_heads(&self) -> Vec<&OutputHead> {
        self.heads.values().collect()
    }

    pub(crate) fn serial(&self) -> u32 {
        self.serial
    }

    pub(crate) fn pixel_format(&self) -> Option<Format> {
        self.pixel_format
    }
}
