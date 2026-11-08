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

use qsl_foundation::source::node_table::{standalone, DebugTable, DebugWith, NodeList};
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
    /// An operand slot of a node-held id names no entry of the holding
    /// node's operand row.
    #[error("operand slot {slot} names no entry of a {len}-entry operand row")]
    SlotOutOfRange {
        /// The slot the segment holds.
        slot: u64,
        /// The holding node's row length.
        len: usize,
    },
    /// An operand slot was read where no operand row applies (a document-held
    /// id).
    #[error("operand slot {slot} has no operand row to resolve against")]
    NoRow {
        /// The slot the segment holds.
        slot: u64,
    },
    /// The rebuilt key is not a well-formed node identity.
    #[error(transparent)]
    Key(#[from] InvalidNodeKey),
}

/// Structural ids built for one task (an emission, a refinement encoding),
/// each operand id interned: every structural id built through one
/// `StructuralIds` holds one shared operand id per operand node, so building
/// one is linear in its own path and equal operands are one allocation.
#[derive(Debug, Default)]
pub struct StructuralIds {
    interned: std::cell::RefCell<HashMap<NodeKey, Arc<CheckedNodeId>>>,
    /// Nodes of imported packages, each named by the id its own package's
    /// emission gave it.
    imported: std::cell::RefCell<HashMap<NodeKey, CheckedNodeId>>,
}

impl StructuralIds {
    /// Read identities emitted by this task while retaining the original
    /// operand keys. An inspection view can then share its nested keys with
    /// the source graph as well as with its other inspection entries.
    pub fn key_reader(&self) -> StructuralKeys {
        StructuralKeys {
            by_operand: self
                .interned
                .borrow()
                .iter()
                .map(|(key, id)| (Arc::as_ptr(id), (id.clone(), key.clone())))
                .collect(),
        }
    }

    /// Names the node `key` of an imported package by `id`, the id that
    /// package's own emission gave it, for every later [`Self::id`] of `key`.
    pub fn bind_imported(&self, key: NodeKey, id: CheckedNodeId) {
        self.imported.borrow_mut().insert(key, id);
    }

    /// The structural form of `key`: each operand is a
    /// [`CheckedNodePathSegment::Node`] holding the operand's own id, shared
    /// with every other id built here that names that operand.
    pub fn id(&self, key: &NodeKey) -> CheckedNodeId {
        if let Some(bound) = self.imported.borrow().get(key) {
            return bound.clone();
        }
        let mut done = self.interned.borrow_mut();
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
    }
}

/// The structural form of `key` on its own: [`StructuralIds::id`] with
/// operands shared within this one id.
pub fn structural_id(key: &NodeKey) -> CheckedNodeId {
    StructuralIds::default().id(key)
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
                CheckedNodePathSegment::Operand(slot) => Err(WireIdError::NoRow { slot: *slot }),
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
            Some(CheckedNodePathSegment::Operand(slot)) => {
                return Err(WireIdError::NoRow { slot: *slot });
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
            Some(CheckedNodePathSegment::Name(_) | CheckedNodePathSegment::Operand(_)) => {}
            None => {
                pending.pop();
            }
        }
    }
    found
}

/// `id`, a node-held id, with each operand slot replaced by the index
/// segment of the node `row` names: the form the rest of this module reads.
fn resolve_operands(id: &CheckedNodeId, row: &[u64]) -> Result<CheckedNodeId, WireIdError> {
    let path = id
        .path
        .iter()
        .map(|segment| match segment {
            CheckedNodePathSegment::Operand(slot) => usize::try_from(*slot)
                .ok()
                .and_then(|at| row.get(at))
                .map(|entry| CheckedNodePathSegment::Index(*entry))
                .ok_or(WireIdError::SlotOutOfRange {
                    slot: *slot,
                    len: row.len(),
                }),
            other => Ok(other.clone()),
        })
        .collect::<Result<_, _>>()?;
    Ok(CheckedNodeId {
        package: id.package.clone(),
        path,
        ordinal: id.ordinal,
    })
}

/// The checked-package document `wire` with every operand slot a node holds
/// replaced by the index segment of the node its row names, and its
/// `operand_rows` removed: the resolved form, in which a node's ids name
/// graph positions directly (IR's `resolved_graph`). A foreign `{package, node}` key holds no slot and is
/// left alone.
///
/// # Errors
///
/// A message when the document has no `semantic_graph.nodes` and
/// `operand_rows` arrays, or a slot names no entry of its node's row.
pub fn resolve_operand_slots(wire: &mut serde_json::Value) -> Result<(), String> {
    use serde_json::Value;
    let graph = wire
        .get_mut("semantic_graph")
        .ok_or("the document has no semantic_graph")?;
    let rows: Vec<Vec<u64>> = serde_json::from_value(
        graph
            .get("operand_rows")
            .cloned()
            .ok_or("the document has no semantic_graph.operand_rows")?,
    )
    .map_err(|error| error.to_string())?;
    if let Some(members) = graph.as_object_mut() {
        members.remove("operand_rows");
    }
    let nodes = graph
        .get_mut("nodes")
        .and_then(Value::as_array_mut)
        .ok_or("the document has no semantic_graph.nodes array")?;
    for (node, row) in nodes.iter_mut().zip(&rows) {
        let mut pending = vec![node];
        while let Some(current) = pending.pop() {
            match current {
                Value::Array(items) => pending.extend(items.iter_mut()),
                Value::Object(object) => {
                    let is_id = object.len() == 3
                        && object.contains_key("package")
                        && object.contains_key("path")
                        && object.contains_key("ordinal");
                    if is_id {
                        if let Some(Value::Array(path)) = object.get_mut("path") {
                            for segment in path.iter_mut() {
                                let slot = segment
                                    .as_object()
                                    .filter(|member| member.len() == 1)
                                    .and_then(|member| member.get("operand"))
                                    .and_then(Value::as_u64);
                                if let Some(slot) = slot {
                                    let entry = usize::try_from(slot)
                                        .ok()
                                        .and_then(|at| row.get(at))
                                        .ok_or_else(|| {
                                            format!("operand slot {slot} has no row entry")
                                        })?;
                                    *segment = serde_json::json!({ "node": entry });
                                }
                            }
                        }
                    } else if object.len() != 2 || !object.contains_key("package") {
                        pending.extend(object.values_mut());
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// FR-322 "Operand index segments": `node`, one semantic node as JSON, with
/// every index segment of every node id it holds replaced by an operand slot
/// numbered by first occurrence in the node's canonical (RFC 8785) member
/// order, and the row listing the graph index each slot names. The emitter
/// writes every node through this walk, so an authored document and an
/// emitted one number slots identically. An `{package, node}` foreign key
/// keeps its index segments: they name the dependency's graph.
pub fn number_index_segments(node: &mut serde_json::Value) -> Vec<u64> {
    use serde_json::Value;
    let mut row: Vec<u64> = Vec::new();
    let mut pending = vec![node];
    while let Some(current) = pending.pop() {
        match current {
            Value::Array(items) => pending.extend(items.iter_mut().rev()),
            Value::Object(object) => {
                let is_id = object.len() == 3
                    && object.contains_key("package")
                    && object.contains_key("path")
                    && object.contains_key("ordinal");
                if is_id {
                    if let Some(Value::Array(path)) = object.get_mut("path") {
                        for segment in path.iter_mut() {
                            let index = segment
                                .as_object()
                                .filter(|member| member.len() == 1)
                                .and_then(|member| member.get("node"))
                                .and_then(Value::as_u64);
                            if let Some(index) = index {
                                let slot = row
                                    .iter()
                                    .position(|entry| *entry == index)
                                    .unwrap_or_else(|| {
                                        row.push(index);
                                        row.len() - 1
                                    });
                                *segment = serde_json::json!({ "operand": slot });
                            }
                        }
                    }
                } else if object.len() != 2 || !object.contains_key("package") {
                    let mut members: Vec<(&String, &mut Value)> = object.iter_mut().collect();
                    members.sort_by(|left, right| left.0.cmp(right.0));
                    pending.extend(members.into_iter().rev().map(|(_, member)| member));
                }
            }
            _ => {}
        }
    }
    row
}

/// The inverse of [`resolve_operand_slots`] for a document authored in
/// resolved form: when it has `semantic_graph.nodes` and no `operand_rows`,
/// each node is written through [`number_index_segments`] and `operand_rows`
/// is added. Any other document is left alone.
pub fn write_operand_rows(wire: &mut serde_json::Value) {
    let Some(graph) = wire.get_mut("semantic_graph") else {
        return;
    };
    if graph.get("operand_rows").is_some() {
        return;
    }
    let Some(nodes) = graph
        .get_mut("nodes")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    let rows: Vec<Vec<u64>> = nodes.iter_mut().map(number_index_segments).collect();
    graph["operand_rows"] = serde_json::json!(rows);
}

#[cfg(any(test, feature = "test-support"))]
/// The checked-package document `bytes` as JSON in its resolved form
/// ([`resolve_operand_slots`]).
///
/// # Errors
///
/// The JSON reader's message, or the [`resolve_operand_slots`] refusal.
pub fn resolved_package_value(bytes: &[u8]) -> Result<serde_json::Value, String> {
    let mut wire: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    resolve_operand_slots(&mut wire)?;
    Ok(wire)
}

/// One checked-package document's `semantic_graph`: the rebuilt key of every
/// node, by graph index, the index of every key, and each node's operand row.
#[derive(Clone, Default)]
pub struct WireGraph {
    keys: Vec<NodeKey>,
    index: HashMap<NodeKey, u64>,
    rows: Vec<Vec<u64>>,
}

/// The keys in graph order; the index is derived from them, and a hash map's
/// iteration order would make two equal graphs format differently.
impl DebugWith for WireGraph {
    fn fmt_with(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
        table: &DebugTable,
    ) -> std::fmt::Result {
        formatter
            .debug_struct("WireGraph")
            .field("keys", &NodeList(self.keys.iter(), table))
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for WireGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        standalone(f, self)
    }
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
        Self::build(ids.into_iter().collect(), Vec::new())
    }

    /// As [`WireGraph::new`] over node ids that hold operand slots (FR-322
    /// "Operand index segments"): `rows[i]` is node `i`'s operand row, through
    /// which each slot of node `i`'s id and of every other node-held id of
    /// node `i` ([`WireGraph::key_in`]) is read.
    ///
    /// # Errors
    ///
    /// [`WireIdError::SlotOutOfRange`] or the [`WireGraph::new`] refusals.
    pub fn with_rows<'a>(
        ids: impl IntoIterator<Item = &'a CheckedNodeId>,
        rows: Vec<Vec<u64>>,
    ) -> Result<Self, WireIdError> {
        let ids: Vec<&CheckedNodeId> = ids.into_iter().collect();
        let resolved = ids
            .iter()
            .enumerate()
            .map(|(at, id)| resolve_operands(id, rows.get(at).map_or(&[], Vec::as_slice)))
            .collect::<Result<Vec<_>, _>>()?;
        Self::build(resolved.iter().collect(), rows)
    }

    fn build(ids: Vec<&CheckedNodeId>, rows: Vec<Vec<u64>>) -> Result<Self, WireIdError> {
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
        Ok(Self { keys, index, rows })
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
            #[serde(default)]
            operand_rows: Vec<Vec<u64>>,
        }
        #[derive(serde::Deserialize)]
        struct Node {
            node_id: CheckedNodeId,
        }
        let document: Document =
            serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        Self::with_rows(
            document
                .semantic_graph
                .nodes
                .iter()
                .map(|node| &node.node_id),
            document.semantic_graph.operand_rows,
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
        let rows = envelope["semantic_graph"]["operand_rows"]
            .as_array()
            .map_or(&[][..], Vec::as_slice);
        Self::of_rowed_node_values(nodes, rows)
    }

    /// The graph whose nodes, in graph order, are the JSON `nodes`, each
    /// holding index segments (the resolved form, with no operand rows).
    ///
    /// # Errors
    ///
    /// As [`WireGraph::of_package_bytes`].
    pub fn of_node_values(nodes: &[serde_json::Value]) -> Result<Self, String> {
        Self::of_rowed_node_values(nodes, &[])
    }

    /// The graph whose nodes, in graph order, are the JSON `nodes`, with
    /// operand rows the JSON `rows`.
    ///
    /// # Errors
    ///
    /// As [`WireGraph::of_package_bytes`].
    pub fn of_rowed_node_values(
        nodes: &[serde_json::Value],
        rows: &[serde_json::Value],
    ) -> Result<Self, String> {
        let ids = nodes
            .iter()
            .map(|node| id_of_value(&node["node_id"]))
            .collect::<Result<Vec<_>, _>>()?;
        let rows = rows
            .iter()
            .map(|row| serde_json::from_value(row.clone()).map_err(|error| error.to_string()))
            .collect::<Result<Vec<Vec<u64>>, _>>()?;
        Self::with_rows(&ids, rows).map_err(|error| error.to_string())
    }

    /// The key of the JSON node id `id` held in graph node `holder`, each
    /// operand slot read through that node's row.
    ///
    /// # Errors
    ///
    /// The JSON reader's message, or the [`WireGraph::key_in`] refusal.
    pub fn key_of_value_in(
        &self,
        holder: usize,
        id: &serde_json::Value,
    ) -> Result<NodeKey, String> {
        self.key_in(holder, &id_of_value(id)?)
            .map_err(|error| error.to_string())
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
        self.structural_value_with(value, |id| self.key_of_value(id))
    }

    /// As [`WireGraph::structural_value`] for a value held in graph node
    /// `holder`, whose operand slots are read through that node's row.
    ///
    /// # Errors
    ///
    /// The [`WireGraph::key_of_value_in`] refusal of the first id that does
    /// not resolve.
    pub fn structural_value_in(
        &self,
        holder: usize,
        value: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.structural_value_with(value, |id| self.key_of_value_in(holder, id))
    }

    fn structural_value_with(
        &self,
        value: &serde_json::Value,
        key_of: impl Fn(&serde_json::Value) -> Result<NodeKey, String>,
    ) -> Result<serde_json::Value, String> {
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
                let key = key_of(current)?;
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

    /// The key of `id`, a node-held id of graph node `holder`: each operand
    /// slot is read through the node's operand row.
    ///
    /// # Errors
    ///
    /// [`WireIdError::SlotOutOfRange`] or the [`WireGraph::key`] refusals.
    pub fn key_in(&self, holder: usize, id: &CheckedNodeId) -> Result<NodeKey, WireIdError> {
        let row = self.rows.get(holder).map_or(&[][..], Vec::as_slice);
        self.key(&resolve_operands(id, row)?)
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
            rows: Vec::new(),
        };
        keys.iter().map(|key| graph.wire_id(key).unwrap()).collect()
    }

    /// FR-322-AC-281: a left-nested chain of 100,000 applications writes one
    /// index segment per operand, and rebuilds 100,001 distinct keys, each
    /// holding its predecessor's key as its last path element.
    #[ix_trace_rs::trace("QSpec-TC-233", "QSpec-FR-322-AC-281")]
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
    #[ix_trace_rs::trace("QSpec-TC-233", "QSpec-FR-322-AC-280")]
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
    #[ix_trace_rs::trace("QSpec-TC-233", "QSpec-FR-322-AC-279")]
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
    #[ix_trace_rs::trace("QSpec-TC-233", "QSpec-FR-322-AC-278")]
    #[test]
    fn a_declared_id_refuses_an_operand() {
        let keys = chain(1);
        assert!(declared_id(&keys[0]).is_ok());
        assert!(matches!(
            declared_id(&keys[1]),
            Err(WireIdError::NotDeclared { .. })
        ));
    }

    /// The inverse memo preserves the emitter's original operand storage,
    /// so formatting an inspection view beside its source graph never has
    /// to compare separately allocated copies of their entire chains.
    /// Provenance: QSL-1358, QSL-381; component sharing regression.
    #[test]
    fn emitted_identity_inspection_reuses_original_operand_keys() {
        let keys = chain(2_000);
        let ids = StructuralIds::default();
        let id = ids.id(keys.last().unwrap());
        let rebuilt = ids.key_reader().key(&id).unwrap();
        assert_eq!(rebuilt, *keys.last().unwrap());
        let PathSegment::Node(operand) = rebuilt.path().pop().unwrap() else {
            panic!("the chain ends in its operand");
        };
        assert!(
            std::ptr::eq(operand.package(), keys[keys.len() - 2].package()),
            "the inverse memo retains the original operand's shared storage"
        );
    }
}
