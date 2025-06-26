use wayland_client::{protocol::wl_compositor::WlCompositor, Dispatch};

use crate::wlr_client::configs::Configurations;

impl Dispatch<WlCompositor, ()> for Configurations {
    fn event(
        _state: &mut Self,
        _proxy: &WlCompositor,
        _event: <WlCompositor as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}
