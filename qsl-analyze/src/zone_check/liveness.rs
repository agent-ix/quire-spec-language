// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-240's liveness phase: ADR-018 EN-1's second phase on a symbolic
//! product, with time divergence as one more acceptance set (ADR-026 EZ-6).
//!
//! For each `over` instance in order, the zone graph is multiplied with the
//! clause's negation automaton, read over positions
//! ([`translate_position_negation`]). A vertex pairs a model state, an
//! automaton state and a zone with:
//!
//! - its **cell**: for each clock constraint of a fairness-constrained
//!   step's guard at the state, whether the clock is below, at or above the
//!   constant. Each constraint is then enabled at every valuation of the
//!   vertex or at none (EZ-6). A delay that crosses into the next cell is a
//!   silent edge, so every instant a behaviour passes through lies in a
//!   vertex it visits, and FR-126's filter reads enabledness over time
//!   (FR-232).
//! - **Divergence**: under a dense source, a clock `z` that a silent edge
//!   resets once it reaches 1, the non-Zeno construction of Herbreteau,
//!   Srivathsan and Walukiewicz: a cycle through such resets has unbounded
//!   total delay. Under a tick source, a cycle through a tick step.
//! - **Positions**: a cycle must take a step or idle, so a vertex entered by
//!   a position belongs to one more acceptance set.
//! - **The idle tail** (FR-231): at a quiescent state, or under a tick source
//!   a terminal one, the behaviour may stutter forever, its clocks growing
//!   past every constant; its vertices read the state's letter again.
//!
//! Zones are identified when each covers the other under the aM
//! abstraction, so the product is finite and its accepting cycles are those
//! of the timed product. FR-126's candidate rule and fairness filter run on
//! it; each passing component in canonical order is concretised to a timed
//! lasso (FR-241, [`super::lasso`]) until one is.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use num_bigint::BigInt;
use num_integer::Integer as _;
use qsl_eval::automaton::{Atom, AutomatonError, Letter, PropertyAutomaton, StateId};
use qsl_eval::fairness::{analyze, Analysis, EdgeLabel, FairnessError, Graph, Identity, Vertex};
use qsl_eval::model_subject::ModelInstance;
use qsl_eval::simulation::model::{GuardedStep, ModelSimState};
use qsl_eval::temporal::{AtomReading, CaptureReading, UndefinedCause};
use qsl_replay::{
    CounterexampleKind, FairnessMember, Lasso, TemporalCounterexample, UndefinedLocus,
};
use qsl_semantics::check::CaptureSite;
use quire_exact::{Integer, Meter, Rational};
use quire_semantic_value::fairness::FairnessGranularity;
use quire_semantic_value::model_transition::ModelTransition;

use super::concretise::{concretise, NotConcretised, Stage, Target};
use super::lasso::{solve_loop, LoopStage};
use super::search::{
    constrain_at, scaled, Badness, Halt, Run, ZoneCheckLimit, ZoneStop, ZoneUndecided,
};
use super::{Bound, Dbm, ZoneCheckRefusal};
use crate::model_check::fair_walk::{fairness_constraints, FairModel};

/// Where a clock lies against one constant of its state's cells.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Sign {
    Below,
    At,
    Above,
}

/// One constant a cell compares a clock with: the zone clock and the scaled
/// constant.
type CellAtom = (usize, BigInt);

/// One vertex of the product.
#[derive(Clone, Debug)]
pub(super) struct LiveVertex {
    pub(super) state: ModelSimState,
    pub(super) q: StateId,
    /// The vertex's sign against each of its state's cell atoms.
    cell: Vec<Sign>,
    /// The zone, over the clock places and, under a dense source, `z`.
    pub(super) zone: Dbm,
    /// Entered by a position: a step, the initial position or a stutter.
    pub(super) positioned: bool,
    /// Entered by a divergence reset, or under a tick source by a tick.
    pub(super) divergent: bool,
    /// In the idle tail: the behaviour takes no further step.
    pub(super) tail: bool,
    /// The initial state's index, for an initial vertex.
    pub(super) initial: Option<usize>,
}

/// How an edge of the product is taken.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Via {
    /// A guarded step's convex part.
    Step { step: usize, part: usize },
    /// A delay into the next cell.
    Wait,
    /// The silent reset of the divergence clock `z`.
    Diverge,
    /// An idle-tail stutter position.
    Stutter,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct LiveEdge {
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) via: Via,
}

/// Where an undefined position is read: an initial state, by its index,
/// or the edge from a vertex.
#[derive(Clone, Copy, Debug)]
enum Origin {
    Initial(usize),
    Edge(usize, Via),
}

/// One instance's product.
#[derive(Default)]
pub(super) struct Product {
    pub(super) vertices: Vec<LiveVertex>,
    pub(super) edges: Vec<LiveEdge>,
    /// The edges out of each vertex, by index into `edges`.
    pub(super) out: Vec<Vec<usize>>,
    /// The edge that discovered each vertex.
    parent: Vec<Option<usize>>,
    passed: BTreeMap<Vec<u8>, Vec<usize>>,
}

/// A position's letter, or the first condition undefined there.
#[derive(Clone, Debug)]
enum Reading {
    Defined(Letter),
    Undefined(UndefinedLocus, UndefinedCause),
}

/// What expanding a vertex gives.
enum Expansion {
    /// The vertices it created, in canonical order.
    Created(Vec<usize>),
    /// The counterexample of an undefined position it reaches.
    Found(Box<TemporalCounterexample>),
}

/// What one instance's product found.
enum Found {
    Counterexample(Box<TemporalCounterexample>),
    /// A fair accepting cycle, none of which concretised.
    NotConcretised,
    /// No fair accepting cycle: why, by component.
    Nothing(InstanceProof),
}

/// FR-240's decision over every instance.
pub(super) enum Liveness {
    /// The first instance's counterexample.
    Violated(Box<TemporalCounterexample>),
    /// No instance has a fair accepting cycle: each instance's product and
    /// the fairness kernels' analysis of it, in instance order.
    Proved(Vec<InstanceProof>),
}

/// One instance's product with no fair accepting cycle, as FR-244's
/// component part reads it.
pub(super) struct InstanceProof {
    pub(super) product: Product,
    /// The kernels' components of the product, each with its outcome.
    pub(super) analyses: Vec<Analysis>,
    /// Each kernel fairness constraint as the clause's member it is.
    pub(super) members: Vec<FairnessMember>,
    /// The automaton's acceptance sets: the kernels' sets after them are
    /// divergence, then position.
    pub(super) automaton_sets: usize,
    /// The aM bounds the product's vertices are identified under.
    pub(super) bounds: Vec<Option<BigInt>>,
}

/// The run's view of one instance's product.
struct Live<'p> {
    instance: &'p ModelInstance,
    automaton: &'p mut PropertyAutomaton,
    /// Whether the product reads no claim: one automaton state that reads
    /// every letter and accepts every run, for FR-240's vacuity test.
    universal: bool,
    start: StateId,
    capture: CaptureSite,
    /// The operation texts the clause's fairness constrains.
    constrained: BTreeSet<String>,
    readings: BTreeMap<(Vec<u8>, bool), Reading>,
    /// The cell atoms of each model state, by its key.
    atoms: BTreeMap<Vec<u8>, Vec<CellAtom>>,
    /// The zone dimension: the reference clock, the clock places and, under
    /// a dense source, `z`.
    dim: usize,
    lower: Vec<Option<BigInt>>,
    upper: Vec<Option<BigInt>>,
    product: Product,
}

impl Live<'_> {
    /// The states the automaton reads `letter` into from `q`.
    fn successors(&mut self, q: StateId, letter: Letter) -> Result<Vec<StateId>, AutomatonError> {
        if self.universal {
            Ok(vec![q])
        } else {
            self.automaton.successors(q, letter)
        }
    }

    /// The atom of a guarded activation's condition.
    fn when(&self) -> Option<Atom> {
        if self.universal {
            None
        } else {
            self.automaton.when()
        }
    }

    /// The `holds` atoms the letter reads.
    fn atoms(&self) -> Vec<qsl_semantics::check::FormulaNodeId> {
        if self.universal {
            Vec::new()
        } else {
            self.automaton.atoms().to_vec()
        }
    }

    /// The number of the automaton's acceptance sets.
    fn acceptance_sets(&self) -> usize {
        if self.universal {
            0
        } else {
            self.automaton.acceptance_set_count()
        }
    }
}

impl Run<'_, '_, '_> {
    /// FR-240 over every instance of the claim, in order: the first
    /// instance's counterexample, `None` when no fair accepting cycle
    /// exists.
    ///
    /// # Errors
    ///
    /// [`ZoneUndecided::LassoNotConcretized`] when a fair accepting cycle
    /// exists and none concretises.
    pub(super) fn liveness(&mut self) -> Result<Liveness, Halt> {
        let mut automaton = self
            .live
            .take()
            .ok_or(ZoneCheckRefusal::Fault("liveness-automaton"))?;
        let found = self.each_instance(&mut automaton);
        self.live = Some(automaton);
        found
    }

    fn each_instance(&mut self, automaton: &mut PropertyAutomaton) -> Result<Liveness, Halt> {
        let claim = self
            .claim()
            .ok_or(ZoneCheckRefusal::Fault("liveness-claim"))?;
        let instances = claim.instances.clone();
        let capture = claim.clause.capture_site();
        let mut constrained = BTreeSet::new();
        for constraint in claim.clause.fairness() {
            let operation = constraint.operation();
            let text = self
                .timed
                .system()
                .operation_text(operation.declaring_type(), operation.name())
                .ok_or(ZoneCheckRefusal::Fault("fairness-operation"))?;
            constrained.insert(text.to_owned());
        }
        let dense = self.timed.tick_period().is_none();
        let dim = self.timed.places().len() + 1 + usize::from(dense);
        // The aM abstraction: one constant per clock, the greatest it is
        // compared with, which bounds every cell atom too; `z` is compared
        // with 1.
        let mut most: Vec<Option<BigInt>> = self
            .lower
            .iter()
            .zip(&self.upper)
            .map(|(lower, upper)| lower.clone().max(upper.clone()))
            .collect();
        if dense {
            most.push(Some(self.scale.clone()));
        }
        let start = self.automaton(automaton.initial())?;
        let mut not_concretised = false;
        let mut proofs = Vec::with_capacity(instances.len());
        for instance in &instances {
            let mut live = Live {
                instance,
                automaton: &mut *automaton,
                universal: false,
                start,
                capture,
                constrained: constrained.clone(),
                readings: BTreeMap::new(),
                atoms: BTreeMap::new(),
                dim,
                lower: most.clone(),
                upper: most.clone(),
                product: Product::default(),
            };
            match self.decide(&mut live)? {
                Found::Counterexample(counterexample) => {
                    return Ok(Liveness::Violated(counterexample))
                }
                Found::NotConcretised => not_concretised = true,
                Found::Nothing(proof) => proofs.push(proof),
            }
        }
        if not_concretised {
            return Err(self.undecided(ZoneUndecided::LassoNotConcretized));
        }
        // FR-240 "Vacuity": a proof needs a fair time-divergent cycle or a
        // quiescent state. FR-239's test before the search found a
        // time-divergent behaviour, which is fair when no constraint
        // applies.
        if let (false, Some(instance)) = (constrained.is_empty(), instances.first()) {
            let mut live = Live {
                instance,
                automaton: &mut *automaton,
                universal: true,
                start,
                capture: CaptureSite::Never,
                constrained,
                readings: BTreeMap::new(),
                atoms: BTreeMap::new(),
                dim,
                lower: most.clone(),
                upper: most,
                product: Product::default(),
            };
            if !self.admits_fair(&mut live)? {
                return Err(self.undecided(ZoneUndecided::NoAdmittedBehaviour));
            }
        }
        Ok(Liveness::Proved(proofs))
    }

    /// Whether the subject, read by `live`'s universal automaton, reaches a
    /// quiescent state (an idle tail) or a fair time-divergent cycle.
    ///
    /// A reached quiescent state admits the subject even when its idle
    /// tail is unfair: FR-240 "Vacuity" bars a proof only when the initial
    /// state "reaches no fair time-divergent cycle and no quiescent state"
    /// (spec gap STD-493, comment fd83efcd, item 3).
    fn admits_fair(&mut self, live: &mut Live<'_>) -> Result<bool, Halt> {
        self.build(live)?;
        if live.product.vertices.iter().any(|vertex| vertex.tail) {
            return Ok(true);
        }
        let (model, vertices, _) = self.fair_model(live)?;
        let analyses = analyze(&model.graph, &vertices, &model.constraints, self.cancel)
            .map_err(|error| self.fairness(error))?;
        Ok(Analysis::first_fair_in(&analyses).is_some())
    }

    /// An automaton result, a state limit stopping the run.
    fn automaton<T>(&self, result: Result<T, AutomatonError>) -> Result<T, Halt> {
        result.map_err(|error| match error {
            AutomatonError::StateLimit { limit } => self.stop(ZoneStop::LimitReached(
                ZoneCheckLimit::MaxAutomatonStates(limit),
            )),
            other => ZoneCheckRefusal::Automaton(other).into(),
        })
    }

    /// One instance: build its product, report the first undefined
    /// position with a time-divergent continuation, then the first fair
    /// accepting cycle that concretises.
    fn decide(&mut self, live: &mut Live<'_>) -> Result<Found, Halt> {
        if let Some(found) = self.build(live)? {
            return Ok(Found::Counterexample(Box::new(found)));
        }
        let (model, vertices, members) = self.fair_model(live)?;
        let analyses = analyze(&model.graph, &vertices, &model.constraints, self.cancel)
            .map_err(|error| self.fairness(error))?;
        let mut passing = false;
        for component in Analysis::fair_in(&analyses) {
            self.poll()?;
            passing = true;
            let Some(&entry) = component.vertices().first() else {
                return Err(ZoneCheckRefusal::Fault("empty-component").into());
            };
            let members: BTreeSet<Vertex> = component.vertices().iter().copied().collect();
            let walk = model
                .greedy_loop(&members, component.removed(), entry)
                .map_err(|_| ZoneCheckRefusal::Fault("component-loop"))?;
            let mut cycle = Vec::with_capacity(walk.len());
            for (from, edge) in &walk {
                let position = model
                    .graph
                    .edges_from(*from)
                    .iter()
                    .position(|held| held == edge)
                    .ok_or(ZoneCheckRefusal::Fault("walk-edge"))?;
                let index = live
                    .product
                    .out
                    .get(from.index())
                    .and_then(|out| out.get(position))
                    .copied()
                    .ok_or(ZoneCheckRefusal::Fault("walk-edge"))?;
                cycle.push(index);
            }
            if let Some(found) = self.lasso(live, entry.index(), &cycle)? {
                return Ok(Found::Counterexample(Box::new(found)));
            }
        }
        Ok(if passing {
            Found::NotConcretised
        } else {
            Found::Nothing(InstanceProof {
                product: std::mem::take(&mut live.product),
                analyses,
                members,
                automaton_sets: live.acceptance_sets(),
                bounds: live.lower.clone(),
            })
        })
    }

    fn fairness(&self, error: FairnessError) -> Halt {
        match error {
            FairnessError::Cancelled => self.stop(ZoneStop::Cancelled),
            _ => ZoneCheckRefusal::Fault("fairness-graph").into(),
        }
    }

    /// The product as the fairness kernels read it, its vertices, and each
    /// kernel constraint as the clause's fairness member it is.
    #[allow(
        clippy::type_complexity,
        reason = "the model, its vertices and members"
    )]
    fn fair_model(
        &mut self,
        live: &Live<'_>,
    ) -> Result<(FairModel, BTreeSet<Vertex>, Vec<FairnessMember>), Halt> {
        let mut transitions: Vec<ModelTransition> = Vec::new();
        let mut number = |transition: &ModelTransition| -> Identity {
            let index = match transitions.iter().position(|held| held == transition) {
                Some(index) => index,
                None => {
                    transitions.push(transition.clone());
                    transitions.len() - 1
                }
            };
            Identity::new(u64::try_from(index).unwrap_or(u64::MAX))
        };
        let mut graph = Graph::new();
        let mut handles = Vec::with_capacity(live.product.vertices.len());
        let mut labels = Vec::with_capacity(live.product.edges.len());
        for vertex in &live.product.vertices {
            let steps = self.steps_for(&vertex.state)?;
            let mut enabled = Vec::new();
            for step in &steps {
                if self.enabled_at(vertex, step)? {
                    enabled.push(number(step.transition()));
                }
            }
            handles.push(graph.add_vertex(enabled));
        }
        let vertex = |index: usize| -> Result<Vertex, Halt> {
            handles
                .get(index)
                .copied()
                .ok_or_else(|| ZoneCheckRefusal::Fault("fairness-vertex").into())
        };
        for edge in &live.product.edges {
            let label = match edge.via {
                Via::Step { step, .. } => {
                    let state = &live.product.vertices[edge.from].state;
                    let steps = self.steps_for(state)?;
                    let step = steps
                        .get(step)
                        .ok_or(ZoneCheckRefusal::Fault("edge-step"))?;
                    EdgeLabel::Transition(number(step.transition()))
                }
                Via::Wait | Via::Diverge | Via::Stutter => EdgeLabel::Stutter,
            };
            labels.push(label);
        }
        for (from, out) in live.product.out.iter().enumerate() {
            for &index in out {
                let edge = live.product.edges[index];
                graph
                    .add_edge(vertex(from)?, vertex(edge.to)?, labels[index])
                    .map_err(|_| ZoneCheckRefusal::Fault("fairness-edge"))?;
            }
        }
        // The automaton's acceptance sets, then divergence, then positions.
        let vertices = &live.product.vertices;
        let mut sets: Vec<Vec<Vertex>> = (0..live.acceptance_sets())
            .map(|set| {
                handles
                    .iter()
                    .zip(vertices)
                    .filter(|(_, at)| live.automaton.is_accepting_in(set, at.q))
                    .map(|(handle, _)| *handle)
                    .collect()
            })
            .collect();
        sets.push(
            handles
                .iter()
                .zip(vertices)
                .filter(|(_, at)| at.divergent)
                .map(|(handle, _)| *handle)
                .collect(),
        );
        sets.push(
            handles
                .iter()
                .zip(vertices)
                .filter(|(_, at)| at.positioned)
                .map(|(handle, _)| *handle)
                .collect(),
        );
        for set in sets {
            graph
                .add_acceptance_set(set)
                .map_err(|_| ZoneCheckRefusal::Fault("acceptance-set"))?;
        }
        let all: BTreeSet<Vertex> = handles.iter().copied().collect();
        let claim = self
            .claim()
            .ok_or(ZoneCheckRefusal::Fault("liveness-claim"))?;
        let constraints = fairness_constraints(
            self.timed.system(),
            claim.clause.fairness(),
            &transitions,
            |transition| {
                transitions
                    .iter()
                    .position(|held| held == transition)
                    .map(|index| Identity::new(u64::try_from(index).unwrap_or(u64::MAX)))
                    .ok_or_else(|| {
                        crate::model_check::ModelCheckRefusal::Fault(
                            quire_semantic_value::catalog::InternalFault::new(
                                "zone-check",
                                "a-constrained-transition-is-numbered",
                            ),
                        )
                    })
            },
        )
        .map_err(|_| ZoneCheckRefusal::Fault("fairness-constraints"))?;
        // `fairness_constraints`' order: each clause constraint in turn, a
        // `whole` one once and an `each` one per numbered transition.
        let mut members = Vec::with_capacity(constraints.len());
        for (index, constraint) in claim.clause.fairness().iter().enumerate() {
            let operation = constraint.operation();
            let text = self
                .timed
                .system()
                .operation_text(operation.declaring_type(), operation.name())
                .ok_or(ZoneCheckRefusal::Fault("fairness-operation"))?;
            match constraint.granularity() {
                FairnessGranularity::Whole => members.push(FairnessMember {
                    constraint: index,
                    transition: None,
                }),
                FairnessGranularity::Each => members.extend(
                    transitions
                        .iter()
                        .filter(|transition| transition.operation() == text)
                        .map(|transition| FairnessMember {
                            constraint: index,
                            transition: Some(transition.clone()),
                        }),
                ),
            }
        }
        if members.len() != constraints.len() {
            return Err(ZoneCheckRefusal::Fault("fairness-members").into());
        }
        Ok((FairModel { graph, constraints }, all, members))
    }

    /// Whether `step` is enabled at the instants of `vertex` (FR-232: a
    /// convex part of its guard holds and its target's time invariants
    /// would): at every one, for a fairness-constrained step, whose
    /// enabledness its cell decides.
    fn enabled_at(&mut self, vertex: &LiveVertex, step: &GuardedStep) -> Result<bool, Halt> {
        // An idle-tail vertex reads its zone too: a step the tail does not
        // take may still be enabled there.
        let zone = vertex.zone.clone();
        for part in self.enabling_parts(step)? {
            let mut at = zone.clone();
            constrain_at(&mut at, &part, &self.scale)?;
            if !at.is_empty() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Explore one instance's product breadth first: the counterexample of
    /// the first undefined position with a time-divergent continuation.
    fn build(&mut self, live: &mut Live<'_>) -> Result<Option<TemporalCounterexample>, Halt> {
        let mut queue = VecDeque::new();
        for (index, initial) in self.timed.initial().iter().enumerate() {
            let state = initial.discrete().clone();
            let zero = Dbm::zero(live.dim);
            let reading = self.reading(live, &state, true)?;
            let letter = match reading {
                Reading::Defined(letter) => letter,
                Reading::Undefined(locus, cause) => {
                    let at = Origin::Initial(index);
                    if let Some(found) = self.undefined(live, at, &state, &zero, locus, cause)? {
                        return Ok(Some(found));
                    }
                    continue;
                }
            };
            let targets = self.automaton(live.successors(live.start, letter))?;
            for (cell, piece) in self.cells(live, &state, &zero)? {
                let Some(zone) = self.close(live, &state, piece, &cell)? else {
                    continue;
                };
                for &q in &targets {
                    let vertex = LiveVertex {
                        state: state.clone(),
                        q,
                        cell: cell.clone(),
                        zone: zone.clone(),
                        positioned: true,
                        divergent: false,
                        tail: false,
                        initial: Some(index),
                    };
                    if let Some(new) = self.add_vertex(live, vertex, None)? {
                        queue.push_back(new);
                    }
                }
            }
        }
        while let Some(at) = queue.pop_front() {
            self.poll()?;
            match self.expand_vertex(live, at)? {
                Expansion::Created(created) => queue.extend(created),
                Expansion::Found(found) => return Ok(Some(*found)),
            }
        }
        // The idle tails come after every stepping vertex, so a cycle of
        // steps comes first in canonical order.
        for at in 0..live.product.vertices.len() {
            self.poll()?;
            queue.extend(self.idle(live, at)?);
        }
        while let Some(at) = queue.pop_front() {
            self.poll()?;
            if let Expansion::Created(created) = self.expand_vertex(live, at)? {
                queue.extend(created);
            }
        }
        Ok(None)
    }

    /// The idle-tail vertices a stepping vertex `at` enters: at a quiescent
    /// state, or under a tick source a terminal one, the behaviour may
    /// stutter forever.
    fn idle(&mut self, live: &mut Live<'_>, at: usize) -> Result<Vec<usize>, Halt> {
        let vertex = live.product.vertices[at].clone();
        let state = vertex.state.clone();
        let mut created = Vec::new();
        let idles = !vertex.tail
            && match self.timed.tick_period() {
                Some(_) => self.steps_for(&state)?.is_empty(),
                None => self.timed(self.timed.is_quiescent(&state))? && self.elapses(&state)?,
            };
        if idles {
            created.extend(self.stutter(live, at)?);
        }
        Ok(created)
    }

    /// The vertices `at`'s edges create, in canonical order, or the
    /// counterexample of an undefined position it reaches.
    fn expand_vertex(&mut self, live: &mut Live<'_>, at: usize) -> Result<Expansion, Halt> {
        let vertex = live.product.vertices[at].clone();
        let mut created = Vec::new();
        let dense = self.timed.tick_period().is_none();
        let zone = vertex.zone.clone();
        let state = vertex.state.clone();
        let scale = self.scale.clone();
        if vertex.tail {
            // An idle-tail vertex stutters on.
            created.extend(self.stutter(live, at)?);
        }
        // The steps, each convex part from the cell; none in the idle tail.
        let steps = if vertex.tail {
            Vec::new()
        } else {
            self.steps_for(&state)?
        };
        for (index, step) in steps.iter().enumerate() {
            let target = self.timed(self.timed.invariants(step.post()))?;
            let tick =
                self.timed.tick_of(step.operation()) > Rational::from_integer(Integer::zero());
            for (part_index, part) in step.parts().iter().enumerate() {
                self.poll()?;
                let mut next = zone.clone();
                constrain_at(&mut next, part, &scale)?;
                if next.is_empty() {
                    continue;
                }
                for (place, value) in step.resets() {
                    next.reset(place + 1, &scaled(value, &scale))?;
                }
                constrain_at(&mut next, &target, &scale)?;
                if next.is_empty() {
                    continue;
                }
                let post = step.post().clone();
                let via = Via::Step {
                    step: index,
                    part: part_index,
                };
                let letter = match self.reading(live, &post, false)? {
                    Reading::Defined(letter) => letter,
                    Reading::Undefined(locus, cause) => {
                        let origin = Origin::Edge(at, via);
                        let found = self.undefined(live, origin, &post, &next, locus, cause)?;
                        if let Some(found) = found {
                            return Ok(Expansion::Found(Box::new(found)));
                        }
                        continue;
                    }
                };
                let targets = self.automaton(live.successors(vertex.q, letter))?;
                for (cell, piece) in self.cells(live, &post, &next)? {
                    let Some(entered) = self.close(live, &post, piece, &cell)? else {
                        continue;
                    };
                    for &q in &targets {
                        let next = LiveVertex {
                            state: post.clone(),
                            q,
                            cell: cell.clone(),
                            zone: entered.clone(),
                            positioned: true,
                            divergent: !dense && tick,
                            tail: false,
                            initial: None,
                        };
                        created.extend(self.add_vertex(live, next, Some((at, via)))?);
                    }
                }
            }
        }
        if dense && self.elapses(&state)? {
            // A delay into each next cell.
            let invariants = self.timed(self.timed.invariants(&state))?;
            let mut later = zone.clone();
            later.up();
            constrain_at(&mut later, &invariants, &scale)?;
            for (cell, piece) in self.cells(live, &state, &later)? {
                if !next_cell(&vertex.cell, &cell) {
                    continue;
                }
                let Some(entered) = self.close(live, &state, piece, &cell)? else {
                    continue;
                };
                let next = LiveVertex {
                    cell,
                    zone: entered,
                    positioned: false,
                    divergent: false,
                    initial: None,
                    ..vertex.clone()
                };
                created.extend(self.add_vertex(live, next, Some((at, Via::Wait)))?);
            }
        }
        if dense {
            // The divergence clock's reset once a unit has passed.
            let z = live.dim - 1;
            let mut reset = zone.clone();
            reset.constrain(0, z, Bound::le(-scale.clone()))?;
            if !reset.is_empty() {
                reset.reset(z, &BigInt::from(0))?;
                if let Some(entered) = self.close(live, &state, reset, &vertex.cell)? {
                    let next = LiveVertex {
                        zone: entered,
                        positioned: false,
                        divergent: true,
                        initial: None,
                        ..vertex.clone()
                    };
                    created.extend(self.add_vertex(live, next, Some((at, Via::Diverge)))?);
                }
            }
        }
        Ok(Expansion::Created(created))
    }

    /// The idle-tail stutters from vertex `at`: the state's letter read
    /// again, the zone kept, and the behaviour in the idle tail.
    fn stutter(&mut self, live: &mut Live<'_>, at: usize) -> Result<Vec<usize>, Halt> {
        let vertex = live.product.vertices[at].clone();
        let Reading::Defined(letter) = self.reading(live, &vertex.state, false)? else {
            return Err(ZoneCheckRefusal::Fault("idle-letter").into());
        };
        let dense = self.timed.tick_period().is_none();
        let mut created = Vec::new();
        for q in self.automaton(live.successors(vertex.q, letter))? {
            let next = LiveVertex {
                q,
                positioned: true,
                divergent: !dense,
                tail: true,
                initial: None,
                ..vertex.clone()
            };
            created.extend(self.add_vertex(live, next, Some((at, Via::Stutter)))?);
        }
        Ok(created)
    }

    /// Store `vertex` unless one with equal state, automaton state, cell
    /// and flags holds an aM-equivalent zone; the new vertex, or `None`
    /// with the edge recorded to the equivalent one.
    fn add_vertex(
        &mut self,
        live: &mut Live<'_>,
        vertex: LiveVertex,
        from: Option<(usize, Via)>,
    ) -> Result<Option<usize>, Halt> {
        let mut key = self.key(&vertex.state)?;
        key.extend_from_slice(
            &u64::try_from(vertex.q.index())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        key.extend(vertex.cell.iter().map(|sign| *sign as u8));
        key.push(u8::from(vertex.positioned));
        key.push(u8::from(vertex.divergent));
        key.push(u8::from(vertex.tail));
        if let Some(stored) = live.product.passed.get(&key) {
            for &candidate in stored {
                let held = &live.product.vertices[candidate].zone;
                let same = vertex.zone.alu_covered_by(held, &live.lower, &live.upper)?
                    && held.alu_covered_by(&vertex.zone, &live.lower, &live.upper)?;
                if same {
                    if let Some((from, via)) = from {
                        self.add_edge(live, from, candidate, via)?;
                    }
                    return Ok(None);
                }
            }
        }
        let bytes = super::search::zone_bytes(&vertex.zone);
        self.charge_vertex(bytes)?;
        self.escape_state();
        let index = live.product.vertices.len();
        live.product.vertices.push(vertex);
        live.product.out.push(Vec::new());
        live.product.parent.push(None);
        live.product.passed.entry(key).or_default().push(index);
        if let Some((from, via)) = from {
            self.add_edge(live, from, index, via)?;
            live.product.parent[index] = Some(live.product.edges.len() - 1);
        }
        Ok(Some(index))
    }

    fn add_edge(
        &mut self,
        live: &mut Live<'_>,
        from: usize,
        to: usize,
        via: Via,
    ) -> Result<(), Halt> {
        self.charge_edge()?;
        self.escape_edge();
        live.product.out[from].push(live.product.edges.len());
        live.product.edges.push(LiveEdge { from, to, via });
        Ok(())
    }

    /// The cell atoms of `state`: each clock constraint of a
    /// fairness-constrained step's enabledness there (FR-232: its guard and
    /// its target's time invariants on the clocks it does not reset),
    /// computed once.
    fn atoms_of(
        &mut self,
        live: &mut Live<'_>,
        state: &ModelSimState,
    ) -> Result<Vec<CellAtom>, Halt> {
        let key = self.key(state)?;
        if let Some(atoms) = live.atoms.get(&key) {
            return Ok(atoms.clone());
        }
        let mut atoms = BTreeSet::new();
        for step in self.steps_for(state)? {
            if !live.constrained.contains(step.operation()) {
                continue;
            }
            for part in self.enabling_parts(&step)? {
                for constraint in &part {
                    atoms.insert((
                        constraint.place() + 1,
                        scaled(constraint.bound(), &self.scale),
                    ));
                }
            }
        }
        let atoms: Vec<CellAtom> = atoms.into_iter().collect();
        live.atoms.insert(key, atoms.clone());
        Ok(atoms)
    }

    /// `zone` split into its cells at `state`, in sign order.
    fn cells(
        &mut self,
        live: &mut Live<'_>,
        state: &ModelSimState,
        zone: &Dbm,
    ) -> Result<Vec<(Vec<Sign>, Dbm)>, Halt> {
        let atoms = self.atoms_of(live, state)?;
        let mut pieces = vec![(Vec::new(), zone.clone())];
        for atom in &atoms {
            let mut split = Vec::with_capacity(pieces.len() * 3);
            for (signs, piece) in pieces {
                for sign in [Sign::Below, Sign::At, Sign::Above] {
                    let mut part = piece.clone();
                    within(&mut part, atom, sign)?;
                    if !part.is_empty() {
                        let mut signs = signs.clone();
                        signs.push(sign);
                        split.push((signs, part));
                    }
                }
            }
            pieces = split;
        }
        Ok(pieces)
    }

    /// `piece` held to `state`'s invariants and to `cell`, and let time
    /// elapse within them; `None` when empty.
    fn close(
        &mut self,
        live: &mut Live<'_>,
        state: &ModelSimState,
        mut piece: Dbm,
        cell: &[Sign],
    ) -> Result<Option<Dbm>, Halt> {
        let atoms = self.atoms_of(live, state)?;
        let invariants = self.timed(self.timed.invariants(state))?;
        let hold = |zone: &mut Dbm| -> Result<(), Halt> {
            constrain_at(zone, &invariants, &self.scale)?;
            for (atom, sign) in atoms.iter().zip(cell) {
                within(zone, atom, *sign)?;
            }
            Ok(())
        };
        hold(&mut piece)?;
        if !piece.is_empty() && self.elapses(state)? {
            piece.up();
            hold(&mut piece)?;
        }
        Ok((!piece.is_empty()).then_some(piece))
    }

    /// The letter of `state` for the instance, `initial` at position 0.
    fn reading(
        &mut self,
        live: &mut Live<'_>,
        state: &ModelSimState,
        initial: bool,
    ) -> Result<Reading, Halt> {
        let key = (
            self.key(state)?,
            initial && live.capture == CaptureSite::Origin,
        );
        if let Some(known) = live.readings.get(&key) {
            return Ok(known.clone());
        }
        let claim = self
            .claim()
            .ok_or(ZoneCheckRefusal::Fault("liveness-claim"))?;
        let letters = claim.letters;
        let limits = self.timed.system().meter_limits();
        let atom_stop = |stop| ZoneCheckRefusal::Atom(format!("{stop:?}"));
        let mut letter = Letter::default();
        let mut reading = None;
        let when = live.when();
        if let Some(when) = when {
            let mut meter = Meter::new(limits);
            match letters
                .activation(live.instance, state, &mut meter)
                .map_err(atom_stop)?
            {
                AtomReading::True => letter = letter.with(when),
                AtomReading::False => {}
                AtomReading::Undefined(cause) => {
                    reading = Some(Reading::Undefined(UndefinedLocus::Activation, cause));
                }
            }
        }
        let capturing = match live.capture {
            CaptureSite::Never => false,
            CaptureSite::Origin => initial,
            CaptureSite::Activations => when.is_none_or(|when| letter.holds(when)),
        };
        if capturing && reading.is_none() {
            let mut meter = Meter::new(limits);
            if let CaptureReading::Undefined { capture, cause } = letters
                .captures(live.instance, state, &mut meter)
                .map_err(atom_stop)?
            {
                reading = Some(Reading::Undefined(UndefinedLocus::Capture(capture), cause));
            }
        }
        for (index, atom) in live.atoms().into_iter().enumerate() {
            if reading.is_some() {
                break;
            }
            self.poll()?;
            let mut meter = Meter::new(limits);
            match letters
                .atom(atom, live.instance, state, &mut meter)
                .map_err(atom_stop)?
            {
                AtomReading::True => {
                    let atom =
                        Atom::new(index).map_err(|_| ZoneCheckRefusal::Fault("atom-in-range"))?;
                    letter = letter.with(atom);
                }
                AtomReading::False => {}
                AtomReading::Undefined(cause) => {
                    reading = Some(Reading::Undefined(UndefinedLocus::Atom(atom), cause));
                }
            }
        }
        let reading = reading.unwrap_or(Reading::Defined(letter));
        live.readings.insert(key, reading.clone());
        Ok(reading)
    }

    /// The counterexample of an undefined position at `state`, reached at
    /// `origin` with the valuations `zone`, when one of them has a
    /// time-divergent continuation (FR-241).
    fn undefined(
        &mut self,
        live: &mut Live<'_>,
        origin: Origin,
        state: &ModelSimState,
        zone: &Dbm,
        locus: UndefinedLocus,
        cause: UndefinedCause,
    ) -> Result<Option<TemporalCounterexample>, Halt> {
        let dense = self.timed.tick_period().is_none();
        let mut entered = zone.clone();
        self.settle_at(state, &mut entered, &self.scale.clone())?;
        if entered.is_empty() {
            return Ok(None);
        }
        let entered = if dense {
            entered.without_last_clock()
        } else {
            entered
        };
        let (scale, lower, upper) = (self.scale.clone(), self.lower.clone(), self.upper.clone());
        let escape = self.escape(state, &entered, &scale, &lower, &upper)?;
        if !escape.diverges {
            return Ok(None);
        }
        let (initial, stages) = match origin {
            Origin::Initial(index) => (index, Vec::new()),
            Origin::Edge(from, via) => {
                let path = self.path_to(live, from);
                let (initial, mut stages) = self.stages_along(live, &path)?;
                stages.push(self.stage(
                    live,
                    LiveEdge {
                        from,
                        to: from,
                        via,
                    },
                    None,
                )?);
                (initial, stages)
            }
        };
        let badness = Badness::Undefined(live.instance.clone(), cause, locus);
        self.finite_counterexample((initial, stages, 0), state.clone(), badness, escape)
            .map(Some)
    }

    /// The edges from an initial vertex to `vertex`, by index.
    fn path_to(&self, live: &Live<'_>, vertex: usize) -> Vec<usize> {
        let mut path = Vec::new();
        let mut at = vertex;
        while let Some(edge) = live.product.parent[at] {
            path.push(edge);
            at = live.product.edges[edge].from;
        }
        path.reverse();
        path
    }

    /// The initial state's index and the stages of the edges `path`, which
    /// starts at an initial vertex; divergence resets take no stage.
    fn stages_along(
        &mut self,
        live: &mut Live<'_>,
        path: &[usize],
    ) -> Result<(usize, Vec<Stage>), Halt> {
        let root = path
            .first()
            .map_or(Ok(0), |&edge| Ok::<_, Halt>(live.product.edges[edge].from))?;
        let initial = live.product.vertices[root]
            .initial
            .ok_or(ZoneCheckRefusal::Fault("path-root"))?;
        let mut stages = Vec::with_capacity(path.len());
        for &edge in path {
            let edge = live.product.edges[edge];
            if matches!(edge.via, Via::Diverge) {
                continue;
            }
            stages.push(self.stage(live, edge, None)?);
        }
        Ok((initial, stages))
    }

    /// The stage of `edge`, its delay `fixed` when given (in units of the
    /// concretisation's scale); the cells are in the run's scale.
    fn stage(
        &mut self,
        live: &mut Live<'_>,
        edge: LiveEdge,
        fixed: Option<BigInt>,
    ) -> Result<Stage, Halt> {
        let source = live.product.vertices[edge.from].clone();
        let target = live.product.vertices[edge.to].clone();
        let invariants = self.timed(self.timed.invariants(&source.state))?;
        let elapses = self.elapses(&source.state)?;
        Ok(match edge.via {
            Via::Step { step, part } => {
                let steps = self.steps_for(&source.state)?;
                let step = steps
                    .get(step)
                    .cloned()
                    .ok_or(ZoneCheckRefusal::Fault("edge-step"))?;
                let post = step.post().clone();
                Stage {
                    invariants,
                    elapses,
                    target: self.timed(self.timed.invariants(&post))?,
                    before: self.cell_zone(live, &source.state, &source.cell)?,
                    piece: if edge.to == edge.from {
                        // An undefined position's own: its cell is not
                        // known.
                        None
                    } else {
                        self.cell_zone(live, &post, &target.cell)?
                    },
                    step: Some((step, part)),
                    position: Vec::new(),
                    silent: false,
                    fixed,
                }
            }
            Via::Wait => Stage {
                invariants: invariants.clone(),
                elapses,
                target: invariants,
                before: None,
                piece: self.cell_zone(live, &target.state, &target.cell)?,
                step: None,
                position: Vec::new(),
                silent: true,
                fixed,
            },
            Via::Diverge | Via::Stutter => {
                return Err(ZoneCheckRefusal::Fault("stage-edge").into());
            }
        })
    }

    /// The zone over clock places, in the run's scale, of `cell` at
    /// `state`; `None` with no cell atom.
    fn cell_zone(
        &mut self,
        live: &mut Live<'_>,
        state: &ModelSimState,
        cell: &[Sign],
    ) -> Result<Option<Dbm>, Halt> {
        let atoms = self.atoms_of(live, state)?;
        if atoms.is_empty() {
            return Ok(None);
        }
        let mut zone = Dbm::universe(self.timed.places().len() + 1);
        for (atom, sign) in atoms.iter().zip(cell) {
            within(&mut zone, atom, *sign)?;
        }
        Ok(Some(zone))
    }

    /// FR-241: the timed lasso of the stem to `entry` and the loop `cycle`
    /// of edges from it, or `None` when it does not concretise.
    fn lasso(
        &mut self,
        live: &mut Live<'_>,
        entry: usize,
        cycle: &[usize],
    ) -> Result<Option<TemporalCounterexample>, Halt> {
        let stem = self.path_to(live, entry);
        let claim = self
            .claim()
            .ok_or(ZoneCheckRefusal::Fault("liveness-claim"))?;
        let fairness = claim.clause.fairness().to_vec();
        let counterexample = |initial, prefix, lasso| TemporalCounterexample {
            initial_state: initial,
            prefix,
            over_binding: Some(live.instance.clone()),
            fairness: fairness.clone(),
            kind: CounterexampleKind::Formula,
            lasso: Some(lasso),
            blocked: Vec::new(),
            refinement: None,
            final_delay: None,
        };
        if live.product.vertices[entry].tail {
            // The idle tail: the stem to the quiescent state, then its
            // stutter forever (FR-236).
            let tail = stem
                .iter()
                .position(|&edge| matches!(live.product.edges[edge].via, Via::Stutter))
                .ok_or(ZoneCheckRefusal::Fault("idle-entry"))?;
            let (initial, stages) = self.stages_along(live, &stem[..tail])?;
            let dim = self.timed.places().len();
            let Ok(concrete) = concretise(self.timed, dim, &stages, &Target::Any, &self.scale)
            else {
                return Ok(None);
            };
            let entry = concrete.prefix.len();
            return Ok(Some(counterexample(
                initial,
                concrete.prefix,
                Lasso {
                    entry,
                    terminal_stutter: true,
                },
            )));
        }
        // Start the loop just after a step, so its entry is a position.
        let first = cycle
            .iter()
            .position(|&edge| matches!(live.product.edges[edge].via, Via::Step { .. }))
            .ok_or(ZoneCheckRefusal::Fault("loop-position"))?;
        let mut stem = stem;
        stem.extend_from_slice(&cycle[..=first]);
        let rotated: Vec<usize> = cycle[first + 1..]
            .iter()
            .chain(&cycle[..=first])
            .copied()
            .filter(|&edge| !matches!(live.product.edges[edge].via, Via::Diverge))
            .collect();
        let dense = self.timed.tick_period().is_none();
        let mut loop_stages = Vec::with_capacity(rotated.len());
        for &edge in &rotated {
            let stage = self.stage(live, live.product.edges[edge], None)?;
            let (part, resets) = match &stage.step {
                Some((step, part)) => (
                    step.parts()
                        .get(*part)
                        .cloned()
                        .ok_or(ZoneCheckRefusal::Fault("edge-part"))?,
                    step.resets().to_vec(),
                ),
                None => (Vec::new(), Vec::new()),
            };
            loop_stages.push(LoopStage {
                elapses: stage.elapses,
                invariants: stage.invariants,
                part,
                resets,
                target: stage.target,
                before: stage.before,
                piece: stage.piece,
            });
        }
        let delays = match solve_loop(&loop_stages, &self.scale, dense, self.transitions_left()) {
            Ok(delays) => delays,
            Err(NotConcretised::Exhausted) => {
                return Err(self.stop(ZoneStop::LimitReached(
                    ZoneCheckLimit::MaxSymbolicTransitions(self.limits.max_symbolic_transitions),
                )))
            }
            Err(_) => return Ok(None),
        };
        // A scale at which every loop delay is whole.
        let factor = delays.iter().fold(BigInt::from(1), |factor, delay| {
            factor.lcm(delay.denominator().as_big())
        });
        let fine = &self.scale * &factor;
        let (initial, mut stem_stages) = self.stages_along(live, &stem)?;
        let mut copy = Vec::with_capacity(rotated.len());
        for (&edge, delay) in rotated.iter().zip(&delays) {
            copy.push(self.stage(live, live.product.edges[edge], Some(scaled(delay, &fine)))?);
        }
        for stage in stem_stages.iter_mut().chain(copy.iter_mut()) {
            for zone in [&mut stage.before, &mut stage.piece].into_iter().flatten() {
                zone.scale_by(&factor);
            }
        }
        // The loop's entry: each clock it resets at the value the loop gives
        // it, each other clock past every constant (FR-236).
        let places = self.timed.places().len();
        let mut values: Vec<Option<Rational>> = vec![None; places];
        for (stage, delay) in loop_stages.iter().zip(&delays) {
            for value in values.iter_mut().flatten() {
                *value = value.add(delay);
            }
            for (place, value) in &stage.resets {
                if let Some(slot) = values.get_mut(*place) {
                    *slot = Some(value.clone());
                }
            }
        }
        let ceiling = self.timed.max_constant();
        let mut at_entry = Dbm::universe(places + 1);
        for (place, value) in values.iter().enumerate() {
            match value {
                Some(value) => {
                    let value = scaled(value, &fine);
                    at_entry.constrain(place + 1, 0, Bound::le(value.clone()))?;
                    at_entry.constrain(0, place + 1, Bound::le(-value))?;
                }
                None => at_entry.constrain(0, place + 1, Bound::lt(-scaled(&ceiling, &fine)))?,
            }
        }
        let total = delays
            .iter()
            .fold(Rational::from_integer(Integer::zero()), |sum, delay| {
                sum.add(delay)
            });
        // Copies of the loop until the clocks it never resets pass the
        // ceiling: one more than the ceiling over the loop's delay.
        let copies = match ceiling.div(&total) {
            Some(rounds) if dense => {
                let rounds = rounds.numerator().as_big() / rounds.denominator().as_big();
                usize::try_from(rounds)
                    .unwrap_or(usize::MAX)
                    .saturating_add(2)
            }
            _ => 2,
        };
        for count in 1..=copies {
            self.poll()?;
            let mut stages = stem_stages.clone();
            for _ in 1..count {
                stages.extend(copy.iter().cloned());
            }
            let positions = stages.iter().filter(|stage| stage.step.is_some()).count();
            let Some(last) = stages.last_mut() else {
                return Err(ZoneCheckRefusal::Fault("lasso-stem").into());
            };
            let mut entering = at_entry.clone();
            if let Some(piece) = &last.piece {
                super::search::intersect(&mut entering, piece)?;
            }
            last.piece = Some(entering);
            stages.extend(copy.iter().cloned());
            let Ok(concrete) = concretise(self.timed, places, &stages, &Target::Any, &fine) else {
                continue;
            };
            return Ok(Some(counterexample(
                initial,
                concrete.prefix,
                Lasso {
                    entry: positions,
                    terminal_stutter: false,
                },
            )));
        }
        Ok(None)
    }
}

/// Hold `zone` to `atom`'s side `sign`.
fn within(zone: &mut Dbm, (clock, value): &CellAtom, sign: Sign) -> Result<(), Halt> {
    match sign {
        Sign::Below => zone.constrain(*clock, 0, Bound::lt(value.clone()))?,
        Sign::At => {
            zone.constrain(*clock, 0, Bound::le(value.clone()))?;
            zone.constrain(0, *clock, Bound::le(-value.clone()))?;
        }
        Sign::Above => zone.constrain(0, *clock, Bound::lt(-value.clone()))?,
    }
    Ok(())
}

/// Whether a delay from cell `from` enters cell `to` with no cell between:
/// from a cell with clocks at their constants, those clocks pass them and
/// no other changes; from one with none, some clocks reach their next
/// constants and no other changes.
fn next_cell(from: &[Sign], to: &[Sign]) -> bool {
    let at = from.contains(&Sign::At);
    let mut changed = false;
    for (before, after) in from.iter().zip(to) {
        match (before, after) {
            (Sign::At, Sign::Above) => changed = true,
            (Sign::At, _) => return false,
            (Sign::Below, Sign::At) if !at => changed = true,
            (before, after) if before == after => {}
            _ => return false,
        }
    }
    changed
}
