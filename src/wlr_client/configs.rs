use std::collections::HashMap;

use super::wlr_head::OutputHead;
use super::wlr_mode::OutputMode;
use wayland_client::{backend::ObjectId, Proxy};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::ZwlrOutputHeadV1, zwlr_output_mode_v1::ZwlrOutputModeV1,
};

#[derive(Default)]
pub struct Configurations {
    heads: HashMap<ObjectId, OutputHead>,
    modes: HashMap<ObjectId, OutputMode>,
    serial: u32,
}

impl Configurations {
    pub fn add_head(&mut self, head: ZwlrOutputHeadV1) {
        self.heads.insert(head.id(), OutputHead::new(head));
    }

    pub fn add_mode(&mut self, mode: &ZwlrOutputModeV1) {
        self.modes.insert(mode.id(), OutputMode::new(mode));
    }

    pub fn remove_head(&mut self, id: &ObjectId) -> Option<OutputHead> {
        self.heads.remove(id)
    }

    pub fn set_serial(&mut self, serial: u32) {
        self.serial = serial;
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

    pub fn heads(&self) -> Vec<&OutputHead> {
        self.heads.values().collect()
    }

    pub(crate) fn serial(&self) -> u32 {
        self.serial
    }

}
