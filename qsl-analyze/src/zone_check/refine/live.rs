// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-143's liveness half over a timed concrete subject (QSpec FR-430,
//! ADR-026 TS-4 and EZ-6): EN-1's second phase on a symbolic refinement
//! product, reading only time-divergent concrete behaviours.
//!
//! A vertex pairs a model state, a refinement position (the safety half's
//! `q`), a cell and a zone over the clock places and, under a dense
//! source, the divergence clock `z`, as FR-240's product does for a claim
//! ([`super::super::liveness`]): the cells split by the guards of `F_C`'s
//! steps, so a concrete constraint's enabledness is exact at every instant
//! (FR-232), a cycle through a reset of `z` has unbounded total delay, and
//! a quiescent state's idle tail stutters on. A discrete edge's position
//! is `check_step`'s; a step `check_step` fails enters a time-locked
//! post-state, since the safety half holds, and takes no behaviour that
//! diverges, so it is not an edge.
//!
//! For each class of `F_A` in order, as for an untimed subject (FR-143),
//! a fair component of the vertices whose position enables the class, over
//! the edges that do not take it, with a reset of `z` and a position on
//! its cycle, is a violation: its loop has positive total delay, which
//! FR-241's exact solution of the loop's delays confirms. The lasso is
//! `Divergence` when the loop takes only abstract stutters and
//! `AbstractUnfair` otherwise.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use num_bigint::BigInt;
use qsl_eval::fairness::{analyze, Analysis, EdgeLabel, FairnessError, Graph, Identity, Vertex};
use qsl_eval::simulation::model::{ModelFinding, ModelSimState};
use qsl_eval::simulation::ExpansionLimit;
use qsl_replay::{
    abstract_classes, check_step, AbstractClass, ConcreteStep, CounterexampleKind, Lasso,
    ModelCheckLimit, ReachedLimit, RefinementFailure, Taken, TemporalCounterexample, Undetermined,
};
use quire_exact::{Integer, Rational};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::model_transition::ModelTransition;

use super::{Decided, Judgement, Product, TimedRefinementStop, TimedRefinementUndecided};
use crate::model_check::fair_walk::{fairness_constraints, FairModel};
use crate::zone_check::concretise::{concretise, Stage, Target};
use crate::zone_check::liveness::{next_cell, Cells, Sign};
use crate::zone_check::search::{
    constrain_at, scaled, widen, zone_bytes, Halt, Run, ZoneCheckRefusal, ZoneStats, ZoneUndecided,
};
use crate::zone_check::{Bound, Dbm};

/// What the liveness half decides.
pub(super) enum Lived {
    /// No class of `F_A` is violated by a fair time-divergent behaviour.
    Holds,
    /// The first violated class's lasso.
    Violated(Box<TemporalCounterexample>),
    /// The half ends with this outcome.
    Ends(Decided),
}

/// One vertex of the product.
#[derive(Clone, Debug)]
struct Node {
    state: ModelSimState,
    /// The refinement position, by its index into the product's.
    q: usize,
    cell: Vec<Sign>,
    /// The zone over the clock places and, under a dense source, `z`.
    zone: Dbm,
    /// Entered by a position: a step, the initial position or a stutter.
    positioned: bool,
    /// Entered by a reset of `z`, or under a tick source by a tick.
    divergent: bool,
    /// In the idle tail: the behaviour takes no further step.
    tail: bool,
    /// The initial state's index, for an initial vertex.
    initial: Option<usize>,
    /// The number of positions from its initial vertex.
    depth: usize,
}

/// How an edge is taken.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Via {
    Step { step: usize, part: usize },
    Wait,
    Diverge,
    Stutter,
}

#[derive(Clone, Debug)]
struct Arc {
    from: usize,
    to: usize,
    via: Via,
    /// The abstract steps a position takes; `None` for a delay.
    taken: Option<Taken>,
}

/// A position's judgement, by its position, its state's key and its step
/// (`None` for a stutter).
type JudgedKey = (usize, Vec<u8>, Option<usize>);

/// The product of one liveness run.
struct Live {
    nodes: Vec<Node>,
    arcs: Vec<Arc>,
    out: Vec<Vec<usize>>,
    /// The arc that discovered each vertex.
    parent: Vec<Option<usize>>,
    passed: BTreeMap<Vec<u8>, Vec<usize>>,
    cells: Cells,
    dim: usize,
    /// The aM bounds vertices are identified under.
    bounds: Vec<Option<BigInt>>,
    judged: BTreeMap<JudgedKey, Option<(usize, Taken)>>,
}

/// A position's step judged: its position after and the abstract steps it
/// takes, `None` when it fails.
enum Moved {
    To(Option<(usize, Taken)>),
    Ends(Decided),
}

impl Product<'_, '_> {
    /// FR-143 over the timed product (QSpec FR-430): the first class of
    /// `F_A` a fair time-divergent behaviour violates, as its lasso.
    ///
    /// # Errors
    ///
    /// [`ZoneUndecided::LassoNotConcretized`] when a violating component
    /// exists and none concretises, or the run's budgets and faults.
    /// The liveness product's own counts come with the decision, whatever
    /// it is: its vertices, its arcs and its zones' bytes.
    pub(super) fn liveness(
        &mut self,
        run: &mut Run<'_, '_, '_>,
    ) -> (Result<Lived, Halt>, ZoneStats) {
        let mut live = match self.live_product(run) {
            Ok(live) => live,
            Err(halt) => return (Err(halt), ZoneStats::default()),
        };
        let decided = self.decide_live(run, &mut live);
        let stats = ZoneStats {
            symbolic_states: widen(live.nodes.len()),
            retained_edges: widen(live.arcs.len()),
            automaton_states: 0,
            zone_bytes: live.nodes.iter().fold(0, |bytes: u64, node| {
                bytes.saturating_add(zone_bytes(&node.zone))
            }),
        };
        (decided, stats)
    }

    /// The empty liveness product of `run`'s subject.
    fn live_product(&self, run: &Run<'_, '_, '_>) -> Result<Live, Halt> {
        let concrete_fairness: Vec<_> = self
            .refinement
            .concrete_operation_fairness()
            .cloned()
            .collect();
        let mut constrained = BTreeSet::new();
        for constraint in &concrete_fairness {
            let operation = constraint.operation();
            let text = run
                .timed
                .system()
                .operation_text(operation.declaring_type(), operation.name())
                .ok_or(ZoneCheckRefusal::Fault("fairness-operation"))?;
            constrained.insert(text.to_owned());
        }
        let dense = run.timed.tick_period().is_none();
        let dim = run.timed.places().len() + 1 + usize::from(dense);
        let mut bounds: Vec<Option<BigInt>> = run
            .lower
            .iter()
            .zip(&run.upper)
            .map(|(lower, upper)| lower.clone().max(upper.clone()))
            .collect();
        if dense {
            bounds.push(Some(run.scale.clone()));
        }
        Ok(Live {
            nodes: Vec::new(),
            arcs: Vec::new(),
            out: Vec::new(),
            parent: Vec::new(),
            passed: BTreeMap::new(),
            cells: Cells::new(constrained),
            dim,
            bounds,
            judged: BTreeMap::new(),
        })
    }

    /// The decision over `live`, built here.
    fn decide_live(&mut self, run: &mut Run<'_, '_, '_>, live: &mut Live) -> Result<Lived, Halt> {
        if let Some(ends) = self.build(run, live)? {
            return Ok(Lived::Ends(ends));
        }
        let mut enabled = BTreeMap::new();
        for node in &live.nodes {
            if let std::collections::btree_map::Entry::Vacant(slot) = enabled.entry(node.q) {
                match self.abstract_enabled(run, node.q)? {
                    Ok(here) => {
                        slot.insert(here);
                    }
                    Err(ends) => return Ok(Lived::Ends(ends)),
                }
            }
        }
        let mut identities = BTreeMap::new();
        let taken = live.arcs.iter().filter_map(|arc| match &arc.taken {
            Some(Taken::Abstract(transitions)) => Some(transitions.as_slice()),
            _ => None,
        });
        for transition in enabled.values().flatten().chain(taken.flatten()) {
            let key = self
                .abstract_system
                .transition_key(transition)
                .map_err(|_| ZoneCheckRefusal::Fault("abstract-transition-key"))?;
            identities.entry(key).or_insert_with(|| transition.clone());
        }
        let identities: Vec<ModelTransition> = identities.into_values().collect();
        let abstract_fairness: Vec<_> = self
            .refinement
            .abstract_operation_fairness()
            .cloned()
            .collect();
        let classes = match abstract_classes(self.abstract_system, &abstract_fairness, &identities)
        {
            Ok(classes) => classes,
            Err(fault) => {
                return Ok(Lived::Ends(Decided::Fault {
                    fault,
                    stats: run.stats(),
                }))
            }
        };
        let mut not_concretised = false;
        for class in &classes {
            let (model, within, kept) = self.fair_model(run, live, class, &enabled)?;
            let analyses = analyze(&model.graph, &within, &model.constraints, run.cancel)
                .map_err(|error| fairness(run, error))?;
            for component in Analysis::fair_in(&analyses) {
                run.poll()?;
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
                    let index = kept
                        .get(from.index())
                        .and_then(|out| out.get(position))
                        .copied()
                        .ok_or(ZoneCheckRefusal::Fault("walk-edge"))?;
                    cycle.push(index);
                }
                if let Some(found) = self.lasso(run, live, class, entry.index(), &cycle)? {
                    return Ok(Lived::Violated(Box::new(found)));
                }
                not_concretised = true;
            }
        }
        if not_concretised {
            return Err(run.undecided(ZoneUndecided::LassoNotConcretized));
        }
        Ok(Lived::Holds)
    }

    /// Explore the product breadth first, then the idle tails.
    fn build(
        &mut self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
    ) -> Result<Option<Decided>, Halt> {
        let concrete = run.timed.system();
        let mut queue = VecDeque::new();
        for (index, initial) in run.timed.initial().iter().enumerate() {
            run.poll()?;
            let state = initial.discrete().clone();
            let verdict = qsl_replay::check_initial(
                self.refinement,
                concrete,
                self.abstract_system,
                index,
                state.state(),
            );
            let q = match self.judge(verdict, run)? {
                Judgement::Passes(q, _) => q,
                // A failing initial state is time-locked, since the safety
                // half holds: no behaviour from it diverges.
                Judgement::Fails(_) => continue,
                Judgement::Ends(ends) => return Ok(Some(ends)),
            };
            let zero = Dbm::zero(live.dim);
            for (cell, piece) in run.split(&mut live.cells, &state, &zero)? {
                let Some(zone) = run.close(&mut live.cells, &state, piece, &cell)? else {
                    continue;
                };
                let node = Node {
                    state: state.clone(),
                    q,
                    cell,
                    zone,
                    positioned: true,
                    divergent: false,
                    tail: false,
                    initial: Some(index),
                    depth: 0,
                };
                if let Some(new) = add_node(run, live, node, None)? {
                    queue.push_back(new);
                }
            }
        }
        while let Some(at) = queue.pop_front() {
            run.poll()?;
            match self.expand_live(run, live, at)? {
                Ok(created) => queue.extend(created),
                Err(ends) => return Ok(Some(ends)),
            }
        }
        // The idle tails come after every stepping vertex.
        for at in 0..live.nodes.len() {
            run.poll()?;
            let node = live.nodes[at].clone();
            let idles = !node.tail
                && match run.timed.tick_period() {
                    Some(_) => run.steps_for(&node.state)?.is_empty(),
                    None => {
                        run.timed(run.timed.is_quiescent(&node.state))?
                            && run.elapses(&node.state)?
                    }
                };
            if idles {
                match self.stutter(run, live, at)? {
                    Ok(created) => queue.extend(created),
                    Err(ends) => return Ok(Some(ends)),
                }
            }
        }
        while let Some(at) = queue.pop_front() {
            run.poll()?;
            match self.expand_live(run, live, at)? {
                Ok(created) => queue.extend(created),
                Err(ends) => return Ok(Some(ends)),
            }
        }
        Ok(None)
    }

    /// The vertices `at`'s edges create, in canonical order.
    fn expand_live(
        &mut self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
        at: usize,
    ) -> Result<Result<Vec<usize>, Decided>, Halt> {
        let node = live.nodes[at].clone();
        let dense = run.timed.tick_period().is_none();
        let scale = run.scale.clone();
        let mut created = Vec::new();
        if node.tail {
            match self.stutter(run, live, at)? {
                Ok(more) => created.extend(more),
                Err(ends) => return Ok(Err(ends)),
            }
        }
        let steps = if node.tail {
            Vec::new()
        } else {
            run.steps_for(&node.state)?
        };
        for (index, step) in steps.iter().enumerate() {
            let target = run.timed(run.timed.invariants(step.post()))?;
            let tick =
                run.timed.tick_of(step.operation()) > Rational::from_integer(Integer::zero());
            for (part_index, part) in step.parts().iter().enumerate() {
                run.poll()?;
                let mut next = node.zone.clone();
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
                let moved =
                    self.moved(run, live, &node, Some((index, step.transition())), &post)?;
                let (q, taken) = match moved {
                    Moved::To(Some(moved)) => moved,
                    Moved::To(None) => continue,
                    Moved::Ends(ends) => return Ok(Err(ends)),
                };
                for (cell, piece) in run.split(&mut live.cells, &post, &next)? {
                    let Some(entered) = run.close(&mut live.cells, &post, piece, &cell)? else {
                        continue;
                    };
                    let next = Node {
                        state: post.clone(),
                        q,
                        cell,
                        zone: entered,
                        positioned: true,
                        divergent: !dense && tick,
                        tail: false,
                        initial: None,
                        depth: node.depth + 1,
                    };
                    let arc = Arc {
                        from: at,
                        to: 0,
                        via: Via::Step {
                            step: index,
                            part: part_index,
                        },
                        taken: Some(taken.clone()),
                    };
                    created.extend(add_node(run, live, next, Some(arc))?);
                }
            }
        }
        if dense && run.elapses(&node.state)? {
            // A delay into each next cell.
            let invariants = run.timed(run.timed.invariants(&node.state))?;
            let mut later = node.zone.clone();
            later.up();
            constrain_at(&mut later, &invariants, &scale)?;
            for (cell, piece) in run.split(&mut live.cells, &node.state, &later)? {
                if !next_cell(&node.cell, &cell) {
                    continue;
                }
                let Some(entered) = run.close(&mut live.cells, &node.state, piece, &cell)? else {
                    continue;
                };
                let next = Node {
                    cell,
                    zone: entered,
                    positioned: false,
                    divergent: false,
                    initial: None,
                    ..node.clone()
                };
                let arc = Arc {
                    from: at,
                    to: 0,
                    via: Via::Wait,
                    taken: None,
                };
                created.extend(add_node(run, live, next, Some(arc))?);
            }
        }
        if dense {
            // The divergence clock's reset once a unit has passed.
            let z = live.dim - 1;
            let mut reset = node.zone.clone();
            reset.constrain(0, z, Bound::le(-scale.clone()))?;
            if !reset.is_empty() {
                reset.reset(z, &BigInt::from(0))?;
                if let Some(entered) = run.close(&mut live.cells, &node.state, reset, &node.cell)? {
                    let next = Node {
                        zone: entered,
                        positioned: false,
                        divergent: true,
                        initial: None,
                        ..node.clone()
                    };
                    let arc = Arc {
                        from: at,
                        to: 0,
                        via: Via::Diverge,
                        taken: None,
                    };
                    created.extend(add_node(run, live, next, Some(arc))?);
                }
            }
        }
        Ok(Ok(created))
    }

    /// The idle-tail stutter from `at`: the state and zone kept, the
    /// position `check_step` gives a stutter.
    fn stutter(
        &mut self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
        at: usize,
    ) -> Result<Result<Vec<usize>, Decided>, Halt> {
        let node = live.nodes[at].clone();
        let dense = run.timed.tick_period().is_none();
        let (q, taken) = match self.moved(run, live, &node, None, &node.state)? {
            Moved::To(Some(moved)) => moved,
            Moved::To(None) => return Ok(Ok(Vec::new())),
            Moved::Ends(ends) => return Ok(Err(ends)),
        };
        let next = Node {
            q,
            positioned: true,
            divergent: !dense,
            tail: true,
            initial: None,
            depth: node.depth + 1,
            ..node.clone()
        };
        let arc = Arc {
            from: at,
            to: 0,
            via: Via::Stutter,
            taken: Some(taken),
        };
        Ok(Ok(add_node(run, live, next, Some(arc))?
            .into_iter()
            .collect()))
    }

    /// `check_step` on the step `transition` (by its index; `None` for a
    /// stutter) from `node` to `post`, once per position, state and step.
    fn moved(
        &mut self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
        node: &Node,
        transition: Option<(usize, &ModelTransition)>,
        post: &ModelSimState,
    ) -> Result<Moved, Halt> {
        let key = (
            node.q,
            run.key(&node.state)?,
            transition.map(|(index, _)| index),
        );
        if let Some(moved) = live.judged.get(&key) {
            return Ok(Moved::To(moved.clone()));
        }
        let result = check_step(
            self.refinement,
            run.timed.system(),
            self.abstract_system,
            &self.positions[node.q],
            &ConcreteStep {
                position: widen(node.depth.saturating_add(1)),
                transition: transition.map(|(_, transition)| transition),
                post,
            },
        );
        let moved = match self.judge(result.verdict, run)? {
            Judgement::Passes(q, taken) => Some((q, taken)),
            // A failing step enters a time-locked post-state, since the
            // safety half holds (QSpec FR-430): no behaviour through it
            // diverges.
            Judgement::Fails(_) => None,
            Judgement::Ends(ends) => return Ok(Moved::Ends(ends)),
        };
        live.judged.insert(key, moved.clone());
        Ok(Moved::To(moved))
    }

    /// The abstract transition identities enabled from position `q`'s
    /// mapped states (ADR-020 CO-2).
    fn abstract_enabled(
        &self,
        run: &Run<'_, '_, '_>,
        q: usize,
    ) -> Result<Result<Vec<ModelTransition>, Decided>, Halt> {
        let mut here = Vec::new();
        for candidate in &self.positions[q].abstract_states {
            run.poll()?;
            let expansion = match self.abstract_system.expand_state(candidate) {
                Ok(expansion) => expansion,
                Err(stop) if stop.is_internal_failure() => {
                    return Ok(Err(Decided::Fault {
                        fault: InternalFault::new("refinement-liveness", "abstract-expansion"),
                        stats: run.stats(),
                    }))
                }
                Err(stop) => {
                    let reached = match stop.limit {
                        Some(ExpansionLimit::EvaluationMeter(counter)) => ReachedLimit {
                            limit: ModelCheckLimit::EvaluationMeter { counter },
                            value: qsl_replay::meter_bound(&self.abstract_system.meter(), counter),
                        },
                        Some(ExpansionLimit::MaxCandidates) | None => ReachedLimit {
                            limit: ModelCheckLimit::MaxCandidates,
                            value: self.abstract_system.max_candidates(),
                        },
                    };
                    return Ok(Err(Decided::Stopped {
                        cause: TimedRefinementStop::Step(reached),
                        stats: run.stats(),
                    }));
                }
            };
            if expansion
                .findings
                .iter()
                .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
            {
                return Ok(Err(Decided::Undecided {
                    cause: TimedRefinementUndecided::Undetermined(Undetermined::UndecidedSuccessor),
                    stats: run.stats(),
                }));
            }
            here.extend(
                expansion
                    .successors
                    .into_iter()
                    .map(|(transition, _)| transition),
            );
        }
        Ok(Ok(here))
    }

    /// The product as the fairness kernels read it for `class`: each
    /// vertex with the concrete steps enabled at its instants, the edges
    /// that do not take `class`, divergence and positions as acceptance
    /// sets; the vertices whose position enables `class`; and the arcs
    /// kept from each vertex, in the graph's order.
    #[allow(
        clippy::type_complexity,
        reason = "the model, the vertices and the kept arcs"
    )]
    fn fair_model(
        &self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
        class: &AbstractClass,
        enabled: &BTreeMap<usize, Vec<ModelTransition>>,
    ) -> Result<(FairModel, BTreeSet<Vertex>, Vec<Vec<usize>>), Halt> {
        let mut transitions: Vec<ModelTransition> = Vec::new();
        let mut number = |transition: &ModelTransition| -> Identity {
            let index = match transitions.iter().position(|held| held == transition) {
                Some(index) => index,
                None => {
                    transitions.push(transition.clone());
                    transitions.len() - 1
                }
            };
            Identity::new(widen(index))
        };
        let mut graph = Graph::new();
        let mut handles = Vec::with_capacity(live.nodes.len());
        for node in &live.nodes {
            let mut here = Vec::new();
            for step in &run.steps_for(&node.state)? {
                if run.enabled_at(&node.zone, step)? {
                    here.push(number(step.transition()));
                }
            }
            handles.push(graph.add_vertex(here));
        }
        let mut kept = vec![Vec::new(); live.nodes.len()];
        for (from, out) in live.out.iter().enumerate() {
            for &index in out {
                let arc = &live.arcs[index];
                if arc
                    .taken
                    .as_ref()
                    .is_some_and(|taken| class.is_taken_by(taken))
                {
                    continue;
                }
                let label = match arc.via {
                    Via::Step { step, .. } => {
                        let steps = run.steps_for(&live.nodes[from].state)?;
                        let step = steps
                            .get(step)
                            .ok_or(ZoneCheckRefusal::Fault("edge-step"))?;
                        EdgeLabel::Transition(number(step.transition()))
                    }
                    Via::Wait | Via::Diverge | Via::Stutter => EdgeLabel::Stutter,
                };
                graph
                    .add_edge(handles[from], handles[arc.to], label)
                    .map_err(|_| ZoneCheckRefusal::Fault("fairness-edge"))?;
                kept[from].push(index);
            }
        }
        for flag in [
            (|node: &Node| node.divergent) as fn(&Node) -> bool,
            |node: &Node| node.positioned,
        ] {
            let set: Vec<Vertex> = handles
                .iter()
                .zip(&live.nodes)
                .filter(|(_, node)| flag(node))
                .map(|(handle, _)| *handle)
                .collect();
            graph
                .add_acceptance_set(set)
                .map_err(|_| ZoneCheckRefusal::Fault("acceptance-set"))?;
        }
        let within: BTreeSet<Vertex> = handles
            .iter()
            .zip(&live.nodes)
            .filter(|(_, node)| {
                enabled
                    .get(&node.q)
                    .is_some_and(|here| class.is_enabled_in(here))
            })
            .map(|(handle, _)| *handle)
            .collect();
        let concrete_fairness: Vec<_> = self
            .refinement
            .concrete_operation_fairness()
            .cloned()
            .collect();
        let constraints = fairness_constraints(
            run.timed.system(),
            &concrete_fairness,
            &transitions,
            |transition| {
                transitions
                    .iter()
                    .position(|held| held == transition)
                    .map(|index| Identity::new(widen(index)))
                    .ok_or_else(|| {
                        crate::model_check::ModelCheckRefusal::Fault(InternalFault::new(
                            "zone-check",
                            "a-constrained-transition-is-numbered",
                        ))
                    })
            },
        )
        .map_err(|_| ZoneCheckRefusal::Fault("fairness-constraints"))?;
        Ok((FairModel { graph, constraints }, within, kept))
    }

    /// FR-241: the lasso of the stem to `entry` and the loop `cycle` of
    /// arcs from it, which violates `class`, or `None` when it does not
    /// concretise.
    fn lasso(
        &self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
        class: &AbstractClass,
        entry: usize,
        cycle: &[usize],
    ) -> Result<Option<TemporalCounterexample>, Halt> {
        let taking = cycle.iter().any(|&arc| {
            live.arcs[arc]
                .taken
                .as_ref()
                .is_some_and(|taken| *taken != Taken::Stutter)
        });
        let constraint = self
            .refinement
            .abstract_operation_fairness()
            .nth(class.constraint())
            .ok_or(ZoneCheckRefusal::Fault("class-constraint"))?
            .clone();
        let failure = if taking {
            RefinementFailure::AbstractUnfair { constraint }
        } else {
            RefinementFailure::Divergence { constraint }
        };
        let counterexample = |initial, prefix, lasso| TemporalCounterexample {
            initial_state: initial,
            prefix,
            over_binding: None,
            fairness: self
                .refinement
                .concrete_operation_fairness()
                .cloned()
                .collect(),
            kind: CounterexampleKind::Formula,
            lasso: Some(lasso),
            refinement: Some(failure.clone()),
            blocked: Vec::new(),
            final_delay: None,
        };
        let places = run.timed.places().len();
        let mut stem = path_to(live, entry);
        if live.nodes[entry].tail {
            // The idle tail: the stem to the quiescent state, then its
            // stutter forever (FR-236).
            let tail = stem
                .iter()
                .position(|&arc| live.arcs[arc].via == Via::Stutter)
                .ok_or(ZoneCheckRefusal::Fault("idle-entry"))?;
            // With no step before the tail, the quiescent state is initial.
            let quiescent = live.arcs[stem[tail]].from;
            let (initial, stages) = stages_along(run, live, &stem[..tail], quiescent)?;
            let Ok(concrete) = concretise(run.timed, places, &stages, &Target::Any, &run.scale)
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
        // The loop starts at a position: the entry when a position enters
        // it, otherwise just after the cycle's first step.
        let mut looped: Vec<usize> = if live.nodes[entry].positioned {
            cycle.to_vec()
        } else {
            let first = cycle
                .iter()
                .position(|&arc| matches!(live.arcs[arc].via, Via::Step { .. }))
                .ok_or(ZoneCheckRefusal::Fault("loop-position"))?;
            stem.extend_from_slice(&cycle[..=first]);
            cycle[first + 1..]
                .iter()
                .chain(&cycle[..=first])
                .copied()
                .collect()
        };
        // The loop rolled back over the stem while the stem's last step is
        // the loop's last, from a position of equal state, refinement
        // position and cell: the shortest stem of the same loop.
        while let (Some(&last_stem), Some(&last_loop)) = (stem.last(), looped.last()) {
            let (left, right) = (&live.arcs[last_stem], &live.arcs[last_loop]);
            let (from, to) = (&live.nodes[left.from], &live.nodes[right.from]);
            let same = matches!(left.via, Via::Step { .. })
                && left.via == right.via
                && left.to == right.to
                && from.positioned
                && from.q == to.q
                && from.cell == to.cell
                && run.key(&from.state)? == run.key(&to.state)?;
            if !same {
                break;
            }
            stem.pop();
            looped.pop();
            looped.insert(0, last_stem);
        }
        let start = live.arcs[looped[0]].from;
        let (initial, stem_stages) = stages_along(run, live, &stem, start)?;
        let mut cycle_stages = Vec::with_capacity(looped.len());
        for &arc in &looped {
            if live.arcs[arc].via != Via::Diverge {
                cycle_stages.push(stage(run, live, arc)?);
            }
        }
        let ceiling = run.timed.max_constant();
        Ok(run
            .timed_lasso(places, &ceiling, stem_stages, &cycle_stages)?
            .map(|(prefix, entry)| {
                counterexample(
                    initial,
                    prefix,
                    Lasso {
                        entry,
                        terminal_stutter: false,
                    },
                )
            }))
    }
}

/// Store `node` unless one with equal state, position, cell and flags
/// holds an aM-equivalent zone; the new vertex, or `None` with `arc`
/// recorded to the equivalent one.
fn add_node(
    run: &mut Run<'_, '_, '_>,
    live: &mut Live,
    node: Node,
    arc: Option<Arc>,
) -> Result<Option<usize>, Halt> {
    let mut key = run.key(&node.state)?;
    key.extend_from_slice(&widen(node.q).to_be_bytes());
    key.extend(node.cell.iter().map(|sign| *sign as u8));
    key.push(u8::from(node.positioned));
    key.push(u8::from(node.divergent));
    key.push(u8::from(node.tail));
    if let Some(stored) = live.passed.get(&key) {
        for &candidate in stored {
            let held = &live.nodes[candidate].zone;
            let same = node.zone.alu_covered_by(held, &live.bounds, &live.bounds)?
                && held.alu_covered_by(&node.zone, &live.bounds, &live.bounds)?;
            if same {
                if let Some(arc) = arc {
                    add_arc(
                        run,
                        live,
                        Arc {
                            to: candidate,
                            ..arc
                        },
                    )?;
                }
                return Ok(None);
            }
        }
    }
    run.charge_vertex(zone_bytes(&node.zone))?;
    run.escape_state();
    let index = live.nodes.len();
    live.nodes.push(node);
    live.out.push(Vec::new());
    live.parent.push(None);
    live.passed.entry(key).or_default().push(index);
    if let Some(arc) = arc {
        add_arc(run, live, Arc { to: index, ..arc })?;
        live.parent[index] = Some(live.arcs.len() - 1);
    }
    Ok(Some(index))
}

fn add_arc(run: &mut Run<'_, '_, '_>, live: &mut Live, arc: Arc) -> Result<(), Halt> {
    run.charge_edge()?;
    run.escape_edge();
    live.out[arc.from].push(live.arcs.len());
    live.arcs.push(arc);
    Ok(())
}

/// The arcs from an initial vertex to `node`.
fn path_to(live: &Live, node: usize) -> Vec<usize> {
    let mut path = Vec::new();
    let mut at = node;
    while let Some(arc) = live.parent[at] {
        path.push(arc);
        at = live.arcs[arc].from;
    }
    path.reverse();
    path
}

/// The initial state's index and the stages of the arcs `path`, which
/// starts at an initial vertex and ends at `end`; resets of `z` take no
/// stage.
fn stages_along(
    run: &mut Run<'_, '_, '_>,
    live: &mut Live,
    path: &[usize],
    end: usize,
) -> Result<(usize, Vec<Stage>), Halt> {
    let root = path.first().map_or(end, |&arc| live.arcs[arc].from);
    let initial = live.nodes[root]
        .initial
        .ok_or(ZoneCheckRefusal::Fault("path-root"))?;
    let mut stages = Vec::with_capacity(path.len());
    for &arc in path {
        if live.arcs[arc].via != Via::Diverge {
            stages.push(stage(run, live, arc)?);
        }
    }
    Ok((initial, stages))
}

/// The stage of `arc`, a step or a delay into the next cell.
fn stage(run: &mut Run<'_, '_, '_>, live: &mut Live, arc: usize) -> Result<Stage, Halt> {
    let arc = live.arcs[arc].clone();
    let source = live.nodes[arc.from].clone();
    let target = live.nodes[arc.to].clone();
    let invariants = run.timed(run.timed.invariants(&source.state))?;
    let elapses = run.elapses(&source.state)?;
    Ok(match arc.via {
        Via::Step { step, part } => {
            let steps = run.steps_for(&source.state)?;
            let step = steps
                .get(step)
                .cloned()
                .ok_or(ZoneCheckRefusal::Fault("edge-step"))?;
            let post = step.post().clone();
            Stage {
                invariants,
                elapses,
                target: run.timed(run.timed.invariants(&post))?,
                before: run.cell_zone(&mut live.cells, &source.state, &source.cell)?,
                piece: run.cell_zone(&mut live.cells, &post, &target.cell)?,
                step: Some((step, part)),
                position: Vec::new(),
                position_resets: Vec::new(),
                silent: false,
                fixed: None,
            }
        }
        Via::Wait => Stage {
            invariants: invariants.clone(),
            elapses,
            target: invariants,
            before: None,
            piece: run.cell_zone(&mut live.cells, &target.state, &target.cell)?,
            step: None,
            position: Vec::new(),
            position_resets: Vec::new(),
            silent: true,
            fixed: None,
        },
        Via::Diverge | Via::Stutter => return Err(ZoneCheckRefusal::Fault("stage-edge").into()),
    })
}

fn fairness(run: &Run<'_, '_, '_>, error: FairnessError) -> Halt {
    match error {
        FairnessError::Cancelled => run.stop(crate::zone_check::search::ZoneStop::Cancelled),
        _ => ZoneCheckRefusal::Fault("fairness-graph").into(),
    }
}
