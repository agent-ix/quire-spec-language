// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-214 "Outputs": the observation of a refinement's step row.
//!
//! `observation` reads a row's target and nothing else, so a protocol on the
//! abstract side (ADR-027 PR-2) reads the same row the model case does. The
//! observation is computed on demand and stored nowhere.

use super::{StepRow, StepTarget};
use crate::model::key::DeclarationKey;

/// The abstract step a visible row maps its concrete step to (ADR-027 PR-2's
/// observation label): the abstract operation and the binder record the
/// row's arguments give.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepLabel {
    /// The abstract operation.
    pub operation: DeclarationKey,
    /// The positions of the arguments the row leaves open with `_`, in
    /// ascending order.
    pub open_arguments: Vec<usize>,
    /// Whether the row binds the abstract result to the concrete one.
    pub binds_result: bool,
}

/// What a concrete step is to the abstract side (ADR-027 PR-2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RowObservation {
    /// A row to an abstract operation: the step passes when an abstract step
    /// with this label is enabled.
    Visible {
        /// The abstract step's label.
        label: StepLabel,
    },
    /// A `stutter` row: the mapped visible fields do not change.
    Internal,
    /// An `any` row: either of the two.
    Either,
}

/// The observation of `row`, from its target.
pub fn observation(row: &StepRow) -> RowObservation {
    match &row.target {
        StepTarget::Abstract {
            operation,
            arguments,
            binds_result,
            ..
        } => RowObservation::Visible {
            label: StepLabel {
                operation: operation.clone(),
                open_arguments: arguments
                    .iter()
                    .enumerate()
                    .filter_map(|(position, argument)| argument.is_none().then_some(position))
                    .collect(),
                binds_result: *binds_result,
            },
        },
        StepTarget::Stutter => RowObservation::Internal,
        StepTarget::Any => RowObservation::Either,
    }
}
