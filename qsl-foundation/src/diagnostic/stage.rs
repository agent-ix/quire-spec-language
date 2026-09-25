// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 T-4: a stage's structured outcome, `Result<Staged<T>,
//! StageFailure<C>>`, and the stage limit a `StageFailure::Limit` names.
//! These live in the foundation `diagnostic` module (T-4). They are
//! distinct from the kernel `Outcome<T>`/`Incomplete`, which only
//! evaluation (S6a) returns.
//!
//! `LimitExceeded`'s catalog code is `stage_limit_exceeded` with the kind's
//! own `<kind>-exceeded` cause ([`LimitKind::catalog_cause`]), under
//! `quire.native.diagnostics/v1` revision `1-draft.7`, which this build
//! claims. It carries the T-5 [`Locus`] where the limit was reached, absent
//! only where FR-096 says no producer can know one.

use super::{CatalogCode, CatalogCoded, Locus};

/// ADR-013 T-4's closed limit kind: one variant per
/// `stage_limit_exceeded` cause of `quire.native.diagnostics/v1` revision
/// `1-draft.7`. The catalog row names which S1 or I2 limit carries each of
/// the four `1-draft.7` kinds.
///
/// Distinct from `quire_exact::LimitKind`, which names the evaluation
/// meter's counters.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LimitKind {
    /// Input bytes, such as a declaration's preimage byte length or the I2
    /// reader's wire bytes.
    InputBytes,
    /// Nesting depth.
    NestingDepth,
    /// Token count: S1's token ceiling (a retained CST leaf under complete
    /// V1).
    TokenCount,
    /// Node count, such as a declaration's expression node count or the I2
    /// reader's semantic graph nodes.
    NodeCount,
    /// Edge count: the I2 reader's graph dependency edges.
    EdgeCount,
    /// Occurrence count: the I2 reader's combined semantic occurrences and
    /// source-map entries.
    OccurrenceCount,
    /// Diagnostic count: the I2 reader's diagnostic entries.
    DiagnosticCount,
    /// Work budget: a stage's cumulative work units.
    WorkBudget,
}

impl LimitKind {
    /// Every kind, in the catalog row's order.
    pub const ALL: [Self; 8] = [
        Self::InputBytes,
        Self::NestingDepth,
        Self::TokenCount,
        Self::NodeCount,
        Self::EdgeCount,
        Self::OccurrenceCount,
        Self::DiagnosticCount,
        Self::WorkBudget,
    ];

    /// The catalog's own cause tag for this kind, exactly
    /// `stage_limit_exceeded/<kind>-exceeded` (revision `1-draft.7`).
    pub const fn catalog_cause(self) -> &'static str {
        match self {
            Self::InputBytes => "input-bytes-exceeded",
            Self::NestingDepth => "nesting-depth-exceeded",
            Self::TokenCount => "token-count-exceeded",
            Self::NodeCount => "node-count-exceeded",
            Self::EdgeCount => "edge-count-exceeded",
            Self::OccurrenceCount => "occurrence-count-exceeded",
            Self::DiagnosticCount => "diagnostic-count-exceeded",
            Self::WorkBudget => "work-budget-exceeded",
        }
    }
}

/// ADR-013 T-4: a stage limit was reached. It is a stage outcome of its
/// own, never a refusal of the input, a checked result or `Incomplete`.
///
/// It names the limit kind, the configured bound, the actual counter and
/// the [`Locus`] where the charge failed (FR-096). The locus is absent only
/// where no producer can know one: a position in a tree not read from a
/// source unit, the I2 reader's own artifact byte ceiling, and an IR limit
/// IR reports no position for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LimitExceeded {
    kind: LimitKind,
    configured_bound: u64,
    actual: u128,
    locus: Option<Locus>,
}

impl LimitExceeded {
    /// A reached limit of `kind`, configured at `configured_bound`, where
    /// the stage's counter reached `actual`, with no locus yet.
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
            locus: None,
        }
    }

    /// This limit, reached at `locus` (`None` where FR-096 says no
    /// producer can know one).
    #[must_use]
    pub fn at(mut self, locus: Option<Locus>) -> Self {
        self.locus = locus;
        self
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

    /// Where the charge failed, when a producer can know it.
    pub fn locus(&self) -> Option<&Locus> {
        self.locus.as_ref()
    }
}

impl CatalogCoded for LimitExceeded {
    /// `stage_limit_exceeded/<kind>-exceeded`: the kind alone decides the
    /// cause; `configured_bound`/`actual` are carried by this value itself,
    /// not folded into the tag.
    fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("stage_limit_exceeded", self.kind.catalog_cause())
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::{CatalogCoded, LimitExceeded, LimitKind};
    use crate::diagnostic::{category_of, CatalogCode, Category};

    /// FR-096-AC-2 at catalog revision `1-draft.7`: each of the eight kinds
    /// reports `stage_limit_exceeded` with its own cause, and a
    /// `LimitExceeded` reports its kind's code with the bound and actual
    /// counter.
    #[trace("TC-427", "FR-096-AC-2")]
    #[test]
    fn limit_exceeded_reports_stage_limit_exceeded_per_kind() {
        let causes = [
            "input-bytes-exceeded",
            "nesting-depth-exceeded",
            "token-count-exceeded",
            "node-count-exceeded",
            "edge-count-exceeded",
            "occurrence-count-exceeded",
            "diagnostic-count-exceeded",
            "work-budget-exceeded",
        ];
        for (kind, cause) in LimitKind::ALL.into_iter().zip(causes) {
            let exceeded = LimitExceeded::new(kind, 10, 11);
            let code = exceeded.catalog_code();
            assert_eq!(code, CatalogCode::new("stage_limit_exceeded", cause));
            assert_eq!(category_of(&code), Some(Category::Refusal));
            assert_eq!(exceeded.configured_bound(), 10);
            assert_eq!(exceeded.actual(), 11);
            assert_eq!(exceeded.locus(), None);
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
