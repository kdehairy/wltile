mod layout_graph;

use tracing::debug;

use crate::{
    functions::position::layout_graph::{LayoutGraph, Node},
    wl_config::Config,
    wlr_client::{
        self,
        errors::ClientError,
        output::config_writer::{HeadUpdateRequest, UpdateRequest},
        point::Point,
    },
};

pub fn exec(config: &Config, client: &wlr_client::Client) -> Result<(), ClientError> {
    debug!("received config: {config}");
    let configs = client.configurations_read_lock();
    let heads = configs.read().heads()?;

    let mut layout_graph = LayoutGraph::default();
    for target in config.targets() {
        if target.position.is_none() {
            continue;
        }
        let reference_name = &target.position.as_ref().unwrap().reference;
        let ref_head = heads
            .find(reference_name)
            .ok_or("reference output does not exist")?;
        layout_graph.ensure_node(Node::try_from(ref_head)?);

        let relation = target.position.as_ref().unwrap().relation;
        let alignment = target.position.as_ref().unwrap().alignment;
        let target_head = heads
            .find(&target.name)
            .ok_or("target output does not exist")?;
        layout_graph.add_edge_with_target(
            reference_name,
            Node::try_from(target_head)?,
            relation,
            alignment,
        );
    }

    layout_graph = layout_graph.apply()?;
    let offset = layout_graph.normalize_space().unwrap();

    let mut head_requests: Vec<HeadUpdateRequest> = Vec::with_capacity(layout_graph.len());
    for (node, _) in &layout_graph {
        head_requests.push(HeadUpdateRequest {
            head_id: heads.find(&node.name).unwrap().id(),
            position: Some(node.position),
            mode_id: None,
            scale: None,
            rotation: None,
        });
    }

    for head in heads.heads() {
        if layout_graph.contains(head.name()) {
            continue;
        }
        let new_position = Point(
            head.position().0.saturating_sub(offset.0),
            head.position().1.saturating_sub(offset.1),
        );
        head_requests.push(HeadUpdateRequest {
            head_id: head.id(),
            position: Some(new_position),
            mode_id: None,
            scale: None,
            rotation: None,
        });
    }

    let request = UpdateRequest {
        serial: client.configurations_read_lock().read().serial(),
        head_requests,
    };

    client.update_configurations(&request)?;
    Ok(())
}
