use crate::{
    cli::{Alignment, Relation},
    heads::Head,
    wlr_client::{
        self,
        config_writer::{HeadUpdateRequest, UpdateRequest},
        Point,
    },
};

pub struct TargetSetup<'a> {
    pub target: Head<'a>,
    pub reference: Head<'a>,
    pub relation: Relation,
    pub alignment: Alignment,
}

pub fn exec(target_setup: &TargetSetup, client: &wlr_client::Client) -> Result<(), String> {
    let head_requests: Vec<HeadUpdateRequest> = vec![
        build_target_request(target_setup),
        build_reference_request(target_setup),
    ];
    let configs = client.configurations()?;
    let request = UpdateRequest {
        serial: configs.serial(),
        head_requests,
    };
    log::info!("position request '{}'", request);
    client.update_configurations(&request)
}

#[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
fn build_reference_request<'a>(target_setup: &'a TargetSetup) -> HeadUpdateRequest<'a> {
    let (target_size, reference_size) = unscaled_sizes(target_setup);
    let mut position = match target_setup.relation {
        Relation::RightOf => Point(0, 0),
        Relation::LeftOf => target_size,
    };
    let position = match target_setup.alignment {
        Alignment::AlignBottom => {
            position.1 = if reference_size.1 < target_size.1 {
                i32::abs(target_size.1.saturating_sub(reference_size.1))
            } else {
                0
            };
            position
        }
        Alignment::AlignTop => {
            position.1 = 0;
            position
        }
    };

    HeadUpdateRequest {
        head: target_setup.reference.output_head(),
        position: Some(position),
    }
}

fn build_target_request<'a>(target_setup: &'a TargetSetup) -> HeadUpdateRequest<'a> {
    let (target_size, reference_size) = unscaled_sizes(target_setup);
    let mut position = match target_setup.relation {
        Relation::RightOf => reference_size,
        Relation::LeftOf => Point(0, 0),
    };
    let position = match target_setup.alignment {
        Alignment::AlignBottom => {
            position.1 = if target_size.1 < reference_size.1 {
                i32::abs(target_size.1.saturating_sub(reference_size.1))
            } else {
                0
            };
            position
        }
        Alignment::AlignTop => {
            position.1 = 0;
            position
        }
    };
    HeadUpdateRequest {
        head: target_setup.target.output_head(),
        position: Some(position),
    }
}

#[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
fn unscaled_sizes(target_setup: &TargetSetup) -> (Point, Point) {
    let target_h = f64::from(target_setup.target.mode().size().1) / target_setup.target.scale();
    let target_h = target_h.round() as i32;

    let target_w = f64::from(target_setup.target.mode().size().0) / target_setup.target.scale();
    let target_w = target_w.round() as i32;

    let reference_h = f64::from(target_setup.reference.mode().size().1) / target_setup.reference.scale();
    let reference_h = reference_h.round() as i32;

    let reference_w = f64::from(target_setup.reference.mode().size().0) / target_setup.reference.scale();
    let reference_w = reference_w.round() as i32;

    (Point(target_w, target_h), Point(reference_w, reference_h))
}

