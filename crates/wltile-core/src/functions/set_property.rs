use crate::{commons::TryFrom, wlr_client::errors::ClientError};
use wayland_client::protocol::wl_output::Transform;

use crate::{
    wl_config::Config,
    wlr_client::{
        Client,
        output::config_writer::{HeadUpdateRequest, UpdateRequest},
    },
};

/// Applies every non-positional property in `config` — mode, scale, rotation
/// and enablement — to the outputs it names.
///
/// # Errors
///
/// Returns a [`ClientError`] if a target names an output that is not connected,
/// if a mode index is out of range for its output, if a rotation is not a
/// multiple of 90 degrees, or if the compositor rejects the configuration.
pub fn exec(config: &Config, client: &Client) -> Result<(), ClientError> {
    let mut head_requests: Vec<HeadUpdateRequest> = Vec::with_capacity(config.targets().len());
    let configs = client.configurations_read_lock();
    let heads = configs.read().heads()?;
    for target in config.targets() {
        let target_head = heads
            .find(&target.name)
            .ok_or("target output does not exist")?;

        let mut mode_id = None;
        if let Some(mode_idx) = target.mode {
            let mut sorted = target_head.modes().clone();
            sorted.sort_by(|a, b| b.cmp(a));
            if let Some(mode) = sorted.get(mode_idx) {
                mode_id = Some(mode.id());
            } else {
                Err(String::from("Invalid mode identifier"))?;
            }
        }

        let mut transform = None;
        if let Some(angle) = target.rotation {
            transform = Some(<Transform as TryFrom<i32>>::try_from(angle)?);
        }

        head_requests.push(HeadUpdateRequest {
            head_id: target_head.id(),
            scale: target.scale,
            position: None,
            mode_id,
            rotation: transform,
            enabled: target.active,
        });
    }

    let request = UpdateRequest {
        serial: configs.read().serial(),
        head_requests,
    };
    client.update_configurations(&request)?;

    Ok(())
}
