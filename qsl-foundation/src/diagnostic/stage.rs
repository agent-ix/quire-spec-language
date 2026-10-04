// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 T-4: a stage's structured outcome, `Result<Staged<T>,
//! StageFailure<C>>`, and the stage limit a `StageFailure::Limit` names.
//! These live in the foundation `diagnostic` module (T-4). They are
//! distinct from the kernel `Outcome<T>`/`Incomplete`, which only
//! evaluation (S6a) returns.
//!
//! `LimitExceeded`'s catalog code is `stage_limit_exceeded` with the kind's
//! own `<kind>-exceeded` cause ([`LimitKind::catalog_cause`]), under
//! `quire.native.diagnostics/v1` revision `1-draft.8`, which this build
//! claims. It carries the T-5 [`Locus`] where the limit was reached, absent
//! only where FR-096 says no producer can know one.

use quire_exact::CancelCause;

use super::{CatalogCode, CatalogCoded, Category, Code, InternalFault, Locus};

/// ADR-013 T-4's closed limit kind: one variant per
/// `stage_limit_exceeded` cause of `quire.native.diagnostics/v1` revision
/// `1-draft.8`. The catalog row names which S1 or I2 limit carries each of
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
    /// `stage_limit_exceeded/<kind>-exceeded` (revision `1-draft.8`).
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

/// The caller's limits field that sets a stage limit's bound (FR-277): the
/// closed set of fields the front end's limits value carries, each spelled
/// `<limits group>.<field>`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LimitsField {
    /// `source.source_bytes`.
    SourceBytes,
    /// `source.tokens`.
    SourceTokens,
    /// `source.nodes`.
    SourceNodes,
    /// `source.work_units`.
    SourceWorkUnits,
    /// `model.declaration_records`.
    ModelDeclarationRecords,
    /// `model.derivation_facts`.
    ModelDerivationFacts,
    /// `model.effective_declarations`.
    ModelEffectiveDeclarations,
    /// `model.dispatch_candidates`.
    ModelDispatchCandidates,
    /// `model.hashed_bytes`.
    ModelHashedBytes,
    /// `model.work_units`.
    ModelWorkUnits,
    /// `model.ancestor_steps`.
    ModelAncestorSteps,
    /// `model.family_steps`.
    ModelFamilySteps,
    /// `environment.ancestor_steps`.
    EnvironmentAncestorSteps,
    /// `environment.work_units`.
    EnvironmentWorkUnits,
    /// `checking.nodes`.
    CheckingNodes,
    /// `checking.input_bytes`.
    CheckingInputBytes,
    /// `checking.work_budget`.
    CheckingWorkBudget,
}

impl LimitsField {
    /// The field's `<limits group>.<field>` spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceBytes => "source.source_bytes",
            Self::SourceTokens => "source.tokens",
            Self::SourceNodes => "source.nodes",
            Self::SourceWorkUnits => "source.work_units",
            Self::ModelDeclarationRecords => "model.declaration_records",
            Self::ModelDerivationFacts => "model.derivation_facts",
            Self::ModelEffectiveDeclarations => "model.effective_declarations",
            Self::ModelDispatchCandidates => "model.dispatch_candidates",
            Self::ModelHashedBytes => "model.hashed_bytes",
            Self::ModelWorkUnits => "model.work_units",
            Self::ModelAncestorSteps => "model.ancestor_steps",
            Self::ModelFamilySteps => "model.family_steps",
            Self::EnvironmentAncestorSteps => "environment.ancestor_steps",
            Self::EnvironmentWorkUnits => "environment.work_units",
            Self::CheckingNodes => "checking.nodes",
            Self::CheckingInputBytes => "checking.input_bytes",
            Self::CheckingWorkBudget => "checking.work_budget",
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
    /// Boxed: a `Locus` names a source reference or a digest and pointer,
    /// far larger than the rest, and every stage's `Result` carries this.
    locus: Option<Box<Locus>>,
    /// The name of the caller's limits field that sets the bound (FR-277),
    /// such as `source.tokens`. `None` where no caller field sets it.
    field: Option<LimitsField>,
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
            field: None,
        }
    }

    /// This limit, named by the caller's limits field `field` that raises
    /// it (FR-277).
    #[must_use]
    pub const fn named(mut self, field: LimitsField) -> Self {
        self.field = Some(field);
        self
    }

    /// The caller's limits field that sets this bound, when one does.
    pub const fn limits_field(&self) -> Option<LimitsField> {
        self.field
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
        self.locus.as_deref()
    }
}

impl CatalogCoded for LimitExceeded {
    /// `stage_limit_exceeded/<kind>-exceeded`: the kind alone decides the
    /// cause; `configured_bound`/`actual` are carried by this value itself,
    /// not folded into the tag.
    fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("stage_limit_exceeded", self.kind.catalog_cause())
    }

    /// The catalog row's payload: the exceeded limit kind (its cause tag),
    /// the configured bound and the actual counter (FR-096). Its position is
    /// its locus.
    fn catalog_fields(&self) -> Option<std::collections::BTreeMap<&'static str, String>> {
        let mut fields = std::collections::BTreeMap::from([
            ("kind", self.kind.catalog_cause().to_owned()),
            ("bound", self.configured_bound.to_string()),
            ("actual", self.actual.to_string()),
        ]);
        if let Some(field) = self.field {
            fields.insert("field", field.as_str().to_owned());
        }
        Some(fields)
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use quire_exact::CancelCause;

    use super::{CatalogCoded, LimitExceeded, LimitKind, StageFailure};
    use crate::diagnostic::{category_of, CatalogCode, Category};

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
                LimitKind::TokenCount,
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

    /// FR-096-AC-2 at catalog revision `1-draft.8`: each of the eight kinds
    /// reports `stage_limit_exceeded` with its own cause, and a
    /// `LimitExceeded` reports its kind's code with the bound and actual
    /// counter.
    #[trace("TC-427", "FR-096-AC-2", "TC-428", "FR-096-AC-7")]
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
            let fields = exceeded.catalog_fields().expect("a key-table row");
            assert_eq!(
                fields.into_iter().collect::<Vec<_>>(),
                [
                    ("actual", "11".to_owned()),
                    ("bound", "10".to_owned()),
                    ("kind", cause.to_owned()),
                ]
            );
        }
    }
}

/// The work each stage did for one operation, as the number of meter
/// charges it made (FR-275-AC-5). S2 and S4 charge no meter, so their
/// counters stay zero.
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
