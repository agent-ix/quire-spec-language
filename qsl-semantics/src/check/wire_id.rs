// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-322 "Operand index segments": the one conversion between a
//! structural [`NodeKey`] and a checked-package [`CheckedNodeId`].
//!
//! A document-held `node_id` holds each operand node as the index segment
//! `{"node": i}`, `i` the operand's position in the same document's
//! `semantic_graph.nodes`. A node-held ID uses `{"operand": k}`, naming
//! entry `k` of its holding node's operand row. The structural key holds the operand's
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
use std::collections::{BTreeMap, HashMap, TryReserveError};
use std::convert::Infallible;
use std::sync::Arc;

use quire_contract_model::{
    CheckedDependencyOperand, CheckedNodeId, CheckedNodePathSegment, CheckedOperandEntry,
    NativeJoinBudget, NativeJoinChargeKind, NativeJoinDeniedCharge, NativeJoinLocus,
};
use quire_exact::{InvalidNodeKey, NodeKey, PathSegment};

/// Why a node id could not be converted.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WireIdError {
    /// The graph does not supply exactly one operand row per node.
    #[error("graph has {nodes} nodes but {rows} operand rows")]
    OperandRowCount {
        /// Number of graph nodes.
        nodes: usize,
        /// Number of operand rows.
        rows: usize,
    },
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
    /// An operand slot was read where only keys are admitted: a slot names an
    /// entry of the holding node's operand row, which this reader never holds.
    #[error("operand slot {slot} has no operand row to resolve against")]
    OperandSlot {
        /// The slot the segment holds.
        slot: u64,
    },
    /// A dependency position was read where only keys are admitted: it names
    /// a node by its position in a dependency's graph, which this reader never
    /// holds.
    #[error("dependency position {package}#{node} has no dependency graph to resolve against")]
    DependencyPosition {
        /// The selected dependency the position is in.
        package: String,
        /// The position in that dependency's graph.
        node: u64,
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

/// Structural ids built for one task (an emission, a refinement encoding),
/// each operand id interned: every structural id built through one
/// `StructuralIds` holds one shared operand id per operand node, so building
/// one is linear in its own path and equal operands are one allocation.
#[derive(Debug, Default)]
pub struct StructuralIds {
    interned: std::cell::RefCell<HashMap<NodeKey, Arc<CheckedNodeId>>>,
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

    /// The structural form of `key`: each operand is a
    /// [`CheckedNodePathSegment::Node`] holding the operand's own id, shared
    /// with every other id built here that names that operand.
    pub fn id(&self, key: &NodeKey) -> CheckedNodeId {
        match materialize_id(key, &mut self.interned.borrow_mut(), &mut Unmetered) {
            Ok(id) => id,
            Err(never) => match never {},
        }
    }
}

/// The structural form of `key` on its own: [`StructuralIds::id`] with
/// operands shared within this one id.
pub fn structural_id(key: &NodeKey) -> CheckedNodeId {
    StructuralIds::default().id(key)
}

/// A structural-id materialization stopped before an unpaid allocation.
#[derive(Debug, thiserror::Error)]
pub enum StructuralIdMaterializationError {
    /// The caller's actual native ledger denied its next logical charge.
    #[error("native structural-id charge denied: {0:?}")]
    Budget(NativeJoinDeniedCharge),
    /// A paid working list or package string could not reserve storage.
    #[error(transparent)]
    Allocation(#[from] TryReserveError),
}

/// Materialize the original structural identity under the caller's one budget.
///
/// Work visits and copies are charged before they happen. Logical storage
/// charges cover each temporary container, its reserved entries, every owned
/// UTF-8 byte, and each newly retained shared id/cache entry. The conversion
/// never resets the ledger and uses the same identity traversal and operand
/// interning as [`structural_id`]. It does not meter a later encoder's heap.
///
/// # Errors
///
/// The original native denial or a paid list/string reservation failure.
pub fn try_structural_id(
    key: &NodeKey,
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
) -> Result<CheckedNodeId, StructuralIdMaterializationError> {
    let mut provision = Metered { budget, locus };
    provision.storage(1)?;
    materialize_id(key, &mut HashMap::new(), &mut provision)
}

trait Materialization {
    type Error;

    fn work(&mut self, amount: u128) -> Result<(), Self::Error>;
    fn storage(&mut self, amount: u128) -> Result<(), Self::Error>;
    fn room<T>(&mut self, list: &mut Vec<T>, additional: usize) -> Result<(), Self::Error>;
    fn text(&mut self, text: &str) -> Result<Box<str>, Self::Error>;
    fn cache_room(
        &mut self,
        cache: &mut HashMap<NodeKey, Arc<CheckedNodeId>>,
    ) -> Result<(), Self::Error>;

    fn path(&mut self, key: &NodeKey) -> Result<Vec<PathSegment>, Self::Error> {
        self.storage(1)?;
        let mut segments = key.view().path();
        loop {
            self.work(1)?;
            let Some(segment) = segments.next() else {
                break;
            };
            self.storage(1)?;
            if let quire_exact::PathSegmentView::Text(text) = segment {
                self.storage(1 + text.len() as u128)?;
                self.work(text.len() as u128)?;
            }
        }
        // The kernel copies exactly these direct text segments; nested keys
        // are shallow owner handles. All logical copies were paid above.
        Ok(key.path())
    }
}

struct Unmetered;

impl Materialization for Unmetered {
    type Error = Infallible;

    fn work(&mut self, _: u128) -> Result<(), Infallible> {
        Ok(())
    }
    fn storage(&mut self, _: u128) -> Result<(), Infallible> {
        Ok(())
    }
    fn room<T>(&mut self, list: &mut Vec<T>, additional: usize) -> Result<(), Infallible> {
        list.reserve(additional);
        Ok(())
    }
    fn text(&mut self, text: &str) -> Result<Box<str>, Infallible> {
        Ok(text.into())
    }
    fn cache_room(
        &mut self,
        _: &mut HashMap<NodeKey, Arc<CheckedNodeId>>,
    ) -> Result<(), Infallible> {
        Ok(())
    }
}

struct Metered<'a> {
    budget: &'a mut NativeJoinBudget,
    locus: NativeJoinLocus,
}

impl Materialization for Metered<'_> {
    type Error = StructuralIdMaterializationError;

    fn work(&mut self, amount: u128) -> Result<(), Self::Error> {
        self.budget
            .accounted_charge(NativeJoinChargeKind::Work, amount, self.locus)
            .map_err(StructuralIdMaterializationError::Budget)
    }
    fn storage(&mut self, amount: u128) -> Result<(), Self::Error> {
        self.budget
            .accounted_charge(NativeJoinChargeKind::Storage, amount, self.locus)
            .map_err(StructuralIdMaterializationError::Budget)
    }
    fn room<T>(&mut self, list: &mut Vec<T>, additional: usize) -> Result<(), Self::Error> {
        let Some(needed) = list.len().checked_add(additional) else {
            self.storage(list.len() as u128 + additional as u128)?;
            list.try_reserve_exact(additional)?;
            return Ok(());
        };
        if needed > list.capacity() {
            let target = needed.max(list.capacity().saturating_mul(2));
            self.storage(target as u128)?;
            list.try_reserve_exact(target - list.len())?;
        }
        Ok(())
    }
    fn text(&mut self, text: &str) -> Result<Box<str>, Self::Error> {
        self.storage(1 + text.len() as u128)?;
        self.work(text.len() as u128)?;
        let mut owned = String::new();
        owned.try_reserve_exact(text.len())?;
        owned.push_str(text);
        Ok(owned.into_boxed_str())
    }
    fn cache_room(
        &mut self,
        cache: &mut HashMap<NodeKey, Arc<CheckedNodeId>>,
    ) -> Result<(), Self::Error> {
        // One retained map entry. Payload/path strings and shared id are
        // charged separately; this is logical storage, not a table-layout estimate.
        self.storage(1)?;
        cache.try_reserve(1)?;
        Ok(())
    }
}

fn materialize_id<P: Materialization>(
    key: &NodeKey,
    done: &mut HashMap<NodeKey, Arc<CheckedNodeId>>,
    provision: &mut P,
) -> Result<CheckedNodeId, P::Error> {
    provision.work(1)?;
    let segments = provision.path(key)?;
    provision.storage(1)?;
    let mut path = Vec::new();
    provision.room(&mut path, segments.len())?;
    for segment in segments {
        provision.work(1)?;
        path.push(match segment {
            PathSegment::Text(text) => CheckedNodePathSegment::Name(text.into()),
            PathSegment::Node(operand) => {
                CheckedNodePathSegment::Node(intern(done, &operand, provision)?)
            }
        });
    }
    Ok(CheckedNodeId {
        package: provision.text(key.package())?,
        path,
        ordinal: key.ordinal(),
    })
}

/// The one shared iterative operand conversion, with allocation policy kept
/// separate from structural identity decisions.
fn intern<P: Materialization>(
    done: &mut HashMap<NodeKey, Arc<CheckedNodeId>>,
    key: &NodeKey,
    provision: &mut P,
) -> Result<Arc<CheckedNodeId>, P::Error> {
    provision.storage(1)?;
    let mut pending = Vec::new();
    provision.room(&mut pending, 1)?;
    pending.push((key.clone(), false));
    while let Some((node, expanded)) = pending.pop() {
        provision.work(1)?;
        if done.contains_key(&node) {
            continue;
        }
        let path = provision.path(&node)?;
        if !expanded {
            provision.room(&mut pending, 1)?;
            pending.push((node.clone(), true));
            for segment in &path {
                provision.work(1)?;
                if let PathSegment::Node(operand) = segment {
                    if !done.contains_key(operand) {
                        provision.room(&mut pending, 1)?;
                        pending.push((operand.clone(), false));
                    }
                }
            }
            continue;
        }
        provision.storage(1)?;
        let mut converted = Vec::new();
        provision.room(&mut converted, path.len())?;
        for segment in path {
            provision.work(1)?;
            converted.push(match segment {
                PathSegment::Text(text) => CheckedNodePathSegment::Name(text.into()),
                PathSegment::Node(operand) => CheckedNodePathSegment::Node(done[&operand].clone()),
            });
        }
        let id = CheckedNodeId {
            package: provision.text(node.package())?,
            path: converted,
            ordinal: node.ordinal(),
        };
        provision.storage(1)?;
        provision.cache_room(done)?;
        done.insert(node, Arc::new(id));
    }
    Ok(done[key].clone())
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
                CheckedNodePathSegment::Operand(slot) => {
                    Err(WireIdError::OperandSlot { slot: *slot })
                }
                CheckedNodePathSegment::Dependency { package, node } => {
                    Err(WireIdError::DependencyPosition {
                        package: package.to_string(),
                        node: *node,
                    })
                }
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
                return Err(WireIdError::OperandSlot { slot: *slot })
            }
            Some(CheckedNodePathSegment::Dependency { package, node }) => {
                return Err(WireIdError::DependencyPosition {
                    package: package.to_string(),
                    node: *node,
                })
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
            Some(
                CheckedNodePathSegment::Name(_)
                | CheckedNodePathSegment::Operand(_)
                | CheckedNodePathSegment::Dependency { .. },
            ) => {}
            None => {
                pending.pop();
            }
        }
    }
    found
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
    let graph = WireGraph::of_package_value(wire)?;
    let semantic_graph = &mut wire["semantic_graph"];
    let nodes = semantic_graph["nodes"]
        .as_array_mut()
        .ok_or("the document has no semantic_graph.nodes array")?;
    for (holder, node) in nodes.iter_mut().enumerate() {
        *node = graph.document_value_in_row(holder, node)?;
    }
    semantic_graph
        .as_object_mut()
        .unwrap()
        .remove("operand_rows");
    Ok(())
}

/// Walk node-ID paths in canonical order, excluding data and foreign graph
/// references: every document index segment `{"node": i}` is an
/// [`CheckedOperandEntry::Node`], and every dependency position
/// `{"node": j, "package": p}` held in a path is an
/// [`CheckedOperandEntry::Dependency`].
///
/// # Errors
///
/// Refuses an unreadable canonical document or a non-integer index segment.
#[ix_trace_rs::string_edge]
pub fn canonical_operand_entries(
    bytes: &[u8],
    byte_limit: u64,
) -> Result<Vec<CheckedOperandEntry>, String> {
    use quire_canonical::{Items, Members, Node, NodeRef};
    enum Pending<'a> {
        Value(NodeRef<'a>),
        Members(Members<'a>),
        Items(Items<'a>),
    }
    let document = quire_canonical::read(bytes, byte_limit)
        .map_err(|_| "invalid canonical operand index".to_owned())?;
    let mut pending = vec![Pending::Value(document.root())];
    let mut indices = Vec::new();
    while let Some(next) = pending.pop() {
        match next {
            Pending::Value(value) => match value.node() {
                Node::Object(members) => {
                    let member_count = members.clone().count();
                    if (member_count == 2
                        || (member_count == 3
                            && value.get("term").is_some_and(|term| {
                                matches!(term.node(), Node::String("dependency_reference"))
                            })))
                        && value.get("package").is_some()
                        && value.get("node").is_some()
                    {
                        continue;
                    }
                    let is_id = member_count == 3
                        && value.get("package").is_some()
                        && value.get("ordinal").is_some()
                        && value.get("path").is_some();
                    if is_id {
                        let Some(path) = value.get("path") else {
                            continue;
                        };
                        let Node::Array(segments) = path.node() else {
                            continue;
                        };
                        for segment in segments {
                            if let Some(index) = segment.get("node") {
                                let Node::Number(number) = index.node() else {
                                    return Err("invalid canonical operand index".to_owned());
                                };
                                let node: u64 = number
                                    .text()
                                    .parse()
                                    .map_err(|_| "invalid canonical operand index".to_owned())?;
                                indices.push(match segment.get("package") {
                                    None => CheckedOperandEntry::Node(node),
                                    Some(package) => {
                                        let Node::String(package) = package.node() else {
                                            return Err(
                                                "invalid canonical dependency package".to_owned()
                                            );
                                        };
                                        CheckedOperandEntry::Dependency(CheckedDependencyOperand {
                                            package: package.into(),
                                            node,
                                        })
                                    }
                                });
                            }
                        }
                    } else {
                        pending.push(Pending::Members(members));
                    }
                }
                Node::Array(items) => pending.push(Pending::Items(items)),
                Node::Null | Node::Bool(_) | Node::Number(_) | Node::String(_) => {}
            },
            Pending::Members(mut members) => {
                if let Some((_, value)) = members.next() {
                    pending.push(Pending::Members(members));
                    pending.push(Pending::Value(value));
                }
            }
            Pending::Items(mut items) => {
                if let Some(value) = items.next() {
                    pending.push(Pending::Items(items));
                    pending.push(Pending::Value(value));
                }
            }
        }
    }
    Ok(indices)
}

/// FR-322 "Operand index segments": `node`, one semantic node as JSON, with
/// every index segment of every node id it holds replaced by an operand slot
/// numbered by first occurrence in the node's canonical (RFC 8785) member
/// order, and the row listing the graph index each slot names. The emitter
/// uses the same canonical index walk, so an authored document and an
/// emitted one number slots identically. An `{package, node}` foreign key
/// keeps its index segments: they name the dependency's graph.
pub fn number_index_segments(node: &mut serde_json::Value) -> Vec<u64> {
    use serde_json::Value;
    let bytes = quire_canonical::to_vec(&*node, quire_canonical::Limits::new(u64::MAX))
        .expect("a JSON inspection value has a canonical encoding");
    let mut row = Vec::new();
    let mut slots = HashMap::new();
    for index in canonical_operand_entries(&bytes, u64::MAX)
        .expect("a resolved inspection value has integer graph indices")
        .into_iter()
        .filter_map(|entry| match entry {
            CheckedOperandEntry::Node(index) => Some(index),
            CheckedOperandEntry::Dependency(_) => None,
        })
    {
        if let std::collections::hash_map::Entry::Vacant(entry) = slots.entry(index) {
            entry.insert(u64::try_from(row.len()).expect("operand row length fits a graph index"));
            row.push(index);
        }
    }
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
                            let index = segment
                                .as_object()
                                .filter(|member| member.len() == 1)
                                .and_then(|member| member.get("node"))
                                .and_then(Value::as_u64);
                            if let Some(index) = index {
                                *segment = serde_json::json!({"operand": slots[&index]});
                            }
                        }
                    }
                } else if !foreign_id_container(object) {
                    pending.extend(object.values_mut());
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

fn foreign_id_container(object: &serde_json::Map<String, serde_json::Value>) -> bool {
    (object.len() == 2
        || (object.len() == 3
            && object.get("term").and_then(serde_json::Value::as_str)
                == Some("dependency_reference")))
        && object.contains_key("package")
        && object.contains_key("node")
}

/// One checked-package document's `semantic_graph`: the rebuilt key of every
/// node, by graph index, and the index of every key.
#[derive(Clone, Default)]
pub struct WireGraph {
    keys: Vec<NodeKey>,
    index: HashMap<NodeKey, u64>,
    slots: BTreeMap<CheckedNodeId, Option<usize>>,
    rows: Vec<Vec<CheckedOperandEntry>>,
}

#[derive(Clone, Copy)]
enum IdentityView {
    Structural,
    Document,
}

fn resolve_slots(id: &mut CheckedNodeId, row: &[CheckedOperandEntry]) -> Result<(), WireIdError> {
    for segment in &mut id.path {
        if let CheckedNodePathSegment::Operand(slot) = segment {
            let entry = usize::try_from(*slot)
                .ok()
                .and_then(|slot| row.get(slot))
                .ok_or(WireIdError::OperandSlot { slot: *slot })?;
            *segment = match entry {
                CheckedOperandEntry::Node(index) => CheckedNodePathSegment::Index(*index),
                CheckedOperandEntry::Dependency(CheckedDependencyOperand { package, node }) => {
                    CheckedNodePathSegment::Dependency {
                        package: package.clone(),
                        node: *node,
                    }
                }
            };
        }
    }
    Ok(())
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
        Ok(Self {
            keys,
            index,
            slots: BTreeMap::new(),
            rows: Vec::new(),
        })
    }

    /// Resolve node-held IDs through their corresponding operand rows.
    ///
    /// # Errors
    ///
    /// Refuses missing rows, invalid slots, graph indices and cycles.
    pub fn with_operand_rows(
        ids: &[CheckedNodeId],
        rows: &[Vec<CheckedOperandEntry>],
    ) -> Result<Self, WireIdError> {
        if ids.len() != rows.len() {
            return Err(WireIdError::OperandRowCount {
                nodes: ids.len(),
                rows: rows.len(),
            });
        }
        let mut resolved = ids.to_vec();
        for (id, row) in resolved.iter_mut().zip(rows) {
            resolve_slots(id, row)?;
        }
        let mut graph = Self::new(&resolved)?;
        graph.rows = rows.to_vec();
        for (position, id) in ids.iter().enumerate() {
            if id
                .path
                .iter()
                .any(|segment| matches!(segment, CheckedNodePathSegment::Operand(_)))
            {
                graph
                    .slots
                    .entry(id.clone())
                    .and_modify(|entry| *entry = None)
                    .or_insert(Some(position));
            }
        }
        Ok(graph)
    }

    /// Resolve an ID held by the node at `holder`, using that node's row.
    ///
    /// # Errors
    ///
    /// Refuses a missing holder, an invalid slot or an invalid graph index.
    pub fn key_in_row(&self, holder: usize, id: &CheckedNodeId) -> Result<NodeKey, WireIdError> {
        self.key(&self.resolved_in_row(holder, id)?)
    }

    /// `id`, held by the node at `holder`, with each operand slot replaced by
    /// what that node's row names: an index segment, or a dependency position.
    fn resolved_in_row(
        &self,
        holder: usize,
        id: &CheckedNodeId,
    ) -> Result<CheckedNodeId, WireIdError> {
        let mut resolved = id.clone();
        if self.rows.is_empty() {
            return Ok(resolved);
        }
        let row = self.rows.get(holder).ok_or(WireIdError::IndexOutOfRange {
            index: holder as u64,
            len: self.rows.len(),
        })?;
        resolve_slots(&mut resolved, row)?;
        Ok(resolved)
    }

    /// The graph of the checked-package document `bytes`, read from its
    /// node IDs and corresponding `semantic_graph.operand_rows`.
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
            operand_rows: Vec<Vec<CheckedOperandEntry>>,
        }
        #[derive(serde::Deserialize)]
        struct Node {
            node_id: CheckedNodeId,
        }
        let document: Document =
            serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        let ids: Vec<_> = document
            .semantic_graph
            .nodes
            .into_iter()
            .map(|node| node.node_id)
            .collect();
        Self::with_operand_rows(&ids, &document.semantic_graph.operand_rows)
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
        let ids = nodes
            .iter()
            .map(|node| id_of_value(&node["node_id"]))
            .collect::<Result<Vec<_>, _>>()?;
        if envelope["semantic_graph"].get("operand_rows").is_none() {
            return Self::new(&ids).map_err(|error| error.to_string());
        }
        let rows: Vec<Vec<CheckedOperandEntry>> =
            serde_json::from_value(envelope["semantic_graph"]["operand_rows"].clone())
                .map_err(|error| error.to_string())?;
        Self::with_operand_rows(&ids, &rows).map_err(|error| error.to_string())
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
        self.identity_value(value, None, IdentityView::Structural)
    }

    /// Convert a node-held JSON value using its holder's operand row.
    ///
    /// # Errors
    ///
    /// Refuses invalid slots, graph indices or structural identities.
    pub fn structural_value_in_row(
        &self,
        holder: usize,
        value: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.identity_value(value, Some(holder), IdentityView::Structural)
    }

    /// Inspect a node-held JSON value with its local operand slots written
    /// as document-held graph indices. Foreign package references retain
    /// their own graph's indices. This is an inspection value, not V2 wire.
    ///
    /// # Errors
    ///
    /// Refuses invalid local slots, indices or structural identities.
    pub fn document_value_in_row(
        &self,
        holder: usize,
        value: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.identity_value(value, Some(holder), IdentityView::Document)
    }

    fn identity_value(
        &self,
        value: &serde_json::Value,
        holder: Option<usize>,
        view: IdentityView,
    ) -> Result<serde_json::Value, String> {
        let mut value = value.clone();
        let mut pending = vec![(&mut value, holder)];
        while let Some((current, mut holder)) = pending.pop() {
            if current.as_object().is_some_and(foreign_id_container) {
                continue;
            }
            if holder.is_none() && !self.rows.is_empty() && current.get("node_tag").is_some() {
                let id = id_of_value(&current["node_id"])?;
                let key = self.key(&id).map_err(|error| error.to_string())?;
                holder = self
                    .index_of(&key)
                    .and_then(|index| usize::try_from(index).ok());
            }
            let is_id = current.as_object().is_some_and(|object| {
                object.len() == 3
                    && object.contains_key("package")
                    && object.contains_key("path")
                    && object.contains_key("ordinal")
            });
            if is_id {
                let id = id_of_value(current)?;
                let id = match holder {
                    Some(holder) => self
                        .resolved_in_row(holder, &id)
                        .map_err(|error| error.to_string())?,
                    None => id,
                };
                // A dependency position names a node of another package's
                // graph: the document view keeps it as the wire writes it.
                let names_dependency = id
                    .path
                    .iter()
                    .any(|segment| matches!(segment, CheckedNodePathSegment::Dependency { .. }));
                if names_dependency && matches!(view, IdentityView::Document) {
                    *current = serde_json::to_value(&id).map_err(|error| error.to_string())?;
                    continue;
                }
                let key = self.key(&id).map_err(|error| error.to_string())?;
                *current = match view {
                    IdentityView::Structural => qsl_foundation::node_wire::node_key_to_json(&key)
                        .map_err(|error| error.to_string())?,
                    IdentityView::Document => {
                        self.wire_value(&key).map_err(|error| error.to_string())?
                    }
                };
                continue;
            }
            match current {
                serde_json::Value::Object(object) => {
                    pending.extend(object.values_mut().map(|value| (value, holder)))
                }
                serde_json::Value::Array(items) => {
                    pending.extend(items.iter_mut().map(|value| (value, holder)))
                }
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

    /// The key of a document-held `id`: each index segment names the node
    /// of this graph at that index. A slot-form own ID also resolves when
    /// exactly one graph node has those bytes. For other node-held IDs,
    /// use [`Self::key_in_row`] with their holding node's position.
    ///
    /// # Errors
    ///
    /// [`WireIdError::IndexOutOfRange`], [`WireIdError::OperandSlot`] for an
    /// ambiguous or unscoped slot-form ID, or [`WireIdError::Key`].
    pub fn key(&self, id: &CheckedNodeId) -> Result<NodeKey, WireIdError> {
        if let Some(Some(position)) = self.slots.get(id) {
            return Ok(self.keys[*position].clone());
        }
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

    #[ix_trace_rs::trace("QSpec-TC-233", "QSpec-FR-322-AC-280")]
    #[test]
    fn operand_slots_use_the_holding_rows_and_ambiguous_ids_are_refused() {
        let keys = chain(2);
        let mut ids = wire_ids(&keys);
        ids[1].path.pop();
        ids[1].path.push(CheckedNodePathSegment::Operand(0));
        ids[2].path.pop();
        ids[2].path.push(CheckedNodePathSegment::Operand(0));
        assert_eq!(ids[1], ids[2]);
        let row = |index| vec![CheckedOperandEntry::Node(index)];
        let rows = vec![vec![], row(0), row(1)];
        let graph = WireGraph::with_operand_rows(&ids, &rows).unwrap();
        assert_eq!(graph.keys(), keys);
        assert_eq!(graph.key_in_row(1, &ids[1]).unwrap(), keys[1]);
        assert_eq!(graph.key_in_row(2, &ids[2]).unwrap(), keys[2]);
        assert!(matches!(
            graph.key(&ids[1]),
            Err(WireIdError::OperandSlot { slot: 0 })
        ));
        assert!(matches!(
            WireGraph::with_operand_rows(&ids, &[vec![], vec![], row(1)]),
            Err(WireIdError::OperandSlot { slot: 0 })
        ));
        assert!(matches!(
            WireGraph::with_operand_rows(&ids, &[vec![], row(2), row(1)]),
            Err(WireIdError::Cycle { .. })
        ));
        assert_eq!(
            WireGraph::with_operand_rows(&ids, &[vec![], row(3), row(1)]).unwrap_err(),
            WireIdError::IndexOutOfRange { index: 3, len: 3 }
        );
        assert_eq!(
            WireGraph::with_operand_rows(&ids, &[]).unwrap_err(),
            WireIdError::OperandRowCount { nodes: 3, rows: 0 }
        );
    }

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
            slots: BTreeMap::new(),
            rows: Vec::new(),
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
    #[ix_trace_rs::trace("TC-1081", "FR-251-AC-49")]
    #[test]
    fn charged_structural_materialization_preserves_original_ids_and_shared_operands() {
        let keys = chain(2);
        let key = NodeKey::new(
            "example/charged-node",
            vec![
                PathSegment::Node(keys[1].clone()),
                PathSegment::Node(keys[1].clone()),
            ],
            7,
        )
        .unwrap();
        let locus = NativeJoinLocus {
            source: quire_contract_model::NativeJoinLocusSource::ReachedRow(37),
            form: "qsl_authority_selection",
        };
        let mut budget = NativeJoinBudget::new(1_000_000, 1_000_000);
        budget.resume();
        let actual = try_structural_id(&key, &mut budget, locus).unwrap();
        assert_eq!(actual, structural_id(&key));
        let (CheckedNodePathSegment::Node(left), CheckedNodePathSegment::Node(right)) =
            (&actual.path[0], &actual.path[1])
        else {
            panic!("original operand paths");
        };
        assert!(Arc::ptr_eq(left, right));
        let total = budget.consumed();
        let mut exact = NativeJoinBudget::new(total.work, total.storage);
        exact.resume();
        assert_eq!(try_structural_id(&key, &mut exact, locus).unwrap(), actual);
        assert_eq!(exact.consumed(), total);
        for kind in [NativeJoinChargeKind::Work, NativeJoinChargeKind::Storage] {
            let mut short = NativeJoinBudget::new(
                total.work - u64::from(kind == NativeJoinChargeKind::Work),
                total.storage - u64::from(kind == NativeJoinChargeKind::Storage),
            );
            short.resume();
            let error = try_structural_id(&key, &mut short, locus).unwrap_err();
            let StructuralIdMaterializationError::Budget(denied) = error else {
                panic!("the original native ledger denies before materialization");
            };
            assert_eq!(denied.kind, kind);
            assert_eq!(Some(denied), short.first_unpaid());
            assert_eq!(short.first_unpaid_locus(), Some(locus));
        }
        let mut reused = NativeJoinBudget::new(total.work * 2, total.storage * 2 - 1);
        reused.resume();
        try_structural_id(&key, &mut reused, locus).unwrap();
        let prefix = reused.consumed();
        assert!(matches!(
            try_structural_id(&key, &mut reused, locus),
            Err(StructuralIdMaterializationError::Budget(_))
        ));
        assert!(reused.consumed().work >= prefix.work);
        assert!(reused.consumed().storage >= prefix.storage);
    }

    fn dependency_id() -> CheckedNodeId {
        CheckedNodeId {
            package: "pkg".into(),
            path: vec![
                CheckedNodePathSegment::Name("clause".into()),
                CheckedNodePathSegment::Dependency {
                    package: "test/producer".into(),
                    node: 7,
                },
            ],
            ordinal: 0,
        }
    }

    /// FR-322 "Operand index segments": a canonical document's index segment
    /// is a local entry, and one carrying a package is a dependency entry.
    #[ix_trace_rs::trace("QSpec-TC-233", "QSpec-FR-322-AC-280")]
    #[test]
    fn canonical_entries_separate_local_and_dependency_segments() {
        let document = serde_json::json!({
            "a": {"package": "pkg", "ordinal": 0, "path": [{"node": 3}]},
            "b": {"package": "pkg", "ordinal": 0, "path": [
                {"node": 5, "package": "test/producer"}
            ]},
        });
        let bytes = serde_json::to_vec(&document).unwrap();
        assert_eq!(
            canonical_operand_entries(&bytes, 1 << 20).unwrap(),
            vec![
                CheckedOperandEntry::Node(3),
                CheckedOperandEntry::Dependency(CheckedDependencyOperand {
                    package: "test/producer".into(),
                    node: 5,
                }),
            ]
        );
    }

    /// A key reader holds no dependency graph, so it refuses a position.
    #[ix_trace_rs::trace("QSpec-TC-233", "QSpec-FR-322-AC-280")]
    #[test]
    fn a_dependency_position_has_no_key() {
        assert_eq!(
            StructuralKeys::default().key(&dependency_id()).unwrap_err(),
            WireIdError::DependencyPosition {
                package: "test/producer".to_owned(),
                node: 7,
            }
        );
    }

    /// The document view keeps a dependency position as the wire writes it.
    #[ix_trace_rs::trace("QSpec-TC-233", "QSpec-FR-322-AC-280")]
    #[test]
    fn the_document_view_keeps_a_dependency_position_inline() {
        let ids = wire_ids(&chain(1));
        let graph = WireGraph::new(&ids).unwrap();
        let held = serde_json::to_value(dependency_id()).unwrap();
        let value = serde_json::json!({"clause": held.clone()});
        assert_eq!(graph.document_value_in_row(0, &value).unwrap(), value);
        // The structural view has no key for it.
        assert!(graph.structural_value_in_row(0, &value).is_err());
    }
}
