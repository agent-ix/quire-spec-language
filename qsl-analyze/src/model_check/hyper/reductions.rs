// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-174 "Preservation rows" (ADR-023 PH-1 to PH-6, ADR-021 EI-1): which of
//! the reductions a request selects EN-1 admits for a hyper item, settled
//! before any state is expanded.
//!
//! A reduction applies exactly where ADR-023 §5's row for the item's form
//! reads "yes". A row that reads "no" settles the item V-6
//! `ReductionNotPreserving`; a row that reads "yes" for a reduction this
//! engine does not offer for a hyper item settles it V-8 `unsupported`, as
//! ADR-021 RV-6 does. Copy-swap is not selected, so it has no place here: the
//! compiler applies it when the clause has the symmetry.

use qsl_replay::{HyperReductionCause, ReductionKind};
use qsl_semantics::check::{CheckedHyperClause, HyperForm};
use quire_semantic_value::fairness::FairnessGranularity;

use crate::model_check::Reductions;

/// The forms EN-1 decides here, with the rows of ADR-023 §5 that read their
/// subjects.
#[derive(Clone, Copy)]
enum Row {
    /// PH-2: HP-2.
    Universal,
    /// PH-6: HP-3.
    ForallExistsSafety,
    /// PH-4: HP-6.
    Projected,
}

impl Row {
    fn of(clause: &CheckedHyperClause) -> Option<Self> {
        match clause.form() {
            Some(HyperForm::Universal) => Some(Self::Universal),
            Some(HyperForm::ForallExistsSafety) => Some(Self::ForallExistsSafety),
            Some(HyperForm::ProjectionAligned) => Some(Self::Projected),
            _ => None,
        }
    }
}

/// Whether ADR-023 §5's row reads "yes" for `reduction` and `clause`.
fn preserves(reduction: ReductionKind, row: Row, clause: &CheckedHyperClause) -> bool {
    let one_subject = clause.aliases().len() == 1;
    match (reduction, row) {
        // Diagonal key symmetry needs one universe to act on, and every
        // fairness constraint `whole` for HP-2 (PH-2, PH-3, PH-6).
        (ReductionKind::Symmetry, Row::Universal) => {
            one_subject
                && clause.quantifiers().iter().all(|quantifier| {
                    quantifier
                        .fairness
                        .iter()
                        .all(|constraint| constraint.granularity() == FairnessGranularity::Whole)
                })
        }
        (ReductionKind::Symmetry, Row::ForallExistsSafety | Row::Projected) => one_subject,
        // Lockstep pairs positions across traces, so no hyper form is
        // invariant under reordering independent steps (PH-2, PH-4, PH-6).
        (ReductionKind::PartialOrder, _) => false,
        // A cut state of HP-3 removes a witness, so a refutation would not be
        // real (PH-6); HP-2's lasso stays real (PH-2).
        (ReductionKind::StateConstraint, Row::Universal | Row::Projected) => true,
        (ReductionKind::StateConstraint, Row::ForallExistsSafety) => false,
    }
}

/// The cause that settles the item before any expansion, when the request
/// selects a reduction EN-1 does not admit for `clause`: the first selected
/// reduction, in the order symmetry, partial-order, constraint, whose row
/// reads "no"; failing that, the first selected reduction the engine does not
/// offer for a hyper item. `None` when the request selects none, or the clause
/// is not one this engine decides.
pub(super) fn refusal(
    reductions: &Reductions,
    clause: &CheckedHyperClause,
) -> Option<HyperReductionCause> {
    let row = Row::of(clause)?;
    let selected = reductions.selected();
    if let Some(reduction) = selected
        .iter()
        .copied()
        .find(|reduction| !preserves(*reduction, row, clause))
    {
        return Some(HyperReductionCause::NotPreserving {
            reduction,
            form: clause.form().cloned()?,
        });
    }
    selected
        .first()
        .map(|reduction| HyperReductionCause::Unsupported {
            reduction: *reduction,
        })
}
