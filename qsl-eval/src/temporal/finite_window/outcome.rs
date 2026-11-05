// SPDX-License-Identifier: AGPL-3.0-or-later
//! What the finite-window evaluator returns (FR-501 "Outputs", "Result
//! report").

use qsl_semantics::check::FormulaNodeId;
use quire_exact::Incomplete;

use super::cover::ClockCover;
use super::trace::LowerBoundary;
use crate::temporal::ledger::{ProgressClosure, ProgressCompleteness};
use crate::temporal::letters::{AtomReading, UndefinedCause};
use crate::temporal::outcome::UndefinedEvaluation;
use crate::temporal::position::TemporalPosition;

/// The valuation of one atom at one position, as a letter reads it
/// (QSpec FR-160 possibility sets).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Valuation {
    /// The atom is true: a singleton `{true}`.
    True,
    /// The atom is false: a singleton `{false}`.
    False,
    /// The valuation is absent from the admitted observation: both
    /// possibilities.
    Missing,
    /// The atom is undefined at the position (FR-327).
    Undefined(UndefinedCause),
}

impl From<AtomReading> for Valuation {
    fn from(reading: AtomReading) -> Self {
        match reading {
            AtomReading::True => Self::True,
            AtomReading::False => Self::False,
            AtomReading::Undefined(cause) => Self::Undefined(cause),
        }
    }
}

/// Why a closed window's verdict stays open (FR-501 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IndeterminateReason {
    /// An atom's valuation is absent at an eligible position.
    MissingValuation {
        /// The position.
        position: TemporalPosition,
        /// The atom.
        atom: FormulaNodeId,
    },
    /// Two differing valuations of an atom were admitted at one position.
    ConflictingValuation {
        /// The position.
        position: TemporalPosition,
        /// The atom.
        atom: FormulaNodeId,
    },
    /// The completeness assertion stops short of the window.
    IncompleteInput,
    /// A past operator's window reaches before a trace whose lower boundary
    /// is a history cutoff.
    MissingHistory,
    /// A counter cover straddles an interval's bound.
    StraddlesBound,
}

/// The settlement of a clause on a timestamped trace (FR-501 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WindowVerdict {
    /// `Completed(true)`.
    Holds,
    /// `Completed(false)` at the first activation that evaluated false.
    Violated {
        /// The activation's position.
        at: TemporalPosition,
    },
    /// An activation whose verdict declared progress could still resolve.
    Pending {
        /// The activation's anchor position.
        at: TemporalPosition,
        /// The first tick whose establishment would resolve it.
        deadline: u64,
    },
    /// An event-position activation whose verdict only a further observation
    /// can resolve (FR-504): its deadline is `NextEvent`, because event
    /// position does not advance in silence.
    PendingNextEvent {
        /// The activation's anchor position.
        at: TemporalPosition,
    },
    /// The window is closed and the evidence still leaves the verdict open.
    Indeterminate {
        /// The activation's anchor position.
        at: TemporalPosition,
        /// Why.
        reason: IndeterminateReason,
    },
    /// An atom is undefined (FR-327).
    Undefined(UndefinedEvaluation),
    /// The work budget ran out; no truth value (FR-327 "Work").
    Incomplete(Incomplete),
    /// An aggregate window exceeded `monitor.max_window_values`.
    WindowLimitReached(crate::temporal::window::WindowLimitReached),
    /// An `on each` guard cannot be established at `at`: an incomplete
    /// activation, not a false guard (FR-044-AC-7); no truth value.
    ActivationIncomplete {
        /// The position whose guard is unresolved.
        at: TemporalPosition,
    },
}

/// What a verdict rests on (FR-501 "Result report").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Basis {
    /// The activation is established and the decision scope is `Closed`.
    ClosedScope,
    /// A definite true that every admissible continuation preserves.
    DecisiveWitness,
    /// A definite false that every admissible continuation preserves.
    DecisiveCounterexample,
    /// A `Pending` result.
    Unsettled,
    /// An `Indeterminate` result, or one with no truth value.
    Unavailable,
}

/// Whether the evaluator produced an outcome (FR-501 "Result report").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Assessment {
    /// The evaluator produced an outcome.
    Completed,
    /// The evaluator returned `Incomplete` or a refusal.
    Failed,
}

/// One `(position, atom)` pair of a decision support.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SupportPair {
    /// The position.
    pub position: TemporalPosition,
    /// The atom.
    pub atom: FormulaNodeId,
}

/// The four independent dimensions and the inputs that qualify the verdict
/// (FR-501 "Result report").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Premises {
    /// The decision scope's closure.
    pub scope: ProgressClosure,
    /// The surrounding execution's closure.
    pub execution: ProgressClosure,
    /// The assessment-execution disposition.
    pub assessment: Assessment,
    /// The input completeness.
    pub completeness: ProgressCompleteness,
    /// The watermark as an inclusive tick, `-1` when progress covers none.
    pub watermark: i128,
    /// The lower-boundary kind.
    pub lower_boundary: LowerBoundary,
    /// The clock cover.
    pub cover: ClockCover,
    /// The clock unit.
    pub unit: String,
}

/// A content identity over the inputs of an evaluation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResultIdentity(pub String);

/// The report of an evaluation that is not a refusal (FR-501 "Result
/// report"): the verdict and, as separate fields no one of which determines
/// another, its basis, decision support, premises and identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowEvaluation {
    /// The verdict.
    pub verdict: WindowVerdict,
    /// The basis.
    pub basis: Basis,
    /// The decision support, in position then atom order.
    pub support: Vec<SupportPair>,
    /// The premises.
    pub premises: Premises,
    /// The result identity.
    pub identity: ResultIdentity,
    /// The `temporal.visit` charges the evaluation made.
    pub visits: u64,
}
