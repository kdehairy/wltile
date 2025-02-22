use std::collections::{hash_map::Values, HashMap};

use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use super::output_head::Head;
use wayland_client::{backend::ObjectId, Proxy};

#[derive(Default)]
pub struct Configurations {
    heads: HashMap<ObjectId, Head>,
    serial: u32,
}

impl Configurations {
    pub(crate) fn add_head(&mut self, head: ZwlrOutputHeadV1) {
        self.heads.insert(head.id(), Head::new(head));
    }

    pub fn remove_head(&mut self, head_id: &ObjectId) -> Option<Head> {
        self.heads.remove(head_id)
    }

    pub fn set_serial(&mut self, serial: u32) {
        self.serial = serial;
    }

    pub fn get_head(&mut self, id: &ObjectId) -> Option<&mut Head> {
        self.heads.get_mut(id)
    }

    pub fn heads(&self) -> Values<'_, ObjectId, Head>{
        self.heads.values()
    }
}

