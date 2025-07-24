use tracing::debug;
use wayland_client::Proxy;
use wayland_client::{Dispatch, event_created_child};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::ZwlrOutputHeadV1,
    zwlr_output_manager_v1::{EVT_HEAD_OPCODE, Event::Done, Event::Head, ZwlrOutputManagerV1},
};

use super::configs::Configurations;

impl Dispatch<ZwlrOutputManagerV1, ()> for Configurations {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrOutputManagerV1,
        event: <ZwlrOutputManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            Head { head } => {
                debug!("Found head {}", head.id());
                state.add_head(head);
            }
            Done { serial } => {
                debug!("serial: {}", serial);
                state.set_serial(serial);
            }
            //Finished => {},
            _ => {}
        }
    }
    event_created_child!(Configurations, ZwlrOutputManagerV1, [
        EVT_HEAD_OPCODE => (ZwlrOutputHeadV1, ()),
    ]);
}
