use std::collections::HashMap;

use crate::{heads::Head, wl_config::Relation, wlr_client::point::Point};

struct Edge<T> {
    relation: Relation,
    target: T,
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct Node {
    name: String,
    position: Point,
    width: i32,
    height: i32,
}

impl From<&Head> for Node {
    fn from(head: &Head) -> Self {
        Self {
            name: head.name().into(),
            position: *head.position(),
            width: head.scaled_corrected_size().0,
            height: head.scaled_corrected_size().1,
        }
    }
}

#[derive(Default)]
struct LayoutGraph {
    graph: HashMap<String, Vec<Edge<String>>>,
    nodes: HashMap<String, Node>,
}

impl LayoutGraph {
    pub fn ensure_node(&mut self, node: Node) {
        self.graph.entry(node.name.clone()).or_default();
        self.nodes.entry(node.name.clone()).or_insert(node);
    }

    pub fn get_node(&self, name: &str) -> Option<&Node> {
        self.nodes.iter().find(|&(k, _)| k == name).map(|(_, v)| v)
    }

    pub fn add_edge_with_target(&mut self, reference_name: &str, target: Node, relation: Relation) {
        self.add_edge(reference_name, &target.name.clone(), relation);
        self.ensure_node(target);
    }

    pub fn add_edge(&mut self, reference_name: &str, target: &str, relation: Relation) {
        if let Some(edges) = self.graph.get_mut(reference_name) {
            edges.push(Edge {
                relation,
                target: target.to_owned(),
            });
        }
    }

    pub fn topological_sort(&self) -> Result<Vec<Node>, String> {
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

        let mut sorted: Vec<Node> = Vec::default();
        while !zero_degree.is_empty() {
            let node = zero_degree.remove(0);
            sorted.push(self.get_node(node).unwrap().to_owned());
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
}

#[cfg(test)]
mod tests {
    use crate::{
        functions::position::layout_graph::{LayoutGraph, Node},
        wl_config::Relation,
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
        );

        dag.ensure_node(Node {
            name: "0".into(),
            position: Point(3, 3),
            width: 42,
            height: 42,
        });
        dag.add_edge("0", "1", Relation::LeftOf);

        let sorted = dag.topological_sort();
        assert!(sorted.is_ok());

        for (i, n) in sorted.unwrap().iter().enumerate() {
            assert_eq!(i, n.name.parse().unwrap());
        }
    }
}
