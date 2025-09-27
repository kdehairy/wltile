use std::time::Duration;

use tracing::{debug, error};
use wayland_client::Proxy;
use wayland_client::{Dispatch, event_created_child};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1::ZwlrOutputHeadV1,
    zwlr_output_manager_v1::{EVT_HEAD_OPCODE, Event::Done, Event::Head, ZwlrOutputManagerV1},
};

use crate::wlr_client::StateWrapper;

impl Dispatch<ZwlrOutputManagerV1, ()> for StateWrapper {
    fn event(
        wrapper: &mut Self,
        _proxy: &ZwlrOutputManagerV1,
        event: <ZwlrOutputManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            Head { head } => {
                debug!("Found head {}", head.id());
                wrapper.state.write().unwrap().add_head(head);
            }
            Done { serial } => {
                debug!("serial: {}", serial);
                let mut configurations = wrapper.state.write().unwrap();
                configurations.set_serial(serial);
                if configurations.is_dirty()
                    && let Err(err) = wrapper.update_tx.send_timeout((), Duration::from_secs(1))
                {
                    error!("Failed to send config update after attaching a head: {err}");
                } else {
                    configurations.set_dirty(false);
                }
            }
            _ => {}
        }
    }
    event_created_child!(StateWrapper, ZwlrOutputManagerV1, [
        EVT_HEAD_OPCODE => (ZwlrOutputHeadV1, ()),
    ]);
}
