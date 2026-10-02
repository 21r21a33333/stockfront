//! Structural checks for recipe nodes, ports, and edges.
//!
//! This module does not know about Binance, Venus, or any other provider. It
//! only checks whether the graph follows the primitive catalog. Keeping these
//! rules here makes validation deterministic and cheap enough to run whenever
//! the editor changes a connection.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use petgraph::{algo::tarjan_scc, graph::DiGraph};
use uuid::Uuid;

use super::{
    RecipeDefinition, RecipeDiagnostic, RecipeDiagnosticCode, RecipeDiagnosticLocation, RecipeEdge,
    RecipeNode, RecipePortDescriptor, RecipePortDirection, RecipePortFlow,
};

pub(super) struct CheckedEdge<'a> {
    pub edge: &'a RecipeEdge,
    pub from_port: RecipePortDescriptor,
}

pub(super) struct RecipeGraph<'a> {
    nodes: BTreeMap<Uuid, &'a RecipeNode>,
    edges: Vec<CheckedEdge<'a>>,
}

impl<'a> RecipeGraph<'a> {
    pub fn check(recipe: &'a RecipeDefinition, diagnostics: &mut Vec<RecipeDiagnostic>) -> Self {
        let nodes = unique_nodes(&recipe.nodes, diagnostics);
        let unique_edge_ids = unique_edge_ids(&recipe.edges, diagnostics);
        let mut edges = Vec::new();

        for edge in &recipe.edges {
            if !unique_edge_ids.contains(&edge.id) {
                continue;
            }
            let Some(from) = nodes.get(&edge.from.node_id) else {
                diagnostics.push(RecipeDiagnostic::error(
                    RecipeDiagnosticCode::UnknownNode,
                    RecipeDiagnosticLocation::Edge { edge_id: edge.id },
                    "the edge starts at a node that does not exist",
                ));
                continue;
            };
            let Some(to) = nodes.get(&edge.to.node_id) else {
                diagnostics.push(RecipeDiagnostic::error(
                    RecipeDiagnosticCode::UnknownNode,
                    RecipeDiagnosticLocation::Edge { edge_id: edge.id },
                    "the edge ends at a node that does not exist",
                ));
                continue;
            };
            if edge.from.node_id == edge.to.node_id {
                diagnostics.push(RecipeDiagnostic::error(
                    RecipeDiagnosticCode::SelfConnection,
                    RecipeDiagnosticLocation::Edge { edge_id: edge.id },
                    "a node cannot connect to itself",
                ));
                continue;
            }

            let Some(from_port) = find_port(from, &edge.from.port) else {
                diagnostics.push(unknown_port(edge.from.node_id, &edge.from.port));
                continue;
            };
            let Some(to_port) = find_port(to, &edge.to.port) else {
                diagnostics.push(unknown_port(edge.to.node_id, &edge.to.port));
                continue;
            };
            if from_port.direction != RecipePortDirection::Output
                || to_port.direction != RecipePortDirection::Input
            {
                diagnostics.push(RecipeDiagnostic::error(
                    RecipeDiagnosticCode::InvalidPortDirection,
                    RecipeDiagnosticLocation::Edge { edge_id: edge.id },
                    "connections must run from an output port to an input port",
                ));
                continue;
            }
            if from_port.value_type != to_port.value_type {
                diagnostics.push(RecipeDiagnostic::error(
                    RecipeDiagnosticCode::PortTypeMismatch,
                    RecipeDiagnosticLocation::Edge { edge_id: edge.id },
                    "the connected ports carry different value types",
                ));
                continue;
            }
            edges.push(CheckedEdge { edge, from_port });
        }

        let graph = Self { nodes, edges };
        graph.check_cardinality(diagnostics);
        graph.check_consumption(diagnostics);
        graph.check_cycles(diagnostics);
        graph.check_connectivity(diagnostics);
        graph
    }

    pub fn node(&self, id: Uuid) -> Option<&'a RecipeNode> {
        self.nodes.get(&id).copied()
    }

    pub fn incoming(&self, node_id: Uuid, port: &str) -> Option<&CheckedEdge<'a>> {
        self.edges
            .iter()
            .find(|edge| edge.edge.to.node_id == node_id && edge.edge.to.port == port)
    }

    /// Returns the same order for the same recipe, even when several nodes are
    /// ready at once. Stable ordering makes previews and tests predictable.
    pub fn topological_order(&self) -> Vec<Uuid> {
        let mut indegree = self
            .nodes
            .keys()
            .copied()
            .map(|id| (id, 0_usize))
            .collect::<BTreeMap<_, _>>();
        let mut outgoing = BTreeMap::<Uuid, Vec<Uuid>>::new();
        for edge in &self.edges {
            *indegree.entry(edge.edge.to.node_id).or_default() += 1;
            outgoing
                .entry(edge.edge.from.node_id)
                .or_default()
                .push(edge.edge.to.node_id);
        }
        for targets in outgoing.values_mut() {
            targets.sort_unstable();
        }

        let mut ready = indegree
            .iter()
            .filter_map(|(id, count)| (*count == 0).then_some(*id))
            .collect::<BTreeSet<_>>();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(id) = ready.pop_first() {
            order.push(id);
            for target in outgoing.get(&id).into_iter().flatten() {
                if let Some(count) = indegree.get_mut(target) {
                    *count -= 1;
                    if *count == 0 {
                        ready.insert(*target);
                    }
                }
            }
        }
        order
    }

    fn check_cardinality(&self, diagnostics: &mut Vec<RecipeDiagnostic>) {
        for (node_id, node) in &self.nodes {
            for port in node
                .descriptor()
                .ports
                .into_iter()
                .filter(|port| port.direction == RecipePortDirection::Input)
            {
                let count = self
                    .edges
                    .iter()
                    .filter(|edge| {
                        edge.edge.to.node_id == *node_id && edge.edge.to.port == port.key
                    })
                    .count();
                if count < usize::from(port.min_connections) {
                    diagnostics.push(RecipeDiagnostic::error(
                        RecipeDiagnosticCode::MissingRequiredInput,
                        RecipeDiagnosticLocation::Port {
                            node_id: *node_id,
                            port: port.key.to_owned(),
                        },
                        format!("the `{}` input needs a connection", port.key),
                    ));
                }
                if port
                    .max_connections
                    .is_some_and(|max| count > usize::from(max))
                {
                    diagnostics.push(RecipeDiagnostic::error(
                        RecipeDiagnosticCode::TooManyConnections,
                        RecipeDiagnosticLocation::Port {
                            node_id: *node_id,
                            port: port.key.to_owned(),
                        },
                        format!("the `{}` input has too many connections", port.key),
                    ));
                }
            }
        }
    }

    fn check_consumption(&self, diagnostics: &mut Vec<RecipeDiagnostic>) {
        let mut uses = HashMap::<(Uuid, &str), usize>::new();
        for edge in &self.edges {
            *uses
                .entry((edge.edge.from.node_id, edge.from_port.key))
                .or_default() += 1;
        }
        for ((node_id, port), count) in uses {
            let flow = self
                .node(node_id)
                .and_then(|node| find_port(node, port))
                .map(|descriptor| descriptor.flow);
            if flow == Some(RecipePortFlow::Consume) && count > 1 {
                diagnostics.push(RecipeDiagnostic::error(
                    RecipeDiagnosticCode::ConsumableOutputReused,
                    RecipeDiagnosticLocation::Port {
                        node_id,
                        port: port.to_owned(),
                    },
                    "a value that is spent cannot be sent to more than one action",
                ));
            }
        }
    }

    fn check_cycles(&self, diagnostics: &mut Vec<RecipeDiagnostic>) {
        let (graph, ids) = self.petgraph();
        for component in tarjan_scc(&graph) {
            if component.len() > 1 {
                for index in component {
                    diagnostics.push(RecipeDiagnostic::error(
                        RecipeDiagnosticCode::RecipeCycle,
                        RecipeDiagnosticLocation::Node {
                            node_id: ids[index.index()],
                        },
                        "this node is part of a cycle",
                    ));
                }
            }
        }
    }

    fn check_connectivity(&self, diagnostics: &mut Vec<RecipeDiagnostic>) {
        let Some(start) = self.nodes.keys().next().copied() else {
            return;
        };
        let mut adjacency = HashMap::<Uuid, Vec<Uuid>>::new();
        for edge in &self.edges {
            adjacency
                .entry(edge.edge.from.node_id)
                .or_default()
                .push(edge.edge.to.node_id);
            adjacency
                .entry(edge.edge.to.node_id)
                .or_default()
                .push(edge.edge.from.node_id);
        }
        let mut visited = HashSet::new();
        let mut pending = vec![start];
        while let Some(node_id) = pending.pop() {
            if visited.insert(node_id) {
                pending.extend(adjacency.get(&node_id).into_iter().flatten().copied());
            }
        }
        for node_id in self.nodes.keys().filter(|id| !visited.contains(id)) {
            diagnostics.push(RecipeDiagnostic::error(
                RecipeDiagnosticCode::DisconnectedNode,
                RecipeDiagnosticLocation::Node { node_id: *node_id },
                "this node is not connected to the recipe",
            ));
        }
    }

    fn petgraph(&self) -> (DiGraph<(), ()>, Vec<Uuid>) {
        let mut graph = DiGraph::new();
        let ids = self.nodes.keys().copied().collect::<Vec<_>>();
        let indexes = ids
            .iter()
            .copied()
            .map(|id| (id, graph.add_node(())))
            .collect::<HashMap<_, _>>();
        for edge in &self.edges {
            if let (Some(from), Some(to)) = (
                indexes.get(&edge.edge.from.node_id),
                indexes.get(&edge.edge.to.node_id),
            ) {
                graph.add_edge(*from, *to, ());
            }
        }
        (graph, ids)
    }
}

fn unique_nodes<'a>(
    nodes: &'a [RecipeNode],
    diagnostics: &mut Vec<RecipeDiagnostic>,
) -> BTreeMap<Uuid, &'a RecipeNode> {
    let mut unique = BTreeMap::new();
    for node in nodes {
        if unique.insert(node.id, node).is_some() {
            diagnostics.push(RecipeDiagnostic::error(
                RecipeDiagnosticCode::DuplicateNodeId,
                RecipeDiagnosticLocation::Node { node_id: node.id },
                "node IDs must be unique",
            ));
        }
    }
    unique
}

fn unique_edge_ids(edges: &[RecipeEdge], diagnostics: &mut Vec<RecipeDiagnostic>) -> HashSet<Uuid> {
    let mut unique = HashSet::new();
    let mut duplicates = HashSet::new();
    for edge in edges {
        if !unique.insert(edge.id) {
            duplicates.insert(edge.id);
            diagnostics.push(RecipeDiagnostic::error(
                RecipeDiagnosticCode::DuplicateEdgeId,
                RecipeDiagnosticLocation::Edge { edge_id: edge.id },
                "edge IDs must be unique",
            ));
        }
    }
    unique.retain(|id| !duplicates.contains(id));
    unique
}

fn find_port(node: &RecipeNode, key: &str) -> Option<RecipePortDescriptor> {
    node.descriptor()
        .ports
        .into_iter()
        .find(|port| port.key == key)
}

fn unknown_port(node_id: Uuid, port: &str) -> RecipeDiagnostic {
    RecipeDiagnostic::error(
        RecipeDiagnosticCode::UnknownPort,
        RecipeDiagnosticLocation::Port {
            node_id,
            port: port.to_owned(),
        },
        format!("`{port}` is not a port on this node"),
    )
}
