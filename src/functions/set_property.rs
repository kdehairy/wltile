use crate::commons::TryFrom;
use wayland_client::protocol::wl_output::Transform;

use crate::{
    wl_config::Config,
    wlr_client::{
        Client,
        output::{
            config_writer::{HeadUpdateRequest, UpdateRequest},
            wlr_mode::OutputMode,
        },
    },
};

pub fn exec(config: &Config, client: &Client) -> Result<(), String> {
    let mut head_requests: Vec<HeadUpdateRequest> = Vec::with_capacity(config.targets().len());
    let configs = client.configurations();
    let heads = configs.heads()?;
    for target in config.targets() {
        let target_head = heads
            .find(&target.name)
            .ok_or("target output does not exist")?;

        let mut wlr_mode = None;
        if let Some(mode_idx) = target.mode {
            let mut modes: Vec<&OutputMode> = target_head
                .mode_ids()
                .iter()
                .map(|id| configs.get_mode(id).expect("Unexpected error"))
                .collect();
            modes.sort_by(|a, b| b.cmp(a));
            if let Some(mode) = modes.get(mode_idx) {
                wlr_mode = Some(mode.wlr_mode());
            } else {
                return Err(String::from("Invalid mode identifier"));
            }
        }

        let mut transform = None;
        if let Some(angle) = target.rotation {
            transform = Some(<Transform as TryFrom<i32>>::try_from(angle)?);
        }

        head_requests.push(HeadUpdateRequest {
            head: target_head.output_head(),
            scale: target.scale,
            position: None,
            mode: wlr_mode,
            rotation: transform,
        });
    }

    let request = UpdateRequest {
        serial: configs.serial(),
        head_requests,
    };
    client.update_configurations(&request)?;

    Ok(())
}
