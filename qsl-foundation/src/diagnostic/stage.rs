// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 T-4: a stage's structured outcome, `Result<Staged<T>,
//! StageFailure<C>>`, and the stage limit a `StageFailure::Limit` names.
//! These live in the foundation `diagnostic` module (T-4). They are
//! distinct from the kernel `Outcome<T>`/`Incomplete`, which only
//! evaluation (S6a) returns.
//!
//! `LimitExceeded`'s catalog code is `stage_limit_exceeded` with the kind's
//! own `<kind>-exceeded` cause ([`LimitKind::catalog_cause`]), under
//! `quire.native.diagnostics/v1`. It carries the T-5 [`Locus`] where the limit was reached, absent
//! only where FR-096 says no producer can know one.

use quire_exact::CancelCause;

use super::{CatalogCode, CatalogCoded, Category, Code, InternalFault, Locus};
use crate::setting::Setting;

/// ADR-013 T-4's closed limit kind: one variant per
/// `stage_limit_exceeded` cause of `quire.native.diagnostics/v1`. Nesting depth is not a kind: no stage refuses on depth
/// (ADR-030).
///
/// Distinct from `quire_exact::LimitKind`, which names the evaluation
/// meter's counters.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LimitKind {
    /// Input bytes, such as a declaration's preimage byte length or the I2
    /// reader's wire bytes.
    InputBytes,
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
    pub const ALL: [Self; 7] = [
        Self::InputBytes,
        Self::TokenCount,
        Self::NodeCount,
        Self::EdgeCount,
        Self::OccurrenceCount,
        Self::DiagnosticCount,
        Self::WorkBudget,
    ];

    /// The catalog's own cause tag for this kind, exactly
    /// `stage_limit_exceeded/<kind>-exceeded`.
    pub const fn catalog_cause(self) -> &'static str {
        match self {
            Self::InputBytes => "input-bytes-exceeded",
            Self::TokenCount => "token-count-exceeded",
            Self::NodeCount => "node-count-exceeded",
            Self::EdgeCount => "edge-count-exceeded",
            Self::OccurrenceCount => "occurrence-count-exceeded",
            Self::DiagnosticCount => "diagnostic-count-exceeded",
            Self::WorkBudget => "work-budget-exceeded",
        }
    }

    /// The counter's name in a rendered limit diagnostic, such as `node` in
    /// "S3 node limit 4 reached (5)".
    pub const fn noun(self) -> &'static str {
        match self {
            Self::InputBytes => "input byte",
            Self::TokenCount => "token",
            Self::NodeCount => "node",
            Self::EdgeCount => "edge",
            Self::OccurrenceCount => "occurrence",
            Self::DiagnosticCount => "diagnostic",
            Self::WorkBudget => "work",
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
    setting: Setting,
    configured_bound: u64,
    actual: u128,
    /// Boxed: a `Locus` names a source reference or a digest and pointer,
    /// far larger than the rest, and every stage's `Result` carries this.
    locus: Option<Box<Locus>>,
}

impl LimitExceeded {
    /// A reached limit set by `setting` (FR-255), configured at
    /// `configured_bound`, where the stage's counter reached `actual`, with
    /// no locus yet. The limit kind is the setting's.
    ///
    /// `actual` is the value the refused step would have taken the counter
    /// to: the measured size for input bytes and node count, the edge count
    /// the refused step would have reached for edge count, and the
    /// cumulative total the refused charge would have reached for the work
    /// budget. It is wider than the bound because a cumulative total of two
    /// `u64` counters can exceed `u64::MAX`.
    pub const fn new(setting: Setting, configured_bound: u64, actual: u128) -> Self {
        Self {
            setting,
            configured_bound,
            actual,
            locus: None,
        }
    }

    /// The setting that raises this limit (FR-255), at every entry point.
    pub const fn setting(&self) -> Setting {
        self.setting
    }

    /// This limit, reached at `locus` (`None` where FR-096 says no
    /// producer can know one).
    #[must_use]
    pub fn at(mut self, locus: Option<Locus>) -> Self {
        self.locus = locus.map(Box::new);
        self
    }

    /// The limit that was reached.
    pub const fn kind(&self) -> LimitKind {
        self.setting.kind()
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
        self.locus.as_deref()
    }
}

impl CatalogCoded for LimitExceeded {
    /// `stage_limit_exceeded/<kind>-exceeded`: the kind alone decides the
    /// cause; `configured_bound`/`actual` are carried by this value itself,
    /// not folded into the tag.
    fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("stage_limit_exceeded", self.kind().catalog_cause())
    }

    /// The catalog row's payload: the exceeded limit kind (its cause tag),
    /// the configured bound, the actual counter and the setting that raises
    /// the limit (FR-096, FR-255). Its position is its locus.
    fn catalog_fields(&self) -> Option<std::collections::BTreeMap<&'static str, String>> {
        Some(std::collections::BTreeMap::from([
            ("kind", self.kind().catalog_cause().to_owned()),
            ("bound", self.configured_bound.to_string()),
            ("actual", self.actual.to_string()),
            ("setting", self.setting.name().to_owned()),
        ]))
    }
}

/// FR-255 Behavior 3, QSpec FR-461's rendering: the stage and counter, the
/// configured bound, the count reached, the locus where known, and the
/// setting that raises the limit at each entry point.
impl std::fmt::Display for LimitExceeded {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} {} limit {} reached ({})",
            self.setting.stage(),
            self.kind().noun(),
            self.configured_bound,
            self.actual
        )?;
        if let Some(locus) = &self.locus {
            write!(formatter, " at {locus}")?;
        }
        write!(
            formatter,
            "; raise it with `--limit {name}=<n>` or the request's `stage_limits` entry `{name}`",
            name = self.setting.name()
        )
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use quire_exact::CancelCause;

    use super::{CatalogCoded, LimitExceeded, StageFailure};
    use crate::diagnostic::{category_of, CatalogCode, Category};
    use crate::setting::Setting;

    /// A stage cause with one fixed catalog code.
    #[derive(Debug)]
    struct Cause(&'static str);

    impl CatalogCoded for Cause {
        fn catalog_code(&self) -> CatalogCode {
            CatalogCode::new(self.0, "cause")
        }

        fn catalog_fields(&self) -> Option<std::collections::BTreeMap<&'static str, String>> {
            None
        }
    }

    /// FR-285-AC-3 (TC-769 step 3): a profile-gated refusal exits 21, an
    /// `ill_typed` one 20, and a reached limit 22.
    #[trace("TC-769", "FR-285-AC-3")]
    #[test]
    fn tc_769_stage_failures_exit_by_their_category() {
        let exit = |failure: StageFailure<Cause>| failure.category().exit_code();
        assert_eq!(
            exit(StageFailure::Refused(Cause("unsupported_construct"))),
            21
        );
        assert_eq!(exit(StageFailure::Refused(Cause("ill_typed"))), 20);
        assert_eq!(
            exit(StageFailure::Limit(LimitExceeded::new(
                Setting::S1Tokens,
                10,
                11
            ))),
            22
        );
    }

    /// FR-275: an internal fault is an internal failure, whatever the stage.
    #[trace("TC-769", "FR-285-AC-3")]
    #[test]
    fn a_stage_fault_is_an_internal_failure() {
        let failure: StageFailure<Cause> =
            StageFailure::Fault(crate::diagnostic::InternalFault::new("S3", "x"));
        assert_eq!(failure.category(), Category::InternalFailure);
    }

    /// FR-276-AC-5 (TC-757 step 5): a cancelled stage is incomplete and exits
    /// 22, whichever cause cancelled it.
    #[trace("TC-757", "FR-276-AC-5", "TC-769", "FR-285-AC-3")]
    #[test]
    fn a_cancelled_stage_failure_is_incomplete_and_exits_22() {
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            let failure: StageFailure<Cause> = StageFailure::Cancelled(cause);
            assert_eq!(failure.category(), Category::Incomplete);
            assert_eq!(failure.category().exit_code(), 22);
        }
    }

    /// FR-096-AC-2: a `LimitExceeded` reports
    /// `stage_limit_exceeded` with its setting's kind as the cause, and its
    /// record carries `kind`, `bound`, `actual` and `setting` (FR-255 Behavior 1).
    #[trace("TC-427", "FR-096-AC-2", "TC-428", "FR-096-AC-7")]
    #[test]
    fn limit_exceeded_reports_stage_limit_exceeded_per_setting() {
        for setting in Setting::ALL.iter().copied() {
            let exceeded = LimitExceeded::new(setting, 10, 11);
            let cause = setting.kind().catalog_cause();
            let code = exceeded.catalog_code();
            assert_eq!(code, CatalogCode::new("stage_limit_exceeded", cause));
            assert_eq!(category_of(&code), Some(Category::Incomplete));
            assert_eq!(exceeded.configured_bound(), 10);
            assert_eq!(exceeded.actual(), 11);
            assert_eq!(exceeded.setting(), setting);
            assert_eq!(exceeded.locus(), None);
            let fields = exceeded.catalog_fields().expect("a key-table row");
            assert_eq!(
                fields.into_iter().collect::<Vec<_>>(),
                [
                    ("actual", "11".to_owned()),
                    ("bound", "10".to_owned()),
                    ("kind", cause.to_owned()),
                    ("setting", setting.name().to_owned()),
                ]
            );
        }
    }

    /// FR-255-AC-2's rendering, with no locus: the stage and counter, the
    /// bound, the count reached and the setting at both entry points.
    #[trace("TC-720", "FR-255-AC-2")]
    #[test]
    fn a_limit_renders_its_setting_at_every_entry_point() {
        assert_eq!(
            LimitExceeded::new(Setting::S3Nodes, 4, 5).to_string(),
            "S3 node limit 4 reached (5); raise it with `--limit s3.nodes=<n>` or the request's \
             `stage_limits` entry `s3.nodes`"
        );
    }
}

/// The work each stage did for one operation, as the number of meter
/// charges it made (FR-275-AC-5). S2 and S4 charge no meter, so their
/// counters stay zero.
///
/// The counts are exact only when one operation counts at a time on a
/// `Cancel` handle. Operations that run at once on a shared handle get an
/// unspecified split of the shared count; use one handle per concurrent
/// operation for exact counts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StageWork {
    /// S1, reading source bytes into syntax.
    pub s1: u64,
    /// S2, building forms.
    pub s2: u64,
    /// I1, admitting domain packages.
    pub i1: u64,
    /// S3, assembling and checking.
    pub s3: u64,
    /// S4, linking.
    pub s4: u64,
    /// E4, emitting the package.
    pub e4: u64,
}

/// ADR-013 T-4: a stage's successful output.
///
/// T-4 also gives `Staged<T>` the warnings the stage raised. No stage
/// raises one yet, so the field waits for its first producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Staged<T> {
    value: T,
    work: StageWork,
}

impl<T> Staged<T> {
    /// A stage's output, with no work counted.
    pub const fn new(value: T) -> Self {
        Self {
            value,
            work: StageWork {
                s1: 0,
                s2: 0,
                i1: 0,
                s3: 0,
                s4: 0,
                e4: 0,
            },
        }
    }

    /// This output, with the work the operation did (FR-275-AC-5).
    #[must_use]
    pub fn with_work(mut self, work: StageWork) -> Self {
        self.work = work;
        self
    }

    /// The work the operation did, by stage.
    pub const fn work(&self) -> StageWork {
        self.work
    }

    /// The stage's output, borrowed.
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// The stage's output.
    pub fn into_value(self) -> T {
        self.value
    }
}

/// ADR-013 T-4: how a stage fails without producing output.
///
/// T-4 gives `Refused` a list of causes plus diagnostics. That is not here:
/// T-4 does not define the diagnostics' type, and a family `check` refuses
/// one form with one cause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StageFailure<C> {
    /// A configured stage limit was reached first.
    Limit(LimitExceeded),
    /// The stage refused its input with its own typed cause.
    Refused(C),
    /// The caller's [`quire_exact::Cancel`] handle was cancelled, and the
    /// stage stopped at its next charge (ADR-029 LC-3, FR-276).
    Cancelled(CancelCause),
    /// An internal invariant failed (FR-275): never a refusal of the input,
    /// and never a panic across the public boundary.
    Fault(InternalFault),
}

impl<C> StageFailure<C> {
    /// The stage's refusal cause, or what stopped it instead.
    pub fn into_refused(self) -> Result<C, Stopped> {
        match self {
            Self::Refused(cause) => Ok(cause),
            Self::Limit(limit) => Err(Stopped::Limit(limit)),
            Self::Cancelled(cause) => Err(Stopped::Cancelled(cause)),
            Self::Fault(fault) => Err(Stopped::Fault(fault)),
        }
    }
}

/// A [`StageFailure`] that is not a refusal of the input: what stopped the
/// stage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Stopped {
    /// A configured stage limit was reached.
    Limit(LimitExceeded),
    /// The caller cancelled the stage.
    Cancelled(CancelCause),
    /// An internal invariant failed.
    Fault(InternalFault),
}

impl<C: CatalogCoded> StageFailure<C> {
    /// FR-285: this failure's ADR-013 O-16 category. `Refused` takes its
    /// cause's: a profile-gated construct (`unsupported_construct`) is
    /// unsupported, any other refusal is a refusal. `Limit` and `Cancelled`
    /// are incomplete, and a `Fault` is the fault's own internal failure.
    pub fn category(&self) -> Category {
        match self {
            Self::Limit(_) | Self::Cancelled(_) => Category::Incomplete,
            Self::Fault(fault) => fault.category(),
            Self::Refused(cause) => match Code::from_code(cause.catalog_code().code()) {
                Some(code) if code.is_unsupported() => Category::Unsupported,
                Some(_) | None => Category::Refusal,
            },
        }
    }
}
