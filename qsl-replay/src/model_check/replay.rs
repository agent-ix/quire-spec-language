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
//!   conjunction or a `terminal when` predicate is undefined is the
//!   payload's `where`, at its locus, with its cause.
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

use qsl_eval::fairness::{check_loop, Constraint, EdgeLabel, Identity, LassoFairness, LoopStep};
use qsl_eval::model_subject::{ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelFinding, ModelSimState, ModelSimulationRequest, ModelSystem,
    PopulationUniverse, TerminalClass,
};
use qsl_eval::simulation::{ExpansionStop, TransitionSystem};
use qsl_eval::temporal::AtomReading;
use qsl_eval::temporal::{
    evaluate_with_enabledness, LetterStop, TemporalLetters, TemporalOutcome, TemporalPosition,
    TemporalTrace,
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
    deadlock_freedom_clause, CounterexampleKind, Lasso, TemporalCounterexample,
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
    /// A bounded-profile prefix that neither ends at a terminal state nor
    /// covers the formula's horizon from the activation it refutes: read as
    /// a closed trace it would show a falsity its continuation may not
    /// (ADR-018 CX-1).
    #[error("invalid_runtime_input/invalid-value: the prefix of {positions} positions neither ends at a terminal state nor covers the horizon {horizon} from the activation at {at}")]
    PrefixShort {
        /// The activation the evaluator found false.
        at: u64,
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
    /// The recompiled clause's fairness set holds a strong constraint,
    /// whose lasso check (ADR-019 FS-8) is not built.
    #[error("unsupported_construct/not-yet-implemented: the clause's fairness set holds a strong constraint")]
    StrongFairness,
    /// A step's transition identity is not enabled at its pre-state.
    #[error("invalid_runtime_input/invalid-value: step {step} is not enabled")]
    NotEnabled {
        /// The step's zero-based index in the prefix.
        step: usize,
        /// Its transition identity.
        transition: Box<ModelTransition>,
    },
    /// No successor of a step's transition identity has the recorded
    /// post-state digest (FR-101's key mismatch).
    #[error("stale_dependency/content-mismatch: step {step} reaches no state {}", .recorded.hex())]
    PostState {
        /// The step's zero-based index in the prefix.
        step: usize,
        /// The recorded post-state digest.
        recorded: DigestRecord,
    },
}

impl ModelTraceRefusal {
    /// The refusal's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::UnknownTemporalClause { .. }
            | Self::ItemIdentity { .. }
            | Self::PostState { .. } => Code::StaleDependency,
            Self::FairnessSet { .. } => Code::StaleDependency,
            Self::StrongFairness | Self::AuthoredUndefined | Self::GuardedActivation => {
                Code::UnsupportedConstruct
            }
            Self::InitialState { .. }
            | Self::NotEnabled { .. }
            | Self::OverBinding
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
            Self::StrongFairness | Self::AuthoredUndefined | Self::GuardedActivation => {
                "not-yet-implemented"
            }
            Self::InitialState { .. }
            | Self::NotEnabled { .. }
            | Self::OverBinding
            | Self::PrefixShort { .. }
            | Self::LoopOpen { .. }
            | Self::StutterNotTerminal { .. } => "invalid-value",
        }
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

/// The result of [`replay_model_trace`]: the settlement and the recompiled
/// `package_id` it was settled against.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelTraceReplayResult {
    /// The settlement.
    pub settlement: ModelTraceSettlement,
    /// The recompiled `package_id`.
    pub package_id: PackageId,
}

/// A model counterexample refutes its item: the proved verdict is
/// `violation`.
fn disagrees(replayed: Category) -> ModelTraceSettlement {
    ModelTraceSettlement::Inconclusive(DisagreementCause::Verdicts {
        proved: Verdict::from_category(Category::Violation),
        replayed: Verdict::from_category(replayed),
    })
}

fn no_value() -> ModelTraceSettlement {
    ModelTraceSettlement::Inconclusive(DisagreementCause::NoValue {
        proved: Verdict::from_category(Category::Violation),
        replayed: Verdict::from_category(Category::Incomplete),
    })
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
    let payload = envelope.family_payload();
    let clause = resolve_item(envelope, compiled.checked.package(), &package_id.record())?;
    let documents: BTreeMap<[u8; 32], Vec<u8>> = request
        .byte_provision()
        .entries()
        .filter(|(digest, _)| digest.domain() == DigestDomain::Sha256Jcs)
        .map(|(digest, bytes)| (*digest.as_bytes(), bytes.to_vec()))
        .collect();
    let system = ModelSystem::new(ModelSimulationRequest {
        package: compiled.checked.package(),
        packages: &compiled.packages,
        normalization: limits.spine.model,
        snapshots: &documents,
        initial: subject.initial.clone(),
        universes: subject.universes.clone(),
        observation: limits.observation,
        expansion: subject.expansion,
        meter: request.accounting_limits(),
    })
    .map_err(ReplayRefusal::Admission)?;
    let formula = clause.map(|clause| FormulaReplay {
        clause,
        letters,
        meter: request.accounting_limits(),
    });
    let settlement = settle(&system, payload, formula)?;
    Ok(ModelTraceReplayResult {
        settlement,
        package_id,
    })
}

fn refused(refusal: ModelTraceRefusal) -> ReplayRefusal {
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
fn resolve_item<'p>(
    envelope: &WitnessEnvelope<TemporalCounterexample>,
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
fn names_deadlock_item(payload: &TemporalCounterexample) -> bool {
    match &payload.kind {
        CounterexampleKind::Deadlock => true,
        CounterexampleKind::UndefinedEvaluation(evaluation) => {
            payload.over_binding.is_none()
                && !matches!(evaluation.site.locus, UndefinedLocus::Atom(_))
        }
        CounterexampleKind::Formula => false,
    }
}

fn fault(invariant: &'static str) -> ReplayRefusal {
    ReplayRefusal::Fault(InternalFault::new("replay-model-trace", invariant))
}

/// One replayed position: its state and digest.
struct Position {
    state: ModelSimState,
    digest: DigestRecord,
}

/// The successors of `state`, or the stop an expansion returned.
fn expand(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
) -> Result<
    Result<
        qsl_eval::simulation::Expansion<ModelTransition, ModelSimState, ModelFinding>,
        ExpansionStop,
    >,
    ReplayRefusal,
> {
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

/// Re-execute the payload's prefix and check what it claims.
fn settle(
    system: &ModelSystem<'_>,
    payload: &TemporalCounterexample,
    formula: Option<FormulaReplay<'_>>,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
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
            Err(_) => return Ok(no_value()),
        };
        let mut enabled = false;
        let mut reached = None;
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
        }
        if !enabled {
            return Err(refused(ModelTraceRefusal::NotEnabled {
                step: index,
                transition: Box::new(step.transition.clone()),
            }));
        }
        let next = reached.ok_or_else(|| {
            refused(ModelTraceRefusal::PostState {
                step: index,
                recorded: step.post_state,
            })
        })?;
        positions.push(next);
    }
    if let Some(lasso) = payload.lasso {
        close_loop(system, &positions, lasso)?;
    }
    match (&payload.kind, formula) {
        (CounterexampleKind::Deadlock, _) => deadlock(system, payload, &positions),
        (CounterexampleKind::UndefinedEvaluation(claimed), _) => {
            undefined(system, payload, &positions, claimed)
        }
        (CounterexampleKind::Formula, Some(formula)) => {
            check_formula(system, payload, &positions, &formula)
        }
        (CounterexampleKind::Formula, None) => Err(fault("formula-counterexample-names-a-clause")),
    }
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
struct ReplayedLetters<'a> {
    letters: &'a dyn ModelLetters,
    instance: &'a ModelInstance,
    positions: &'a [Position],
}

impl TemporalLetters for ReplayedLetters<'_> {
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
        Ok(self
            .letters
            .atom(atom, self.instance, &state.state, meter)?)
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
/// reproduces; `true`, or no value, settles `inconclusive`.
fn check_formula(
    system: &ModelSystem<'_>,
    payload: &TemporalCounterexample,
    positions: &[Position],
    formula: &FormulaReplay<'_>,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    let instance = payload
        .over_binding
        .as_ref()
        .ok_or_else(|| refused(ModelTraceRefusal::OverBinding))?;
    let mut letters = ReplayedLetters {
        letters: formula.letters,
        instance,
        positions,
    };
    let mut meter = Meter::new(formula.meter);
    let trace = match payload.lasso {
        Some(lasso) => {
            match loop_fairness(system, payload, positions, lasso, formula.clause)? {
                LoopFairness::Fair => {}
                LoopFairness::Unfair(constraint) => {
                    return Err(ReplayRefusal::UnfairLasso { constraint })
                }
                LoopFairness::Stopped => return Ok(no_value()),
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
    let outcome = evaluate_with_enabledness(formula.clause, trace, &mut letters, &mut meter);
    Ok(match outcome {
        // A lasso is evaluated from its origin (ADR-018 section 6): a clause
        // false only at a later activation is not the refutation EN-1 found.
        Ok(TemporalOutcome::Violated { at }) if payload.lasso.is_some() && at.index() != 0 => {
            disagrees(Category::Success)
        }
        Ok(TemporalOutcome::Violated { at }) => {
            if payload.lasso.is_none() {
                short_prefix(system, formula.clause, positions, at)?;
            }
            ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Truth(false),
                trace_position: payload.trace_position(),
            }
        }
        Ok(TemporalOutcome::Holds) => disagrees(Category::Success),
        Ok(_) => no_value(),
        Err(qsl_eval::temporal::TemporalRefusal::Fault(_)) => {
            return Err(fault("temporal-evaluation"))
        }
        Err(_) => no_value(),
    })
}

/// ADR-018 CX-1: under a bounded profile a finite prefix reads as a closed
/// trace, so a falsity at the activation `at` is shown only when the prefix
/// holds the formula's horizon from `at` or ends at a terminal state (SM-4).
/// Otherwise the continuation, which the prefix does not carry, may satisfy
/// the formula, and the prefix refutes nothing.
fn short_prefix(
    system: &ModelSystem<'_>,
    clause: &CheckedTemporalClause,
    positions: &[Position],
    at: TemporalPosition,
) -> Result<(), ReplayRefusal> {
    if !clause.profile().is_bounded() {
        return Ok(());
    }
    let horizon = clause.horizon().unwrap_or(0);
    let last = position_of(positions.len().saturating_sub(1));
    if at.index().saturating_add(horizon) <= last {
        return Ok(());
    }
    let ends_terminal = match positions.last() {
        Some(position) => matches!(
            expand(system, &position.state)?,
            Ok(expansion) if expansion.successors.is_empty()
        ),
        None => false,
    };
    if ends_terminal {
        return Ok(());
    }
    Err(refused(ModelTraceRefusal::PrefixShort {
        at: at.index(),
        horizon,
        positions: positions.len(),
    }))
}

/// Whether a lasso's loop is fair under the clause's fairness set.
enum LoopFairness {
    /// Fair under every constraint.
    Fair,
    /// Unfair under this constraint of the clause's fairness set.
    Unfair(FairnessConstraint),
    /// An expansion stopped while enabledness was read.
    Stopped,
}

/// ADR-018 FA-3: the first constraint of the clause's fairness set that the
/// lasso's loop is unfair under, enabledness read at every loop state from
/// the model's own successor relation (FA-2). The terminal stutter step is
/// outside every constraint and enables none (SM-4). A `each` constraint is
/// one constraint per transition identity of its operation that the loop
/// takes or enables; a `whole` constraint is one over all of them.
fn loop_fairness(
    system: &ModelSystem<'_>,
    payload: &TemporalCounterexample,
    positions: &[Position],
    lasso: Lasso,
    clause: &CheckedTemporalClause,
) -> Result<LoopFairness, ReplayRefusal> {
    if clause
        .fairness()
        .iter()
        .any(|constraint| constraint.kind() == FairnessKind::Strong)
    {
        return Err(refused(ModelTraceRefusal::StrongFairness));
    }
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
    } else {
        for (position, step) in positions.iter().zip(&payload.prefix).skip(lasso.entry) {
            let Ok(expansion) = expand(system, &position.state)? else {
                return Ok(LoopFairness::Stopped);
            };
            let enabled: Vec<Identity> = expansion
                .successors
                .iter()
                .map(|(transition, _)| number(transition))
                .collect();
            steps.push(LoopStep::new(
                enabled,
                EdgeLabel::Transition(number(&step.transition)),
            ));
        }
    }
    // The constraints, each remembering the authored constraint it came from.
    let mut constraints = Vec::new();
    let mut authored = Vec::new();
    for constraint in clause.fairness() {
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
                constraints.push(Constraint::weak(covered));
                authored.push(constraint);
            }
            FairnessGranularity::Each => {
                for identity in covered {
                    constraints.push(Constraint::weak([identity]));
                    authored.push(constraint);
                }
            }
        }
    }
    match check_loop(&steps, &constraints) {
        Ok(LassoFairness::Fair) => Ok(LoopFairness::Fair),
        Ok(LassoFairness::Unfair { constraint }) => authored
            .get(constraint)
            .map(|constraint| LoopFairness::Unfair((*constraint).clone()))
            .ok_or_else(|| fault("an-unfair-constraint-is-one-of-the-set")),
        Err(_) => Err(fault("a-lasso-loop-has-a-step")),
    }
}

fn digest_of(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
) -> Result<DigestRecord, ReplayRefusal> {
    system
        .state_key(state)
        .map(|(_, digest)| digest)
        .map_err(|_| fault("replayed-state-encodes"))
}

/// ADR-018 DL-5: the last replayed state enables no transition identity and
/// no declaration intends it.
fn deadlock(
    system: &ModelSystem<'_>,
    payload: &TemporalCounterexample,
    positions: &[Position],
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    let last = positions.last().ok_or_else(|| fault("a-position-exists"))?;
    let expansion = match expand(system, &last.state)? {
        Ok(expansion) => expansion,
        Err(_) => return Ok(no_value()),
    };
    let class = match system.terminal_class(&last.state, !expansion.successors.is_empty()) {
        Ok(class) => class,
        Err(stop) if stop.is_internal_failure() => return Err(fault("terminal-predicate")),
        Err(_) => return Ok(no_value()),
    };
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
fn undefined(
    system: &ModelSystem<'_>,
    payload: &TemporalCounterexample,
    positions: &[Position],
    claimed: &UndefinedEvaluation,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    for (index, position) in positions.iter().enumerate() {
        let expansion = match expand(system, &position.state)? {
            Ok(expansion) => expansion,
            Err(_) => return Ok(no_value()),
        };
        let site = |locus, transition| UndefinedSite {
            position: position_of(index),
            state_key: position.digest,
            locus,
            transition,
        };
        let found = if let Some(contract) = expansion
            .findings
            .iter()
            .find_map(ModelFinding::undefined_contract)
        {
            Some(UndefinedEvaluation {
                site: site(UndefinedLocus::Contract(contract.clause), contract.guard_of),
                cause: contract.cause,
            })
        } else {
            match system.terminal_class(&position.state, !expansion.successors.is_empty()) {
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
