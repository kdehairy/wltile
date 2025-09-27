use std::fmt::Display;

use tracing::trace;
use wayland_client::Dispatch;
use wayland_client::Proxy;
use wayland_client::backend::ObjectId;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::{
    Event, ZwlrOutputModeV1,
};

use crate::wlr_client::StateWrapper;
use crate::wlr_client::point::Point;

impl Dispatch<ZwlrOutputModeV1, ()> for StateWrapper {
    fn event(
        wrapper: &mut Self,
        proxy: &ZwlrOutputModeV1,
        event: <ZwlrOutputModeV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        let mut kill_me_please = false;
        let mut configs = wrapper.state.write().unwrap();
        if let Some(mode) = configs.get_mode_mut(&proxy.id()) {
            match event {
                Event::Size { width, height } => {
                    trace!("Mode {}: size={}", mode.wl_id(), Point(width, height));
                    mode.size = Point(width, height);
                }
                Event::Refresh { refresh } => mode.refresh = refresh,
                Event::Preferred => mode.prefered = true,
                Event::Finished => {
                    trace!("receieved Finish event for {}", mode.wl_id());
                    kill_me_please = true;
                }
                _ => {}
            }
        }

        if kill_me_please {
            let mode = configs.remove_mode(&proxy.id());
            if let Some(mode) = mode {
                mode.wlr_mode().release();
            }
        }
    }
}

#[derive(Eq, Clone)]
pub struct OutputMode {
    wl_id: ObjectId,
    wlr_mode: ZwlrOutputModeV1,
    size: Point,
    refresh: i32,
    prefered: bool,
}

impl OutputMode {
    pub fn new(mode: ZwlrOutputModeV1) -> Self {
        Self {
            wl_id: mode.id(),
            wlr_mode: mode,
            size: Point::default(),
            refresh: 0,
            prefered: bool::default(),
        }
    }

    pub fn wl_id(&self) -> &ObjectId {
        &self.wl_id
    }

    pub fn size(&self) -> Point {
        self.size
    }

    pub fn refresh(&self) -> i32 {
        self.refresh
    }

    pub fn prefered(&self) -> bool {
        self.prefered
    }

    pub fn wlr_mode(&self) -> &ZwlrOutputModeV1 {
        &self.wlr_mode
    }
}

impl Drop for OutputMode {
    fn drop(&mut self) {
        self.wlr_mode.release();
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
        Some(self.cmp(other))
    }
}
