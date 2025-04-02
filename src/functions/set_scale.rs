use crate::{
    heads::Head,
    wlr_client::{
        self,
        config_writer::{HeadUpdateRequest, UpdateRequest},
    },
};

pub fn exec(head: &Head, scale: f64, client: &wlr_client::Client) -> Result<(), String> {
    let configs = client.configurations()?;

    let request = UpdateRequest {
        serial: configs.serial(),
        head_requests: vec![HeadUpdateRequest {
            head: head.output_head(),
            position: None,
            mode: None,
            scale: Some(scale),
        }],
    };
    client.update_configurations(&request)?;
    Ok(())
}
