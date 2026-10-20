use std::{
    collections::{HashMap, HashSet, hash_map},
    fmt::Display,
};

use tracing::trace;

use crate::{
    wl_config::{Alignment, Relation},
    wlr_client::{errors::ClientError, output::heads::Head, point::Point},
};

/// A directed placement constraint from a reference output to `target`.
///
/// Read as "`target` is `relation` the reference, `alignment`", e.g.
/// "`target` is `right-of` the reference, `align-top`". The reference is the
/// key under which the edge is stored in `LayoutGraph::graph`, so it is not
/// repeated here.
#[derive(Debug, PartialEq, Hash, Eq)]
pub struct Edge<T> {
    relation: Relation,
    alignment: Alignment,
    target: T,
}

/// An output's name and geometry, in layout (logical) coordinates.
///
/// `width` and `height` are the scaled, transform-corrected size of the
/// output's current mode, so they match the space the compositor positions
/// outputs in.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
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

/// A DAG of placement constraints between outputs.
///
/// Each edge points from a reference output to the output placed relative to
/// it. Solving the graph ([`apply`](Self::apply)) walks it in topological order,
/// so every output is placed only after the output it depends on.
///
/// Typical use, as in [`super::exec`]:
///
/// 1. [`ensure_node`](Self::ensure_node) the reference, then
///    [`add_edge_with_target`](Self::add_edge_with_target) the target, once per
///    configured position.
/// 2. [`apply`](Self::apply) to compute the new positions.
/// 3. [`normalize_space`](Self::normalize_space) to move the layout's top-left
///    corner to the origin.
///
/// The solver assumes the graph has a single root (an output that is not
/// positioned relative to anything). See [`apply`](Self::apply).
#[derive(Default)]
pub(super) struct LayoutGraph {
    /// Adjacency list: reference name → edges to the outputs placed relative to
    /// it. Every node has an entry here, possibly with an empty set.
    graph: HashMap<String, HashSet<Edge<String>>>,
    /// Geometry of every node, keyed by name. Always has the same keys as
    /// `graph`.
    nodes: HashMap<String, Node>,
}

impl LayoutGraph {
    /// Adds `node` to the graph with no edges if it is not already present.
    ///
    /// If a node with the same name exists it is left untouched: the first
    /// geometry inserted for a name wins.
    pub fn ensure_node(&mut self, node: Node) {
        self.graph.entry(node.name.clone()).or_default();
        self.nodes.entry(node.name.clone()).or_insert(node);
    }

    fn get_node(&self, name: &str) -> Option<&Node> {
        self.nodes.get(name)
    }

    /// Returns the outgoing edges of `name`, or `None` if it has none (or does
    /// not exist).
    fn get_edges_for_node(&self, name: &str) -> Option<&HashSet<Edge<String>>> {
        if let Some(edges) = self.graph.get(name)
            && !edges.is_empty()
        {
            return Some(edges);
        }
        None
    }

    /// Adds an edge from `reference_name` to `target` and inserts `target` as
    /// a node (via [`ensure_node`](Self::ensure_node)) if it is new.
    ///
    /// Prefer this over [`add_edge`](Self::add_edge) when the target may not
    /// be in the graph yet. The reference must already be present, otherwise
    /// the edge is dropped (but `target` is still added).
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

    /// Adds an edge from `reference_name` to the existing node `target`.
    ///
    /// Does nothing if `reference_name` is not in the graph. `target` must
    /// already be a node: an edge to an unknown target makes
    /// [`topological_sort`](Self::topological_sort) panic.
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

    /// Orders node names so every reference comes before the outputs placed
    /// relative to it (Kahn's algorithm).
    ///
    /// Roots come first, in arbitrary (`HashMap`) order; the rest follow in
    /// breadth-first order.
    ///
    /// # Errors
    ///
    /// Returns an error if the edges form a cycle.
    ///
    /// # Panics
    ///
    /// Panics if an edge targets a name that is not a node.
    fn topological_sort(&self) -> Result<Vec<String>, String> {
        // Number of incoming edges per node, i.e. how many references a node
        // still waits on.
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

        // Nodes with nothing left to wait on, used as a FIFO queue.
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
            // Emitting `node` satisfies one dependency of each of its targets.
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

        // Nodes on a cycle never reach indegree 0, so they are never emitted.
        if self.graph.keys().len() == sorted.len() {
            Ok(sorted)
        } else {
            Err(String::from("cyclic dependency detected"))
        }
    }

    /// Solves the constraints, returning a new graph with the same edges whose
    /// nodes carry their computed positions.
    ///
    /// The first node in topological order (the root) keeps its current
    /// position and anchors the layout. Every other node is then placed from
    /// its reference's *already computed* position, in two steps:
    ///
    /// 1. The relation puts the target flush against one side of the
    ///    reference (e.g. `right-of`: target's left edge = reference's right
    ///    edge).
    /// 2. The alignment sets the other axis (e.g. `align-top`: target's top
    ///    edge = reference's top edge).
    ///
    /// Positions may be negative (e.g. `left-of` a root at x = 0); use
    /// [`normalize_space`](Self::normalize_space) afterwards.
    ///
    /// A node reachable through several edges is placed by the first one
    /// processed; later edges are kept but do not move it.
    ///
    /// # Errors
    ///
    /// Returns an error if the edges form a cycle.
    ///
    /// # Panics
    ///
    /// Panics if the graph has more than one root: only the first root is
    /// seeded into the result, so a second root is not found when it is
    /// reached.
    pub fn apply(self) -> Result<Self, String> {
        let sorted_nodes = self.topological_sort()?;
        trace!("Graph Nodes Sorted: {sorted_nodes:?}");
        if sorted_nodes.is_empty() {
            return Ok(self);
        }
        // `applied` only ever holds placed nodes: the root, then each target as
        // its edge is resolved. Topological order guarantees a node is placed
        // before its own edges are processed.
        let mut applied = LayoutGraph::default();
        let first_node = self.get_node(&sorted_nodes[0]).unwrap().to_owned();
        trace!("First Node: {first_node:?}");
        applied.ensure_node(first_node);
        for node_name in sorted_nodes {
            trace!("Node name: {node_name}");
            let node = applied.get_node(&node_name).unwrap().clone();
            if let Some(edges) = self.get_edges_for_node(&node_name) {
                for edge in edges {
                    trace!("Node edge {} found", edge.relation);
                    let mut child = self.get_node(&edge.target).unwrap().clone();
                    trace!("> edge targe: {}", child.name);
                    // Step 1: the relation fixes the axis the outputs touch on.
                    // The other coordinate is a placeholder the alignment below
                    // is expected to overwrite.
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
                    // Step 2: the alignment fixes the other axis. Nothing checks
                    // that it is perpendicular to the relation (see `Alignment`);
                    // a parallel one overwrites the coordinate set in step 1.
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
    /// Translates every node so the layout's bounding box starts at `(0, 0)`.
    /// Returns the offset that was subtracted from every node, so callers can
    /// shift outputs outside the graph by the same amount.
    ///
    /// # Errors
    ///
    /// Returns an error if the edges form a cycle.
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
        }
        Ok(offset_from_origin)
    }

    /// Number of nodes in the graph.
    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether an output named `name` is a node in the graph.
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.nodes.contains_key(name)
    }
}

/// Renders the graph as a readable tree: one line per node followed by
/// indented lines for each of its outgoing edges.
/// Nodes and edges are sorted so the rendering is deterministic.
impl Display for LayoutGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "LayoutGraph {{")?;
        let mut nodes: Vec<&Node> = self.nodes.values().collect();
        nodes.sort_by(|a, b| a.name.cmp(&b.name));
        for node in nodes {
            writeln!(
                f,
                "  {} ({}x{} @{})",
                node.name, node.width, node.height, node.position
            )?;
            let mut edges: Vec<&Edge<String>> = self
                .graph
                .get(&node.name)
                .map(|edges| edges.iter().collect())
                .unwrap_or_default();
            edges.sort_by(|a, b| {
                (&a.target, a.relation.to_string(), a.alignment.to_string()).cmp(&(
                    &b.target,
                    b.relation.to_string(),
                    b.alignment.to_string(),
                ))
            });
            for edge in edges {
                writeln!(
                    f,
                    "    -{} [{}]-> {}",
                    edge.relation, edge.alignment, edge.target
                )?;
            }
        }
        write!(f, "}}")
    }
}

/// Iterates over every node with its outgoing edges, in arbitrary order.
impl<'a> IntoIterator for &'a LayoutGraph {
    type Item = (&'a Node, &'a HashSet<Edge<String>>);

    type IntoIter = GraphIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        GraphIterator::new(self)
    }
}

/// Iterator over `(node, outgoing edges)` pairs of a [`LayoutGraph`].
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
    fn test_display() {
        let mut dag = LayoutGraph::default();
        dag.ensure_node(Node {
            name: "1".into(),
            position: Point(0, 0),
            width: 100,
            height: 100,
        });
        dag.add_edge_with_target(
            "1",
            Node {
                name: "2".into(),
                position: Point(1, 1),
                width: 50,
                height: 50,
            },
            Relation::RightOf,
            Alignment::AlignTop,
        );
        dag.add_edge("2", "1", Relation::BottomOf, Alignment::AlignLeft);

        let rendered = dag.to_string();
        let expected = "\
LayoutGraph {
  1 (100x100 @(0, 0))
    -right-of [align-top]-> 2
  2 (50x50 @(1, 1))
    -bottom-of [align-left]-> 1
}";
        assert_eq!(expected, rendered);
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
