// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-113: the `ProtocolClause` family's S3 scope check over one protocol's
//! `ScopedAnchorForm`s (ADR-012 §12.2 Check row).
//!
//! Each anchor's first segment resolves in the innermost scope of its
//! `scope`, then in each enclosing scope outward, then at the top level; a
//! later segment resolves among the names its previous target declares
//! directly. A scope that declares two static nodes of one name refuses
//! `ambiguous_declaration`/`ambiguous-name` at each declaration, and an
//! anchor whose resolution decides on that name refuses the same way at the
//! anchor. A resolved anchor is checked against the admitted target kinds
//! of its site (FR-113 "Behavior" table); a mismatch refuses
//! `ill_typed`/`type-mismatch`, naming the site, the admitted kinds and the
//! actual kind. A `receive` whose channel disagrees with the `send` its
//! `receive-of` anchor names refuses the same way, naming both channels.
//!
//! FR-113's binder no-shadowing rule (a record binder, a capture, a
//! compensation trigger or a retry or recovery parameter shadowing a
//! model/profile alias, a native declaration of the package or another
//! visible binder) is checked here too, over the `BinderForm`s
//! the S2 extension (`qsl_forms::protocol_clause::BinderForm`)
//! now builds for every binder position. QSpec `shared-grammar.md` makes
//! every binder "unique in their enclosing declaration": that declaration
//! is the checked protocol as a whole, so every binder name in it shares
//! one flat namespace, independent of the lexical scope it is written in
//! (SR-765 FND-001) -- a `finish` binder can shadow a `run`-tree binder,
//! two different `compensate` declarations' binders of one name collide,
//! and so do sibling `case`/`branch` binders of one name, matching the
//! composed lane's own `DuplicateBinder` checker
//! (`src/linking/composed/scopes.rs`). Only a *different* protocol
//! declaration is a separate enclosing declaration ("Two separate
//! protocols may reuse a binder name"), which this function's own
//! per-protocol call already gives for free. Binders are still checked
//! against a package-level alias or native declaration the same way (by
//! name alone, package-wide). Only one refusal is raised, at the shadowing
//! binder, naming what it shadows -- unlike a duplicate declaration,
//! shadowing is directional, so the shadowed declaration is not itself
//! refused.

mod anchors;
mod attempts;
mod awaits;
mod binders;
mod coverage;
mod graph;
mod types;

#[cfg(test)]
mod tests;

pub use awaits::Selections;
pub use coverage::content;
pub(crate) use graph::dominates;
pub use types::{
    BoundAttempt, CausalEdge, CheckedAttempt, CheckedAwait, CheckedControl, CheckedControlTree,
    CheckedOperationNodes, CheckedProtocol, EdgeKind, Port, PortRef, ProtocolNodeId,
};

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{BinderForm, BinderKind, ProtocolDeclarationForm};
use qsl_foundation::Span;

use super::protocol_obligation::CheckedObligations;
use super::refusal::{AliasKind, CheckCause, CheckRefusal, ProtocolAnchorCause};
use super::state_clause::{AttemptDeclaration, StateClauseDeclaration};
use super::{Scope, Signatures};
use anchors::{check_kind, resolve, scope_key, ScopeKey};
use attempts::bind_attempts;
use binders::shadow_refusals;
use quire_semantic_value::location::Origin;

fn refusal(cause: ProtocolAnchorCause) -> CheckRefusal {
    CheckRefusal {
        // A protocol has no expression tree for a `Location` path to walk;
        // `region::DeclarationRegions::refusal_region` reads a
        // `ProtocolAnchor` cause's own span directly instead, so this
        // `Location` is never resolved.
        location: quire_semantic_value::location::Location::root(Origin::Expression),
        cause: CheckCause::ProtocolAnchor(Box::new(cause)),
    }
}

/// FR-113: the protocol node `protocol`'s scoped anchors resolve to, and
/// FR-114: each `attempt`'s own `contracts` list checked against its
/// already-resolved operation -- or the refusals below, in source
/// position order (declaration-order duplicate refusals first at equal
/// span, then binder-shadow refusals, then per-anchor refusals, then
/// per-contract refusals). `alias_names` (the unit's own `profile`/`model`
/// selection aliases), `native_names` (its `dimension` and `unit`
/// declaration names), `scope` (the package's other native type
/// declarations: `type`, `record`, `tuple` and `enum`) and `signatures`
/// (its native function declarations) are the only package-wide state this
/// checker reads, for FR-113 "Refusals" binder no-shadowing rule;
/// `attempts` (the assembler's own per-attempt operation resolution) and
/// `state_clauses` (the unit's own checked state clauses, for FR-114's
/// `contracts` check) are read for FR-114 alone; every other check is over
/// the protocol's own form. Once every anchor resolves, the `run` control
/// tree and its compiled causal edge relation are built (FR-218), and a
/// cycle without the `repeat` back edges, or a `repeat` whose body does not
/// necessarily step, is refused. Each `await` is checked too: its awaited
/// event's form, its `within` window and the unit's `temporal_profile`
/// selection (`selections`; the protocol profile's identity is what the
/// awaited-form and progress refusals name). Returns the checked
/// protocol (its `attempts` empty: S4/lowering fills it in) alongside the
/// S3-level `BoundAttempt`s `check::mod`'s own pipeline lowers next.
#[allow(clippy::too_many_arguments)] // each input is a distinct package table the check reads
pub fn check(
    protocol: &ProtocolDeclarationForm,
    alias_names: &BTreeMap<String, AliasKind>,
    native_names: &BTreeSet<String>,
    scope: &Scope,
    signatures: &Signatures,
    attempts: &[AttemptDeclaration],
    state_clauses: &[StateClauseDeclaration],
    selections: Selections<'_>,
) -> Result<(CheckedProtocol, Vec<BoundAttempt>), Vec<CheckRefusal>> {
    let mut refusals: Vec<(Span, CheckRefusal)> = Vec::new();

    // Every static node by (its scope, its name): the declaration
    // collection FR-113 Inputs asks S2 for. A scope with two or more
    // entries of one name is refused up front, independent of whether any
    // anchor names it (FR-113 "Refusals").
    let mut by_scope_name: BTreeMap<(ScopeKey, String), Vec<usize>> = BTreeMap::new();
    for (index, declaration) in protocol.declarations.iter().enumerate() {
        by_scope_name
            .entry((
                scope_key(protocol, declaration.scope),
                declaration.name.name.clone(),
            ))
            .or_default()
            .push(index);
    }
    for ((_, name), indices) in &by_scope_name {
        if indices.len() < 2 {
            continue;
        }
        let loci: Vec<Span> = indices
            .iter()
            .map(|&index| protocol.declarations[index].name.span)
            .collect();
        for &index in indices {
            let span = protocol.declarations[index].name.span;
            refusals.push((
                span,
                refusal(ProtocolAnchorCause::Ambiguous {
                    name: name.clone(),
                    loci: loci.clone(),
                    span,
                }),
            ));
        }
    }

    refusals.extend(shadow_refusals(
        protocol,
        alias_names,
        native_names,
        scope,
        signatures,
    ));

    let mut resolved = Vec::with_capacity(protocol.scoped_anchors.len());
    for anchor in &protocol.scoped_anchors {
        match resolve(protocol, &by_scope_name, anchor) {
            Ok(target) => {
                if let Some(cause) = check_kind(protocol, anchor, target) {
                    refusals.push((anchor.anchor.span, refusal(cause)));
                } else {
                    resolved.push(target);
                }
            }
            Err(cause) => refusals.push((anchor.anchor.span, refusal(cause))),
        }
    }

    let (bound_attempts, contract_refusals) =
        bind_attempts(protocol, attempts, state_clauses, scope.types());
    refusals.extend(contract_refusals);

    let awaits = match awaits::check(protocol, selections) {
        Ok(awaits) => awaits,
        Err(await_refusals) => {
            refusals.extend(await_refusals);
            Vec::new()
        }
    };

    let compiled = if refusals.is_empty() {
        match graph::compile(protocol, &resolved, selections.protocol) {
            Ok(compiled) => Some(compiled),
            Err(graph_refusals) => {
                refusals.extend(graph_refusals);
                None
            }
        }
    } else {
        None
    };

    let Some(compiled) = compiled.filter(|_| refusals.is_empty()) else {
        // FR-113 "Outputs": refusals in source position order. `sort_by_key`
        // is stable, so refusals at equal spans (the up-front duplicate
        // pass, which emits one per declaration at that declaration's own
        // span) keep their relative order.
        refusals.sort_by_key(|(span, _)| span.start);
        return Err(refusals.into_iter().map(|(_, refusal)| refusal).collect());
    };

    Ok((
        CheckedProtocol {
            name: protocol.name.name.clone(),
            resolved_anchors: resolved,
            attempts: Vec::new(),
            obligations: CheckedObligations::default(),
            channels: Vec::new(),
            parallels: Vec::new(),
            fences: Vec::new(),
            controls: compiled.tree,
            edges: compiled.edges,
            slots: crate::check::protocol_slots::CheckedSlots::default(),
            compensation_operations: Vec::new(),
            awaits,
            requirements: Vec::new(),
            relationships: Vec::new(),
        },
        bound_attempts,
    ))
}

/// The `as (x: T)` record binder of the node `declaration` declares: the
/// first record binder written after the node's name. Every node that has
/// one writes it before any other binder.
pub(crate) fn record_binder(
    protocol: &ProtocolDeclarationForm,
    declaration: usize,
) -> Option<&BinderForm> {
    let declaration = protocol.declarations.get(declaration)?;
    protocol
        .binders
        .iter()
        .filter(|binder| {
            binder.kind == BinderKind::RecordBinder
                && binder.name.span.start >= declaration.name.span.end
        })
        .min_by_key(|binder| binder.name.span.start)
}
