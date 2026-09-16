// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-143 finite-value construction from a containment graph.
//!
//! A [`ValueGraph`] names constructor nodes whose slots may contain other
//! nodes. Building a root admits nodes bottom-up: a node reached twice becomes
//! one shared immutable value (a DAG), and a containment back-edge refuses at
//! the node that closes the cycle. Sharing never creates object identity.

use std::collections::{BTreeMap, BTreeSet};

use super::composite::{
    Component, ConstructionCause, ConstructionRefusal, FieldValue, TypeEnvironment, Value,
};
use super::node::NodeKey;

/// A graph-local node name.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GraphNodeId(pub u64);

/// One slot of a graph node.
#[derive(Clone, Debug)]
pub enum GraphSlot {
    /// An already completed value.
    Value(Value),
    /// The value built from another graph node.
    Node(GraphNodeId),
    /// Explicit absence.
    Absent,
    /// Explicit `null`.
    Null,
}

/// A constructor node.
#[derive(Clone, Debug)]
pub enum GraphNode {
    /// A record of `declaration`.
    Record {
        /// The record declaration.
        declaration: NodeKey,
        /// Supplied fields.
        fields: Vec<(NodeKey, GraphSlot)>,
    },
    /// A tuple of `declaration`.
    Tuple {
        /// The tuple declaration.
        declaration: NodeKey,
        /// Supplied positions.
        positions: Vec<GraphSlot>,
    },
    /// One constructor of a variant `declaration`.
    Variant {
        /// The variant declaration.
        declaration: NodeKey,
        /// The constructor identity.
        constructor: NodeKey,
        /// Supplied fields.
        fields: Vec<(NodeKey, GraphSlot)>,
    },
}

impl GraphNode {
    fn children(&self) -> impl Iterator<Item = GraphNodeId> + '_ {
        let slots: Box<dyn Iterator<Item = &GraphSlot>> = match self {
            Self::Record { fields, .. } | Self::Variant { fields, .. } => {
                Box::new(fields.iter().map(|(_, slot)| slot))
            }
            Self::Tuple { positions, .. } => Box::new(positions.iter()),
        };
        slots.filter_map(|slot| match slot {
            GraphSlot::Node(id) => Some(*id),
            GraphSlot::Value(_) | GraphSlot::Absent | GraphSlot::Null => None,
        })
    }
}

/// A named containment graph.
#[derive(Clone, Debug, Default)]
pub struct ValueGraph {
    nodes: BTreeMap<GraphNodeId, GraphNode>,
}

/// A graph construction refusal at its originating node.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("value graph refused at node {node:?}: {cause:?}")]
pub struct GraphRefusal {
    /// The originating node.
    pub node: GraphNodeId,
    /// The typed cause.
    pub cause: GraphCause,
}

/// The typed cause of a [`GraphRefusal`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GraphCause {
    /// Two nodes share one name.
    DuplicateNode,
    /// A slot or the root names no node.
    UnknownNode,
    /// This node contains itself through value containment.
    ContainmentCycle,
    /// The node's constructor refuses.
    Construction(ConstructionRefusal),
}

impl ValueGraph {
    /// A graph from named nodes.
    pub fn new(
        nodes: impl IntoIterator<Item = (GraphNodeId, GraphNode)>,
    ) -> Result<Self, GraphRefusal> {
        let mut admitted = BTreeMap::new();
        for (id, node) in nodes {
            if admitted.insert(id, node).is_some() {
                return Err(GraphRefusal {
                    node: id,
                    cause: GraphCause::DuplicateNode,
                });
            }
        }
        Ok(Self { nodes: admitted })
    }
}

enum Visit {
    Enter(GraphNodeId),
    Exit(GraphNodeId),
}

impl TypeEnvironment {
    /// Build the finite value rooted at `root`.
    pub fn build(&self, graph: &ValueGraph, root: GraphNodeId) -> Result<Value, GraphRefusal> {
        let mut built: BTreeMap<GraphNodeId, Value> = BTreeMap::new();
        let mut on_path = BTreeSet::new();
        let mut visits = vec![Visit::Enter(root)];
        while let Some(visit) = visits.pop() {
            match visit {
                Visit::Enter(id) => {
                    if built.contains_key(&id) {
                        continue;
                    }
                    if on_path.contains(&id) {
                        return Err(GraphRefusal {
                            node: id,
                            cause: GraphCause::ContainmentCycle,
                        });
                    }
                    let node = graph.nodes.get(&id).ok_or(GraphRefusal {
                        node: id,
                        cause: GraphCause::UnknownNode,
                    })?;
                    on_path.insert(id);
                    visits.push(Visit::Exit(id));
                    visits.extend(node.children().map(Visit::Enter));
                }
                Visit::Exit(id) => {
                    on_path.remove(&id);
                    let value = graph
                        .nodes
                        .get(&id)
                        .ok_or(GraphCause::UnknownNode)
                        .and_then(|node| self.construct(node, &built))
                        .map_err(|cause| GraphRefusal { node: id, cause })?;
                    built.insert(id, value);
                }
            }
        }
        built.remove(&root).ok_or(GraphRefusal {
            node: root,
            cause: GraphCause::UnknownNode,
        })
    }

    fn construct(
        &self,
        node: &GraphNode,
        built: &BTreeMap<GraphNodeId, Value>,
    ) -> Result<Value, GraphCause> {
        let resolve = |slot: &GraphSlot| match slot {
            GraphSlot::Value(value) => Ok(FieldValue::Present(value.clone())),
            GraphSlot::Node(id) => built
                .get(id)
                .map(|value| FieldValue::Present(value.clone()))
                .ok_or(GraphCause::UnknownNode),
            GraphSlot::Absent => Ok(FieldValue::Absent),
            GraphSlot::Null => Ok(FieldValue::Null),
        };
        let fields = |fields: &[(NodeKey, GraphSlot)]| {
            fields
                .iter()
                .map(|(key, slot)| resolve(slot).map(|value| (*key, value)))
                .collect::<Result<Vec<_>, _>>()
        };
        let constructed = match node {
            GraphNode::Record {
                declaration,
                fields: supplied,
            } => self.record(*declaration, fields(supplied)?),
            GraphNode::Variant {
                declaration,
                constructor,
                fields: supplied,
            } => self.variant(*declaration, *constructor, fields(supplied)?),
            GraphNode::Tuple {
                declaration,
                positions,
            } => {
                let mut values = Vec::with_capacity(positions.len());
                for (index, slot) in positions.iter().enumerate() {
                    let cause = match resolve(slot)? {
                        FieldValue::Present(value) => {
                            values.push(value);
                            continue;
                        }
                        FieldValue::Absent => ConstructionCause::AbsenceNotAdmitted,
                        FieldValue::Null => ConstructionCause::NullNotAdmitted,
                    };
                    return Err(GraphCause::Construction(ConstructionRefusal {
                        component: Component::Position(index),
                        cause,
                    }));
                }
                self.tuple(*declaration, values)
            }
        };
        constructed.map_err(GraphCause::Construction)
    }
}
