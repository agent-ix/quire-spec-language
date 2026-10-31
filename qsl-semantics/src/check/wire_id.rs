// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-322 "Operand index segments": the one conversion between a
//! structural [`NodeKey`] and a checked-package [`CheckedNodeId`].
//!
//! A wire `node_id` holds each operand node as the index segment
//! `{"node": i}`, `i` the operand's position in the same document's
//! `semantic_graph.nodes`. The in-memory structural key holds the operand's
//! own key instead. [`WireGraph`] is one document's graph: it rebuilds the
//! key of every graph node once, sharing each operand key wherever it is
//! named, and converts any id the document holds in either direction.
//!
//! [`structural_id`] writes the structural form, each operand as a shared
//! [`CheckedNodePathSegment::Node`]; it is what the emitter builds before the
//! graph order (and so every index) is known.
//!
//! Every walk uses an explicit stack, so the depth of an operand chain never
//! consumes the call stack.

use std::collections::HashMap;
use std::sync::Arc;

use quire_contract_model::{CheckedNodeId, CheckedNodePathSegment};
use quire_exact::{InvalidNodeKey, NodeKey, PathSegment};

/// Why a node id could not be converted.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WireIdError {
    /// An index segment names no node of the document's graph.
    #[error("index segment {index} names no node of a {len}-node graph")]
    IndexOutOfRange {
        /// The index the segment holds.
        index: u64,
        /// The graph's node count.
        len: usize,
    },
    /// Graph node `node` reaches itself through own-path index segments.
    #[error("graph node {node} reaches itself through its own path's index segments")]
    Cycle {
        /// The graph index of the node whose key is defined by itself.
        node: usize,
    },
    /// An index segment was read with no graph to resolve it against.
    #[error("index segment {index} has no graph to resolve against")]
    NoGraph {
        /// The index the segment holds.
        index: u64,
    },
    /// An operand of the key is not a node of the document's graph, so the
    /// key has no wire form there.
    #[error("operand {operand} of the key is not a node of the graph")]
    OperandNotInGraph {
        /// The operand key.
        operand: NodeKey,
    },
    /// The path holds an operand where a declared, string-only path is
    /// required (a `dependency_reference` `node`).
    #[error("the path of {key} holds an operand where only strings are admitted")]
    NotDeclared {
        /// The key whose path holds an operand.
        key: NodeKey,
    },
    /// The rebuilt key is not a well-formed node identity.
    #[error(transparent)]
    Key(#[from] InvalidNodeKey),
}

thread_local! {
    /// This thread's interned operand ids, by key: every structural id built
    /// here holds one shared operand id per operand node, so building one is
    /// linear in its own path and equal operands are one allocation.
    static INTERNED: std::cell::RefCell<HashMap<NodeKey, Arc<CheckedNodeId>>> =
        std::cell::RefCell::new(HashMap::new());
}

/// The structural form of `key`: each operand is a
/// [`CheckedNodePathSegment::Node`] holding the operand's own id, interned on
/// this thread, so every structural id built for one operand shares it.
pub fn structural_id(key: &NodeKey) -> CheckedNodeId {
    INTERNED.with(|interned| {
        let mut done = interned.borrow_mut();
        let path = key
            .path()
            .into_iter()
            .map(|segment| match segment {
                PathSegment::Text(text) => CheckedNodePathSegment::Name(text.into()),
                PathSegment::Node(operand) => {
                    CheckedNodePathSegment::Node(intern(&mut done, &operand))
                }
            })
            .collect();
        CheckedNodeId {
            package: key.package().into(),
            path,
            ordinal: key.ordinal(),
        }
    })
}

/// Release this thread's interned operand ids. The emitter calls this once
/// a package is written; ids already built keep their operands alive.
pub fn release_structural_ids() {
    INTERNED.with(|interned| interned.borrow_mut().clear());
}

/// The interned id of `key`, converting each operand not yet interned once.
fn intern(done: &mut HashMap<NodeKey, Arc<CheckedNodeId>>, key: &NodeKey) -> Arc<CheckedNodeId> {
    let mut pending = vec![(key.clone(), false)];
    while let Some((node, expanded)) = pending.pop() {
        if done.contains_key(&node) {
            continue;
        }
        let path = node.path();
        if !expanded {
            pending.push((node.clone(), true));
            for segment in &path {
                if let PathSegment::Node(operand) = segment {
                    if !done.contains_key(operand) {
                        pending.push((operand.clone(), false));
                    }
                }
            }
            continue;
        }
        let id = CheckedNodeId {
            package: node.package().into(),
            path: path
                .into_iter()
                .map(|segment| match segment {
                    PathSegment::Text(text) => CheckedNodePathSegment::Name(text.into()),
                    PathSegment::Node(operand) => {
                        CheckedNodePathSegment::Node(done[&operand].clone())
                    }
                })
                .collect(),
            ordinal: node.ordinal(),
        };
        done.insert(node, Arc::new(id));
    }
    done[key].clone()
}

/// The structural keys of structural ids, each distinct operand id converted
/// once (by identity of its shared allocation), so converting every id of a
/// package is linear in the package. The memo holds each allocation it keys
/// by, so no address it has seen is reused while it lives.
#[derive(Default)]
pub struct StructuralKeys {
    by_operand: HashMap<*const CheckedNodeId, (Arc<CheckedNodeId>, NodeKey)>,
}

impl StructuralKeys {
    /// The key of `id`, which holds no index segment.
    ///
    /// # Errors
    ///
    /// [`WireIdError::NoGraph`] at an index segment, or [`WireIdError::Key`].
    pub fn key(&mut self, id: &CheckedNodeId) -> Result<NodeKey, WireIdError> {
        // Explicit post-order over operand allocations not yet converted.
        let mut pending: Vec<(Arc<CheckedNodeId>, bool)> = id
            .path
            .iter()
            .filter_map(|segment| match segment {
                CheckedNodePathSegment::Node(operand) => Some((operand.clone(), false)),
                _ => None,
            })
            .collect();
        while let Some((node, expanded)) = pending.pop() {
            if self.by_operand.contains_key(&Arc::as_ptr(&node)) {
                continue;
            }
            if !expanded {
                pending.push((node.clone(), true));
                for segment in &node.path {
                    if let CheckedNodePathSegment::Node(operand) = segment {
                        if !self.by_operand.contains_key(&Arc::as_ptr(operand)) {
                            pending.push((operand.clone(), false));
                        }
                    }
                }
                continue;
            }
            let key = self.shallow(&node)?;
            self.by_operand.insert(Arc::as_ptr(&node), (node, key));
        }
        self.shallow(id)
    }

    /// The key of `id`, every operand already converted.
    fn shallow(&self, id: &CheckedNodeId) -> Result<NodeKey, WireIdError> {
        let path = id
            .path
            .iter()
            .map(|segment| match segment {
                CheckedNodePathSegment::Name(name) => Ok(PathSegment::Text(name.to_string())),
                CheckedNodePathSegment::Index(index) => Err(WireIdError::NoGraph { index: *index }),
                CheckedNodePathSegment::Node(operand) => Ok(PathSegment::Node(
                    self.by_operand[&Arc::as_ptr(operand)].1.clone(),
                )),
            })
            .collect::<Result<_, _>>()?;
        Ok(NodeKey::new(&*id.package, path, id.ordinal)?)
    }
}

/// The wire form of a declared node's key, whose path is its declaration
/// path, all strings: a `dependency_reference` `node` (FR-322 "Dependency
/// references").
///
/// # Errors
///
/// [`WireIdError::NotDeclared`] when the path holds an operand.
pub fn declared_id(key: &NodeKey) -> Result<CheckedNodeId, WireIdError> {
    let path = key
        .view()
        .path()
        .map(|segment| match segment {
            quire_exact::PathSegmentView::Text(text) => {
                Ok(CheckedNodePathSegment::Name(text.into()))
            }
            quire_exact::PathSegmentView::Node(_) => {
                Err(WireIdError::NotDeclared { key: key.clone() })
            }
        })
        .collect::<Result<_, _>>()?;
    Ok(CheckedNodeId {
        package: key.package().into(),
        path,
        ordinal: key.ordinal(),
    })
}

/// The key of `id` when it holds no index segment: a structural id, or the
/// id of a declared node.
///
/// # Errors
///
/// [`WireIdError::NoGraph`] at an index segment, or [`WireIdError::Key`].
pub fn node_key(id: &CheckedNodeId) -> Result<NodeKey, WireIdError> {
    key_with(id, &mut |index| Err(WireIdError::NoGraph { index }))
}

/// The key of `id`, each index segment's key given by `index`.
fn key_with(
    id: &CheckedNodeId,
    index: &mut dyn FnMut(u64) -> Result<NodeKey, WireIdError>,
) -> Result<NodeKey, WireIdError> {
    struct Frame<'a> {
        id: &'a CheckedNodeId,
        rest: std::slice::Iter<'a, CheckedNodePathSegment>,
        path: Vec<PathSegment>,
    }
    fn frame(id: &CheckedNodeId) -> Frame<'_> {
        Frame {
            id,
            rest: id.path.iter(),
            path: Vec::with_capacity(id.path.len()),
        }
    }
    let mut stack = vec![frame(id)];
    while let Some(top) = stack.last_mut() {
        match top.rest.next() {
            Some(CheckedNodePathSegment::Name(name)) => {
                top.path.push(PathSegment::Text(name.to_string()));
            }
            Some(CheckedNodePathSegment::Index(at)) => {
                let key = index(*at)?;
                top.path.push(PathSegment::Node(key));
            }
            Some(CheckedNodePathSegment::Node(nested)) => stack.push(frame(nested)),
            None => {
                let done = stack.pop().expect("the top frame exists");
                let key = NodeKey::new(&*done.id.package, done.path, done.id.ordinal)?;
                match stack.last_mut() {
                    Some(parent) => parent.path.push(PathSegment::Node(key)),
                    None => return Ok(key),
                }
            }
        }
    }
    unreachable!("the root frame returns its key")
}

/// The JSON node id `value` as a [`CheckedNodeId`], which reads raw JSON
/// text rather than a [`serde_json::Value`].
fn id_of_value(value: &serde_json::Value) -> Result<CheckedNodeId, String> {
    serde_json::from_str(&value.to_string()).map_err(|error| error.to_string())
}

/// The index segments `id` holds anywhere in its path, in path order.
fn index_segments(id: &CheckedNodeId) -> Vec<u64> {
    let mut found = Vec::new();
    let mut pending = vec![id.path.iter()];
    while let Some(segments) = pending.last_mut() {
        match segments.next() {
            Some(CheckedNodePathSegment::Index(index)) => found.push(*index),
            Some(CheckedNodePathSegment::Node(nested)) => pending.push(nested.path.iter()),
            Some(CheckedNodePathSegment::Name(_)) => {}
            None => {
                pending.pop();
            }
        }
    }
    found
}

/// One checked-package document's `semantic_graph`: the rebuilt key of every
/// node, by graph index, and the index of every key.
#[derive(Clone, Debug, Default)]
pub struct WireGraph {
    keys: Vec<NodeKey>,
    index: HashMap<NodeKey, u64>,
}

impl WireGraph {
    /// Rebuild the key of each node whose own `node_id` is `ids`, in graph
    /// order. Each index segment names a node of this same graph, which may
    /// come earlier or later in graph order; each named node is resolved once.
    ///
    /// # Errors
    ///
    /// [`WireIdError::IndexOutOfRange`], [`WireIdError::Cycle`] when a node's
    /// key is defined in terms of itself, or [`WireIdError::Key`].
    pub fn new<'a>(ids: impl IntoIterator<Item = &'a CheckedNodeId>) -> Result<Self, WireIdError> {
        let ids: Vec<&CheckedNodeId> = ids.into_iter().collect();
        let len = ids.len();
        let position = |index: u64| {
            usize::try_from(index)
                .ok()
                .filter(|at| *at < len)
                .ok_or(WireIdError::IndexOutOfRange { index, len })
        };
        let mut keys: Vec<Option<NodeKey>> = vec![None; len];
        let mut open = vec![false; len];
        for root in 0..len {
            if keys[root].is_some() {
                continue;
            }
            open[root] = true;
            let mut stack = vec![(root, index_segments(ids[root]), 0_usize)];
            while let Some((node, operands, next)) = stack.last_mut() {
                let node = *node;
                if let Some(&operand) = operands.get(*next) {
                    *next += 1;
                    let at = position(operand)?;
                    if keys[at].is_some() {
                        continue;
                    }
                    if open[at] {
                        return Err(WireIdError::Cycle { node: at });
                    }
                    open[at] = true;
                    stack.push((at, index_segments(ids[at]), 0));
                    continue;
                }
                let key = key_with(ids[node], &mut |index| {
                    Ok(keys[position(index)?]
                        .clone()
                        .expect("every operand is resolved before its holder"))
                })?;
                keys[node] = Some(key);
                open[node] = false;
                stack.pop();
            }
        }
        let keys: Vec<NodeKey> = keys.into_iter().flatten().collect();
        let index = keys
            .iter()
            .enumerate()
            .map(|(at, key)| (key.clone(), at as u64))
            .collect();
        Ok(Self { keys, index })
    }

    /// The graph of the checked-package document `bytes`, read from its
    /// `semantic_graph.nodes[*].node_id` members alone.
    ///
    /// # Errors
    ///
    /// The JSON reader's message when the bytes hold no such graph, or the
    /// [`WireGraph::new`] refusal.
    pub fn of_package_bytes(bytes: &[u8]) -> Result<Self, String> {
        #[derive(serde::Deserialize)]
        struct Document {
            semantic_graph: Graph,
        }
        #[derive(serde::Deserialize)]
        struct Graph {
            nodes: Vec<Node>,
        }
        #[derive(serde::Deserialize)]
        struct Node {
            node_id: CheckedNodeId,
        }
        let document: Document =
            serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        Self::new(
            document
                .semantic_graph
                .nodes
                .iter()
                .map(|node| &node.node_id),
        )
        .map_err(|error| error.to_string())
    }

    /// The graph of the checked-package document `envelope`, already parsed.
    ///
    /// # Errors
    ///
    /// As [`WireGraph::of_package_bytes`].
    pub fn of_package_value(envelope: &serde_json::Value) -> Result<Self, String> {
        let nodes = envelope["semantic_graph"]["nodes"]
            .as_array()
            .ok_or("the document has no semantic_graph.nodes array")?;
        Self::of_node_values(nodes)
    }

    /// The graph whose nodes, in graph order, are the JSON `nodes`.
    ///
    /// # Errors
    ///
    /// As [`WireGraph::of_package_bytes`].
    pub fn of_node_values(nodes: &[serde_json::Value]) -> Result<Self, String> {
        let ids = nodes
            .iter()
            .map(|node| id_of_value(&node["node_id"]))
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(&ids).map_err(|error| error.to_string())
    }

    /// The key of the JSON node id `id` this document holds.
    ///
    /// # Errors
    ///
    /// The JSON reader's message, or the [`WireGraph::key`] refusal.
    pub fn key_of_value(&self, id: &serde_json::Value) -> Result<NodeKey, String> {
        self.key(&id_of_value(id)?)
            .map_err(|error| error.to_string())
    }

    /// The wire id of `key` in this document, as JSON.
    ///
    /// # Errors
    ///
    /// As [`WireGraph::wire_id`].
    pub fn wire_value(&self, key: &NodeKey) -> Result<serde_json::Value, WireIdError> {
        let id = self.wire_id(key)?;
        Ok(serde_json::to_value(&id).expect("an index-segment node id is plain JSON"))
    }

    /// `value`, a JSON value this document holds, with every node id in it
    /// (an object of exactly `package`, `path` and `ordinal`) written as its
    /// structural key's FR-201 identity value, operands nested: the form two
    /// documents' values compare in, whatever their graph orders.
    ///
    /// # Errors
    ///
    /// The [`WireGraph::key_of_value`] refusal of the first id that does not
    /// resolve.
    pub fn structural_value(&self, value: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut value = value.clone();
        let mut pending = vec![&mut value];
        while let Some(current) = pending.pop() {
            let is_id = current.as_object().is_some_and(|object| {
                object.len() == 3
                    && object.contains_key("package")
                    && object.contains_key("path")
                    && object.contains_key("ordinal")
            });
            if is_id {
                let key = self.key_of_value(current)?;
                *current = qsl_foundation::node_wire::node_key_to_json(&key)
                    .map_err(|error| error.to_string())?;
                continue;
            }
            match current {
                serde_json::Value::Object(object) => pending.extend(object.values_mut()),
                serde_json::Value::Array(items) => pending.extend(items.iter_mut()),
                _ => {}
            }
        }
        Ok(value)
    }

    /// The rebuilt keys, in graph order.
    pub fn keys(&self) -> &[NodeKey] {
        &self.keys
    }

    /// The graph index of `key`, when it is a node of this graph.
    pub fn index_of(&self, key: &NodeKey) -> Option<u64> {
        self.index.get(key).copied()
    }

    /// The key of `id`, an id this document holds anywhere: each index
    /// segment names the node of this graph at that index.
    ///
    /// # Errors
    ///
    /// [`WireIdError::IndexOutOfRange`] or [`WireIdError::Key`].
    pub fn key(&self, id: &CheckedNodeId) -> Result<NodeKey, WireIdError> {
        key_with(id, &mut |index| {
            usize::try_from(index)
                .ok()
                .and_then(|at| self.keys.get(at))
                .cloned()
                .ok_or(WireIdError::IndexOutOfRange {
                    index,
                    len: self.keys.len(),
                })
        })
    }

    /// The wire id of `key` in this document: its own path, each operand
    /// written as the index segment of that operand's node.
    ///
    /// # Errors
    ///
    /// [`WireIdError::OperandNotInGraph`] when an operand is not a node of
    /// this graph.
    pub fn wire_id(&self, key: &NodeKey) -> Result<CheckedNodeId, WireIdError> {
        let path = key
            .path()
            .into_iter()
            .map(|segment| match segment {
                PathSegment::Text(text) => Ok(CheckedNodePathSegment::Name(text.into())),
                PathSegment::Node(operand) => self
                    .index_of(&operand)
                    .map(CheckedNodePathSegment::Index)
                    .ok_or(WireIdError::OperandNotInGraph { operand }),
            })
            .collect::<Result<_, _>>()?;
        Ok(CheckedNodeId {
            package: key.package().into(),
            path,
            ordinal: key.ordinal(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> PathSegment {
        PathSegment::Text(value.to_owned())
    }

    fn integer() -> NodeKey {
        NodeKey::new("p", vec![text("scalar_type"), text("integer")], 0).unwrap()
    }

    fn chain(length: usize) -> Vec<NodeKey> {
        let mut keys = vec![integer()];
        for _ in 0..length {
            let previous = keys.last().unwrap().clone();
            keys.push(
                NodeKey::new(
                    "p",
                    vec![
                        text("expression"),
                        text("application"),
                        text("+"),
                        PathSegment::Node(previous),
                    ],
                    0,
                )
                .unwrap(),
            );
        }
        keys
    }

    fn wire_ids(keys: &[NodeKey]) -> Vec<CheckedNodeId> {
        let graph = WireGraph {
            keys: keys.to_vec(),
            index: keys
                .iter()
                .enumerate()
                .map(|(at, key)| (key.clone(), at as u64))
                .collect(),
        };
        keys.iter().map(|key| graph.wire_id(key).unwrap()).collect()
    }

    /// FR-322-AC-281: a left-nested chain of 100,000 applications writes one
    /// index segment per operand, and rebuilds 100,001 distinct keys, each
    /// holding its predecessor's key as its last path element.
    #[ix_trace_rs::trace("TC-233", "FR-322-AC-281")]
    #[test]
    fn a_100000_node_chain_round_trips_through_index_segments() {
        let keys = chain(100_000);
        let ids = wire_ids(&keys);
        assert_eq!(ids[1].path.last(), Some(&CheckedNodePathSegment::Index(0)));
        assert!(ids.iter().all(|id| index_segments(id).len() <= 1));
        let graph = WireGraph::new(&ids).unwrap();
        assert_eq!(graph.keys().len(), keys.len());
        // One deep comparison: two independently built chains compare in
        // time linear in their depth.
        assert_eq!(graph.keys().last(), keys.last());
        let distinct: std::collections::HashSet<_> = graph.keys().iter().collect();
        assert_eq!(distinct.len(), 100_001);
        for (at, key) in graph.keys().iter().enumerate().skip(1) {
            assert_eq!(
                key.path().last(),
                Some(&PathSegment::Node(graph.keys()[at - 1].clone()))
            );
        }
    }

    /// FR-322 "Order": an index segment may name a later node.
    #[ix_trace_rs::trace("TC-233", "FR-322-AC-280")]
    #[test]
    fn an_index_segment_naming_a_later_node_resolves() {
        let keys = chain(3);
        let mut ids = wire_ids(&keys);
        ids.reverse();
        for id in &mut ids {
            for segment in &mut id.path {
                if let CheckedNodePathSegment::Index(index) = segment {
                    *index = 3 - *index;
                }
            }
        }
        let graph = WireGraph::new(&ids).unwrap();
        let mut expected = keys.clone();
        expected.reverse();
        assert_eq!(graph.keys(), expected.as_slice());
    }

    /// FR-322-AC-279 and AC-280: an index past the graph, a self-naming node
    /// and two nodes naming each other are refused.
    #[ix_trace_rs::trace("TC-233", "FR-322-AC-279")]
    #[test]
    fn out_of_range_and_cyclic_index_segments_are_refused() {
        let node = |index: u64| CheckedNodeId {
            package: "p".into(),
            path: vec![
                CheckedNodePathSegment::Name("expression".into()),
                CheckedNodePathSegment::Index(index),
            ],
            ordinal: 0,
        };
        assert_eq!(
            WireGraph::new([&node(4)]).unwrap_err(),
            WireIdError::IndexOutOfRange { index: 4, len: 1 }
        );
        assert_eq!(
            WireGraph::new([&node(0)]).unwrap_err(),
            WireIdError::Cycle { node: 0 }
        );
        assert!(matches!(
            WireGraph::new([&node(1), &node(0)]).unwrap_err(),
            WireIdError::Cycle { .. }
        ));
    }

    /// The structural form shares an operand named twice.
    #[test]
    fn structural_id_round_trips_and_shares_operands() {
        let integer = integer();
        let pair = NodeKey::new(
            "p",
            vec![
                text("composite_type"),
                text("map"),
                PathSegment::Node(integer.clone()),
                PathSegment::Node(integer),
            ],
            0,
        )
        .unwrap();
        let id = structural_id(&pair);
        let (CheckedNodePathSegment::Node(left), CheckedNodePathSegment::Node(right)) =
            (&id.path[2], &id.path[3])
        else {
            panic!("two operands");
        };
        assert!(Arc::ptr_eq(left, right));
        assert_eq!(node_key(&id).unwrap(), pair);
    }

    /// FR-322 "Dependency references": a declared id holds only strings.
    #[ix_trace_rs::trace("TC-233", "FR-322-AC-278")]
    #[test]
    fn a_declared_id_refuses_an_operand() {
        let keys = chain(1);
        assert!(declared_id(&keys[0]).is_ok());
        assert!(matches!(
            declared_id(&keys[1]),
            Err(WireIdError::NotDeclared { .. })
        ));
    }
}
