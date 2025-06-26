use tracing::trace;
use wayland_client::{
    protocol::wl_shm::{Event, Format, WlShm},
    Dispatch, WEnum,
};

use crate::wlr_client::configs::Configurations;

impl Dispatch<WlShm, ()> for Configurations {
    fn event(
        state: &mut Self,
        _proxy: &WlShm,
        event: <WlShm as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // We are guaranteed by wayland specifications that ARGB8888 and XRGB8888 must be supported
        // by the compositor.
        if let Event::Format {
            format: WEnum::Value(Format::Argb8888),
        } = event
        {
            state.set_pixel_format(Format::Argb8888);
            trace!("pixel format 'ARGB8888' is supported");
        }
    }
}
