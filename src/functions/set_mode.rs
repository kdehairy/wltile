use crate::{
    heads::Head,
    wlr_client::{
        self,
        config_writer::{HeadUpdateRequest, UpdateRequest},
        wlr_mode::OutputMode,
    },
};

pub fn exec(head: &Head, mode_idx: usize, client: &wlr_client::Client) -> Result<(), String> {
    let configs = client.configurations()?;
    let mut modes: Vec<&OutputMode> = head
        .mode_ids()
        .iter()
        .map(|id| configs.get_mode(id).expect("Unexpected error"))
        .collect();
    modes.sort_by(|a, b| b.cmp(a));

    if let Some(target_mode) = modes.get(mode_idx) {
        let request = UpdateRequest {
            serial: configs.serial(),
            head_requests: vec![HeadUpdateRequest {
                head: head.output_head(),
                position: None,
                mode: Some(target_mode.wl_mode()),
                scale: None,
                rotation: None,
            }],
        };
        client.update_configurations(&request)?;
        Ok(())
    } else {
        Err(String::from("Invalid mode identifier"))
    }
}
