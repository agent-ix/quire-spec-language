// SPDX-License-Identifier: AGPL-3.0-or-later
//! The property-automaton translation (ADR-018 SM-6, IV-3): a checked
//! temporal clause becomes a [`Gba`] over the letter of its atoms.
//!
//! A state of the automaton is a [`Config`]: the formulas the next position
//! must satisfy, one counter per interval-operator occurrence (IV-3), the
//! pending shifts of operators with a lower bound, and the memory of every
//! past operator. Reading a letter expands the formulas the next position
//! must satisfy into guarded successors, in the manner of a tableau:
//!
//! - an unbounded `eventually` or `until` either holds its witness now or
//!   promises itself to the next position, and each has an acceptance set of
//!   the states that promise nothing;
//! - an interval operator keeps its counter instead of a promise per pending
//!   next-position step. A counter whose operand is claimed at a position
//!   asserts the operand there, so an operand that holds a temporal operator
//!   is a promise of its own;
//! - a past operator is tracked as memory, never as a promise, which is why a
//!   past operator over a future operator is refused.
//!
//! A bounded formula and an infinite-trace safety formula translate to the
//! automaton of the formula itself, with no acceptance set; its bad-prefix
//! [`Monitor`](crate::automaton::Monitor) is the deterministic monitor
//! (SM-6). A liveness formula translates to the automaton of its negation.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_semantics::check::{Activation, CheckedTemporalClause, FormulaNodeId, TemporalProfile};

use crate::automaton::counters::{DeadlineCounter, ExtentCounter};
use crate::automaton::dnf::Dnf;
use crate::automaton::error::AutomatonError;
use crate::automaton::formula::{invariant, lower, FutureKind, Ir, Lowered, Node, NodeId, Window};
use crate::automaton::gba::{Gba, GbaBuilder};
use crate::automaton::letter::{Atom, Guard};
use crate::automaton::numbering::Numbering;
use crate::automaton::past::{Memory, Outcome, Past};
use crate::automaton::state::StateId;

/// Which representation ADR-018 SM-6 gives a property.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutomatonKind {
    /// A bounded-profile clause (TP-2): the automaton of the clause, whose
    /// bad-prefix monitor is deterministic and finite.
    Bounded,
    /// An infinite-trace safety formula (TP-1, TP-3): the automaton of the
    /// formula, with no acceptance set.
    Safety,
    /// Any other infinite-trace formula (TP-4): the automaton of its
    /// negation, which accepts exactly the behaviours that violate it.
    Liveness,
}

/// A clause translated to a property automaton, and the letter it reads.
///
/// The letter has one atom for each `holds` atom of the clause, in clause-node
/// order; then one for the `when` condition of an `on each` activation, if it
/// has one; then, for a bounded `on each` clause, one that is true at every
/// represented position and false at the positions the profile's false
/// extension adds (its activations stop at the end of the trace).
#[derive(Clone, Debug)]
pub struct PropertyAutomaton {
    kind: AutomatonKind,
    automaton: Gba,
    atoms: Vec<FormulaNodeId>,
    when: Option<Atom>,
    live: Option<Atom>,
}

impl PropertyAutomaton {
    /// Which representation the property got.
    pub fn kind(&self) -> AutomatonKind {
        self.kind
    }

    /// The automaton.
    pub fn automaton(&self) -> &Gba {
        &self.automaton
    }

    /// The `holds` atoms of the clause: atom `i` of the letter is
    /// `atoms()[i]`.
    pub fn atoms(&self) -> &[FormulaNodeId] {
        &self.atoms
    }

    /// The atom that holds where the activation's `when` condition holds.
    pub fn when(&self) -> Option<Atom> {
        self.when
    }

    /// The atom that holds at the represented positions of a bounded
    /// `on each` clause.
    pub fn live(&self) -> Option<Atom> {
        self.live
    }
}

/// Translate `clause`, numbering at most `max_states` automaton states
/// (`max_automaton_states`, ADR-018 IV-6). The numbering covers every
/// state reachable on some letter.
///
/// # Errors
///
/// [`AutomatonError::PastOverFuture`] for a past operator whose operand holds
/// a future operator; [`AutomatonError::ProfileNotTranslated`] for the
/// timestamped profile; [`AutomatonError::AtomOutOfRange`] when the letter
/// needs more atoms than it holds; [`AutomatonError::StateLimit`] when the
/// automaton has more than `max_states` states.
pub fn translate(
    clause: &CheckedTemporalClause,
    max_states: u64,
) -> Result<PropertyAutomaton, AutomatonError> {
    let profile = clause.profile();
    if matches!(profile, TemporalProfile::TimestampedEventFiniteWindow) {
        return Err(AutomatonError::ProfileNotTranslated { profile });
    }
    let bounded = profile.is_bounded();
    let Lowered {
        mut ir,
        atoms,
        positive,
        negative,
    } = lower(clause.formula())?;
    let mut next_atom = atoms.len();
    let mut take_atom = || {
        let atom = Atom::new(next_atom);
        next_atom += 1;
        atom
    };
    let (property, negation, when, live) = match clause.activation() {
        Activation::Origin => (positive, negative, None, None),
        Activation::Each { guarded } => {
            let when = if guarded { Some(take_atom()?) } else { None };
            let live = if bounded { Some(take_atom()?) } else { None };
            let mut guard = ir.constant(true);
            for atom in when.into_iter().chain(live) {
                let node = ir.atom(atom);
                guard = ir.and(guard, node)?;
            }
            let outside = ir.not(guard)?;
            let body = ir.or(outside, positive)?;
            let violation = ir.and(guard, negative)?;
            (
                ir.future(FutureKind::Always, Window::From { lower: 0 }, None, body),
                ir.future(
                    FutureKind::Eventually,
                    Window::From { lower: 0 },
                    None,
                    violation,
                ),
                when,
                live,
            )
        }
    };
    let (kind, target) = if bounded {
        (AutomatonKind::Bounded, property)
    } else if is_safety(&mut ir, property)? {
        (AutomatonKind::Safety, property)
    } else {
        (AutomatonKind::Liveness, negation)
    };
    let order = ir.reachable(target)?;
    let translator = Translator { ir: &ir, order };
    Ok(PropertyAutomaton {
        kind,
        automaton: translator.automaton(target, max_states)?,
        atoms,
        when,
        live,
    })
}

/// Whether the negation normal form of `root` has no unbounded `eventually`
/// or `until` (ADR-014 A-4 as amended by ADR-018 IV-4: `eventually[a,*]` and
/// `until[a,*]` are outside the safety fragment).
fn is_safety(ir: &mut Ir, root: NodeId) -> Result<bool, AutomatonError> {
    for id in ir.reachable(root)? {
        if let Node::Future {
            kind,
            window: Window::From { .. },
            ..
        } = ir.node(id)?
        {
            if kind.is_eventuality() {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// One counter of a closed interval operator with lower bound 0 (IV-3).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Counter {
    /// `eventually[0,b]` and `until[0,b]`: the earliest open deadline.
    Deadline(DeadlineCounter),
    /// `always[0,b]` and `release[0,b]`: the furthest required position.
    Extent(ExtentCounter),
}

impl Counter {
    fn is_pending(self) -> bool {
        match self {
            Self::Deadline(counter) => counter.is_pending(),
            Self::Extent(counter) => counter.is_pending(),
        }
    }
}

/// An automaton state.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Config {
    /// The formulas the next position must satisfy.
    must: BTreeSet<NodeId>,
    /// The counters that are not idle, by the occurrence they count.
    counters: BTreeMap<NodeId, Counter>,
    /// For an operator with a lower bound, keyed by its lower-bound-0 core,
    /// the positions after the next one at which it starts.
    shifts: BTreeMap<NodeId, BTreeSet<u64>>,
    /// The memory of every past operator.
    memory: BTreeMap<NodeId, Memory>,
}

/// What the next position inherits from the branch that read this one.
#[derive(Clone, Debug, Default)]
struct Next {
    must: BTreeSet<NodeId>,
    counters: BTreeMap<NodeId, Counter>,
    shifts: BTreeMap<NodeId, BTreeSet<u64>>,
}

#[derive(Clone, Copy, Debug)]
enum Task {
    /// The node must hold at this position.
    Assert(NodeId),
    /// The lower-bound-0 operator starts at this position.
    Activate(NodeId),
}

/// One way of satisfying what this position must satisfy.
#[derive(Clone, Debug)]
struct Branch {
    /// The letters of this branch.
    cube: Guard,
    todo: Vec<Task>,
    asserted: BTreeSet<NodeId>,
    activated: BTreeSet<NodeId>,
    /// The interval operators that start here.
    opens: BTreeSet<NodeId>,
    /// The counters already stepped.
    handled: BTreeSet<NodeId>,
    next: Next,
}

enum Progress {
    /// Everything this position must satisfy is decided.
    Finished(Branch),
    /// The branches that replace this one; none when it contradicts itself.
    Continue(Vec<Branch>),
}

/// The truth of every state formula at one position, given the memory.
struct Truths {
    conditions: Vec<Dnf>,
    outcomes: Vec<(NodeId, Vec<Outcome>)>,
}

impl Truths {
    fn of(&self, id: NodeId) -> Result<&Dnf, AutomatonError> {
        self.conditions
            .get(id.index())
            .ok_or_else(|| invariant("truth-in-arena"))
    }
}

struct Translator<'a> {
    ir: &'a Ir,
    /// The nodes the target reaches, in ascending order.
    order: Vec<NodeId>,
}

impl Translator<'_> {
    fn automaton(&self, root: NodeId, max_states: u64) -> Result<Gba, AutomatonError> {
        let initial = self.initial(root)?;
        let mut numbering = Numbering::new(max_states);
        let mut builder = GbaBuilder::new();
        let first = numbering.intern(initial.clone())?;
        let mut queue = VecDeque::from([(first.state, initial)]);
        while let Some((state, config)) = queue.pop_front() {
            for (guard, next) in self.expand(&config)? {
                let reached = numbering.intern(next.clone())?;
                if reached.created {
                    if builder.add_state() != reached.state {
                        return Err(invariant("states-numbered-in-order"));
                    }
                    queue.push_back((reached.state, next));
                }
                builder.add_transition(state, guard, reached.state)?;
            }
        }
        for eventuality in self.eventualities()? {
            let quiet: Vec<StateId> = (0..numbering.len())
                .map(StateId::new)
                .filter(|state| {
                    numbering
                        .key(*state)
                        .is_some_and(|config| !config.must.contains(&eventuality))
                })
                .collect();
            if quiet.len() < numbering.len() {
                builder.add_acceptance_set(quiet)?;
            }
        }
        Ok(builder.build())
    }

    /// The unbounded `eventually` and `until` cores: what a state may carry
    /// forever without the run being accepted.
    fn eventualities(&self) -> Result<BTreeSet<NodeId>, AutomatonError> {
        let mut cores = BTreeSet::new();
        for id in &self.order {
            if let Node::Future {
                kind,
                window: Window::From { .. },
                ..
            } = self.ir.node(*id)?
            {
                if kind.is_eventuality() {
                    cores.insert(self.ir.core(*id));
                }
            }
        }
        Ok(cores)
    }

    /// The state before position 0: `root` must hold there, and every past
    /// operator remembers the stationary history before it, in which every
    /// atom reads false (QSpec FR-092).
    fn initial(&self, root: NodeId) -> Result<Config, AutomatonError> {
        let mut values = vec![false; self.ir.len()];
        let mut memory = BTreeMap::new();
        for id in &self.order {
            if self.ir.has_future(*id) {
                continue;
            }
            let value_of = |operand: NodeId| values.get(operand.index()).copied().unwrap_or(false);
            let value = match self.ir.node(*id)? {
                Node::Constant(value) => value,
                Node::Atom(_) => false,
                Node::Not(operand) => !value_of(operand),
                Node::And(left, right) => value_of(left) && value_of(right),
                Node::Or(left, right) => value_of(left) || value_of(right),
                Node::Past {
                    kind,
                    window,
                    right,
                    ..
                } => {
                    let witness = value_of(right);
                    memory.insert(*id, Past::new(kind, window).stationary(witness));
                    witness
                }
                Node::Future { .. } => return Err(invariant("state-formula-has-no-future")),
            };
            if let Some(slot) = values.get_mut(id.index()) {
                *slot = value;
            }
        }
        Ok(Config {
            must: BTreeSet::from([root]),
            counters: BTreeMap::new(),
            shifts: BTreeMap::new(),
            memory,
        })
    }

    /// Every transition out of `config`: the guard on the letter read at the
    /// next position and the state it leads to.
    fn expand(&self, config: &Config) -> Result<Vec<(Guard, Config)>, AutomatonError> {
        let truths = self.truths(&config.memory)?;
        let memories = Self::memories(&truths);
        let mut transitions = Vec::new();
        for (cube, next) in self.branches(config, &truths)? {
            for (memory_cube, memory) in &memories {
                let guard = cube.conjoined(*memory_cube);
                if guard.is_satisfiable() {
                    transitions.push((
                        guard,
                        Config {
                            must: next.must.clone(),
                            counters: next.counters.clone(),
                            shifts: next.shifts.clone(),
                            memory: memory.clone(),
                        },
                    ));
                }
            }
        }
        Ok(transitions)
    }

    /// The condition under which each state formula holds at the position
    /// read, from the memory of the positions before it.
    fn truths(&self, memory: &BTreeMap<NodeId, Memory>) -> Result<Truths, AutomatonError> {
        let mut truths = Truths {
            conditions: vec![Dnf::never(); self.ir.len()],
            outcomes: Vec::new(),
        };
        for id in &self.order {
            if self.ir.has_future(*id) {
                continue;
            }
            let value = match self.ir.node(*id)? {
                Node::Constant(true) => Dnf::always(),
                Node::Constant(false) => Dnf::never(),
                Node::Atom(atom) => Dnf::atom(atom),
                Node::Not(operand) => truths.of(operand)?.negate(),
                Node::And(left, right) => truths.of(left)?.and(truths.of(right)?),
                Node::Or(left, right) => truths.of(left)?.or(truths.of(right)?),
                Node::Past {
                    kind,
                    window,
                    left,
                    right,
                } => {
                    let remembered = memory
                        .get(id)
                        .ok_or_else(|| invariant("past-operator-remembered"))?;
                    let guard = left.map(|left| truths.of(left)).transpose()?;
                    let cases =
                        Past::new(kind, window).outcomes(remembered, guard, truths.of(right)?);
                    let holding = cases
                        .iter()
                        .filter(|case| case.holds)
                        .fold(Dnf::never(), |holding, case| holding.or(&case.condition));
                    truths.outcomes.push((*id, cases));
                    holding
                }
                Node::Future { .. } => return Err(invariant("state-formula-has-no-future")),
            };
            if let Some(slot) = truths.conditions.get_mut(id.index()) {
                *slot = value;
            }
        }
        Ok(truths)
    }

    /// The memory every past operator has after the position read, with the
    /// letters that lead to it: each operator's cases, combined.
    fn memories(truths: &Truths) -> Vec<(Guard, BTreeMap<NodeId, Memory>)> {
        let mut combined = vec![(Guard::always(), BTreeMap::new())];
        for (id, cases) in &truths.outcomes {
            let mut extended = Vec::new();
            for (cube, memory) in &combined {
                for case in cases {
                    for guard in case.condition.guards() {
                        let joined = cube.conjoined(*guard);
                        if joined.is_satisfiable() {
                            let mut memory = memory.clone();
                            memory.insert(*id, case.next.clone());
                            extended.push((joined, memory));
                        }
                    }
                }
            }
            combined = extended;
        }
        combined
    }

    /// Every way the next position's obligations can be met at the position
    /// read, with what the position after it inherits.
    fn branches(
        &self,
        config: &Config,
        truths: &Truths,
    ) -> Result<Vec<(Guard, Next)>, AutomatonError> {
        let mut start = Branch {
            cube: Guard::always(),
            todo: config.must.iter().map(|id| Task::Assert(*id)).collect(),
            asserted: BTreeSet::new(),
            activated: BTreeSet::new(),
            opens: BTreeSet::new(),
            handled: BTreeSet::new(),
            next: Next::default(),
        };
        for (core, delays) in &config.shifts {
            for delay in delays {
                match delay.checked_sub(1) {
                    None => start.todo.push(Task::Activate(*core)),
                    Some(remaining) => self.add_shift(&mut start.next, *core, remaining)?,
                }
            }
        }
        let mut finished = Vec::new();
        let mut frontier = vec![start];
        while let Some(branch) = frontier.pop() {
            match self.advance(branch, config, truths)? {
                Progress::Finished(branch) => finished.push((branch.cube, branch.next)),
                Progress::Continue(branches) => frontier.extend(branches),
            }
        }
        Ok(finished)
    }

    /// Decide one more thing about `branch`: its next task, or, with none
    /// left, its next counter. The counters run from the largest node to the
    /// smallest, because an operator only ever starts the operators of its
    /// own operands, which are smaller.
    fn advance(
        &self,
        mut branch: Branch,
        config: &Config,
        truths: &Truths,
    ) -> Result<Progress, AutomatonError> {
        match branch.todo.pop() {
            Some(Task::Assert(id)) => self.assert(branch, id, truths),
            Some(Task::Activate(id)) => self.activate(branch, id),
            None => {
                let counting = branch
                    .opens
                    .iter()
                    .chain(config.counters.keys())
                    .filter(|id| !branch.handled.contains(*id))
                    .max()
                    .copied();
                match counting {
                    None => Ok(Progress::Finished(branch)),
                    Some(id) => {
                        branch.handled.insert(id);
                        self.count(branch, config, id)
                    }
                }
            }
        }
    }

    fn assert(
        &self,
        mut branch: Branch,
        id: NodeId,
        truths: &Truths,
    ) -> Result<Progress, AutomatonError> {
        if !branch.asserted.insert(id) {
            return Ok(Progress::Continue(vec![branch]));
        }
        if !self.ir.has_future(id) {
            let branches = truths
                .of(id)?
                .guards()
                .iter()
                .filter_map(|guard| {
                    let cube = branch.cube.conjoined(*guard);
                    cube.is_satisfiable().then(|| Branch {
                        cube,
                        ..branch.clone()
                    })
                })
                .collect();
            return Ok(Progress::Continue(branches));
        }
        match self.ir.node(id)? {
            Node::And(left, right) => {
                branch
                    .todo
                    .extend([Task::Assert(left), Task::Assert(right)]);
                Ok(Progress::Continue(vec![branch]))
            }
            Node::Or(left, right) => {
                let mut other = branch.clone();
                other.todo.push(Task::Assert(right));
                branch.todo.push(Task::Assert(left));
                Ok(Progress::Continue(vec![branch, other]))
            }
            Node::Future { window, .. } => {
                match window.lower().checked_sub(1) {
                    None => branch.todo.push(Task::Activate(id)),
                    Some(remaining) => {
                        self.add_shift(&mut branch.next, self.ir.core(id), remaining)?;
                    }
                }
                Ok(Progress::Continue(vec![branch]))
            }
            Node::Constant(_) | Node::Atom(_) | Node::Not(_) | Node::Past { .. } => {
                Err(invariant("skeleton-node-is-a-connective-or-future"))
            }
        }
    }

    /// Start the lower-bound-0 operator `id` at this position.
    fn activate(&self, mut branch: Branch, id: NodeId) -> Result<Progress, AutomatonError> {
        if !branch.activated.insert(id) {
            return Ok(Progress::Continue(vec![branch]));
        }
        let Node::Future {
            kind,
            window,
            left,
            right,
        } = self.ir.node(id)?
        else {
            return Err(invariant("activated-node-is-a-future-operator"));
        };
        if matches!(window, Window::Closed { .. }) {
            branch.opens.insert(id);
            return Ok(Progress::Continue(vec![branch]));
        }
        let guard = || left.ok_or_else(|| invariant("binary-operator-has-a-left-operand"));
        // An unbounded operator: its witness now, or, for the others, its
        // own promise to the next position.
        let branches = match kind {
            FutureKind::Eventually => {
                let mut postponed = branch.clone();
                postponed.next.must.insert(id);
                self.exclude(&mut postponed, right)?;
                branch.todo.push(Task::Assert(right));
                vec![branch, postponed]
            }
            FutureKind::Until => {
                let mut postponed = branch.clone();
                postponed.next.must.insert(id);
                postponed.todo.push(Task::Assert(guard()?));
                self.exclude(&mut postponed, right)?;
                branch.todo.push(Task::Assert(right));
                vec![branch, postponed]
            }
            FutureKind::Always => {
                branch.next.must.insert(id);
                branch.todo.push(Task::Assert(right));
                vec![branch]
            }
            FutureKind::Release => {
                branch.todo.push(Task::Assert(right));
                let mut released = branch.clone();
                released.todo.push(Task::Assert(guard()?));
                branch.next.must.insert(id);
                self.exclude(&mut branch, guard()?)?;
                vec![released, branch]
            }
        };
        Ok(Progress::Continue(branches))
    }

    /// Step the counter of the interval operator `id` over this position
    /// (ADR-018 IV-3).
    fn count(
        &self,
        mut branch: Branch,
        config: &Config,
        id: NodeId,
    ) -> Result<Progress, AutomatonError> {
        let Node::Future {
            kind,
            window: Window::Closed { upper, .. },
            left,
            right,
        } = self.ir.node(id)?
        else {
            return Err(invariant("counted-node-is-an-interval-operator"));
        };
        let opens = branch.opens.contains(&id);
        let stored = config.counters.get(&id).copied();
        match kind {
            FutureKind::Eventually | FutureKind::Until => {
                let counter = match stored {
                    None => DeadlineCounter::new(upper),
                    Some(Counter::Deadline(counter)) => counter,
                    Some(Counter::Extent(_)) => return Err(invariant("counter-matches-kind")),
                };
                let waiting = opens || counter.is_pending();
                // The operand holds here: the obligations are discharged.
                let mut claimed = branch.clone();
                claimed.todo.push(Task::Assert(right));
                Self::keep(
                    &mut claimed,
                    id,
                    counter.step(opens, true),
                    Counter::Deadline,
                )?;
                let mut branches = vec![claimed];
                // It does not, and an obligation waits for a later position.
                if let Ok(aged) = counter.step(opens, false) {
                    if kind == FutureKind::Until && waiting {
                        let guard = left.ok_or_else(|| invariant("until-has-a-left-operand"))?;
                        branch.todo.push(Task::Assert(guard));
                    }
                    self.exclude(&mut branch, right)?;
                    Self::set(&mut branch, id, Counter::Deadline(aged));
                    branches.push(branch);
                }
                Ok(Progress::Continue(branches))
            }
            FutureKind::Always | FutureKind::Release => {
                let counter = match stored {
                    None => ExtentCounter::new(upper),
                    Some(Counter::Extent(counter)) => counter,
                    Some(Counter::Deadline(_)) => return Err(invariant("counter-matches-kind")),
                };
                if !opens && !counter.is_pending() {
                    return Ok(Progress::Continue(vec![branch]));
                }
                branch.todo.push(Task::Assert(right));
                let mut released = None;
                if kind == FutureKind::Release {
                    let guard = left.ok_or_else(|| invariant("release-has-a-left-operand"))?;
                    // The left operand holds here: the later positions are
                    // released, and the counter goes idle.
                    let mut cleared = branch.clone();
                    cleared.todo.push(Task::Assert(guard));
                    released = Some(cleared);
                    self.exclude(&mut branch, guard)?;
                }
                Self::keep(&mut branch, id, counter.step(opens, true), Counter::Extent)?;
                Ok(Progress::Continue(
                    released.into_iter().chain([branch]).collect(),
                ))
            }
        }
    }

    /// Keep the counter `stepped` produced, or refuse the invariant that a
    /// counter whose operand holds never fails.
    fn keep<T, E>(
        branch: &mut Branch,
        id: NodeId,
        stepped: Result<T, E>,
        wrap: fn(T) -> Counter,
    ) -> Result<(), AutomatonError> {
        let counter = stepped.map_err(|_| invariant("a-held-operand-fails-no-obligation"))?;
        Self::set(branch, id, wrap(counter));
        Ok(())
    }

    fn set(branch: &mut Branch, id: NodeId, counter: Counter) {
        if counter.is_pending() {
            branch.next.counters.insert(id, counter);
        } else {
            branch.next.counters.remove(&id);
        }
    }

    /// When `operand` is a state formula, require it false in this branch:
    /// the branch that does not discharge on it is only the one for the
    /// letters that make it false, which keeps the automaton deterministic
    /// there and leaves every behaviour accepted by some run.
    fn exclude(&self, branch: &mut Branch, operand: NodeId) -> Result<(), AutomatonError> {
        if !self.ir.has_future(operand) {
            branch.todo.push(Task::Assert(self.ir.negation(operand)?));
        }
        Ok(())
    }

    /// Record that the operator whose lower-bound-0 core is `core` starts
    /// `delay` positions after the next one (ADR-018 IV-3). An open
    /// `eventually` keeps only its latest start and an open `always` only
    /// its earliest, because each subsumes the others (the `[a,*]` shift
    /// counter).
    fn add_shift(&self, next: &mut Next, core: NodeId, delay: u64) -> Result<(), AutomatonError> {
        let Node::Future { kind, window, .. } = self.ir.node(core)? else {
            return Err(invariant("shifted-node-is-a-future-operator"));
        };
        let delays = next.shifts.entry(core).or_default();
        match (window, kind) {
            (Window::From { .. }, FutureKind::Eventually) => {
                let latest = delays.last().copied().map_or(delay, |last| last.max(delay));
                *delays = BTreeSet::from([latest]);
            }
            (Window::From { .. }, FutureKind::Always) => {
                let earliest = delays
                    .first()
                    .copied()
                    .map_or(delay, |first| first.min(delay));
                *delays = BTreeSet::from([earliest]);
            }
            _ => {
                delays.insert(delay);
            }
        }
        Ok(())
    }
}
