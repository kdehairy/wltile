use std::collections::HashMap;
use std::fmt::Display;

use wayland_client::backend::ObjectId;
use wayland_client::Proxy;
use wayland_client::{event_created_child, Dispatch};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::{
        Event::{
            AdaptiveSync, CurrentMode, Description, Enabled, Finished, Make, Mode, Model, Name,
            PhysicalSize, Position, Scale, SerialNumber, Transform,
        },
        ZwlrOutputHeadV1, EVT_MODE_OPCODE,
    },
    zwlr_output_manager_v1::ZwlrOutputManagerV1,
    zwlr_output_mode_v1::ZwlrOutputModeV1,
};

use super::configs::Configurations;

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
                Name { name } => head.name = name,
                Description { description } => head.description = description,
                PhysicalSize { width, height } => head.physical_size = Point(width, height),
                Mode { mode } => {
                    head.add_mode(mode);
                }
                Enabled { enabled } => head.enabled = !matches!(enabled, 0),
                CurrentMode { mode } => head.current_mode = mode.id(),
                Position { x, y } => head.position = Point(x, y),
                Transform { transform: _ } => {}
                Scale { scale: _ } => {}
                Finished => {
                    kill_me_please = true;
                }
                Make { make } => head.make = make,
                Model { model } => head.model = model,
                SerialNumber { serial_number } => head.serial_number = serial_number,
                AdaptiveSync { state: _ } => {}
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

#[derive(Default)]
struct Point(i32, i32);
impl Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

pub struct Head {
    head: ZwlrOutputHeadV1,
    name: String,
    description: String,
    physical_size: Point,
    modes: HashMap<ObjectId, ZwlrOutputModeV1>,
    enabled: bool,
    current_mode: ObjectId,
    position: Point,
    make: String,
    model: String,
    serial_number: String,
}

impl Head {
    pub fn new(head: ZwlrOutputHeadV1) -> Self {
        Self {
            head,
            name: String::default(),
            description: String::default(),
            physical_size: Point::default(),
            modes: HashMap::default(),
            enabled: bool::default(),
            current_mode: ObjectId::null(),
            position: Point::default(),
            make: String::default(),
            model: String::default(),
            serial_number: String::default(),
        }
    }

    pub fn release(&self) {
        //TODO: release modes
        self.head.release();
    }

    fn add_mode(&mut self, mode: ZwlrOutputModeV1) {
        self.modes.insert(mode.id(), mode);
    }
}

impl Display for Head {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {}: {} at position {}", self.make, self.model, self.physical_size, self.position)
    }
}
