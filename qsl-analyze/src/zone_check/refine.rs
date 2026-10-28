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
//! A failing edge refutes only when the behaviour can go on from it with
//! time diverging: its post-state, the successor's state and zone, has a
//! valuation with a time-divergent continuation (ADR-026 TV-1 and TS-4,
//! FR-240's test). An edge failing only into time-locked valuations is no
//! behaviour and is dropped. The counterexample is the breadth-first tree
//! path to the pre-state, then the failing step, concretised with exact
//! delays (FR-241) to end at such a valuation.
//!
//! A product state is stored unless a stored one with the same model state
//! and position covers its zone under the aLU abstraction, as FR-239's
//! search stores its symbolic states: the failure of an edge reads its
//! discrete ends and position only, which the abstraction preserves.
//!
//! A failing initial state refutes only when its symbolic state has a
//! time-divergent point; a time-locked one holds vacuously, and a half
//! whose initial states admit no time-divergent behaviour settles ADR-026
//! TD-6's `NoAdmittedBehaviour`.
//!
//! A failing step whose post-state is time-locked is kept: its post-state
//! is the `locked` entry of a locked region, whose nodes carry no position
//! and whose concrete steps no `check_step` judges.
//!
//! A half that holds carries FR-244's refinement-form zone certificate:
//! the search's reachability part, each node's `q` an index into the
//! product's positions (FR-149's `CertifiedPosition`) or the locked
//! region's reserved index, and each edge's `Taken`. FR-245 checks it, and
//! the item settles `proved` only once it accepts. FR-149's simulation
//! certificate does not apply, since its explicit successors ignore clock
//! guards.
//!
//! This is the safety half only. Remaining work on QSL-1005: the liveness
//! half over a timed concrete subject (ADR-026 TS-4), and the route of a
//! refinement record whose concrete subject declares `time dense` to EN-6,
//! which waits on the refinement node identity (QSL-544, IR-1014).

use std::collections::{BTreeMap, VecDeque};

use qsl_eval::simulation::model::{
    check_model_bounds, GuardedStep, ModelSimState, ModelSimulationRequest, ModelSystem,
    TimedSubjectRefusal, TimedSystem,
};
use qsl_eval::simulation::BOUND_POSITIONS;
use qsl_package::CheckedPackage;
use qsl_replay::{
    check_initial, check_step, open_systems, CertifiedPosition, ConcreteStep, CounterexampleKind,
    ReachedLimit, RefinementFailure, RefinementPart, RefinementPosition, RefinementSystems,
    StepVerdict, SubjectRefusal, Taken, TemporalCounterexample, UnavailabilityCause,
    UndefinedEvaluation, Undetermined, ZoneCertificate,
};
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::Cancel;
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::fairness::FairnessKind;

use super::certificate::Proof;
use super::concretise::{concretise, Stage, Target};
use super::divergence::Escape;
use super::search::{
    not_simulated, widen, Arrival, Halt, Judged, Run, Searched, ZoneCheckLimits, ZoneCheckRefusal,
    ZoneReduction, ZoneStats, ZoneStop, ZoneUndecided,
};
use super::Dbm;

mod live;

/// A request to decide a timed refinement's safety half.
pub struct TimedRefinementRequest<'a, O> {
    /// The checked refinement, whose concrete model is timed.
    pub refinement: &'a CheckedRefinement,
    /// The refinement item's obligation identity, which its certificate
    /// names (FR-244).
    pub obligation: O,
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
pub enum TimedSafetyOutcome<O> {
    /// Every reachable product state was examined, every initial state and
    /// every discrete edge into a time-divergent post-state passed, and the
    /// subject admits a time-divergent behaviour. It carries FR-244's
    /// refinement-form certificate, which settles `proved` only once FR-245
    /// accepts it.
    Holds {
        /// The certificate.
        certificate: Box<ZoneCertificate<O>>,
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

/// The safety half before its certificate names the obligation: a `Holds`
/// names the initial node whose behaviour diverges.
enum Decided {
    Holds {
        root: usize,
    },
    Violated {
        counterexample: Box<TemporalCounterexample>,
        stats: ZoneStats,
    },
    Undecided {
        cause: TimedRefinementUndecided,
        stats: ZoneStats,
    },
    Stopped {
        cause: TimedRefinementStop,
        stats: ZoneStats,
    },
    Fault {
        fault: InternalFault,
        stats: ZoneStats,
    },
}

/// A timed refinement's outcome (QSpec FR-430; FR-142 and FR-143
/// Outputs).
#[derive(Clone, Debug)]
pub struct TimedRefinementOutcome<O> {
    /// The safety half.
    pub safety: TimedSafetyOutcome<O>,
    /// The liveness half: `None` with `F_A` empty. With a non-empty `F_A`
    /// it is `Unsupported` for a refinement with a hidden abstract field or
    /// a strong `F_A` constraint, whatever the safety half is; otherwise it
    /// is `None` when the safety half is other than `Holds`, and `Checked`
    /// when it holds.
    pub liveness: Option<TimedLivenessHalf>,
}

/// The liveness half of a timed refinement (FR-143 Outputs, ADR-020 RE-1).
#[derive(Clone, Debug)]
pub enum TimedLivenessHalf {
    /// The second phase ran on the symbolic product.
    Checked(TimedLivenessOutcome),
    /// The second phase did not run (ADR-020 AX-5).
    Unsupported(UnavailabilityCause),
}

/// The second phase's outcome over a timed concrete subject: only
/// time-divergent concrete behaviours are read (ADR-026 TS-4).
#[derive(Clone, Debug)]
pub enum TimedLivenessOutcome {
    /// Every time-divergent behaviour of the concrete subject that is fair
    /// under `F_C` satisfies `F_A`. Each variant's counts are the liveness
    /// product's own, not the safety half's. It has no certificate, since the core
    /// has no checker for it (ADR-020 CT-4), so its `proved` is
    /// `Uncertified`.
    HoldsReduced {
        /// The reductions the run applied: none.
        reductions: Vec<ZoneReduction>,
        /// The run's counts.
        stats: ZoneStats,
    },
    /// A timed lasso with positive loop delay, fair under `F_C`, on which a
    /// class of `F_A` is enabled throughout and never taken: its failure is
    /// `Divergence` or `AbstractUnfair` (FR-143, FR-241).
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
    /// An internal fault of the evaluator.
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

/// QSpec FR-430: decide `request`'s safety half on EN-6's symbolic product
/// and, when it holds and `F_A` is non-empty, its liveness half there.
///
/// # Errors
///
/// [`TimedRefinementRefusal`] when the subjects do not open, the concrete
/// subject is not timed or has an unbounded root, or the engine breaks an
/// invariant.
pub fn check_timed_refinement<O>(
    request: TimedRefinementRequest<'_, O>,
    cancel: &Cancel,
) -> Result<TimedRefinementOutcome<O>, TimedRefinementRefusal> {
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
        taken: BTreeMap::new(),
    };
    let decided = match product.search(&mut run) {
        Ok(decided) => decided,
        Err(halt) => {
            return Ok(TimedRefinementOutcome {
                safety: halted(halt)?,
                liveness: unsupported_liveness(refinement),
            })
        }
    };
    let holds = matches!(decided, Decided::Holds { .. });
    let safety = match decided {
        Decided::Holds { root } => match product.certificate(&mut run, root) {
            Ok(proof) => TimedSafetyOutcome::Holds {
                certificate: Box::new(ZoneCertificate {
                    refinement: Some(RefinementPart {
                        refinement: refinement.name().to_owned(),
                        concrete_package: refinement.concrete_model_selection().clone(),
                        abstract_package: refinement.abstract_model_selection().clone(),
                        positions: proof.positions,
                        taken: proof.taken,
                    }),
                    ..proof.proof.proving(request.obligation)
                }),
                stats: run.stats(),
            },
            Err(halt) => halted(halt)?,
        },
        Decided::Violated {
            counterexample,
            stats,
        } => TimedSafetyOutcome::Violated {
            counterexample,
            stats,
        },
        Decided::Undecided { cause, stats } => TimedSafetyOutcome::Undecided { cause, stats },
        Decided::Stopped { cause, stats } => TimedSafetyOutcome::Stopped { cause, stats },
        Decided::Fault { fault, stats } => TimedSafetyOutcome::Fault { fault, stats },
    };
    let liveness = match unsupported_liveness(refinement) {
        Some(unsupported) => Some(unsupported),
        None if holds
            && refinement.abstract_operation_fairness().next().is_some()
            && matches!(safety, TimedSafetyOutcome::Holds { .. }) =>
        {
            Some(TimedLivenessHalf::Checked(liveness(
                &mut product,
                &mut run,
            )?))
        }
        None => None,
    };
    Ok(TimedRefinementOutcome { safety, liveness })
}

/// The liveness half of a refinement it does not run on: none with `F_A`
/// empty; `Unsupported` with a hidden abstract field, then with a strong
/// `F_A` constraint, as for an untimed subject (FR-143).
fn unsupported_liveness(refinement: &CheckedRefinement) -> Option<TimedLivenessHalf> {
    if refinement.abstract_operation_fairness().next().is_none() {
        None
    } else if refinement
        .objects()
        .iter()
        .any(|object| !object.hidden.is_empty())
    {
        Some(TimedLivenessHalf::Unsupported(
            UnavailabilityCause::HiddenFieldLiveness,
        ))
    } else if refinement
        .abstract_operation_fairness()
        .any(|constraint| constraint.kind() == FairnessKind::Strong)
    {
        Some(TimedLivenessHalf::Unsupported(
            UnavailabilityCause::StrongAbstractFairness,
        ))
    } else {
        None
    }
}

/// The liveness half over the product of a holding safety half; its
/// counts are the liveness product's own.
fn liveness(
    product: &mut Product<'_, '_>,
    run: &mut Run<'_, '_, '_>,
) -> Result<TimedLivenessOutcome, TimedRefinementRefusal> {
    let (decided, stats) = product.liveness(run);
    Ok(match decided {
        Ok(live::Lived::Holds) => TimedLivenessOutcome::HoldsReduced {
            reductions: Vec::new(),
            stats,
        },
        Ok(live::Lived::Violated(counterexample)) => TimedLivenessOutcome::Violated {
            counterexample,
            stats,
        },
        Ok(live::Lived::Ends(ends)) => match ends {
            Decided::Undecided { cause, .. } => TimedLivenessOutcome::Undecided { cause, stats },
            Decided::Stopped { cause, .. } => TimedLivenessOutcome::Stopped { cause, stats },
            Decided::Fault { fault, .. } => TimedLivenessOutcome::Fault { fault, stats },
            Decided::Holds { .. } | Decided::Violated { .. } => {
                return Err(ZoneCheckRefusal::Fault("liveness-end").into())
            }
        },
        Err(Halt::Refusal(refusal)) => return Err(refusal.into()),
        Err(Halt::Outcome(searched)) => match *searched {
            Searched::Stopped { cause, .. } => TimedLivenessOutcome::Stopped {
                cause: TimedRefinementStop::Zone(cause),
                stats,
            },
            Searched::Undecided { cause, .. } => TimedLivenessOutcome::Undecided {
                cause: TimedRefinementUndecided::Zone(cause),
                stats,
            },
            Searched::Holds { .. } | Searched::HoldsReduced { .. } | Searched::Violated { .. } => {
                return Err(ZoneCheckRefusal::Fault("liveness-halt").into())
            }
        },
    })
}

/// The outcome or refusal a halt of the run is.
fn halted<O>(halt: Halt) -> Result<TimedSafetyOutcome<O>, TimedRefinementRefusal> {
    match halt {
        Halt::Refusal(refusal) => Err(refusal.into()),
        Halt::Outcome(searched) => Ok(match *searched {
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

/// The `q` of a node in the locked region a failing step enters, whose
/// behaviour is time-locked: it names no position (FR-244 "Refinement
/// form").
const LOCKED: usize = usize::MAX;

/// A proof's reachability part with the refinement part's positions and
/// `taken`.
struct RefinementProof {
    proof: Proof,
    positions: Vec<CertifiedPosition>,
    taken: Vec<Option<Taken>>,
}

/// The key of a refinement position: the concrete state's key, the history
/// values' key and the abstract candidates' state keys.
type PositionKey = (Vec<u8>, Vec<u8>, Vec<Vec<u8>>);

/// FR-142's discrete product over EN-6's run: the positions the run's
/// nodes carry as their `q`.
struct Product<'p, 'a> {
    refinement: &'p CheckedRefinement,
    abstract_system: &'p ModelSystem<'a>,
    /// Each position, by the index a node's `q` names.
    positions: Vec<RefinementPosition>,
    known: BTreeMap<PositionKey, usize>,
    /// The number of discrete steps from its initial node to each node.
    depth: Vec<usize>,
    /// The `Taken` of each passing step, by its source node and step
    /// index.
    taken: BTreeMap<(usize, usize), Taken>,
}

/// A judged step: the position after it, or its failure.
enum Edge {
    Passes(usize),
    Fails(Box<(RefinementFailure, CounterexampleKind)>),
}

/// What a verdict decides.
enum Judgement {
    /// The step passes, to the position at this index, taking these
    /// abstract steps.
    Passes(usize, Taken),
    /// The step or initial state fails: the failure and the counterexample
    /// kind.
    Fails(Box<(RefinementFailure, CounterexampleKind)>),
    /// The run ends with this outcome.
    Ends(Decided),
}

impl Product<'_, '_> {
    fn search(&mut self, run: &mut Run<'_, '_, '_>) -> Result<Decided, Halt> {
        let concrete = run.timed.system();
        let initial = run.timed.initial();
        let (scale, lower, upper) = (run.scale.clone(), run.lower.clone(), run.upper.clone());
        let mut queue = VecDeque::new();
        let mut roots = Vec::new();
        // Whether a failing initial state was time-locked (QSpec FR-430 "A
        // failing initial state").
        let mut locked_failure = false;
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
            let judged = self.judge(verdict, run)?;
            let mut zone = Dbm::zero(run.dim());
            run.settle(&discrete, &mut zone)?;
            let q = match judged {
                Judgement::Passes(q, _) => q,
                // The failing step of the empty prefix refutes only when
                // the initial symbolic state has a point with a
                // time-divergent continuation; otherwise it holds
                // vacuously.
                Judgement::Fails(failed) => {
                    if zone.is_empty() {
                        continue;
                    }
                    if run
                        .escape(&discrete, &zone, &scale, &lower, &upper)?
                        .diverges
                    {
                        let (failure, kind) = *failed;
                        return Ok(Decided::Violated {
                            counterexample: Box::new(refuted(index, Vec::new(), failure, kind)),
                            stats: run.stats(),
                        });
                    }
                    // A time-locked initial state: the `locked` entry of a
                    // locked region, as a failing step's post-state is
                    // (ADR-026 TV-1, per initial state).
                    locked_failure = true;
                    if let Some(node) = run.insert(discrete, LOCKED, zone, Some(index), None)? {
                        run.nodes[node].locked = true;
                        self.depth.push(0);
                        queue.push_back(node);
                    }
                    continue;
                }
                Judgement::Ends(outcome) => return Ok(outcome),
            };
            if zone.is_empty() {
                continue;
            }
            if let Some(node) = run.insert(discrete, q, zone, Some(index), None)? {
                self.depth.push(0);
                roots.push(node);
                queue.push_back(node);
            }
        }
        if roots.is_empty() {
            let cause = if locked_failure {
                ZoneUndecided::NoAdmittedBehaviour
            } else {
                ZoneUndecided::NoInitialState
            };
            return Ok(Decided::Undecided {
                cause: TimedRefinementUndecided::Zone(cause),
                stats: run.stats(),
            });
        }
        while let Some(node) = queue.pop_front() {
            run.poll()?;
            if let Some(outcome) = self.expand(run, node, &mut queue)? {
                return Ok(outcome);
            }
        }
        // ADR-026 TD-6: the half holds only over a subject that admits a
        // time-divergent behaviour from some initial state.
        for root in roots {
            if run.node_escape(root)?.diverges {
                return Ok(Decided::Holds { root });
            }
        }
        Ok(Decided::Undecided {
            cause: TimedRefinementUndecided::Zone(ZoneUndecided::NoAdmittedBehaviour),
            stats: run.stats(),
        })
    }

    /// Expand `node`: each symbolic successor by a discrete step, judged by
    /// `check_step` once per step. The outcome when an admitted edge fails
    /// or the run ends.
    fn expand(
        &mut self,
        run: &mut Run<'_, '_, '_>,
        node: usize,
        queue: &mut VecDeque<usize>,
    ) -> Result<Option<Decided>, Halt> {
        let state = run.nodes[node].state.clone();
        let zone = run.nodes[node].zone.clone();
        let q = run.nodes[node].q;
        let depth = self.depth[node];
        let steps = run.steps_for(&state)?;
        let scale = run.scale.clone();
        let (lower, upper) = (run.lower.clone(), run.upper.clone());
        // Each step's verdict, judged once: the position after it, or its
        // failure.
        let mut judged: BTreeMap<usize, Edge> = BTreeMap::new();
        for successor in run.successors(&state, &zone, &scale)? {
            let step = steps
                .get(successor.step)
                .ok_or(ZoneCheckRefusal::Fault("refinement-step"))?;
            let arrival = Arrival {
                from: node,
                step: Some((successor.step, successor.part)),
                piece: None,
            };
            if q == LOCKED {
                // The locked region: concrete steps, judged by no
                // `check_step`, kept for the certificate (FR-244).
                if let Some(created) =
                    run.insert(successor.state, LOCKED, successor.zone, None, Some(arrival))?
                {
                    self.depth.push(depth.saturating_add(1));
                    queue.push_back(created);
                }
                continue;
            }
            if let std::collections::btree_map::Entry::Vacant(slot) = judged.entry(successor.step) {
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
                let edge = match self.judge(result.verdict, run)? {
                    Judgement::Passes(next, taken) => {
                        self.taken.insert((node, successor.step), taken);
                        Edge::Passes(next)
                    }
                    Judgement::Fails(failed) => Edge::Fails(failed),
                    Judgement::Ends(outcome) => return Ok(Some(outcome)),
                };
                slot.insert(edge);
            }
            let next = match judged.get(&successor.step) {
                Some(Edge::Passes(next)) => *next,
                Some(Edge::Fails(failed)) => {
                    // ADR-026 TV-1 and TS-4: the failing step refutes only
                    // when the behaviour can go on from it, time diverging:
                    // its post-state, the successor's zone, has a valuation
                    // with a time-divergent continuation.
                    let escape =
                        run.escape(&successor.state, &successor.zone, &scale, &lower, &upper)?;
                    if !escape.diverges {
                        // No behaviour takes it: its post-state is kept as
                        // the `locked` entry of the locked region (FR-430).
                        self.lock(run, successor.state, successor.zone, arrival, depth, queue)?;
                        continue;
                    }
                    let failed = (**failed).clone();
                    let counterexample =
                        self.counterexample(run, node, (step, successor.part), failed, &escape)?;
                    return Ok(Some(Decided::Violated {
                        counterexample: Box::new(counterexample),
                        stats: run.stats(),
                    }));
                }
                None => return Err(ZoneCheckRefusal::Fault("refinement-verdict").into()),
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

    /// Keep the post-state `(state, zone)` of a failing step from `arrival`
    /// as the `locked` entry of the locked region, or mark the node of the
    /// region that covers it (FR-244 "Refinement form").
    fn lock(
        &mut self,
        run: &mut Run<'_, '_, '_>,
        state: ModelSimState,
        zone: Dbm,
        arrival: Arrival,
        depth: usize,
        queue: &mut VecDeque<usize>,
    ) -> Result<(), Halt> {
        let entry = match run.insert(state, LOCKED, zone, None, Some(arrival))? {
            Some(created) => {
                self.depth.push(depth.saturating_add(1));
                queue.push_back(created);
                created
            }
            None => run
                .edges
                .last()
                .map(|edge| edge.ends().1)
                .ok_or(ZoneCheckRefusal::Fault("locked-cover"))?,
        };
        run.nodes[entry].locked = true;
        Ok(())
    }

    /// FR-244's refinement-form certificate of a completed run that holds,
    /// its `admitted` part from initial node `root`: the reachability part
    /// with each node's position, the locked region's nodes at the reserved
    /// index, and each edge's `Taken`.
    fn certificate(&self, run: &mut Run<'_, '_, '_>, root: usize) -> Result<RefinementProof, Halt> {
        let escape = run.node_escape(root)?;
        let witness = escape
            .witness
            .filter(|_| escape.diverges)
            .ok_or(ZoneCheckRefusal::Fault("refinement-divergence-witness"))?;
        let roots: Vec<usize> = (0..run.nodes.len())
            .filter(|&node| run.nodes[node].initial.is_some())
            .collect();
        let mut proof = run.certificate(&roots, root, &witness, false)?;
        let reserved = self.positions.len();
        for node in &mut proof.nodes {
            if node.automaton_state == LOCKED {
                node.automaton_state = reserved;
            }
        }
        let concrete = run.timed.system();
        let encodes = |_| Halt::from(ZoneCheckRefusal::Fault("position-key"));
        let mut positions = Vec::with_capacity(self.positions.len());
        for position in &self.positions {
            let (_, digest) = concrete.state_key_of(&position.concrete).map_err(encodes)?;
            let mut abstract_states = Vec::with_capacity(position.abstract_states.len());
            for state in &position.abstract_states {
                abstract_states.push(self.abstract_system.state_key_of(state).map_err(encodes)?.1);
            }
            positions.push(CertifiedPosition {
                concrete: digest,
                history: position.history.key(concrete).map_err(encodes)?,
                abstract_states,
            });
        }
        // The certificate's edges are the run's, by source in discovery
        // order (`Run::certificate`).
        let mut order: Vec<usize> = (0..run.edges.len()).collect();
        order.sort_by_key(|&edge| run.edges[edge].from);
        let mut taken = Vec::with_capacity(order.len());
        for edge in order {
            let edge = &run.edges[edge];
            let into_region = run.nodes[edge.to].q == LOCKED;
            taken.push(match edge.step {
                Some((step, _)) if !into_region => Some(
                    self.taken
                        .get(&(edge.from, step))
                        .cloned()
                        .ok_or(ZoneCheckRefusal::Fault("edge-taken"))?,
                ),
                _ => None,
            });
        }
        Ok(RefinementProof {
            proof,
            positions,
            taken,
        })
    }

    /// What `verdict` decides; a passing one's position interned.
    fn judge(&mut self, verdict: StepVerdict, run: &Run<'_, '_, '_>) -> Result<Judgement, Halt> {
        Ok(match verdict {
            StepVerdict::Passes { post, taken } => {
                Judgement::Passes(self.intern(post, run)?, taken)
            }
            StepVerdict::Fails(failure) => {
                Judgement::Fails(Box::new((failure, CounterexampleKind::Formula)))
            }
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
                Judgement::Fails(Box::new((
                    failure,
                    CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                        site: evaluation.site,
                        cause: evaluation.cause,
                    }),
                )))
            }
            StepVerdict::Undetermined(undetermined) => Judgement::Ends(Decided::Undecided {
                cause: TimedRefinementUndecided::Undetermined(undetermined),
                stats: run.stats(),
            }),
            StepVerdict::Stopped(reached) => Judgement::Ends(Decided::Stopped {
                cause: TimedRefinementStop::Step(reached),
                stats: run.stats(),
            }),
            StepVerdict::Fault(fault) => Judgement::Ends(Decided::Fault {
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
    /// the failing `step` by its guard part, ending at a valuation of the
    /// step's post-state with a time-divergent continuation: within the
    /// valuations from which `escape`'s paths reach a quiescent state, in
    /// its breadth-first order, failing that the least-delay point,
    /// checked.
    fn counterexample(
        &self,
        run: &mut Run<'_, '_, '_>,
        node: usize,
        (step, part): (&GuardedStep, usize),
        (failure, kind): (RefinementFailure, CounterexampleKind),
        escape: &Escape,
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
        let mut targets = Vec::with_capacity(escape.to_quiescent.len());
        for path in &escape.to_quiescent {
            targets.push(run.pre_path(path)?);
        }
        let clocks = run.dim() - 1;
        let within = targets.into_iter().find_map(|zone| {
            concretise(
                run.timed,
                clocks,
                &stages,
                &Target::Within(zone),
                &run.scale,
            )
            .ok()
        });
        let concrete = match within {
            Some(concrete) => concrete,
            None => {
                let concrete = concretise(run.timed, clocks, &stages, &Target::Any, &run.scale)
                    .map_err(|_| ZoneCheckRefusal::Fault("concretisation"))?;
                // FR-241 names no cause for a finite path with no divergent
                // concretisation; `LassoNotConcretized` is reused, as for a
                // claim (spec gap STD-493).
                if !run.point_diverges(step.post(), &concrete)? {
                    return Err(run.undecided(ZoneUndecided::LassoNotConcretized));
                }
                concrete
            }
        };
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
