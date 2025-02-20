use wayland_client::{event_created_child, Dispatch};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_manager_v1::ZwlrOutputManagerV1,
    zwlr_output_head_v1::{
        ZwlrOutputHeadV1,
        EVT_MODE_OPCODE,
    },
    zwlr_output_mode_v1::ZwlrOutputModeV1
};

use super::configs::Configurations;

impl Dispatch<ZwlrOutputHeadV1, ()> for Configurations {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputHeadV1,
        _event: <ZwlrOutputHeadV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    }
    event_created_child!(Configurations, ZwlrOutputManagerV1, [
        EVT_MODE_OPCODE => (ZwlrOutputModeV1, ()),
    ]);
}
