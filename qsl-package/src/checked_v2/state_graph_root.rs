// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-410 "Outputs" and "Behavior" (TC-1140): the I2 lookup of a state-graph
//! claim from an admitted `quire.checked-package/v2`.
//!
//! Each `claim` occurrence of a `claim`/`state_graph` node is one declaration
//! of the claim: its identity triple is `[node, "claim", ordinal]` (an O-07
//! occurrence key), and the node's body (QSpec FR-475) gives its form, its
//! `from` and `target` predicate nodes and its `over` node. The lookup is
//! separate from [`super::resolve_clause_root`]: each accepts only its own
//! node form, so the temporal lookup returns `clause-root-unresolved` for a
//! state-graph triple and this one does for a temporal clause's.
//!
//! A triple naming a node the package does not hold, a node that is not a
//! `claim`/`state_graph` node, or an occurrence the node does not carry
//! (another role, another ordinal) is [`ClauseRootUnresolved`], a lookup
//! result and not a package refusal.

use quire_contract_model::{
    CheckedNodeKind, CheckedNodeTag, CheckedOccurrenceRole, CheckedPackageV2,
    CheckedSemanticNodeV2, CheckedTerm, ClaimForm,
};

use qsl_foundation::source::provenance::OccurrenceKey;
use quire_contract_model::CheckedNodeId;
use quire_exact::{NodeKey, Origin, Role};
use quire_semantic_value::catalog::InternalFault;

use super::ClauseRootUnresolved;

/// The form of an admitted state-graph claim (QSpec FR-475 `kind`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateGraphKind {
    /// `possible`.
    Possible,
    /// `always-possible`.
    AlwaysPossible,
    /// `unique-path`.
    UniquePath,
}

/// One declaration of an admitted state-graph claim: its identity triple, its
/// form, its predicates and its `over` node (FR-410 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateGraphRoot {
    occurrence: OccurrenceKey,
    kind: StateGraphKind,
    over: Option<NodeKey>,
    from: Option<NodeKey>,
    target: NodeKey,
}

impl StateGraphRoot {
    /// The identity triple `[node, "claim", ordinal]`.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The claim's form.
    pub fn kind(&self) -> StateGraphKind {
        self.kind
    }

    /// The `value`/`parameter` node of the `over` parameter, or `None` for a
    /// claim written without one.
    pub fn over(&self) -> Option<&NodeKey> {
        self.over.as_ref()
    }

    /// The predicate node of `from`, or `None` for `possible`.
    pub fn from(&self) -> Option<&NodeKey> {
        self.from.as_ref()
    }

    /// The predicate node of `target` (`unique path`'s `Y`).
    pub fn target(&self) -> &NodeKey {
        &self.target
    }
}

/// A broken invariant between IR's admission and this lookup.
const NODE_ID_FAULT: InternalFault =
    InternalFault::new("I2", "admitted-node-id-is-a-structural-node-id");

/// A broken invariant between IR's admission of the body and this reading.
const BODY_FAULT: InternalFault = InternalFault::new("I2", "admitted-state-graph-body-is-read");

/// Whether `node` is a `claim`/`state_graph` node.
fn is_state_graph(node: &CheckedSemanticNodeV2) -> bool {
    CheckedNodeTag::from_wire(&node.node_tag)
        .and_then(|tag| CheckedNodeKind::decode(tag, &node.semantic_form))
        == Some(CheckedNodeKind::Claim(ClaimForm::StateGraph))
}

/// What one binding of the body holds: a reference's target, or an empty
/// aggregate (`None`).
#[derive(Clone, Copy)]
enum Slot<'a> {
    Reference(&'a CheckedNodeId),
    Empty,
    Other,
}

/// The slot of the body binding `name`, and the literal text of `kind`.
/// The bindings' wire names are the one place the written text is read
/// (FR-064's string-edge): QSpec FR-475 names them `kind`, `over`, `from`
/// and `target`.
#[ix_trace_rs::string_edge]
fn binding<'a>(node: &'a CheckedSemanticNodeV2, name: &str) -> Option<&'a CheckedTerm> {
    let CheckedTerm::Aggregate { members } = &node.body else {
        return None;
    };
    members.iter().find_map(|member| match member {
        CheckedTerm::Binding { name: bound, value } if &**bound == name => Some(&**value),
        _ => None,
    })
}

fn slot<'a>(node: &'a CheckedSemanticNodeV2, name: &str) -> Option<Slot<'a>> {
    Some(match binding(node, name)? {
        CheckedTerm::Reference { target } => Slot::Reference(target),
        CheckedTerm::Aggregate { members } if members.is_empty() => Slot::Empty,
        _ => Slot::Other,
    })
}

#[ix_trace_rs::string_edge]
fn kind_of(node: &CheckedSemanticNodeV2) -> Option<StateGraphKind> {
    let CheckedTerm::Literal { value, .. } = binding(node, "kind")? else {
        return None;
    };
    match value.as_str()? {
        "possible" => Some(StateGraphKind::Possible),
        "always-possible" => Some(StateGraphKind::AlwaysPossible),
        "unique-path" => Some(StateGraphKind::UniquePath),
        _ => None,
    }
}

/// The structural key of the admitted wire node id `id`, through the
/// shared node codec.
pub(super) fn node_key(id: &CheckedNodeId) -> Option<NodeKey> {
    let value = serde_json::to_value(id).ok()?;
    qsl_foundation::node_wire::node_key_from_json(&value).ok()
}

fn reference(node: &CheckedSemanticNodeV2, name: &str) -> Result<Option<NodeKey>, InternalFault> {
    match slot(node, name) {
        Some(Slot::Reference(id)) => node_key(id).map(Some).ok_or(BODY_FAULT),
        Some(Slot::Empty) => Ok(None),
        Some(Slot::Other) | None => Err(BODY_FAULT),
    }
}

/// The claim declared by `node`'s `claim` occurrence `ordinal`.
fn root(
    node: &CheckedSemanticNodeV2,
    id: &NodeKey,
    ordinal: u64,
) -> Result<StateGraphRoot, InternalFault> {
    Ok(StateGraphRoot {
        occurrence: OccurrenceKey::new(
            id.clone(),
            Origin::new(
                Role::new(super::role_spelling(&CheckedOccurrenceRole::Claim)),
                ordinal,
            ),
        ),
        kind: kind_of(node).ok_or(BODY_FAULT)?,
        over: reference(node, "over")?,
        from: reference(node, "from")?,
        target: reference(node, "target")?.ok_or(BODY_FAULT)?,
    })
}

/// Every declaration of every state-graph claim of `package`: one
/// [`StateGraphRoot`] per `claim` occurrence, in graph order and then
/// occurrence order.
pub fn state_graph_roots(package: &CheckedPackageV2) -> Result<Vec<StateGraphRoot>, InternalFault> {
    let mut roots = Vec::new();
    for node in package
        .graph()
        .nodes
        .iter()
        .filter(|node| is_state_graph(node))
    {
        let id = node_key(&node.node_id).ok_or(NODE_ID_FAULT)?;
        for occurrence in node
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.role == CheckedOccurrenceRole::Claim)
        {
            roots.push(root(node, &id, occurrence.ordinal)?);
        }
    }
    Ok(roots)
}

/// Resolve the state-graph claim `triple` in `package`: the declaration it
/// names, or [`ClauseRootUnresolved`] at `triple` when it names no `claim`
/// occurrence of a `claim`/`state_graph` node of the package (FR-410
/// "Behavior"). A body the reader admitted but this reading cannot follow is
/// an [`InternalFault`] in the outer `Result`.
pub fn resolve_state_graph_root(
    package: &CheckedPackageV2,
    triple: &OccurrenceKey,
) -> Result<Result<StateGraphRoot, ClauseRootUnresolved>, InternalFault> {
    let unresolved = || ClauseRootUnresolved {
        root: triple.clone(),
    };
    let Some(node) = package
        .graph()
        .nodes
        .iter()
        .find(|node| node_key(&node.node_id).as_ref() == Some(triple.node()))
        .filter(|node| is_state_graph(node))
    else {
        return Ok(Err(unresolved()));
    };
    if super::occurrence_role(triple.origin().role().as_str()) != Some(CheckedOccurrenceRole::Claim)
    {
        return Ok(Err(unresolved()));
    }
    let Some(occurrence) = node.occurrences.iter().find(|occurrence| {
        occurrence.role == CheckedOccurrenceRole::Claim
            && occurrence.ordinal == triple.origin().ordinal()
    }) else {
        return Ok(Err(unresolved()));
    };
    root(node, triple.node(), occurrence.ordinal).map(Ok)
}
