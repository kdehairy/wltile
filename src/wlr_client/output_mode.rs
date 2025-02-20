use wayland_client::Dispatch;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

use super::configs::Configurations;

impl Dispatch<ZwlrOutputModeV1, ()> for Configurations {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputModeV1,
        _event: <ZwlrOutputModeV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
}
