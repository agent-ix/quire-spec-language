// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-181 (ADR-023 SE-1 to SE-5, HV-8, HC-8; ADR-022 GV-1, GV-2, GV-5, GV-7,
//! GM-6): EN-1 checks a single-existential hyperproperty (HP-5) through the
//! possible family.
//!
//! [`check_single_existential`] decides a `hyper` clause over `behaviours`
//! whose prefix is exactly one `exists trace b of M` as a possibility claim:
//! from every initial state of the subject some behaviour fair under the
//! clause's fairness set satisfies the body. For each instance of the
//! clause's object parameters it runs, in order:
//!
//! 1. **Phase 0** (FR-166): `witness_samples` seeded walks per initial state,
//!    drawn before any exploration. A walk ends at its first revisit of a
//!    state, which is a candidate lasso; the candidate is a witness only when
//!    its loop is fair under the fairness set and the evaluator reads the body
//!    true on it. Phase 0 settles nothing.
//! 2. **Exploration**: the product of the subject with the generalized Büchi
//!    automaton of the body itself, not its negation ([`BodySense::Itself`]),
//!    retaining every edge, whose `max_states` counts product states (HC-8).
//!    A product state the run did not fully expand is open (GM-6): at the
//!    horizon, partly expanded or undiscovered when a limit stopped the run,
//!    or with a contract conjunction undefined or refused.
//! 3. **Decision**: the first product state, in discovery order, whose letter
//!    is undefined ends the instance `Undefined` (HV-8), unless a trap
//!    precedes it; a trap is the first initial state, in subject order, whose
//!    part of the product is closed and holds no fair accepting cycle; failing
//!    both, every initial state has a witness (the sampled one, else the
//!    canonical lasso of its part) and the instance is `Witnessed` on a run
//!    that completed with no open node and `WitnessedUnchecked` otherwise;
//!    anything else is `NoDecision`.
//!
//! An initial state's part is what is reachable from its product states, and
//! its canonical lasso is ADR-018's: the stem is the first breadth-first path
//! of the part to the first fair accepting component in the part's discovery
//! order, and the loop is CX-5's greedy walk inside it. A lasso whose loop is
//! the terminal stutter step is given as the path to the terminal state with
//! `loop_entry` at the path's length (ADR-018 CX-1): the stutter steps of a
//! path are all at its end and repeat the terminal state, so dropping them
//! leaves the same infinite behaviour.
//!
//! Not built here, and absent rather than approximated: the product-closure
//! certificate a `Witnessed` outcome carries (FR-163), the replay of a witness
//! lasso and of a trap (FR-170, FR-181 Replay) and the settlement of the
//! outcome (FR-169, FR-182).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_eval::fairness::{
    analyze, check_loop, Analysis, EdgeLabel, FairnessError, Graph, Identity, LassoFairness,
    LoopStep, Vertex,
};
use qsl_eval::hyper::{
    BodySense, ComponentStep, HyperFinding, HyperInputs, HyperLetters, HyperProduct, HyperState,
    HyperTransition, ParameterBinding, Property,
};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{check_model_bounds, ModelSimState, ModelSystem};
use qsl_eval::simulation::{Limit, Outcome, SampleProvenance, StopReason, TransitionSystem};
use qsl_eval::temporal::{evaluate_tuple, ComponentTrace, TemporalOutcome};
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::IdentityLimits;
use qsl_replay::{
    ExistentialInstanceOutcome, ExistentialOutcome, IncompleteCause, LassoWitness, ModelCheckLimit,
    ModelCheckLimits, ModelLasso, ModelPath, ModelStep, OpenCause, ReachedLimit, RunEnd, RunStats,
    UndefinedEvaluation, UndefinedLocus, UndefinedSite, WitnessSource, DEFAULT_WITNESS_SEED,
};
use qsl_semantics::check::{CheckedHyperClause, HyperForm};
use quire_exact::{Cancel, Meter, ScalarLimits};
use quire_semantic_value::model_transition::ModelTransition;

use super::{
    fault, instances, retain, tree_path, widen, FairModel, HyperGraph, HyperRefusal, Positions,
    ProductStateId, RetainedEdge, Totals, BOUND_POSITIONS,
};
use crate::model_check::check::not_simulated;
use crate::model_check::fair_walk::fairness_constraints;
use crate::model_check::state_graph::{
    budget_is_ceiling, cancelled, expansion_stop, sampler, step_ceiling, stopped, trace_index,
    trace_indices_fit, Budgets, Halt,
};
use crate::model_check::ModelCheckRefusal;

/// One EN-1 run over an HP-5 item (FR-181 Inputs).
pub struct SingleExistentialRequest<'a> {
    /// The checked clause: one `exists` over the subject's behaviours.
    pub clause: &'a CheckedHyperClause,
    /// The model subject the one trace variable ranges over.
    pub subject: ModelSubject<'a>,
    /// The seam to the body's `holds` atoms.
    pub letters: &'a dyn HyperLetters,
    /// The run's caller-raisable budgets, `witness_samples` among them
    /// (FR-166, FR-184).
    pub limits: ModelCheckLimits,
    /// The search horizon, a method parameter and never a limit; it bounds
    /// each phase 0 walk too.
    pub max_depth: Option<usize>,
    /// Phase 0's seed; `None` runs under [`DEFAULT_WITNESS_SEED`].
    pub seed: Option<u64>,
    /// The evaluation meter budget of each atom and each candidate's body.
    pub meter: ScalarLimits,
}

/// What one HP-5 run returns.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SingleExistentialRun {
    /// Each instance's decision, in the canonical instance order of the
    /// clause's object parameters (ADR-023 HS-3); none for a clause with no
    /// instance (FR-125 `NoInstance`).
    pub instances: Vec<ExistentialInstanceOutcome>,
    /// The seed phase 0 ran under.
    pub seed: u64,
}

/// Decide an HP-5 item (FR-181).
///
/// # Errors
///
/// [`ModelCheckRefusal`] as [`check_model`](crate::model_check::check_model)
/// returns it for a subject that does not admit or has an unbounded root, and
/// [`HyperRefusal`] for a clause that is not HP-5.
pub fn check_single_existential(
    request: SingleExistentialRequest<'_>,
    cancel: &Cancel,
) -> Result<SingleExistentialRun, ModelCheckRefusal> {
    let clause = request.clause;
    if !matches!(clause.form(), Some(HyperForm::SingleExistential)) {
        return Err(HyperRefusal::NotSingleExistential {
            form: clause.form().cloned(),
        }
        .into());
    }
    let seed = request.seed.unwrap_or(DEFAULT_WITNESS_SEED);
    let budgets = Budgets {
        meter: request.subject.simulation.meter.work_units,
        max_candidates: request.subject.simulation.expansion.max_candidates,
    };
    let no_initial = request.subject.simulation.initial.is_empty();
    let system = ModelSystem::new(request.subject.simulation.clone())
        .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    check_model_bounds(&system, BOUND_POSITIONS).map_err(not_simulated)?;
    let bindings = instances(clause, std::slice::from_ref(&system), &|_| Ok(0))?;
    if no_initial {
        return Ok(SingleExistentialRun {
            instances: bindings
                .iter()
                .map(|binding| ExistentialInstanceOutcome {
                    instance: named(binding),
                    outcome: ExistentialOutcome::NoInitialState,
                    sampled: Vec::new(),
                    walks_stopped: 0,
                    stats: RunStats::default(),
                })
                .collect(),
            seed,
        });
    }
    let run = Run {
        request: &request,
        system,
        budgets,
        seed,
        cancel,
    };
    run.decide(&bindings)
}

/// The instance a result names: each parameter and its key.
fn named(binding: &[ParameterBinding]) -> Vec<(String, String)> {
    binding
        .iter()
        .map(|binding| (binding.parameter.clone(), binding.instance.key.clone()))
        .collect()
}

/// Phase 0's result for one instance.
struct Sampled {
    /// The witness of each initial state, where a walk gave one.
    witnesses: Vec<Option<LassoWitness>>,
    /// The walks `max_walk_steps` stopped.
    stopped: u64,
}

/// A walk that revisited a state: the lasso it describes.
struct Candidate {
    /// The states before the revisit, one per position.
    states: Vec<ModelSimState>,
    /// The steps taken to reach the revisit: one per state before it.
    steps: Vec<ModelStep>,
    /// The position the loop re-enters: the first occurrence of the revisited
    /// state.
    entry: usize,
    provenance: SampleProvenance,
}

/// What a walk drew.
struct Walked {
    /// The candidate lasso, when the walk revisited a state.
    candidate: Option<Candidate>,
    /// Whether `max_walk_steps` ended the walk.
    budget_stopped: bool,
}

/// How an instance's exploration ended.
struct Product<'x, 'a> {
    product: HyperProduct<'x, 'a>,
    graph: HyperGraph,
    /// Whether the search horizon cut the exploration short.
    horizon_cut: bool,
}

/// The exploration of one instance: its product, or why it has none.
enum Explored<'x, 'a> {
    /// The run stopped before it had a graph.
    Halted(RunEnd),
    /// The retained product and how its exploration ended.
    Product(Box<Product<'x, 'a>>, RunEnd),
}

/// An initial state's part of the product: what is reachable from its product
/// states, in breadth-first discovery order.
struct Part {
    /// The product states, in discovery order.
    members: Vec<ProductStateId>,
    /// The edge that first discovered each member but a root: its source and
    /// its position among the source's edges.
    found_by: BTreeMap<ProductStateId, (ProductStateId, usize)>,
    /// The part's roots.
    roots: Vec<ProductStateId>,
}

/// One instance's decision under way.
struct Decision<'r, 'x, 'a> {
    run: &'r Run<'r, 'a>,
    product: &'r Product<'x, 'a>,
    end: &'r RunEnd,
    /// The product states the run did not fully expand, with the reason.
    open: BTreeMap<ProductStateId, OpenCause>,
    /// The digest of each initial state of the subject, in subject order.
    initials: Vec<DigestRecord>,
}

struct Run<'r, 'a> {
    request: &'r SingleExistentialRequest<'a>,
    system: ModelSystem<'a>,
    budgets: Budgets,
    seed: u64,
    cancel: &'r Cancel,
}

impl<'a> Run<'_, 'a> {
    fn decide(
        &self,
        bindings: &[Vec<ParameterBinding>],
    ) -> Result<SingleExistentialRun, ModelCheckRefusal> {
        let sampled = match self.sample(bindings)? {
            Ok(sampled) => sampled,
            Err(halt) => {
                // Phase 0 stopped before any exploration: nothing is decided.
                return Ok(SingleExistentialRun {
                    instances: bindings
                        .iter()
                        .map(|binding| ExistentialInstanceOutcome {
                            instance: named(binding),
                            outcome: ExistentialOutcome::NoDecision {
                                end: halt.end(),
                                open: BTreeSet::new(),
                            },
                            sampled: Vec::new(),
                            walks_stopped: 0,
                            stats: RunStats::default(),
                        })
                        .collect(),
                    seed: self.seed,
                });
            }
        };
        let mut totals = Totals::default();
        let mut instances = Vec::with_capacity(bindings.len());
        for (binding, sampled) in bindings.iter().zip(sampled) {
            let mut mine = Totals::default();
            let explored = self.explore(binding, &mut totals, &mut mine)?;
            let outcome = match &explored {
                Explored::Halted(end) => Self::undecided(&sampled.witnesses, end.clone()),
                Explored::Product(product, end) => {
                    let initials = self
                        .system
                        .initial()
                        .iter()
                        .map(|state| {
                            self.system
                                .state_key(state)
                                .map(|(_, digest)| digest)
                                .map_err(ModelCheckRefusal::Encoding)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Decision {
                        run: self,
                        product,
                        end,
                        open: Self::open_states(product),
                        initials,
                    }
                    .decide(&sampled.witnesses)?
                }
            };
            instances.push(ExistentialInstanceOutcome {
                instance: named(binding),
                outcome,
                sampled: sampled.witnesses,
                walks_stopped: sampled.stopped,
                stats: mine.stats(),
            });
        }
        Ok(SingleExistentialRun {
            instances,
            seed: self.seed,
        })
    }

    /// Phase 0 for every instance (FR-166): `Err` is the stop that ended it.
    fn sample(
        &self,
        bindings: &[Vec<ParameterBinding>],
    ) -> Result<Result<Vec<Sampled>, Halt>, ModelCheckRefusal> {
        let count = self.request.subject.simulation.initial.len();
        let samples = self.request.limits.witness_samples;
        let mut all = Vec::with_capacity(bindings.len());
        if samples == 0 || bindings.is_empty() {
            return Ok(Ok(bindings
                .iter()
                .map(|_| Sampled {
                    witnesses: Vec::new(),
                    stopped: 0,
                })
                .collect()));
        }
        if !trace_indices_fit(count, samples) {
            return Ok(Err(stopped(ModelCheckLimit::WitnessSamples, samples)));
        }
        let sampler = sampler()?;
        // A walk does not depend on the instance (only whether a candidate is a
        // witness does), so each is drawn once and tested against every
        // instance that has no witness yet for this initial state.
        for _ in bindings {
            all.push(Sampled {
                witnesses: vec![None; count],
                stopped: 0,
            });
        }
        for initial in 0..count {
            for walk in 0..samples {
                if let Some(halt) = cancelled(self.cancel) {
                    return Ok(Err(halt));
                }
                let trace = trace_index(initial, samples, walk)?;
                let walked = match self.walk(&sampler, initial, trace)? {
                    Ok(walked) => walked,
                    Err(halt) => return Ok(Err(halt)),
                };
                let mut pending = false;
                for (binding, sampled) in bindings.iter().zip(all.iter_mut()) {
                    if sampled.witnesses.get(initial).is_some_and(Option::is_some) {
                        continue;
                    }
                    let Some(candidate) = &walked.candidate else {
                        if walked.budget_stopped {
                            sampled.stopped = sampled.stopped.saturating_add(1);
                        }
                        pending = true;
                        continue;
                    };
                    match self.witness(binding, initial, candidate)? {
                        Ok(Some(witness)) => {
                            if let Some(slot) = sampled.witnesses.get_mut(initial) {
                                *slot = Some(witness);
                            }
                        }
                        Ok(None) => pending = true,
                        Err(halt) => return Ok(Err(halt)),
                    }
                }
                if !pending {
                    break;
                }
            }
        }
        Ok(Ok(all))
    }

    /// Walk `trace` from `initial` to its first revisit of a state.
    fn walk(
        &self,
        sampler: &qsl_foundation::selection::DefinitionRef,
        initial: usize,
        trace: u64,
    ) -> Result<Result<Walked, Halt>, ModelCheckRefusal> {
        let limits = &self.request.limits;
        let horizon = self.request.max_depth;
        let mut seen: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
        let mut states: Vec<ModelSimState> = Vec::new();
        let mut revisit = None;
        let mut stop: Option<Result<Halt, ModelCheckRefusal>> = None;
        let walked = qsl_eval::simulation::model::sample_model_walk(
            &self.system,
            initial,
            BOUND_POSITIONS,
            sampler,
            self.seed,
            trace,
            step_ceiling(limits, horizon),
            |state| {
                if let Some(halt) = cancelled(self.cancel) {
                    stop = Some(Ok(halt));
                    return true;
                }
                let key = match self.system.state_key(state) {
                    Ok((key, _)) => key,
                    Err(refusal) => {
                        stop = Some(Err(ModelCheckRefusal::Encoding(refusal)));
                        return true;
                    }
                };
                let position = states.len();
                if let Some(entry) = seen.get(&key) {
                    revisit = Some(*entry);
                    return true;
                }
                seen.insert(key, position);
                states.push(state.clone());
                false
            },
        );
        if let Some(stop) = stop {
            return stop.map(Err);
        }
        let trace = walked.map_err(|refusal| match refusal {
            qsl_eval::simulation::NotSimulated::KeyEncoding(refusal) => {
                ModelCheckRefusal::Encoding(refusal)
            }
            _ => fault("phase-zero-walk-samples"),
        })?;
        let provenance = trace
            .provenance
            .ok_or_else(|| fault("sampled-trace-has-provenance"))?;
        let budget_stopped =
            budget_is_ceiling(limits, horizon) && provenance.stopped == StopReason::StepLimit;
        let candidate = revisit.map(|entry| {
            let steps = trace
                .steps
                .into_iter()
                .take(states.len())
                .map(|step| ModelStep {
                    transition: step.transition,
                    post_state: step.key,
                })
                .collect();
            Candidate {
                states,
                steps,
                entry,
                provenance,
            }
        });
        Ok(Ok(Walked {
            candidate,
            budget_stopped,
        }))
    }

    /// Whether `candidate` is a witness for `initial` (FR-181 Phase 0): its
    /// loop is fair under the clause's fairness set and the evaluator reads
    /// the body true on it. `Err` is a stop of the evaluation.
    fn witness(
        &self,
        instance: &[ParameterBinding],
        initial: usize,
        candidate: &Candidate,
    ) -> Result<Result<Option<LassoWitness>, Halt>, ModelCheckRefusal> {
        let entry = candidate.entry;
        let length = candidate.states.len();
        let Some(loop_steps) = candidate.steps.get(entry..length) else {
            return Err(fault("a-revisit-has-the-steps-of-its-loop"));
        };
        let mut enabled: Vec<Vec<ModelTransition>> = Vec::with_capacity(loop_steps.len());
        for state in candidate.states.get(entry..).into_iter().flatten() {
            let expansion = match self.system.successors(state) {
                Ok(expansion) => expansion,
                Err(stop) => return expansion_stop(&stop, self.budgets).map(Err),
            };
            // A contract conjunction with no value, or undecided, leaves what
            // the state enables unknown: the loop is not shown fair.
            if expansion.findings.iter().any(|finding| {
                finding.undefined_contract().is_some() || finding.is_refused_contract()
            }) {
                return Ok(Ok(None));
            }
            let mut here: Vec<ModelTransition> = expansion
                .successors
                .into_iter()
                .map(|(transition, _)| transition)
                .collect();
            here.sort();
            here.dedup();
            enabled.push(here);
        }
        if !self.loop_is_fair(loop_steps, &enabled)? {
            return Ok(Ok(None));
        }
        let hyper: Vec<HyperState> = candidate
            .states
            .iter()
            .map(|state| HyperState {
                components: vec![state.clone()],
                property: Property::Dead,
            })
            .collect();
        let mut letters = Positions {
            letters: self.request.letters,
            instance,
            sequence: hyper.iter().collect(),
        };
        let mut meter = Meter::new(self.request.meter).with_cancel(self.cancel.clone());
        let component = ComponentTrace::Lasso {
            prefix: widen(entry),
            loop_len: widen(length - entry),
        };
        let outcome = evaluate_tuple(
            self.request.clause,
            &[component],
            &mut letters,
            &mut meter,
            Some(self.cancel),
        )
        .map_err(|_| fault("the-evaluator-reads-the-candidate"))?;
        match outcome {
            TemporalOutcome::Holds => {}
            TemporalOutcome::Violated { .. } | TemporalOutcome::Undefined(_) => {
                return Ok(Ok(None))
            }
            TemporalOutcome::Incomplete(_) | TemporalOutcome::WindowLimitReached(_) => {
                return Ok(Err(cancelled(self.cancel).unwrap_or_else(|| {
                    stopped(
                        ModelCheckLimit::EvaluationMeter,
                        self.request.meter.work_units,
                    )
                })));
            }
            _ => return Err(fault("the-evaluator-reads-the-candidate-on-a-lasso")),
        }
        let entry = u32::try_from(entry).map_err(|_| fault("loop-entry-fits-u32"))?;
        Ok(Ok(Some(LassoWitness {
            lasso: ModelLasso {
                path: ModelPath {
                    initial: u32::try_from(initial).map_err(|_| fault("initial-index-fits-u32"))?,
                    steps: candidate.steps.clone(),
                },
                loop_entry: entry,
            },
            source: WitnessSource::Sampled(SampleProvenance {
                stopped: StopReason::Reached,
                ..candidate.provenance.clone()
            }),
        })))
    }

    /// Whether the loop `steps`, leaving states that enable `enabled`, is fair
    /// under the clause's fairness set (ADR-018 FA-3).
    fn loop_is_fair(
        &self,
        steps: &[ModelStep],
        enabled: &[Vec<ModelTransition>],
    ) -> Result<bool, ModelCheckRefusal> {
        let seen: BTreeSet<&ModelTransition> = steps
            .iter()
            .map(|step| &step.transition)
            .chain(enabled.iter().flatten())
            .collect();
        let transitions: Vec<ModelTransition> = seen.into_iter().cloned().collect();
        let numbered: BTreeMap<&ModelTransition, Identity> = transitions
            .iter()
            .enumerate()
            .map(|(index, transition)| (transition, Identity::new(widen(index))))
            .collect();
        let identity = |transition: &ModelTransition| {
            numbered
                .get(transition)
                .copied()
                .ok_or_else(|| fault("every-seen-transition-is-numbered"))
        };
        let constraints =
            fairness_constraints(&self.system, self.fairness(), &transitions, identity)?;
        let loop_steps = steps
            .iter()
            .zip(enabled)
            .map(|(step, enabled)| {
                let taken = identity(&step.transition)?;
                let enabled = enabled
                    .iter()
                    .map(identity)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(LoopStep::new(enabled, EdgeLabel::Transition(taken)))
            })
            .collect::<Result<Vec<_>, ModelCheckRefusal>>()?;
        match check_loop(&loop_steps, &constraints) {
            Ok(LassoFairness::Fair) => Ok(true),
            Ok(LassoFairness::Unfair { .. }) => Ok(false),
            Err(_) => Err(fault("a-revisit-has-a-loop")),
        }
    }

    /// The fairness set of the clause's one variable.
    fn fairness(&self) -> &[quire_semantic_value::fairness::FairnessConstraint] {
        self.request
            .clause
            .quantifiers()
            .first()
            .map_or(&[], |quantifier| quantifier.fairness.as_slice())
    }

    /// Explore `instance`'s product under the limits `shared` leaves, adding
    /// its counts to `shared` and to `mine`.
    fn explore<'x>(
        &'x self,
        instance: &'x [ParameterBinding],
        shared: &mut Totals,
        mine: &mut Totals,
    ) -> Result<Explored<'x, 'a>, ModelCheckRefusal> {
        let clause = self.request.clause;
        let limits = &self.request.limits;
        let built = HyperProduct::new(HyperInputs {
            systems: vec![&self.system],
            instance,
            body: clause.body(),
            profile: clause.profile(),
            sense: BodySense::Itself,
            letters: self.request.letters,
            matcher: None,
            max_automaton_states: limits.max_automaton_states,
            meter: self.request.meter,
            cancel: self.cancel,
            copy_swap: None,
        });
        let mut product = match built {
            Ok(product) => product,
            Err(qsl_eval::automaton::AutomatonError::StateLimit { limit }) => {
                return Ok(Explored::Halted(
                    stopped(ModelCheckLimit::MaxAutomatonStates, limit).end(),
                ))
            }
            Err(other) => return Err(HyperRefusal::Automaton(other).into()),
        };
        if product.start().is_err() {
            return Ok(Explored::Halted(self.halt_end(&product)?));
        }
        let spent = limits.limits;
        let exploration = retain(
            &product,
            shared.remaining(spent),
            IdentityLimits::default(),
            self.request.max_depth,
            || self.cancel.poll() && self.cancel.cause().is_some(),
        )
        .map_err(|error| {
            ModelCheckRefusal::Encoding(qsl_eval::simulation::EncodingRefusal::of(
                &error,
                IdentityLimits::default(),
            ))
        })?;
        let transitions = match &exploration.outcome {
            Outcome::Exhaustive(stats) => stats.transitions,
            Outcome::BoundReached { stats, .. }
            | Outcome::Bounded { stats, .. }
            | Outcome::Cancelled { stats, .. }
            | Outcome::Stopped { stats, .. } => stats.transitions,
        };
        let automaton = widen(product.automaton().reached_states());
        shared.add(&exploration.graph, transitions, automaton);
        mine.add(&exploration.graph, transitions, automaton);
        let (end, horizon_cut) = match &exploration.outcome {
            Outcome::Exhaustive(_) => (RunEnd::Completed, false),
            Outcome::BoundReached { .. } => (RunEnd::Completed, true),
            Outcome::Bounded { limit, .. } => {
                let (limit, value) = match limit {
                    Limit::States(_) => (ModelCheckLimit::MaxStates, widen(spent.max_states)),
                    Limit::Transitions(_) => (
                        ModelCheckLimit::MaxTransitions,
                        widen(spent.max_transitions),
                    ),
                };
                (
                    RunEnd::Stopped {
                        cause: IncompleteCause::ResourceExhausted,
                        limit: Some(ReachedLimit { limit, value }),
                    },
                    false,
                )
            }
            Outcome::Cancelled { .. } => (self.cancelled_end()?, false),
            Outcome::Stopped { .. } => (self.halt_end(&product)?, false),
        };
        Ok(Explored::Product(
            Box::new(Product {
                product,
                graph: exploration.graph,
                horizon_cut,
            }),
            end,
        ))
    }

    fn cancelled_end(&self) -> Result<RunEnd, ModelCheckRefusal> {
        match self.cancel.cause() {
            Some(source) => Ok(RunEnd::Stopped {
                cause: IncompleteCause::Cancelled { source },
                limit: None,
            }),
            None => Err(fault("a-cancelled-run-has-a-cause")),
        }
    }

    /// The end a product's recorded halt settles.
    fn halt_end(&self, product: &HyperProduct<'_, '_>) -> Result<RunEnd, ModelCheckRefusal> {
        use qsl_eval::hyper::Halt as ProductHalt;
        let halt = product
            .take_halt()
            .ok_or_else(|| fault("a-stopped-expansion-recorded-its-halt"))?;
        let halt = match halt {
            ProductHalt::Expansion { stop, .. } => expansion_stop(&stop, self.budgets)?,
            ProductHalt::Meter => stopped(
                ModelCheckLimit::EvaluationMeter,
                self.request.meter.work_units,
            ),
            ProductHalt::Cancelled(source) => Halt {
                cause: IncompleteCause::Cancelled { source },
                limit: None,
            },
            ProductHalt::Automaton(limit) => stopped(ModelCheckLimit::MaxAutomatonStates, limit),
            ProductHalt::Encoding(refusal) => return Err(ModelCheckRefusal::Encoding(refusal)),
            ProductHalt::Fault(internal) => return Err(ModelCheckRefusal::Fault(internal)),
            ProductHalt::WitnessSet(_) => return Err(fault("a-single-trace-has-no-witness-set")),
        };
        Ok(halt.end())
    }

    /// The product states the run did not fully expand (ADR-022 GM-6).
    fn open_states(product: &Product<'_, '_>) -> BTreeMap<ProductStateId, OpenCause> {
        let mut open = BTreeMap::new();
        for (index, state) in product.graph.states().iter().enumerate() {
            let id = ProductStateId(index);
            if matches!(state.state.property, Property::Undefined { .. }) {
                // The first position it reads is undefined: nothing follows.
                continue;
            }
            if !state.expanded {
                let cause = if product.horizon_cut {
                    OpenCause::MaxDepth
                } else {
                    OpenCause::NotExpanded
                };
                open.insert(id, cause);
            } else if state.findings.iter().any(|finding| {
                matches!(finding, HyperFinding::Contract { finding, .. }
                    if finding.undefined_contract().is_some() || finding.is_refused_contract())
            }) {
                open.insert(id, OpenCause::UndecidedSuccessor);
            }
        }
        open
    }

    /// The outcome of a run that has no graph to decide from: a witness for
    /// every initial state, sampled, is not checked for well-definedness;
    /// anything else decides nothing.
    fn undecided(witnesses: &[Option<LassoWitness>], end: RunEnd) -> ExistentialOutcome {
        let open = BTreeSet::new();
        match witnesses.iter().cloned().collect::<Option<Vec<_>>>() {
            Some(lassos) if !lassos.is_empty() => {
                ExistentialOutcome::WitnessedUnchecked { lassos, end, open }
            }
            _ => ExistentialOutcome::NoDecision { end, open },
        }
    }
}

impl<'a> Decision<'_, '_, 'a> {
    /// Decide the instance (FR-181 Outcomes).
    fn decide(
        &self,
        sampled: &[Option<LassoWitness>],
    ) -> Result<ExistentialOutcome, ModelCheckRefusal> {
        let graph = &self.product.graph;
        let count = self.initials.len();
        let mut wanted: Vec<Option<LassoWitness>> = (0..count)
            .map(|initial| sampled.get(initial).cloned().flatten())
            .collect();
        // The first trap, in subject order, and the rank in discovery order
        // it takes: the first product state of its initial state. An
        // undefined letter at that very state comes first: a position's
        // letter is read before the automaton steps on it (ADR-018 UE-4).
        let mut trap: Option<(usize, usize)> = None;
        let fair = self.fair_model()?;
        let analyses = if wanted.iter().any(Option::is_none) {
            match self.analyses(&fair)? {
                Ok(analyses) => analyses,
                Err(end) => {
                    return Ok(ExistentialOutcome::NoDecision {
                        end,
                        open: self.open_causes(),
                    })
                }
            }
        } else {
            Vec::new()
        };
        for (initial, slot) in wanted.iter_mut().enumerate() {
            if slot.is_some() {
                continue;
            }
            let part = self.part(initial)?;
            match self.lasso(&fair, &analyses, &part, initial)? {
                Ok(Some(witness)) => *slot = Some(witness),
                Ok(None) => {
                    let closed = !part.roots.is_empty()
                        && part.members.iter().all(|id| !self.open.contains_key(id));
                    if closed && trap.is_none() {
                        let rank = part.roots.first().map_or(0, |root| root.index());
                        trap = Some((initial, rank));
                    }
                }
                Err(end) => {
                    return Ok(ExistentialOutcome::NoDecision {
                        end,
                        open: self.open_causes(),
                    })
                }
            }
        }
        let undefined = graph
            .states()
            .iter()
            .position(|state| matches!(state.state.property, Property::Undefined { .. }));
        match (undefined, trap) {
            (Some(at), Some((_, rank))) if at <= rank => self.undefined(ProductStateId(at)),
            (Some(at), None) => self.undefined(ProductStateId(at)),
            (_, Some((initial, _))) => Ok(ExistentialOutcome::Trapped {
                stem: ModelPath {
                    initial: u32::try_from(initial).map_err(|_| fault("initial-index-fits-u32"))?,
                    steps: Vec::new(),
                },
            }),
            (None, None) => Ok(match wanted.into_iter().collect::<Option<Vec<_>>>() {
                Some(lassos) if self.open.is_empty() && *self.end == RunEnd::Completed => {
                    ExistentialOutcome::Witnessed { lassos }
                }
                Some(lassos) => ExistentialOutcome::WitnessedUnchecked {
                    lassos,
                    end: self.end.clone(),
                    open: self.open_causes(),
                },
                None => ExistentialOutcome::NoDecision {
                    end: self.end.clone(),
                    open: self.open_causes(),
                },
            }),
        }
    }

    fn open_causes(&self) -> BTreeSet<OpenCause> {
        self.open.values().copied().collect()
    }

    /// The index of the initial state of the subject whose digest is
    /// `digest`: the first with it.
    fn initial_of(&self, digest: &DigestRecord) -> Result<usize, ModelCheckRefusal> {
        self.initials
            .iter()
            .position(|known| known == digest)
            .ok_or_else(|| fault("a-root-component-is-an-initial-state"))
    }

    /// The digest of the model state of product state `id`.
    fn model_digest(&self, id: ProductStateId) -> Result<DigestRecord, ModelCheckRefusal> {
        let state = self
            .product
            .graph
            .state(id)
            .and_then(|state| state.state.components.first())
            .ok_or_else(|| fault("a-product-state-has-its-component"))?;
        self.run
            .system
            .state_key(state)
            .map(|(_, digest)| digest)
            .map_err(ModelCheckRefusal::Encoding)
    }

    /// `initial`'s part of the product: what is reachable from the product
    /// states whose model state is that initial state's.
    fn part(&self, initial: usize) -> Result<Part, ModelCheckRefusal> {
        let graph = &self.product.graph;
        let digest = self.initials.get(initial);
        let mut roots = Vec::new();
        for root in graph.initial() {
            if Some(&self.model_digest(*root)?) == digest {
                roots.push(*root);
            }
        }
        let mut members = Vec::new();
        let mut known: BTreeSet<ProductStateId> = roots.iter().copied().collect();
        let mut found_by = BTreeMap::new();
        let mut queue: VecDeque<ProductStateId> = roots.iter().copied().collect();
        while let Some(at) = queue.pop_front() {
            members.push(at);
            for (position, edge) in graph.outgoing(at).enumerate() {
                if known.insert(edge.target) {
                    found_by.insert(edge.target, (at, position));
                    queue.push_back(edge.target);
                }
            }
        }
        Ok(Part {
            members,
            found_by,
            roots,
        })
    }

    /// The edges of the path through `part`'s discovery tree to `at`, and the
    /// root it starts from.
    fn stem(
        &self,
        part: &Part,
        at: ProductStateId,
    ) -> Result<Vec<&RetainedEdge<HyperTransition>>, ModelCheckRefusal> {
        let mut path = Vec::new();
        let mut cursor = at;
        while let Some((source, position)) = part.found_by.get(&cursor) {
            path.push(
                self.product
                    .graph
                    .outgoing(*source)
                    .nth(*position)
                    .ok_or_else(|| fault("a-discovery-edge-is-retained"))?,
            );
            cursor = *source;
        }
        path.reverse();
        Ok(path)
    }

    /// The fairness kernels' view of the product, with the body automaton's
    /// acceptance sets. A body whose automaton has none (a safety body) accepts
    /// every run that has no bad prefix: the states the automaton still has a
    /// run in, so the `Dead` states the product keeps for UE-1 accept nothing.
    fn fair_model(&self) -> Result<FairModel, ModelCheckRefusal> {
        let graph = &self.product.graph;
        let mut seen: BTreeSet<&ModelTransition> = BTreeSet::new();
        for edge in graph.edges() {
            if let [ComponentStep::Model(transition)] = edge.transition.0.as_slice() {
                seen.insert(transition);
            }
        }
        for state in graph.states() {
            for finding in &state.findings {
                if let HyperFinding::Enabled { transitions, .. } = finding {
                    seen.extend(transitions);
                }
            }
        }
        let transitions: Vec<ModelTransition> = seen.into_iter().cloned().collect();
        let numbered: BTreeMap<&ModelTransition, Identity> = transitions
            .iter()
            .enumerate()
            .map(|(index, transition)| (transition, Identity::new(widen(index))))
            .collect();
        let identity = |transition: &ModelTransition| {
            numbered
                .get(transition)
                .copied()
                .ok_or_else(|| fault("every-seen-transition-is-numbered"))
        };
        let mut fair = Graph::new();
        let mut vertices = Vec::with_capacity(graph.states().len());
        for state in graph.states() {
            let mut enabled = Vec::new();
            for finding in &state.findings {
                if let HyperFinding::Enabled { transitions, .. } = finding {
                    for transition in transitions {
                        enabled.push(identity(transition)?);
                    }
                }
            }
            vertices.push(fair.add_vertex(enabled));
        }
        let vertex = |id: ProductStateId| {
            vertices
                .get(id.index())
                .copied()
                .ok_or_else(|| fault("an-edge-names-a-retained-state"))
        };
        for index in 0..graph.states().len() {
            let source = ProductStateId(index);
            for edge in graph.outgoing(source) {
                let label = match edge.transition.0.as_slice() {
                    [ComponentStep::Model(transition)] => {
                        EdgeLabel::Transition(identity(transition)?)
                    }
                    [ComponentStep::Stutter] => EdgeLabel::Stutter,
                    _ => return Err(fault("a-single-trace-step-is-one-component-step")),
                };
                fair.add_edge(vertex(source)?, vertex(edge.target)?, label)
                    .map_err(|_| fault("an-edge-joins-two-vertices"))?;
            }
        }
        let automaton = self.product.product.automaton();
        let count = automaton.acceptance_set_count();
        let accepting = |set: Option<usize>| -> Result<Vec<Vertex>, ModelCheckRefusal> {
            graph
                .states()
                .iter()
                .enumerate()
                .filter(|(_, state)| match (&state.state.property, set) {
                    (Property::After(at), Some(set)) => automaton.is_accepting_in(set, *at),
                    (Property::After(_), None) => true,
                    _ => false,
                })
                .map(|(index, _)| vertices.get(index).copied())
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| fault("a-member-is-a-vertex"))
        };
        let sets: Vec<Vec<Vertex>> = if count == 0 {
            vec![accepting(None)?]
        } else {
            (0..count)
                .map(|set| accepting(Some(set)))
                .collect::<Result<_, _>>()?
        };
        for members in sets {
            fair.add_acceptance_set(members)
                .map_err(|_| fault("an-acceptance-member-is-a-vertex"))?;
        }
        let constraints = fairness_constraints(
            &self.run.system,
            self.run.fairness(),
            &transitions,
            identity,
        )?;
        Ok(FairModel {
            graph: fair,
            constraints,
        })
    }

    /// The fairness analyses of the whole graph over the states the run fully
    /// expanded and left no cause open at, which no state's unknown enabled
    /// operations can have touched (computed once for every part). `Err` is the
    /// end of a cancelled run.
    fn analyses(
        &self,
        fair: &FairModel,
    ) -> Result<Result<Vec<Analysis>, RunEnd>, ModelCheckRefusal> {
        let graph = &self.product.graph;
        let vertices: Vec<Vertex> = fair.graph.vertices().collect();
        let within: BTreeSet<Vertex> = graph
            .states()
            .iter()
            .enumerate()
            .filter(|(index, state)| {
                state.expanded && !self.open.contains_key(&ProductStateId(*index))
            })
            .filter_map(|(index, _)| vertices.get(index).copied())
            .collect();
        match analyze(&fair.graph, &within, &fair.constraints, self.run.cancel) {
            Ok(analyses) => Ok(Ok(analyses)),
            Err(FairnessError::Cancelled) => Ok(Err(self.run.cancelled_end()?)),
            Err(FairnessError::UnknownVertex { .. } | FairnessError::EmptyLoop) => {
                Err(fault("the-fairness-graph-is-built-from-retained-states"))
            }
        }
    }

    /// The canonical lasso of `initial`'s part (ADR-018 CX-5): the stem is the
    /// path of the part's discovery tree to the first fair accepting component
    /// in the part's discovery order, and the loop is the greedy walk inside it.
    /// `Ok(None)` is a part with no fair accepting cycle among the states the
    /// run fully expanded, `Err` the end of a cancelled run. A cycle through a
    /// state the run did not fully expand is not counted: its enabled
    /// operations are not known.
    fn lasso(
        &self,
        fair: &FairModel,
        analyses: &[Analysis],
        part: &Part,
        initial: usize,
    ) -> Result<Result<Option<LassoWitness>, RunEnd>, ModelCheckRefusal> {
        let graph = &self.product.graph;
        let rank: BTreeMap<usize, usize> = part
            .members
            .iter()
            .enumerate()
            .map(|(rank, id)| (id.index(), rank))
            .collect();
        let first = |analysis: &&Analysis| {
            analysis
                .vertices()
                .iter()
                .filter_map(|vertex| rank.get(&vertex.index()))
                .min()
                .copied()
        };
        // A part is closed under successors, so a component of the whole
        // graph lies in it or is disjoint from it.
        let in_part = |analysis: &&Analysis| first(analysis).is_some();
        let Some(passing) = Analysis::fair_in(analyses)
            .filter(in_part)
            .min_by_key(|analysis| first(analysis))
        else {
            return Ok(Ok(None));
        };
        let Some(entry) = passing
            .vertices()
            .iter()
            .copied()
            .min_by_key(|vertex| rank.get(&vertex.index()).copied())
        else {
            return Err(fault("a-component-has-a-vertex"));
        };
        let component: BTreeSet<Vertex> = passing.vertices().iter().copied().collect();
        let walk = fair.greedy_loop(&component, passing.removed(), entry)?;
        let loop_path = fair
            .loop_edges(&walk)?
            .into_iter()
            .map(|(source, position)| {
                graph
                    .outgoing(source)
                    .nth(position)
                    .ok_or_else(|| fault("a-loop-edge-is-retained"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let stem = self.stem(part, ProductStateId(entry.index()))?;
        let terminal = loop_path
            .iter()
            .any(|edge| matches!(edge.transition.0.as_slice(), [ComponentStep::Stutter]));
        let mut steps = self.model_steps(&stem)?;
        let loop_entry = steps.len();
        if !terminal {
            steps.extend(self.model_steps(&loop_path)?);
        }
        Ok(Ok(Some(LassoWitness {
            lasso: ModelLasso {
                path: ModelPath {
                    initial: u32::try_from(initial).map_err(|_| fault("initial-index-fits-u32"))?,
                    steps,
                },
                loop_entry: u32::try_from(loop_entry).map_err(|_| fault("loop-entry-fits-u32"))?,
            },
            source: WitnessSource::Explored,
        })))
    }

    /// The model steps of `edges`, each with the digest of its post-state. The
    /// terminal stutter steps of a path are all at its end and repeat the
    /// terminal state, so they are left out.
    fn model_steps(
        &self,
        edges: &[&RetainedEdge<HyperTransition>],
    ) -> Result<Vec<ModelStep>, ModelCheckRefusal> {
        let mut steps = Vec::with_capacity(edges.len());
        for edge in edges {
            match edge.transition.0.as_slice() {
                [ComponentStep::Model(transition)] => steps.push(ModelStep {
                    transition: transition.clone(),
                    post_state: self.model_digest(edge.target)?,
                }),
                [ComponentStep::Stutter] => {}
                _ => return Err(fault("a-single-trace-step-is-one-component-step")),
            }
        }
        Ok(steps)
    }

    /// The `Undefined` outcome for the product state `at`, whose letter has no
    /// value (ADR-023 HV-8): the canonical breadth-first path to it, `where`
    /// naming its position, and the evaluator's cause.
    fn undefined(&self, at: ProductStateId) -> Result<ExistentialOutcome, ModelCheckRefusal> {
        let graph = &self.product.graph;
        let Some(Property::Undefined { atom, cause, .. }) =
            graph.state(at).map(|state| &state.state.property)
        else {
            return Err(fault("an-undefined-state-is-undefined"));
        };
        let (root, path) = tree_path(graph, at);
        let steps = self.model_steps(&path)?;
        // A stutter step follows a terminal state, which reads its letter once.
        if steps.len() != path.len() {
            return Err(fault("an-undefined-letter-is-reached-by-model-steps"));
        }
        let initial = self.initial_of(&self.model_digest(root)?)?;
        Ok(ExistentialOutcome::Undefined {
            stem: ModelPath {
                initial: u32::try_from(initial).map_err(|_| fault("initial-index-fits-u32"))?,
                steps: steps.clone(),
            },
            undefined: UndefinedEvaluation {
                site: UndefinedSite {
                    position: widen(steps.len()),
                    state_key: self.model_digest(at)?,
                    locus: UndefinedLocus::Atom(*atom),
                    transition: None,
                },
                cause: cause.clone(),
            },
        })
    }
}
