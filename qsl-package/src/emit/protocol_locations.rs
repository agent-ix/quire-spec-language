// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-383 "Protocol term locations": one `checked_term_data.protocol_locations`
//! row for every term of every protocol slot expression the emitter writes.
//!
//! Each slot is walked in pre-order over its checked expression tree, the
//! children of a node taken in its `NodeKind` operand order. A position is a
//! term when the checker lowered an application node there: it then recorded
//! an `expression` occurrence of that node at the position's `Location`, and
//! the row is keyed to that occurrence and carries that `Location`. A
//! literal, a `Local` read, a `let`, a conditional and a `case` are not terms
//! (FR-383 assumption A1) and get no row. Nothing here derives a location
//! from a source region, a node order or an operand position.

use std::collections::{BTreeMap, HashMap, HashSet};

use qsl_semantics::check::{
    BodyTerm, CheckedActivation, CheckedBody, CheckedGraph, CheckedLifetime, CheckedNode,
    CheckedOrdering, CheckedProtocol, CheckedTerminalMember, LeafTerm, NodeTag,
};
use quire_checked::node::NodeKind;
use quire_contract_model::{
    CheckedKeyOrdinal, CheckedNodeId, CheckedOccurrence, CheckedProtocolLocation,
    CheckedProtocolLocationOrigin, CheckedProtocolSemanticLocation,
};
use quire_exact::NodeKey;
use quire_semantic_value::location::{Location, Origin};

use super::{checked_occurrence, EmitRefusal};

/// The largest integer a JSON number carries exactly.
const MAX_SAFE: u64 = 9_007_199_254_740_991;

/// One row before its `term_ordinal` is assigned.
pub(super) struct PendingRow {
    application: CheckedNodeId,
    occurrence: CheckedOccurrence,
    /// The origin `name`, which orders the origins of one group.
    origin_name: String,
    /// The visit's place in the emitter's walk of every slot, which orders
    /// the rows of one origin by visit.
    visit: usize,
    location: CheckedProtocolLocation,
}

/// Every `protocol_locations` row of the emitted protocols, in the
/// (application, occurrence, `term_ordinal`) order QSpec FR-322-AC-291
/// requires.
pub(super) fn checked_protocol_locations(
    graph: &CheckedGraph,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<Vec<CheckedProtocolSemanticLocation>, EmitRefusal> {
    let mut slots = Vec::new();
    let mut roots = HashSet::new();
    for protocol in graph.protocols() {
        // A refused protocol writes no node, so none of its slots is
        // emitted (FR-503).
        let emitted = graph
            .protocol_node_keys(&protocol.name)
            .and_then(|keys| keys.clause.as_ref())
            .is_some_and(|clause| emitted_node_ids.contains_key(clause));
        if !emitted {
            continue;
        }
        for body in slot_bodies(protocol) {
            if roots.insert(body.root().location()) {
                slots.push(body);
            }
        }
    }
    let origins: HashSet<&Origin> = roots.iter().map(|root| &root.origin).collect();
    let applications = applications_at(graph, &origins);
    let mut pending = Vec::new();
    for body in slots {
        slot_rows(graph, body, &applications, emitted_node_ids, &mut pending)?;
    }
    Ok(ordered(pending))
}

/// The rows of one slot's walk, appended to `pending` in visit order.
pub(super) fn slot_rows(
    graph: &CheckedGraph,
    body: &CheckedBody,
    applications: &HashMap<&Location, Vec<(NodeKey, quire_exact::Origin)>>,
    emitted_node_ids: &HashMap<NodeKey, CheckedNodeId>,
    pending: &mut Vec<PendingRow>,
) -> Result<(), EmitRefusal> {
    let positions = walk(body);
    // The application each position names, so a position below it at
    // the same location (an implicit coercion's operand) names another.
    let mut claimed: Vec<Option<&NodeKey>> = Vec::with_capacity(positions.len());
    let mut visits: HashMap<(&Location, &NodeKey), usize> = HashMap::new();
    for &(node, parent) in &positions {
        let at = node.location();
        let mut taken = Vec::new();
        let mut above = parent;
        while let Some(index) = above {
            let Some(&(ancestor, next)) = positions.get(index) else {
                break;
            };
            if ancestor.location() != at {
                break;
            }
            taken.extend(claimed.get(index).copied().flatten());
            above = next;
        }
        let application = match applications.get(at) {
            Some(candidates) => term_application(graph, candidates, &taken)?,
            None => None,
        };
        claimed.push(application);
        if !is_term_kind(node) {
            continue;
        }
        let Some(application) = application else {
            // FR-383 writes one row per term. A kind that always lowers
            // to an application yet has none recorded at its position
            // leaves no application to key the row to: refuse rather
            // than drop the row.
            if always_application(node) {
                return Err(EmitRefusal::UnrecordedProtocolTerm {
                    location: at.clone(),
                });
            }
            continue;
        };
        // The `j`th visit of one position takes the `j`th occurrence
        // recorded there; a position recorded once keeps that one.
        let recorded: Vec<&quire_exact::Origin> = applications
            .get(at)
            .into_iter()
            .flatten()
            .filter(|(key, _)| key == application)
            .map(|(_, origin)| origin)
            .collect();
        let seen = visits.entry((at, application)).or_default();
        let origin = recorded
            .get(*seen)
            .or_else(|| recorded.last())
            .copied()
            .ok_or_else(|| refusal(application))?;
        *seen += 1;
        let (origin_name, location) = wire_location(at).ok_or_else(|| refusal(application))?;
        pending.push(PendingRow {
            application: emitted_node_ids
                .get(application)
                .cloned()
                .ok_or_else(|| refusal(application))?,
            occurrence: checked_occurrence(origin).ok_or_else(|| refusal(application))?,
            origin_name,
            visit: pending.len(),
            location,
        });
    }
    Ok(())
}

fn refusal(application: &NodeKey) -> EmitRefusal {
    EmitRefusal::MissingProtocolLocationContext {
        application: application.clone(),
    }
}

/// Number each group's rows from zero across its origins, the origins in
/// origin order and one origin's rows in visit order, then order every row
/// by (application, occurrence, `term_ordinal`).
fn ordered(pending: Vec<PendingRow>) -> Vec<CheckedProtocolSemanticLocation> {
    let mut groups: BTreeMap<(CheckedNodeId, CheckedOccurrence), Vec<PendingRow>> = BTreeMap::new();
    for row in pending {
        groups
            .entry((row.application.clone(), row.occurrence.clone()))
            .or_default()
            .push(row);
    }
    let mut rows = Vec::new();
    for (_, mut group) in groups {
        group.sort_by(|left, right| {
            left.origin_name
                .as_bytes()
                .cmp(right.origin_name.as_bytes())
                .then(left.visit.cmp(&right.visit))
        });
        rows.extend(group.into_iter().zip(0_u64..).map(|(row, term_ordinal)| {
            CheckedProtocolSemanticLocation {
                application: row.application,
                occurrence: row.occurrence,
                term_ordinal,
                location: row.location,
            }
        }));
    }
    rows
}

/// Every slot expression of `protocol` (FR-383's site-name registry). A
/// `repeat`'s loop proof is not a slot (FR-390) and is not listed.
pub(super) fn slot_bodies(protocol: &CheckedProtocol) -> Vec<&CheckedBody> {
    let mut bodies = Vec::new();
    if let CheckedActivation::OnEach {
        guard: Some(guard), ..
    } = &protocol.obligations.activation
    {
        bodies.push(guard);
    }
    bodies.extend(
        protocol
            .obligations
            .captures
            .iter()
            .map(|capture| &capture.initializer),
    );
    for compensation in &protocol.obligations.compensations {
        bodies.push(&compensation.guard);
        bodies.push(&compensation.retry.relation);
        bodies.push(&compensation.recover.predicate);
    }
    bodies.extend(protocol.slots.bodies.iter().map(|body| &body.body));
    for choice in &protocol.slots.choices {
        bodies.extend(&choice.visible);
        bodies.extend(choice.cases.iter().map(|case| &case.guard));
    }
    for repeat in &protocol.slots.repeats {
        bodies.extend(&repeat.visible);
        bodies.push(&repeat.guard);
    }
    for related in &protocol.slots.related {
        bodies.extend(&related.ends);
    }
    if let CheckedTerminalMember::When { predicate, .. } = &protocol.slots.terminal {
        bodies.push(predicate);
    }
    for role in &protocol.slots.roles {
        if let CheckedLifetime::Until { predicate, .. } = &role.lifetime {
            bodies.push(predicate);
        }
    }
    for channel in &protocol.channels {
        if let CheckedOrdering::Fifo { key, .. } = &channel.ordering {
            bodies.push(key);
        }
    }
    bodies
}

/// The positions of `body` in pre-order, children in operand order, a node
/// reached through two parents once per parent, each with the index of its
/// parent position.
fn walk(body: &CheckedBody) -> Vec<(CheckedNode<'_>, Option<usize>)> {
    let mut visited = Vec::new();
    let mut stack = vec![(body.root(), None)];
    while let Some((node, parent)) = stack.pop() {
        let index = visited.len();
        visited.push((node, parent));
        stack.extend(
            node.children()
                .into_iter()
                .rev()
                .map(|child| (child, Some(index))),
        );
    }
    visited
}

/// Whether a position can be a term: a literal, a `Local` read, a `let`, a
/// conditional and a `case` never are (FR-383 assumption A1), even where
/// lowering keys a node for them.
pub(super) fn is_term_kind(node: CheckedNode<'_>) -> bool {
    !matches!(
        node.kind(),
        NodeKind::Literal(_)
            | NodeKind::Local(_)
            | NodeKind::Let { .. }
            | NodeKind::If { .. }
            | NodeKind::Case { .. }
    )
}

/// Whether a position's kind always lowers to an application node, so a
/// position of it with no recorded application is a lowering gap rather than
/// a term the checker folded away.
fn always_application(node: CheckedNode<'_>) -> bool {
    matches!(
        node.kind(),
        NodeKind::Coerce(..)
            | NodeKind::Arithmetic(..)
            | NodeKind::Order(..)
            | NodeKind::Equality(..)
            | NodeKind::Connective(..)
            | NodeKind::Not(_)
            | NodeKind::Field { .. }
            | NodeKind::Attribute { .. }
            | NodeKind::Call { .. }
    )
}

/// Each `expression` occurrence of an application node located under one of
/// `origins`, by location, ascending by node and then ordinal.
fn applications_at<'g>(
    graph: &'g CheckedGraph,
    origins: &HashSet<&Origin>,
) -> HashMap<&'g Location, Vec<(NodeKey, quire_exact::Origin)>> {
    let mut at: HashMap<&'g Location, Vec<(NodeKey, quire_exact::Origin)>> = HashMap::new();
    for (key, origin, location) in graph.occurrences() {
        if origin.role().as_str() != "expression" || !origins.contains(&location.origin) {
            continue;
        }
        let application = graph
            .semantic_graph()
            .node(key.clone())
            .is_some_and(|node| {
                node.node_tag() == NodeTag::Expression
                    && matches!(node.body(), BodyTerm::Application(_))
            });
        if application {
            at.entry(location).or_default().push((key, origin));
        }
    }
    at
}

/// The application a position names: of the applications lowered at its
/// location and not `taken` by an enclosing position at the same location,
/// the one no other of them takes as an operand. An attribute read lowers a
/// `deref` and the `project` over it at one position; an implicit coercion
/// and its operand are two positions at one location.
fn term_application<'c>(
    graph: &CheckedGraph,
    candidates: &'c [(NodeKey, quire_exact::Origin)],
    taken: &[&NodeKey],
) -> Result<Option<&'c NodeKey>, EmitRefusal> {
    let mut keys: Vec<&NodeKey> = Vec::new();
    for (key, _) in candidates {
        if !keys.contains(&key) && !taken.contains(&key) {
            keys.push(key);
        }
    }
    let operands: HashSet<&NodeKey> = keys
        .iter()
        .filter_map(|key| graph.semantic_graph().node((*key).clone()))
        .flat_map(|node| node.body().leaves())
        .filter_map(|leaf| match leaf {
            LeafTerm::Reference { target } => target.admitted(),
            LeafTerm::Literal { .. } | LeafTerm::DependencyReference { .. } => None,
        })
        .collect();
    let mut outer = keys.iter().filter(|key| !operands.contains(**key));
    match (outer.next(), outer.next()) {
        (Some(key), None) => Ok(Some(key)),
        (None, None) if keys.is_empty() => Ok(None),
        // Two outermost applications, or a cycle, at one position: the
        // lowering left no single term there.
        (Some(key), Some(_)) => Err(refusal(key)),
        (None, _) => Err(keys
            .first()
            .map_or(EmitRefusal::SourceMapInvariant, |key| refusal(key))),
    }
}

/// The checker's `Location` of a term as the wire carries it, with the
/// slot's site name: every slot expression is located under a
/// type-declaration origin named by its site (FR-383), never under a
/// `protocol_attempt` origin.
fn wire_location(location: &Location) -> Option<(String, CheckedProtocolLocation)> {
    let name = match &location.origin {
        Origin::TypeDeclaration { name } => name.clone(),
        Origin::Body { .. }
        | Origin::Measure { .. }
        | Origin::Expression
        | Origin::StateClause { .. }
        | Origin::ProtocolAttempt { .. }
        | Origin::ModelOperationClause { .. } => return None,
    };
    let path = location
        .path()
        .into_iter()
        .map(|index| {
            let index = u64::try_from(index).ok()?;
            Some(if index <= MAX_SAFE {
                CheckedKeyOrdinal::Safe(index)
            } else {
                CheckedKeyOrdinal::BeyondSafe(index.to_string().into())
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let origin = CheckedProtocolLocationOrigin::TypeDeclaration {
        name: name.as_str().into(),
    };
    Some((name, CheckedProtocolLocation { origin, path }))
}
