// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-488 "Clauses S4 does not emit": the checked hyper clauses over
//! `behaviours` that no package encoding exists for, refused
//! `unsupported_construct`/`not-yet-implemented` at the construct in the
//! order the requirement states.
//!
//! The refusals are ordered by precedence, and a clause stops at the first:
//! a timed clause, an alias that binds no object parameter, a fairness set
//! under a bounded profile, a constraint declared by a systems interface, and
//! a model read with no encodable component.

use quire_checked::node::NodeKind;
use std::collections::BTreeMap;

use qsl_foundation::Span;
use quire_semantic_value::location::Location;

use super::{CheckedHyperClause, TemporalCause};
use crate::check::ir::{CheckedNode, Slot};
use crate::check::profile::TemporalProfile;
use crate::check::refusal::{CheckCause, CheckRefusal};
use crate::check::AdmittedModel;
use quire_semantic_value::location::Origin;

/// The refusal of `clause` at `span`, or the first reason no package
/// encodes it.
fn refusal(clause: &CheckedHyperClause, span: Span) -> CheckRefusal {
    CheckRefusal {
        location: Location::root(Origin::Expression),
        cause: CheckCause::Temporal(Box::new(TemporalCause::NotEmitted {
            name: clause.name().to_owned(),
            span,
        })),
    }
}

/// The refusal S4 gives `clause` when it has no encoding, or `None` when
/// the clause is emitted (FR-488 "Clauses S4 does not emit").
pub(crate) fn unemitted(
    clause: &CheckedHyperClause,
    models: &[AdmittedModel],
) -> Option<CheckRefusal> {
    // 1. A timed clause, at the clause.
    if clause.profile() == TemporalProfile::Timed {
        return Some(refusal(clause, clause.span()));
    }
    // 2. An alias that binds no object parameter: its subject node is
    // undefined.
    if clause.aliases().iter().any(|alias| {
        !clause
            .parameters()
            .iter()
            .any(|parameter| &parameter.alias == alias)
    }) {
        return Some(refusal(clause, clause.span()));
    }
    // 3. A fairness set under a profile whose fairness argument is empty.
    if clause.profile().is_bounded() {
        if let Some(span) = constraints(clause).next().map(|(_, span, _)| span) {
            return Some(refusal(clause, span));
        }
    }
    // 4. A constraint whose declaring node is a systems interface.
    for (constraint, span, alias) in constraints(clause) {
        let declaring = constraint.operation().declaring_type();
        let interface = models
            .iter()
            .filter(|model| model.alias() == Some(alias))
            .any(|model| model.is_systems_interface(declaring.clone()));
        if interface {
            return Some(refusal(clause, span));
        }
    }
    // 5. A model read with no encodable component.
    unencodable_read(clause).map(|span| refusal(clause, span))
}

/// Every fairness constraint of the clause in quantifier order, with the
/// span of the constraint it was written as and the quantifier's alias.
fn constraints(
    clause: &CheckedHyperClause,
) -> impl Iterator<
    Item = (
        &quire_semantic_value::fairness::FairnessConstraint,
        Span,
        &str,
    ),
> {
    clause
        .quantifiers()
        .iter()
        .zip(clause.fairness_spans())
        .flat_map(|(quantifier, spans)| {
            quantifier
                .fairness
                .iter()
                .zip(spans)
                .map(|(constraint, span)| (constraint, *span, quantifier.alias.as_str()))
        })
}

/// The span of the first model read that no package term encodes: the first
/// in atom arena order, then source order.
fn unencodable_read(clause: &CheckedHyperClause) -> Option<Span> {
    let slots = clause.atom_slots();
    for atom in clause.atom_conditions().values() {
        let root = atom.root();
        let nodes = root.descendants();
        let mut bound: BTreeMap<Slot, CheckedNode<'_>> = BTreeMap::new();
        for node in &nodes {
            match node.kind() {
                NodeKind::Let { slot, value, .. } => {
                    bound.insert(*slot, node.at(*value));
                }
                NodeKind::Query { slot, source, .. } => {
                    bound.insert(*slot, node.at(*source));
                }
                NodeKind::Fold { binder, source, .. } => {
                    bound.insert(*binder, node.at(*source));
                }
                _ => {}
            }
        }
        let mut refused: Vec<Span> = Vec::new();
        for node in &nodes {
            let operands = match node.kind() {
                NodeKind::Attribute { reference, .. }
                | NodeKind::History { reference, .. }
                | NodeKind::Navigate { reference, .. }
                | NodeKind::Lookup { reference, .. } => vec![*reference],
                NodeKind::Reaches { source, target, .. }
                | NodeKind::ReachesEnd { source, target, .. } => vec![*source, *target],
                NodeKind::AllInstances { .. } => Vec::new(),
                _ => continue,
            };
            let Some(component) = atom.components.get(node.location()) else {
                continue;
            };
            let traces = roots(
                operands.into_iter().map(|id| node.at(id)),
                &bound,
                slots.len(),
            );
            let encodable = !traces.is_empty()
                && traces
                    .iter()
                    .all(|slot| slots.get(*slot).map(|slot| slot.trace) == Some(*component));
            if !encodable {
                if let Some(span) = atom.read_spans.get(node.location()) {
                    refused.push(*span);
                }
            }
        }
        if let Some(first) = refused.into_iter().min_by_key(|span| span.start) {
            return Some(first);
        }
    }
    None
}

/// The atom slots in the subtrees of `operands`, a `let`-bound name replaced
/// by its initializer's subtree.
fn roots<'n>(
    operands: impl Iterator<Item = CheckedNode<'n>>,
    bound: &BTreeMap<Slot, CheckedNode<'n>>,
    slot_count: usize,
) -> Vec<Slot> {
    let mut found: Vec<Slot> = Vec::new();
    let mut pending: Vec<CheckedNode<'n>> = operands.collect();
    let mut expanded: Vec<Slot> = Vec::new();
    while let Some(node) = pending.pop() {
        if let NodeKind::Local(slot) = node.kind() {
            if *slot < slot_count {
                if !found.contains(slot) {
                    found.push(*slot);
                }
            } else if let Some(initializer) = bound.get(slot) {
                if !expanded.contains(slot) {
                    expanded.push(*slot);
                    pending.push(*initializer);
                }
            }
            continue;
        }
        pending.extend(node.children());
    }
    found
}
