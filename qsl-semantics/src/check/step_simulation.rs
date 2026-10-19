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
//! field makes `map` no function of the visible state. A history row is
//! refused before a [`CheckedRefinement`] exists (FR-138 is not checked
//! yet), so a checked refinement never has a history field here.
//!
//! A record's identity binds the refinement node, the obligation and the
//! hypotheses (ADR-013 O-09): they are the record's own members, and its
//! canonical bytes (`quire-canonical`, `Encode`) are its identity preimage.
//!
//! A row is named by its zero-based position in
//! [`CheckedRefinement::steps`], which is declaration order. The spec does
//! not say that order is stable (SR-13116); this module assumes it is.

use qsl_forms::StateClauseKind;
use quire_exact::NodeKey;
use quire_semantic_value::step_simulation::{StepObligation, StepRowRef, StepSimulationRecord};

use super::{CheckedRefinement, CheckedStateClause, StepTarget};

/// Why a refinement has more step rows than a [`StepRowRef`] names.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("a refinement's step rows outnumber the positions a step row reference names")]
pub struct StepRowOverflow;

/// The per-step simulation records of `refinement`, keyed by its node
/// `refinement_key`, with `clauses` the concrete package's checked state
/// clauses: those that are invariants of a concrete object type are the
/// hypotheses, in the order given.
///
/// Empty when the refinement has a hidden field or an `any` row.
///
/// # Errors
///
/// [`StepRowOverflow`] when a row's position exceeds `u32`.
pub fn step_simulation_records(
    refinement_key: NodeKey,
    refinement: &CheckedRefinement,
    clauses: &[CheckedStateClause],
) -> Result<Vec<StepSimulationRecord>, StepRowOverflow> {
    let has_hidden = refinement
        .objects()
        .iter()
        .any(|object| !object.hidden.is_empty());
    let has_any = refinement
        .steps()
        .iter()
        .any(|row| matches!(row.target, StepTarget::Any));
    if has_hidden || has_any {
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
        .collect();
    let mut records = vec![StepSimulationRecord::new(
        refinement_key,
        StepObligation::Initial,
        hypotheses.clone(),
    )];
    for index in 0..refinement.steps().len() {
        let row = StepRowRef::new(u32::try_from(index).map_err(|_| StepRowOverflow)?);
        records.push(StepSimulationRecord::new(
            refinement_key,
            StepObligation::Step { row },
            hypotheses.clone(),
        ));
    }
    Ok(records)
}
