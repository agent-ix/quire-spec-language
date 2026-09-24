// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 T-4: a stage's structured outcome, `Result<Staged<T>,
//! StageFailure<C>>`, and the stage limit a `StageFailure::Limit` names.
//! These live in the foundation `diagnostic` module (T-4). They are
//! distinct from the kernel `Outcome<T>`/`Incomplete`, which only
//! evaluation (S6a) returns.
//!
//! No type here carries a `Locus` yet. T-4 asks `LimitExceeded` to name the
//! `Locus` where the limit was reached, but no current producer reaches a
//! limit at a position it can turn into one (Remaining work: QSL-233).
//! `LimitExceeded` has no catalog code yet: its code, `stage_limit_exceeded`,
//! is catalog revision `1-draft.6`, and this build claims `1-draft.3`
//! (Remaining work: QSL-236).

/// ADR-013 T-4's closed limit kind: the four stage-entry limits a compiler
/// stage, the I2 reader, a family `check`, `replay` or `route` can reach.
///
/// Distinct from `quire_exact::LimitKind`, which names the evaluation
/// meter's counters.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LimitKind {
    /// Input bytes, such as a declaration's preimage byte length.
    InputBytes,
    /// Nesting depth.
    NestingDepth,
    /// Node count, such as a declaration's expression node count.
    NodeCount,
    /// Work budget: a stage's cumulative work units.
    WorkBudget,
}

/// ADR-013 T-4: a stage limit was reached. It is a stage outcome of its
/// own, never a refusal of the input, a checked result or `Incomplete`.
///
/// It names the limit kind, the configured bound and the actual counter.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LimitExceeded {
    kind: LimitKind,
    configured_bound: u64,
    actual: u128,
}

impl LimitExceeded {
    /// A reached limit of `kind`, configured at `configured_bound`, where
    /// the stage's counter reached `actual`.
    ///
    /// `actual` is the value the refused step would have taken the counter
    /// to: the measured size for input bytes and node count, the level the
    /// refused entry would have reached for nesting depth, and the
    /// cumulative total the refused charge would have reached for the work
    /// budget. It is wider than the bound because a cumulative total of two
    /// `u64` counters can exceed `u64::MAX`.
    pub const fn new(kind: LimitKind, configured_bound: u64, actual: u128) -> Self {
        Self {
            kind,
            configured_bound,
            actual,
        }
    }

    /// The limit that was reached.
    pub const fn kind(&self) -> LimitKind {
        self.kind
    }

    /// The configured bound of that limit.
    pub const fn configured_bound(&self) -> u64 {
        self.configured_bound
    }

    /// The counter value the refused step would have reached.
    pub const fn actual(&self) -> u128 {
        self.actual
    }
}

/// ADR-013 T-4: a stage's successful output.
///
/// T-4 also gives `Staged<T>` the warnings the stage raised. No stage
/// raises one yet, so the field waits for its first producer (QSL-162).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Staged<T> {
    value: T,
}

impl<T> Staged<T> {
    /// A stage's output.
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    /// The stage's output.
    pub fn into_value(self) -> T {
        self.value
    }
}

/// ADR-013 T-4: how a stage fails without producing output.
///
/// T-4's shape also has `Fault(InternalFault)` and gives `Refused` a list of
/// causes plus diagnostics. Neither is here: no stage returning this type
/// raises an internal fault, T-4 does not define the diagnostics' type, and
/// a family `check` refuses one form with one cause (QSL-148).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StageFailure<C> {
    /// A configured stage limit was reached first.
    Limit(LimitExceeded),
    /// The stage refused its input with its own typed cause.
    Refused(C),
}
