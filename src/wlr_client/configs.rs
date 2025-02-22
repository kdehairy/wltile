use std::collections::{hash_map::Values, HashMap};

use super::wlr_head::Head;
use super::wlr_mode::Mode;
use wayland_client::{backend::ObjectId, Proxy};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::ZwlrOutputHeadV1, zwlr_output_mode_v1::ZwlrOutputModeV1,
};

#[derive(Default)]
pub struct Configurations {
    heads: HashMap<ObjectId, Head>,
    modes: HashMap<ObjectId, Mode>,
    serial: u32,
}

impl Configurations {
    pub fn add_head(&mut self, head: ZwlrOutputHeadV1) {
        self.heads.insert(head.id(), Head::new(head));
    }

    pub fn add_mode(&mut self, mode: &ZwlrOutputModeV1) {
        self.modes.insert(mode.id(), Mode::new(mode));
    }

    pub fn remove_head(&mut self, id: &ObjectId) -> Option<Head> {
        self.heads.remove(id)
    }

    pub fn set_serial(&mut self, serial: u32) {
        self.serial = serial;
    }

    pub fn get_head(&mut self, id: &ObjectId) -> Option<&mut Head> {
        self.heads.get_mut(id)
    }

    pub(crate) fn get_mode_mut(&mut self, id: &ObjectId) -> Option<&mut Mode> {
        self.modes.get_mut(id)
    }

    pub(crate) fn get_mode(&self, id: &ObjectId) -> Option<&Mode> {
        self.modes.get(id)
    }

    pub fn heads(&self) -> Values<'_, ObjectId, Head> {
        self.heads.values()
    }

}
