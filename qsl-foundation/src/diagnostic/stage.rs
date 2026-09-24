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
//! `LimitExceeded`'s catalog code (QSL-236): `stage_limit_exceeded`, catalog
//! revision `1-draft.6`, which this build now claims. [`LimitExceeded::catalog_code`]
//! pairs it with the kind's own `<kind>-exceeded` cause tag
//! ([`LimitKind::catalog_cause`]).

use super::{CatalogCode, CatalogCoded};

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

impl LimitKind {
    /// The catalog's own cause tag for this kind, exactly
    /// `stage_limit_exceeded/<kind>-exceeded` (QSL-236, catalog revision
    /// `1-draft.6`).
    pub const fn catalog_cause(self) -> &'static str {
        match self {
            Self::InputBytes => "input-bytes-exceeded",
            Self::NestingDepth => "nesting-depth-exceeded",
            Self::NodeCount => "node-count-exceeded",
            Self::WorkBudget => "work-budget-exceeded",
        }
    }
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

impl CatalogCoded for LimitExceeded {
    /// `stage_limit_exceeded/<kind>-exceeded` (QSL-236): the kind alone
    /// decides the cause; `configured_bound`/`actual` are carried by this
    /// value itself, not folded into the tag.
    fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("stage_limit_exceeded", self.kind.catalog_cause())
    }
}

#[cfg(test)]
mod tests {
    use super::{CatalogCoded, LimitExceeded, LimitKind};
    use crate::diagnostic::{category_of, CatalogCode, Category};

    /// QSL-236: every kind's `LimitExceeded` reports the catalog's
    /// `stage_limit_exceeded/<kind>-exceeded`, with the bound and actual
    /// counter this value was built with (not folded into the cause tag).
    #[test]
    fn limit_exceeded_reports_stage_limit_exceeded_per_kind() {
        let cases = [
            (LimitKind::InputBytes, "input-bytes-exceeded"),
            (LimitKind::NestingDepth, "nesting-depth-exceeded"),
            (LimitKind::NodeCount, "node-count-exceeded"),
            (LimitKind::WorkBudget, "work-budget-exceeded"),
        ];
        for (kind, cause) in cases {
            let exceeded = LimitExceeded::new(kind, 10, 11);
            let code = exceeded.catalog_code();
            assert_eq!(code, CatalogCode::new("stage_limit_exceeded", cause));
            assert_eq!(category_of(&code), Some(Category::Refusal));
            assert_eq!(exceeded.configured_bound(), 10);
            assert_eq!(exceeded.actual(), 11);
        }
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
