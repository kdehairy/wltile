mod layout_graph;

use tracing::{debug, info};

use crate::{
    heads::Head,
    wl_config::Config,
    wl_config::{Alignment, Relation},
    wlr_client::{
        self,
        output::config_writer::{HeadUpdateRequest, UpdateRequest},
        point::Point,
    },
};

pub struct TargetSetup {
    pub target: Head,
    pub reference: Head,
    pub relation: Relation,
    pub alignment: Alignment,
}

pub fn exec(config: &Config, client: &wlr_client::Client) -> Result<(), String> {
    let configs = client.configurations();
    let heads = configs.heads()?;
    for target in config.targets() {
        let position = target.position.as_ref().unwrap();
        let target_head = heads
            .find(&target.name)
            .cloned()
            .ok_or("target output does not exist")?;
        let reference_head = heads
            .find(&position.reference)
            .cloned()
            .ok_or("reference output does not exist")?;

        let target_setup = TargetSetup {
            target: target_head,
            reference: reference_head,
            relation: position.relation,
            alignment: position.alignment,
        };

        let head_requests: Vec<HeadUpdateRequest> = vec![
            build_target_request(&target_setup),
            build_reference_request(&target_setup),
        ];
        let request = UpdateRequest {
            serial: client.configurations().serial(),
            head_requests,
        };
        info!("position request '{}'", request);
        client.update_configurations(&request)?;
    }
    Ok(())
}

#[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
fn build_reference_request(target_setup: &TargetSetup) -> HeadUpdateRequest {
    let target_size = target_setup.target.scaled_corrected_size();
    let reference_size = target_setup.target.scaled_corrected_size();
    debug!("{} size: {target_size}", target_setup.target.name());
    debug!("{} size: {reference_size}", target_setup.reference.name());
    //FIXME: this strategy is not suitable for more than 2 outputs. we need to treat the
    // reference output as a fixed anchor.
    let mut position = match target_setup.relation {
        Relation::RightOf | Relation::BottomOf => Point(0, 0),
        Relation::LeftOf | Relation::TopOf => target_size,
    };
    let position = match target_setup.alignment {
        Alignment::AlignBottom => {
            position.1 = if reference_size.1 < target_size.1 {
                target_size.1.saturating_sub(reference_size.1).abs()
            } else {
                0
            };
            position
        }
        Alignment::AlignTop => {
            position.1 = 0;
            position
        }
        Alignment::AlignRight => {
            position.0 = if reference_size.0 < target_size.0 {
                target_size.0.saturating_sub(reference_size.0).abs()
            } else {
                0
            };
            position
        }
        Alignment::AlignLeft => {
            position.0 = 0;
            position
        }
    };

    HeadUpdateRequest {
        head: target_setup.reference.output_head(),
        position: Some(position),
        mode: None,
        scale: None,
        rotation: None,
    }
}

fn build_target_request(target_setup: &TargetSetup) -> HeadUpdateRequest {
    let target_size = target_setup.target.scaled_corrected_size();
    let reference_size = target_setup.reference.scaled_corrected_size();
    let mut position = match target_setup.relation {
        Relation::RightOf | Relation::BottomOf => reference_size,
        Relation::LeftOf | Relation::TopOf => Point(0, 0),
    };
    let position = match target_setup.alignment {
        Alignment::AlignBottom => {
            position.1 = if target_size.1 < reference_size.1 {
                target_size.1.saturating_sub(reference_size.1).abs()
            } else {
                0
            };
            position
        }
        Alignment::AlignTop => {
            position.1 = 0;
            position
        }
        Alignment::AlignRight => {
            position.0 = if target_size.0 < reference_size.0 {
                target_size.0.saturating_sub(reference_size.0).abs()
            } else {
                0
            };
            position
        }
        Alignment::AlignLeft => {
            position.0 = 0;
            position
        }
    };
    HeadUpdateRequest {
        head: target_setup.target.output_head(),
        position: Some(position),
        mode: None,
        scale: None,
        rotation: None,
    }
}
