// SPDX-License-Identifier: AGPL-3.0-or-later
//! The layer-5 TemporalTrace evaluator (FR-327, FR-328, FR-329, FR-125):
//! the one definition of a checked temporal clause's truth on a closed
//! finite trace, a finite prefix and a fair lasso, which replay, monitors and
//! the model checker answer to (ADR-018 SM-1).
//!
//! [`evaluate`] settles a [`CheckedTemporalClause`] on a [`TemporalTrace`]:
//! it reads the clause's letter through a [`TemporalLetters`], visits the
//! formula with a work unit charged on the caller's meter for each visit,
//! and returns a [`TemporalOutcome`], or [`TemporalRefusal`] for a trace it
//! cannot read. [`evaluate_at`] reads the formula at one position, and
//! [`unrolling`] states the positions a lasso is unrolled to (ADR-018 SM-8).
//!
//! [`AggregateAtom`] is the other kind of atom a monitor reads: a window
//! aggregate over the positions before the current one (FR-194), computed
//! at atom evaluation as an exact function of the trace, so the temporal
//! evaluator receives a Boolean per position.
//!
//! The trace's observations and the `holds` conditions are the caller's:
//! the evaluator reaches them only through [`TemporalLetters`], the seam to
//! the one clause evaluator (FR-107).

mod engine;
mod input;
mod letters;
mod outcome;
mod plan;
mod position;
mod window;

pub use engine::{unrolling, Unrolling};
pub use input::TemporalTrace;
pub use letters::{AtomReading, LetterStop, TemporalLetters, UndefinedCause};
pub use outcome::{TemporalOutcome, TemporalRefusal, UndefinedEvaluation};
pub use position::{PositionRefusal, TemporalPosition};
pub use window::{
    AggregateAtom, AggregateKind, AggregateTerm, InsufficientData, PastWindow, TermRefusal,
    WindowLimitReached, WindowRefusal, WindowSource, WindowTrace, DEFAULT_MAX_WINDOW_VALUES,
    MAX_WINDOW_VALUES_SETTING,
};

use qsl_semantics::check::CheckedTemporalClause;
use quire_exact::Meter;

/// Evaluate `clause` on `trace` under the clause's profile (FR-327 for a
/// closed trace, FR-328 for a prefix, FR-329 for a lasso).
///
/// A clause under the infinite-trace profile that has a fairness set settles
/// [`TemporalOutcome::MissingFairnessPremise`] on any trace given as
/// observations, and evaluates nothing.
pub fn evaluate<L: TemporalLetters>(
    clause: &CheckedTemporalClause,
    trace: TemporalTrace,
    letters: &mut L,
    meter: &mut Meter,
) -> Result<TemporalOutcome, TemporalRefusal> {
    engine::run(clause, trace, engine::Goal::Clause, letters, meter)
}

/// Evaluate `clause`'s formula at `position` of `trace`, ignoring its
/// activation and fairness (ADR-018 SM-8; FR-125 "Reading a lasso"). A
/// lasso reaches every position by wrapping; a finite trace refuses a
/// position it does not represent. `Holds` and `Violated` are the formula's
/// value at `position`.
pub fn evaluate_at<L: TemporalLetters>(
    clause: &CheckedTemporalClause,
    trace: TemporalTrace,
    position: TemporalPosition,
    letters: &mut L,
    meter: &mut Meter,
) -> Result<TemporalOutcome, TemporalRefusal> {
    engine::run(clause, trace, engine::Goal::At(position), letters, meter)
}
