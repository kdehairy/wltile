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
                    log::debug!("Mode {}: size={}", mode.wl_id(), Point(width, height));
                    mode.size = Point(width, height);
                }
                Event::Refresh { refresh } => mode.refresh = refresh,
                Event::Preferred => mode.prefered = true,
                //Event::Finished => {},
                _ => {}
            }
        }
    }
}

#[derive(Eq)]
pub struct OutputMode {
    wl_id: ObjectId,
    //wlr_mode: ZwlrOutputModeV1,
    size: Point,
    refresh: i32,
    prefered: bool,
}

impl OutputMode {
    pub fn new(mode: &ZwlrOutputModeV1) -> Self {
        Self {
            wl_id: mode.id(),
            //wlr_mode: mode,
            size: Point::default(),
            refresh: 0,
            prefered: bool::default(),
        }
    }

    pub fn wl_id(&self) -> &ObjectId {
        &self.wl_id
    }

    pub fn size(&self) -> &Point {
        &self.size
    }

    pub fn refresh(&self) -> i32 {
        self.refresh
    }

    pub fn prefered(&self) -> bool {
        self.prefered
    }
}

impl Display for OutputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} @{} [prefered: {}]",
            self.size, self.refresh, self.prefered
        )
    }
}

impl PartialEq for OutputMode {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.refresh == other.refresh
    }
}

impl Ord for OutputMode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.size.cmp(&other.size) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.refresh.cmp(&other.refresh)
    }
}

impl PartialOrd for OutputMode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.size.partial_cmp(&other.size) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.refresh.partial_cmp(&other.refresh)
    }
}
