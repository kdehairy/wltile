use std::fmt::Display;
use std::time::Duration;

use tracing::{debug, error, trace};
use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_output::Transform;
use wayland_client::{Dispatch, event_created_child};
use wayland_client::{Proxy, WEnum};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::{EVT_MODE_OPCODE, Event, ZwlrOutputHeadV1},
    zwlr_output_manager_v1::ZwlrOutputManagerV1,
    zwlr_output_mode_v1::ZwlrOutputModeV1,
};

use crate::commons::ToString;
use crate::wlr_client::StateWrapper;

use crate::wlr_client::point::Point;

impl Dispatch<ZwlrOutputHeadV1, ()> for StateWrapper {
    fn event(
        wrapper: &mut Self,
        proxy: &ZwlrOutputHeadV1,
        event: <ZwlrOutputHeadV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let mut kill_me_please = false;
        let mut configs = wrapper.state.write().unwrap();
        if let Some(head) = configs.get_head_mut(&proxy.id()) {
            match event {
                Event::Name { name } => {
                    debug!("Head {}: name={}", head.id().to_string(), name);
                    head.name = name;
                }
                Event::Description { description } => {
                    trace!("Head {}: description={}", head.id(), description);
                    head.description = description;
                }
                Event::PhysicalSize { width, height } => head.physical_size = Point(width, height),
                Event::Mode { mode } => {
                    trace!("Head {}: mode={}", head.id(), mode.id());
                    head.add_mode(&mode);
                    configs.add_mode(mode);
                }
                Event::Enabled { enabled } => head.enabled = !matches!(enabled, 0),
                Event::CurrentMode { mode } => {
                    debug!("Head {}: current_mode={}", head.id(), mode.id());
                    head.current_mode_id = mode.id();
                }
                Event::Position { x, y } => {
                    debug!("Head {}: position={}", head.id(), Point(x, y));
                    head.position = Point(x, y);
                }
                Event::Finished => {
                    kill_me_please = true;
                    if let Err(err) = wrapper.update_tx.send_timeout((), Duration::from_secs(1)) {
                        error!("Failed to send config update after detaching a head: {err}");
                    }
                }
                Event::Make { make } => {
                    trace!("Head {}: make={}", head.id(), make);
                    head.make = make;
                }
                Event::Model { model } => {
                    trace!("Head {}: model={}", head.id(), model);
                    head.model = model;
                }
                Event::SerialNumber { serial_number } => head.serial_number = serial_number,
                Event::Scale { scale } => {
                    trace!("Head {}: scale={}", head.id(), scale);
                    head.scale = scale;
                }
                #[allow(clippy::as_conversions)]
                Event::Transform {
                    transform: WEnum::Value(transform),
                } => {
                    trace!("Head {}: trasform={}", head.id(), transform as u8);
                    head.transform = transform;
                }
                //Event::AdaptiveSync { state: _ },
                _ => {}
            }
        }

        if kill_me_please {
            let head = configs.remove_head(&proxy.id());
            if let Some(head) = head {
                head.release();
            }
        }
    }
    event_created_child!(StateWrapper, ZwlrOutputManagerV1, [
        EVT_MODE_OPCODE => (ZwlrOutputModeV1, ()),
    ]);
}

#[derive(Clone, Debug)]
pub struct OutputHead {
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
    scale: f64,
    transform: Transform,
}

impl OutputHead {
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
            scale: f64::default(),
            transform: Transform::Normal,
        }
    }

    pub fn id(&self) -> &ObjectId {
        &self.id
    }

    pub fn release(&self) {
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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn _description(&self) -> &str {
        &self.description
    }

    pub fn physical_size(&self) -> &Point {
        &self.physical_size
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn position(&self) -> &Point {
        &self.position
    }

    pub fn make(&self) -> &str {
        &self.make
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn serial_number(&self) -> &str {
        &self.serial_number
    }

    pub fn wlr_head(&self) -> &ZwlrOutputHeadV1 {
        &self.wlr_head
    }

    pub fn scale(&self) -> f64 {
        self.scale
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }
}

impl Drop for OutputHead {
    fn drop(&mut self) {
        self.wlr_head.release();
    }
}

impl Display for OutputHead {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} => {} {}: {} at position {}",
            self.name, self.make, self.model, self.physical_size, self.position
        )
    }
}

impl ToString for Transform {
    fn to_string(&self) -> String {
        match self {
            Transform::Normal => String::from("0°"),
            Transform::_90 => String::from("-90°"),
            Transform::_180 => String::from("-180°"),
            Transform::_270 => String::from("-270°"),
            Transform::Flipped => String::from("Flipped"),
            Transform::Flipped90 => String::from("flipped then rotated 90°"),
            Transform::Flipped180 => String::from("flipped then rotated 180°"),
            Transform::Flipped270 => String::from("flipped then rotated 270°"),
            _ => String::from("?"),
        }
    }
}
