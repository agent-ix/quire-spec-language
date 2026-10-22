// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-146 (ADR-020 RE-2): the per-step simulation records a refinement
//! declaration's `requirements` hook writes beside its refinement record.
//!
//! A refinement with no hidden field, no history field and no `any` row
//! writes one [`StepObligation::Initial`] record and one
//! [`StepObligation::Step`] record per step row, each listing the concrete
//! package's invariant clauses as hypotheses. With a hidden field or an
//! `any` row it writes none: an `any` row has no per-step obligation, so the
//! other rows' records would not discharge the safety half, and a hidden
//! field or a history field makes `map` no function of the concrete step.
//!
//! A record's members are the refinement node, the obligation and the
//! hypotheses: QSV's canonical encoding of them is the record's content
//! identity (ADR-013 O-09, which CG hashes). Each record is carried with its
//! occurrence key (ADR-013 O-07): the refinement node, role `generated`, and
//! ordinal 0 for `Initial` or row position + 1 for a `Step`. The key is a
//! locator and not a member of the content identity.
//! The hypotheses are sorted ascending by node key and deduplicated, so the
//! order of the invariant clauses in the source does not change a record.
//!
//! A row is named by its zero-based position in the order QSpec FR-375 gives
//! the checked refinement node's `steps` array: ascending by the node-key
//! digest of the step's first named declaration (its concrete operation),
//! then by member name as UTF-8 bytes. The caller supplies each concrete
//! operation's node key; this module sorts. The records come out in that
//! order, so reordering rows in the source changes nothing.

use std::collections::BTreeSet;

use qsl_forms::StateClauseKind;
use qsl_foundation::source::provenance::OccurrenceKey;
use quire_exact::{NodeKey, Origin};
use quire_semantic_value::step_simulation::{StepObligation, StepRowRef, StepSimulationRecord};

use super::claims::wire;
use super::family::OccurrenceRole;
use super::{CheckedRefinement, CheckedStateClause, StepClass, StepTarget};
use crate::model::key::DeclarationKey;

/// Why a refinement's step rows have no [`StepRowRef`]s.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum StepRowRefusal {
    /// The refinement has more step rows than a [`StepRowRef`] names.
    #[error("a refinement's step rows outnumber the positions a step row reference names")]
    Overflow,
    /// A row of the refinement is not an operation row. FR-136 orders a
    /// protocol subject's rows by the node key of the declaration each names,
    /// and FR-383's minting of the protocol nodes' keys is not built, so a
    /// protocol subject's rows have no position yet.
    #[error("a protocol subject's step rows have no order yet: protocol node keys are not minted")]
    ProtocolSubject,
}

/// One per-step simulation record with its ADR-013 O-07 occurrence key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepSimulationEntry {
    occurrence: OccurrenceKey,
    record: StepSimulationRecord,
}

impl StepSimulationEntry {
    fn new(refinement: NodeKey, ordinal: u64, record: StepSimulationRecord) -> Self {
        let origin = Origin::new(OccurrenceRole::Generated.kernel_role(), ordinal);
        Self {
            occurrence: OccurrenceKey::new(wire(refinement), origin),
            record,
        }
    }

    /// The occurrence key: the refinement node, role `generated`, and
    /// ordinal 0 for the `Initial` record or the row's position + 1.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The QSV record.
    pub fn record(&self) -> &StepSimulationRecord {
        &self.record
    }
}

/// The per-step simulation records of `refinement`, keyed by its node
/// `refinement_key`, with `clauses` the declaring package's checked state
/// clauses: those that are invariants of a concrete-model object type are
/// the hypotheses, sorted by node key.
/// The records name rows by position in FR-375's order; [`step_row_refs`]
/// maps a source row to its position.
///
/// Empty when the refinement has a hidden field, a history field or an `any`
/// row.
///
/// # Errors
///
/// [`StepRowRefusal::Overflow`] when a row's position exceeds `u32`;
/// [`StepRowRefusal::ProtocolSubject`] when a row is not an operation row.
pub fn step_simulation_records(
    refinement_key: NodeKey,
    refinement: &CheckedRefinement,
    clauses: &[CheckedStateClause],
) -> Result<Vec<StepSimulationEntry>, StepRowRefusal> {
    operation_rows(refinement)?;
    let has_hidden = refinement
        .objects()
        .iter()
        .any(|object| !object.hidden.is_empty());
    let has_any = refinement
        .steps()
        .iter()
        .any(|row| matches!(row.target, StepTarget::Any));
    if has_hidden || has_any || !refinement.history().is_empty() {
        return Ok(Vec::new());
    }
    let hypotheses: Vec<NodeKey> = clauses
        .iter()
        .filter(|clause| {
            clause.kind() == StateClauseKind::Invariant
                && refinement
                    .concrete_object_types()
                    .contains(&clause.context())
        })
        .map(CheckedStateClause::identity)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut records = vec![StepSimulationEntry::new(
        refinement_key,
        0,
        StepSimulationRecord::new(refinement_key, StepObligation::Initial, hypotheses.clone()),
    )];
    for index in 0..refinement.steps().len() {
        let position = u32::try_from(index).map_err(|_| StepRowRefusal::Overflow)?;
        records.push(StepSimulationEntry::new(
            refinement_key,
            u64::from(position) + 1,
            StepSimulationRecord::new(
                refinement_key,
                StepObligation::Step {
                    row: StepRowRef::new(position),
                },
                hypotheses.clone(),
            ),
        ));
    }
    Ok(records)
}

/// The [`StepRowRef`] of each step row of `refinement`, indexed by the row's
/// position in [`CheckedRefinement::steps`] (source order) and ascending in
/// FR-375's order: by the node key of the row's concrete operation, then by
/// the operation's name as UTF-8 bytes.
///
/// # Errors
///
/// [`StepRowRefusal::Overflow`] when a row's position exceeds `u32`;
/// [`StepRowRefusal::ProtocolSubject`] when a row is not an operation row.
pub fn step_row_refs(
    refinement: &CheckedRefinement,
    operation_key: impl Fn(&DeclarationKey) -> NodeKey,
) -> Result<Vec<StepRowRef>, StepRowRefusal> {
    let mut order: Vec<(NodeKey, &str, usize)> = operation_rows(refinement)?
        .into_iter()
        .enumerate()
        .map(|(source, operation)| (operation_key(operation), operation.node.as_str(), source))
        .collect();
    order.sort_by(|left, right| {
        left.0
            .as_bytes()
            .cmp(right.0.as_bytes())
            .then_with(|| left.1.as_bytes().cmp(right.1.as_bytes()))
    });
    let mut refs = vec![StepRowRef::new(0); order.len()];
    for (position, (_, _, source)) in order.into_iter().enumerate() {
        refs[source] =
            StepRowRef::new(u32::try_from(position).map_err(|_| StepRowRefusal::Overflow)?);
    }
    Ok(refs)
}

/// The concrete operation of each step row of `refinement`, in source order.
fn operation_rows(refinement: &CheckedRefinement) -> Result<Vec<&DeclarationKey>, StepRowRefusal> {
    refinement
        .steps()
        .iter()
        .map(|row| match &row.class {
            StepClass::Operation(operation) => Ok(operation),
            _ => Err(StepRowRefusal::ProtocolSubject),
        })
        .collect()
}
