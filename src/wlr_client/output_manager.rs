use wayland_client::{event_created_child, Dispatch};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_manager_v1::{
        Event::Done, Event::Finished, Event::Head, 
        ZwlrOutputManagerV1,
        EVT_HEAD_OPCODE,
    },
    zwlr_output_head_v1::ZwlrOutputHeadV1,
};

use super::configs::Configurations;

impl Dispatch<ZwlrOutputManagerV1, ()> for Configurations {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputManagerV1,
        _event: <ZwlrOutputManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match _event {
            Head { head } => {
                println!("{:#?}", head);
            }
            Done { serial } => {
                println!("serial: {}", serial);
            },
            Finished => {},
            _ => {},
        }
    }
    event_created_child!(Configurations, ZwlrOutputManagerV1, [
        EVT_HEAD_OPCODE => (ZwlrOutputHeadV1, ()),
    ]);
}
