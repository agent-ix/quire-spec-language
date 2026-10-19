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
//! - a deadlock counterexample (ADR-018 DL-5): no transition identity is
//!   enabled at the last state and no `terminal` declaration intends it;
//! - an undefined-evaluation counterexample of the deadlock-freedom item
//!   (UE-5, UE-6): the first replayed position at which a contract
//!   conjunction or a `terminal when` predicate is undefined is the
//!   payload's `where`, at its locus, with its cause.
//!
//! A counterexample of an authored clause names the clause by its node, and
//! no recompiled package holds a temporal clause node yet (FR-337), so its
//! replay refuses `stale_dependency`/`content-mismatch` before any
//! admission. Replay needs no model checker and no solver, and reads no
//! path, environment variable or clock.

use std::collections::BTreeMap;

use qsl_eval::simulation::model::{
    ExpansionLimits, ModelFinding, ModelSimState, ModelSimulationRequest, ModelSystem,
    PopulationUniverse, TerminalClass,
};
use qsl_eval::simulation::{ExpansionStop, TransitionSystem};
use qsl_foundation::diagnostic::{Category, Code, InternalFault};
use qsl_foundation::digest::{DigestDomain, DigestRecord, WireNodeId};
use qsl_semantics::library::PackageId;
use qsl_semantics::model::observation::DocumentRef;
use quire_semantic_value::model_transition::ModelTransition;

use crate::bounds::ReplayLimits;
use crate::execute::{recompile, request_limits, ReplayRefusal};
use crate::model_check::counterexample::{
    CounterexampleKind, TemporalCounterexample, UndefinedEvaluation, UndefinedLocus,
    UndefinedSite,
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
            Self::UnknownTemporalClause { .. } | Self::PostState { .. } => Code::StaleDependency,
            Self::InitialState { .. } | Self::NotEnabled { .. } => Code::InvalidRuntimeInput,
        }
    }

    /// The refusal's catalog cause.
    pub fn cause(&self) -> &'static str {
        match self {
            Self::UnknownTemporalClause { .. } | Self::PostState { .. } => "content-mismatch",
            Self::InitialState { .. } | Self::NotEnabled { .. } => "invalid-value",
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
/// digest) and the limits.
///
/// Refuses, with no partial result, when the request does not decode, by
/// FR-098's recompile rules, on a stale `package_id`, for a counterexample
/// of an authored clause, when the subject does not admit (FR-106's record),
/// on an `initial_state` outside the subject, on a step that is not
/// enabled, and on a post-state digest no successor has. An expansion that
/// stops while replaying completes no value and settles `NoValue`.
///
/// # Errors
///
/// [`ReplayRefusal`] as above.
pub fn replay_model_trace(
    wire: ReplayRequestWire,
    subject: &ModelTraceSubject,
    envelope: &WitnessEnvelope<TemporalCounterexample>,
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
    if !names_deadlock_item(payload) {
        return Err(refused(ModelTraceRefusal::UnknownTemporalClause {
            clause_node: envelope.clause_node(),
        }));
    }
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
    let settlement = settle(&system, payload)?;
    Ok(ModelTraceReplayResult {
        settlement,
        package_id,
    })
}

fn refused(refusal: ModelTraceRefusal) -> ReplayRefusal {
    ReplayRefusal::ModelTrace(Box::new(refusal))
}

/// Whether the payload refutes the deadlock-freedom item: a deadlock, or
/// an undefined model-side expression with no `over` binding, which only
/// the derived item lacks.
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
) -> Result<Result<qsl_eval::simulation::Expansion<ModelTransition, ModelSimState, ModelFinding>, ExpansionStop>, ReplayRefusal>
{
    match system.successors(state) {
        Ok(expansion) => Ok(Ok(expansion)),
        Err(stop) if stop.cause.code() == "runtime_invariant" => Err(fault("model-expansion")),
        Err(stop) => Ok(Err(stop)),
    }
}

/// Re-execute the payload's prefix and check what it claims.
fn settle(
    system: &ModelSystem<'_>,
    payload: &TemporalCounterexample,
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
    match &payload.kind {
        CounterexampleKind::Deadlock => deadlock(system, payload, &positions),
        CounterexampleKind::UndefinedEvaluation(claimed) => {
            undefined(system, payload, &positions, claimed)
        }
        CounterexampleKind::Formula => Err(fault("formula-counterexample-names-a-clause")),
    }
}

fn digest_of(system: &ModelSystem<'_>, state: &ModelSimState) -> Result<DigestRecord, ReplayRefusal> {
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
        Err(stop) if stop.cause.code() == "runtime_invariant" => {
            return Err(fault("terminal-predicate"))
        }
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
                Err(stop) if stop.cause.code() == "runtime_invariant" => {
                    return Err(fault("terminal-predicate"))
                }
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
