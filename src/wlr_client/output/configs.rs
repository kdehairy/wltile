use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};

use crate::wlr_client::{
    errors::ClientError,
    output::heads::{Head, Heads, Mode},
};

use super::wlr_head::OutputHead;
use super::wlr_mode::OutputMode;
use tracing::{debug, warn};
use wayland_client::Proxy;
use wayland_client::backend::ObjectId;
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::ZwlrOutputHeadV1, zwlr_output_mode_v1::ZwlrOutputModeV1,
};

#[derive(Default)]
pub struct Configurations {
    latest_id: AtomicU32,
    heads: HashMap<u32, OutputHead>,
    modes: HashMap<u32, OutputMode>,
    serial: u32,
    dirty: AtomicBool,
}

impl Configurations {
    pub fn heads(&self) -> Result<Heads, ClientError> {
        let mut heads = Heads::default();
        for output_head in self.heads.values() {
            // A disabled head is never sent a `current_mode` event, so it may have none.
            // That's expected — `Head` represents it with `mode() == None`.
            let current_mode = self
                .to_mode_idx(output_head.current_mode_id())
                .and_then(|idx| self.modes.get(&idx));
            let head = Head::new(output_head, current_mode, self)?;
            heads.insert(head.name().to_string(), head);
        }
        Ok(heads)
    }

    pub fn add_head(&mut self, head: ZwlrOutputHeadV1) -> u32 {
        let id = self.latest_id.fetch_add(1, Ordering::Relaxed);
        debug!("adding head {} with id {id}", head.id());
        self.heads.insert(id, OutputHead::new(head));
        self.set_dirty(true);
        id
    }

    pub fn add_mode(&mut self, mode: ZwlrOutputModeV1) -> u32 {
        let id = self.latest_id.fetch_add(1, Ordering::Relaxed);
        debug!("adding mode {} with id {id}", mode.id());
        self.modes.insert(id, OutputMode::new(mode));
        id
    }

    pub(super) fn to_head_idx(&self, object_id: &ObjectId) -> Option<u32> {
        self.heads
            .iter()
            .filter(|&(_, val)| object_id == &val.id())
            .map(|(&id, _)| id)
            .last()
    }

    pub(super) fn to_mode_idx(&self, object_id: &ObjectId) -> Option<u32> {
        self.modes
            .iter()
            .filter(|&(_, val)| object_id == val.wl_id())
            .map(|(&id, _)| id)
            .last()
    }

    pub(super) fn remove_head(&mut self, object_id: &ObjectId) -> Option<OutputHead> {
        if let Some(id) = self.to_head_idx(object_id) {
            self.heads.remove(&id)
        } else {
            None
        }
    }

    pub(super) fn remove_mode(&mut self, object_id: &ObjectId) -> Option<OutputMode> {
        if let Some(id) = self.to_mode_idx(object_id) {
            self.modes.remove(&id)
        } else {
            warn!("No Idx mapping for the mode {object_id}");
            None
        }
    }

    pub(super) fn set_serial(&mut self, serial: u32) {
        self.serial = serial;
    }

    pub(super) fn get_head_mut(&mut self, object_id: &ObjectId) -> Option<&mut OutputHead> {
        if let Some(id) = self.to_head_idx(object_id) {
            self.heads.get_mut(&id)
        } else {
            None
        }
    }

    pub(super) fn get_mode_mut(&mut self, object_id: &ObjectId) -> Option<&mut OutputMode> {
        if let Some(id) = self.to_mode_idx(object_id) {
            self.modes.get_mut(&id)
        } else {
            None
        }
    }

    pub(crate) fn get_mode(&self, id: u32) -> Option<&OutputMode> {
        self.modes.get(&id)
    }

    pub(crate) fn get_head(&self, id: u32) -> Option<&OutputHead> {
        self.heads.get(&id)
    }

    pub fn output_heads(&self) -> Vec<&OutputHead> {
        self.heads.values().collect()
    }

    pub(crate) fn serial(&self) -> u32 {
        self.serial
    }

    pub(super) fn get_head_modes(&self, head_id: u32) -> Result<Vec<Mode>, ClientError> {
        if let Some(head) = self.heads.get(&head_id) {
            Ok(head
                .mode_ids()
                .iter()
                .map(|id| {
                    let id = self.to_mode_idx(id).expect("No mode id");
                    let mode = self.get_mode(id).expect("no mode");
                    Mode::new(id, mode)
                })
                .collect())
        } else {
            Err(ClientError::General {
                msg: "no modes found for this head".to_string(),
            })
        }
    }

    pub(super) fn set_dirty(&mut self, value: bool) {
        self.dirty.store(value, Ordering::Relaxed);
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Relaxed)
    }
}
