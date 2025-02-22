use std::fmt::Display;

use wayland_client::backend::ObjectId;
use wayland_client::Dispatch;
use wayland_client::Proxy;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::{
    Event, ZwlrOutputModeV1,
};

use super::configs::Configurations;
use super::Point;

impl Dispatch<ZwlrOutputModeV1, ()> for Configurations {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputModeV1,
        event: <ZwlrOutputModeV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let Some(mode) = state.get_mode_mut(&proxy.id()) {
            match event {
                Event::Size { width, height } => {
                    log::debug!("Mode {}: size={}", mode.id(), Point(width, height));
                    mode.size = Point(width, height);
                },
                Event::Refresh { refresh } => mode.refresh = refresh,
                Event::Preferred => mode.prefered = true,
                //Event::Finished => {},
                _ => {}
            }
        }
    }
}

pub struct Mode {
    id: ObjectId,
    //wlr_mode: ZwlrOutputModeV1,
    size: Point,
    refresh: i32,
    prefered: bool,
}

impl Mode {
    pub(crate) fn new(mode: &ZwlrOutputModeV1) -> Self {
        Self {
            id: mode.id(),
            //wlr_mode: mode,
            size: Point::default(),
            refresh: 0,
            prefered: bool::default(),
        }
    }

    pub(crate) fn id(&self) -> &ObjectId {
        &self.id
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} @{} [prefered: {}]", self.size, self.refresh, self.prefered)
    }
}
