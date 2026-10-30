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
//! behaviour: its post-state is kept as a `locked` entry node, whose
//! forward closure (the locked region) is explored on concrete steps only,
//! for the certificate. The counterexample is the breadth-first tree path
//! to the pre-state, then the failing step, concretised with exact delays
//! (FR-241) to end at such a valuation.
//!
//! A product state is stored unless a stored one with the same model state
//! and position covers its zone under the aLU abstraction, as FR-239's
//! search stores its symbolic states: the failure of an edge reads its
//! discrete ends and position only, which the abstraction preserves.
//!
//! A failing initial state refutes only when its symbolic state has a
//! time-divergent point; a time-locked one holds vacuously, as a `locked`
//! entry node, and a half whose initial states admit no time-divergent
//! behaviour settles ADR-026 TD-6's `NoAdmittedBehaviour`.
//!
//! A half that holds carries FR-244's zone certificate in its refinement
//! form: the retained product graph, each node's position an index into
//! the part's `positions` (the locked region's nodes carry
//! `positions.len()`), and what `check_step` took on each edge. FR-245's
//! checker verifies it with `check_step` of its own. FR-149's simulation
//! certificate does not apply, since its explicit successors ignore clock
//! guards.
//!
//! This is the safety half only. Remaining work on QSL-1005: the liveness
//! half over a timed concrete subject (ADR-026 TS-4). A refinement record
//! whose concrete subject declares `time dense` routes here (FR-430): S3
//! marks it dense and negotiation keeps EN-6 alone.

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
use qsl_route::deadlock::ItemObligation;
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::Cancel;
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::fairness::FairnessKind;

use super::certificate::Proof;
use super::concretise::{concretise, Stage, Target};
use super::divergence::{Escape, EscapeWitness};
use super::search::{
    not_simulated, widen, Arrival, Halt, Inserted, Judged, Run, Searched, ZoneCheckLimits,
    ZoneCheckRefusal, ZoneStats, ZoneStop, ZoneUndecided,
};
use super::Dbm;

mod live;

/// A request to decide a timed refinement's safety half.
pub struct TimedRefinementRequest<'a, S> {
    /// The refinement item's obligation, which a proof's certificate names.
    pub obligation: ItemObligation<S>,
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
pub enum TimedSafetyOutcome<S> {
    /// Every reachable product state was examined and every initial state
    /// and every discrete edge with a time-divergent pre-state passed.
    Holds {
        /// FR-244's certificate in its refinement form.
        certificate: Box<ZoneCertificate<ItemObligation<S>>>,
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

/// A timed refinement's outcome (QSpec FR-430; FR-142 and FR-143
/// Outputs).
#[derive(Clone, Debug)]
pub struct TimedRefinementOutcome<S> {
    /// The safety half.
    pub safety: TimedSafetyOutcome<S>,
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
    Holds {
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
pub fn check_timed_refinement<S>(
    request: TimedRefinementRequest<'_, S>,
    cancel: &Cancel,
) -> Result<TimedRefinementOutcome<S>, TimedRefinementRefusal> {
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
    if timed.carries_clocks() {
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
        taken: Vec::new(),
    };
    let ended = product.search(&mut run).and_then(|ended| match ended {
        Ended::Outcome(outcome) => Ok(outcome),
        Ended::Holds {
            initial,
            root,
            witness,
        } => {
            let certified = product.certificate(&mut run, &initial, root, &witness)?;
            let refinement_node = refinement
                .requirement_occurrence()
                .ok_or(Halt::from(ZoneCheckRefusal::Fault("refinement-record")))?
                .node();
            let part = RefinementPart {
                refinement_node,
                positions: certified.positions,
                taken: certified.taken,
            };
            let mut certificate = certified.proof.proving(request.obligation);
            certificate.refinement = Some(part);
            Ok(TimedSafetyOutcome::Holds {
                certificate: Box::new(certificate),
                stats: run.stats(),
            })
        }
    });
    let safety = match ended {
        Ok(outcome) => outcome,
        Err(Halt::Refusal(refusal)) => return Err(refusal.into()),
        Err(Halt::Outcome(searched)) => match *searched {
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
        },
    };
    let liveness = match unsupported_liveness(refinement) {
        Some(unsupported) => Some(unsupported),
        None if refinement.abstract_operation_fairness().next().is_some()
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
        Ok(live::Lived::Holds) => TimedLivenessOutcome::Holds { stats },
        Ok(live::Lived::Violated(counterexample)) => TimedLivenessOutcome::Violated {
            counterexample,
            stats,
        },
        Ok(live::Lived::Ends(ends)) => match ends {
            TimedSafetyOutcome::Undecided { cause, .. } => {
                TimedLivenessOutcome::Undecided { cause, stats }
            }
            TimedSafetyOutcome::Stopped { cause, .. } => {
                TimedLivenessOutcome::Stopped { cause, stats }
            }
            TimedSafetyOutcome::Fault { fault, .. } => TimedLivenessOutcome::Fault { fault, stats },
            TimedSafetyOutcome::Holds { .. } | TimedSafetyOutcome::Violated { .. } => {
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
    /// What `check_step` took on each retained edge of the run, by the
    /// run's edge index; `None` for an edge into or inside the locked
    /// region.
    taken: Vec<Option<Taken>>,
}

/// The position index of a `locked` entry node: the post-state of a step,
/// or an initial state, that fails only into time-locked valuations.
const LOCKED_ENTRY: usize = usize::MAX;

/// The position index of a node of the locked region reached from an entry
/// node. Distinct from [`LOCKED_ENTRY`], so no unmarked node covers an
/// entry node; both are written as `positions.len()`.
const LOCKED_INTERIOR: usize = usize::MAX - 1;

/// A judged step: the position after it and what it took, or its failure.
enum Edge {
    Passes(usize, Taken),
    Fails(Box<(RefinementFailure, CounterexampleKind)>),
}

/// What a verdict decides.
enum Judgement<S> {
    /// The step passes, to the position at this index, taking this.
    Passes(usize, Taken),
    /// The step or initial state fails: the failure and the counterexample
    /// kind.
    Fails(Box<(RefinementFailure, CounterexampleKind)>),
    /// The run ends with this outcome.
    Ends(TimedSafetyOutcome<S>),
}

/// How the search ends.
enum Ended<S> {
    /// With this outcome.
    Outcome(TimedSafetyOutcome<S>),
    /// The half holds: each initial state's node in index order, and the
    /// positioned initial node `root` whose behaviour `witness` shows
    /// time-divergent (ADR-026 TD-6).
    Holds {
        initial: Vec<usize>,
        root: usize,
        witness: EscapeWitness,
    },
}

/// The certificate of a half that holds, before it names its obligation
/// and packages.
struct Certified {
    proof: Proof,
    positions: Vec<CertifiedPosition>,
    taken: Vec<Option<Taken>>,
}

impl Product<'_, '_> {
    fn search<S>(&mut self, run: &mut Run<'_, '_, '_>) -> Result<Ended<S>, Halt> {
        let concrete = run.timed.system();
        let initial = run.timed.initial();
        let (scale, lower, upper) = (run.scale.clone(), run.lower.clone(), run.upper.clone());
        let mut queue = VecDeque::new();
        // The positioned initial nodes, and each initial state's node in
        // index order (FR-244's `initials`).
        let mut roots = Vec::new();
        let mut initials = Vec::new();
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
                    if !zone.is_empty()
                        && run
                            .escape(&discrete, &zone, &scale, &lower, &upper)?
                            .diverges
                    {
                        let (failure, kind) = *failed;
                        return Ok(Ended::Outcome(TimedSafetyOutcome::Violated {
                            counterexample: Box::new(refuted(index, Vec::new(), failure, kind)),
                            stats: run.stats(),
                        }));
                    }
                    if zone.is_empty() {
                        continue;
                    }
                    // The failing step of the empty prefix, into a
                    // time-locked state: a `locked` entry node (FR-244).
                    locked_failure = true;
                    let node = self.store(
                        run,
                        discrete,
                        LOCKED_ENTRY,
                        zone,
                        Some(index),
                        None,
                        &mut queue,
                    )?;
                    initials.push(node);
                    continue;
                }
                Judgement::Ends(outcome) => return Ok(Ended::Outcome(outcome)),
            };
            if zone.is_empty() {
                continue;
            }
            let node = self.store(run, discrete, q, zone, Some(index), None, &mut queue)?;
            initials.push(node);
            if !roots.contains(&node) {
                roots.push(node);
            }
        }
        if roots.is_empty() {
            let cause = if locked_failure {
                ZoneUndecided::NoAdmittedBehaviour
            } else {
                ZoneUndecided::NoInitialState
            };
            return Ok(Ended::Outcome(TimedSafetyOutcome::Undecided {
                cause: TimedRefinementUndecided::Zone(cause),
                stats: run.stats(),
            }));
        }
        while let Some(node) = queue.pop_front() {
            run.poll()?;
            if let Some(outcome) = self.expand(run, node, &mut queue)? {
                return Ok(Ended::Outcome(outcome));
            }
        }
        // ADR-026 TD-6: the half holds only over a subject that admits a
        // time-divergent behaviour from some positioned initial state.
        for root in roots {
            let escape = run.node_escape(root)?;
            if escape.diverges {
                let witness = escape
                    .witness
                    .ok_or(ZoneCheckRefusal::Fault("divergence-witness"))?;
                return Ok(Ended::Holds {
                    initial: initials,
                    root,
                    witness,
                });
            }
        }
        Ok(Ended::Outcome(TimedSafetyOutcome::Undecided {
            cause: TimedRefinementUndecided::Zone(ZoneUndecided::NoAdmittedBehaviour),
            stats: run.stats(),
        }))
    }

    /// Store `(state, q, zone)` as [`Run::insert_at`] does, entered by
    /// `arrival` taking `taken`: the node that holds it, a new one queued.
    /// A new `locked` entry node is marked.
    #[allow(clippy::too_many_arguments)]
    fn store(
        &mut self,
        run: &mut Run<'_, '_, '_>,
        state: ModelSimState,
        q: usize,
        zone: Dbm,
        initial: Option<usize>,
        arrival: Option<(Arrival, Option<Taken>)>,
        queue: &mut VecDeque<usize>,
    ) -> Result<usize, Halt> {
        let (arrival, taken) = match arrival {
            Some((arrival, taken)) => (Some(arrival), Some(taken)),
            None => (None, None),
        };
        let depth = arrival
            .as_ref()
            .map_or(0, |arrival| self.depth[arrival.from].saturating_add(1));
        let inserted = run.insert_at(state, q, zone, initial, arrival)?;
        if let Some(taken) = taken {
            self.taken.push(taken);
        }
        Ok(match inserted {
            Inserted::Covered(node) => node,
            Inserted::New(node) => {
                if q == LOCKED_ENTRY {
                    run.nodes[node].locked = true;
                }
                self.depth.push(depth);
                queue.push_back(node);
                node
            }
        })
    }

    /// Expand `node`: each symbolic successor by a discrete step, judged by
    /// `check_step` once per step. The outcome when an admitted edge fails
    /// or the run ends.
    fn expand<S>(
        &mut self,
        run: &mut Run<'_, '_, '_>,
        node: usize,
        queue: &mut VecDeque<usize>,
    ) -> Result<Option<TimedSafetyOutcome<S>>, Halt> {
        let state = run.nodes[node].state.clone();
        let zone = run.nodes[node].zone.clone();
        let q = run.nodes[node].q;
        let depth = self.depth[node];
        let steps = run.steps_for(&state)?;
        let scale = run.scale.clone();
        let (lower, upper) = (run.lower.clone(), run.upper.clone());
        if q == LOCKED_ENTRY || q == LOCKED_INTERIOR {
            // The locked region, on concrete steps only: no `check_step`.
            for successor in run.successors(&state, &zone, &scale)? {
                let arrival = Arrival {
                    from: node,
                    step: Some((successor.step, successor.part)),
                    piece: None,
                };
                self.store(
                    run,
                    successor.state,
                    LOCKED_INTERIOR,
                    successor.zone,
                    None,
                    Some((arrival, None)),
                    queue,
                )?;
            }
            return Ok(None);
        }
        // Each step's verdict, judged once: the position after it, or its
        // failure.
        let mut judged: BTreeMap<usize, Edge> = BTreeMap::new();
        for successor in run.successors(&state, &zone, &scale)? {
            let step = steps
                .get(successor.step)
                .ok_or(ZoneCheckRefusal::Fault("refinement-step"))?;
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
                    Judgement::Passes(next, taken) => Edge::Passes(next, taken),
                    Judgement::Fails(failed) => Edge::Fails(failed),
                    Judgement::Ends(outcome) => return Ok(Some(outcome)),
                };
                slot.insert(edge);
            }
            let arrival = Arrival {
                from: node,
                step: Some((successor.step, successor.part)),
                piece: None,
            };
            let (next, taken) = match judged.get(&successor.step) {
                Some(Edge::Passes(next, taken)) => (*next, Some(taken.clone())),
                Some(Edge::Fails(failed)) => {
                    // ADR-026 TV-1 and TS-4: the failing step refutes only
                    // when the behaviour can go on from it, time diverging:
                    // its post-state, the successor's zone, has a valuation
                    // with a time-divergent continuation.
                    let escape =
                        run.escape(&successor.state, &successor.zone, &scale, &lower, &upper)?;
                    if !escape.diverges {
                        // No behaviour: the post-state is a `locked` entry
                        // node (FR-244), entered with no `taken`.
                        self.store(
                            run,
                            successor.state,
                            LOCKED_ENTRY,
                            successor.zone,
                            None,
                            Some((arrival, None)),
                            queue,
                        )?;
                        continue;
                    }
                    let failed = (**failed).clone();
                    let counterexample =
                        self.counterexample(run, node, (step, successor.part), failed, &escape)?;
                    return Ok(Some(TimedSafetyOutcome::Violated {
                        counterexample: Box::new(counterexample),
                        stats: run.stats(),
                    }));
                }
                None => return Err(ZoneCheckRefusal::Fault("refinement-verdict").into()),
            };
            self.store(
                run,
                successor.state,
                next,
                successor.zone,
                None,
                Some((arrival, taken)),
                queue,
            )?;
        }
        Ok(None)
    }

    /// FR-244's refinement form of the run's certificate: the run's proof
    /// with each locked-region node's index written `positions.len()`, the
    /// positions as digests, and each edge's `taken` in certificate order.
    fn certificate(
        &self,
        run: &mut Run<'_, '_, '_>,
        initial: &[usize],
        root: usize,
        witness: &EscapeWitness,
    ) -> Result<Certified, Halt> {
        if self.taken.len() != run.edges.len() {
            return Err(ZoneCheckRefusal::Fault("refinement-taken").into());
        }
        let mut proof = run.certificate(initial, root, witness, false)?;
        let reserved = self.positions.len();
        for node in &mut proof.nodes {
            if node.automaton_state == LOCKED_ENTRY || node.automaton_state == LOCKED_INTERIOR {
                node.automaton_state = reserved;
            }
        }
        let taken = run
            .edge_order()
            .into_iter()
            .map(|edge| self.taken[edge].clone())
            .collect();
        let concrete = run.timed.system();
        let encodes = |_| Halt::from(ZoneCheckRefusal::Fault("position-key"));
        let mut positions = Vec::with_capacity(reserved);
        for position in &self.positions {
            let abstract_states = position
                .abstract_states
                .iter()
                .map(|state| {
                    self.abstract_system
                        .state_key_of(state)
                        .map(|(_, digest)| digest)
                        .map_err(encodes)
                })
                .collect::<Result<_, _>>()?;
            positions.push(CertifiedPosition {
                concrete: concrete
                    .state_key_of(&position.concrete)
                    .map_err(encodes)?
                    .1,
                history: position.history.key(concrete).map_err(encodes)?,
                abstract_states,
            });
        }
        Ok(Certified {
            proof,
            positions,
            taken,
        })
    }

    /// What `verdict` decides; a passing one's position interned.
    fn judge<S>(
        &mut self,
        verdict: StepVerdict,
        run: &Run<'_, '_, '_>,
    ) -> Result<Judgement<S>, Halt> {
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
