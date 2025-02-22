use std::fmt::Display;

use wayland_client::backend::ObjectId;
use wayland_client::Proxy;
use wayland_client::{event_created_child, Dispatch};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::{
        Event,
        ZwlrOutputHeadV1, EVT_MODE_OPCODE,
    },
    zwlr_output_manager_v1::ZwlrOutputManagerV1,
    zwlr_output_mode_v1::ZwlrOutputModeV1,
};

use super::configs::Configurations;
use super::Point;

impl Dispatch<ZwlrOutputHeadV1, ()> for Configurations {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputHeadV1,
        event: <ZwlrOutputHeadV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let mut kill_me_please = false;
        if let Some(head) = state.get_head(&proxy.id()) {
            match event {
                Event::Name { name } => {
                    log::debug!("Head {}: name={}", head.id(), name);
                    head.name = name
                },
                Event::Description { description } => {
                    log::trace!("Head {}: description={}", head.id(), description);
                    head.description = description
                },
                Event::PhysicalSize { width, height } => head.physical_size = Point(width, height),
                Event::Mode { mode } => {
                    log::trace!("Head {}: mode={}", head.id(), mode.id());
                    head.add_mode(&mode);
                    state.add_mode(&mode);
                }
                Event::Enabled { enabled } => head.enabled = !matches!(enabled, 0),
                Event::CurrentMode { mode } => {
                    log::debug!("Head {}: current_mode={}", head.id(), mode.id());
                    head.current_mode_id = mode.id()
                },
                Event::Position { x, y } => {
                    log::debug!("Head {}: position={}", head.id(), Point(x, y));
                    head.position = Point(x, y)
                },
                Event::Finished => {
                    kill_me_please = true;
                }
                Event::Make { make } => {
                    log::trace!("Head {}: make={}", head.id(), make);
                    head.make = make
                },
                Event::Model { model } => {
                    log::trace!("Head {}: model={}", head.id(), model);
                    head.model = model
                },
                Event::SerialNumber { serial_number } => head.serial_number = serial_number,
                //AdaptiveSync { state: _ } | Transform { transform: _ } | Scale { scale: _ } => {}
                _ => {}
            }
        }

        if kill_me_please {
            let head = state.remove_head(&proxy.id());
            if let Some(head) = head {
                head.release();
            }
        }
    }
    event_created_child!(Configurations, ZwlrOutputManagerV1, [
        EVT_MODE_OPCODE => (ZwlrOutputModeV1, ()),
    ]);
}


pub struct Head {
    id: ObjectId,
    #[allow(clippy::struct_field_names)]
    wlr_head: ZwlrOutputHeadV1,
    name: String,
    description: String,
    physical_size: Point,
    mode_ids: Vec<ObjectId>,
    current_mode_id: ObjectId,
    enabled: bool,
    position: Point,
    make: String,
    model: String,
    serial_number: String,
}

impl Head {
    pub fn new(head: ZwlrOutputHeadV1) -> Self {
        Self {
            id: head.id(),
            wlr_head: head,
            name: String::default(),
            description: String::default(),
            physical_size: Point::default(),
            mode_ids: Vec::default(),
            enabled: bool::default(),
            current_mode_id: ObjectId::null(),
            position: Point::default(),
            make: String::default(),
            model: String::default(),
            serial_number: String::default(),
        }
    }

    pub fn id(&self) -> &ObjectId {
        &self.id
    }

    pub fn release(&self) {
        //TODO: release modes
        self.wlr_head.release();
    }

    fn add_mode(&mut self, mode: &ZwlrOutputModeV1) {
        self.mode_ids.push(mode.id());
    }

    pub fn current_mode_id(&self) -> &ObjectId {
        &self.current_mode_id
    }

    pub fn mode_ids(&self) -> &Vec<ObjectId> {
        &self.mode_ids
    }
}

impl Display for Head {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} => {} {}: {} at position {}",
            self.name, self.make, self.model, self.physical_size, self.position
        )
    }
}
