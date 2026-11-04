// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-501: a temporal clause under
//! `quire.temporal.timestamped-event.finite-window/v1` evaluated over a
//! timestamped trace, with definite verdicts on a closed complete window.
//!
//! [`evaluate_finite_window`] reads a [`TimestampedTrace`]: positions in admitted
//! order, each with a stamp in ticks of the clock unit and an optional order
//! key, with the trace's progress (watermark, completeness, closures, lower
//! boundary). An operator's eligible positions are those whose stamp
//! distance from the anchor lies in its closed interval; quantification
//! ranges over them only and no synthetic atom is added after the window
//! closes. Each atom at an eligible position has a possibility set
//! ([`Valuation`]); the formula is lifted over the sets and over every
//! admissible continuation of an unestablished window, so the result is
//! `Holds` or `Violated` whenever the lifted result is a singleton, and
//! `Pending` or `Indeterminate` only for an obligation genuinely unresolved.

mod cover;
mod identity;
mod lift;
mod outcome;
mod progressed;
mod run;
mod trace;

pub use cover::{ClockCover, Containment};
pub use outcome::{
    Assessment, Basis, IndeterminateReason, Premises, ResultIdentity, SupportPair, Valuation,
    WindowEvaluation, WindowVerdict,
};
pub use progressed::{ProgressedRefusal, SampleProgress};
pub use run::{evaluate_finite_window, evaluate_finite_window_at, WindowCharge, WindowUnit};
pub use trace::{
    LowerBoundary, OrderKey, TimestampedObservation, TimestampedTrace, Watermark, WindowClock,
    WindowProgress,
};
