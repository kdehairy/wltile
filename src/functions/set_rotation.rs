use wayland_client::protocol::wl_output::Transform;

use crate::{
    heads::Head,
    commons::TryFrom,
    wlr_client::{
        self,
        output::config_writer::{HeadUpdateRequest, UpdateRequest},
    },
};

pub fn exec(head: &Head, angle: i32, client: &wlr_client::Client) -> Result<(), String> {
    let configs = client.configurations();

    let transform = <Transform as TryFrom<i32>>::try_from(angle)?;

    let request = UpdateRequest {
        serial: configs.serial(),
        head_requests: vec![HeadUpdateRequest {
            head: head.output_head(),
            position: None,
            mode: None,
            scale: None,
            rotation: Some(transform),
        }],
    };
    client.update_configurations(&request)?;
    Ok(())
}
