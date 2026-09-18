// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bridges one FR-151 dispatch family from its normalized model into the
//! value checker (TC-196 D06-D08).
//!
//! [`crate::model::dispatch::link_dispatch`] links a dispatch family by
//! [`ProducerKey`] alone; it has no `Expression` to check and builds no
//! call-graph index (its own module docs record both as out of scope: it is
//! link-time linking only). This module is the other half: for one
//! dispatch-eligible root operation, it types every linked candidate's
//! effective precondition and body against
//! [`crate::value::expression::PackageDeclarations`], and translates
//! `link_dispatch`'s [`ProducerKey`]-keyed
//! [`crate::model::dispatch::DispatchTable`] into the checked layer's
//! [`NodeKey`]- and function-index-keyed
//! [`crate::value::expression::DispatchTable`], ready for
//! [`PackageDeclarations::check`](crate::value::expression::PackageDeclarations)
//! and the evaluator.
//!
//! Pure: no intake, no I/O. It takes a caller-supplied [`Bundle`] and an
//! [`OperationClauses`] side table naming each candidate's own clause
//! `Expression` and signature. `Bundle` carries no `Expression` payload —
//! adding one would perturb the producer-interface-1.3.0 correspondence
//! every other rung of this crate keys against — so this is where a real
//! intake (`agent-ix/quire-specification#131`) will eventually plug in its
//! own clause source in place of a test-built side table; nothing else here
//! needs to change when it does.
//!
//! Scope, kept to exactly what TC-196 D06-D08 need (re-read this note before
//! extending it):
//!
//! - One dispatch-eligible operation at a time, mirroring `link_dispatch`'s
//!   own per-operation signature. A package with several dispatch call sites
//!   calls this once per distinct receiver-type/member pair and merges the
//!   resulting `functions`/`dispatch_operations`/`dispatch_tables`.
//! - `receiver_type` is supplied by the caller as the exact [`NodeKey`] the
//!   checker requires of a dispatched call's receiver. This bridge does not
//!   derive it from `OperationMemberRecord::owner` and does not expose a
//!   dispatch-eligible operation through every ancestor static type that
//!   would also admit the call. TC-196 D06-D08 only ever dispatch through an
//!   operation's own declared owner type, so this narrower scope covers
//!   them; broadening it to inherited exposure is future work, not a gap
//!   this bridge silently papers over.
//! - Parameter and result types come from [`OperationClauses`], not from a
//!   translation of `OperationMemberRecord`'s own producer-interface
//!   parameter/result records: turning a model-layer type reference into a
//!   checker [`ValueType`] is its own, separately-scoped piece of work
//!   (needed well beyond dispatch), so this bridge accepts it pre-translated
//!   exactly as `link_dispatch` accepts `bundle`/`view` pre-normalized.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::accounting::Meter;
use crate::model::bundle::{Bundle, BundleRecord, RedefinitionRecord};
use crate::model::dispatch::{
    link_dispatch, DispatchLinkOutcome, GeneralizationClosure, LinkCheckOutcome,
};
use crate::model::key::ProducerKey;
use crate::model::normalize::EffectiveView;
use crate::value::{
    BinaryOperator, ClauseKind, DispatchCandidate, DispatchOperation, DispatchTable, Expression,
    FunctionDeclaration, NodeKey, PackageDeclarations, ValueType,
};

/// Bounds the effective-precondition ancestor walk. Mirrors
/// `crate::model::dispatch::MAX_DISPATCH_DEPTH`'s own style; declared
/// separately here since that constant is private to its module.
const MAX_ANCESTOR_DEPTH: usize = 128;

/// One dispatch candidate's own clause `Expression`s and signature, supplied
/// by the caller since [`Bundle`] carries no `Expression` payload. Every
/// operation this bridge is asked to check — the root operation and every
/// one of its linked candidates — needs an entry.
#[derive(Clone, Debug, Default)]
pub struct OperationClauses {
    /// The unqualified member name. Only the *root* operation's entry is
    /// consulted for this field.
    pub member: BTreeMap<ProducerKey, String>,
    /// Declared parameters, receiver first: element 0 is always the
    /// receiver (`self`) parameter every candidate's own
    /// [`FunctionDeclaration`] needs at slot 0, matching how the evaluator's
    /// `NodeKind::Dispatch` binds `[receiver, ..arguments]` positionally into
    /// a linked candidate's frame. [`DispatchOperation::parameters`] (the
    /// call site's own argument types) is everything after that receiver
    /// element; this bridge strips it when building the call site.
    pub parameters: BTreeMap<ProducerKey, Vec<(String, ValueType)>>,
    /// The declared result type.
    pub result: BTreeMap<ProducerKey, ValueType>,
    /// This operation's own precondition clause, when it declares one.
    pub own_precondition: BTreeMap<ProducerKey, Expression>,
    /// This operation's own body clause. Required for every linked
    /// candidate (an operation without a body is never a candidate;
    /// `link_dispatch` already filters those out).
    pub own_body: BTreeMap<ProducerKey, Expression>,
}

/// Why [`checked_dispatch_operation`] could not produce a checked
/// [`PackageDeclarations`].
#[derive(Clone, Debug)]
pub enum DispatchBridgeRefusal {
    /// `link_dispatch` did not produce a linked table: ambiguous, refused,
    /// incomplete, or an open generalization closure. Boxed: `LinkCheckOutcome`
    /// is far larger than the other variant, and this refusal is returned by
    /// value from every fallible step below.
    Unlinked(Box<LinkCheckOutcome>),
    /// `clauses` (or `object_keys`) has no entry for this operation under
    /// the named field. `operation` is boxed: [`ProducerKey`] itself is
    /// larger than the rest of this refusal put together.
    MissingClauseData {
        /// The operation (or subtype) missing an entry.
        operation: Box<ProducerKey>,
        /// The field that was missing it.
        field: &'static str,
    },
}

fn missing(operation: &ProducerKey, field: &'static str) -> DispatchBridgeRefusal {
    DispatchBridgeRefusal::MissingClauseData {
        operation: Box::new(operation.clone()),
        field,
    }
}

fn require_expression(
    map: &BTreeMap<ProducerKey, Expression>,
    operation: &ProducerKey,
    field: &'static str,
) -> Result<Expression, DispatchBridgeRefusal> {
    map.get(operation)
        .cloned()
        .ok_or_else(|| missing(operation, field))
}

fn require_signature(
    clauses: &OperationClauses,
    operation: &ProducerKey,
) -> Result<(Vec<(String, ValueType)>, ValueType), DispatchBridgeRefusal> {
    let parameters = clauses
        .parameters
        .get(operation)
        .cloned()
        .ok_or_else(|| missing(operation, "parameters"))?;
    let result = clauses
        .result
        .get(operation)
        .cloned()
        .ok_or_else(|| missing(operation, "result"))?;
    Ok((parameters, result))
}

/// The FR-151 dispatch-eligible operation [`checked_dispatch_operation`] is
/// asked to check: its own key, the receiver type its dispatch call sites
/// require, and whether the generalization closure is known closed.
/// Grouped into one value to keep that function's own argument count small.
pub struct DispatchRoot {
    /// The root operation's own original producer key.
    pub key: ProducerKey,
    /// The exact static type the checker requires of a dispatched call's
    /// receiver. See the module docs: this bridge does not derive it from
    /// `OperationMemberRecord::owner`.
    pub receiver_type: NodeKey,
    /// Whether the generalization closure relevant to this operation is
    /// known closed.
    pub closure: GeneralizationClosure,
}

/// Types every linked candidate's effective precondition and body for one
/// FR-151 dispatch-eligible operation, and assembles the checked-layer
/// [`DispatchTable`] the evaluator needs. See the module docs for this
/// bridge's exact scope.
pub fn checked_dispatch_operation(
    bundle: &Bundle,
    view: &EffectiveView,
    root: &DispatchRoot,
    object_keys: &BTreeMap<ProducerKey, NodeKey>,
    clauses: &OperationClauses,
    meter: &mut Meter,
) -> Result<PackageDeclarations, DispatchBridgeRefusal> {
    let outcome = link_dispatch(bundle, view, &root.key, root.closure, meter);
    let table = match outcome {
        LinkCheckOutcome::Completed(DispatchLinkOutcome::Linked(table)) => table,
        other => return Err(DispatchBridgeRefusal::Unlinked(Box::new(other))),
    };

    let redefinitions: Vec<RedefinitionRecord> = bundle
        .records
        .iter()
        .filter_map(|record| match record {
            BundleRecord::Redefinition(redefinition) => Some(redefinition.clone()),
            _ => None,
        })
        .collect();

    let mut distinct: BTreeSet<ProducerKey> = BTreeSet::new();
    for (_, candidate) in table.entries() {
        distinct.insert(candidate.clone());
    }

    let mut functions: Vec<FunctionDeclaration> = Vec::new();
    let mut body_index: BTreeMap<ProducerKey, usize> = BTreeMap::new();
    let mut precondition_index: BTreeMap<ProducerKey, usize> = BTreeMap::new();

    for candidate in &distinct {
        let (parameters, result) = require_signature(clauses, candidate)?;
        if let Some(effective) = effective_precondition(&redefinitions, clauses, candidate) {
            let index = functions.len();
            functions.push(FunctionDeclaration {
                name: format!("{}.precondition", candidate.identity),
                parameters: parameters.clone(),
                result: ValueType::Boolean,
                measure: None,
                body: effective,
                clause_kind: ClauseKind::Precondition,
            });
            precondition_index.insert(candidate.clone(), index);
        }
        let body = require_expression(&clauses.own_body, candidate, "own_body")?;
        let index = functions.len();
        functions.push(FunctionDeclaration {
            name: candidate.identity.clone(),
            parameters,
            result,
            measure: None,
            body,
            clause_kind: ClauseKind::Body,
        });
        body_index.insert(candidate.clone(), index);
    }

    let mut entries = Vec::with_capacity(table.entries().len());
    for (subtype, candidate) in table.entries() {
        let subtype_key = object_keys
            .get(subtype)
            .copied()
            .ok_or_else(|| missing(subtype, "object_keys"))?;
        let Some(&body) = body_index.get(candidate) else {
            return Err(missing(candidate, "own_body"));
        };
        let precondition = precondition_index.get(candidate).copied();
        entries.push((subtype_key, DispatchCandidate { body, precondition }));
    }
    let candidate_count = distinct.len() as u64;
    let checked_table = DispatchTable::new(entries, candidate_count);

    let member = clauses
        .member
        .get(&root.key)
        .cloned()
        .ok_or_else(|| missing(&root.key, "member"))?;
    let (parameters, result) = require_signature(clauses, &root.key)?;
    // `parameters` is receiver-first (see `OperationClauses::parameters`);
    // the call site's own argument types are everything after it.
    let call_arguments = parameters.get(1..).unwrap_or_default().to_vec();
    let dispatch_operations = vec![DispatchOperation {
        receiver_type: root.receiver_type,
        member,
        parameters: call_arguments
            .into_iter()
            .map(|(_, value_type)| value_type)
            .collect(),
        result,
        table: 0,
    }];

    Ok(PackageDeclarations {
        functions,
        dispatch_operations,
        dispatch_tables: vec![checked_table],
        ..PackageDeclarations::default()
    })
}

/// The disjunction of every declared precondition in `candidate`'s
/// redefinition ancestry, nearest first: `candidate`'s own precondition (if
/// it declares one) `or` its nearest ancestor's `or` ... `None` only when no
/// member in the whole chain declares one (TC-196 D08: a redefinition with
/// no own precondition inherits its ancestor's, disjoined). Bounded task
/// walk, mirroring `crate::model::dispatch::build_family`'s own style.
fn effective_precondition(
    redefinitions: &[RedefinitionRecord],
    clauses: &OperationClauses,
    candidate: &ProducerKey,
) -> Option<Expression> {
    let mut chain: Vec<Expression> = Vec::new();
    let mut current = candidate.clone();
    let mut visited: BTreeSet<ProducerKey> = BTreeSet::new();
    visited.insert(current.clone());
    let mut steps: usize = 0;
    loop {
        if let Some(expression) = clauses.own_precondition.get(&current) {
            chain.push(expression.clone());
        }
        steps += 1;
        if steps > MAX_ANCESTOR_DEPTH {
            break;
        }
        let parent = redefinitions
            .iter()
            .find(|redefinition| redefinition.redefining == current)
            .map(|redefinition| redefinition.redefined.clone());
        match parent {
            Some(next) if visited.insert(next.clone()) => current = next,
            _ => break,
        }
    }
    chain.into_iter().reduce(|left, right| Expression::Binary {
        operator: BinaryOperator::Or,
        left: Box::new(left),
        right: Box::new(right),
    })
}
