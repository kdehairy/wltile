use tracing::{debug, info};
use wayland_client::protocol::wl_output::Transform;

use crate::{
    cli::{Alignment, Relation},
    heads::Head,
    wlr_client::{
        self,
        output::config_writer::{HeadUpdateRequest, UpdateRequest},
        point::Point,
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
    let configs = client.configurations();
    let request = UpdateRequest {
        serial: configs.serial(),
        head_requests,
    };
    info!("position request '{}'", request);
    client.update_configurations(&request)
}

fn is_vertical(head: &Head<'_>) -> bool {
    match head.transform() {
        Transform::Normal | Transform::_180 | Transform::Flipped | Transform::Flipped180 => false,
        Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270 => true,
        _ => panic!("Unexpected value for transform"),
    }
}

#[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
fn build_reference_request<'a>(target_setup: &'a TargetSetup) -> HeadUpdateRequest<'a> {
    let (target_size, reference_size) = scaled_corrected_sizes(target_setup);
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

fn build_target_request<'a>(target_setup: &'a TargetSetup) -> HeadUpdateRequest<'a> {
    let (target_size, reference_size) = scaled_corrected_sizes(target_setup);
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

#[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
fn scaled_corrected_sizes(target_setup: &TargetSetup) -> (Point, Point) {
    let target_h = f64::from(target_setup.target.mode().size().1) / target_setup.target.scale();
    let target_h = target_h.round() as i32;

    let target_w = f64::from(target_setup.target.mode().size().0) / target_setup.target.scale();
    let target_w = target_w.round() as i32;
    debug!(
        "{} size: {target_w} x {target_h}",
        target_setup.target.name()
    );

    let reference_h =
        f64::from(target_setup.reference.mode().size().1) / target_setup.reference.scale();
    let reference_h = reference_h.round() as i32;

    let reference_w =
        f64::from(target_setup.reference.mode().size().0) / target_setup.reference.scale();
    let reference_w = reference_w.round() as i32;
    debug!(
        "{} size: {reference_w} x {reference_h}",
        target_setup.reference.name()
    );

    let target = if is_vertical(&target_setup.target) {
        Point(target_h, target_w)
    } else {
        debug!("{} is vertical", target_setup.target.name());
        Point(target_w, target_h)
    };
    let reference = if is_vertical(&target_setup.reference) {
        Point(reference_h, reference_w)
    } else {
        debug!("{} is vertical", target_setup.reference.name());
        Point(reference_w, reference_h)
    };
    debug!("{} corrected size: {target}", target_setup.target.name());
    debug!(
        "{} corrected size: {reference}",
        target_setup.reference.name()
    );

    (target, reference)
}
