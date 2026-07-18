use std::collections::{HashMap, HashSet, hash_map};

use crate::{
    wl_config::{Alignment, Relation},
    wlr_client::{errors::ClientError, output::heads::Head, point::Point},
};

#[derive(Debug, PartialEq, Hash, Eq)]
pub struct Edge<T> {
    relation: Relation,
    alignment: Alignment,
    target: T,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Node {
    pub(super) name: String,
    pub(super) position: Point,
    pub(super) width: i32,
    pub(super) height: i32,
}

impl TryFrom<&Head> for Node {
    type Error = ClientError;

    fn try_from(head: &Head) -> Result<Self, Self::Error> {
        let size = head.scaled_corrected_size()?;
        Ok(Self {
            name: head.name().into(),
            position: head.position(),
            width: size.0,
            height: size.1,
        })
    }
}

#[derive(Default)]
pub(super) struct LayoutGraph {
    graph: HashMap<String, HashSet<Edge<String>>>,
    nodes: HashMap<String, Node>,
}

impl LayoutGraph {
    pub fn ensure_node(&mut self, node: Node) {
        self.graph.entry(node.name.clone()).or_default();
        self.nodes.entry(node.name.clone()).or_insert(node);
    }

    fn get_node(&self, name: &str) -> Option<&Node> {
        self.nodes.get(name)
    }

    fn get_edges_for_node(&self, name: &str) -> Option<&HashSet<Edge<String>>> {
        if let Some(edges) = self.graph.get(name) && !edges.is_empty() {
                return Some(edges);
        }
        None
    }

    pub fn add_edge_with_target(
        &mut self,
        reference_name: &str,
        target: Node,
        relation: Relation,
        alignment: Alignment,
    ) {
        self.add_edge(reference_name, &target.name.clone(), relation, alignment);
        self.ensure_node(target);
    }

    pub fn add_edge(
        &mut self,
        reference_name: &str,
        target: &str,
        relation: Relation,
        alignment: Alignment,
    ) {
        if let Some(edges) = self.graph.get_mut(reference_name) {
            edges.insert(Edge {
                relation,
                alignment,
                target: target.to_owned(),
            });
        }
    }

    fn topological_sort(&self) -> Result<Vec<String>, String> {
        let mut indegree: HashMap<&str, u32> = HashMap::default();

        for node in self.graph.keys() {
            indegree.insert(node, 0);
        }
        for edges in self.graph.values() {
            #[allow(clippy::arithmetic_side_effects)]
            for edge in edges {
                *(indegree.get_mut(edge.target.as_str()).unwrap()) += 1;
            }
        }

        let mut zero_degree: Vec<&str> = Vec::default();
        for (&node, &degree) in &indegree {
            if degree == 0 {
                zero_degree.push(node);
            }
        }

        let mut sorted: Vec<String> = Vec::default();
        while !zero_degree.is_empty() {
            let node = zero_degree.remove(0);
            sorted.push(node.into());
            if let Some(edges) = self.graph.get(node) {
                #[allow(clippy::arithmetic_side_effects)]
                for edge in edges {
                    let target_degree = indegree.get_mut(edge.target.as_str()).unwrap();
                    *target_degree -= 1;
                    if *target_degree == 0 {
                        zero_degree.push(&edge.target);
                    }
                }
            }
        }

        if self.graph.keys().len() == sorted.len() {
            Ok(sorted)
        } else {
            Err(String::from("cyclic dependency detected"))
        }
    }

    pub fn apply(self) -> Result<Self, String> {
        let sorted_nodes = self.topological_sort()?;
        if sorted_nodes.is_empty() {
            return Ok(self);
        }
        let mut applied = LayoutGraph::default();
        let first_node = self.get_node(&sorted_nodes[0]).unwrap().to_owned();
        applied.ensure_node(first_node);
        for node_name in sorted_nodes {
            let node = applied.get_node(&node_name).unwrap().clone();
            if let Some(edges) = self.get_edges_for_node(&node_name) {
                for edge in edges {
                    let mut child = self.get_node(&edge.target).unwrap().clone();
                    let mut position = match edge.relation {
                        Relation::LeftOf => Point(
                            node.position.0.saturating_sub(child.width),
                            child.position.1,
                        ),
                        Relation::RightOf => {
                            Point(node.position.0.saturating_add(node.width), child.position.1)
                        }
                        Relation::TopOf => {
                            Point(child.width, node.position.1.saturating_sub(child.height))
                        }
                        Relation::BottomOf => {
                            Point(child.width, node.position.1.saturating_add(node.height))
                        }
                    };
                    let position = match edge.alignment {
                        Alignment::AlignBottom => {
                            let delta = node.height.saturating_sub(child.height);
                            position.1 = node.position.1.saturating_add(delta);
                            position
                        }
                        Alignment::AlignTop => {
                            position.1 = node.position.1;
                            position
                        }
                        Alignment::AlignRight => {
                            let delta = node.width.saturating_sub(child.width);
                            position.0 = node.position.0.saturating_add(delta);
                            position
                        }
                        Alignment::AlignLeft => {
                            position.0 = node.position.0;
                            position
                        }
                    };
                    child.position = position;
                    applied.add_edge_with_target(&node.name, child, edge.relation, edge.alignment);
                }
            }
        }

        Ok(applied)
    }

    /// Puts all nodes in the positive space.
    ///
    /// Returns offset applied on all nodes to move them.
    pub(crate) fn normalize_space(&mut self) -> Result<Point, String> {
        let sorted_nodes = self.topological_sort()?;
        if sorted_nodes.is_empty() {
            return Ok(Point::default());
        }
        let first_node = self.get_node(&sorted_nodes[0]).unwrap().to_owned();
        let mut offset_from_origin = first_node.position;
        for node in self.nodes.values() {
            offset_from_origin.0 = offset_from_origin.0.min(node.position.0);
            offset_from_origin.1 = offset_from_origin.1.min(node.position.1);
        }

        for node in self.nodes.values_mut() {
            node.position.0 = node.position.0.saturating_sub(offset_from_origin.0);
            node.position.1 = node.position.1.saturating_sub(offset_from_origin.1);
        };
        Ok(offset_from_origin)
    }

    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.nodes.contains_key(name)
    }
}

impl<'a> IntoIterator for &'a LayoutGraph {
    type Item = (&'a Node, &'a HashSet<Edge<String>>);

    type IntoIter = GraphIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        GraphIterator::new(self)
    }
}

pub struct GraphIterator<'a> {
    graph: &'a LayoutGraph,
    iter: hash_map::Iter<'a, String, HashSet<Edge<String>>>,
}

impl<'a> GraphIterator<'a> {
    fn new(graph: &'a LayoutGraph) -> Self {
        Self {
            graph,
            iter: graph.graph.iter(),
        }
    }
}

impl<'a> Iterator for GraphIterator<'a> {
    type Item = (&'a Node, &'a HashSet<Edge<String>>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((name, edges)) = self.iter.next() {
            let node = self.graph.nodes.get(name).unwrap();
            Some((node, edges))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        functions::position::layout_graph::{LayoutGraph, Node},
        wl_config::{Alignment, Relation},
        wlr_client::point::Point,
    };

    #[test]
    fn test_add_node() {
        let mut dag = LayoutGraph::default();
        dag.ensure_node(Node {
            name: "test".into(),
            position: Point(0, 0),
            width: 42,
            height: 42,
        });

        assert_eq!(1, dag.graph.len());
        let node = dag.nodes.get("test");
        assert!(node.is_some());

        let node = dag.graph.iter().find(|&(k, _)| k == "test");
        assert!(node.is_some());
    }

    #[test]
    fn test_add_edge() {
        let mut dag = LayoutGraph::default();
        let reference = Node {
            name: "ref".into(),
            position: Point(0, 0),
            width: 42,
            height: 42,
        };
        dag.ensure_node(reference.clone());
        dag.add_edge_with_target(
            "ref",
            Node {
                name: "target".into(),
                position: Point(1, 1),
                width: 42,
                height: 42,
            },
            Relation::LeftOf,
            Alignment::AlignBottom,
        );

        let edges = dag.graph.get("ref");
        assert!(edges.is_some());
        let edge = edges.unwrap().iter().find(|e| e.target == "target");
        assert!(edge.is_some());
    }

    #[test]
    fn test_topological_sort() {
        let mut dag = LayoutGraph::default();
        let reference = Node {
            name: "1".into(),
            position: Point(0, 0),
            width: 42,
            height: 42,
        };
        dag.ensure_node(reference.clone());
        dag.add_edge_with_target(
            "1",
            Node {
                name: "2".into(),
                position: Point(1, 1),
                width: 42,
                height: 42,
            },
            Relation::LeftOf,
            Alignment::AlignBottom,
        );
        dag.add_edge_with_target(
            "2",
            Node {
                name: "3".into(),
                position: Point(1, 1),
                width: 42,
                height: 42,
            },
            Relation::LeftOf,
            Alignment::AlignBottom,
        );

        dag.ensure_node(Node {
            name: "0".into(),
            position: Point(3, 3),
            width: 42,
            height: 42,
        });
        dag.add_edge("0", "1", Relation::LeftOf, Alignment::AlignBottom);

        let sorted = dag.topological_sort();
        assert!(sorted.is_ok());

        for (i, n) in sorted.unwrap().iter().enumerate() {
            assert_eq!(i, n.parse().unwrap());
        }
    }

    #[test]
    fn test_apply() {
        let mut dag = LayoutGraph::default();
        let reference = Node {
            name: "1".into(),
            position: Point(100, 100),
            width: 100,
            height: 100,
        };
        dag.ensure_node(reference.clone());

        dag.add_edge_with_target(
            "1",
            Node {
                name: "2".into(),
                position: Point(200, 200),
                width: 100,
                height: 100,
            },
            Relation::LeftOf,
            Alignment::AlignBottom,
        );

        dag.add_edge_with_target(
            "1",
            Node {
                name: "3".into(),
                position: Point(300, 300),
                width: 100,
                height: 100,
            },
            Relation::TopOf,
            Alignment::AlignLeft,
        );

        let applied = dag.apply().unwrap();

        assert_eq!(Point(100, 0), applied.get_node("3").unwrap().position);
        assert_eq!(Point(0, 100), applied.get_node("2").unwrap().position);
        assert_eq!(Point(100, 100), applied.get_node("1").unwrap().position);
    }
}
