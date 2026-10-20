// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-128 (ADR-018 CX-3, DL-5, UE-5, UE-6): [`replay_model_trace`], the
//! replay facade's entry for a finite-prefix model counterexample.
//!
//! It recompiles the request's package by FR-098's rules, refuses a stale
//! `package_id`, re-admits the subject from the byte provision and the
//! request's universes, re-executes each step of the prefix through FR-120's
//! `ModelSystem` (a step's post-state digest selects its successor among the
//! post-states of its transition identity), and then checks what the
//! counterexample claims of the prefix:
//!
//! - a formula counterexample (CX-3): a finite prefix or, with its
//!   [`Lasso`](crate::Lasso), a loop that closes at its entry state and is
//!   fair under the recompiled clause's fairness set, on which the clause's
//!   formula evaluates false by FR-125 (ADR-018 SM-8 reads the lasso);
//! - a deadlock counterexample (ADR-018 DL-5): no transition identity is
//!   enabled at the last state and no `terminal` declaration intends it;
//! - an undefined-evaluation counterexample of the deadlock-freedom item
//!   (UE-5, UE-6): the first replayed position at which a contract
//!   conjunction or a `terminal when` predicate is undefined, or the state
//!   constraint the payload names (ADR-021 SC-2), is the payload's `where`,
//!   at its locus, with its cause.
//!
//! The envelope's `clause_node` and `occurrence_key` resolve the item
//! before any admission. The deadlock-freedom item's identity is
//! [`deadlock_freedom_clause`] of the recompiled `package_id`; a payload
//! only that item has, under any other identity, or an authored payload
//! under that identity, refuses `stale_dependency`/`content-mismatch`. Any
//! other identity names an authored clause: the recompiled package's
//! temporal clause node of that `clause_node`, whose `claim` occurrence the
//! envelope's `occurrence_key` must equal, and whose fairness set the
//! payload's must equal; a package with no such node refuses
//! `stale_dependency`/`content-mismatch`. Replay needs no model checker and
//! no solver, and reads no path, environment variable or clock.

use std::collections::BTreeMap;

use qsl_eval::fairness::{
    check_loop, Constraint, EdgeLabel, Identity, LassoFairness, LoopStep, Strength,
};
use qsl_eval::model_subject::{
    ModelInstance, ModelLetters, ModelSubject, ProtocolSubject, RoleBinding,
};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimState, ModelSimulationRequest, ModelSystem, PopulationUniverse,
    TerminalClass,
};
use qsl_eval::simulation::protocol::{
    check_protocol_bounds, ProtocolRefusal, ProtocolSystem, ProtocolTransition, Sc,
};
use qsl_eval::simulation::{
    ConstraintValue, Expansion, ExpansionStop, NotSimulated, SubjectSystem, TransitionSystem,
    BOUND_POSITIONS,
};
use qsl_eval::temporal::AtomReading;
use qsl_eval::temporal::{
    evaluate_open_prefix, evaluate_with_enabledness, LetterStop, TemporalLetters, TemporalOutcome,
    TemporalPosition, TemporalTrace,
};
use qsl_foundation::diagnostic::{Category, Code, InternalFault};
use qsl_foundation::digest::{DigestDomain, DigestRecord, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{Activation, CheckedTemporalClause, FormulaNodeId};
use qsl_semantics::library::PackageId;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Meter, ScalarLimits};
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity, FairnessKind};
use quire_semantic_value::model_transition::ModelTransition;

use crate::bounds::ReplayLimits;
use crate::execute::{recompile, request_limits, ReplayRefusal};
use crate::model_check::counterexample::{
    deadlock_freedom_clause, undefined_finding, CounterexampleKind, Lasso, TemporalCounterexample,
    UndefinedEvaluation, UndefinedLocus, UndefinedSite,
};
use crate::request::{ReplayRequest, ReplayRequestWire};
use crate::result::{DisagreementCause, Verdict};
use crate::witness::WitnessEnvelope;

/// The subject members FR-128's request names beside FR-098's: the initial
/// states (documents of the byte provision), the universes, and the
/// per-expansion candidate bound the subject was checked under.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelTraceSubject {
    /// The subject's initial-state snapshots, in subject order.
    pub initial: Vec<DocumentRef>,
    /// The subject's universes.
    pub universes: Vec<PopulationUniverse>,
    /// The subject's expansion limits.
    pub expansion: ExpansionLimits,
}

/// FR-217's subject members beside FR-098's: the model subject's, the
/// protocol clause and its bindings (ADR-027 PS-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolTraceSubject {
    /// The model subject's members.
    pub model: ModelTraceSubject,
    /// The protocol clause's declared name.
    pub protocol: String,
    /// The object bound to its `over` parameter.
    pub over: ModelInstance,
    /// Each static role's binding.
    pub roles: Vec<RoleBinding>,
}

/// Why [`replay_model_trace`] settled no result (FR-128).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ModelTraceRefusal {
    /// The counterexample names an authored clause, and the recompiled
    /// package holds no temporal clause node (FR-337).
    #[error("stale_dependency/content-mismatch: the recompiled package holds no temporal clause node {clause_node}")]
    UnknownTemporalClause {
        /// The envelope's clause node.
        clause_node: WireNodeId,
    },
    /// `initial_state` is not an index of the subject's initial states.
    #[error("invalid_runtime_input/invalid-value: initial state {index} of {count}")]
    InitialState {
        /// The payload's index.
        index: usize,
        /// The subject's number of initial states.
        count: usize,
    },
    /// The envelope's clause identity is not the one its payload's item
    /// has: a deadlock-freedom payload under any identity but the derived
    /// item's, or an authored clause's payload under the derived item's.
    #[error("stale_dependency/content-mismatch: the envelope names clause {clause_node} at {occurrence_key}; the payload's item is {expected_node} at {expected_key}")]
    ItemIdentity {
        /// The envelope's clause node.
        clause_node: WireNodeId,
        /// The envelope's occurrence key.
        occurrence_key: Box<OccurrenceKey>,
        /// The clause node the payload's item has.
        expected_node: WireNodeId,
        /// The occurrence key the payload's item has.
        expected_key: Box<OccurrenceKey>,
    },
    /// The payload's fairness set is not the recompiled clause's.
    #[error("stale_dependency/content-mismatch: the payload's fairness set {payload:?} is not the recompiled clause's {recompiled:?}")]
    FairnessSet {
        /// The payload's fairness set.
        payload: Vec<FairnessConstraint>,
        /// The recompiled clause's fairness set.
        recompiled: Vec<FairnessConstraint>,
    },
    /// A formula counterexample names no instance of the clause's `over`
    /// parameter.
    #[error(
        "invalid_runtime_input/invalid-value: a formula counterexample names no `over` binding"
    )]
    OverBinding,
    /// The loop does not end at its entry state, is empty, or re-enters
    /// outside the prefix.
    #[error("invalid_runtime_input/invalid-value: the loop entered at position {entry} of {steps} steps does not close")]
    LoopOpen {
        /// The loop's entry position.
        entry: usize,
        /// The number of steps the lasso lists.
        steps: usize,
    },
    /// A `terminal-stutter` loop at a state that enables a transition
    /// identity.
    #[error("invalid_runtime_input/invalid-value: the terminal-stutter loop is at position {position}, which is not terminal")]
    StutterNotTerminal {
        /// The stutter loop's position.
        position: usize,
    },
    /// A bounded-profile prefix that does not end at a terminal state and
    /// does not decide the formula false whatever its continuation is: read
    /// as a closed trace it would show a falsity its continuation may not
    /// (ADR-018 CX-1).
    #[error("invalid_runtime_input/invalid-value: the prefix of {positions} positions does not end at a terminal state and does not decide the formula (horizon {horizon}) false")]
    PrefixShort {
        /// The formula's horizon.
        horizon: u64,
        /// The positions the prefix holds.
        positions: usize,
    },
    /// An undefined-evaluation counterexample of an authored clause, whose
    /// replay (FR-128-AC-5) is not built.
    #[error("unsupported_construct/not-yet-implemented: replay of an authored clause's undefined evaluation")]
    AuthoredUndefined,
    /// A clause with a guarded `on each ... when` activation, which EN-1
    /// refuses and replay does not read.
    #[error(
        "unsupported_construct/not-yet-implemented: the clause has a guarded `on each` activation"
    )]
    GuardedActivation,
    /// A refinement counterexample that names no refinement failure.
    #[error("invalid_runtime_input/invalid-value: the payload names no refinement failure")]
    NoRefinementFailure,
    /// A refinement counterexample that is a lasso, which FR-143's liveness
    /// failures write and replay does not build yet.
    #[error("unsupported_construct/not-yet-implemented: replay of a refinement lasso")]
    RefinementLasso,
    /// A step's transition identity is not enabled at its pre-state.
    #[error("invalid_runtime_input/invalid-value: step {step} is not enabled")]
    NotEnabled {
        /// The step's zero-based index in the prefix.
        step: usize,
        /// Its transition identity.
        transition: Box<ModelTransition>,
    },
    /// A protocol step's transition identity is not enabled at its
    /// pre-state (FR-217).
    #[error("invalid_runtime_input/invalid-value: protocol step {step} is not enabled")]
    ProtocolNotEnabled {
        /// The step's zero-based index in the prefix.
        step: usize,
        /// Its transition identity.
        transition: Box<ProtocolTransition>,
    },
    /// A protocol counterexample with a loop, whose replay against FR-212's
    /// fairness classes is not built.
    #[error("unsupported_construct/not-yet-implemented: replay of a protocol lasso")]
    ProtocolLasso,
    /// The subject's protocol system does not build (FR-205).
    #[error("{0}")]
    Protocol(ProtocolRefusal),
    /// No successor of a step's transition identity has the recorded
    /// post-state digest (FR-101's key mismatch).
    #[error(
        "stale_dependency/content-mismatch: step {step} reaches no state {}; it reaches {}",
        .recorded.hex(),
        .reached.iter().map(DigestRecord::hex).collect::<Vec<_>>().join(", ")
    )]
    PostState {
        /// The step's zero-based index in the prefix.
        step: usize,
        /// The recorded post-state digest.
        recorded: DigestRecord,
        /// The post-state digests the step's transition identity does
        /// reach, in successor order.
        reached: Vec<DigestRecord>,
    },
    /// The subject's roots are not simulated (FR-215's requires-bound
    /// pre-check): a ranged root of an unbounded type, which no check
    /// explores, so no counterexample of the subject is one a check wrote;
    /// or the classification stopped at a limit.
    #[error("the subject is not simulated: {0}")]
    NotSimulated(Box<NotSimulated>),
}

impl ModelTraceRefusal {
    /// The refusal's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::UnknownTemporalClause { .. }
            | Self::ItemIdentity { .. }
            | Self::PostState { .. } => Code::StaleDependency,
            Self::FairnessSet { .. } => Code::StaleDependency,
            Self::AuthoredUndefined
            | Self::GuardedActivation
            | Self::RefinementLasso
            | Self::ProtocolLasso => Code::UnsupportedConstruct,
            Self::Protocol(refusal) => refusal.code(),
            Self::NotSimulated(refusal) => not_simulated_code(refusal),
            Self::InitialState { .. }
            | Self::NotEnabled { .. }
            | Self::ProtocolNotEnabled { .. }
            | Self::OverBinding
            | Self::NoRefinementFailure
            | Self::PrefixShort { .. }
            | Self::LoopOpen { .. }
            | Self::StutterNotTerminal { .. } => Code::InvalidRuntimeInput,
        }
    }

    /// The refusal's catalog cause.
    pub fn cause(&self) -> &'static str {
        match self {
            Self::UnknownTemporalClause { .. }
            | Self::ItemIdentity { .. }
            | Self::FairnessSet { .. }
            | Self::PostState { .. } => "content-mismatch",
            Self::AuthoredUndefined
            | Self::GuardedActivation
            | Self::RefinementLasso
            | Self::ProtocolLasso => "not-yet-implemented",
            Self::Protocol(refusal) => refusal.cause(),
            Self::NotSimulated(refusal) => match refusal.catalog_code() {
                Some(code) => code.cause(),
                None => "invalid-value",
            },
            Self::InitialState { .. }
            | Self::NotEnabled { .. }
            | Self::ProtocolNotEnabled { .. }
            | Self::OverBinding
            | Self::NoRefinementFailure
            | Self::PrefixShort { .. }
            | Self::LoopOpen { .. }
            | Self::StutterNotTerminal { .. } => "invalid-value",
        }
    }
}

/// The code of a subject that is not simulated: a requires-bound subject
/// has no counterexample a check wrote (`invalid_runtime_input`), and a
/// classification that stopped keeps its own code.
fn not_simulated_code(refusal: &NotSimulated) -> Code {
    use qsl_semantics::family::ClassifyFailure;
    match refusal {
        NotSimulated::RequiresBound(_)
        | NotSimulated::EmptyInitial
        | NotSimulated::GeneratorMismatch { .. }
        | NotSimulated::KeyEncoding(qsl_eval::simulation::EncodingRefusal::NoEncoding(_)) => {
            Code::InvalidRuntimeInput
        }
        NotSimulated::Extent(ClassifyFailure::Limit(_))
        | NotSimulated::KeyEncoding(qsl_eval::simulation::EncodingRefusal::Limit(_)) => {
            Code::StageLimitExceeded
        }
        NotSimulated::KeyEncoding(qsl_eval::simulation::EncodingRefusal::Allocation { .. }) => {
            Code::ResourceExhausted
        }
        NotSimulated::Extent(ClassifyFailure::Fault(_)) => Code::RuntimeInvariant,
    }
}

/// The value a reproduced model counterexample evaluated to (FR-128
/// Outputs): the item's truth for a deadlock, or the reproduced undefined
/// evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelTraceValue {
    /// The item's value on the replayed prefix.
    Truth(bool),
    /// The undefined value reproduced at `where`.
    Undefined(UndefinedEvaluation),
}

/// FR-128's settlement of a model counterexample's replay.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelTraceSettlement {
    /// `reproduced-with-evaluated-witness`.
    Reproduced {
        /// The evaluated value.
        value: ModelTraceValue,
        /// The counterexample's trace position.
        trace_position: Option<u64>,
    },
    /// `inconclusive`: the replay disagrees with the engine
    /// (`Verdicts`) or completed no value (`NoValue`).
    Inconclusive(DisagreementCause),
}

/// FR-133 (ADR-019 SV-6): a strong constraint that would exclude a
/// reproduced lasso and that its clause does not carry. It is a
/// warning-severity diagnostic, [`Self::CODE`], on the item's terminal
/// record, outside the counterexample, its replay identity and the
/// obligation identity, and it changes no verdict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrongFairnessHint {
    /// The suggested constraint: kind `Strong`, the operation and the
    /// granularity.
    pub suggested: FairnessConstraint,
    /// For a `Each` hint, the transition identity that is enabled in the
    /// loop and never taken.
    pub identity: Option<ModelTransition>,
    /// The first loop position (zero-based from the loop's entry, so the
    /// loop's first state is 0 whatever the stem's length) at which the
    /// operation, or for `Each` the identity, is enabled.
    pub first_enabled_position: u64,
}

impl StrongFairnessHint {
    /// The diagnostic code of a hint.
    pub const CODE: &'static str = "fairness.strong-would-exclude";
}

/// The result of [`replay_model_trace`]: the settlement, the strong-fairness
/// hints of a reproduced liveness lasso (FR-133) and the recompiled
/// `package_id` it was settled against.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelTraceReplayResult {
    /// The settlement.
    pub settlement: ModelTraceSettlement,
    /// The hints, in the model's canonical operation order; empty unless the
    /// settlement reproduced a lasso.
    pub hints: Vec<StrongFairnessHint>,
    /// The recompiled `package_id`.
    pub package_id: PackageId,
}

/// The disagreement of a replay that reads `replayed` where a model
/// counterexample refutes its item: the proved verdict is `violation`.
pub(super) fn disagreement(replayed: Category) -> DisagreementCause {
    DisagreementCause::Verdicts {
        proved: Verdict::from_category(Category::Violation),
        replayed: Verdict::from_category(replayed),
    }
}

/// The disagreement of a replay that completed no value.
pub(super) fn no_value_cause() -> DisagreementCause {
    DisagreementCause::NoValue {
        proved: Verdict::from_category(Category::Violation),
        replayed: Verdict::from_category(Category::Incomplete),
    }
}

fn disagrees(replayed: Category) -> ModelTraceSettlement {
    ModelTraceSettlement::Inconclusive(disagreement(replayed))
}

fn no_value() -> ModelTraceSettlement {
    ModelTraceSettlement::Inconclusive(no_value_cause())
}

/// FR-128: replay `envelope`'s model counterexample over `subject`, with
/// FR-098's request `wire` supplying the package reference, the byte
/// provision (domain packages and initial-state snapshots by `sha256-jcs`
/// digest) and the limits. `letters` reads the `holds` atoms of an authored
/// clause at a replayed state (FR-125), as the engine that found the
/// counterexample read them.
///
/// Refuses, with no partial result, when the request does not decode, by
/// FR-098's recompile rules, on a stale `package_id`, for a counterexample
/// of a clause the recompiled package does not hold or whose occurrence or
/// fairness set differ, when the subject does not admit (FR-106's record),
/// on an `initial_state` outside the subject, on a step that is not
/// enabled, on a post-state digest no successor has, on a loop that does
/// not close, on a `terminal-stutter` loop at a state that is not terminal,
/// and on an unfair lasso. An expansion that stops while replaying
/// completes no value and settles `NoValue`.
///
/// # Errors
///
/// [`ReplayRefusal`] as above.
pub fn replay_model_trace(
    wire: ReplayRequestWire,
    subject: &ModelTraceSubject,
    envelope: &WitnessEnvelope<TemporalCounterexample>,
    letters: &dyn ModelLetters,
    replay_limits: ReplayLimits,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    replay_on(
        wire,
        envelope,
        replay_limits,
        |package, simulation, meter| {
            let clause = resolve_item(envelope, package.0, &package.1)?;
            let system = ModelSystem::new(simulation(subject)).map_err(ReplayRefusal::Admission)?;
            let formula = clause.map(|clause| FormulaReplay {
                clause,
                letters,
                meter,
            });
            settle(&system, envelope.family_payload(), formula)
        },
    )
}

/// FR-217 (ADR-027 PX-2): replay `envelope`'s protocol counterexample over
/// `subject` by [`replay_model_trace`]'s rules, re-executing each step
/// through the subject's `ProtocolSystem<Sc>`. For a deadlock it checks
/// that no protocol step is enabled at the last state, that some protocol
/// instance is live there, and that the blocked threads recompute equal
/// to the payload's.
///
/// # Errors
///
/// [`ReplayRefusal`] as [`replay_model_trace`] returns it; a protocol that
/// does not build refuses with its own code, and a protocol lasso refuses
/// `unsupported_construct`/`not-yet-implemented`.
pub fn replay_protocol_trace(
    wire: ReplayRequestWire,
    subject: &ProtocolTraceSubject,
    envelope: &WitnessEnvelope<TemporalCounterexample<ProtocolTransition>>,
    letters: &dyn ModelLetters,
    replay_limits: ReplayLimits,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    replay_on(
        wire,
        envelope,
        replay_limits,
        |package, simulation, meter| {
            let clause = resolve_item(envelope, package.0, &package.1)?;
            let payload = envelope.family_payload();
            if payload.lasso.is_some() {
                return Err(refused(ModelTraceRefusal::ProtocolLasso));
            }
            let simulation = simulation(&subject.model);
            let protocol = package
                .0
                .graph()
                .protocol(&subject.protocol)
                .ok_or_else(|| {
                    refused(ModelTraceRefusal::Protocol(
                        ProtocolRefusal::UnknownProtocol {
                            protocol: subject.protocol.clone(),
                        },
                    ))
                })?;
            let model = ModelSystem::new(simulation.clone()).map_err(ReplayRefusal::Admission)?;
            check_protocol_bounds(&model, protocol, BOUND_POSITIONS)
                .map_err(|refusal| refused(ModelTraceRefusal::NotSimulated(Box::new(refusal))))?;
            let protocol_subject = ProtocolSubject {
                model: ModelSubject {
                    simulation,
                    proof_bounds: Vec::new(),
                },
                protocol: subject.protocol.clone(),
                over: subject.over.clone(),
                roles: subject.roles.clone(),
            };
            let system = ProtocolSystem::new(model, &protocol_subject, Sc)
                .map_err(|refusal| refused(ModelTraceRefusal::Protocol(refusal)))?;
            let positions = match replay_prefix(&system, payload, protocol_not_enabled)? {
                Some(positions) => positions,
                None => return Ok((no_value(), Vec::new())),
            };
            let settlement = match (&payload.kind, clause) {
                (CounterexampleKind::Deadlock, _) => deadlock(&system, payload, &positions)?,
                (CounterexampleKind::UndefinedEvaluation(claimed), _) => {
                    undefined(&system, &NoConstraints, payload, &positions, claimed)?
                }
                (CounterexampleKind::Formula, Some(clause)) => {
                    let formula = FormulaReplay {
                        clause,
                        letters,
                        meter,
                    };
                    check_formula(&system, payload, &positions, &formula, |_| {
                        Err(fault("a-protocol-lasso-is-refused-before-replay"))
                    })?
                    .0
                }
                (CounterexampleKind::Formula, None) => {
                    return Err(fault("formula-counterexample-names-a-clause"))
                }
            };
            Ok((settlement, Vec::new()))
        },
    )
}

/// The recompiled package and its `package_id` record.
type ReplayPackage<'p> = (&'p CheckedPackage, DigestRecord);

/// FR-128's recompile, `package_id` check and subject re-admission, then
/// `replay` over the recompiled package, the subject's simulation request
/// and the request's evaluation meter.
fn replay_on<P: crate::witness::FamilyPayload>(
    wire: ReplayRequestWire,
    envelope: &WitnessEnvelope<P>,
    replay_limits: ReplayLimits,
    replay: impl for<'p> FnOnce(
        ReplayPackage<'p>,
        &dyn Fn(&ModelTraceSubject) -> ModelSimulationRequest<'p>,
        ScalarLimits,
    ) -> Result<
        (ModelTraceSettlement, Vec<StrongFairnessHint>),
        ReplayRefusal,
    >,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    let request = ReplayRequest::decode(wire, replay_limits)?;
    let limits = request_limits(
        request.stage_limits(),
        request.accounting_limits(),
        replay_limits,
    )?;
    let compiled = recompile(&request, &limits)?;
    let package_id = compiled.emitted.package().package_id();
    if !package_id.matches(&envelope.package_id()) {
        return Err(ReplayRefusal::PackageIdMismatch {
            requested: envelope.package_id(),
            recompiled: package_id,
        });
    }
    let documents: BTreeMap<[u8; 32], Vec<u8>> = request
        .byte_provision()
        .entries()
        .filter(|(digest, _)| digest.domain() == DigestDomain::Sha256Jcs)
        .map(|(digest, bytes)| (*digest.as_bytes(), bytes.to_vec()))
        .collect();
    let meter = request.accounting_limits();
    let package = compiled.checked.package();
    let simulation = |subject: &ModelTraceSubject| ModelSimulationRequest {
        package,
        packages: &compiled.packages,
        normalization: limits.spine.model,
        snapshots: &documents,
        initial: subject.initial.clone(),
        universes: subject.universes.clone(),
        observation: limits.observation,
        expansion: subject.expansion,
        meter,
    };
    let (settlement, hints) = replay((package, package_id.record()), &simulation, meter)?;
    Ok(ModelTraceReplayResult {
        settlement,
        hints,
        package_id,
    })
}

pub(super) fn refused(refusal: ModelTraceRefusal) -> ReplayRefusal {
    ReplayRefusal::ModelTrace(Box::new(refusal))
}

/// FR-128: resolve the envelope's item by its `clause_node` and
/// `occurrence_key` against the recompiled package `package_id`. The
/// derived deadlock-freedom item's identity is [`deadlock_freedom_clause`];
/// its payload must be one only that item has, and a payload of that shape
/// under any other identity, or an authored payload under the derived
/// identity, refuses [`ModelTraceRefusal::ItemIdentity`]. Any other identity
/// names an authored clause: the package's temporal clause node of that
/// `clause_node`, whose `claim` occurrence the envelope must carry and whose
/// fairness set the payload must equal. A package with no such node refuses
/// [`ModelTraceRefusal::UnknownTemporalClause`].
fn resolve_item<'p, T: Clone + std::fmt::Debug + Eq>(
    envelope: &WitnessEnvelope<TemporalCounterexample<T>>,
    package: &'p CheckedPackage,
    package_id: &DigestRecord,
) -> Result<Option<&'p CheckedTemporalClause>, ReplayRefusal> {
    let (node, occurrence) = deadlock_freedom_clause(package_id);
    let derived = envelope.clause_node() == node && *envelope.occurrence_key() == occurrence;
    let deadlock_payload = names_deadlock_item(envelope.family_payload());
    if derived && deadlock_payload {
        return Ok(None);
    }
    let identity_mismatch = |expected_node, expected_key| {
        refused(ModelTraceRefusal::ItemIdentity {
            clause_node: envelope.clause_node(),
            occurrence_key: Box::new(envelope.occurrence_key().clone()),
            expected_node,
            expected_key: Box::new(expected_key),
        })
    };
    if derived != deadlock_payload && deadlock_payload {
        return Err(identity_mismatch(node, occurrence));
    }
    if derived {
        return Err(identity_mismatch(
            envelope.clause_node(),
            envelope.occurrence_key().clone(),
        ));
    }
    let found = package.graph().temporal_clauses().iter().find(|candidate| {
        WireNodeId::from_digest(*candidate.identity().as_bytes()) == envelope.clause_node()
    });
    let Some(found) = found else {
        return Err(refused(ModelTraceRefusal::UnknownTemporalClause {
            clause_node: envelope.clause_node(),
        }));
    };
    let expected = OccurrenceKey::new(envelope.clause_node(), found.claim().clone());
    if expected != *envelope.occurrence_key() {
        return Err(identity_mismatch(envelope.clause_node(), expected));
    }
    let clause = found.clause();
    let payload = envelope.family_payload();
    if payload.fairness != clause.fairness() {
        return Err(refused(ModelTraceRefusal::FairnessSet {
            payload: payload.fairness.clone(),
            recompiled: clause.fairness().to_vec(),
        }));
    }
    if matches!(payload.kind, CounterexampleKind::UndefinedEvaluation(_)) {
        return Err(refused(ModelTraceRefusal::AuthoredUndefined));
    }
    if matches!(clause.activation(), Activation::Each { guarded: true }) {
        return Err(refused(ModelTraceRefusal::GuardedActivation));
    }
    Ok(Some(clause))
}

/// Whether the payload's shape is one only the deadlock-freedom item has: a
/// deadlock, or an undefined model-side expression with no `over` binding.
fn names_deadlock_item<T>(payload: &TemporalCounterexample<T>) -> bool {
    match &payload.kind {
        CounterexampleKind::Deadlock => true,
        CounterexampleKind::UndefinedEvaluation(evaluation) => {
            payload.over_binding.is_none()
                && !matches!(evaluation.site.locus, UndefinedLocus::Atom(_))
        }
        CounterexampleKind::Formula => false,
    }
}

pub(super) fn fault(invariant: &'static str) -> ReplayRefusal {
    ReplayRefusal::Fault(InternalFault::new("replay-model-trace", invariant))
}

/// One replayed position: its state and digest.
pub(super) struct Position<State = ModelSimState> {
    pub(super) state: State,
    pub(super) digest: DigestRecord,
}

/// One expansion of a subject's state.
type SubjectExpansion<S> = Expansion<
    <S as TransitionSystem>::TransitionId,
    <S as TransitionSystem>::State,
    qsl_eval::simulation::model::ModelFinding,
>;

/// The successors of `state`, or the stop an expansion returned.
#[allow(
    clippy::type_complexity,
    reason = "an expansion or its stop, or a refusal"
)]
fn expand<S: SubjectSystem>(
    system: &S,
    state: &S::State,
) -> Result<Result<SubjectExpansion<S>, ExpansionStop>, ReplayRefusal> {
    match system.successors(state) {
        Ok(expansion) => Ok(Ok(expansion)),
        Err(stop) if stop.is_internal_failure() => Err(fault("model-expansion")),
        Err(stop) => Ok(Err(stop)),
    }
}

/// What replaying an authored clause's counterexample reads beside the
/// model: the recompiled clause, the seam to the one clause evaluator, and
/// the evaluation budget.
struct FormulaReplay<'a> {
    clause: &'a CheckedTemporalClause,
    letters: &'a dyn ModelLetters,
    meter: ScalarLimits,
}

/// The refusal of a model step that is not enabled.
fn model_not_enabled(step: usize, transition: &ModelTransition) -> ModelTraceRefusal {
    ModelTraceRefusal::NotEnabled {
        step,
        transition: Box::new(transition.clone()),
    }
}

/// The refusal of a protocol step that is not enabled.
fn protocol_not_enabled(step: usize, transition: &ProtocolTransition) -> ModelTraceRefusal {
    ModelTraceRefusal::ProtocolNotEnabled {
        step,
        transition: Box::new(transition.clone()),
    }
}

/// Re-execute the payload's prefix and check what it claims.
fn settle(
    system: &ModelSystem<'_>,
    payload: &TemporalCounterexample,
    formula: Option<FormulaReplay<'_>>,
) -> Result<(ModelTraceSettlement, Vec<StrongFairnessHint>), ReplayRefusal> {
    let Some(positions) = replay_prefix(system, payload, model_not_enabled)? else {
        return Ok((no_value(), Vec::new()));
    };
    if let Some(lasso) = payload.lasso {
        close_loop(system, &positions, lasso)?;
    }
    match (&payload.kind, formula) {
        (CounterexampleKind::Deadlock, _) => {
            deadlock(system, payload, &positions).map(|settlement| (settlement, Vec::new()))
        }
        (CounterexampleKind::UndefinedEvaluation(claimed), _) => {
            undefined(system, system, payload, &positions, claimed)
                .map(|settlement| (settlement, Vec::new()))
        }
        (CounterexampleKind::Formula, Some(formula)) => {
            let (settlement, loop_states) =
                check_formula(system, payload, &positions, &formula, |lasso| {
                    let Some(states) = loop_states_of(system, payload, &positions, lasso)? else {
                        return Ok(Err(no_value()));
                    };
                    if let Some(constraint) =
                        unfair_constraint(system, &states, lasso, formula.clause)?
                    {
                        return Err(ReplayRefusal::UnfairLasso { constraint });
                    }
                    Ok(Ok(states))
                })?;
            let hints = if matches!(settlement, ModelTraceSettlement::Reproduced { .. }) {
                strong_hints(system, &loop_states)
            } else {
                Vec::new()
            };
            Ok((settlement, hints))
        }
        (CounterexampleKind::Formula, None) => Err(fault("formula-counterexample-names-a-clause")),
    }
}

/// Re-execute a model counterexample's prefix from its initial state, one
/// position per state: a step's post-state digest selects its successor
/// among the post-states of its transition identity. `None` when an
/// expansion stops (the replay then completes no value).
///
/// # Errors
///
/// [`ModelTraceRefusal::InitialState`], [`ModelTraceRefusal::NotEnabled`] or
/// [`ModelTraceRefusal::PostState`].
pub(super) fn reexecute(
    system: &ModelSystem<'_>,
    payload: &TemporalCounterexample,
) -> Result<Option<Vec<Position>>, ReplayRefusal> {
    replay_prefix(system, payload, model_not_enabled)
}

/// Re-execute the payload's prefix from its initial state: each step's
/// identity must be enabled (else `not_enabled` refuses) and reach its
/// recorded post-state digest. `None` when an expansion stops.
fn replay_prefix<S: SubjectSystem>(
    system: &S,
    payload: &TemporalCounterexample<S::TransitionId>,
    not_enabled: fn(usize, &S::TransitionId) -> ModelTraceRefusal,
) -> Result<Option<Vec<Position<S::State>>>, ReplayRefusal> {
    let initial = system.initial();
    let count = initial.len();
    let state = initial
        .into_iter()
        .nth(payload.initial_state)
        .ok_or_else(|| {
            refused(ModelTraceRefusal::InitialState {
                index: payload.initial_state,
                count,
            })
        })?;
    let digest = digest_of(system, &state)?;
    let mut positions = vec![Position { state, digest }];
    for (index, step) in payload.prefix.iter().enumerate() {
        let current = positions.last().ok_or_else(|| fault("a-position-exists"))?;
        let expansion = match expand(system, &current.state)? {
            Ok(expansion) => expansion,
            Err(_) => return Ok(None),
        };
        let mut enabled = false;
        let mut reached = None;
        let mut digests = Vec::new();
        for (transition, next) in expansion.successors {
            if transition != step.transition {
                continue;
            }
            enabled = true;
            let digest = digest_of(system, &next)?;
            if digest == step.post_state {
                reached = Some(Position {
                    state: next,
                    digest,
                });
                break;
            }
            digests.push(digest);
        }
        if !enabled {
            return Err(refused(not_enabled(index, &step.transition)));
        }
        let next = reached.ok_or_else(|| {
            refused(ModelTraceRefusal::PostState {
                step: index,
                recorded: step.post_state,
                reached: digests,
            })
        })?;
        positions.push(next);
    }
    Ok(Some(positions))
}

/// FR-128: the lasso's loop must end at its entry state; a
/// `terminal-stutter` loop must stand at a state that enables no transition
/// identity (FR-124, ADR-018 SM-4), and a loop of model steps must hold one.
fn close_loop(
    system: &ModelSystem<'_>,
    positions: &[Position],
    lasso: Lasso,
) -> Result<(), ReplayRefusal> {
    let steps = positions.len() - 1;
    let open = || {
        refused(ModelTraceRefusal::LoopOpen {
            entry: lasso.entry,
            steps,
        })
    };
    if lasso.terminal_stutter {
        if lasso.entry != steps {
            return Err(open());
        }
        let last = positions.last().ok_or_else(|| fault("a-position-exists"))?;
        // An expansion that stops is not a terminal state.
        let terminal = matches!(expand(system, &last.state)?, Ok(e) if e.successors.is_empty());
        return if terminal {
            Ok(())
        } else {
            Err(refused(ModelTraceRefusal::StutterNotTerminal {
                position: steps,
            }))
        };
    }
    let entry = positions.get(lasso.entry).ok_or_else(open)?;
    let last = positions.last().ok_or_else(|| fault("a-position-exists"))?;
    if lasso.entry >= steps || entry.digest != last.digest {
        return Err(open());
    }
    Ok(())
}

/// The `holds` atoms of the clause at a replayed position, for the one
/// instance the counterexample binds, as FR-125's temporal evaluator reads
/// them (ADR-018 SM-8: a lasso's positions past the prefix read the loop's
/// states).
struct ReplayedLetters<'a, S: SubjectSystem> {
    system: &'a S,
    letters: &'a dyn ModelLetters,
    instance: &'a ModelInstance,
    positions: &'a [Position<S::State>],
}

impl<S: SubjectSystem> TemporalLetters for ReplayedLetters<'_, S> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let state = usize::try_from(position.index())
            .ok()
            .and_then(|index| self.positions.get(index))
            .ok_or_else(|| {
                LetterStop::Fault(InternalFault::new(
                    "replay-model-trace",
                    "the-evaluator-reads-a-replayed-position",
                ))
            })?;
        Ok(self.letters.atom(
            atom,
            self.instance,
            &self.system.atom_state(&state.state),
            meter,
        )?)
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        Err(LetterStop::Fault(InternalFault::new(
            "replay-model-trace",
            "a-guarded-activation-is-never-replayed",
        )))
    }
}

/// FR-128 for `kind: Formula`: a lasso is checked against the fairness set,
/// reading enabledness from `ModelSystem` at every loop state, and then both
/// it and a finite prefix are evaluated by FR-125. The formula `false`
/// reproduces; `true`, or no value, settles `inconclusive`. A reproduced
/// lasso also names the strong constraints that would exclude it (FR-133),
/// from the same enabled sets.
#[allow(
    clippy::type_complexity,
    reason = "the loop's states, its stop, or a refusal"
)]
fn check_formula<S: SubjectSystem>(
    system: &S,
    payload: &TemporalCounterexample<S::TransitionId>,
    positions: &[Position<S::State>],
    formula: &FormulaReplay<'_>,
    fair_loop: impl FnOnce(Lasso) -> Result<Result<Vec<LoopState>, ModelTraceSettlement>, ReplayRefusal>,
) -> Result<(ModelTraceSettlement, Vec<LoopState>), ReplayRefusal> {
    let instance = payload
        .over_binding
        .as_ref()
        .ok_or_else(|| refused(ModelTraceRefusal::OverBinding))?;
    let mut letters = ReplayedLetters {
        system,
        letters: formula.letters,
        instance,
        positions,
    };
    let mut meter = Meter::new(formula.meter);
    let mut loop_states = Vec::new();
    let trace = match payload.lasso {
        Some(lasso) => {
            match fair_loop(lasso)? {
                Ok(states) => loop_states = states,
                Err(settlement) => return Ok((settlement, Vec::new())),
            }
            let steps = positions.len() - 1;
            TemporalTrace::Lasso {
                prefix: position_of(lasso.entry),
                loop_len: if lasso.terminal_stutter {
                    1
                } else {
                    position_of(steps - lasso.entry)
                },
            }
        }
        None => TemporalTrace::Finite {
            len: position_of(positions.len()),
        },
    };
    // The fairness set is checked above against the replayed enabledness; a
    // bad prefix refutes whatever the set says about the continuation.
    //
    // Under a bounded profile a prefix reads as a closed trace, false past
    // its end, only where it ends at a terminal state (SM-4). Anywhere else
    // its continuation is unknown, and only a falsity every continuation
    // shares refutes (ADR-018 CX-1).
    let open = payload.lasso.is_none()
        && formula.clause.profile().is_bounded()
        && !ends_terminal(system, positions)?;
    let outcome = if open {
        evaluate_open_prefix(
            formula.clause,
            position_of(positions.len()),
            &mut letters,
            &mut meter,
        )
    } else {
        evaluate_with_enabledness(formula.clause, trace, &mut letters, &mut meter)
    };
    if open
        && !matches!(
            outcome,
            Ok(TemporalOutcome::Violated { .. } | TemporalOutcome::Incomplete(_))
        )
    {
        return Err(refused(ModelTraceRefusal::PrefixShort {
            horizon: formula.clause.horizon().unwrap_or(0),
            positions: positions.len(),
        }));
    }
    Ok(match outcome {
        // A lasso is evaluated from its origin (ADR-018 section 6): a clause
        // false only at a later activation is not the refutation EN-1 found.
        Ok(TemporalOutcome::Violated { at }) if payload.lasso.is_some() && at.index() != 0 => {
            (disagrees(Category::Success), Vec::new())
        }
        Ok(TemporalOutcome::Violated { .. }) => (
            ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Truth(false),
                trace_position: payload.trace_position(),
            },
            loop_states,
        ),
        Ok(TemporalOutcome::Holds) => (disagrees(Category::Success), Vec::new()),
        Ok(_) => (no_value(), Vec::new()),
        Err(qsl_eval::temporal::TemporalRefusal::Fault(_)) => {
            return Err(fault("temporal-evaluation"))
        }
        Err(_) => (no_value(), Vec::new()),
    })
}

/// Whether the last replayed state is terminal: no transition identity is
/// enabled at it (ADR-018 SM-4).
fn ends_terminal<S: SubjectSystem>(
    system: &S,
    positions: &[Position<S::State>],
) -> Result<bool, ReplayRefusal> {
    Ok(match positions.last() {
        Some(position) => matches!(
            expand(system, &position.state)?,
            Ok(expansion) if expansion.successors.is_empty()
        ),
        None => false,
    })
}

/// One model step of a lasso's loop: where it starts, the transition
/// identities enabled there (FA-2) and the one it takes.
struct LoopState {
    /// The step's position in the loop, from its entry.
    position: usize,
    enabled: Vec<ModelTransition>,
    taken: ModelTransition,
}

/// The model steps of the lasso's loop with the identities enabled at each
/// state they leave, read from the model's own successor relation. The
/// terminal stutter step is outside every constraint and enables none
/// (SM-4), so a stutter loop has none. `None` when an expansion stops.
fn loop_states_of(
    system: &ModelSystem<'_>,
    payload: &TemporalCounterexample,
    positions: &[Position],
    lasso: Lasso,
) -> Result<Option<Vec<LoopState>>, ReplayRefusal> {
    let mut states = Vec::new();
    if lasso.terminal_stutter {
        return Ok(Some(states));
    }
    for (index, (position, step)) in positions
        .iter()
        .zip(&payload.prefix)
        .enumerate()
        .skip(lasso.entry)
    {
        let Ok(expansion) = expand(system, &position.state)? else {
            return Ok(None);
        };
        states.push(LoopState {
            position: index - lasso.entry,
            enabled: expansion
                .successors
                .into_iter()
                .map(|(transition, _)| transition)
                .collect(),
            taken: step.transition.clone(),
        });
    }
    Ok(Some(states))
}

/// ADR-018 FA-3: the first constraint of the clause's fairness set that the
/// lasso's loop is unfair under. A `each` constraint is one constraint per
/// transition identity of its operation that the loop takes or enables; a
/// `whole` constraint is one over all of them.
fn unfair_constraint(
    system: &ModelSystem<'_>,
    states: &[LoopState],
    lasso: Lasso,
    clause: &CheckedTemporalClause,
) -> Result<Option<FairnessConstraint>, ReplayRefusal> {
    let mut numbered: BTreeMap<ModelTransition, Identity> = BTreeMap::new();
    let mut number = |transition: &ModelTransition| {
        let next = u64::try_from(numbered.len()).unwrap_or(u64::MAX);
        *numbered
            .entry(transition.clone())
            .or_insert_with(|| Identity::new(next))
    };
    let mut steps = Vec::new();
    if lasso.terminal_stutter {
        steps.push(LoopStep::new([], EdgeLabel::Stutter));
    }
    for state in states {
        let enabled: Vec<Identity> = state.enabled.iter().map(&mut number).collect();
        steps.push(LoopStep::new(
            enabled,
            EdgeLabel::Transition(number(&state.taken)),
        ));
    }
    // The constraints, each remembering the authored constraint it came from.
    let mut constraints = Vec::new();
    let mut authored = Vec::new();
    for constraint in clause.fairness() {
        let strength = Strength::from(constraint.kind());
        let operation = constraint.operation();
        let text = system
            .operation_text(operation.declaring_type(), operation.name())
            .ok_or_else(|| fault("a-recompiled-fairness-names-a-declared-operation"))?;
        let covered: Vec<Identity> = numbered
            .iter()
            .filter(|(transition, _)| transition.operation() == text)
            .map(|(_, identity)| *identity)
            .collect();
        match constraint.granularity() {
            FairnessGranularity::Whole => {
                constraints.push(Constraint::new(strength, covered));
                authored.push(constraint);
            }
            FairnessGranularity::Each => {
                for identity in covered {
                    constraints.push(Constraint::new(strength, [identity]));
                    authored.push(constraint);
                }
            }
        }
    }
    match check_loop(&steps, &constraints) {
        Ok(LassoFairness::Fair) => Ok(None),
        Ok(LassoFairness::Unfair { constraint }) => authored
            .get(constraint)
            .map(|constraint| Some((*constraint).clone()))
            .ok_or_else(|| fault("an-unfair-constraint-is-one-of-the-set")),
        Err(_) => Err(fault("a-lasso-loop-has-a-step")),
    }
}

/// FR-133 (ADR-019 SV-6): for each operation of the model in canonical
/// order, the strong constraint that would exclude the lasso. An operation
/// enabled at some loop state and taken by no loop step is named `Whole`;
/// otherwise the first transition identity of it, in canonical transition
/// order, that is enabled at a loop state and taken by no loop step is named
/// `Each`. A terminal stutter loop, where nothing is enabled, gets none.
fn strong_hints(system: &ModelSystem<'_>, states: &[LoopState]) -> Vec<StrongFairnessHint> {
    let mut hints = Vec::new();
    for (declaring, name, text) in system.operations() {
        let of_operation = |transition: &ModelTransition| transition.operation() == text;
        let taken =
            |identity: &ModelTransition| states.iter().any(|state| state.taken == *identity);
        let first_enabled = |wanted: &dyn Fn(&ModelTransition) -> bool| {
            states
                .iter()
                .find(|state| state.enabled.iter().any(wanted))
                .map(|state| position_of(state.position))
        };
        let operation =
            quire_semantic_value::fairness::FairnessOperation::new(declaring, name.to_owned());
        let named = |granularity| {
            FairnessConstraint::new(FairnessKind::Strong, operation.clone(), granularity)
        };
        let whole = first_enabled(&of_operation)
            .filter(|_| !states.iter().any(|state| of_operation(&state.taken)));
        let (suggested, identity, position) = if let Some(position) = whole {
            (named(FairnessGranularity::Whole), None, position)
        } else {
            let never_taken = states
                .iter()
                .flat_map(|state| &state.enabled)
                .filter(|transition| of_operation(transition) && !taken(transition))
                .min();
            let Some(identity) = never_taken else {
                continue;
            };
            let Some(position) = first_enabled(&|transition| transition == identity) else {
                continue;
            };
            (
                named(FairnessGranularity::Each),
                Some(identity.clone()),
                position,
            )
        };
        hints.push(StrongFairnessHint {
            suggested,
            identity,
            first_enabled_position: position,
        });
    }
    hints
}

fn digest_of<S: SubjectSystem>(
    system: &S,
    state: &S::State,
) -> Result<DigestRecord, ReplayRefusal> {
    SubjectSystem::state_key(system, state)
        .map(|(_, digest)| digest)
        .map_err(|_| fault("replayed-state-encodes"))
}

/// ADR-018 DL-5 and ADR-027 PX-2: the last replayed state enables no
/// transition identity and no declaration intends it, and its blocked
/// threads recompute equal to the payload's (none for a model subject).
fn deadlock<S: SubjectSystem>(
    system: &S,
    payload: &TemporalCounterexample<S::TransitionId>,
    positions: &[Position<S::State>],
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    let last = positions.last().ok_or_else(|| fault("a-position-exists"))?;
    let expansion = match expand(system, &last.state)? {
        Ok(expansion) => expansion,
        Err(_) => return Ok(no_value()),
    };
    let class = match SubjectSystem::terminal_class(
        system,
        &last.state,
        !expansion.successors.is_empty(),
    ) {
        Ok(class) => class,
        Err(stop) if stop.is_internal_failure() => return Err(fault("terminal-predicate")),
        Err(_) => return Ok(no_value()),
    };
    if class == TerminalClass::Deadlocked {
        let blocked = match system.blocked(&last.state) {
            Ok(blocked) => blocked,
            Err(stop) if stop.is_internal_failure() => return Err(fault("blocked-threads")),
            Err(_) => return Ok(no_value()),
        };
        if blocked != payload.blocked {
            return Ok(disagrees(Category::Violation));
        }
    }
    Ok(match class {
        TerminalClass::Deadlocked => ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position: payload.trace_position(),
        },
        TerminalClass::Undefined { model, cause } => ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Undefined(UndefinedEvaluation {
                site: UndefinedSite {
                    position: position_of(positions.len() - 1),
                    state_key: last.digest,
                    locus: UndefinedLocus::Terminal(model),
                    transition: None,
                },
                cause,
            }),
            trace_position: payload.trace_position(),
        },
        TerminalClass::NotTerminal | TerminalClass::Intended => disagrees(Category::Success),
        TerminalClass::Undecided { .. } => no_value(),
    })
}

fn position_of(index: usize) -> u64 {
    u64::try_from(index).unwrap_or(u64::MAX)
}

/// ADR-018 UE-5 and UE-6 for the deadlock-freedom item: the first replayed
/// position at which a contract conjunction of the expansion, or the
/// `terminal when` predicate at a terminal state, is undefined must be the
/// payload's `where`, at its locus, with its cause.
fn undefined<S: SubjectSystem>(
    system: &S,
    constraints: &dyn Constraints<S::State>,
    payload: &TemporalCounterexample<S::TransitionId>,
    positions: &[Position<S::State>],
    claimed: &UndefinedEvaluation,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    // ADR-021 SC-2, EI-7: a counterexample of an undefined state constraint
    // names it, and replay re-evaluates it at each position, where the
    // engine reads it: when the state is reached, before it is expanded.
    let constraint = match &claimed.site.locus {
        UndefinedLocus::Constraint(name) => Some(name.as_str()),
        UndefinedLocus::Atom(_)
        | UndefinedLocus::Contract(_)
        | UndefinedLocus::Terminal(_)
        | UndefinedLocus::Protocol(_)
        | UndefinedLocus::Mapping { .. } => None,
    };
    if constraint.is_some_and(|name| !constraints.has_constraint(name)) {
        return Ok(disagrees(Category::Success));
    }
    for (index, position) in positions.iter().enumerate() {
        let site = |locus, transition| UndefinedSite {
            position: position_of(index),
            state_key: position.digest,
            locus,
            transition,
        };
        if let Some(name) = constraint {
            match constraints.constraint_value(&position.state, name) {
                Ok(ConstraintValue::Undefined { cause, .. }) => {
                    let found = UndefinedEvaluation {
                        site: site(UndefinedLocus::Constraint(name.to_owned()), None),
                        cause,
                    };
                    return Ok(if found == *claimed {
                        ModelTraceSettlement::Reproduced {
                            value: ModelTraceValue::Undefined(found),
                            trace_position: payload.trace_position(),
                        }
                    } else {
                        disagrees(Category::Violation)
                    });
                }
                Ok(ConstraintValue::Holds | ConstraintValue::Boundary) => {}
                Err(stop) if stop.is_internal_failure() => return Err(fault("state-constraint")),
                Err(_) => return Ok(no_value()),
            }
        }
        let expansion = match expand(system, &position.state)? {
            Ok(expansion) => expansion,
            Err(_) => return Ok(no_value()),
        };
        let found = if let Some((locus, guard_of, cause)) = undefined_finding(&expansion.findings) {
            Some(UndefinedEvaluation {
                site: site(locus, guard_of),
                cause,
            })
        } else {
            match SubjectSystem::terminal_class(
                system,
                &position.state,
                !expansion.successors.is_empty(),
            ) {
                Ok(TerminalClass::Undefined { model, cause }) => Some(UndefinedEvaluation {
                    site: site(UndefinedLocus::Terminal(model), None),
                    cause,
                }),
                Ok(_) => None,
                Err(stop) if stop.is_internal_failure() => return Err(fault("terminal-predicate")),
                Err(_) => return Ok(no_value()),
            }
        };
        if let Some(found) = found {
            return Ok(if found == *claimed {
                ModelTraceSettlement::Reproduced {
                    value: ModelTraceValue::Undefined(found),
                    trace_position: payload.trace_position(),
                }
            } else {
                disagrees(Category::Violation)
            });
        }
    }
    Ok(disagrees(Category::Success))
}

/// FR-158's state constraints of a replayed subject (ADR-021 SC-2).
trait Constraints<State> {
    fn has_constraint(&self, name: &str) -> bool;

    fn constraint_value(&self, state: &State, name: &str)
        -> Result<ConstraintValue, ExpansionStop>;
}

impl Constraints<ModelSimState> for ModelSystem<'_> {
    fn has_constraint(&self, name: &str) -> bool {
        ModelSystem::has_constraint(self, name)
    }

    fn constraint_value(
        &self,
        state: &ModelSimState,
        name: &str,
    ) -> Result<ConstraintValue, ExpansionStop> {
        ModelSystem::constraint_value(self, state, name)
    }
}

/// A protocol subject's run takes no state constraint.
struct NoConstraints;

impl<State> Constraints<State> for NoConstraints {
    fn has_constraint(&self, _name: &str) -> bool {
        false
    }

    fn constraint_value(
        &self,
        _state: &State,
        _name: &str,
    ) -> Result<ConstraintValue, ExpansionStop> {
        Ok(ConstraintValue::Holds)
    }
}
