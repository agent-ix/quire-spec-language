// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-043/FR-044: the orthogonal dimensions a temporal report names.
//!
//! Both progress/closure axes, assessment execution and input completeness are
//! independent. Closing one axis never closes or completes another, and no
//! refusal is reported as a Boolean.

/// Independent progress/closure state of one axis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Closure {
    /// The axis has not yet progressed to its final state.
    Open,
    /// The axis has reached its final state; nothing further can revise it.
    Closed,
}

/// Independent assessment-execution disposition, separate from truth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Execution {
    /// The assessment ran to completion.
    Completed,
    /// The assessment could not run to completion.
    Failed,
}

/// Independent input completeness, separate from both closure axes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Completeness {
    /// Every required input was supplied.
    Complete,
    /// At least one required input was absent.
    Incomplete,
}

/// A located refusal. No refusal is ever reported as a temporal Boolean.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
    /// A valuation-evidence row failed one of the checks QSL makes against
    /// values it holds (FR-043-AC-16). No truth and no partial support follow.
    #[error("temporal valuation row {row} refused: {cause:?}")]
    Valuation {
        /// The check the row failed.
        cause: super::valuation::ValuationCause,
        /// The row's index in admission order.
        row: usize,
    },
}
