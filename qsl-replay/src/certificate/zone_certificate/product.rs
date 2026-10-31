// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-245 over FR-240's liveness product: the checker's own reading of a
//! CF-2 certificate (ADR-026 CF-2).
//!
//! The checker rebuilds the clause's negation automaton with qsl-eval's
//! translation and recomputes every product successor of a node itself:
//!
//! - a position: each step's convex part from the node's zone, its resets,
//!   the target's invariants, split into the target state's **cells** (the
//!   constants of the fairness-constrained guards there, each clock below,
//!   at or above each) and let time elapse within its cell; the claim
//!   automaton reads the target's letter;
//! - under a dense source, a delay into each next cell, and the reset of
//!   the divergence clock `z` once it has reached 1;
//! - an idle-tail stutter at a quiescent (dense) or terminal (tick) state,
//!   after which the behaviour takes no step.
//!
//! A successor matches an edge by its kind, step and part, post-state,
//! automaton state, instance and entry marks, and its target's zone covers
//! it and lies in its cell. Every node's zone thus lies in one cell, so a
//! constrained step is enabled at every valuation of a node or at none, and
//! the aLU simulation, under bounds that reach every cell constant from
//! both sides, keeps each valuation's cell: a fair accepting run of the
//! timed product maps to a path of certificate edges through nodes of its
//! own cells, automaton states and marks. Each component's reason is then
//! re-derived from the nodes and edges: that no fair accepting cycle lies
//! in it.
//!
//! A reachable position whose letter is undefined is rejected `BadState`:
//! the checker does not decide whether such a position diverges.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::automaton::{Atom, AutomatonError, ClockedEdge, Letter, PositionAutomaton, StateId};
use qsl_eval::model_subject::{ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{GuardedStep, ModelSimState, PlaceConstraint};
use qsl_eval::temporal::{AtomReading, CaptureReading};
use qsl_semantics::check::CaptureSite;
use quire_exact::{Integer, Meter, Rational};
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity, FairnessKind};
use quire_semantic_value::model_transition::ModelTransition;

use super::{
    at_node, fault, reject, undecided, AcceptanceSet, CertificateLocus, CertificateRule, Checker,
    CheckerFault, ComponentReason, Components, FairnessMember, ProductMark, SilentMove, Step,
    Successor,
};
use crate::certificate::zone::Zone;

/// A clause decided on the liveness product, as the checker reads it.
pub(super) struct LiveClaim<'c> {
    pub(super) automaton: RefCell<PositionAutomaton>,
    pub(super) start: StateId,
    pub(super) letters: &'c dyn ModelLetters,
    pub(super) instances: Vec<ModelInstance>,
    pub(super) capture: CaptureSite,
    pub(super) fairness: Vec<FairnessConstraint>,
    /// Each fairness constraint's operation text.
    pub(super) operations: Vec<String>,
    /// Each letter read, by the state's state key, the instance and whether
    /// it is position 0 of an `on origin` clause with captures; `None` when
    /// undefined.
    readings: RefCell<BTreeMap<(qsl_eval::simulation::StateKey, usize, bool), Option<Letter>>>,
}

/// Where a clock lies against one cell constant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Sign {
    Below,
    At,
    Above,
}

/// A cell constant: the zone clock and the scaled constant.
type CellAtom = (usize, Integer);

impl<'c> LiveClaim<'c> {
    pub(super) fn new(
        automaton: PositionAutomaton,
        start: StateId,
        letters: &'c dyn ModelLetters,
        instances: Vec<ModelInstance>,
        capture: CaptureSite,
        fairness: Vec<FairnessConstraint>,
        operations: Vec<String>,
    ) -> Self {
        Self {
            automaton: RefCell::new(automaton),
            start,
            letters,
            instances,
            capture,
            fairness,
            operations,
            readings: RefCell::new(BTreeMap::new()),
        }
    }
}

/// The checker's automaton reads: its state limit stops the check; any
/// other refusal is a fault of the checker's own.
fn automaton<T, O>(result: Result<T, AutomatonError>) -> Step<T, O> {
    result.map_err(|error| match error {
        AutomatonError::StateLimit { limit } => super::Halt::Verdict(super::automaton_stop(limit)),
        _ => fault(CheckerFault::Automaton),
    })
}

impl<O> Checker<'_, '_, O> {
    fn live(&self) -> Step<&LiveClaim<'_>, O> {
        self.live.as_ref().ok_or(fault(CheckerFault::Automaton))
    }

    /// The divergence clock's index in a zone, under a dense source: the
    /// last, after the clock places and the automaton's clocks.
    pub(super) fn z_clock(&self) -> usize {
        self.dim() - 1
    }

    /// The number of the claim automaton's clocks.
    pub(super) fn automaton_clocks(&self) -> usize {
        self.live
            .as_ref()
            .map_or(0, |live| live.automaton.borrow().clock_count())
    }

    /// `zone` read by the claim-automaton transition `read` at a position:
    /// held to its clock guard, then its clocks reset (FR-242); the
    /// automaton's clocks follow the clock places.
    pub(super) fn read_at(&self, zone: &mut Zone, read: &ClockedEdge) -> Step<(), O> {
        let base = self.timed.places().len();
        let guard: Vec<PlaceConstraint> = read
            .guard
            .iter()
            .map(|constraint| {
                PlaceConstraint::new(
                    base + constraint.place(),
                    constraint.op(),
                    constraint.bound().clone(),
                )
            })
            .collect();
        self.constrain(zone, &guard)?;
        if zone.is_empty() {
            return Ok(());
        }
        for &clock in &read.resets {
            zone.reset(base + 1 + clock, &Integer::zero())?;
        }
        Ok(())
    }

    /// The valuations from which `read` leads into `zone`: the inverse of
    /// [`Self::read_at`].
    pub(super) fn read_before(&self, zone: &mut Zone, read: &ClockedEdge) -> Step<(), O> {
        let base = self.timed.places().len();
        for &clock in &read.resets {
            zone.constrain(base + 1 + clock, 0, &Integer::zero(), false)?;
            zone.free(base + 1 + clock)?;
        }
        let guard: Vec<PlaceConstraint> = read
            .guard
            .iter()
            .map(|constraint| {
                PlaceConstraint::new(
                    base + constraint.place(),
                    constraint.op(),
                    constraint.bound().clone(),
                )
            })
            .collect();
        self.constrain(zone, &guard)
    }

    /// The letter instance `instance` reads at `state`, `initial` at
    /// position 0; `None` when a condition is undefined there.
    fn letter(
        &self,
        instance: usize,
        state: &ModelSimState,
        initial: bool,
    ) -> Step<Option<Letter>, O> {
        let live = self.live()?;
        let state_key = self.state_key(state)?;
        let key = (
            state_key.clone(),
            instance,
            initial && live.capture == CaptureSite::Origin,
        );
        if let Some(known) = live.readings.borrow().get(&key) {
            return Ok(*known);
        }
        let binding = live
            .instances
            .get(instance)
            .ok_or(fault(CheckerFault::UnreachedNode))?;
        let limits = self.timed.system().meter_limits();
        let mut letter = Letter::default();
        let when = live.automaton.borrow().when();
        let mut defined = true;
        if let Some(when) = when {
            match live
                .letters
                .activation(binding, state, &mut Meter::new(limits))
                .or_else(undecided)?
            {
                AtomReading::True => letter = letter.with(when),
                AtomReading::False => {}
                AtomReading::Undefined(_) => defined = false,
            }
        }
        let capturing = match live.capture {
            CaptureSite::Never => false,
            CaptureSite::Origin => initial,
            CaptureSite::Activations => when.is_none_or(|when| letter.holds(when)),
        };
        if defined && capturing {
            if let CaptureReading::Undefined { .. } = live
                .letters
                .captures(binding, state, &mut Meter::new(limits))
                .or_else(undecided)?
            {
                defined = false;
            }
        }
        let atoms = live.automaton.borrow().atoms().to_vec();
        for (index, atom) in atoms.into_iter().enumerate() {
            if !defined {
                break;
            }
            match live
                .letters
                .atom(atom, binding, state, &mut Meter::new(limits))
                .or_else(undecided)?
            {
                AtomReading::True => {
                    letter =
                        letter.with(Atom::new(index).map_err(|_| fault(CheckerFault::Automaton))?);
                }
                AtomReading::False => {}
                AtomReading::Undefined(_) => defined = false,
            }
        }
        let reading = defined.then_some(letter);
        live.readings.borrow_mut().insert(key, reading);
        Ok(reading)
    }

    /// The transitions `q` reads `letter` by; each target is the checker's
    /// index.
    fn reads(&self, q: usize, letter: Letter) -> Step<Vec<ClockedEdge>, O> {
        let live = self.live()?;
        let from = self.state_id(q)?;
        automaton(live.automaton.borrow_mut().successors(from, letter))
    }

    /// The checker's automaton state of index `q`, one it has reached.
    fn state_id(&self, q: usize) -> Step<StateId, O> {
        self.live()?
            .automaton
            .borrow()
            .reached_state(q)
            .ok_or(fault(CheckerFault::Automaton))
    }

    /// The product's initial nodes' zones and marks for each initial state:
    /// position 0 read at time 0, split into cells.
    #[allow(clippy::type_complexity, reason = "each start's successors")]
    pub(super) fn live_initial(
        &self,
        state: &ModelSimState,
    ) -> Step<Option<Vec<(usize, usize, ProductMark, Zone)>>, O> {
        let live = self.live()?;
        let mut found = Vec::new();
        for instance in 0..live.instances.len() {
            let Some(letter) = self.letter(instance, state, true)? else {
                return Ok(None);
            };
            for read in self.reads(live.start.index(), letter)? {
                let mut zero = Zone::zero(self.dim());
                self.read_at(&mut zero, &read)?;
                if zero.is_empty() {
                    continue;
                }
                for (cell, piece) in self.cells(state, &zero)? {
                    let Some(zone) = self.close(state, piece, &cell)? else {
                        continue;
                    };
                    let mark = ProductMark {
                        instance,
                        positioned: true,
                        divergent: false,
                        tail: false,
                    };
                    found.push((instance, read.target.index(), mark, zone));
                }
            }
        }
        Ok(Some(found))
    }

    /// FR-232's enabledness of `step` over clocks: each convex part of its
    /// guard with the target's time invariants on the clocks it does not
    /// reset; none when a reset value breaks a target bound.
    fn enabling_parts(&self, step: &GuardedStep) -> Step<Vec<Vec<PlaceConstraint>>, O> {
        let target = self.invariants(step.post())?;
        let mut pulled_back = Vec::with_capacity(target.len());
        for bound in target {
            if let Some((_, value)) = step
                .resets()
                .iter()
                .find(|(place, _)| *place == bound.place())
            {
                if !bound.op().holds(value, bound.bound()) {
                    return Ok(Vec::new());
                }
                continue;
            }
            let place = step
                .carries()
                .iter()
                .find(|(target, _)| *target == bound.place())
                .map_or(bound.place(), |(_, source)| *source);
            pulled_back.push(PlaceConstraint::new(
                place,
                bound.op(),
                bound.bound().clone(),
            ));
        }
        Ok(step
            .parts()
            .iter()
            .map(|part| part.iter().chain(&pulled_back).cloned().collect())
            .collect())
    }

    /// The cell constants of `state`: each clock constraint of a
    /// fairness-constrained step's enabledness there (FR-232).
    fn atoms(&self, state: &ModelSimState) -> Step<Vec<CellAtom>, O> {
        let live = self.live()?;
        let mut atoms = Vec::new();
        for step in self.timed.guarded_steps(state).or_else(undecided)? {
            if !live.operations.iter().any(|text| text == step.operation()) {
                continue;
            }
            for part in self.enabling_parts(&step)? {
                for constraint in &part {
                    atoms.push((constraint.place() + 1, self.scaled(constraint.bound())?));
                }
            }
        }
        atoms.sort();
        atoms.dedup();
        Ok(atoms)
    }

    /// `zone` split into the cells of `state`.
    fn cells(&self, state: &ModelSimState, zone: &Zone) -> Step<Vec<(Vec<Sign>, Zone)>, O> {
        let mut pieces = vec![(Vec::new(), zone.clone())];
        for atom in &self.atoms(state)? {
            let mut split = Vec::new();
            for (signs, piece) in pieces {
                for sign in [Sign::Below, Sign::At, Sign::Above] {
                    let mut part = piece.clone();
                    hold(&mut part, atom, sign)?;
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

    /// The zone of `cell` at `state`, over every clock of a node.
    fn cell_zone(&self, state: &ModelSimState, cell: &[Sign]) -> Step<Zone, O> {
        let mut zone = Zone::universe(self.dim());
        for (atom, sign) in self.atoms(state)?.iter().zip(cell) {
            hold(&mut zone, atom, *sign)?;
        }
        Ok(zone)
    }

    /// The one cell of `state` that holds `zone`; `None` when it straddles
    /// a cell constant.
    fn cell_of(&self, state: &ModelSimState, zone: &Zone) -> Step<Option<Vec<Sign>>, O> {
        let mut cell = Vec::new();
        for atom in &self.atoms(state)? {
            let mut sign = None;
            for candidate in [Sign::Below, Sign::At, Sign::Above] {
                let mut part = Zone::universe(zone.dim());
                hold(&mut part, atom, candidate)?;
                if part.includes(zone)? {
                    sign = Some(candidate);
                }
            }
            let Some(sign) = sign else {
                return Ok(None);
            };
            cell.push(sign);
        }
        Ok(Some(cell))
    }

    /// The zone of the one cell of `state` that holds `zone`.
    pub(super) fn cell_zone_of(&self, state: &ModelSimState, zone: &Zone) -> Step<Zone, O> {
        match self.cell_of(state, zone)? {
            Some(cell) => self.cell_zone(state, &cell),
            None => Err(fault(CheckerFault::ZoneShape)),
        }
    }

    /// `piece` held to `state`'s invariants and `cell`, time let elapse
    /// within them; `None` when empty.
    fn close(
        &self,
        state: &ModelSimState,
        mut piece: Zone,
        cell: &[Sign],
    ) -> Step<Option<Zone>, O> {
        let cell = self.cell_zone(state, cell)?;
        self.settle_within(state, &mut piece, Some(&cell))?;
        Ok((!piece.is_empty()).then_some(piece))
    }

    /// `zone` within `state`'s invariants and `cell`, time let elapse
    /// within them unless the state is urgent.
    pub(super) fn settle_within(
        &self,
        state: &ModelSimState,
        zone: &mut Zone,
        cell: Option<&Zone>,
    ) -> Step<(), O> {
        let invariants = self.invariants(state)?;
        let hold = |zone: &mut Zone| -> Step<(), O> {
            self.constrain(zone, &invariants)?;
            if let Some(cell) = cell {
                super::intersect(zone, cell)?;
            }
            Ok(())
        };
        hold(zone)?;
        if !zone.is_empty() && self.elapses(state)? {
            zone.up();
            hold(zone)?;
        }
        Ok(())
    }

    /// Whether a behaviour may idle forever at `state`: under a dense
    /// source a quiescent state where time elapses, under a tick source
    /// one that enables no step.
    fn idles_at(&self, state: &ModelSimState) -> Step<bool, O> {
        Ok(match self.timed.tick_period() {
            Some(_) => self
                .timed
                .guarded_steps(state)
                .or_else(undecided)?
                .is_empty(),
            None => self.timed.is_quiescent(state).or_else(undecided)? && self.elapses(state)?,
        })
    }

    /// FR-240's product successors of node `node`, at `state` with
    /// automaton state `q`, mark `mark` and `zone`.
    pub(super) fn live_successors(
        &self,
        node: usize,
        state: &ModelSimState,
        q: usize,
        mark: ProductMark,
        zone: &Zone,
    ) -> Step<Vec<Successor>, O> {
        let mut zone = zone.clone();
        self.constrain(&mut zone, &self.invariants(state)?)?;
        let mut found = Vec::new();
        if zone.is_empty() {
            return Ok(found);
        }
        let Some(own) = self.cell_of(state, &zone)? else {
            return reject(CertificateRule::SuccessorMissing, at_node(node));
        };
        let dense = self.timed.tick_period().is_none();
        let state_key = self.state_key(state)?;
        let instance = mark.instance;
        if !mark.tail {
            for step in self.timed.guarded_steps(state).or_else(undecided)? {
                let target = self.invariants(step.post())?;
                let post_state_key = self.state_key(step.post())?;
                let tick =
                    self.timed.tick_of(step.operation()) > Rational::from_integer(Integer::zero());
                for (part, constraints) in step.parts().iter().enumerate() {
                    let mut next = zone.clone();
                    self.constrain(&mut next, constraints)?;
                    if next.is_empty() {
                        continue;
                    }
                    self.apply_clock_updates(&mut next, &step)?;
                    self.constrain(&mut next, &target)?;
                    if next.is_empty() {
                        continue;
                    }
                    let Some(letter) = self.letter(instance, step.post(), false)? else {
                        return reject(CertificateRule::BadState, at_node(node));
                    };
                    for read in self.reads(q, letter)? {
                        let mut read_zone = next.clone();
                        self.read_at(&mut read_zone, &read)?;
                        if read_zone.is_empty() {
                            continue;
                        }
                        for (cell, piece) in self.cells(step.post(), &read_zone)? {
                            let Some(entered) = self.close(step.post(), piece, &cell)? else {
                                continue;
                            };
                            found.push(Successor {
                                step: Some(step.clone()),
                                part: Some(part),
                                horizon: None,
                                frees: false,
                                state_key: post_state_key.clone(),
                                q: read.target.index(),
                                zone: entered,
                                silent: None,
                                mark: Some(ProductMark {
                                    instance,
                                    positioned: true,
                                    divergent: !dense && tick,
                                    tail: false,
                                }),
                                cell: Some(self.cell_zone(step.post(), &cell)?),
                                read: Some(read.clone()),
                            });
                        }
                    }
                }
            }
        }
        if dense && self.elapses(state)? {
            let mut later = zone.clone();
            later.up();
            self.constrain(&mut later, &self.invariants(state)?)?;
            for (cell, piece) in self.cells(state, &later)? {
                if !next_cell(&own, &cell) {
                    continue;
                }
                let Some(entered) = self.close(state, piece, &cell)? else {
                    continue;
                };
                found.push(Successor {
                    step: None,
                    part: None,
                    horizon: None,
                    frees: false,
                    state_key: state_key.clone(),
                    q,
                    zone: entered,
                    silent: Some(SilentMove::Wait),
                    mark: Some(ProductMark {
                        instance,
                        positioned: false,
                        divergent: false,
                        tail: mark.tail,
                    }),
                    cell: Some(self.cell_zone(state, &cell)?),
                    read: None,
                });
            }
        }
        if dense {
            let z = self.z_clock();
            let mut reset = zone.clone();
            reset.constrain(0, z, &self.certificate.scale.neg(), false)?;
            if !reset.is_empty() {
                reset.reset(z, &Integer::zero())?;
                if let Some(entered) = self.close(state, reset, &own)? {
                    found.push(Successor {
                        step: None,
                        part: None,
                        horizon: None,
                        frees: false,
                        state_key: state_key.clone(),
                        q,
                        zone: entered,
                        silent: Some(SilentMove::Diverge),
                        mark: Some(ProductMark {
                            instance,
                            positioned: false,
                            divergent: true,
                            tail: mark.tail,
                        }),
                        cell: Some(self.cell_zone(state, &own)?),
                        read: None,
                    });
                }
            }
        }
        if mark.tail || self.idles_at(state)? {
            let Some(letter) = self.letter(instance, state, false)? else {
                return reject(CertificateRule::BadState, at_node(node));
            };
            for read in self.reads(q, letter)? {
                let mut read_zone = zone.clone();
                self.read_at(&mut read_zone, &read)?;
                if read_zone.is_empty() {
                    continue;
                }
                if let Some(entered) = self.close(state, read_zone, &own)? {
                    found.push(Successor {
                        step: None,
                        part: None,
                        horizon: None,
                        frees: false,
                        state_key: state_key.clone(),
                        q: read.target.index(),
                        zone: entered,
                        silent: None,
                        mark: Some(ProductMark {
                            instance,
                            positioned: true,
                            divergent: !dense,
                            tail: true,
                        }),
                        cell: Some(self.cell_zone(state, &own)?),
                        read: Some(read),
                    });
                }
            }
        }
        Ok(found)
    }

    /// FR-244's Büchi part: every edge goes to an equal or greater
    /// component, and each component's reason holds.
    pub(super) fn components(&self) -> Step<(), O> {
        let Some(components) = &self.certificate.components else {
            return super::malformed(super::CertificateMember::Components);
        };
        for node in 0..self.certificate.nodes.len() {
            if self.states[node].is_some() && components.numbering[node].is_none() {
                return super::malformed(super::CertificateMember::Components);
            }
        }
        for (index, edge) in self.certificate.edges.iter().enumerate() {
            if self.covers[index].is_none() {
                continue;
            }
            if let (Some(from), Some(to)) = (
                components.numbering[edge.from],
                components.numbering[edge.to],
            ) {
                if to < from {
                    return reject(
                        CertificateRule::BackwardEdge,
                        CertificateLocus::ZoneEdge {
                            edge: u64::try_from(index).unwrap_or(u64::MAX),
                        },
                    );
                }
            }
        }
        self.reasons(components)
    }

    /// Each reason of `components` holds, a strong reason's nested
    /// numbering read before the next reason; a failure is located at the
    /// component, or, inside a nested numbering, at the outermost component
    /// enclosing it. Read without recursion, however deep the nesting
    /// (SR-1647001 FND-001).
    fn reasons(&self, components: &Components) -> Step<(), O> {
        // Each frame: a numbering, its outermost enclosing component, and
        // its next reason.
        let mut frames: Vec<(&Components, Option<usize>, usize)> = vec![(components, None, 0)];
        while let Some(frame) = frames.last_mut() {
            let (components, outer, index) = *frame;
            let Some(reason) = components.reasons.get(index) else {
                frames.pop();
                continue;
            };
            frame.2 += 1;
            self.poll()?;
            let at = outer.unwrap_or(index);
            let fails = || {
                reject(
                    CertificateRule::WitnessFails,
                    CertificateLocus::ZoneComponent {
                        component: u64::try_from(at).unwrap_or(u64::MAX),
                    },
                )
            };
            let members: BTreeSet<usize> = (0..self.certificate.nodes.len())
                .filter(|&node| {
                    self.states[node].is_some() && components.numbering[node] == Some(index)
                })
                .collect();
            let inside: Vec<usize> = members
                .iter()
                .flat_map(|&node| self.leaving[node].iter().copied())
                .filter(|&edge| {
                    self.covers[edge].is_some()
                        && members.contains(&self.certificate.edges[edge].to)
                })
                .collect();
            let holds = match reason {
                ComponentReason::Acyclic => inside.is_empty(),
                ComponentReason::Lacks(set) => {
                    let mut lacks = true;
                    for &member in &members {
                        if self.in_set(member, *set)? {
                            lacks = false;
                            break;
                        }
                    }
                    lacks
                }
                ComponentReason::Weak(member) => {
                    self.kind_of(member, FairnessKind::Weak)?
                        && !self.taken(member, &inside)?
                        && self.everywhere(member, &members)?
                }
                ComponentReason::Strong {
                    constraints,
                    nested,
                } => {
                    let mut holds = !constraints.is_empty();
                    for member in constraints {
                        holds &= self.kind_of(member, FairnessKind::Strong)?
                            && !self.taken(member, &inside)?;
                    }
                    if holds {
                        holds = self.nested(&members, constraints, nested)?;
                    }
                    if holds {
                        frames.push((nested, Some(at), 0));
                    }
                    holds
                }
            };
            if !holds {
                return fails();
            }
        }
        Ok(())
    }

    /// A strong reason's nested numbering: it numbers exactly the members
    /// of the component that enable none of `constraints` at any valuation,
    /// and every edge among them goes to an equal or greater index.
    fn nested(
        &self,
        members: &BTreeSet<usize>,
        constraints: &[FairnessMember],
        nested: &Components,
    ) -> Step<bool, O> {
        for node in 0..self.certificate.nodes.len() {
            let numbered = nested.numbering[node].is_some();
            if !members.contains(&node) {
                if numbered {
                    return Ok(false);
                }
                continue;
            }
            let mut enables = false;
            for member in constraints {
                enables |= self.somewhere(member, node)?;
            }
            if enables == numbered {
                return Ok(false);
            }
        }
        for (index, edge) in self.certificate.edges.iter().enumerate() {
            if self.covers[index].is_none() {
                continue;
            }
            if let (Some(from), Some(to)) = (nested.numbering[edge.from], nested.numbering[edge.to])
            {
                if to < from {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// Whether node `node` is in acceptance set `set`.
    fn in_set(&self, node: usize, set: AcceptanceSet) -> Step<bool, O> {
        let listed = &self.certificate.nodes[node];
        let mark = listed.product.ok_or(fault(CheckerFault::UnreachedNode))?;
        Ok(match set {
            AcceptanceSet::Divergence => mark.divergent,
            AcceptanceSet::Position => mark.positioned,
            AcceptanceSet::Automaton(index) => {
                let live = self.live()?;
                let q = self
                    .bound
                    .get(&listed.automaton_state)
                    .copied()
                    .ok_or(fault(CheckerFault::UnreachedNode))?;
                let state = self.state_id(q)?;
                let automaton = live.automaton.borrow();
                // A set the automaton does not have holds every node.
                index >= automaton.acceptance_set_count() || automaton.is_accepting_in(index, state)
            }
        })
    }

    /// Whether `member` names a constraint of the clause of kind `kind`
    /// and its granularity.
    fn kind_of(&self, member: &FairnessMember, kind: FairnessKind) -> Step<bool, O> {
        let live = self.live()?;
        let Some(constraint) = live.fairness.get(member.constraint) else {
            return Ok(false);
        };
        let shaped = match constraint.granularity() {
            FairnessGranularity::Whole => member.transition.is_none(),
            FairnessGranularity::Each => member.transition.is_some(),
        };
        Ok(shaped && constraint.kind() == kind)
    }

    /// Whether `transition` is one of `member`'s.
    fn names(&self, member: &FairnessMember, transition: &ModelTransition) -> Step<bool, O> {
        let live = self.live()?;
        let Some(text) = live.operations.get(member.constraint) else {
            return Ok(false);
        };
        Ok(transition.operation() == text
            && member
                .transition
                .as_ref()
                .is_none_or(|named| named == transition))
    }

    /// Whether an edge of `edges` takes `member`.
    fn taken(&self, member: &FairnessMember, edges: &[usize]) -> Step<bool, O> {
        for &edge in edges {
            if let Some(step) = &self.covered(edge)?.step {
                if self.names(member, step.transition())? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// `member`'s steps at node `node`'s state and the node's zone, held to
    /// its invariants.
    fn member_steps(
        &self,
        member: &FairnessMember,
        node: usize,
    ) -> Step<(Vec<GuardedStep>, Zone), O> {
        let state = self.state(node)?;
        let mut zone = self.certificate.nodes[node].zone.clone();
        self.constrain(&mut zone, &self.invariants(state)?)?;
        let mut steps = Vec::new();
        for step in self.timed.guarded_steps(state).or_else(undecided)? {
            if self.names(member, step.transition())? {
                steps.push(step);
            }
        }
        Ok((steps, zone))
    }

    /// Whether `member` is enabled at every valuation of every node of
    /// `nodes` (FR-232): some enabling part of one of its steps holds the
    /// node's zone.
    fn everywhere(&self, member: &FairnessMember, nodes: &BTreeSet<usize>) -> Step<bool, O> {
        for &node in nodes {
            let (steps, zone) = self.member_steps(member, node)?;
            let mut enabled = false;
            for step in &steps {
                for part in self.enabling_parts(step)? {
                    let mut holds = Zone::universe(zone.dim());
                    self.constrain(&mut holds, &part)?;
                    enabled |= holds.includes(&zone)?;
                }
            }
            if !enabled {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Whether `member` is enabled at some valuation of node `node`.
    fn somewhere(&self, member: &FairnessMember, node: usize) -> Step<bool, O> {
        let (steps, zone) = self.member_steps(member, node)?;
        for step in &steps {
            for part in self.enabling_parts(step)? {
                let mut at = zone.clone();
                self.constrain(&mut at, &part)?;
                if !at.is_empty() {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

/// Hold `zone` to `atom`'s side `sign`.
fn hold(
    zone: &mut Zone,
    (clock, value): &CellAtom,
    sign: Sign,
) -> Result<(), crate::certificate::zone::ZoneError> {
    match sign {
        Sign::Below => zone.constrain(*clock, 0, value, true),
        Sign::At => {
            zone.constrain(*clock, 0, value, false)?;
            zone.constrain(0, *clock, &value.neg(), false)
        }
        Sign::Above => zone.constrain(0, *clock, &value.neg(), true),
    }
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
