mod layout_graph;

use crate::{
    functions::position::layout_graph::{LayoutGraph, Node},
    wl_config::Config,
    wlr_client::{
        self,
        output::config_writer::{HeadUpdateRequest, UpdateRequest},
        point::Point,
    },
};

pub fn exec(config: &Config, client: &wlr_client::Client) -> Result<(), String> {
    let configs = client.configurations();
    let heads = configs.heads()?;

    let mut layout_graph = LayoutGraph::default();
    for target in config.targets() {
        let reference_name = &target.position.as_ref().unwrap().reference;
        let ref_head = heads
            .find(reference_name)
            //.cloned() //Why cloning makes the wlr_head invalid argument for wayland `enable_head()`?!
            .ok_or("reference output does not exist")?;
        layout_graph.ensure_node(Node::from(ref_head));

        let relation = target.position.as_ref().unwrap().relation;
        let alignment = target.position.as_ref().unwrap().alignment;
        let target_head = heads
            .find(&target.name)
            //.cloned()
            .ok_or("target output does not exist")?;
        layout_graph.add_edge_with_target(
            reference_name,
            Node::from(target_head),
            relation,
            alignment,
        );
    }

    layout_graph = layout_graph.apply()?;
    let offset = layout_graph.normalize_space().unwrap();

    let mut head_requests: Vec<HeadUpdateRequest> = Vec::with_capacity(layout_graph.len());
    for (node, _) in &layout_graph {
        head_requests.push(HeadUpdateRequest {
            head: heads.find(&node.name).unwrap().output_head(),
            position: Some(node.position),
            mode: None,
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
            head: head.output_head(),
            position: Some(new_position),
            mode: None,
            scale: None,
            rotation: None,
        });
    }

    let request = UpdateRequest {
        serial: client.configurations().serial(),
        head_requests,
    };

    client.update_configurations(&request)?;
    Ok(())
}
