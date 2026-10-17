// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-341 and FR-342: the refinement class of a spine compile result and of a
//! clause-run disposition (ADR-017 RF-2), and [`CompileRefusal::causes`], the
//! all-causes accessor FR-341 reads.
//!
//! Both classifications are one exhaustive `match` with no wildcard arm, so a
//! new refusal or disposition variant does not compile until it is
//! classified. They read typed codes, causes and limits, never message text
//! (ADR-011 FB-02).

use std::borrow::Borrow;

use qsl_foundation::diagnostic::{LimitExceeded, StageFailure};
use qsl_foundation::{Code, Setting, SyntaxLimit};
use qsl_package::ImportViewRefusal;
use qsl_semantics::check::{AssemblyCause, CheckCause};
use qsl_semantics::model::intake::UnitIntakeCause;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_semantic_value::semantic_node::IdentityRefusal;

use super::{
    CallIncomplete, CallOutcome, CallRefusal, CallValue, ClauseDisposition, ClauseRunReport,
    ClauseRunStage, CompileRefusal, ImportRefusal,
};

/// The refinement class of one gate observation (ADR-017 RF-2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefinementClass {
    /// The input compiled, or the clause held.
    Admitted,
    /// A typed refusal or a violated clause: the specification is wrong.
    Refused,
    /// The selected profile prohibits the form (QSpec AD-003).
    Prohibited,
    /// A QSL implementation gap.
    Unsupported,
    /// A limit or budget stopped the run.
    Incomplete,
    /// The case's own setup is broken.
    ToolFailure,
    /// The selected clause is not declared. Only a clause run gives it.
    Absent,
}

/// The limit a `stage_limit_exceeded` or budget cause names and its configured
/// value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReachedLimit {
    /// The limit's FR-255 setting name, such as `s1.nodes`, or an accounting
    /// counter's bare name, such as `work_units`.
    pub name: &'static str,
    /// The limit's configured value.
    pub bound: u64,
}

impl ReachedLimit {
    fn of(limit: &LimitExceeded) -> Self {
        Self {
            name: limit.setting().name(),
            bound: limit.configured_bound(),
        }
    }

    fn of_setting(setting: Setting, bound: u64) -> Self {
        Self {
            name: setting.name(),
            bound,
        }
    }
}

/// One cause of a refusal: its catalog code and cause, and the limit it
/// names when it is a reached limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RefusalCause {
    /// The catalog code.
    pub code: Code,
    /// The catalog cause under `code`, where the cause names one.
    pub cause: Option<&'static str>,
    /// The limit this cause reached, where the cause carries one.
    pub limit: Option<ReachedLimit>,
}

impl RefusalCause {
    const fn new(code: Code, cause: Option<&'static str>) -> Self {
        Self {
            code,
            cause,
            limit: None,
        }
    }

    const fn reaching(mut self, limit: Option<ReachedLimit>) -> Self {
        self.limit = limit;
        self
    }

    /// The cause of a broken invariant.
    const INVARIANT: Self = Self::new(Code::RuntimeInvariant, None);
}

/// `causes`, or the one `runtime_invariant` cause when a stage that must hold
/// at least one holds none.
fn or_invariant(causes: Vec<RefusalCause>) -> Vec<RefusalCause> {
    if causes.is_empty() {
        vec![RefusalCause::INVARIANT]
    } else {
        causes
    }
}

impl CompileRefusal {
    /// Every cause of this refusal, in order: each entry of an `Assembly`,
    /// `Check` or `Profile` refusal's vector, the causes of a `Dependency`
    /// refusal's inner refusal, and the one cause of every other variant. A
    /// vector that is empty against its invariant yields one
    /// `runtime_invariant` cause, as [`Self::code`] does.
    pub fn causes(&self) -> Vec<RefusalCause> {
        match self {
            Self::Source(diagnostic) => {
                vec![
                    RefusalCause::new(diagnostic.code, None).reaching(diagnostic.limit().map(
                        |limit| match limit {
                            SyntaxLimit::Tokens { bound } => {
                                ReachedLimit::of_setting(Setting::S1Tokens, widen(bound))
                            }
                            SyntaxLimit::Nodes { bound } => {
                                ReachedLimit::of_setting(Setting::S1Nodes, widen(bound))
                            }
                            SyntaxLimit::Work { bound } => {
                                ReachedLimit::of_setting(Setting::S1WorkUnits, widen(bound))
                            }
                            SyntaxLimit::SourceBytes { bound } => {
                                ReachedLimit::of_setting(Setting::S1InputBytes, widen(bound))
                            }
                        },
                    )),
                ]
            }
            Self::Forms { failure, .. } => {
                vec![RefusalCause::new(failure.catalog_code(), None)]
            }
            Self::Intake { refusal, .. } => {
                vec![
                    RefusalCause::new(refusal.cause.code(), refusal.cause.cause()).reaching(
                        refusal.cause.denial().map(|denial| {
                            ReachedLimit::of_setting(denial.setting(), denial.limit())
                        }),
                    ),
                ]
            }
            Self::OperationClause { .. } | Self::TypeInvariant { .. } => {
                vec![RefusalCause::new(self.code(), None)]
            }
            Self::Profile { refusals, .. } => or_invariant(
                refusals
                    .iter()
                    .map(|refusal| RefusalCause::new(refusal.code(), Some(refusal.cause())))
                    .collect(),
            ),
            Self::Assembly { refusal, .. } => or_invariant(
                refusal
                    .errors
                    .iter()
                    .map(|error| {
                        RefusalCause::new(error.cause.code(), None)
                            .reaching(assembly_limit(&error.cause))
                    })
                    .collect(),
            ),
            Self::Check { refusals, .. } => or_invariant(
                refusals
                    .iter()
                    .map(|refusal| {
                        RefusalCause::new(refusal.cause.code(), refusal.cause.cause())
                            .reaching(check_limit(&refusal.cause))
                    })
                    .collect(),
            ),
            Self::DependencyInput(refusal) => {
                vec![RefusalCause::new(refusal.code(), refusal.cause())]
            }
            Self::Import { refusal, .. } => {
                vec![
                    RefusalCause::new(refusal.code(), refusal.cause()).reaching(match refusal {
                        ImportRefusal::Limit(limit) => Some(ReachedLimit::of(limit)),
                        ImportRefusal::View { refusal, .. } => match refusal.as_ref() {
                            ImportViewRefusal::Read {
                                refusal: StageFailure::Limit(limit),
                                ..
                            } => Some(ReachedLimit::of(limit)),
                            _ => None,
                        },
                        ImportRefusal::Cycle { .. }
                        | ImportRefusal::MissingSelection { .. }
                        | ImportRefusal::UnnamedImport => None,
                    }),
                ]
            }
            Self::Dependency { refusal, .. } => refusal.causes(),
            Self::Link(refusal) => vec![RefusalCause::new(refusal.code(), refusal.cause())],
            Self::Emit(refusal) => vec![RefusalCause::new(refusal.code(), refusal.cause())],
            Self::Limit(limit) => vec![RefusalCause::new(
                Code::StageLimitExceeded,
                Some(limit.kind().catalog_cause()),
            )
            .reaching(Some(ReachedLimit::of(limit)))],
            Self::Omitted(_) => vec![RefusalCause::new(Code::UnsupportedProjection, None)],
        }
    }
}

fn widen(bound: usize) -> u64 {
    u64::try_from(bound).unwrap_or(u64::MAX)
}

/// The limit an assembly cause reached, where it carries one.
fn assembly_limit(cause: &AssemblyCause) -> Option<ReachedLimit> {
    match cause {
        AssemblyCause::TypeLimit(limit)
        | AssemblyCause::IdentityLimit(limit)
        | AssemblyCause::DecimalScaleLimit(limit) => Some(ReachedLimit::of(limit)),
        AssemblyCause::UnresolvedTypeName { .. }
        | AssemblyCause::ImportedTypeName { .. }
        | AssemblyCause::AmbiguousTypeName { .. }
        | AssemblyCause::IllFormedBounds(_)
        | AssemblyCause::FloatingType { .. }
        | AssemblyCause::AliasCycle { .. }
        | AssemblyCause::UndeclaredAlias { .. }
        | AssemblyCause::DuplicateAlias { .. }
        | AssemblyCause::InvalidTypeDeclaration(_)
        | AssemblyCause::IdentityAllocation { .. }
        | AssemblyCause::StorageAllocation { .. }
        | AssemblyCause::StorageCapacity { .. }
        | AssemblyCause::Handle(_)
        | AssemblyCause::DuplicateEnumMember { .. }
        | AssemblyCause::DuplicateQuantityName { .. }
        | AssemblyCause::UnresolvedQuantityName { .. }
        | AssemblyCause::QuantityCycle { .. }
        | AssemblyCause::ZeroDenominator
        | AssemblyCause::UnitGraphTopology { .. }
        | AssemblyCause::NominalAdmission(_)
        | AssemblyCause::UnsuppliedImport { .. }
        | AssemblyCause::UnadmittedModel { .. }
        | AssemblyCause::UnsupportedModelMember { .. }
        | AssemblyCause::ModelType { .. }
        | AssemblyCause::UnresolvedOperation { .. }
        | AssemblyCause::AmbiguousOperation { .. }
        | AssemblyCause::DuplicateOperationBody { .. }
        | AssemblyCause::UnresolvedPopulationBound { .. }
        | AssemblyCause::AmbiguousPopulationBound { .. }
        | AssemblyCause::AmbiguousPopulation { .. } => None,
    }
}

/// The limit a check cause reached, where it carries one.
fn check_limit(cause: &CheckCause) -> Option<ReachedLimit> {
    match cause {
        CheckCause::ResourceExhausted(exhausted) => Some(ReachedLimit::of_setting(
            exhausted.kind.setting(),
            exhausted.limit,
        )),
        CheckCause::Identity(IdentityRefusal::InputBytes { bound, .. }) => Some(
            ReachedLimit::of_setting(Setting::IdentityInputBytes, *bound),
        ),
        CheckCause::ModelIncomplete(incomplete) => {
            let denial = incomplete.denial();
            Some(ReachedLimit::of_setting(denial.setting(), denial.limit()))
        }
        // A type invariant's wrapper classifies as the cause it wraps.
        CheckCause::TypeInvariant(refusal) => check_limit(&refusal.cause),
        CheckCause::DispatchLink(link) => link
            .denial
            .as_ref()
            .map(|denial| ReachedLimit::of_setting(denial.setting(), denial.limit())),
        CheckCause::Identity(
            IdentityRefusal::Allocation { .. } | IdentityRefusal::NonCanonical,
        )
        | CheckCause::ProfileProhibitedOperator(_)
        | CheckCause::StateModel(_)
        | CheckCause::ObjectReadInValueInvariant
        | CheckCause::ModelConformance(_)
        | CheckCause::TypeAdmission { .. }
        | CheckCause::IllTyped(_)
        | CheckCause::WrongSnapshot(_)
        | CheckCause::MissingName(_)
        | CheckCause::AmbiguousName { .. }
        | CheckCause::AmbiguousMember(_)
        | CheckCause::AmbiguousQualified(_)
        | CheckCause::UnsupportedNavigation(_)
        | CheckCause::NonBooleanRoot
        | CheckCause::IntegerOutsideI128(_)
        | CheckCause::MalformedUuidLiteral { .. }
        | CheckCause::UnanchoredResult { .. }
        | CheckCause::Unproved(_)
        | CheckCause::SumCase(_)
        | CheckCause::UnprovedDecrease { .. }
        | CheckCause::IeeeProfileNotAdmitted
        | CheckCause::UnrepresentableBound
        | CheckCause::DefinitionCycle { .. }
        | CheckCause::InvalidDispatchDeclaration(_)
        | CheckCause::MissingSelection { .. }
        | CheckCause::UnsupportedFeature { .. }
        | CheckCause::InternalFault(_)
        | CheckCause::NodePreimage(_)
        | CheckCause::ProtocolAnchor(_)
        | CheckCause::ProtocolReference(_) => None,
        #[cfg(seam_probe)]
        CheckCause::__SeamProbe => None,
    }
}

/// A classification: the class, and the codes and causes of every cause the
/// refusal or disposition holds, in order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Classification {
    /// The class.
    pub class: RefinementClass,
    /// Every cause, in the refusal's order. Empty for a disposition that
    /// holds none (a clause that held, a clause that evaluated `false`).
    pub causes: Vec<RefusalCause>,
}

impl Classification {
    fn new(class: RefinementClass, causes: Vec<RefusalCause>) -> Self {
        Self { class, causes }
    }

    /// For an `incomplete` class, the limit each limit cause names and that
    /// limit's value, in cause order; empty for every other class.
    pub fn limits(&self) -> Vec<ReachedLimit> {
        if self.class != RefinementClass::Incomplete {
            return Vec::new();
        }
        self.causes.iter().filter_map(|cause| cause.limit).collect()
    }
}

/// What the refusal variant says before its causes are read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Group {
    /// `Source`, `Forms`, `OperationClause`, `TypeInvariant`, `Assembly` or `Check`: the
    /// unit's own content was refused, so a typed code is a defect of it.
    Content,
    /// The case's setup or the toolchain failed, not the unit's content.
    Setup,
    /// `Link` or `Emit`: setup, unless the emitter reports a projection it
    /// has no form for.
    Projection,
    /// I1's caller-budget ceiling on a `model` declaration.
    IntakeBudget,
    /// A stage limit raised outside the content stages: `Limit` and an
    /// `Import` that reached its own or its I2 read's ceiling.
    Limit,
    /// The emitter would omit nodes.
    Omitted,
}

fn group(refusal: &CompileRefusal) -> Group {
    match refusal {
        CompileRefusal::Source(_)
        | CompileRefusal::Forms { .. }
        | CompileRefusal::OperationClause { .. }
        | CompileRefusal::TypeInvariant { .. }
        | CompileRefusal::Assembly { .. }
        | CompileRefusal::Check { .. } => Group::Content,
        CompileRefusal::Intake { refusal, .. } => match refusal.cause {
            UnitIntakeCause::Limit(_) => Group::IntakeBudget,
            UnitIntakeCause::ArtifactDigest
            | UnitIntakeCause::Refused(_)
            | UnitIntakeCause::Invariant => Group::Setup,
        },
        CompileRefusal::Profile { .. }
        | CompileRefusal::DependencyInput(_)
        | CompileRefusal::Dependency { .. } => Group::Setup,
        CompileRefusal::Import { refusal, .. } => match refusal {
            ImportRefusal::Limit(_) => Group::Limit,
            ImportRefusal::View { refusal, .. } => match refusal.as_ref() {
                ImportViewRefusal::Read {
                    refusal: StageFailure::Limit(_),
                    ..
                } => Group::Limit,
                ImportViewRefusal::Read { .. }
                | ImportViewRefusal::Emission { .. }
                | ImportViewRefusal::UnheldDependency { .. }
                | ImportViewRefusal::Evidence { .. } => Group::Setup,
            },
            ImportRefusal::Cycle { .. }
            | ImportRefusal::MissingSelection { .. }
            | ImportRefusal::UnnamedImport => Group::Setup,
        },
        CompileRefusal::Link(_) | CompileRefusal::Emit(_) => Group::Projection,
        CompileRefusal::Limit(_) => Group::Limit,
        CompileRefusal::Omitted(_) => Group::Omitted,
    }
}

/// Whether `code` records a reached stage limit or the caller's exhausted
/// budget (`resource_exhausted`, at I1 or S3): incomplete work, not a defect.
fn is_limit_code(code: Code) -> bool {
    matches!(code, Code::StageLimitExceeded | Code::ResourceExhausted)
}

/// Whether `code` is a typed refusal code of the unit's content: neither a
/// reached limit or budget nor a construct or projection the build or profile
/// does not admit.
fn is_typed_refusal(code: Code) -> bool {
    !is_limit_code(code)
        && !matches!(
            code,
            Code::UnsupportedConstruct | Code::UnsupportedProjection
        )
}

/// FR-341: classify a spine compile result into one refinement class.
pub fn classify_compile<T, E: Borrow<CompileRefusal>>(result: &Result<T, E>) -> Classification {
    match result {
        Ok(_) => Classification::new(RefinementClass::Admitted, Vec::new()),
        Err(refusal) => classify_refusal(refusal.borrow()),
    }
}

/// FR-341: classify one compile refusal. The rules apply in the table's
/// order and the first match gives the class.
pub fn classify_refusal(refusal: &CompileRefusal) -> Classification {
    let causes = refusal.causes();
    let group = group(refusal);
    let holds = |code: Code| causes.iter().any(|cause| cause.code == code);
    let class = if holds(Code::RuntimeInvariant)
        || group == Group::Setup
        || (group == Group::Projection && !holds(Code::UnsupportedProjection))
    {
        RefinementClass::ToolFailure
    } else if group == Group::Content && causes.iter().any(|cause| is_typed_refusal(cause.code)) {
        RefinementClass::Refused
    } else if group == Group::IntakeBudget || causes.iter().any(|cause| is_limit_code(cause.code)) {
        RefinementClass::Incomplete
    } else if group == Group::Omitted
        || holds(Code::UnsupportedProjection)
        || causes.iter().any(|cause| {
            cause.code == Code::UnsupportedConstruct && cause.cause == Some("not-yet-implemented")
        })
    {
        RefinementClass::Unsupported
    } else {
        RefinementClass::Prohibited
    };
    Classification::new(class, causes)
}

/// FR-342: a clause run's classification and the stage it settled at.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunClassification {
    /// The FR-109 stage of the disposition.
    pub stage: ClauseRunStage,
    /// The class and its causes.
    pub classification: Classification,
}

/// FR-342: classify a clause-run report's disposition once, keeping its
/// stage beside the class.
pub fn classify_clause_run(report: &ClauseRunReport) -> RunClassification {
    RunClassification {
        stage: report.disposition.stage(),
        classification: classify_disposition(&report.disposition),
    }
}

fn classify_disposition(disposition: &ClauseDisposition) -> Classification {
    use RefinementClass::{Absent, Incomplete, Refused, ToolFailure};
    let one = |class, code, cause| Classification::new(class, vec![RefusalCause::new(code, cause)]);
    match disposition {
        ClauseDisposition::Compile(refusal) => classify_refusal(refusal),
        ClauseDisposition::UnknownLanguage { .. } => one(ToolFailure, Code::UnknownLanguage, None),
        ClauseDisposition::StalePackage { .. } => one(ToolFailure, Code::StaleDependency, None),
        ClauseDisposition::MissingName { .. } => {
            one(Absent, Code::MissingDeclaration, Some("missing-name"))
        }
        ClauseDisposition::NotAPredicate { .. } => {
            one(Refused, Code::IllTyped, Some("type-mismatch"))
        }
        ClauseDisposition::Admit(failure) => match failure {
            AdmissionFailure::Refused(record) => one(Refused, record.code, Some(record.cause)),
            AdmissionFailure::Incomplete(record) => {
                one(Incomplete, record.code, Some(record.cause))
            }
            AdmissionFailure::Fault(_) => one(ToolFailure, Code::RuntimeInvariant, None),
        },
        ClauseDisposition::ArgumentRefusal(refusal) => one(Refused, refusal.code(), None),
        ClauseDisposition::Evaluate(outcome) => classify_outcome(outcome),
        ClauseDisposition::FrameViolation(witness) => {
            one(Refused, witness.code, Some(witness.cause))
        }
        ClauseDisposition::EvaluateFault(_) => one(ToolFailure, Code::RuntimeInvariant, None),
    }
}

fn classify_outcome(outcome: &CallOutcome) -> Classification {
    use RefinementClass::{Admitted, Incomplete, Refused, ToolFailure};
    match outcome {
        CallOutcome::Completed(CallValue::Boolean(true)) => {
            Classification::new(Admitted, Vec::new())
        }
        CallOutcome::Completed(CallValue::Boolean(false)) => {
            Classification::new(Refused, Vec::new())
        }
        // FR-109 selects only `Boolean` functions, so neither completes.
        CallOutcome::Completed(CallValue::Integer(_) | CallValue::Reference(_)) => {
            Classification::new(ToolFailure, vec![RefusalCause::INVARIANT])
        }
        CallOutcome::Refused(refusal) => {
            let (CallRefusal::Record { code, .. } | CallRefusal::Family { code, .. }) = refusal;
            let causes = Code::from_code(code.code())
                .map(|known| RefusalCause::new(known, Some(code.cause())))
                .into_iter()
                .collect();
            Classification::new(Refused, causes)
        }
        // An undefined evaluation settles refuted.
        CallOutcome::Undefined { reason } => Classification::new(
            Refused,
            vec![RefusalCause::new(Code::UndefinedExpression, Some(reason))],
        ),
        CallOutcome::Incomplete(incomplete) => Classification::new(
            Incomplete,
            vec![RefusalCause::new(Code::ResourceExhausted, None)
                .reaching(Some(accounting_limit(incomplete)))],
        ),
    }
}

fn accounting_limit(incomplete: &CallIncomplete) -> ReachedLimit {
    ReachedLimit {
        name: incomplete.setting().name(),
        bound: incomplete.record.limit,
    }
}
