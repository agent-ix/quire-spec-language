// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-109: `qsl_replay::spine::run_clause`, the spine's clause-run
//! entry beside [`super::call::run`]'s function-run entry (ADR-011 §5).
//!
//! Compiles a `1-draft` unit through the spine ([`super::compile`]),
//! resolves one state clause or one Boolean function run as a claim by
//! name, admits the supplied observations (FR-106,
//! `qsl_semantics::model::observation`), evaluates through S6a (FR-107's
//! `evaluate_clause`, or `CheckedPackage::call` for a `Function`
//! selection) and returns one [`ClauseRunReport`]: a typed disposition.
//! Reuses FR-100's outcome mapping ([`super::call::convert_outcome`]) by
//! reference, rather than restating it.

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::source::Source;
use qsl_foundation::source_map::NativeLanguage;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{CheckedGraph, CheckedStateClause};
use qsl_semantics::library::PackageId;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::observation::{
    admit_current_snapshot, admit_frame_invocation, admit_observations, population_universe_for,
    AdmissionFailure, AdmissionRecord, AdmittedObservations, ClauseFacts, ClauseSelection,
    ClauseSelectionInput, DocumentRef, FrameFacts, FrameWitness, ObservationLimits, OperationFacts,
    Provisions,
};
use quire_exact::{Cancel, Identifier, Meter, ScalarLimits, Value, ValueType};

mod temporal;
pub use temporal::{OverObject, TraceInput};

use super::call::{convert_call_failure, convert_outcome, select};
pub use super::call::{CallOutcome, CallValue, RunRefusal};
use super::lifecycle as front_end;
use super::{CompileRefusal, DependencyInput, SpineLimits};
use crate::proof_result::SettlementBasis;
use crate::result::SeparatingWitnessRecord;
use crate::witness::derive_separating_witness;

/// FR-109's `Function` selection argument value: FR-100's canonical
/// integer, or an object reference resolved in the selection's current
/// snapshot.
#[derive(Clone, Debug)]
pub enum ClauseArgumentValue {
    /// A canonical integer (FR-100's rule): the integer itself, or `0`/`1`
    /// for a `Boolean` parameter.
    Integer(i64),
    /// `{"reference": {population, key}}`.
    Reference {
        /// The population identity.
        population: String,
        /// The object's declared key.
        key: String,
    },
}

/// One `{parameter, value}` argument of a `Function` selection.
#[derive(Clone, Debug)]
pub struct ClauseArgument {
    /// The parameter's declared name.
    pub parameter: String,
    /// The argument's value.
    pub value: ClauseArgumentValue,
}

/// FR-109's selection: a state clause (FR-106's [`ClauseSelection`]), a
/// Boolean function run as a claim, (FR-115) an operation frame, or (FR-330)
/// a temporal clause over a supplied trace.
#[derive(Clone, Debug)]
pub enum ClauseRunSelection {
    /// A state clause, admitted per FR-106.
    Clause(ClauseSelection),
    /// A function, called as a claim (FR-109's own `Function` selection).
    Function {
        /// The function's name (FR-100's `function` rule; one segment,
        /// resolved by `super::call::select`).
        name: String,
        /// The call's arguments, one per parameter, in any order.
        arguments: Vec<ClauseArgument>,
        /// The current snapshot each object argument resolves against.
        snapshot: DocumentRef,
    },
    /// FR-115: an operation's frame, checked over one invocation. The
    /// request carries no frame and no permission: the frame comes only from
    /// the compiled package (QSpec FR-013-AC-3).
    Frame {
        /// The operation `M::T::op`.
        operation: OperationName,
        /// The `quire.state.invocation/v1` document, with its pre and post
        /// snapshots in the provision.
        invocation: DocumentRef,
    },
    /// FR-330: a temporal clause, run over a supplied trace.
    Temporal {
        /// The temporal clause's declared name.
        name: String,
        /// The observations the clause is run over.
        trace: TraceInput,
        /// The object the clause's `over` parameter names, resolved in the
        /// observation at position 0.
        over: OverObject,
    },
}

/// FR-115: the operation a `Frame` selection names, `M::T::op`: model
/// alias `M`, object type `T` and operation `op`, as FR-104 spells a
/// clause's operation. [`CheckedGraph::resolve_operation`] resolves it once
/// against the compiled package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationName {
    /// The model alias `M`.
    pub model: Identifier,
    /// The object type `T`.
    pub object: Identifier,
    /// The operation `op`.
    pub operation: Identifier,
}

impl std::fmt::Display for OperationName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}::{}::{}",
            self.model.as_str(),
            self.object.as_str(),
            self.operation.as_str()
        )
    }
}

/// FR-109 Inputs' unit: the program source, or an I3 extracted source.
///
/// `#[non_exhaustive]`: the `Extracted` variant exists only under the
/// `quire-extraction` feature, and Cargo unifies features across a build,
/// so a downstream match must not rely on which variants it sees.
#[derive(Debug)]
#[non_exhaustive]
pub enum ClauseRunSource {
    /// The unit's source bytes and its four FR-001 labels.
    Program {
        /// The unit's four FR-001 labels.
        identity: SourceIdentity,
        /// The source's display path; only displayed, never opened.
        path: String,
        /// The unit's source bytes.
        bytes: Vec<u8>,
    },
    /// An I3 extracted source (ADR-011 I3, `qsl-source`): the verified body
    /// `qsl_source::extract` returned. Its body, with the body's own
    /// identity and path, is the unit `compile` reads. Its declared fence language
    /// must be `ix:native`, or the run reports
    /// [`ClauseDisposition::UnknownLanguage`] at stage `compile`.
    #[cfg(feature = "quire-extraction")]
    Extracted(qsl_source::ExtractedSource),
}

/// The unit a [`ClauseRunSource`] compiles, borrowed from it.
struct Unit<'a> {
    identity: &'a SourceIdentity,
    path: &'a str,
    bytes: &'a [u8],
    /// The I3 extracted clause's declared fence language; `None` for a
    /// program source, whose language is its own header's.
    language: Option<&'a str>,
}

impl ClauseRunSource {
    fn unit(&self) -> Unit<'_> {
        match self {
            Self::Program {
                identity,
                path,
                bytes,
            } => Unit {
                identity,
                path,
                bytes,
                language: None,
            },
            #[cfg(feature = "quire-extraction")]
            Self::Extracted(extracted) => {
                let map = extracted.map();
                let body = map.body();
                Unit {
                    identity: body.identity(),
                    path: body.path(),
                    bytes: body.text().as_bytes(),
                    language: Some(extracted.language()),
                }
            }
        }
    }
}

/// FR-109 Inputs: one clause-run request.
pub struct ClauseRunRequest {
    /// The unit: program source bytes, or an I3 extracted source.
    pub source: ClauseRunSource,
    /// FR-056's package input.
    pub packages: BTreeMap<[u8; 32], Vec<u8>>,
    /// FR-099's dependency input.
    pub dependencies: DependencyInput,
    /// FR-106's snapshot provision.
    pub snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    /// FR-106's invocation provision.
    pub invocations: BTreeMap<[u8; 32], Vec<u8>>,
    /// The selection.
    pub selection: ClauseRunSelection,
    /// An optional expected `package_id` (FR-098's stale package rule).
    pub expected_package_id: Option<PackageId>,
    /// A wire caller's lowercase hex claim, kept separate from a minted
    /// `PackageId` until it is compared with the compiled package.
    pub expected_package_id_claim: Option<String>,
    /// The spine's stage limits.
    pub limits: SpineLimits,
    /// FR-106's observation limits.
    pub observation_limits: ObservationLimits,
    /// The evaluation meter's accounting limits (FR-100's `accounting` object).
    pub accounting: ScalarLimits,
}

/// Why [`run_clause`] returned no report: a request that cannot be formed
/// (FR-109 Outputs). Every compile, selection, admission or evaluation
/// result is a report, never this.
#[derive(Debug, thiserror::Error)]
pub enum ClauseRunRefusal {
    /// The unit's source is empty.
    #[error("the unit source is empty")]
    EmptySource,
}

/// FR-109's disposition stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseRunStage {
    /// Compile (S1 to S4).
    Compile,
    /// Name resolution.
    Select,
    /// FR-106 admission.
    Admit,
    /// S6a evaluation.
    Evaluate,
}

/// FR-109's disposition: one record per non-`Evaluate` stage, or the S6a
/// outcome (FR-100's mapping, reused unchanged) for `Evaluate`.
#[derive(Debug)]
pub enum ClauseDisposition {
    /// Stage `compile`: the spine compile refused, naming its own cause
    /// code.
    Compile(Box<CompileRefusal>),
    /// Stage `compile`, category `refusal`, `unknown_language`: an I3
    /// extracted clause whose declared fence language is not `ix:native`
    /// (the same refusal the root crate's `mapped::compile` gives). Nothing
    /// is compiled.
    UnknownLanguage {
        /// The declared fence language.
        language: String,
    },
    /// Stage `compile`, category `refusal`, `stale_dependency`: the
    /// recompiled `package_id` differs from the request's expected one.
    StalePackage {
        /// The expected `package_id`.
        expected: String,
        /// The recompiled `package_id`.
        actual: PackageId,
    },
    /// Stage `select`, `missing_declaration`/`missing-name`.
    MissingName {
        /// The selected name.
        name: String,
    },
    /// Stage `select`, `ill_typed`/`type-mismatch`: a `Function` selection
    /// names a function whose declared result is not `Boolean`.
    NotAPredicate {
        /// The selected name.
        name: String,
    },
    /// Stage `admit`: FR-106 admission's own failure, or (for a `Function`
    /// selection) the analogous snapshot-only admission failure.
    Admit(AdmissionFailure),
    /// Stage `admit`: a `Function` selection's argument refused before any
    /// call (FR-100's own refusal, reused).
    ArgumentRefusal(Box<RunRefusal>),
    /// Stage `evaluate`: the mapped S6a outcome.
    Evaluate(CallOutcome),
    /// Stage `evaluate`, category `violation`, `truth: false` (FR-115): a
    /// `Frame` run found a change outside the frame. Carries the evaluated
    /// frame witness, whose cause is `frame_violation`/`unauthorized-change`.
    /// A verdict that evaluated, not a refusal.
    FrameViolation(Box<FrameWitness>),
    /// Stage `evaluate` (FR-330): the TemporalTrace evaluator's outcome for
    /// a `Temporal` selection. Its category is the outcome's: a pending
    /// result is `inconclusive`, a missing fairness premise `unsupported`,
    /// and a false or undefined clause a `violation` at its position.
    Temporal(TemporalVerdict),
    /// Stage `evaluate`, category `refusal` (FR-330): the evaluator refused
    /// the supplied trace (a lasso with an empty loop, for one).
    TemporalRefusal(TemporalTraceRefusal),
    /// Stage `evaluate`, category `internal-failure`: a broken S6a
    /// invariant reached outside the outcome mapping (FR-100's own
    /// internal-failure exit status).
    EvaluateFault(qsl_foundation::diagnostic::InternalFault),
}

impl ClauseDisposition {
    /// The FR-109 stage this disposition reports at.
    pub fn stage(&self) -> ClauseRunStage {
        match self {
            Self::Compile(_) | Self::UnknownLanguage { .. } | Self::StalePackage { .. } => {
                ClauseRunStage::Compile
            }
            Self::MissingName { .. } | Self::NotAPredicate { .. } => ClauseRunStage::Select,
            Self::Admit(_) | Self::ArgumentRefusal(_) => ClauseRunStage::Admit,
            Self::Evaluate(_)
            | Self::Temporal(_)
            | Self::TemporalRefusal(_)
            | Self::FrameViolation(_)
            | Self::EvaluateFault(_) => ClauseRunStage::Evaluate,
        }
    }

    /// FR-109 Outputs' `category` (ADR-013 O-16, `qsl_foundation::
    /// diagnostic::Category`). A compile refusal, an argument refusal and an
    /// admission refusal take their code's
    /// [`Code::category`](qsl_foundation::diagnostic::Code::category)
    /// (`unknown_required_feature` is unsupported); an admission
    /// `Incomplete` is incomplete and a `Fault` an internal failure. The
    /// run's exit code is this category's
    /// [`Category::exit_code`](qsl_foundation::diagnostic::Category::exit_code)
    /// (FR-285).
    pub fn category(&self) -> qsl_foundation::diagnostic::Category {
        use qsl_foundation::diagnostic::Category;
        match self {
            Self::Compile(refusal) => refusal.code().category(),
            Self::ArgumentRefusal(refusal) => refusal.category(),
            Self::UnknownLanguage { .. }
            | Self::StalePackage { .. }
            | Self::MissingName { .. }
            | Self::NotAPredicate { .. } => Category::Refusal,
            Self::Admit(AdmissionFailure::Refused(record)) => record.code.category(),
            Self::Admit(AdmissionFailure::Incomplete(_)) => Category::Incomplete,
            Self::Admit(AdmissionFailure::Fault(_)) | Self::EvaluateFault(_) => {
                Category::InternalFailure
            }
            Self::FrameViolation(_) => Category::Violation,
            Self::Temporal(outcome) => outcome.category(),
            // The refusal's own catalog code decides: a malformed trace is a
            // refusal, a profile the evaluator does not read is unsupported.
            Self::TemporalRefusal(refusal) => {
                qsl_foundation::diagnostic::Code::from_code(refusal.catalog_code().code())
                    .map_or(Category::InternalFailure, |code| code.category())
            }
            Self::Evaluate(CallOutcome::Completed(CallValue::Boolean(false))) => {
                Category::Violation
            }
            Self::Evaluate(CallOutcome::Completed(CallValue::Reference(_))) => {
                Category::InternalFailure
            }
            Self::Evaluate(outcome) => outcome.category(),
        }
    }

    /// FR-109 Outputs' `truth`: only for `success` and `violation`.
    pub fn truth(&self) -> Option<bool> {
        match self {
            Self::Evaluate(CallOutcome::Completed(CallValue::Boolean(value))) => Some(*value),
            Self::FrameViolation(_) => Some(false),
            Self::Temporal(TemporalVerdict::Holds) => Some(true),
            Self::Temporal(TemporalVerdict::Violated { .. }) => Some(false),
            _ => None,
        }
    }
}

/// The settlement of a temporal clause on one trace (FR-327 "Outputs"), as
/// the spine reports it (FR-100-AC-8: the spine's public surface owns its
/// types). Positions are zero-based trace indices.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemporalVerdict {
    /// `Completed(true)`: every activation evaluated true. Evidence for this
    /// trace only (`tested`), never `proved` (ADR-014 section 8).
    Holds,
    /// `Completed(false)`: the first activation, in position order, that
    /// evaluated false.
    Violated {
        /// The position of that activation.
        at: u64,
    },
    /// The claim is not defined on the trace (ADR-018 UE-2); it does not
    /// hold on it.
    Undefined {
        /// The first position at which an atom the clause reads is
        /// undefined.
        at: u64,
        /// The first undefined atom at `at`, in clause-node order.
        cause: TemporalUndefinedCause,
    },
    /// A finite prefix that neither shows a bad prefix nor can show more
    /// (FR-328): O-16 inconclusive.
    Pending,
    /// The clause has a fairness set and the trace carries no enabledness
    /// to check it against (FR-328, FR-329): O-16 unsupported.
    MissingFairnessPremise {
        /// The first constraint of the checked fairness set.
        constraint: quire_semantic_value::fairness::FairnessConstraint,
    },
    /// The work budget ran out; no truth value (ADR-014 TR-5).
    Incomplete(quire_exact::Incomplete),
    /// A window aggregate outgrew `max_window_values` (FR-194; ADR-018
    /// V-7); no truth value.
    WindowLimitReached {
        /// The limit's name, `max_window_values`.
        limit: &'static str,
        /// The limit's value in this run.
        value: u64,
        /// The setting that raises it, `monitor.max_window_values`.
        setting: &'static str,
    },
}

impl TemporalVerdict {
    /// The ADR-013 O-16 category of this verdict (FR-327 "Results").
    pub fn category(&self) -> qsl_foundation::diagnostic::Category {
        use qsl_foundation::diagnostic::Category;
        match self {
            Self::Holds => Category::Success,
            Self::Undefined {
                cause: TemporalUndefinedCause::InsufficientData { .. },
                ..
            } => Category::Undefined,
            Self::Violated { .. } | Self::Undefined { .. } => Category::Violation,
            Self::Pending => Category::Inconclusive,
            Self::MissingFairnessPremise { .. } => Category::Unsupported,
            Self::Incomplete(_) | Self::WindowLimitReached { .. } => Category::Incomplete,
        }
    }
}

/// Why an atom has no value at a position (ADR-018 UE-2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemporalUndefinedCause {
    /// The kernel `Outcome::Undefined`, for example division by zero.
    Kernel(quire_exact::Undefined),
    /// A family's `FamilyResult::Undefined` record.
    Family(qsl_foundation::UndefinedRecord),
    /// A window aggregate held fewer values than its `min_count` (ADR-024
    /// WA-4).
    InsufficientData {
        /// The values the window held.
        count: u64,
        /// The term's `min_count`.
        min_count: u64,
    },
}

/// Why the temporal evaluator could not read the supplied trace at all.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TemporalTraceRefusal {
    /// A finite trace holds no observation (FR-327 "Inputs").
    #[error("a finite trace holds at least one observation")]
    EmptyTrace,
    /// A tuple whose component count differs from the hyper clause's trace
    /// variables (FR-175 "Inputs").
    #[error("the tuple holds {found} traces for {expected} trace variables")]
    TupleArity {
        /// The clause's trace variables.
        expected: usize,
        /// The components supplied.
        found: usize,
    },
    /// A hyper clause with `align skip` (HP-6, FR-178) handed to the lockstep
    /// evaluator: it is read over projected prefixes, and no replay of that
    /// reading is built yet.
    #[error("a clause with `align skip` is not evaluated in lockstep; its replay is not built")]
    AlignSkipNotImplemented,
    /// A lasso with an empty loop (FR-329 "Lasso shape").
    #[error("a lasso holds at least one loop position")]
    EmptyLoop,
    /// A lasso's prefix and loop together exceed `u64` positions.
    #[error("a lasso's prefix and loop exceed u64 positions")]
    TraceTooLong,
    /// A lasso under a bounded profile: only the infinite-trace profile
    /// reads one (FR-329).
    #[error("a lasso is read under the infinite-trace profile only, not {profile:?}")]
    LassoUnderBoundedProfile {
        /// The clause's profile.
        profile: qsl_semantics::check::TemporalProfile,
    },
    /// The profile reads timestamped instants, which a position trace does
    /// not carry.
    #[error("{profile:?} reads timestamped instants, which this trace does not carry")]
    ProfileNotEvaluated {
        /// The clause's profile.
        profile: qsl_semantics::check::TemporalProfile,
    },
    /// A position is outside the represented trace (FR-327 "Positions").
    #[error(transparent)]
    Position(TemporalPositionRefusal),
    /// A window aggregate cannot be read on the trace.
    #[error(transparent)]
    Window(TemporalWindowRefusal),
    /// An invariant of the checked clause or the evaluator broke.
    #[error("temporal evaluation broke invariant {0:?}")]
    Fault(InternalFault),
}

impl TemporalTraceRefusal {
    /// The catalog code of this refusal: `invalid_runtime_input`/`invalid-value`
    /// for a malformed trace or position (FR-327, FR-329),
    /// `unsupported_construct`/`expression-form` for a profile or trace shape
    /// the evaluator does not read, `unsupported_construct`/
    /// `not-yet-implemented` for `align skip`, and
    /// `runtime_invariant`/`established-invariant-broken` for a fault.
    pub fn catalog_code(&self) -> qsl_foundation::CatalogCode {
        use qsl_foundation::CatalogCode;
        match self {
            Self::EmptyTrace
            | Self::EmptyLoop
            | Self::TraceTooLong
            | Self::TupleArity { .. }
            | Self::Position(_)
            | Self::Window(_) => CatalogCode::new("invalid_runtime_input", "invalid-value"),
            Self::AlignSkipNotImplemented => {
                CatalogCode::new("unsupported_construct", "not-yet-implemented")
            }
            Self::LassoUnderBoundedProfile { .. } | Self::ProfileNotEvaluated { .. } => {
                CatalogCode::new("unsupported_construct", "expression-form")
            }
            Self::Fault(fault) => fault.catalog_code(),
        }
    }
}

/// Why a trace position descriptor is refused (FR-327 "Positions").
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TemporalPositionRefusal {
    /// The descriptor is empty or holds a byte that is not an ASCII digit.
    #[error("a trace position is decimal ASCII digits")]
    NotDecimal,
    /// The descriptor has a leading zero.
    #[error("a trace position has no leading zero")]
    LeadingZero,
    /// The descriptor exceeds `u64`.
    #[error("a trace position exceeds u64")]
    Overflow,
    /// The decoded position lies outside the represented trace.
    #[error("trace position TemporalPosition({position}) lies outside the represented trace")]
    OutsideTrace {
        /// The refused position.
        position: u64,
    },
}

/// Why a window aggregate cannot be read on the trace.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TemporalWindowRefusal {
    /// The position is not on the trace.
    #[error("position {position} is outside the trace")]
    OutsideTrace {
        /// The refused position.
        position: u64,
    },
    /// A position window on a timestamped trace, or a duration window on a
    /// trace without time stamps.
    #[error("the window's kind does not match the trace's profile")]
    WindowKindMismatch,
    /// Time stamps are not in non-decreasing order.
    #[error("time stamps do not decrease along a trace")]
    TimestampsDecrease,
}

/// FR-109 Outputs' usage: "the admission work and the evaluation meter
/// charges, separately" (`FR-109-run-a-state-clause-through-the-spine.
/// md:85`).
///
/// **Production-safe by construction.** `quire_exact::Meter`
/// deliberately keeps no per-charge log outside `test-support`
/// (`quire_exact::Meter`'s own doc: "a production meter holds only
/// fixed-size state... the meter never grows with the charge count"), and
/// `test-support` may only be a dev-dependency. `evaluation_charges` below is
/// therefore the meter's own fixed-size, always-available surface --
/// `admission_count()` and one `consumed(kind)` per `LimitKind::ALL` -- never
/// `Meter::admitted_charges()`'s per-event log. This still gives every
/// property AC-5 needs (equal reports on a repeated run) without adding
/// growing state to a production report.
///
/// Admission work is tracked by `ObservationLimits`' own four
/// document-shape ceilings, not `quire_exact`'s `LimitKind` (admission has
/// no analogue of `integer_bits`/`work_units`): `admission_consumed` is
/// `qsl_semantics::model::observation::AdmissionUsage`, the amount those
/// four ceilings actually consumed across every document admission read
/// (SR-751 FND-002 round 2), never pretended complete by reusing a
/// `LimitKind` that would misreport zeroes for kinds admission never
/// touches.
#[derive(Clone, Debug, Default)]
pub struct ClauseRunUsage {
    /// The admission work consumed.
    pub admission_consumed: qsl_semantics::model::observation::AdmissionUsage,
    /// How many charges the evaluation meter admitted in total.
    pub evaluation_admissions: u64,
    /// The evaluation meter's consumed total, one pair per `LimitKind::ALL`.
    pub evaluation_consumed: Vec<(quire_exact::LimitKind, u64)>,
}

impl ClauseRunUsage {
    fn from_meter(
        meter: &Meter,
        admission_consumed: qsl_semantics::model::observation::AdmissionUsage,
    ) -> Self {
        Self {
            admission_consumed,
            evaluation_admissions: meter.admission_count(),
            evaluation_consumed: quire_exact::LimitKind::ALL
                .iter()
                .map(|kind| (*kind, meter.consumed(*kind)))
                .collect(),
        }
    }
}

/// FR-109's `ClauseRunReport`: the disposition, the usage and, where compile
/// reached it, the compiled `package_id`.
#[derive(Debug)]
pub struct ClauseRunReport {
    /// The compiled `package_id`, when compile completed.
    pub package_id: Option<PackageId>,
    /// The disposition.
    pub disposition: ClauseDisposition,
    /// FR-109 Outputs' usage.
    pub usage: ClauseRunUsage,
    /// FR-266 (QSpec FR-243): the run's settlement basis, on every report.
    pub basis: SettlementBasis,
    /// FR-266 (QSpec FR-351): the separating witness record, present
    /// exactly when `basis` is decisive.
    pub witness: Option<SeparatingWitnessRecord>,
}

/// FR-266: the basis of a report that derives no witness: `closed-scope`
/// for a `success` or `violation` at stage `evaluate` (a `Function` run's
/// result is decided by the whole call, a `Frame` run's witness is
/// FR-115's), `unavailable` for every other disposition.
fn undecided_basis(disposition: &ClauseDisposition) -> SettlementBasis {
    use qsl_foundation::diagnostic::Category;
    match (disposition.stage(), disposition.category()) {
        (ClauseRunStage::Evaluate, Category::Success | Category::Violation) => {
            SettlementBasis::ClosedScope
        }
        _ => SettlementBasis::Unavailable,
    }
}

/// The one document a clause selection input names.
fn clause_input_document(input: &ClauseSelectionInput) -> &DocumentRef {
    match input {
        ClauseSelectionInput::Current { snapshot, .. }
        | ClauseSelectionInput::PreCall { snapshot, .. } => snapshot,
        ClauseSelectionInput::Invocation { invocation }
        | ClauseSelectionInput::Context { invocation, .. } => invocation,
    }
}

/// Every document admission read for `input`, in FR-106 check 1's read
/// order: the current or pre-call snapshot, or the invocation and then its
/// pre snapshot and, for a postcondition, its post snapshot (a precondition
/// reads no post side).
fn admitted_documents(
    input: &ClauseSelectionInput,
    observations: &AdmittedObservations,
) -> Vec<DocumentRef> {
    let invocation = match input {
        ClauseSelectionInput::Invocation { invocation }
        | ClauseSelectionInput::Context { invocation, .. } => Some(invocation),
        ClauseSelectionInput::Current { .. } | ClauseSelectionInput::PreCall { .. } => None,
    };
    invocation
        .into_iter()
        .cloned()
        .chain(
            [&observations.current, &observations.pre, &observations.post]
                .into_iter()
                .flatten()
                .map(|observation| observation.identity.clone()),
        )
        .collect()
}

/// FR-106 admission of `selection` for `clause`, one checked state clause of
/// `graph`: [`run_clause`]'s own admission step, shared with the tests that
/// evaluate an admitted clause under a caller-supplied `Meter`.
///
/// The model -> check edge must stay empty (FR-074-AC-3):
/// `admit_observations` (FR-106) is a `model`-layer function, so this
/// caller reads the checked clause's own facts out of
/// `check::CheckedStateClause` here, into model-level
/// `ClauseFacts`/`OperationFacts`, rather than that module importing
/// `check` itself.
pub(crate) fn admit_clause_observations(
    graph: &CheckedGraph,
    clause: &CheckedStateClause,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    model_limits: ModelNormalizationLimits,
    provisions: &Provisions<'_>,
    selection: &ClauseSelection,
    observation_limits: ObservationLimits,
) -> Result<AdmittedObservations, AdmissionFailure> {
    let clause_facts = ClauseFacts {
        identity: clause.identity(),
        kind: clause.kind(),
        context: clause.context(),
        operation: clause.operation().map(|operation| OperationFacts {
            declaring: operation.declaring,
            declaration: operation.declaration.clone(),
        }),
        operations: graph.scope().operations().clone(),
        named_populations: clause.named_reads(),
    };
    admit_observations(
        graph.model_selections(),
        graph.scope().types(),
        &clause_facts,
        packages,
        model_limits,
        provisions,
        selection,
        observation_limits,
    )
}

/// FR-109: `qsl_replay::spine::run_clause`.
pub fn run_clause(request: ClauseRunRequest) -> Result<ClauseRunReport, ClauseRunRefusal> {
    let unit = request.source.unit();
    if unit.bytes.is_empty() {
        return Err(ClauseRunRefusal::EmptySource);
    }
    // Every pre-compile disposition reports no `package_id`, and the usage is
    // empty until an evaluation meter has run.
    let report = |package_id, disposition: ClauseDisposition| ClauseRunReport {
        basis: undecided_basis(&disposition),
        witness: None,
        package_id,
        disposition,
        usage: ClauseRunUsage::default(),
    };

    if let Some(language) = unit.language {
        if NativeLanguage::of(language).is_none() {
            return Ok(report(
                None,
                ClauseDisposition::UnknownLanguage {
                    language: language.to_owned(),
                },
            ));
        }
    }

    let cancel = Cancel::new();
    let front_end = || -> Result<_, front_end::FrontEndFailure> {
        let parsed = front_end::parse(
            &front_end::ParseRequest {
                source: unit.identity,
                path: unit.path,
                bytes: unit.bytes,
            },
            request.limits.source,
            &cancel,
        )?
        .into_value();
        let models = front_end::select(&parsed, &request.packages, request.limits.model, &cancel)?
            .into_value();
        let checked = front_end::check(
            &parsed,
            &models,
            &request.dependencies,
            &super::LockEvidence::default(),
            request.limits,
            &cancel,
        )?
        .into_value();
        let emitted = front_end::package(&checked, front_end::PackageLimits::default(), &cancel)?
            .into_value();
        Ok((checked, emitted))
    };
    let (checked, emitted) = match front_end() {
        Ok(compiled) => compiled,
        Err(failure) => {
            let disposition = match front_end::refusal_or_fault(failure) {
                Ok(refusal) => ClauseDisposition::Compile(refusal),
                Err(fault) => ClauseDisposition::EvaluateFault(fault),
            };
            return Ok(report(None, disposition));
        }
    };
    let package_id = emitted.package().package_id();
    let package = checked.package();
    if let Some(expected) = request
        .expected_package_id
        .map(|id| id.hex())
        .or_else(|| request.expected_package_id_claim.clone())
    {
        if package_id.hex() != expected {
            return Ok(report(
                Some(package_id),
                ClauseDisposition::StalePackage {
                    expected,
                    actual: package_id,
                },
            ));
        }
    }
    // FND-016: the unit's and every resolved library's source, reused
    // exactly as the check already read them (mirroring `call::run`'s own
    // `sources` build), so a locus resolves without a second read of
    // already-admitted bytes.
    let mut sources = Vec::with_capacity(1 + checked.libraries().len());
    sources.push(checked.source().clone());
    sources.extend(checked.libraries().iter().cloned());

    let run = CompiledRun {
        packages: &request.packages,
        model_limits: request.limits.model,
        provisions: Provisions {
            snapshots: &request.snapshots,
            invocations: &request.invocations,
        },
        observation_limits: request.observation_limits,
        accounting: request.accounting,
        package,
        package_id,
        sources: &sources,
    };
    match request.selection {
        ClauseRunSelection::Clause(ref selection) => {
            let Some(clause) = package.graph().state_clause(&selection.name) else {
                return Ok(run.report(ClauseDisposition::MissingName {
                    name: selection.name.clone(),
                }));
            };
            Ok(check_clause(&run, clause, selection))
        }
        ClauseRunSelection::Function {
            ref name,
            ref arguments,
            ref snapshot,
        } => run_function(&run, name, arguments, snapshot),
        ClauseRunSelection::Frame {
            ref operation,
            ref invocation,
        } => Ok(run_frame(&run, operation, invocation)),
        ClauseRunSelection::Temporal {
            ref name,
            ref trace,
            ref over,
        } => Ok(temporal::run_temporal(&run, name, trace, over)),
    }
}

/// What a clause run or an FR-115 frame run reads besides its selection:
/// [`run_clause`] builds it from its request, and FR-116's frame replay and
/// FR-122's state-clause replay (`crate::execute`) from the recompiled
/// package and byte provision.
pub(crate) struct CompiledRun<'a> {
    /// FR-056's package input.
    pub(crate) packages: &'a BTreeMap<[u8; 32], Vec<u8>>,
    /// The limits admission re-reads (`intake.input_bytes`) and
    /// re-normalizes the domain packages under.
    pub(crate) model_limits: ModelNormalizationLimits,
    /// FR-106's snapshot and invocation provisions.
    pub(crate) provisions: Provisions<'a>,
    /// FR-106's observation limits.
    pub(crate) observation_limits: ObservationLimits,
    /// The evaluation meter's accounting limits.
    pub(crate) accounting: ScalarLimits,
    /// The compiled package.
    pub(crate) package: &'a qsl_package::CheckedPackage,
    /// The compiled `package_id`.
    pub(crate) package_id: PackageId,
    /// The unit's and every resolved library's source, for loci.
    pub(crate) sources: &'a [Source],
}

impl CompiledRun<'_> {
    fn report(&self, disposition: ClauseDisposition) -> ClauseRunReport {
        ClauseRunReport {
            basis: undecided_basis(&disposition),
            witness: None,
            package_id: Some(self.package_id),
            disposition,
            usage: ClauseRunUsage::default(),
        }
    }
}

/// FR-109's clause run over a resolved state clause: admit `selection` for
/// `clause` by FR-106, then evaluate the clause once through S6a (FR-107)
/// under a fresh meter built from `run.accounting`. `clause` is
/// `run.package`'s own state clause of `selection.name`. FR-122's replay
/// (`crate::execute`) runs this same evaluation.
pub(crate) fn check_clause(
    run: &CompiledRun<'_>,
    clause: &CheckedStateClause,
    selection: &ClauseSelection,
) -> ClauseRunReport {
    check_clause_admitted(run, clause, selection).report
}

/// [`check_clause`]'s report, with the observations admission produced
/// (when it admitted) and the evaluation meter, which FR-268's separation
/// check goes on charging.
pub(crate) struct ClauseCheck {
    /// The report.
    pub(crate) report: ClauseRunReport,
    /// Every snapshot or invocation document admission read, in read order.
    pub(crate) documents: Vec<DocumentRef>,
    /// The admitted observations; `None` when admission failed.
    pub(crate) observations: Option<AdmittedObservations>,
    /// The evaluation meter, after the clause's evaluation.
    pub(crate) meter: Meter,
}

/// [`check_frame`]'s report and the documents admission read.
pub(crate) struct FrameCheck {
    /// The report.
    pub(crate) report: ClauseRunReport,
    /// Every document admission read, in read order: the invocation, then
    /// its pre and post snapshots once admitted.
    pub(crate) documents: Vec<DocumentRef>,
}

/// [`check_clause`], keeping the admitted observations and the meter.
pub(crate) fn check_clause_admitted(
    run: &CompiledRun<'_>,
    clause: &CheckedStateClause,
    selection: &ClauseSelection,
) -> ClauseCheck {
    use qsl_eval::value::{CallFailure, CheckedPackageEvaluation, QualifiedName};
    use quire_semantic_value::call::InputRefusal;

    let graph = run.package.graph();
    let observations = match admit_clause_observations(
        graph,
        clause,
        run.packages,
        run.model_limits,
        &run.provisions,
        selection,
        run.observation_limits,
    ) {
        Ok(observations) => observations,
        Err(failure) => {
            return ClauseCheck {
                report: run.report(ClauseDisposition::Admit(failure)),
                documents: vec![clause_input_document(&selection.input).clone()],
                observations: None,
                meter: Meter::new(run.accounting),
            }
        }
    };
    let documents = admitted_documents(&selection.input, &observations);
    let mut meter = Meter::new(run.accounting);
    let Ok(qualified) = QualifiedName::unqualified(&selection.name) else {
        return ClauseCheck {
            report: run.report(ClauseDisposition::MissingName {
                name: selection.name.clone(),
            }),
            documents,
            observations: Some(observations),
            meter,
        };
    };
    let mut derived = None;
    let disposition = match run
        .package
        .evaluate_clause(&qualified, &observations, &mut meter)
    {
        Ok(evaluation) => match derive_separating_witness(clause, &evaluation) {
            Err(fault) => ClauseDisposition::EvaluateFault(fault),
            Ok(witness) => {
                derived = witness;
                match convert_outcome(evaluation.evaluation, graph, run.sources) {
                    Ok(outcome) => ClauseDisposition::Evaluate(outcome),
                    Err(refusal) => match *refusal {
                        RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
                        other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
                    },
                }
            }
        },
        Err(CallFailure::Input(input)) => match input {
            InputRefusal::UnknownClause(name) => ClauseDisposition::MissingName { name },
            // Unreachable in this flow: `clause` was resolved by name and
            // `observations` admitted against that same clause's identity,
            // so `evaluate_clause`'s own name/identity checks can never
            // disagree with it. A broken invariant rather than a document
            // defect, never a wildcard arm (FR-090).
            InputRefusal::ObservationsMismatch => ClauseDisposition::EvaluateFault(
                InternalFault::new("call", "clause-observations-mismatch"),
            ),
            InputRefusal::UnknownFunction(_)
            | InputRefusal::Arity { .. }
            | InputRefusal::WrongValueKind { .. }
            | InputRefusal::DuplicateClause(_)
            | InputRefusal::DanglingReference { .. } => ClauseDisposition::EvaluateFault(
                InternalFault::new("call", "evaluate-clause-supplies-no-function-style-input"),
            ),
        },
        Err(CallFailure::Fault(fault)) => ClauseDisposition::EvaluateFault(fault),
        Err(
            storage @ (CallFailure::Storage { .. }
            | CallFailure::Supplied(_)
            | CallFailure::UnsupportedHelper(_)),
        ) => ClauseDisposition::ArgumentRefusal(convert_call_failure(storage)),
        // This meter holds no `Cancel` handle, so no charge is cancelled.
        Err(CallFailure::Cancelled(_)) => ClauseDisposition::EvaluateFault(InternalFault::new(
            "call",
            "cancelled-without-a-handle",
        )),
    };
    let mut report = run.report(disposition);
    report.usage = ClauseRunUsage::from_meter(&meter, observations.usage);
    // FR-266: a `Clause` run that completed a Boolean takes FR-265's basis
    // and record; every other disposition keeps `unavailable`.
    if let (Some(derived), ClauseRunStage::Evaluate) = (derived, report.disposition.stage()) {
        if report.disposition.truth().is_some() {
            report.basis = derived.basis;
            report.witness = derived.record;
        }
    }
    ClauseCheck {
        report,
        documents,
        observations: Some(observations),
        meter,
    }
}

/// FR-115: resolve `operation` to its frame, admit `invocation` by FR-106's
/// checks 1 and 3 to 10, and check it against the frame through the
/// `ProtocolClause` S6a `evaluate` arm (FR-106's check 11).
fn run_frame(
    run: &CompiledRun<'_>,
    operation: &OperationName,
    invocation: &DocumentRef,
) -> ClauseRunReport {
    let Some((context, operation_frame)) = resolve_frame(run.package.graph(), operation) else {
        return run.report(ClauseDisposition::MissingName {
            name: operation.to_string(),
        });
    };
    check_frame(run, context, operation_frame, invocation).report
}

/// FR-104's Resolution of a `Frame` selection: `M` and `T` resolve to a
/// domain package and one of its object types, `op` names an operation of
/// that type's effective view, and the package holds its frame node only
/// when a clause or attempt names it (FR-105). `None` when any step
/// resolves nothing.
pub(crate) fn resolve_frame<'g>(
    graph: &'g CheckedGraph,
    operation: &OperationName,
) -> Option<(
    quire_exact::EffectiveId,
    &'g qsl_semantics::check::CheckedOperationFrame,
)> {
    let selection =
        graph.resolve_operation(&operation.model, &operation.object, &operation.operation)?;
    graph.operation_frame(&selection)
}

/// FR-115 over a resolved operation frame: admit `invocation` by FR-106's
/// checks 1 and 3 to 10, and check it against the frame through the
/// `ProtocolClause` S6a `evaluate` arm (FR-106's check 11). `context` and
/// `operation_frame` are [`resolve_frame`]'s answer over `run.package`.
pub(crate) fn check_frame(
    run: &CompiledRun<'_>,
    context: quire_exact::EffectiveId,
    operation_frame: &qsl_semantics::check::CheckedOperationFrame,
    invocation: &DocumentRef,
) -> FrameCheck {
    use qsl_eval::value::{CallFailure, CheckedPackageEvaluation, FrameEvaluation};
    use qsl_semantics::family::FamilyOutcome;

    let graph = run.package.graph();
    let frame = operation_frame.frame();
    let facts = FrameFacts {
        context,
        frame,
        operation: OperationFacts {
            declaring: operation_frame.operation().declaring,
            declaration: operation_frame.operation().declaration.clone(),
        },
    };
    let admitted = match admit_frame_invocation(
        graph.model_selections(),
        graph.scope().types(),
        &facts,
        run.packages,
        run.model_limits,
        &run.provisions,
        invocation,
        run.observation_limits,
    ) {
        Ok(admitted) => admitted,
        Err(failure) => {
            return FrameCheck {
                report: run.report(ClauseDisposition::Admit(failure)),
                documents: vec![invocation.clone()],
            }
        }
    };
    let documents = vec![
        admitted.invocation.clone(),
        admitted.pre.clone(),
        admitted.post.clone(),
    ];

    let mut meter = Meter::new(run.accounting);
    let disposition = match run.package.evaluate_frame(frame, &admitted, &mut meter) {
        Ok(FrameEvaluation {
            evaluation,
            witness: Some(witness),
        }) => match evaluation.outcome {
            FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(false))) => {
                ClauseDisposition::FrameViolation(witness)
            }
            _ => ClauseDisposition::EvaluateFault(InternalFault::new(
                "S6a",
                "frame-witness-without-a-false-verdict",
            )),
        },
        // FR-115: a violation always carries its witness. A false verdict
        // without one is a broken S6a invariant, never a witness-less
        // violation.
        Ok(FrameEvaluation {
            evaluation:
                qsl_eval::value::Evaluation {
                    outcome:
                        FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(false))),
                    ..
                },
            witness: None,
        }) => ClauseDisposition::EvaluateFault(InternalFault::new(
            "S6a",
            "frame-false-verdict-without-a-witness",
        )),
        Ok(FrameEvaluation {
            evaluation,
            witness: None,
        }) => match convert_outcome(evaluation, graph, run.sources) {
            Ok(outcome) => ClauseDisposition::Evaluate(outcome),
            Err(refusal) => match *refusal {
                RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
                other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
            },
        },
        // Unreachable in this flow: `admitted` was admitted for `frame`
        // just above, so `evaluate_frame`'s own mismatch check cannot fire.
        Err(CallFailure::Input(_)) => ClauseDisposition::EvaluateFault(InternalFault::new(
            "call",
            "frame-invocation-mismatch",
        )),
        Err(CallFailure::Fault(fault)) => ClauseDisposition::EvaluateFault(fault),
        Err(
            storage @ (CallFailure::Storage { .. }
            | CallFailure::Supplied(_)
            | CallFailure::UnsupportedHelper(_)),
        ) => ClauseDisposition::ArgumentRefusal(convert_call_failure(storage)),
        // This meter holds no `Cancel` handle, so no charge is cancelled.
        Err(CallFailure::Cancelled(_)) => ClauseDisposition::EvaluateFault(InternalFault::new(
            "call",
            "cancelled-without-a-handle",
        )),
    };
    let mut report = run.report(disposition);
    report.usage = ClauseRunUsage::from_meter(&meter, admitted.usage);
    FrameCheck { report, documents }
}

fn run_function(
    run: &CompiledRun<'_>,
    name: &str,
    arguments: &[ClauseArgument],
    snapshot: &DocumentRef,
) -> Result<ClauseRunReport, ClauseRunRefusal> {
    use qsl_eval::value::CheckedPackageEvaluation;
    let packages = run.packages;
    let model_limits = run.model_limits;
    let snapshots = run.provisions.snapshots;
    let observation_limits = run.observation_limits;
    let accounting = run.accounting;
    let package = run.package;
    let package_id = run.package_id;
    let sources = run.sources;
    let report = |disposition: ClauseDisposition| ClauseRunReport {
        basis: undecided_basis(&disposition),
        witness: None,
        package_id: Some(package_id),
        disposition,
        usage: ClauseRunUsage::default(),
    };

    let selected = match select(package, name) {
        Ok(selected) => selected,
        Err(refusal) => {
            return Ok(match *refusal {
                RunRefusal::MissingDeclaration { function } => {
                    report(ClauseDisposition::MissingName { name: function })
                }
                RunRefusal::UnsupportedResult { function } => {
                    report(ClauseDisposition::NotAPredicate { name: function })
                }
                other => report(ClauseDisposition::ArgumentRefusal(Box::new(other))),
            })
        }
    };
    if !matches!(
        package
            .graph()
            .callable(name)
            .map(|callable| callable.result),
        Some(ValueType::Boolean)
    ) {
        return Ok(report(ClauseDisposition::NotAPredicate {
            name: name.to_owned(),
        }));
    }

    let (environment, admission_usage) = match admit_current_snapshot(
        package.graph().model_selections(),
        package.graph().scope().types(),
        packages,
        model_limits,
        snapshots,
        snapshot,
        observation_limits,
    ) {
        Ok(admitted) => admitted,
        Err(failure) => return Ok(report(ClauseDisposition::Admit(failure))),
    };

    // Bind each argument to its parameter (FR-100's own name-binding
    // refusals), then convert to a value: a canonical integer as FR-100
    // does, or an object reference resolved in `environment`.
    let mut slots: Vec<Option<&ClauseArgumentValue>> = vec![None; selected.parameters.len()];
    for argument in arguments {
        let Some(position) = selected
            .parameters
            .iter()
            .position(|(parameter, _)| *parameter == argument.parameter)
        else {
            return Ok(report(ClauseDisposition::ArgumentRefusal(Box::new(
                RunRefusal::UnknownParameter {
                    parameter: argument.parameter.clone(),
                },
            ))));
        };
        if slots[position].replace(&argument.value).is_some() {
            return Ok(report(ClauseDisposition::ArgumentRefusal(Box::new(
                RunRefusal::DuplicateArgument {
                    parameter: argument.parameter.clone(),
                },
            ))));
        }
    }
    let mut bound = Vec::with_capacity(selected.parameters.len());
    for (position, (parameter, value_type)) in selected.parameters.iter().enumerate() {
        let Some(value) = slots[position] else {
            return Ok(report(ClauseDisposition::ArgumentRefusal(Box::new(
                RunRefusal::UnboundParameter {
                    parameter: parameter.clone(),
                },
            ))));
        };
        let converted = match (value, value_type) {
            (ClauseArgumentValue::Integer(0), ValueType::Boolean) => Value::Boolean(false),
            (ClauseArgumentValue::Integer(1), ValueType::Boolean) => Value::Boolean(true),
            (ClauseArgumentValue::Integer(value), ValueType::Integer | ValueType::Int(_)) => {
                Value::Integer(quire_exact::Integer::from(*value))
            }
            (
                ClauseArgumentValue::Reference { population, key },
                ValueType::Reference(type_identity),
            ) => {
                let universe = match population_universe_for(
                    package.graph().model_selections(),
                    packages,
                    model_limits,
                    *type_identity,
                ) {
                    Ok(universe) => universe,
                    Err(failure) => {
                        return Ok(report(ClauseDisposition::Admit(failure)));
                    }
                };
                let Some(reference) = environment.objects().find(universe, key) else {
                    let mut fields = BTreeMap::new();
                    fields.insert("population", population.clone());
                    fields.insert("object", key.clone());
                    return Ok(report(ClauseDisposition::Admit(AdmissionFailure::Refused(
                        AdmissionRecord {
                            code: qsl_foundation::diagnostic::Code::InvalidRuntimeInput,
                            cause: "wrong-role-mapping",
                            fields,
                            path: Vec::new(),
                            kernel: None,
                        },
                    ))));
                };
                Value::Reference(reference.clone())
            }
            _ => {
                return Ok(report(ClauseDisposition::ArgumentRefusal(Box::new(
                    RunRefusal::WrongValueKind { position },
                ))))
            }
        };
        bound.push(converted);
    }

    let mut meter = Meter::new(accounting);
    let disposition = match package.call(&selected.name, bound, &environment, &mut meter) {
        Ok(evaluation) => match convert_outcome(evaluation, package.graph(), sources) {
            Ok(outcome) => ClauseDisposition::Evaluate(outcome),
            Err(refusal) => match *refusal {
                RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
                other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
            },
        },
        Err(failure) => match *convert_call_failure(failure) {
            RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
            other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
        },
    };
    let mut result = report(disposition);
    result.usage = ClauseRunUsage::from_meter(&meter, admission_usage);
    Ok(result)
}

#[cfg(test)]
mod tests;
