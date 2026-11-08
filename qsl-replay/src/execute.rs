// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-098 (ADR-013 O-26, C-13, TK-01; ADR-011 §2.1 E9, §4, §6.1): the
//! replay executor entry, [`replay`].
//!
//! It reads the request (FR-071), recompiles the one source unit the
//! package reference names through the spine ([`crate::spine::parse`],
//! S1 to S4) from the byte provision, selects the function by
//! its `QualifiedName` in the recompiled package's declarations, joins the
//! replay source's arguments to the function's parameters by parameter node
//! id (an id names a parameter only by lookup in the recompiled
//! package), calls it through S6a (`CheckedPackage::call`) and
//! settles the verdict against the counterexample's. No `CheckedPackage` is
//! built from wire bytes, and nothing is read from a path, environment
//! variable or search location.
//!
//! Every failure before a verdict is a [`ReplayRefusal`], and no partial
//! result accompanies it. A replay that runs and disagrees with the
//! counterexample, including an S6a outcome that completed no value,
//! settles `inconclusive` with its typed cause and is never repaired
//! ([`crate::ReplayResult`]).

use qsl_eval::temporal::{evaluate_observed, TemporalPosition, TemporalRefusal, TemporalTrace};
use qsl_eval::value::{
    input_refusal_code, supplied_stop_cause, supplied_stop_code, CheckedPackageEvaluation,
};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{Code, SourceIdentity};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{Capability, CheckedInterval};
use qsl_semantics::library::LibraryName;
use qsl_semantics::model::intake::package_input;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::environment_stage::{membership_failure_cause, membership_failure_code};
use quire_eval::CallFailure;
use quire_eval::FamilyOutcome;
use quire_exact::{
    Cancel, CancelCause, LimitKind, Meter, NodeKey, Origin, Outcome, Role, ScalarLimits, Value,
    ValueType,
};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::declaration::EnvironmentFailure;
use quire_semantic_value::location::Location;
use quire_semantic_value::supplied_stop::{
    SuppliedCause, SuppliedFailure, SuppliedRun, SuppliedStop, UnsupportedHelperStop,
};

use crate::bounds::ReplayLimits;
use crate::identity::{DefinitionSourceRef, ObligationIdentity, ProfileSelection, QualifiedName};
use crate::limits::CallerLimits;
use crate::request::StageLimits;
use crate::request::{ReplayRequest, ReplayRequestRefusal, ReplayRequestWire};
use crate::result::{
    EvaluatedValue, InputArmResult, ObservedTemporalResult, ReplayResult, Verdict,
    WitnessArmResult, WitnessCheck,
};
use crate::spine::{
    self, CompileRefusal, DependencyInput, DependencyInputRefusal, ParseRequest, SpineLimits,
    SuppliedLibrary,
};
use crate::target::{ReplayEntry, ReplayTarget, TargetMember};
use crate::witness::{
    DecodeRefusal, EnvelopeSource, FrameOperation, ReplaySource, WitnessBinding, WitnessValue,
    WitnessValueType,
};
use argument::Stopped;
use qsl_forms::StateClauseKind;
use qsl_foundation::Setting;
use qsl_semantics::model::observation::{
    admit_trace_snapshot, population_universe_for, AdmissionFailure, DocumentRef, ObservationForm,
};
use quire_exact::Identifier;
use quire_semantic_value::catalog::Category;
use std::borrow::Cow;
use std::collections::BTreeSet;

mod argument;
mod object_environment;
pub use object_environment::{ObjectEnvironmentFailure, Unresolved};
pub(crate) mod frame;
pub use frame::{replay_frame, FrameIdentityMismatch, FrameReplayResult};

mod composite_domain;
mod composite_parity;
mod composite_site;
mod operator_parity;
mod parity_identity;
pub(crate) use parity_identity::process_provider_obligation;
pub use parity_identity::{
    parity_obligation, value_parity_obligation, BoundEntries, Domain, IdentityEncodeError,
    ParityArgument, ParityPreimage, ValueParityArgument, ValueParityPreimage,
};
pub(crate) mod scalar_site;
pub use composite_domain::ParityBoundRefusal;
pub use composite_parity::{replay_composite_parity, settle_verified_shadow};
pub use operator_parity::{replay_operator_parity, ScalarIdentityMismatch};

mod value_parity;
pub use value_parity::{replay_value_parity, ValueParityReport, ValueParityResult};

mod state_clause;
pub use state_clause::{replay_state_clause, ClauseIdentityMismatch, StateClauseReplayResult};
#[cfg(test)]
pub(crate) use state_clause::{separate, settle_separation, stopped_reason, SeparationOutcome};

/// The S1 limit a request names that is above this executor's reader
/// limit (ADR-013 O-26: "a limit above the reader limit").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LimitAboveReader {
    /// The request's `s1.input_bytes` entry.
    pub requested: u64,
    /// The reader limit: `replay.input_bytes`, the most bytes any admitted
    /// request can carry.
    pub reader: u64,
}

/// Why a `frame_operation` object is not the operation's declaring object
/// (FR-116, QSpec FR-323).
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum MalformedOperationReason {
    /// The object names a declaration that is no operation owner (not an
    /// object type, a process or a state machine: a value type, an enum, a
    /// record). The variant keeps its name from before processes and state
    /// machines were owners.
    #[error("the object names a declaration that cannot declare operations")]
    NotAnObjectType,
    /// The object only inherits the operation; the target names the
    /// declaring object.
    #[error("the object only inherits the operation")]
    OnlyInherited,
}

/// Why [`replay`] settled no verdict. Each variant is one ADR-013 O-26
/// refusal, with its typed cause; none carries a partial result.
#[derive(Debug, thiserror::Error)]
pub enum ReplayRefusal {
    /// FR-071: the request did not decode -- an unknown version, an input
    /// absent from the byte provision, or an encoding above the reader
    /// bound.
    #[error(transparent)]
    Request(#[from] ReplayRequestRefusal),
    /// The request's S1 source limit is above the reader limit: the request
    /// asks for more than the reader allows, which is invalid input
    /// (`invalid-request`, exit 20), not an exhausted limit.
    #[error(
        "invalid-request: the request's `s1.input_bytes` {} is above the reader limit `replay.input_bytes` {}",
        .0.requested,
        .0.reader
    )]
    LimitAboveReader(LimitAboveReader),
    /// The package name is not a library name (it is empty).
    #[error("invalid-request: the package name is not a library name: {0}")]
    InvalidPackageName(qsl_semantics::library::InvalidLibraryName),
    /// The package reference does not name exactly one source unit. The
    /// spine recompiles one unit: a package over more than one waits on
    /// E3 import resolution (FR-087-AC-13).
    #[error("the package reference names {0} source units; the spine recompiles exactly one")]
    SourceCount(usize),
    /// FR-480: the package reference's source list is empty, repeats an
    /// owner, or names a document from the byte provision.
    #[error("invalid_package/invalid-value at /package/lock/sources: {0}")]
    PackageSources(&'static str),
    /// FR-098: one dependency entry's source list is empty, repeats an owner,
    /// or names a definition document.
    #[error("invalid_package/invalid-value at /dependencies/{index}/sources: {reason}")]
    DependencySources {
        /// The dependency entry's zero-based index.
        index: usize,
        /// Why the source list is invalid.
        reason: &'static str,
    },
    /// A defect of a `refinement_internals` value found by the recompile
    /// (QSpec FR-323-AC-17): `source` is the recompile refusal, and `locus`
    /// is the RFC 6901 pointer into the request, either
    /// `/package/lock/refinement_internals<pointer>` or
    /// `/dependencies/<n>/refinement_internals<pointer>`.
    #[error("{source} (at {locus})")]
    RefinementInternals {
        /// The request pointer of the offending value.
        locus: String,
        /// The recompile refusal, a `Recompile` or
        /// `RecompileWithProtocolResults`.
        source: Box<ReplayRefusal>,
    },
    /// The recompile refused at one spine stage, or reached one of the
    /// stage limits the request carries (`stage_limit_exceeded`).
    #[error("the recompile refused at stage {stage}: {refusal}", stage = .0.stage().as_str(), refusal = .0)]
    Recompile(Box<CompileRefusal>),
    /// A compile stage reached an attributable protocol outcome and retained
    /// the ordered declaration results with its primary refusal or limit.
    #[error("the recompile refused at stage {refusal_stage}: {refusal}", refusal_stage = .refusal.stage().as_str(), refusal = .refusal)]
    RecompileWithProtocolResults {
        /// The primary stage refusal or limit, which determines category.
        refusal: Box<CompileRefusal>,
        /// Ordered per-protocol results through the stage that stopped.
        protocol_results: Vec<spine::ProtocolResult>,
    },
    /// QC-1: the request's domain package documents do not form a package
    /// input (an unreadable document, or two for one identity).
    #[error("{}", .0.detail)]
    DomainPackages(Box<qsl_semantics::model::normalize::ModelRefusal>),
    /// ADR-015 D-4 rules 1 and 6: the package reference's `dependencies`
    /// are not in strictly ascending UTF-8 byte order of identity, or name
    /// an identity the recompiled package does not select
    /// (`invalid_package`/`invalid-value` at `/dependencies`).
    #[error("invalid_package/invalid-value at /dependencies: {0}")]
    DependencySelections(DependencySelectionsCause),
    /// ADR-015 D-4 rule 3: the `dependencies` entries do not form a
    /// dependency input (ADR-015 D-1).
    #[error("the package reference's dependencies are no dependency input: {0}")]
    DependencyInput(DependencyInputRefusal),
    /// The selection names no function node of the recompiled package.
    #[error(
        "missing_declaration/missing-name: package {package} declares no function {selection}"
    )]
    UnknownFunction {
        /// The request's selection.
        selection: QualifiedName,
        /// The recompiled package it was looked up in.
        package: LibraryName,
    },
    /// FR-098, FR-121: a `function` request names an owner the unit does not
    /// import directly, so the dependency input holds no package for it
    /// (`missing_import`/`missing-selection` at `/replay/function_owner`).
    #[error("missing_import/missing-selection at /replay/function_owner: package {package} imports no library {owner}")]
    UnknownOwner {
        /// The request's `function_owner`.
        owner: LibraryName,
        /// The recompiled package that was asked for the owner.
        package: LibraryName,
    },
    /// FR-098, FR-121: the owner's function has a package-dependent
    /// signature (ADR-015 D-5), so no importing package may call it.
    #[error("ill_typed/operator-ineligible: package {package} function {selection} has a package-dependent signature")]
    NotImportable {
        /// The request's selection.
        selection: QualifiedName,
        /// The owner's package name.
        package: LibraryName,
    },
    /// FR-098: this entry replays the unit's own function and reads no
    /// `function_owner`.
    #[error("invalid_runtime_input/unknown-member at /replay/function_owner: the request's target is {target}, but `{entry}` replays only the unit's own functions")]
    ImportedFunctionTarget {
        /// The request's replay target.
        target: Box<ReplayTarget>,
        /// The entry that refused it.
        entry: ReplayEntry,
    },
    /// FR-098 "The claim a function request decides": the request's
    /// `occurrence_key` selects no claim of the selected function. It is
    /// neither the function's `declaration` occurrence nor the key of a
    /// `value-validity` record the function owns (a key whose node the
    /// package does not hold, another function's declaration, an occurrence
    /// holding no record, a record of another kind or owner). Refused before
    /// any call, with no charge.
    #[error("missing_declaration/missing-name: the occurrence {key} selects no claim of the function {function}")]
    ClaimOccurrence {
        /// The request's occurrence key.
        key: OccurrenceKey,
        /// The selected function.
        function: QualifiedName,
    },
    /// An argument names a node that is not a parameter of the selected
    /// function.
    #[error("invalid_runtime_input: node {0} is not a parameter of the selected function")]
    UnknownParameter(NodeKey),
    /// Two arguments name the same parameter.
    #[error("invalid_runtime_input: parameter {0} is bound twice")]
    DuplicateArgument(NodeKey),
    /// A parameter of the selected function has no argument.
    #[error("invalid_runtime_input: parameter {0} has no argument")]
    UnboundParameter(NodeKey),
    /// The backend witness does not decode against the selected function's
    /// parameters. Carries the obligation the request replays (ADR-013
    /// O-25), so a caller replaying many obligations knows which failed.
    #[error("the witness for obligation {obligation} does not decode: {refusal}")]
    Witness {
        /// The request's obligation identity (ADR-013 O-09).
        obligation: ObligationIdentity,
        /// The decode's cause.
        refusal: DecodeRefusal,
    },
    /// An argument refused admission as `WrongValueKind`: its value is not
    /// of the parameter's declared type, whether that is found before the
    /// call (an integer bound to a Boolean parameter, a Boolean to an
    /// integer one, or any value to a parameter of a kind no witness value
    /// is) or by S6a admission in
    /// `CheckedPackage::call` (a value outside the declared domain, such as
    /// `12` for `Int[0, 9]`) -- carried as ADR-011 §2.3's
    /// `StageFailure::Refused` cause.
    #[error("{code} ({cause}): {refusal}", code = input_refusal_code(&.0).as_str(), cause = .0.cause(), refusal = .0)]
    Input(InputRefusal),
    /// FR-056: a UUID or Timestamp argument whose payload text the kernel's
    /// canonical constructor refuses, `invalid_runtime_input`/`invalid-value`
    /// with the kernel's class beneath it (`refusal.cause`): noncanonical
    /// UUID text, noncanonical Timestamp text, or a canonical Timestamp
    /// outside the signed 128-bit domain. No value is built.
    #[error("invalid_runtime_input (invalid-value): the argument at position {parameter} is refused: {refusal}")]
    NativePayload {
        /// The parameter's zero-based declared position.
        parameter: usize,
        /// The kernel's typed refusal.
        refusal: quire_exact::ConstructionRefusal,
    },
    /// An argument's membership in its parameter's declared type was not
    /// decided because the type environment could not reserve storage:
    /// `resource_exhausted`, never an [`Self::Input`] wrong-value-kind
    /// refusal.
    #[error("the argument at position {position} was not decided: {failure:?}")]
    Storage {
        /// The parameter's zero-based declared position.
        position: usize,
        /// The kernel's storage failure.
        failure: EnvironmentFailure,
    },
    /// FR-277: a supplied-value phase stopped before the call on invalid
    /// input, the original cancellation, a failed reservation or an
    /// unrepresentable size, with the phase context (a helper-ceiling stop
    /// settles `inconclusive`/`NoValue` instead, see
    /// [`ReplayResult::pre_call_limit`]). No evaluation ran.
    #[error("supplied-value {phase} stopped before the call: {cause:?}", phase = .0.context.phase.as_str(), cause = .0.cause)]
    Supplied(Box<SuppliedStop>),
    /// FR-277: the supplied-value walk reached a helper family QSV has not
    /// qualified, before that helper ran. Distinct from invalid input, a
    /// limit, a cancellation, storage and capacity.
    #[error("supplied-value {phase} reached the unqualified {family:?} helper", phase = .0.context.phase.as_str(), family = .0.family)]
    UnsupportedHelper(Box<UnsupportedHelperStop>),
    /// The caller's [`Cancel`] stopped the evaluation (ADR-029 LC-3): no
    /// value and no verdict.
    #[error("the replay was cancelled: {0:?}")]
    Cancelled(CancelCause),
    /// The selected function's declared result is not `Boolean`, so it
    /// states no property a counterexample refutes. Refused before the
    /// call, from the declaration alone.
    #[error("the selected function {selection} of package {package} is not a predicate: its declared result is not Boolean")]
    NotAPredicate {
        /// The request's selection.
        selection: QualifiedName,
        /// The recompiled package that declares it.
        package: LibraryName,
    },
    /// FR-357: the selected function's declared result is `Boolean`, so its
    /// claim is a predicate's, replayed by [`replay`], not a value-parity
    /// claim. Refused before the call, from the declaration alone.
    #[error("the selected function {selection} of package {package} is a predicate: its declared result is Boolean, so it has no value-parity replay")]
    NotAValueFunction {
        /// The request's selection.
        selection: QualifiedName,
        /// The recompiled package that declares it.
        package: LibraryName,
    },
    /// FR-357: an operator-level claim's scalar node, enclosing function or
    /// operator is not the recompiled package's.
    #[error("stale_dependency/content-mismatch: {0}")]
    ScalarIdentity(Box<ScalarIdentityMismatch>),
    /// FR-358: a request's declared domain or a claim's harness bound does
    /// not fit the claimed node's operands (naming the domain key), or the
    /// operand types hold more positions than the check stage allows.
    #[error("{0}")]
    ParityBound(Box<ParityBoundRefusal>),
    /// FR-098, FR-116, FR-122: the request's replay target is not the member
    /// the entry reads (`invalid_runtime_input`/`unknown-member` at the
    /// target member's `/replay/<member>` pointer). The code and pointer
    /// tell it apart from QSpec FR-323's refusal of a request carrying two
    /// targets (`invalid-value`).
    #[error("invalid_runtime_input/unknown-member at {}: the request's target is {target}, but `{entry}` reads `{member}`", .target.member().pointer())]
    WrongReplayTarget {
        /// The request's replay target.
        target: Box<ReplayTarget>,
        /// The entry that refused it.
        entry: ReplayEntry,
        /// The member the entry reads.
        member: TargetMember,
    },
    /// FR-331: a temporal counterexample arm requires a `state_clause`
    /// target and reports the observed-counterexample member as the one
    /// this entry cannot read.
    #[error("invalid_runtime_input/unknown-member at /replay/observed_counterexample: replay target {target} is not a state clause")]
    WrongObservedTemporalTarget {
        /// The request's replay target.
        target: Box<ReplayTarget>,
    },
    /// FR-331: observed temporal replay requires a witness source.
    #[error("invalid_runtime_input/invalid-value at /replay/source: observed temporal replay requires a Witness source")]
    WrongObservedTemporalSource,
    /// FR-170 replay requires its explicit subject argument.
    #[error("invalid_runtime_input/invalid-value: model-graph replay requires replay_model_graph and its subject")]
    WrongModelGraphEntry,
    /// FR-170: model-graph envelope metadata differs from the request or
    /// the recompiled claim.
    #[error("stale_dependency/content-mismatch: model-graph {member} differs from the replay request or recompiled claim")]
    ModelGraphMismatch {
        /// The mismatching envelope member.
        member: &'static str,
    },
    /// FR-170: the explicit state-graph replay entry received another arm.
    #[error("invalid_runtime_input/invalid-value at /replay/source: state-graph replay requires a ModelGraph source")]
    WrongModelGraphSource,
    /// FR-116, FR-122: the envelope's replay target differs from the
    /// request's (`stale_dependency`/`content-mismatch`), naming both.
    #[error("stale_dependency/content-mismatch: the request's target is {request} but the envelope's is {envelope}")]
    TargetMismatch {
        /// The request's replay target.
        request: Box<ReplayTarget>,
        /// The envelope's replay target.
        envelope: Box<ReplayTarget>,
    },
    /// FR-116: the request's `frame_operation` names no operation frame of
    /// the recompiled package.
    #[error(
        "missing_declaration/missing-name: package {package} holds no operation frame {operation}"
    )]
    UnknownOperation {
        /// The request's operation.
        operation: FrameOperation,
        /// The recompiled package it was looked up in.
        package: LibraryName,
    },
    /// FR-116: the request's `frame_operation` names an object that cannot
    /// be its operation's declaring object (`invalid_model_binding`/
    /// `malformed-declaration`).
    #[error("invalid_model_binding/malformed-declaration: package {package} cannot select operation {operation}: {reason}")]
    MalformedOperationTarget {
        /// The request's operation.
        operation: FrameOperation,
        /// The recompiled package it was looked up in.
        package: LibraryName,
        /// Why the object cannot be the declaring object.
        reason: MalformedOperationReason,
    },
    /// FR-116: two supertypes of the request's `frame_operation` object
    /// expose the operation's name (`ambiguous_declaration`/
    /// `ambiguous-name`).
    #[error("ambiguous_declaration/ambiguous-name: package {package} exposes operation {operation} through two supertypes")]
    AmbiguousOperation {
        /// The request's operation.
        operation: FrameOperation,
        /// The recompiled package it was looked up in.
        package: LibraryName,
    },
    /// FR-116: a frame counterexample's anchor, frame or occurrence
    /// identity is not the recompiled package's (refused before any
    /// admission), or its envelope's clause node or occurrence key is not
    /// the payload's frame node or occurrence (refused before recompiling).
    #[error("stale_dependency/content-mismatch: {0}")]
    FrameIdentity(Box<FrameIdentityMismatch>),
    /// FR-122: the request's `state_clause` names no state clause of the
    /// recompiled package (a function's name included).
    #[error(
        "missing_declaration/missing-name: package {package} declares no state clause {clause}"
    )]
    UnknownClause {
        /// The request's `state_clause`.
        clause: QualifiedName,
        /// The recompiled package it was looked up in.
        package: LibraryName,
    },
    /// FR-331: an observed trace carries no enabledness with which to check
    /// this fairness constraint, so the formula is not evaluated.
    #[error("unsupported_projection/missing-fairness-premise: observed trace cannot check {constraint:?}")]
    MissingFairnessPremise {
        /// The first member of the checked clause's fairness set.
        constraint: quire_semantic_value::fairness::FairnessConstraint,
    },
    /// FR-331: a profile, interval or checked temporal identity in the
    /// packet differs from the recompiled clause.
    #[error("stale_dependency/content-mismatch: observed temporal {member} differs from the recompiled clause")]
    ObservedTemporalMismatch {
        /// The member that differs.
        member: &'static str,
    },
    /// FR-331: the observed trace shape or requested position is invalid.
    #[error("invalid_runtime_input/invalid-value at {pointer}: {reason}")]
    ObservedTemporalInput {
        /// The observed-counterexample member that failed.
        pointer: &'static str,
        /// Why the member is invalid.
        reason: &'static str,
    },
    /// FR-331: a refusal from the temporal evaluator.
    #[error(transparent)]
    ObservedTemporalRefusal(#[from] TemporalRefusal),
    /// FR-122: a state-clause counterexample's envelope names a clause node
    /// or `claim` occurrence that is not the recompiled clause's (refused
    /// before any admission).
    #[error("stale_dependency/content-mismatch: {0}")]
    ClauseIdentity(Box<ClauseIdentityMismatch>),
    /// FR-122: a state-clause counterexample's observation form is not the
    /// one its clause's kind takes (refused before any admission).
    #[error("wrong_snapshot/wrong-observation: a {kind:?} clause takes no {form} observation")]
    WrongObservation {
        /// The resolved clause's kind.
        kind: StateClauseKind,
        /// The payload's observation form.
        form: ObservationForm,
    },
    /// FR-116, FR-122: FR-106 admission of the counterexample's documents
    /// failed, with its own record.
    #[error(transparent)]
    Admission(AdmissionFailure),
    /// FR-098 step 1: the request carries an `object_environment` and no
    /// parameter of the selected function holds a `Reference`
    /// (`invalid_runtime_input`/`unknown-member`, naming
    /// `object_environment`).
    #[error("invalid_runtime_input/unknown-member: `object_environment` is supplied for a function with no Reference parameter")]
    UnusedObjectEnvironment,
    /// FR-484: a function-value parity request carries `profile_selections`.
    /// A value-parity claim has no proving run, so the list is fixed empty
    /// and lock rows bind through the package alone
    /// (`invalid_runtime_input`/`unknown-member`). Refused before the
    /// recompile, with no call and no charge.
    #[error("invalid_runtime_input/unknown-member: `profile_selections` is supplied for a function-value parity claim, which has no proving run")]
    UnusedProfileSelections {
        /// The request's selections, in request order.
        selections: Vec<ProfileSelection>,
    },
    /// FR-098 step 2, FR-100: a reference argument's universe is not the
    /// universe of its declared `Reference<T>`'s object type
    /// (`foreign_reference`/`foreign-universe`).
    #[error("foreign_reference/foreign-universe: the reference at position {position} is in universe {supplied:?}, not {required:?}")]
    ForeignReference {
        /// The parameter's zero-based declared position.
        position: usize,
        /// The universe of the declared object type.
        required: quire_exact::UniverseId,
        /// The universe the reference names.
        supplied: quire_exact::UniverseId,
    },
    /// FR-098 steps 3 and 4: FR-106 admission of the environment document, or
    /// FR-364's `resolve` of a reference argument, refused.
    #[error("the object environment is refused: {0}")]
    ObjectEnvironment(ObjectEnvironmentFailure),
    /// FR-128: a model counterexample's replay refused.
    #[error("{0}")]
    ModelTrace(Box<crate::model_check::replay::ModelTraceRefusal>),
    /// FR-128, FR-131: a model counterexample's lasso is unfair under a
    /// constraint of the clause's fairness set, the first that fails.
    #[error("invalid_runtime_input/invalid-value: the lasso is unfair under {constraint:?}")]
    UnfairLasso {
        /// The first constraint of the fairness set the loop fails.
        constraint: quire_semantic_value::fairness::FairnessConstraint,
    },
    /// An executor or S6a invariant broke.
    #[error("internal fault in {}: {}", .0.stage(), .0.invariant())]
    Fault(InternalFault),
}

impl ReplayRefusal {
    /// The catalog code of this refusal: the request's, the recompile
    /// stage's or S6a admission's own where one refused, and the executor's
    /// otherwise.
    pub fn code(&self) -> Code {
        match self {
            Self::Request(refusal) => refusal.code(),
            Self::LimitAboveReader(_) => Code::InvalidRequest,
            Self::SourceCount(_) => Code::InvalidRequest,
            Self::PackageSources(_) => Code::InvalidPackage,
            Self::DependencySources { .. } => Code::InvalidPackage,
            Self::InvalidPackageName(_) => Code::InvalidRequest,
            Self::RefinementInternals { source, .. } => source.code(),
            Self::Recompile(refusal) => refusal.code(),
            Self::RecompileWithProtocolResults { refusal, .. } => refusal.code(),
            Self::DomainPackages(refusal) => refusal.code,
            Self::DependencySelections(_) => Code::InvalidPackage,
            Self::DependencyInput(refusal) => refusal.code(),
            Self::UnknownFunction { .. }
            | Self::UnknownOperation { .. }
            | Self::ClaimOccurrence { .. } => Code::MissingDeclaration,
            Self::UnknownOwner { .. } => Code::MissingImport,
            Self::NotImportable { .. } => Code::IllTyped,
            Self::ImportedFunctionTarget { .. } => Code::InvalidRuntimeInput,
            Self::MissingFairnessPremise { .. } => Code::UnsupportedProjection,
            Self::ObservedTemporalMismatch { .. } => Code::StaleDependency,
            Self::ModelGraphMismatch { .. } => Code::StaleDependency,
            Self::ObservedTemporalInput { .. } => Code::InvalidRuntimeInput,
            Self::ObservedTemporalRefusal(TemporalRefusal::Fault(_)) => Code::RuntimeInvariant,
            Self::ObservedTemporalRefusal(_) => Code::InvalidRuntimeInput,
            Self::WrongReplayTarget { .. } => Code::InvalidRuntimeInput,
            Self::WrongObservedTemporalTarget { .. } | Self::WrongObservedTemporalSource => {
                Code::InvalidRuntimeInput
            }
            Self::WrongModelGraphEntry | Self::WrongModelGraphSource => Code::InvalidRuntimeInput,
            Self::MalformedOperationTarget { .. } => Code::InvalidModelBinding,
            Self::AmbiguousOperation { .. } => Code::AmbiguousDeclaration,
            Self::FrameIdentity(_)
            | Self::ClauseIdentity(_)
            | Self::ScalarIdentity(_)
            | Self::TargetMismatch { .. } => Code::StaleDependency,
            Self::UnknownClause { .. } => Code::MissingDeclaration,
            Self::UnusedObjectEnvironment | Self::UnusedProfileSelections { .. } => {
                Code::InvalidRuntimeInput
            }
            Self::ForeignReference { .. } => Code::ForeignReference,
            Self::ObjectEnvironment(failure) => failure.code(),
            Self::ParityBound(refusal) => refusal.code(),
            Self::WrongObservation { .. } => Code::WrongSnapshot,
            Self::Admission(
                AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record),
            ) => Code::from_code(record.catalog.code()).unwrap_or(Code::RuntimeInvariant),
            Self::Admission(AdmissionFailure::Fault(_)) => Code::RuntimeInvariant,
            Self::UnfairLasso { .. }
            | Self::UnknownParameter(_)
            | Self::DuplicateArgument(_)
            | Self::UnboundParameter(_)
            | Self::Witness { .. }
            | Self::NotAPredicate { .. }
            | Self::NotAValueFunction { .. } => Code::InvalidRuntimeInput,
            Self::Input(refusal) => input_refusal_code(refusal),
            Self::NativePayload { .. } => Code::InvalidRuntimeInput,
            Self::Storage { failure, .. } => membership_failure_code(failure),
            Self::Supplied(stop) => supplied_stop_code(stop),
            Self::UnsupportedHelper(_) => Code::UnsupportedConstruct,
            Self::Cancelled(_) => Code::Cancelled,
            Self::ModelTrace(refusal) => refusal.code(),
            Self::Fault(_) => Code::RuntimeInvariant,
        }
    }

    /// The catalog cause tag under [`Self::code`], where the refusing stage
    /// or request check names one.
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::Request(refusal) => refusal.cause(),
            Self::RefinementInternals { source, .. } => source.cause(),
            Self::Recompile(refusal) => refusal.cause(),
            Self::RecompileWithProtocolResults { refusal, .. } => refusal.cause(),
            Self::DomainPackages(refusal) => Some(refusal.cause.as_str()),
            Self::DependencySelections(_) => Some("invalid-value"),
            Self::PackageSources(_) => Some("invalid-value"),
            Self::DependencySources { .. } => Some("invalid-value"),
            Self::DependencyInput(refusal) => refusal.cause(),
            Self::UnknownFunction { .. }
            | Self::UnknownOperation { .. }
            | Self::ClaimOccurrence { .. }
            | Self::UnknownClause { .. } => Some("missing-name"),
            Self::UnknownOwner { .. } => Some("missing-selection"),
            Self::NotImportable { .. } => Some("operator-ineligible"),
            Self::ImportedFunctionTarget { .. } => Some("unknown-member"),
            Self::MissingFairnessPremise { .. } => Some("missing-fairness-premise"),
            Self::ObservedTemporalMismatch { .. } => Some("content-mismatch"),
            Self::ModelGraphMismatch { .. } => Some("content-mismatch"),
            Self::ObservedTemporalInput { .. } => Some("invalid-value"),
            Self::ObservedTemporalRefusal(TemporalRefusal::Fault(_)) => None,
            Self::ObservedTemporalRefusal(_) => Some("invalid-value"),
            Self::FrameIdentity(_)
            | Self::ClauseIdentity(_)
            | Self::ScalarIdentity(_)
            | Self::TargetMismatch { .. } => Some("content-mismatch"),
            Self::WrongReplayTarget { .. } => Some("unknown-member"),
            Self::WrongObservedTemporalTarget { .. } => Some("unknown-member"),
            Self::WrongObservedTemporalSource => Some("invalid-value"),
            Self::WrongModelGraphEntry | Self::WrongModelGraphSource => Some("invalid-value"),
            Self::MalformedOperationTarget { .. } => Some("malformed-declaration"),
            Self::AmbiguousOperation { .. } => Some("ambiguous-name"),
            Self::WrongObservation { .. } => Some("wrong-observation"),
            Self::ModelTrace(refusal) => Some(refusal.cause()),
            Self::UnusedObjectEnvironment | Self::UnusedProfileSelections { .. } => {
                Some("unknown-member")
            }
            Self::ForeignReference { .. } => Some("foreign-universe"),
            Self::ObjectEnvironment(failure) => failure.cause(),
            Self::Input(refusal) => Some(refusal.cause()),
            Self::NativePayload { .. } | Self::UnfairLasso { .. } => Some("invalid-value"),
            Self::Storage { failure, .. } => membership_failure_cause(failure),
            Self::Supplied(stop) => supplied_stop_cause(stop),
            Self::Admission(
                AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record),
            ) => Some(record.catalog.cause()),
            Self::LimitAboveReader(_)
            | Self::SourceCount(_)
            | Self::InvalidPackageName(_)
            | Self::Admission(AdmissionFailure::Fault(_))
            | Self::UnknownParameter(_)
            | Self::DuplicateArgument(_)
            | Self::UnboundParameter(_)
            | Self::Witness { .. }
            | Self::NotAPredicate { .. }
            | Self::NotAValueFunction { .. }
            | Self::ParityBound(_)
            | Self::UnsupportedHelper(_)
            | Self::Cancelled(_)
            | Self::Fault(_) => None,
        }
    }

    /// Per-protocol results retained by a compile-stage refusal, when the
    /// stage could attribute the result to protocol declarations.
    pub fn protocol_results(&self) -> Option<&[spine::ProtocolResult]> {
        match self {
            Self::RefinementInternals { source, .. } => source.protocol_results(),
            Self::RecompileWithProtocolResults {
                protocol_results, ..
            } => Some(protocol_results),
            _ => None,
        }
    }

    /// The RFC 6901 pointer of a refusal that names a request or observed
    /// counterexample member. The member names are the wire's, so this is a
    /// string edge.
    #[ix_trace_rs::string_edge]
    pub fn pointer(&self) -> Option<Cow<'static, str>> {
        match self {
            Self::Request(refusal) => refusal.pointer().map(Cow::Borrowed),
            Self::WrongReplayTarget { target, .. } => {
                Some(Cow::Borrowed(target.member().pointer()))
            }
            Self::WrongObservedTemporalTarget { .. } => {
                Some(Cow::Borrowed("/replay/observed_counterexample"))
            }
            Self::WrongObservedTemporalSource => Some(Cow::Borrowed("/replay/source")),
            Self::WrongModelGraphEntry | Self::WrongModelGraphSource => {
                Some(Cow::Borrowed("/replay/source"))
            }
            Self::ObservedTemporalMismatch { member } => Some(match *member {
                "clause_node" => Cow::Borrowed("/replay/observed_counterexample/clause_node"),
                "occurrence_key" => Cow::Borrowed("/replay/observed_counterexample/occurrence_key"),
                "fairness" => Cow::Borrowed("/replay/observed_counterexample/fairness"),
                "interval" => Cow::Borrowed("/replay/observed_counterexample/interval"),
                "profile_selections" => Cow::Borrowed("/replay/profile_selections"),
                "obligation_identity" => Cow::Borrowed("/replay/obligation_identity"),
                _ => Cow::Borrowed("/replay/source"),
            }),
            Self::ModelGraphMismatch { member } => Some(match *member {
                "claim_identity" | "clause_node" => {
                    Cow::Borrowed("/replay/source/model_graph/clause_node")
                }
                "occurrence_key" => Cow::Borrowed("/replay/source/model_graph/occurrence_key"),
                "profile_selections" => Cow::Borrowed("/replay/profile_selections"),
                "declared_domains" => Cow::Borrowed("/replay/declared_domains"),
                "undefined_locus" => {
                    Cow::Borrowed("/replay/source/model_graph/undefined/where/locus")
                }
                _ => Cow::Borrowed("/replay/obligation_identity"),
            }),
            Self::ObservedTemporalInput { pointer, .. } => Some(Cow::Borrowed(pointer)),
            Self::RefinementInternals { locus, .. } => Some(Cow::Owned(locus.clone())),
            Self::DependencySelections(_) => Some(Cow::Borrowed("/dependencies")),
            Self::PackageSources(_) => Some(Cow::Borrowed("/package/lock/sources")),
            Self::DependencySources { index, .. } => {
                Some(Cow::Owned(format!("/dependencies/{index}/sources")))
            }
            _ => None,
        }
    }

    /// The input refusal this refusal carries, whether it stopped before
    /// any phase budget or inside the supplied-value conversion or
    /// admission phase: the original kind and parameter either way.
    pub fn input_refusal(&self) -> Option<&InputRefusal> {
        if let Self::Input(refusal) = self {
            return Some(refusal);
        }
        if let Self::Supplied(stop) = self {
            if let SuppliedCause::Input { refusal, .. } = &stop.cause {
                return Some(refusal);
            }
        }
        None
    }
}

/// Why the package reference's `dependencies` refused (ADR-015 D-4 rules 1
/// and 6).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DependencySelectionsCause {
    /// Rule 1: the entry at `index` does not follow the one before it in
    /// strictly ascending UTF-8 byte order of identity; a repeated identity
    /// included.
    #[error("entry {index} ({identity}) is not after the entry before it")]
    Unordered {
        /// The entry's position.
        index: usize,
        /// The entry's identity.
        identity: LibraryName,
    },
    /// Rule 6: the recompiled package selects no library of this identity.
    #[error("the recompiled package selects no {identity}")]
    Unselected {
        /// The entry's identity.
        identity: LibraryName,
    },
}

/// ADR-013 C-13: replay `wire` and settle its verdict. The counterexample a
/// request replays refuted its property, so the proved verdict is
/// `violation`; the replayed verdict is the selected predicate's: `false`
/// is `violation`, `true` is `success`, and an S6a outcome that completed
/// no value is its own category, which never agrees.
///
/// FR-063 seam: adding a `FamilyOutcome` variant with no arm here fails
/// `--cfg seam_probe` with `E0004`; FR-063-AC-7's `#[deny(...)]` closes the
/// wildcard-arm escape the probe alone cannot see.
///
/// # Function selection (TC-166)
///
/// FR-062-AC-10, FR-065-AC-6: the request selects its function by a typed
/// [`QualifiedName`], resolved against the recompiled package's own
/// declarations; a name that resolves to none refuses
/// [`ReplayRefusal::UnknownFunction`]. No entry takes a bare `&str` in its
/// place: this request, which differs from the one after it only in its
/// `replay_target` line, does not compile.
///
/// ```compile_fail,E0308
/// # use qsl_replay::*;
/// # let unit = "language \"ix:native\" edition \"1-draft\";\n\
/// #     profile v = \"quire.value.complete/v1\";\n\
/// #     function small using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";
/// # let small = QualifiedName::new(vec![Identifier::new("small").unwrap()]).unwrap();
/// # let site = call_site(
/// #     SourceIdentity::new("a", "u"),
/// #     "fixture".to_owned(),
/// #     "u",
/// #     unit.as_bytes(),
/// #     [],
/// #     &DependencyInput::default(),
/// #     &small,
/// # )
/// # .unwrap();
/// # let unlimited = ScalarLimits {
/// #     integer_bits: u64::MAX,
/// #     decimal_digits: u64::MAX,
/// #     scale_expansion: u64::MAX,
/// #     text_input_bytes: u64::MAX,
/// #     text_scalars: u64::MAX,
/// #     normalized_scalars: u64::MAX,
/// #     unit_edges: u64::MAX,
/// #     value_occurrences: u64::MAX,
/// #     work_units: u64::MAX,
/// #     result_units: u64::MAX,
/// # };
/// # let key = site.site.declaration.clone();
/// let wire = ReplayRequestWire {
///     replay_target: RequestTargetWire::function("small", key),
/// #   profile_selections: vec![],
/// #   package_name: "fixture".to_owned(),
/// #   sources: vec![(
/// #       "a".to_owned(),
/// #       "u".to_owned(),
/// #   )],
/// #   dependencies: Vec::new(),
/// #   source: ReplaySource::Input(vec![CanonicalAssignment {
/// #       parameter: site.site.parameters[0].1.clone(),
/// #       value: WitnessValue::Integer(7),
/// #   }]),
/// #   obligation_identity: ObligationIdentity::new(NodeKey::new("fixture", vec![quire_exact::PathSegment::Text("0".to_owned())], 0).unwrap(), None, 0),
/// #   backend: "kani-backend-1".to_owned(),
/// #   state_environment: StateEnvironment::new(vec![]),
/// #   accounting_limits: unlimited,
/// #   stage_limits: std::collections::BTreeMap::new(),
/// #   supplied_limits: None,
/// #   object_environment: None,
/// #   declared_domains: Vec::new(),
/// #   refinement_internals: None,
/// #   byte_provision: vec![("a".to_owned(), "u".to_owned(), unit.as_bytes().to_vec())],
/// #   documents: vec![],
///     // ...every other member names the proved package and its input.
/// };
/// assert!(replay(wire, ReplayLimits::default()).is_ok());
/// ```
///
/// The same request selecting `small` by its `QualifiedName` compiles and
/// replays:
///
/// ```
/// # use qsl_replay::*;
/// # let unit = "language \"ix:native\" edition \"1-draft\";\n\
/// #     profile v = \"quire.value.complete/v1\";\n\
/// #     function small using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";
/// # let small = QualifiedName::new(vec![Identifier::new("small").unwrap()]).unwrap();
/// # let site = call_site(
/// #     SourceIdentity::new("a", "u"),
/// #     "fixture".to_owned(),
/// #     "u",
/// #     unit.as_bytes(),
/// #     [],
/// #     &DependencyInput::default(),
/// #     &small,
/// # )
/// # .unwrap();
/// # let unlimited = ScalarLimits {
/// #     integer_bits: u64::MAX,
/// #     decimal_digits: u64::MAX,
/// #     scale_expansion: u64::MAX,
/// #     text_input_bytes: u64::MAX,
/// #     text_scalars: u64::MAX,
/// #     normalized_scalars: u64::MAX,
/// #     unit_edges: u64::MAX,
/// #     value_occurrences: u64::MAX,
/// #     work_units: u64::MAX,
/// #     result_units: u64::MAX,
/// # };
/// # let key = site.site.declaration.clone();
/// let wire = ReplayRequestWire {
///     replay_target: RequestTargetWire::function(small, key),
/// #   profile_selections: vec![],
/// #   package_name: "fixture".to_owned(),
/// #   sources: vec![(
/// #       "a".to_owned(),
/// #       "u".to_owned(),
/// #   )],
/// #   refinement_internals: None,
/// #   dependencies: Vec::new(),
/// #   source: ReplaySource::Input(vec![CanonicalAssignment {
/// #       parameter: site.site.parameters[0].1.clone(),
/// #       value: WitnessValue::Integer(7),
/// #   }]),
/// #   obligation_identity: ObligationIdentity::new(NodeKey::new("fixture", vec![quire_exact::PathSegment::Text("0".to_owned())], 0).unwrap(), None, 0),
/// #   backend: "kani-backend-1".to_owned(),
/// #   state_environment: StateEnvironment::new(vec![]),
/// #   accounting_limits: unlimited,
/// #   stage_limits: std::collections::BTreeMap::new(),
/// #   supplied_limits: None,
/// #   object_environment: None,
/// #   declared_domains: Vec::new(),
/// #   refinement_internals: None,
/// #   byte_provision: vec![("a".to_owned(), "u".to_owned(), unit.as_bytes().to_vec())],
/// #   documents: vec![],
///     // ...every other member names the proved package and its input.
/// };
/// assert!(replay(wire, ReplayLimits::default()).is_ok());
/// ```
fn replay_observed_temporal(
    request: ReplayRequest,
    compiled: Recompiled,
    caller: CallerLimits,
    cancel: &Cancel,
) -> Result<ReplayResult, ReplayRefusal> {
    let ReplaySource::ObservedTemporal(envelope) = request.source() else {
        return Err(ReplayRefusal::WrongObservedTemporalSource);
    };
    let envelope = envelope.as_ref();
    let Some(target) = request.replay_target() else {
        return Err(ReplayRefusal::WrongModelGraphEntry);
    };
    if target.member() != TargetMember::StateClause {
        return Err(ReplayRefusal::WrongObservedTemporalTarget {
            target: Box::new(target.clone()),
        });
    }
    if !matches!(envelope.source(), EnvelopeSource::Witness(_)) {
        return Err(ReplayRefusal::WrongObservedTemporalSource);
    }
    check_envelope_target(&request, envelope.replay_target())?;
    let Some(name) = target.as_state_clause() else {
        return Err(ReplayRefusal::Fault(InternalFault::new(
            "replay.temporal",
            "state-clause-target-has-a-name",
        )));
    };
    if request.obligation_identity() != envelope.obligation_identity() {
        return Err(ReplayRefusal::ObservedTemporalMismatch {
            member: "obligation_identity",
        });
    }
    if request.profile_selections() != envelope.profile_selections() {
        return Err(ReplayRefusal::ObservedTemporalMismatch {
            member: "profile_selections",
        });
    }
    let package_name = compiled.package.clone();
    let package = compiled.checked.package();
    let graph = package.graph();
    let Some(clause_node) = graph
        .temporal_clauses()
        .iter()
        .find(|candidate| candidate.clause().name() == name.to_string())
    else {
        return Err(ReplayRefusal::UnknownClause {
            clause: name.clone(),
            package: package_name,
        });
    };
    let expected_node = clause_node.identity().clone();
    let expected_occurrence =
        OccurrenceKey::new(expected_node.clone(), clause_node.claim().clone());
    if envelope.clause_node() != &expected_node || envelope.occurrence_key() != &expected_occurrence
    {
        return Err(ReplayRefusal::ObservedTemporalMismatch {
            member: if envelope.clause_node() != &expected_node {
                "clause_node"
            } else {
                "occurrence_key"
            },
        });
    }
    let payload = envelope.family_payload();
    let clause = clause_node.clause();
    if payload.fairness.as_slice() != clause.fairness() {
        return Err(ReplayRefusal::ModelTrace(Box::new(
            crate::model_check::replay::ModelTraceRefusal::FairnessSet {
                payload: payload.fairness.clone(),
                recompiled: clause.fairness().to_vec(),
            },
        )));
    }
    let packet_position = envelope
        .trace_position()
        .and_then(|position| TemporalPosition::decode(position.as_str()).ok());
    if packet_position != Some(payload.trace_position) {
        return Err(ReplayRefusal::ObservedTemporalInput {
            pointer: "/replay/observed_counterexample/trace_position",
            reason: "the envelope and observed payload trace positions differ",
        });
    }
    if matches!(
        payload.kind,
        crate::model_check::counterexample::ObservedTemporalKind::Formula
    ) {
        let checked_interval = clause
            .operators()
            .iter()
            .find_map(|operator| operator.interval.as_ref());
        let interval_matches = match (checked_interval, payload.interval) {
            (None, None) => true,
            (Some(CheckedInterval::Steps(interval)), Some(packet)) => *interval == packet,
            _ => false,
        };
        if !interval_matches {
            return Err(ReplayRefusal::ObservedTemporalMismatch { member: "interval" });
        }
    }

    let documents: Vec<DocumentRef> = payload
        .prefix
        .iter()
        .map(|step| step.document.clone())
        .collect();
    let len = u64::try_from(documents.len()).unwrap_or(u64::MAX);
    if documents.is_empty() {
        return Err(ReplayRefusal::ObservedTemporalInput {
            pointer: "/replay/observed_counterexample/prefix",
            reason: "an observed counterexample must contain at least one document",
        });
    }
    if payload.trace_position.index() >= len {
        return Err(ReplayRefusal::ObservedTemporalInput {
            pointer: "/replay/observed_counterexample/trace_position",
            reason: "trace_position lies outside the represented observed trace",
        });
    }
    let trace = match payload.loop_start {
        Some(prefix) if prefix < len => TemporalTrace::Lasso {
            prefix,
            loop_len: len - prefix,
        },
        Some(_) => {
            return Err(ReplayRefusal::ObservedTemporalInput {
                pointer: "/replay/observed_counterexample/loop",
                reason: "an observed lasso must have a non-empty loop",
            })
        }
        None => TemporalTrace::Finite { len },
    };
    if matches!(trace, TemporalTrace::Lasso { .. })
        && clause.profile() != qsl_semantics::check::TemporalProfile::InfiniteTrace
    {
        return Err(ReplayRefusal::ObservedTemporalRefusal(
            qsl_eval::temporal::TemporalRefusal::LassoUnderBoundedProfile {
                profile: clause.profile(),
            },
        ));
    }

    let snapshots = request.byte_provision().snapshots();
    let mut environments = Vec::with_capacity(documents.len());
    for document in &documents {
        let (environment, _) = admit_trace_snapshot(
            graph.model_selections(),
            graph.scope().types(),
            &compiled.packages,
            compiled.model_limits,
            &snapshots,
            document,
            caller.observation,
            &clause_node.named_populations(),
        )
        .map_err(ReplayRefusal::Admission)?;
        environments.push(environment);
    }
    let Some(over) = request.over() else {
        return Err(ReplayRefusal::Input(InputRefusal::ObservationsMismatch));
    };
    let (_, over_type) = clause.over();
    let ValueType::Reference(type_identity) = over_type else {
        return Err(ReplayRefusal::Fault(InternalFault::new(
            "replay.temporal",
            "checked-temporal-over-is-a-reference",
        )));
    };
    let universe = population_universe_for(
        graph.model_selections(),
        &compiled.packages,
        compiled.model_limits,
        type_identity.clone(),
    )
    .map_err(ReplayRefusal::Admission)?;
    let reference = environments
        .first()
        .and_then(|environment| environment.objects().find(universe, &over.key))
        .ok_or(ReplayRefusal::Input(InputRefusal::ObservationsMismatch))?;
    if let Some(constraint) = clause.fairness().first() {
        return Err(ReplayRefusal::MissingFairnessPremise {
            constraint: constraint.clone(),
        });
    }
    let mut meter = Meter::new(request.accounting_limits()).with_cancel(cancel.clone());
    let evaluation = evaluate_observed(
        package,
        clause_node,
        &environments,
        trace,
        reference,
        &mut meter,
    );
    if evaluation.stopped.is_some() {
        return Err(ReplayRefusal::Fault(InternalFault::new(
            "replay.temporal",
            "observed-holds-evaluation-stopped",
        )));
    }
    let outcome = evaluation
        .result
        .map_err(ReplayRefusal::ObservedTemporalRefusal)?;
    if let Some(cause) = cancel.tripped() {
        return Err(ReplayRefusal::Cancelled(cause));
    }
    Ok(ReplayResult::ObservedTemporal(
        ObservedTemporalResult::settle(
            payload.kind.clone(),
            payload.trace_position,
            outcome,
            // FR-331-AC-9: an observed packet names an undefined atom only.
            evaluation.undefined.and_then(|first| match first.locus {
                qsl_eval::temporal::ObservedLocus::Atom(atom) => Some(atom),
                qsl_eval::temporal::ObservedLocus::Activation
                | qsl_eval::temporal::ObservedLocus::Capture(_) => None,
            }),
            consumed(&meter),
        ),
    ))
}

/// FR-098: decode, recompile and replay a typed request. An observed
/// temporal counterexample source dispatches to FR-331 after the common
/// request and package checks.
pub fn replay(
    wire: ReplayRequestWire,
    replay_limits: ReplayLimits,
) -> Result<ReplayResult, ReplayRefusal> {
    replay_with_temporal_selections(wire, &[], replay_limits)
}

/// [`replay`] with explicit clock configurations for checked temporal clauses.
pub fn replay_with_temporal_selections(
    wire: ReplayRequestWire,
    temporal_selections: &[qsl_package::TemporalClockSelection],
    replay_limits: ReplayLimits,
) -> Result<ReplayResult, ReplayRefusal> {
    replay_with_cancel_and_temporal_selections(
        wire,
        temporal_selections,
        replay_limits,
        &Cancel::new(),
    )
}

/// [`replay`] under the caller's original [`Cancel`] handle (ADR-029,
/// FR-277). The handle reaches every supplied-value phase and the call: a
/// cancellation observed in conversion or admission is
/// [`ReplayRefusal::Supplied`] with the original cause and its phase
/// context, one observed during the call [`ReplayRefusal::Cancelled`], and
/// neither is ever read as a limit.
///
/// The request's `limits.supplied` ceilings bound the conversion helper work
/// H and the final admission helper work A as two cumulative budgets across
/// every argument. A pre-call limit (those two, the per-argument converted
/// nodes against `work_units`, or the occurrence count against
/// `value_occurrences`) makes no call and settles `inconclusive`/`NoValue`
/// carrying [`ReplayResult::pre_call_limit`]: zero replay charges, no
/// `FunctionCall` charge point.
pub fn replay_with_cancel(
    wire: ReplayRequestWire,
    replay_limits: ReplayLimits,
    cancel: &Cancel,
) -> Result<ReplayResult, ReplayRefusal> {
    replay_with_cancel_and_temporal_selections(wire, &[], replay_limits, cancel)
}

/// [`replay_with_cancel`] with explicit clock configurations for checked
/// temporal clauses.
#[deny(clippy::wildcard_enum_match_arm)]
pub fn replay_with_cancel_and_temporal_selections(
    wire: ReplayRequestWire,
    temporal_selections: &[qsl_package::TemporalClockSelection],
    replay_limits: ReplayLimits,
    cancel: &Cancel,
) -> Result<ReplayResult, ReplayRefusal> {
    let request = ReplayRequest::decode(wire, replay_limits)?;
    if matches!(request.source(), ReplaySource::ModelGraph(_)) {
        return Err(ReplayRefusal::WrongModelGraphEntry);
    }
    let function = match request.source() {
        ReplaySource::ObservedTemporal(_) => None,
        ReplaySource::ModelGraph(_) => None,
        ReplaySource::Witness(_) | ReplaySource::Input(_) => {
            Some(declared_function_target(&request, ReplayEntry::Replay)?)
        }
    };
    let caller = request_limits(
        request.stage_limits(),
        request.accounting_limits(),
        replay_limits,
    )?;
    let compiled = recompile_with_temporal_selections(&request, &caller, temporal_selections)?;
    if let ReplaySource::ObservedTemporal(_) = request.source() {
        return replay_observed_temporal(request, compiled, caller, cancel);
    }
    let (function, owner) = function.ok_or_else(|| {
        ReplayRefusal::Fault(InternalFault::new(
            "replay",
            "non-temporal-source-has-function-target",
        ))
    })?;
    // FR-098, FR-121: the function is looked up in the owner's package when
    // the target names one, and in the unit's own only otherwise.
    let declaring = compiled.declaring(owner)?;
    let package = declaring.package;
    let replay_target = request
        .replay_target()
        .ok_or(ReplayRefusal::WrongModelGraphEntry)?;
    let claim = function_claim(declaring, function, replay_target)?;
    let call = select(declaring, function, claim.kind())?;
    let joined = arguments(
        &call,
        request.source(),
        request.obligation_identity(),
        claim.unread_roots(),
    )?;
    // FR-098 step 1, after the join and before any conversion.
    if request.object_environment().is_some()
        && !object_environment::holds_reference(package.graph().scope().types(), &call.types)
    {
        return Err(ReplayRefusal::UnusedObjectEnvironment);
    }
    let limits = request.accounting_limits();
    let mut run = SuppliedRun::new(request.supplied_limits(), cancel.clone())
        .map_err(ReplayRefusal::Supplied)?;
    let converted = match argument::convert_arguments(
        package,
        &call.types,
        &joined,
        &limits,
        &mut run,
        &compiled.reference_model(package),
    ) {
        Ok(converted) => converted,
        Err(Stopped::Refusal(refusal)) => return Err(*refusal),
        Err(Stopped::Supplied(failure)) => return settle_supplied(&request, failure),
    };
    // FR-098 steps 3 and 4: a request without an environment runs over
    // unresolved references.
    let environment = if request.object_environment().is_some() {
        object_environment::admit(
            &request,
            &compiled,
            declaring,
            call.identity,
            &converted.references,
            &caller,
        )?
    } else {
        ObjectEnvironment::default().with_unresolved_references()
    };
    let mut meter = Meter::new(limits).with_cancel(cancel.clone());
    let evaluation = match package.call_supplied(
        &call.name,
        converted.values,
        &environment,
        &mut meter,
        &mut run,
    ) {
        Ok(evaluation) => evaluation,
        // A supplied-phase stop is the run's full record, whatever arm the
        // checked call gave it; anything else is the call's own failure.
        Err(failure) => return after_call_failure(&request, &run, failure),
    };
    if let Some(cause) = cancel.tripped() {
        return Err(ReplayRefusal::Cancelled(cause));
    }
    let mut limit = None;
    // FR-098 "Application claim: the observation": the call stopped
    // undefined at the application step of the claim's own site. A stop
    // inside an operand, in a nested function or at another occurrence of
    // the node is a different location and no observation. `Location` has
    // no frame, so in a recursive function a stop at the site may belong to
    // a nested call of the function itself: it then settles `NoValue`, which
    // loses a reproduction and never invents one.
    let failed_at_site = match &claim {
        FunctionClaim::Application(application) => {
            !application.recursive && evaluation.location.as_ref() == Some(&application.site)
        }
        FunctionClaim::Predicate => false,
    };
    let (replayed, value) = match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => match (&claim, value) {
            (FunctionClaim::Predicate, Value::Boolean(holds)) => (
                if holds {
                    Category::Success
                } else {
                    Category::Violation
                },
                Some(EvaluatedValue::Boolean(holds)),
            ),
            // A completed call with no failing observation holds the claim,
            // whether or not it evaluated the occurrence on any path. FR-098's
            // "completes outside the record's result bound" arm is not
            // implemented: no checked package reaches it, and omitting it
            // fails safe (a missed reproduction, never an invented one).
            (FunctionClaim::Application(_), _) => {
                (Category::Success, Some(EvaluatedValue::Boolean(true)))
            }
            // `select` admitted only a function declared `Boolean`.
            (FunctionClaim::Predicate, _) => {
                return Err(ReplayRefusal::Fault(InternalFault::new(
                    "replay",
                    "boolean-function-completes-a-boolean",
                )));
            }
        },
        FamilyOutcome::Evaluated(Outcome::Undefined(_)) if failed_at_site => {
            (Category::Violation, Some(EvaluatedValue::Boolean(false)))
        }
        FamilyOutcome::Evaluated(Outcome::Refused(_)) => (Category::Refusal, None),
        FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
            limit = Some(incomplete);
            (Category::Incomplete, None)
        }
        // O-16's `undefined` row is not a proof category, and a
        // family-owned result is not a kernel verdict: neither agrees.
        FamilyOutcome::Evaluated(Outcome::Undefined(_)) | FamilyOutcome::FamilyEvaluated(_) => {
            (Category::Inconclusive, None)
        }
        // FR-063: no arm for the probe variant under `--cfg seam_probe`
        // alone -- this match is the replay facade's own seam over
        // `FamilyOutcome` (`E0004` in `xtask seam-probe`'s build of this
        // crate): a family whose S6a result is new widens the facade here
        // (ADR-013 TK-01). The arm below exists only in the probe's build of
        // the root crate (`--cfg seam_probe_replay_downstream`), which
        // depends on this crate. Do not add a catch-all to make it compile.
        #[cfg(seam_probe_replay_downstream)]
        FamilyOutcome::__SeamProbe => {
            unreachable!("never constructed outside the probe build")
        }
    };
    let regions = evaluation
        .location
        .as_ref()
        .and_then(|location| package.graph().region(location))
        .into_iter()
        .collect();
    let settled = settle(&request, replayed, value, regions, consumed(&meter));
    Ok(match limit {
        Some(incomplete) => settled.with_limit(incomplete),
        None => settled,
    })
}

/// The checked call's failure, read as the run's full supplied-phase record
/// when the run stopped (FR-277), and as the call's own failure otherwise.
fn after_call_failure(
    request: &ReplayRequest,
    run: &SuppliedRun,
    failure: CallFailure,
) -> Result<ReplayResult, ReplayRefusal> {
    match run.failure() {
        Some(recorded) => settle_supplied(request, recorded.clone()),
        None => Err(call_failure_to_replay_refusal(failure)),
    }
}

/// A supplied-value phase that stopped before the call: a helper or counter
/// ceiling settles the replay `inconclusive`/`NoValue` with the phase-marked
/// pre-call limit; every other stop is its own typed refusal (FR-277).
fn settle_supplied(
    request: &ReplayRequest,
    failure: SuppliedFailure,
) -> Result<ReplayResult, ReplayRefusal> {
    match failure {
        SuppliedFailure::Stop(stop) if stop.limit().is_some() => Ok(settle(
            request,
            Category::Incomplete,
            None,
            Vec::new(),
            charges(std::iter::empty()),
        )
        .with_pre_call(stop)),
        SuppliedFailure::Stop(stop) => Err(ReplayRefusal::Supplied(stop)),
        SuppliedFailure::Unsupported(stop) => Err(ReplayRefusal::UnsupportedHelper(stop)),
        SuppliedFailure::Fault(fault) => Err(ReplayRefusal::Fault(fault)),
    }
}

/// The result on the arm of the request's replay source.
fn settle(
    request: &ReplayRequest,
    replayed: Category,
    value: Option<EvaluatedValue>,
    regions: Vec<qsl_foundation::source::provenance::SourceRegion>,
    charges: ScalarLimits,
) -> ReplayResult {
    let proved = Verdict::from_category(Category::Violation);
    // ADR-031 SW-7: a function call's result has no decisive occurrence, so
    // it carries no record.
    match request.source() {
        ReplaySource::Witness(_) => ReplayResult::Witness(WitnessArmResult::settle(
            proved,
            Verdict::from_category(replayed),
            replayed,
            value,
            WitnessCheck::Agrees(None),
            regions,
            charges,
        )),
        ReplaySource::Input(_) => ReplayResult::Input(InputArmResult::settle(
            proved,
            Verdict::from_category(replayed),
            replayed,
            value,
            &WitnessCheck::Agrees(None),
            regions,
            charges,
        )),
        ReplaySource::ObservedTemporal(_) => {
            unreachable!("observed temporal requests dispatch before function replay settlement")
        }
        ReplaySource::ModelGraph(_) => {
            unreachable!("state-graph requests dispatch before function replay settlement")
        }
    }
}

/// FR-096-AC-15, SR-746 FND-002: a broken S6a invariant -- including a
/// kernel `Refusal::CheckedInvariant`, which the seam itself now turns into
/// `CallFailure::Fault` before `package.call` ever returns -- settles
/// `Err(ReplayRefusal::Fault(_))`, never `Ok(ReplayResult::..)` with
/// `Category::Refusal`. `CallFailure::Input` passes its own refusal
/// through unchanged.
fn call_failure_to_replay_refusal(failure: CallFailure) -> ReplayRefusal {
    match failure {
        CallFailure::Input(refusal) => ReplayRefusal::Input(refusal),
        // FR-049-AC-2: the call's entry refused its object environment, a
        // caller input refusal. This path selects no checked protocol slot,
        // so every other admission failure is a broken invariant.
        CallFailure::Admission(failure) => {
            match qsl_eval::value::object_environment_refusal(&failure) {
                Some(record) => ReplayRefusal::ObjectEnvironment(
                    object_environment::ObjectEnvironmentFailure::Admission(
                        qsl_semantics::model::observation::AdmissionFailure::Refused(Box::new(
                            record.clone(),
                        )),
                    ),
                ),
                None => ReplayRefusal::Fault(InternalFault::new(
                    "call",
                    "function-replay-has-no-protocol-slot-admission",
                )),
            }
        }
        CallFailure::Fault(fault) => ReplayRefusal::Fault(fault),
        // The request replay path constructs no registered host body. Body
        // failures are surfaced unchanged by the checked application facade.
        CallFailure::Body(_) => unreachable!("request replay registers no host bodies"),
        CallFailure::Storage { parameter, failure } => ReplayRefusal::Storage {
            position: parameter,
            failure,
        },
        CallFailure::Supplied(stop) => ReplayRefusal::Supplied(stop),
        CallFailure::UnsupportedHelper(stop) => ReplayRefusal::UnsupportedHelper(stop),
        CallFailure::Cancelled(cause) => ReplayRefusal::Cancelled(cause),
    }
}

/// The limits a request replays under: every stage at the request's
/// `stage_limits` entry for its setting, or at its published default, and
/// `replay.input_bytes` from the library entry's own limits. An `s1.input_bytes`
/// entry above `replay.input_bytes` refuses before the recompile.
pub(crate) fn request_limits(
    stage_limits: &StageLimits,
    accounting: ScalarLimits,
    replay: ReplayLimits,
) -> Result<CallerLimits, ReplayRefusal> {
    let limits = CallerLimits::for_request(stage_limits, accounting, replay);
    if let Some(requested) = stage_limits.bound(Setting::S1InputBytes) {
        if requested > replay.input_bytes {
            return Err(ReplayRefusal::LimitAboveReader(LimitAboveReader {
                requested,
                reader: replay.input_bytes,
            }));
        }
    }
    Ok(limits)
}

/// The recompile of a request's unit: its checked package and the bytes it
/// emits.
pub(crate) struct Recompiled {
    /// The recompiled in-process package, with the sources it came from.
    pub(crate) checked: spine::CheckedUnit,
    /// The recompiled package's v2 bytes.
    pub(crate) emitted: spine::EmittedUnit,
    /// The package the unit declares, by its source identity.
    pub(crate) package: LibraryName,
    /// The domain packages the recompile read, by package identity.
    pub(crate) packages: qsl_semantics::model::intake::PackageInput,
    /// The limits the models were normalized under.
    pub(crate) model_limits: qsl_semantics::model::accounting::ModelNormalizationLimits,
}

impl Recompiled {
    /// Where a reference argument's universe is read from, over `package`.
    pub(crate) fn reference_model<'a>(
        &'a self,
        package: &'a CheckedPackage,
    ) -> argument::ReferenceModel<'a> {
        argument::ReferenceModel::new(package, &self.packages, self.model_limits)
    }

    /// The unit's own package, the one a one-segment function name resolves in.
    pub(crate) fn own(&self) -> Declaring<'_> {
        Declaring {
            package: self.checked.package(),
            library: &self.package,
            imported: false,
        }
    }

    /// The package that declares a `function` target's function: the unit's
    /// own, or the checked package of the direct import `owner` names (FR-121:
    /// `CheckedPackage::dependencies` holds only direct imports, built from the
    /// request's dependency lock).
    pub(crate) fn declaring<'a>(
        &'a self,
        owner: Option<&'a LibraryName>,
    ) -> Result<Declaring<'a>, ReplayRefusal> {
        let Some(owner) = owner else {
            return Ok(self.own());
        };
        let package = self
            .checked
            .package()
            .dependencies()
            .get(owner)
            .ok_or_else(|| ReplayRefusal::UnknownOwner {
                owner: owner.clone(),
                package: self.package.clone(),
            })?;
        Ok(Declaring {
            package,
            library: owner,
            imported: true,
        })
    }
}

/// The checked package a replayed function is declared in, with the name its
/// refusals pair the function with.
#[derive(Clone, Copy)]
pub(crate) struct Declaring<'a> {
    pub(crate) package: &'a CheckedPackage,
    pub(crate) library: &'a LibraryName,
    /// The package is a direct import's, not the unit's own.
    pub(crate) imported: bool,
}

/// The one source unit a spine run recompiles: its identity, the library name
/// its nodes are keyed under, its display path and its bytes.
pub(crate) struct SpineUnit<'a> {
    pub(crate) identity: &'a SourceIdentity,
    pub(crate) package: &'a LibraryName,
    pub(crate) path: &'a str,
    pub(crate) bytes: &'a [u8],
}

#[allow(
    clippy::too_many_arguments,
    reason = "spine inputs plus selected temporal clocks"
)]
pub(crate) fn run_spine_with_temporal_selections_and_execution_points(
    unit: SpineUnit<'_>,
    packages: &qsl_semantics::model::intake::PackageInput,
    dependencies: &DependencyInput,
    execution_points: &spine::StateClauseExecutionPoints,
    temporal_selections: &[qsl_package::TemporalClockSelection],
    integer_division: Option<quire_checked::integer_division::AdmittedIntegerDivision>,
    limits: SpineLimits,
) -> Result<Recompiled, ReplayRefusal> {
    let SpineUnit {
        identity: unit,
        package,
        path,
        bytes,
    } = unit;
    #[cfg(test)]
    SPINE_RUNS.with(|runs| runs.set(runs.get() + 1));
    let cancel = Cancel::new();
    let refusal = |failure| match failure {
        qsl_foundation::diagnostic::StageFailure::RefusedWithPayload(refusal, protocol_results) => {
            ReplayRefusal::RecompileWithProtocolResults {
                refusal: *refusal,
                protocol_results,
            }
        }
        qsl_foundation::diagnostic::StageFailure::LimitWithPayload(limit, protocol_results) => {
            ReplayRefusal::RecompileWithProtocolResults {
                refusal: Box::new(CompileRefusal::Limit(limit)),
                protocol_results,
            }
        }
        failure => match spine::refusal_or_fault(failure) {
            Ok(refusal) => ReplayRefusal::Recompile(refusal),
            Err(fault) => ReplayRefusal::Fault(fault),
        },
    };
    let parsed = spine::parse_for_continuation(
        &ParseRequest {
            source: unit,
            path,
            bytes,
        },
        limits.source,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let models = spine::select(&parsed, packages, limits.model, &cancel)
        .map_err(refusal)?
        .into_value();
    let checked = spine::check_for_continuation(
        &parsed,
        spine::ContinuationInputs {
            package_name: package.as_str().to_owned(),
            execution_points,
            temporal_selections,
            integer_division,
        },
        &models,
        dependencies,
        limits,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let emitted = spine::package_with_temporal_selections(
        &checked,
        temporal_selections,
        limits.package,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    Ok(Recompiled {
        checked,
        emitted,
        package: package.clone(),
        packages: packages.clone(),
        model_limits: limits.model,
    })
}

#[cfg(test)]
thread_local! {
    static SPINE_RUNS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn spine_run_count() -> u64 {
    SPINE_RUNS.with(std::cell::Cell::get)
}

/// Recompile the request's one source unit from the byte provision against
/// the dependency input its `dependencies` entries build, with every domain
/// package the provision carries under its package identity as I1's
/// package input, applying ADR-015 D-4's seven rules in order: each rule
/// over every entry, in entry order, before the next.
pub(crate) fn recompile(
    request: &ReplayRequest,
    limits: &CallerLimits,
) -> Result<Recompiled, ReplayRefusal> {
    recompile_with_temporal_selections(request, limits, &[])
}

pub(crate) fn recompile_with_temporal_selections(
    request: &ReplayRequest,
    limits: &CallerLimits,
    temporal_selections: &[qsl_package::TemporalClockSelection],
) -> Result<Recompiled, ReplayRefusal> {
    // Rule 1: strictly ascending identities, before anything is built.
    for (index, pair) in request.dependencies().windows(2).enumerate() {
        if let [before, entry] = pair {
            if entry.identity() <= before.identity() {
                return Err(ReplayRefusal::DependencySelections(
                    DependencySelectionsCause::Unordered {
                        index: index + 1,
                        identity: entry.identity().clone(),
                    },
                ));
            }
        }
    }
    // Rule 2: the proved package's and each dependency's sources name
    // non-empty sets of unique source units, independent of request order.
    let definition_documents = DefinitionDocumentOwners::from_request(request);
    let sources = package_sources(request, &definition_documents)?;
    let library_sources = request
        .dependencies()
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            canonical_dependency_sources(index, entry.sources(), &definition_documents)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let provided = |reference: &DefinitionSourceRef| {
        request.byte_provision().get(reference).ok_or_else(|| {
            // `ReplayRequest::decode` refuses an incomplete provision.
            ReplayRefusal::Fault(InternalFault::new(
                "replay",
                "decoded-request-byte-provision-complete",
            ))
        })
    };
    // Rule 3: the dependency input.
    let libraries = request
        .dependencies()
        .iter()
        .zip(library_sources)
        .map(|(entry, references)| {
            references
                .into_iter()
                .map(|reference| {
                    Ok(SuppliedLibrary {
                        identity: entry.identity().as_str().to_owned(),
                        source: labels(reference),
                        path: reference.identity().to_owned(),
                        bytes: provided(reference)?.to_vec(),
                    })
                })
                .collect::<Result<Vec<_>, ReplayRefusal>>()
        })
        .collect::<Result<Vec<_>, ReplayRefusal>>()?;
    let dependencies = DependencyInput::new(libraries.iter().map(|units| {
        units
            .first()
            .expect("dependency sources are non-empty")
            .clone()
    }))
    .map_err(ReplayRefusal::DependencyInput)?;
    let dependencies = request.dependencies().iter().zip(libraries).try_fold(
        dependencies,
        |dependencies, (entry, units)| {
            dependencies
                .with_library_units(entry.identity().as_str(), units)
                .map_err(ReplayRefusal::DependencyInput)
        },
    )?;
    // Each dependency's own `refinement_internals` value goes to the
    // recompile unchanged (QSpec FR-323-AC-17); an absent member is the
    // empty list.
    let dependencies = dependencies
        .with_refinement_internals(request.dependencies().iter().filter_map(|entry| {
            entry
                .refinement_internals()
                .map(|value| (entry.identity().as_str().to_owned(), value.clone()))
        }))
        .map_err(ReplayRefusal::DependencyInput)?;
    for source in &sources {
        dependencies
            .check_unit_owner(&labels(source))
            .map_err(ReplayRefusal::DependencyInput)?;
    }
    let packages = domain_packages(request)?;
    // Rule 4: the recompile.
    let package_name =
        LibraryName::new(request.package_name()).map_err(ReplayRefusal::InvalidPackageName)?;
    let units = sources
        .iter()
        .map(|source| {
            Ok::<_, ReplayRefusal>((
                labels(source),
                source.identity().to_owned(),
                provided(source)?,
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let compiled = run_spine_units(
        &units,
        &package_name,
        &packages,
        &dependencies,
        temporal_selections,
        limits.spine,
        request.refinement_internals(),
    )
    .map_err(|refusal| locate_refinement_internals(refusal, request.dependencies()))?;
    // Rule 6: every entry names a selection of the recompiled closure.
    let selections = compiled.checked.package().dependency_selections();
    for entry in request.dependencies() {
        if !selections.contains_key(entry.identity()) {
            return Err(ReplayRefusal::DependencySelections(
                DependencySelectionsCause::Unselected {
                    identity: entry.identity().clone(),
                },
            ));
        }
    }
    Ok(compiled)
}

/// I1's package input: every domain package document the provision carries,
/// keyed by the identity it declares (QC-1).
fn domain_packages(
    request: &ReplayRequest,
) -> Result<qsl_semantics::model::intake::PackageInput, ReplayRefusal> {
    package_input(request.byte_provision().package_documents())
        .map_err(|refusal| ReplayRefusal::DomainPackages(Box::new(refusal)))
}

/// A dependency entry's source references in canonical owner order, else
/// the entry's [`ReplayRefusal::DependencySources`].
fn canonical_dependency_sources<'a>(
    index: usize,
    references: &'a [DefinitionSourceRef],
    definition_documents: &DefinitionDocumentOwners,
) -> Result<Vec<&'a DefinitionSourceRef>, ReplayRefusal> {
    canonical_sources(references, definition_documents, |reason| {
        ReplayRefusal::DependencySources { index, reason }
    })
}

/// Validate a non-empty list of unique, non-definition-document source
/// references and return it in canonical authority and identity order.
fn canonical_sources<'a>(
    references: &'a [DefinitionSourceRef],
    definition_documents: &DefinitionDocumentOwners,
    refuse: impl Fn(&'static str) -> ReplayRefusal,
) -> Result<Vec<&'a DefinitionSourceRef>, ReplayRefusal> {
    if references.is_empty() {
        return Err(refuse("the list is empty"));
    }
    for (second, source) in references.iter().enumerate() {
        if references[..second].iter().any(|earlier| {
            earlier.authority() == source.authority() && earlier.identity() == source.identity()
        }) {
            return Err(refuse("an owner is repeated"));
        }
        if definition_documents.contains(source) {
            return Err(refuse("a source reference names a definition document"));
        }
    }
    let mut canonical = references.iter().collect::<Vec<_>>();
    canonical.sort_by(|left, right| {
        left.authority()
            .as_bytes()
            .cmp(right.authority().as_bytes())
            .then_with(|| left.identity().as_bytes().cmp(right.identity().as_bytes()))
    });
    Ok(canonical)
}
struct DefinitionDocumentOwners {
    source_owners: BTreeSet<(String, String)>,
    package_identities: BTreeSet<String>,
}
impl DefinitionDocumentOwners {
    fn from_request(request: &ReplayRequest) -> Self {
        let source_owners = request
            .byte_provision()
            .documents()
            .filter_map(qsl_semantics::model::observation::document_reference)
            .map(|document| (document.authority, document.identity))
            .collect();
        let package_identities = qsl_semantics::model::intake::package_input(
            request.byte_provision().package_documents(),
        )
        .map(|packages| packages.keys().cloned().collect())
        .unwrap_or_default();
        Self {
            source_owners,
            package_identities,
        }
    }

    fn contains(&self, source: &DefinitionSourceRef) -> bool {
        self.source_owners
            .contains(&(source.authority().to_owned(), source.identity().to_owned()))
            || self.package_identities.contains(source.identity())
    }
}
/// The proved package's source references in canonical owner order.
fn package_sources<'a>(
    request: &'a ReplayRequest,
    definition_documents: &DefinitionDocumentOwners,
) -> Result<Vec<&'a DefinitionSourceRef>, ReplayRefusal> {
    canonical_sources(
        request.sources(),
        definition_documents,
        ReplayRefusal::PackageSources,
    )
}

fn run_spine_units(
    units: &[(SourceIdentity, String, &[u8])],
    package: &LibraryName,
    packages: &qsl_semantics::model::intake::PackageInput,
    dependencies: &DependencyInput,
    temporal_selections: &[qsl_package::TemporalClockSelection],
    limits: SpineLimits,
    refinement_internals: Option<&serde_json::Value>,
) -> Result<Recompiled, ReplayRefusal> {
    #[cfg(test)]
    SPINE_RUNS.with(|runs| runs.set(runs.get() + 1));
    let cancel = Cancel::new();
    let refusal = |failure| match failure {
        qsl_foundation::diagnostic::StageFailure::RefusedWithPayload(refusal, protocol_results) => {
            ReplayRefusal::RecompileWithProtocolResults {
                refusal: *refusal,
                protocol_results,
            }
        }
        qsl_foundation::diagnostic::StageFailure::LimitWithPayload(limit, protocol_results) => {
            ReplayRefusal::RecompileWithProtocolResults {
                refusal: Box::new(CompileRefusal::Limit(limit)),
                protocol_results,
            }
        }
        failure => match spine::refusal_or_fault(failure) {
            Ok(refusal) => ReplayRefusal::Recompile(refusal),
            Err(fault) => ReplayRefusal::Fault(fault),
        },
    };
    let requests = units
        .iter()
        .map(|(identity, path, bytes)| ParseRequest {
            source: identity,
            path,
            bytes,
        })
        .collect::<Vec<_>>();
    spine::preflight_unit_list(&requests, dependencies).map_err(ReplayRefusal::Recompile)?;
    let parsed = requests
        .iter()
        .map(|request| {
            spine::parse_for_continuation(request, limits.source, &cancel)
                .map(|parsed| parsed.into_value())
                .map_err(&refusal)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let admitted =
        spine::admit_parsed_unit_list(parsed, dependencies).map_err(ReplayRefusal::Recompile)?;
    let models = spine::select_units(&admitted, packages, limits.model, limits.source, &cancel)
        .map_err(&refusal)?
        .into_value();
    let checked = spine::check_units_with_options(
        &admitted,
        package.as_str().to_owned(),
        &models,
        dependencies,
        spine::UnitCheckOptions {
            temporal_selections,
            continue_protocol_refusals: true,
            refinement_internals,
        },
        limits,
        &cancel,
    )
    .map_err(&refusal)?
    .into_value();
    let checked = checked.into_iter().next().ok_or_else(|| {
        ReplayRefusal::Fault(InternalFault::new(
            "replay",
            "non-empty-package-has-checked-unit",
        ))
    })?;
    let emitted = spine::package_with_temporal_selections(
        &checked,
        temporal_selections,
        limits.package,
        &cancel,
    )
    .map_err(&refusal)?
    .into_value();
    Ok(Recompiled {
        checked,
        emitted,
        package: package.clone(),
        packages: packages.clone(),
        model_limits: limits.model,
    })
}

/// Wrap a recompile refusal that is a defect of a `refinement_internals`
/// value with its request locus (QSpec FR-323-AC-17): the proved package's
/// value at `/package/lock/refinement_internals`, the `n`th dependency
/// entry's at `/dependencies/<n>/refinement_internals`, each followed by the
/// pointer into the value the check reports (FR-147).
fn locate_refinement_internals(
    refusal: ReplayRefusal,
    entries: &[crate::request::DependencyEntry],
) -> ReplayRefusal {
    let locus = match &refusal {
        ReplayRefusal::Recompile(compile) => internals_locus(compile, entries),
        ReplayRefusal::RecompileWithProtocolResults { refusal, .. } => {
            internals_locus(refusal, entries)
        }
        _ => None,
    };
    match locus {
        Some(locus) => ReplayRefusal::RefinementInternals {
            locus,
            source: Box::new(refusal),
        },
        None => refusal,
    }
}

/// The request locus of a `refinement_internals` defect the recompile
/// refused with, `None` for any other refusal.
fn internals_locus(
    compile: &CompileRefusal,
    entries: &[crate::request::DependencyEntry],
) -> Option<String> {
    let defect_pointer = |refusal: &CompileRefusal| match refusal {
        CompileRefusal::Check { refusals, .. } => match &refusals.first()?.cause {
            qsl_semantics::check::CheckCause::Refinement(cause) => match &cause.cause {
                qsl_semantics::check::RefinementCause::InternalList(defect) => {
                    Some(defect.pointer.clone())
                }
                _ => None,
            },
            _ => None,
        },
        _ => None,
    };
    match compile {
        CompileRefusal::Dependency { path, refusal } => {
            let pointer = defect_pointer(refusal)?;
            let owner = path.last()?;
            let index = entries.iter().position(|entry| entry.identity() == owner)?;
            Some(format!(
                "/dependencies/{index}/refinement_internals{pointer}"
            ))
        }
        other => {
            let pointer = defect_pointer(other)?;
            Some(format!("/package/lock/refinement_internals{pointer}"))
        }
    }
}

/// A source reference's two FR-001 labels.
fn labels(reference: &DefinitionSourceRef) -> SourceIdentity {
    SourceIdentity::new(reference.authority(), reference.identity())
}

/// What a replay claims of the function it selects.
#[derive(Clone, Copy)]
enum Claim {
    /// A Boolean predicate is false at the bindings ([`replay`]).
    Predicate,
    /// An application of the function's body is undefined at the bindings
    /// ([`replay`]); the function may declare any result type.
    Application,
    /// The generated code's outcome equals QSL's `f(b)`
    /// ([`replay_value_parity`]).
    ValueParity,
}

/// The claim a `function` request's `occurrence_key` selects (FR-098 "The
/// claim a function request decides").
enum FunctionClaim {
    /// The function's `declaration` occurrence: the function returns `true`.
    Predicate,
    /// A `value-validity` record the function owns: the application at the
    /// record's occurrence is defined and within the record's result bound.
    Application(ApplicationClaim),
}

/// An application claim: where its application is evaluated and which of
/// the function's parameters its extent reads.
struct ApplicationClaim {
    /// The record's claim site, the `Location` `check` paired with its key.
    site: Location,
    /// The extent roots that are parameter nodes of the function.
    roots: BTreeSet<NodeKey>,
    /// The owning function is in a recursion group (FR-092). A stop at the
    /// site may then be a nested call's, and `Location` carries no frame.
    recursive: bool,
}

impl FunctionClaim {
    /// The claim as `select` reads it.
    fn kind(&self) -> Claim {
        match self {
            Self::Predicate => Claim::Predicate,
            Self::Application(_) => Claim::Application,
        }
    }

    /// The extent roots an application claim's replay source must bind; an
    /// owner parameter outside them may be left unbound (FR-098 "unread
    /// parameters"). `None` for a predicate claim, which binds every
    /// parameter.
    fn unread_roots(&self) -> Option<&BTreeSet<NodeKey>> {
        match self {
            Self::Predicate => None,
            Self::Application(application) => Some(&application.roots),
        }
    }
}

/// Resolve the claim `target`'s occurrence key selects for the function
/// `name`, after the function resolves and before the argument join: the
/// predicate claim at the function's `declaration` occurrence, else the
/// application claim of a `value-validity` record the function owns, else
/// [`ReplayRefusal::ClaimOccurrence`].
fn function_claim(
    declaring: Declaring<'_>,
    name: &QualifiedName,
    target: &ReplayTarget,
) -> Result<FunctionClaim, ReplayRefusal> {
    let (segment, callable) = lookup(declaring, name)?;
    let fault = |invariant| ReplayRefusal::Fault(InternalFault::new("replay", invariant));
    let key = target
        .occurrence_key()
        .ok_or_else(|| fault("function-request-carries-an-occurrence-key"))?;
    if *key == declaration_occurrence(callable.identity.clone()) {
        return Ok(FunctionClaim::Predicate);
    }
    let graph = declaring.package.graph();
    let refused = || ReplayRefusal::ClaimOccurrence {
        key: key.clone(),
        function: name.clone(),
    };
    let record = graph
        .requirements()
        .get(key)
        .filter(|record| {
            record.requirements().kind() == Capability::ValueValidity
                && record.owner() == Some(segment.as_str())
        })
        .ok_or_else(refused)?;
    let semantic = graph.semantic_graph();
    let node = semantic
        .node(key.node().clone())
        .map(|node| node.key().clone())
        .ok_or_else(refused)?;
    let site = graph
        .occurrence(node, key.origin())
        .cloned()
        .ok_or_else(|| fault("requirement-record-has-a-claim-site"))?;
    let roots = record
        .extent_roots()
        .iter()
        .map(|root| {
            semantic
                .node(root.clone())
                .map(|node| node.key().clone())
                .ok_or_else(|| fault("extent-root-is-a-node"))
        })
        .collect::<Result<_, _>>()?;
    let recursive = semantic
        .node(callable.identity.clone())
        .is_some_and(|node| node.recursion().is_some());
    Ok(FunctionClaim::Application(ApplicationClaim {
        site,
        roots,
        recursive,
    }))
}

/// A function node's one `declaration` occurrence (ADR-013 O-07): the node,
/// role `declaration`, ordinal 0. The one constructor `call_site`, the
/// predicate claim and the value-parity claim share.
pub(crate) fn declaration_occurrence(function: NodeKey) -> OccurrenceKey {
    OccurrenceKey::new(function, Origin::new(Role::new("declaration"), 0))
}

/// The selected function: its S6a name, and its parameter nodes and
/// declared types in declared order.
struct Selected {
    name: qsl_eval::value::QualifiedName,
    /// The function's node.
    identity: NodeKey,
    parameters: Vec<NodeKey>,
    types: Vec<ValueType>,
}

/// `callable`'s own parameter node keys, checked against its own declared
/// signature: the node lookup [`select`] and [`crate::call_site::call_site`]
/// each need to turn a `CallableFunction` into the parameter node ids a
/// `CanonicalAssignment` or a witness transcript names (ADR-013 O-25, C-11).
/// Shared so the two entries cannot silently drift apart (SR-780 FND-002).
pub(crate) fn callable_parameter_keys(
    package: &CheckedPackage,
    callable: &qsl_semantics::check::CallableFunction<'_>,
) -> Result<Vec<NodeKey>, InternalFault> {
    let parameters = package
        .graph()
        .semantic_graph()
        .node(callable.identity.clone())
        .and_then(|node| node.function_parameters())
        .ok_or_else(|| InternalFault::new("replay", "callable-identity-is-a-function-node"))?;
    if parameters.len() != callable.parameters.len() {
        return Err(InternalFault::new(
            "replay",
            "function-node-parameters-match-signature",
        ));
    }
    Ok(parameters)
}

/// FR-484: the preimage of the value-parity claim about the function node
/// `function` with the parameter nodes `parameters`, read from the checked
/// package alone: each argument names the parameter node and its declared
/// type node (`semantic_type`). The one builder `call_site` mints from and
/// `replay_value_parity` recomputes from, so the two cannot disagree.
pub(crate) fn value_parity_preimage(
    package: &CheckedPackage,
    function: NodeKey,
    parameters: &[NodeKey],
) -> Result<ValueParityPreimage, InternalFault> {
    let graph = package.graph().semantic_graph();
    let arguments = parameters
        .iter()
        .enumerate()
        .map(|(position, parameter)| {
            let semantic_type = graph
                .node(parameter.clone())
                .and_then(|node| node.semantic_type())
                .ok_or_else(|| {
                    InternalFault::new("replay", "parameter-node-has-a-semantic-type")
                })?;
            Ok(ValueParityArgument {
                parameter: parameter.clone(),
                position,
                semantic_type: semantic_type.clone(),
            })
        })
        .collect::<Result<Vec<_>, InternalFault>>()?;
    Ok(ValueParityPreimage {
        node: function.clone(),
        arguments,
    })
}

/// The function `name` selects in the recompiled package, by name lookup in
/// its declarations (OQ-5): its one segment and its callable. A name that
/// resolves to none refuses [`ReplayRefusal::UnknownFunction`].
/// QSpec FR-323 resolution, first step: the request's `function` replay
/// target, or the `unknown-member` refusal of `entry` for any other target
/// (FR-098, FR-357, FR-358), made after the request decodes and before it
/// recompiles.
pub(crate) fn function_target(
    request: &ReplayRequest,
    entry: ReplayEntry,
) -> Result<&QualifiedName, ReplayRefusal> {
    let (name, owner) = declared_function_target(request, entry)?;
    match (owner, request.replay_target()) {
        (Some(_), Some(target)) => Err(ReplayRefusal::ImportedFunctionTarget {
            target: Box::new(target.clone()),
            entry,
        }),
        (Some(_), None) | (None, _) => Ok(name),
    }
}

/// The `function` target's name and its declaring owner, if it names one.
pub(crate) fn declared_function_target(
    request: &ReplayRequest,
    entry: ReplayEntry,
) -> Result<(&QualifiedName, Option<&LibraryName>), ReplayRefusal> {
    let target = request
        .replay_target()
        .ok_or(ReplayRefusal::WrongModelGraphEntry)?;
    target
        .as_function()
        .map(|name| (name, target.function_owner()))
        .ok_or_else(|| wrong_target(target, entry, TargetMember::Function))
}

/// `entry` reads `member`, but the request's target is `target`.
pub(crate) fn wrong_target(
    target: &ReplayTarget,
    entry: ReplayEntry,
    member: TargetMember,
) -> ReplayRefusal {
    ReplayRefusal::WrongReplayTarget {
        target: Box::new(target.clone()),
        entry,
        member,
    }
}

/// FR-116, FR-122 check 2: the envelope's replay target is the request's.
pub(crate) fn check_envelope_target(
    request: &ReplayRequest,
    envelope: &ReplayTarget,
) -> Result<(), ReplayRefusal> {
    let Some(target) = request.replay_target() else {
        return Err(ReplayRefusal::WrongModelGraphEntry);
    };
    if target == envelope {
        return Ok(());
    }
    Err(ReplayRefusal::TargetMismatch {
        request: Box::new(target.clone()),
        envelope: Box::new(envelope.clone()),
    })
}

pub(crate) fn lookup<'a>(
    declaring: Declaring<'a>,
    name: &'a QualifiedName,
) -> Result<(&'a Identifier, qsl_semantics::check::CallableFunction<'a>), ReplayRefusal> {
    let unknown = || ReplayRefusal::UnknownFunction {
        selection: name.clone(),
        package: declaring.library.clone(),
    };
    let [segment] = name.segments() else {
        return Err(unknown());
    };
    let callable = declaring
        .package
        .graph()
        .callable(segment.as_str())
        .ok_or_else(unknown)?;
    // A function of another package is callable only through an import, which
    // ADR-015 D-5 allows when its signature is package-independent.
    if declaring.imported && !callable.is_importable() {
        return Err(ReplayRefusal::NotImportable {
            selection: name.clone(),
            package: declaring.library.clone(),
        });
    }
    Ok((segment, callable))
}

/// OQ-5: resolve `name` by name lookup in the recompiled package's
/// declarations -- the one name lookup after the check stage (R-06).
/// Complete-V1 declares no qualified names, so only a one-segment name
/// can resolve. A predicate replay of a function whose declared result is
/// not `Boolean` states no property, and a value-parity replay of one whose
/// result is `Boolean` is a predicate's; each refuses here, before any call
/// or charge.
fn select(
    declaring: Declaring<'_>,
    name: &QualifiedName,
    claim: Claim,
) -> Result<Selected, ReplayRefusal> {
    let package = declaring.package;
    let (segment, callable) = lookup(declaring, name)?;
    match (claim, *callable.result == ValueType::Boolean) {
        (Claim::Predicate, true) | (Claim::ValueParity, false) | (Claim::Application, _) => {}
        (Claim::Predicate, false) => {
            return Err(ReplayRefusal::NotAPredicate {
                selection: name.clone(),
                package: declaring.library.clone(),
            });
        }
        (Claim::ValueParity, true) => {
            return Err(ReplayRefusal::NotAValueFunction {
                selection: name.clone(),
                package: declaring.library.clone(),
            });
        }
    }
    let types: Vec<ValueType> = callable
        .parameters
        .iter()
        .map(|(_, value_type)| value_type.clone())
        .collect();
    let parameters = callable_parameter_keys(package, &callable).map_err(ReplayRefusal::Fault)?;
    let name = qsl_eval::value::QualifiedName::unqualified(segment.as_str()).map_err(|_| {
        ReplayRefusal::UnknownFunction {
            selection: name.clone(),
            package: declaring.library.clone(),
        }
    })?;
    Ok(Selected {
        name,
        identity: callable.identity,
        parameters,
        types,
    })
}

/// The call's arguments in declared parameter order (ADR-013 O-25, C-11):
/// an `Input` source's assignments joined by parameter node id, or a
/// `Witness` source's transcript decoded against one binding per
/// parameter, each naming the parameter's node id and declared type.
fn arguments(
    call: &Selected,
    source: &ReplaySource,
    obligation: ObligationIdentity,
    roots: Option<&BTreeSet<NodeKey>>,
) -> Result<Vec<WitnessValue>, ReplayRefusal> {
    // FR-098 "Application claim: unread parameters": a parameter outside the
    // claim's extent roots, of type `Boolean` or an integer type, may be left
    // unbound and takes its nearest-to-zero value.
    let filled = |position: usize| -> Option<WitnessValue> {
        let unread = roots.is_some_and(|roots| {
            call.parameters
                .get(position)
                .is_some_and(|parameter| !roots.contains(parameter))
        });
        if !unread {
            return None;
        }
        match call.types.get(position)? {
            ValueType::Boolean => Some(WitnessValue::Boolean(false)),
            ValueType::Integer => Some(WitnessValue::Integer(0)),
            ValueType::Int(interval) => {
                let nearest = if interval.contains(&quire_exact::Integer::zero()) {
                    quire_exact::Integer::zero()
                } else if interval.lower() > &quire_exact::Integer::zero() {
                    interval.lower().clone()
                } else {
                    interval.upper().clone()
                };
                // A bound beyond `i128` has no `WitnessValue`; the parameter then stays unbound.
                Some(WitnessValue::Integer(nearest.to_string().parse().ok()?))
            }
            _ => None,
        }
    };
    let values = match source {
        ReplaySource::Input(assignments) => {
            let mut values: Vec<Option<WitnessValue>> = std::iter::repeat_with(|| None)
                .take(call.parameters.len())
                .collect();
            for assignment in assignments {
                let position = call
                    .parameters
                    .iter()
                    .position(|parameter| *parameter == assignment.parameter)
                    .ok_or_else(|| ReplayRefusal::UnknownParameter(assignment.parameter.clone()))?;
                let slot = values
                    .get_mut(position)
                    .ok_or_else(|| ReplayRefusal::UnknownParameter(assignment.parameter.clone()))?;
                if slot.replace(assignment.value.clone()).is_some() {
                    return Err(ReplayRefusal::DuplicateArgument(
                        assignment.parameter.clone(),
                    ));
                }
            }
            values
                .into_iter()
                .zip(&call.parameters)
                .enumerate()
                .map(|(position, (value, parameter))| {
                    value
                        .or_else(|| filled(position))
                        .ok_or_else(|| ReplayRefusal::UnboundParameter(parameter.clone()))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        ReplaySource::Witness(witness) => {
            let bindings = call
                .parameters
                .iter()
                .zip(&call.types)
                .enumerate()
                .map(|(position, (parameter, value_type))| {
                    Ok(WitnessBinding {
                        parameter: parameter.clone(),
                        value_type: witness_type(position, value_type)?,
                    })
                })
                .collect::<Result<Vec<_>, ReplayRefusal>>()?;
            let decoded = witness
                .decode_optional(&bindings, |binding| {
                    call.parameters
                        .iter()
                        .position(|parameter| parameter.clone() == binding.parameter)
                        .is_some_and(|position| filled(position).is_some())
                })
                .map_err(|refusal| ReplayRefusal::Witness {
                    obligation,
                    refusal,
                })?;
            decoded
                .into_iter()
                .enumerate()
                .map(|(position, value)| {
                    value.or_else(|| filled(position)).ok_or_else(|| {
                        ReplayRefusal::Fault(InternalFault::new(
                            "replay",
                            "optional-witness-binding-is-filled",
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        ReplaySource::ObservedTemporal(_) => {
            return Err(ReplayRefusal::Fault(InternalFault::new(
                "replay",
                "observed-temporal-source-dispatched-by-replay-facade",
            )))
        }
        ReplaySource::ModelGraph(_) => {
            return Err(ReplayRefusal::Fault(InternalFault::new(
                "replay",
                "state-graph-source-dispatched-by-replay-facade",
            )))
        }
    };
    Ok(values)
}

/// The witness type a parameter of `value_type` is read as: `Boolean` for
/// `Boolean`, `I128` for an integer type, and `Canonical` (a witness value
/// text) for every other value type, the native `UUID` and `Timestamp`
/// included. A `Population<T>` and a `Map` are not values, and a
/// `BoundedQuantity` and `CarriedQuantity` have no native binding
/// (`argument_value` refuses them too), so none has a witness form and each
/// refuses `WrongValueKind` before the call.
fn witness_type(
    parameter: usize,
    value_type: &ValueType,
) -> Result<WitnessValueType, ReplayRefusal> {
    match value_type {
        ValueType::Boolean => Ok(WitnessValueType::Boolean),
        ValueType::Integer | ValueType::Int(_) => Ok(WitnessValueType::I128),
        ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::String
        | ValueType::Text(_)
        | ValueType::Enum(_)
        | ValueType::Option(_)
        | ValueType::Composite(_)
        | ValueType::Collection(_)
        | ValueType::Reference(_)
        | ValueType::Uuid
        | ValueType::Timestamp => Ok(WitnessValueType::Canonical),
        ValueType::Map(_)
        | ValueType::BoundedQuantity(_)
        | ValueType::CarriedQuantity(_)
        | ValueType::Population(_) => Err(ReplayRefusal::Input(InputRefusal::WrongValueKind {
            parameter,
        })),
    }
}

/// The replay's charges: what `meter` consumed of each counter.
pub(crate) fn consumed(meter: &Meter) -> ScalarLimits {
    charges(
        LimitKind::ALL
            .iter()
            .map(|kind| (*kind, meter.consumed(*kind))),
    )
}

/// Per-counter consumed totals as a replay result's charges: one
/// [`ScalarLimits`] member per [`LimitKind`], zero where none is given.
fn charges(consumed: impl IntoIterator<Item = (LimitKind, u64)>) -> ScalarLimits {
    let mut charges = ScalarLimits {
        integer_bits: 0,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 0,
        work_units: 0,
        result_units: 0,
    };
    for (kind, amount) in consumed {
        let counter = match kind {
            LimitKind::IntegerBits => &mut charges.integer_bits,
            LimitKind::DecimalDigits => &mut charges.decimal_digits,
            LimitKind::ScaleExpansion => &mut charges.scale_expansion,
            LimitKind::TextInputBytes => &mut charges.text_input_bytes,
            LimitKind::TextScalars => &mut charges.text_scalars,
            LimitKind::NormalizedScalars => &mut charges.normalized_scalars,
            LimitKind::UnitEdges => &mut charges.unit_edges,
            LimitKind::ValueOccurrences => &mut charges.value_occurrences,
            LimitKind::WorkUnits => &mut charges.work_units,
            LimitKind::ResultUnits => &mut charges.result_units,
        };
        *counter = amount;
    }
    charges
}

#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
mod carried_quantity_tests {
    use super::{witness_type, InputRefusal, ReplayRefusal};
    use ix_trace_rs::trace;

    fn carried() -> quire_exact::ValueType {
        let unit = quire_exact::UnitId::declared(
            quire_exact::NodeKey::new(
                "fixture",
                vec![quire_exact::PathSegment::Text("unit".into())],
                5,
            )
            .expect("a key"),
        );
        let domain = quire_exact::IntegerInterval::spanning(
            quire_exact::Integer::from(-2_i64),
            quire_exact::Integer::from(2_i64),
        );
        quire_exact::ValueType::CarriedQuantity(std::sync::Arc::new(
            quire_exact::CarriedQuantityType::new(
                unit,
                quire_exact::QuantityCarrier::Integer {
                    domain,
                    rounding: quire_exact::RoundingMode::Exact,
                },
            ),
        ))
    }

    /// FR-497: a carried quantity has no native binding, so no witness form;
    /// it refuses `WrongValueKind` before the call.
    #[trace("FR-497-AC-2")]
    #[test]
    fn a_carried_quantity_parameter_has_no_witness_form() {
        assert!(matches!(
            witness_type(3, &carried()),
            Err(ReplayRefusal::Input(InputRefusal::WrongValueKind {
                parameter: 3
            }))
        ));
    }
}
