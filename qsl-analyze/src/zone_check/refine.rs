// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-430 "Timed refinement": the safety half of a refinement whose
//! concrete model is timed, decided by EN-6 on the symbolic product whose
//! states are (FR-142's discrete product state, zone).
//!
//! A product state is a model state of the concrete subject, its refinement
//! position (FR-142: the history values and the abstract candidates) and a
//! zone over the concrete subject's clock places. Its successors are
//! FR-239's symbolic successors of the model state and zone, each taken by
//! a discrete step; [`check_step`] runs on each such edge, in FR-239's
//! canonical breadth-first order, and gives the edge's target its
//! position. A delay maps to no abstract step: it changes the zone only, so
//! no `check_step` is run for it and the position is unchanged across it.
//! The rows read no clock (S3 refuses one), so the abstract state is a
//! function of the discrete state alone.
//!
//! A failing edge refutes only when its pre-state has a valuation with a
//! time-divergent continuation (ADR-026 TV-1, FR-240's test); an edge
//! failing only on time-locked valuations is no behaviour and is dropped.
//! The counterexample is the breadth-first tree path to the pre-state, then
//! the failing step, concretised with exact delays (FR-241).
//!
//! A product state is stored unless a stored one with the same model state
//! and position covers its zone under the aLU abstraction, as FR-239's
//! search stores its symbolic states: the failure of an edge reads its
//! discrete ends and position only, which the abstraction preserves.
//!
//! No core checker reads a zone product's simulation relation (FR-149
//! reads explicit concrete successors, which ignore clock guards), so a
//! half that holds carries no certificate (spec gap STD-493).

use std::collections::{BTreeMap, VecDeque};

use qsl_eval::simulation::model::{
    check_model_bounds, GuardedStep, ModelSimState, ModelSimulationRequest, TimedSubjectRefusal,
    TimedSystem,
};
use qsl_eval::simulation::BOUND_POSITIONS;
use qsl_package::CheckedPackage;
use qsl_replay::{
    check_initial, check_step, open_systems, ConcreteStep, CounterexampleKind, ReachedLimit,
    RefinementFailure, RefinementPosition, RefinementSystems, StepVerdict, SubjectRefusal,
    TemporalCounterexample, UndefinedEvaluation, Undetermined,
};
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::Cancel;
use quire_semantic_value::catalog::InternalFault;

use super::concretise::{concretise, Stage, Target};
use super::search::{
    not_simulated, Arrival, Halt, Judged, Run, Searched, ZoneCheckLimits, ZoneCheckRefusal,
    ZoneStats, ZoneStop, ZoneUndecided,
};
use super::Dbm;

/// A request to decide a timed refinement's safety half.
pub struct TimedRefinementRequest<'a> {
    /// The checked refinement, whose concrete model is timed.
    pub refinement: &'a CheckedRefinement,
    /// FR-120's request of the concrete subject.
    pub concrete: ModelSimulationRequest<'a>,
    /// The package holding the abstract model.
    pub abstract_package: &'a CheckedPackage,
    /// The abstract model's initial states.
    pub abstract_initial: Vec<DocumentRef>,
    /// EN-6's budgets.
    pub limits: ZoneCheckLimits,
}

/// Why the safety half decided nothing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimedRefinementUndecided {
    /// The zone search decided nothing.
    Zone(ZoneUndecided),
    /// A mapping row or step-row argument was refused or incomplete, or a
    /// contract conjunction of either model was undecided (FR-142).
    Undetermined(Undetermined),
}

/// Why the safety half stopped with no verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimedRefinementStop {
    /// An EN-6 budget or the caller's cancellation.
    Zone(ZoneStop),
    /// A run budget of `check_initial` or `check_step` (FR-142).
    Step(ReachedLimit),
}

/// The safety half of a timed refinement (QSpec FR-430, FR-142 Outputs).
#[derive(Clone, Debug)]
pub enum TimedSafetyOutcome {
    /// Every reachable product state was examined and every initial state
    /// and every discrete edge with a time-divergent pre-state passed. No
    /// core checker reads a zone product, so it carries no certificate.
    Holds {
        /// The run's counts.
        stats: ZoneStats,
    },
    /// The first failing initial state or admitted edge in breadth-first
    /// order, its counterexample a finite timed prefix with exact delays
    /// (FR-241), its `refinement` member set.
    Violated {
        /// The counterexample.
        counterexample: Box<TemporalCounterexample>,
        /// The run's counts.
        stats: ZoneStats,
    },
    /// The run decided nothing.
    Undecided {
        /// Why.
        cause: TimedRefinementUndecided,
        /// The run's counts.
        stats: ZoneStats,
    },
    /// The run stopped.
    Stopped {
        /// Why.
        cause: TimedRefinementStop,
        /// The run's counts.
        stats: ZoneStats,
    },
    /// An internal fault of the evaluator; the item settles `failed`
    /// (QSpec FR-383).
    Fault {
        /// The fault.
        fault: InternalFault,
        /// The run's counts.
        stats: ZoneStats,
    },
}

/// Why EN-6 does not take a timed refinement.
#[derive(Debug, thiserror::Error)]
pub enum TimedRefinementRefusal {
    /// The refinement's subjects have no systems (FR-142).
    #[error("the refinement's subjects do not open: {0:?}")]
    Subject(SubjectRefusal),
    /// The concrete subject is no timed subject (FR-231).
    #[error(transparent)]
    Timed(TimedSubjectRefusal),
    /// The zone engine refuses the subject.
    #[error(transparent)]
    Zone(ZoneCheckRefusal),
}

impl From<ZoneCheckRefusal> for TimedRefinementRefusal {
    fn from(refusal: ZoneCheckRefusal) -> Self {
        Self::Zone(refusal)
    }
}

/// QSpec FR-430: decide `request`'s safety half on EN-6's symbolic product.
///
/// # Errors
///
/// [`TimedRefinementRefusal`] when the subjects do not open, the concrete
/// subject is not timed or has an unbounded root, or the engine breaks an
/// invariant.
pub fn check_timed_refinement(
    request: TimedRefinementRequest<'_>,
    cancel: &Cancel,
) -> Result<TimedSafetyOutcome, TimedRefinementRefusal> {
    let refinement = request.refinement;
    let RefinementSystems {
        concrete,
        abstract_system,
    } = open_systems(
        refinement,
        &request.concrete,
        request.abstract_package,
        request.abstract_initial.clone(),
    )
    .map_err(TimedRefinementRefusal::Subject)?;
    check_model_bounds(&concrete, BOUND_POSITIONS).map_err(not_simulated)?;
    check_model_bounds(&abstract_system, BOUND_POSITIONS).map_err(not_simulated)?;
    let timed = TimedSystem::new(&concrete, refinement.concrete_model())
        .map_err(TimedRefinementRefusal::Timed)?;
    if timed.writes_collections() {
        return Err(ZoneCheckRefusal::CarriedClocks.into());
    }
    let mut run = Run::new(
        &timed,
        Judged::Refinement,
        None,
        &[],
        request.limits,
        cancel,
    );
    let mut product = Product {
        refinement,
        abstract_system: &abstract_system,
        positions: Vec::new(),
        known: BTreeMap::new(),
        depth: Vec::new(),
    };
    match product.search(&mut run) {
        Ok(outcome) => Ok(outcome),
        Err(Halt::Refusal(refusal)) => Err(refusal.into()),
        Err(Halt::Outcome(searched)) => Ok(match *searched {
            Searched::Stopped { cause, stats } => TimedSafetyOutcome::Stopped {
                cause: TimedRefinementStop::Zone(cause),
                stats,
            },
            Searched::Undecided { cause, stats } => TimedSafetyOutcome::Undecided {
                cause: TimedRefinementUndecided::Zone(cause),
                stats,
            },
            Searched::Holds { .. } | Searched::HoldsReduced { .. } | Searched::Violated { .. } => {
                return Err(ZoneCheckRefusal::Fault("refinement-halt").into())
            }
        }),
    }
}

/// The key of a refinement position: the concrete state's key, the history
/// values' key and the abstract candidates' state keys.
type PositionKey = (Vec<u8>, Vec<u8>, Vec<Vec<u8>>);

/// FR-142's discrete product over EN-6's run: the positions the run's
/// nodes carry as their `q`.
struct Product<'p, 'a> {
    refinement: &'p CheckedRefinement,
    abstract_system: &'p qsl_eval::simulation::model::ModelSystem<'a>,
    /// Each position, by the index a node's `q` names.
    positions: Vec<RefinementPosition>,
    known: BTreeMap<PositionKey, usize>,
    /// The number of discrete steps from its initial node to each node.
    depth: Vec<usize>,
}

/// What a verdict decides.
enum Judgement {
    /// The step passes, to the position at this index.
    Passes(usize),
    /// The step or initial state fails: the failure and the counterexample
    /// kind.
    Fails(RefinementFailure, CounterexampleKind),
    /// The run ends with this outcome.
    Ends(TimedSafetyOutcome),
}

impl Product<'_, '_> {
    fn search(&mut self, run: &mut Run<'_, '_, '_>) -> Result<TimedSafetyOutcome, Halt> {
        let concrete = run.timed.system();
        let initial = run.timed.initial();
        let mut queue = VecDeque::new();
        for (index, state) in initial.iter().enumerate() {
            run.poll()?;
            let discrete = state.discrete().clone();
            let verdict = check_initial(
                self.refinement,
                concrete,
                self.abstract_system,
                index,
                discrete.state(),
            );
            let q = match self.judge(verdict, run)? {
                Judgement::Passes(q) => q,
                // An initial state is no edge: its failure refutes as
                // FR-142's does, at position 0.
                Judgement::Fails(failure, kind) => {
                    return Ok(TimedSafetyOutcome::Violated {
                        counterexample: Box::new(refuted(index, Vec::new(), failure, kind)),
                        stats: run.stats(),
                    })
                }
                Judgement::Ends(outcome) => return Ok(outcome),
            };
            let mut zone = Dbm::zero(run.dim());
            run.settle(&discrete, &mut zone)?;
            if zone.is_empty() {
                continue;
            }
            if let Some(node) = run.insert(discrete, q, zone, Some(index), None)? {
                self.depth.push(0);
                queue.push_back(node);
            }
        }
        if queue.is_empty() {
            return Ok(TimedSafetyOutcome::Undecided {
                cause: TimedRefinementUndecided::Zone(ZoneUndecided::NoInitialState),
                stats: run.stats(),
            });
        }
        while let Some(node) = queue.pop_front() {
            run.poll()?;
            if let Some(outcome) = self.expand(run, node, &mut queue)? {
                return Ok(outcome);
            }
        }
        Ok(TimedSafetyOutcome::Holds { stats: run.stats() })
    }

    /// Expand `node`: each symbolic successor by a discrete step, judged by
    /// `check_step` once per step. The outcome when an admitted edge fails
    /// or the run ends.
    fn expand(
        &mut self,
        run: &mut Run<'_, '_, '_>,
        node: usize,
        queue: &mut VecDeque<usize>,
    ) -> Result<Option<TimedSafetyOutcome>, Halt> {
        let state = run.nodes[node].state.clone();
        let zone = run.nodes[node].zone.clone();
        let q = run.nodes[node].q;
        let depth = self.depth[node];
        let steps = run.steps_for(&state)?;
        let scale = run.scale.clone();
        // Each step's position after it, judged once; `None` for a step
        // whose failure the pre-state's time-locked valuations only reach.
        let mut judged: BTreeMap<usize, Option<usize>> = BTreeMap::new();
        for successor in run.successors(&state, &zone, &scale)? {
            let next = match judged.get(&successor.step) {
                Some(next) => *next,
                None => {
                    let step = steps
                        .get(successor.step)
                        .ok_or(ZoneCheckRefusal::Fault("refinement-step"))?;
                    let result = check_step(
                        self.refinement,
                        run.timed.system(),
                        self.abstract_system,
                        &self.positions[q],
                        &ConcreteStep {
                            position: widen(depth.saturating_add(1)),
                            transition: Some(step.transition()),
                            post: step.post(),
                        },
                    );
                    let next = match self.judge(result.verdict, run)? {
                        Judgement::Passes(next) => Some(next),
                        Judgement::Ends(outcome) => return Ok(Some(outcome)),
                        Judgement::Fails(failure, kind) => {
                            // ADR-026 TV-1: only a pre-state with a
                            // time-divergent valuation refutes.
                            if run.node_escape(node)?.diverges {
                                let counterexample = self.counterexample(
                                    run,
                                    node,
                                    (step, successor.part),
                                    failure,
                                    kind,
                                )?;
                                return Ok(Some(TimedSafetyOutcome::Violated {
                                    counterexample: Box::new(counterexample),
                                    stats: run.stats(),
                                }));
                            }
                            None
                        }
                    };
                    judged.insert(successor.step, next);
                    next
                }
            };
            let Some(next) = next else {
                continue;
            };
            let arrival = Arrival {
                from: node,
                step: Some((successor.step, successor.part)),
                piece: None,
            };
            if let Some(created) =
                run.insert(successor.state, next, successor.zone, None, Some(arrival))?
            {
                self.depth.push(depth.saturating_add(1));
                queue.push_back(created);
            }
        }
        Ok(None)
    }

    /// What `verdict` decides; a passing one's position interned.
    fn judge(&mut self, verdict: StepVerdict, run: &Run<'_, '_, '_>) -> Result<Judgement, Halt> {
        Ok(match verdict {
            StepVerdict::Passes { post, .. } => Judgement::Passes(self.intern(post, run)?),
            StepVerdict::Fails(failure) => Judgement::Fails(failure, CounterexampleKind::Formula),
            StepVerdict::Undefined(evaluation) => {
                let qsl_replay::UndefinedLocus::Mapping(site) = &evaluation.site.locus else {
                    return Err(
                        ZoneCheckRefusal::Fault("an-undefined-mapping-names-its-row").into(),
                    );
                };
                let failure = RefinementFailure::Undefined {
                    position: evaluation.site.position,
                    row: site.row.clone(),
                };
                Judgement::Fails(
                    failure,
                    CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                        site: evaluation.site,
                        cause: evaluation.cause,
                    }),
                )
            }
            StepVerdict::Undetermined(undetermined) => {
                Judgement::Ends(TimedSafetyOutcome::Undecided {
                    cause: TimedRefinementUndecided::Undetermined(undetermined),
                    stats: run.stats(),
                })
            }
            StepVerdict::Stopped(reached) => Judgement::Ends(TimedSafetyOutcome::Stopped {
                cause: TimedRefinementStop::Step(reached),
                stats: run.stats(),
            }),
            StepVerdict::Fault(fault) => Judgement::Ends(TimedSafetyOutcome::Fault {
                fault,
                stats: run.stats(),
            }),
        })
    }

    /// The index of `position`: the stored one with its key, else a new
    /// one.
    fn intern(
        &mut self,
        position: RefinementPosition,
        run: &Run<'_, '_, '_>,
    ) -> Result<usize, Halt> {
        let concrete = run.timed.system();
        let encodes = |_| Halt::from(ZoneCheckRefusal::Fault("position-key"));
        let (state, _) = concrete.state_key_of(&position.concrete).map_err(encodes)?;
        let history = position.history.key(concrete).map_err(encodes)?;
        let mut candidates = Vec::with_capacity(position.abstract_states.len());
        for candidate in &position.abstract_states {
            candidates.push(
                self.abstract_system
                    .state_key_of(candidate)
                    .map_err(encodes)?
                    .0,
            );
        }
        let key = (state, history, candidates);
        if let Some(&at) = self.known.get(&key) {
            return Ok(at);
        }
        let at = self.positions.len();
        self.positions.push(position);
        self.known.insert(key, at);
        Ok(at)
    }

    /// FR-241: the counterexample through the real edges to `node`, then
    /// the failing `step` by its guard part.
    fn counterexample(
        &self,
        run: &mut Run<'_, '_, '_>,
        node: usize,
        (step, part): (&GuardedStep, usize),
        failure: RefinementFailure,
        kind: CounterexampleKind,
    ) -> Result<TemporalCounterexample, Halt> {
        let lifted = run.lifted_to(node)?;
        let mut stages = lifted.stages;
        let state: &ModelSimState = &lifted.state;
        stages.push(Stage {
            invariants: run.timed(run.timed.invariants(state))?,
            elapses: run.elapses(state)?,
            target: run.timed(run.timed.invariants(step.post()))?,
            step: Some((step.clone(), part)),
            position: Vec::new(),
            position_resets: Vec::new(),
            before: None,
            piece: None,
            silent: false,
            fixed: None,
        });
        let concrete = concretise(run.timed, run.dim() - 1, &stages, &Target::Any, &run.scale)
            .map_err(|_| ZoneCheckRefusal::Fault("concretisation"))?;
        Ok(refuted(lifted.initial, concrete.prefix, failure, kind))
    }
}

/// The counterexample from initial state `initial` along `prefix` to a
/// refinement failure.
fn refuted(
    initial: usize,
    prefix: Vec<qsl_replay::ModelStep>,
    failure: RefinementFailure,
    kind: CounterexampleKind,
) -> TemporalCounterexample {
    TemporalCounterexample {
        initial_state: initial,
        prefix,
        over_binding: None,
        fairness: Vec::new(),
        kind,
        lasso: None,
        refinement: Some(failure),
        blocked: Vec::new(),
        final_delay: None,
    }
}

fn widen(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
