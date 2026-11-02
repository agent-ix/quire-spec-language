// SPDX-License-Identifier: AGPL-3.0-or-later
//! The reference evaluator of a tick-monitor plan (QSL FR-251, FR-251-AC-7).
//!
//! It evaluates the declarative meaning of a timed clause over a stream of
//! counter readings, directly and without the plan's entries: the half-open
//! tick cover `[k, k + 1)` widened by the uncertainty `u` to
//! `[k - u, k + 1 + u)` (QSpec FR-160 "Counter-tick intervals and
//! endpoints"), three-valued results with `pending` and `indeterminate`,
//! counter width and the reading-gap fault, and watermarks.
//!
//! Every value is a pair of possibility sets over `{true, false}`: the
//! *open* set, which counts events that may still arrive, and the *closed*
//! set, which assumes no further event. A singleton open set is a verdict.
//! Otherwise the result is `pending` when the closed set is a singleton (the
//! declared progress resolves it) and `indeterminate` when the closed
//! evidence stays conflicting. Boolean and temporal operators lift pointwise
//! over the sets. Time stamps compare by their widened covers: the offset of
//! two stamps `d` ticks apart is the open interval `(d - w, d + w)` with
//! `w = 1 + 2u`; an offset between an event and itself is exactly 0.
//!
//! A buffered operator (an interval operator whose operands hold no temporal
//! operator) keeps its entries in a buffer of the plan's capacity. An event
//! that would exceed the buffer is merged with the newest entry into one
//! `lost` entry that spans both: no event disappears, and any result that
//! depends on a lost entry and is not otherwise decided settles `Incomplete`.

use std::collections::{BTreeMap, BTreeSet};

use qsl_semantics::check::{
    Activation, BinaryTemporal, CheckedFormula, FormulaNode, UnaryTemporal,
};
use quire_exact::{Integer, Rational};

/// An exact tick-interval bound of one interval operator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TickBound {
    /// Lower endpoint in ticks.
    pub lower: Rational,
    /// Whether the lower endpoint is included.
    pub lower_closed: bool,
    /// Upper endpoint in ticks.
    pub upper: Rational,
    /// Whether the upper endpoint is included.
    pub upper_closed: bool,
}

/// What the reference evaluator needs to know about one plan.
#[derive(Clone, Debug)]
pub struct TickMonitorSpec<'f> {
    /// The checked formula.
    pub formula: &'f CheckedFormula,
    /// The clause activation.
    pub activation: Activation,
    /// The tick bound of each interval operator, by formula node index.
    pub bounds: BTreeMap<usize, TickBound>,
    /// The clock uncertainty in ticks, at least 0.
    pub uncertainty: Rational,
    /// The counter width in bits.
    pub width_bits: u32,
    /// The largest admitted tick difference between consecutive readings.
    pub max_reading_gap: u64,
    /// The buffer capacity of each buffered operator, by formula node index.
    pub capacities: BTreeMap<usize, u64>,
}

/// One input of the reference evaluator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TickStep {
    /// A counter reading that creates an event position.
    Reading {
        /// The raw (wrapping) counter reading.
        reading: u64,
        /// The truth of each `holds` atom, by formula node index.
        atoms: BTreeMap<usize, bool>,
        /// Whether the activation guard holds at this event.
        guard: bool,
    },
    /// A bare counter poll: no event position, no entry call.
    Poll {
        /// The raw counter reading.
        reading: u64,
    },
    /// An admitted progress assertion: no event at or before `boundary`
    /// arrives later.
    Watermark {
        /// The unwrapped history-boundary tick.
        boundary: u64,
    },
}

/// Why a step was not applied.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TickStepError {
    /// A `holds` atom of the formula has no value in the event.
    #[error("event is missing the value of atom {atom}")]
    MissingAtom {
        /// The formula node index of the atom.
        atom: usize,
    },
    /// An event arrived at a tick the watermark already covered.
    #[error("event at tick {tick} is not after the watermark {boundary}")]
    EventBehindWatermark {
        /// The unwrapped event tick.
        tick: i128,
        /// The asserted boundary.
        boundary: i128,
    },
}

/// The settlement of one obligation instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TickOutcome {
    /// Definitely true.
    True,
    /// Definitely false.
    False,
    /// Declared progress could still resolve the verdict.
    Pending,
    /// Closed evidence stays conflicting.
    Indeterminate,
    /// A buffer limit hid evidence; `capacity` is the buffer's capacity.
    Incomplete {
        /// The capacity that was reached.
        capacity: u64,
    },
    /// The instance was open across a reading-gap fault.
    Failed {
        /// The checked gap in ticks.
        gap: u64,
        /// The plan's maximum reading gap.
        max_reading_gap: u64,
    },
}

/// One emitted result: the instance and its new outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TickResult {
    /// The formula node index of the subformula.
    pub subformula: usize,
    /// The event position the instance began at.
    pub position: usize,
    /// The unwrapped tick of that event.
    pub origin: i128,
    /// The outcome.
    pub outcome: TickOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Poss {
    t: bool,
    f: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Val {
    open: Poss,
    closed: Poss,
    /// The node whose buffer lost evidence that this value depends on.
    lost: Option<usize>,
}

const BOTH: Poss = Poss { t: true, f: true };
const ONLY_T: Poss = Poss { t: true, f: false };
const ONLY_F: Poss = Poss { t: false, f: true };

impl Val {
    const TRUE: Self = Self::fixed(ONLY_T);
    const FALSE: Self = Self::fixed(ONLY_F);
    const UNKNOWN: Self = Self::fixed(BOTH);
    const fn fixed(poss: Poss) -> Self {
        Self {
            open: poss,
            closed: poss,
            lost: None,
        }
    }
    fn of(value: bool) -> Self {
        if value {
            Self::TRUE
        } else {
            Self::FALSE
        }
    }
    fn not(self) -> Self {
        let flip = |p: Poss| Poss { t: p.f, f: p.t };
        Self {
            open: flip(self.open),
            closed: flip(self.closed),
            lost: self.lost,
        }
    }
    fn and(self, other: Self) -> Self {
        let both = |a: Poss, b: Poss| Poss {
            t: a.t && b.t,
            f: a.f || b.f,
        };
        Self {
            open: both(self.open, other.open),
            closed: both(self.closed, other.closed),
            lost: self.lost.or(other.lost),
        }
    }
    fn or(self, other: Self) -> Self {
        self.not().and(other.not()).not()
    }
    fn outcome(self, capacities: &BTreeMap<usize, u64>) -> TickOutcome {
        match (self.open.t, self.open.f) {
            (true, false) => TickOutcome::True,
            (false, _) => TickOutcome::False,
            _ => match self.lost {
                Some(node) => TickOutcome::Incomplete {
                    capacity: capacities.get(&node).copied().unwrap_or(0),
                },
                None if self.closed.t != self.closed.f => TickOutcome::Pending,
                None => TickOutcome::Indeterminate,
            },
        }
    }
    fn is_final(self) -> bool {
        self.open.t != self.open.f
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Member {
    All,
    Some,
    No,
}

#[derive(Clone, Copy, Debug)]
struct Cell {
    first: usize,
    last: usize,
    t_first: i128,
    t_last: i128,
    lost: bool,
}

struct Event {
    tick: i128,
    atoms: BTreeMap<usize, bool>,
    guard: bool,
}

#[derive(Clone, Copy)]
enum Dir {
    Future,
    Past,
}

#[derive(Clone, Copy)]
struct Op {
    dir: Dir,
    /// `true` for the dual operators (`always`, `historically`, `release`,
    /// `triggered`), which negate their operands and their result.
    dual: bool,
    /// Whether the operator has a left operand (`until`-like).
    binary: bool,
}

fn q(value: i128) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn operator(node: &FormulaNode) -> Option<(Op, Option<usize>, usize, Option<usize>)> {
    // (op, left, right-or-only operand, _)
    match node {
        FormulaNode::Unary {
            operator, operand, ..
        } => {
            let (dir, dual) = match operator {
                UnaryTemporal::Eventually => (Dir::Future, false),
                UnaryTemporal::Always => (Dir::Future, true),
                UnaryTemporal::Once => (Dir::Past, false),
                UnaryTemporal::Historically => (Dir::Past, true),
            };
            Some((
                Op {
                    dir,
                    dual,
                    binary: false,
                },
                None,
                operand.index(),
                None,
            ))
        }
        FormulaNode::Binary {
            operator,
            left,
            right,
            ..
        } => {
            let (dir, dual) = match operator {
                BinaryTemporal::Until => (Dir::Future, false),
                BinaryTemporal::Release => (Dir::Future, true),
                BinaryTemporal::Since => (Dir::Past, false),
                BinaryTemporal::Triggered => (Dir::Past, true),
            };
            Some((
                Op {
                    dir,
                    dual,
                    binary: true,
                },
                Some(left.index()),
                right.index(),
                None,
            ))
        }
        _ => None,
    }
}

/// The streaming reference evaluator of one plan.
pub struct TickMonitorReference<'f> {
    spec: TickMonitorSpec<'f>,
    width: Rational,
    events: Vec<Event>,
    watermark: i128,
    now: i128,
    baseline: Option<u64>,
    buffers: BTreeMap<usize, Vec<Cell>>,
    /// Settled values of instances of buffered nodes.
    settled: BTreeMap<(usize, usize), Val>,
    past_values: BTreeMap<(usize, usize), Val>,
    failed: BTreeSet<(usize, usize)>,
    emitted: BTreeMap<(usize, usize), TickOutcome>,
}

impl<'f> TickMonitorReference<'f> {
    /// A fresh evaluator for `spec` with the unwrapped time at 0.
    pub fn new(spec: TickMonitorSpec<'f>) -> Self {
        let width = q(1).add(&q(2).mul(&spec.uncertainty));
        let buffers = Self::buffered_nodes(&spec)
            .into_iter()
            .map(|node| (node, Vec::new()))
            .collect();
        Self {
            spec,
            width,
            events: Vec::new(),
            watermark: -1,
            now: 0,
            baseline: None,
            buffers,
            settled: BTreeMap::new(),
            past_values: BTreeMap::new(),
            failed: BTreeSet::new(),
            emitted: BTreeMap::new(),
        }
    }

    /// The formula nodes that keep a buffer: interval operators whose
    /// operands hold no temporal operator.
    pub fn buffered_nodes(spec: &TickMonitorSpec<'_>) -> Vec<usize> {
        let nodes = spec.formula.nodes();
        (0..nodes.len())
            .filter(|index| {
                spec.bounds.contains_key(index)
                    && operator(&nodes[*index]).is_some_and(|(_, left, right, _)| {
                        left.into_iter()
                            .chain([right])
                            .all(|child| !Self::has_temporal(nodes, child))
                    })
            })
            .collect()
    }

    fn has_temporal(nodes: &[FormulaNode], index: usize) -> bool {
        let mut stack = vec![index];
        while let Some(next) = stack.pop() {
            if operator(&nodes[next]).is_some() {
                return true;
            }
            stack.extend(nodes[next].operands().map(|operand| operand.index()));
        }
        false
    }

    fn is_future(&self, node: usize) -> bool {
        operator(&self.spec.formula.nodes()[node])
            .is_some_and(|(op, ..)| matches!(op.dir, Dir::Future))
    }

    /// The earliest tick a future event can carry.
    fn horizon(&self) -> i128 {
        let last = self.events.last().map_or(0, |event| event.tick);
        last.max(self.watermark + 1)
    }

    fn member(&self, node: usize, dmin: i128, dmax: i128, same: bool) -> Member {
        let Some(bound) = self.spec.bounds.get(&node) else {
            return Member::All;
        };
        if same {
            let zero = q(0);
            let above = zero > bound.lower || (zero == bound.lower && bound.lower_closed);
            let below = zero < bound.upper || (zero == bound.upper && bound.upper_closed);
            return if above && below {
                Member::All
            } else {
                Member::No
            };
        }
        let lo = q(dmin).sub(&self.width);
        let hi = q(dmax).add(&self.width);
        if lo >= bound.lower && hi <= bound.upper {
            Member::All
        } else if hi <= bound.lower || lo >= bound.upper {
            Member::No
        } else {
            Member::Some
        }
    }

    fn closed(&self, node: usize, tick: i128) -> bool {
        self.spec
            .bounds
            .get(&node)
            .is_some_and(|bound| q(self.horizon()).sub(&q(tick)).sub(&self.width) >= bound.upper)
    }

    fn member_val(member: Member) -> Val {
        match member {
            Member::All => Val::TRUE,
            Member::Some => Val::UNKNOWN,
            Member::No => Val::FALSE,
        }
    }

    fn event_cells(&self, upto: Option<usize>, from: usize) -> Vec<Cell> {
        let end = upto.map_or(self.events.len(), |last| last + 1);
        (from..end)
            .map(|position| Cell {
                first: position,
                last: position,
                t_first: self.events[position].tick,
                t_last: self.events[position].tick,
                lost: false,
            })
            .collect()
    }

    fn operand(&self, owner: usize, child: usize, cell: &Cell, memo: &mut Memo) -> Val {
        if cell.lost {
            return Val {
                lost: Some(owner),
                ..Val::UNKNOWN
            };
        }
        self.eval(child, cell.first, memo)
    }

    /// Evaluate the temporal `node` at `position` over `cells`, ascending.
    fn temporal(&self, node: usize, position: usize, cells: &[Cell], memo: &mut Memo) -> Val {
        let Some((op, left, right, _)) = operator(&self.spec.formula.nodes()[node]) else {
            return Val::UNKNOWN;
        };
        let polar = |value: Val| if op.dual { value.not() } else { value };
        let phi = |this: &Self, cell: &Cell, memo: &mut Memo| match left {
            Some(child) => polar(this.operand(node, child, cell, memo)),
            None => Val::TRUE,
        };
        let psi = |this: &Self, cell: &Cell, memo: &mut Memo| {
            polar(this.operand(node, right, cell, memo))
        };
        let tick = self.events[position].tick;
        let mut acc = Val::FALSE;
        match op.dir {
            Dir::Future => {
                let mut prefix = Val::TRUE;
                for cell in cells.iter().filter(|cell| cell.first >= position) {
                    let same = cell.first == position && cell.last == position;
                    let member = self.member(node, cell.t_first - tick, cell.t_last - tick, same);
                    let contribution = Self::member_val(member)
                        .and(psi(self, cell, memo))
                        .and(prefix);
                    acc = acc.or(contribution);
                    prefix = prefix.and(phi(self, cell, memo));
                }
                if !self.closed(node, tick) {
                    acc = acc.or(Val {
                        open: BOTH,
                        closed: ONLY_F,
                        lost: None,
                    }
                    .and(prefix));
                }
            }
            Dir::Past => {
                let mut suffix = Val::TRUE;
                for cell in cells.iter().rev().filter(|cell| cell.last <= position) {
                    let same = cell.first == position && cell.last == position;
                    let member = self.member(node, tick - cell.t_last, tick - cell.t_first, same);
                    let contribution = Self::member_val(member)
                        .and(psi(self, cell, memo))
                        .and(suffix);
                    acc = acc.or(contribution);
                    suffix = suffix.and(phi(self, cell, memo));
                }
            }
        }
        polar(acc)
    }

    fn eval(&self, node: usize, position: usize, memo: &mut Memo) -> Val {
        if self.failed.contains(&(node, position)) {
            return Val::UNKNOWN;
        }
        if let Some(value) = memo.get(&(node, position)) {
            return *value;
        }
        let value = match &self.spec.formula.nodes()[node] {
            FormulaNode::Constant(value) => Val::of(*value),
            FormulaNode::Holds { .. } => self.events[position]
                .atoms
                .get(&node)
                .map_or(Val::UNKNOWN, |value| Val::of(*value)),
            FormulaNode::Aggregate { .. } => Val::UNKNOWN,
            FormulaNode::Not(operand) => self.eval(operand.index(), position, memo).not(),
            FormulaNode::And(a, b) => {
                let left = self.eval(a.index(), position, memo);
                left.and(self.eval(b.index(), position, memo))
            }
            FormulaNode::Or(a, b) => {
                let left = self.eval(a.index(), position, memo);
                left.or(self.eval(b.index(), position, memo))
            }
            FormulaNode::Implies(a, b) => {
                let left = self.eval(a.index(), position, memo).not();
                left.or(self.eval(b.index(), position, memo))
            }
            FormulaNode::Unary { .. } | FormulaNode::Binary { .. } => {
                if let Some(value) = self.settled.get(&(node, position)) {
                    *value
                } else if let Some(value) = self.past_values.get(&(node, position)) {
                    *value
                } else if let Some(cells) = self.buffers.get(&node) {
                    self.temporal(node, position, cells, memo)
                } else {
                    let upto = (!self.is_future(node)).then_some(position);
                    let cells = self.event_cells(upto, 0);
                    self.temporal(node, position, &cells, memo)
                }
            }
        };
        memo.insert((node, position), value);
        value
    }

    /// Apply one step and return the results whose outcome changed.
    pub fn step(&mut self, step: &TickStep) -> Result<Vec<TickResult>, TickStepError> {
        match step {
            TickStep::Poll { .. } => Ok(Vec::new()),
            TickStep::Watermark { boundary } => {
                self.watermark = self.watermark.max(i128::from(*boundary));
                self.expire();
                Ok(self.settle_and_emit())
            }
            TickStep::Reading {
                reading,
                atoms,
                guard,
            } => self.reading(*reading, atoms, *guard),
        }
    }

    fn modulus(&self) -> i128 {
        1_i128 << self.spec.width_bits
    }

    fn reading(
        &mut self,
        reading: u64,
        atoms: &BTreeMap<usize, bool>,
        guard: bool,
    ) -> Result<Vec<TickResult>, TickStepError> {
        let delta = match self.baseline {
            None => 0,
            Some(base) => (i128::from(reading) - i128::from(base)).rem_euclid(self.modulus()),
        };
        if delta > i128::from(self.spec.max_reading_gap) {
            self.baseline = Some(reading);
            self.now += delta;
            return Ok(self.gap(u64::try_from(delta).unwrap_or(u64::MAX)));
        }
        for (index, node) in self.spec.formula.nodes().iter().enumerate() {
            if matches!(node, FormulaNode::Holds { .. }) && !atoms.contains_key(&index) {
                return Err(TickStepError::MissingAtom { atom: index });
            }
        }
        let tick = self.now + delta;
        if tick <= self.watermark {
            return Err(TickStepError::EventBehindWatermark {
                tick,
                boundary: self.watermark,
            });
        }
        self.baseline = Some(reading);
        self.now = tick;
        let position = self.events.len();
        self.events.push(Event {
            tick,
            atoms: atoms.clone(),
            guard,
        });
        self.expire();
        let nodes: Vec<usize> = self.buffers.keys().copied().collect();
        for node in nodes {
            self.add_event(node, position);
        }
        Ok(self.settle_and_emit())
    }

    fn add_event(&mut self, node: usize, position: usize) {
        let tick = self.events[position].tick;
        let future = self.is_future(node);
        let capacity = self.spec.capacities.get(&node).map_or(usize::MAX, |value| {
            usize::try_from(*value).unwrap_or(usize::MAX)
        });
        let cell = Cell {
            first: position,
            last: position,
            t_first: tick,
            t_last: tick,
            lost: false,
        };
        if !future {
            let mut cells = self.buffers.get(&node).cloned().unwrap_or_default();
            cells.push(cell);
            let value = self.temporal(node, position, &cells, &mut Memo::new());
            self.past_values.insert((node, position), value);
        }
        let Some(buffer) = self.buffers.get_mut(&node) else {
            return;
        };
        if buffer.len() >= capacity.max(1) {
            if let Some(last) = buffer.last_mut() {
                let (first, was_lost) = (last.first, last.lost);
                last.last = position;
                last.t_last = tick;
                last.lost = true;
                if future {
                    let lost = Val {
                        lost: Some(node),
                        ..Val::UNKNOWN
                    };
                    for lost_position in first..=position {
                        self.settled.insert((node, lost_position), lost);
                    }
                }
                let _ = was_lost;
            }
        } else {
            buffer.push(cell);
        }
    }

    /// Drop past entries that no future position can place in the window.
    fn expire(&mut self) {
        let horizon = self.horizon();
        let nodes: Vec<usize> = self.buffers.keys().copied().collect();
        for node in nodes {
            if self.is_future(node) {
                continue;
            }
            let Some(bound) = self.spec.bounds.get(&node).cloned() else {
                continue;
            };
            let width = self.width.clone();
            if let Some(buffer) = self.buffers.get_mut(&node) {
                buffer.retain(|cell| q(horizon).sub(&q(cell.t_last)).sub(&width) < bound.upper);
            }
        }
    }

    fn reported(&self) -> BTreeSet<(usize, usize)> {
        let mut out = BTreeSet::new();
        let root = self.spec.formula.root().index();
        for position in 0..self.events.len() {
            for index in self.spec.bounds.keys() {
                out.insert((*index, position));
            }
            let active = match self.spec.activation {
                Activation::Origin => position == 0,
                Activation::Each { guarded } => !guarded || self.events[position].guard,
            };
            if active {
                out.insert((root, position));
            }
        }
        out
    }

    fn settle_and_emit(&mut self) -> Vec<TickResult> {
        // Settle future instances of buffered nodes that can no longer change.
        let nodes: Vec<usize> = self.buffers.keys().copied().collect();
        for node in &nodes {
            if !self.is_future(*node) {
                continue;
            }
            let cells = self.buffers.get(node).cloned().unwrap_or_default();
            for cell in cells.iter().filter(|cell| !cell.lost) {
                let key = (*node, cell.first);
                if self.settled.contains_key(&key) || self.failed.contains(&key) {
                    continue;
                }
                let value = self.temporal(*node, cell.first, &cells, &mut Memo::new());
                let tick = self.events[cell.first].tick;
                if value.is_final() || self.closed(*node, tick) {
                    self.settled.insert(key, value);
                }
            }
            let oldest_open = cells
                .iter()
                .find(|cell| {
                    !cell.lost
                        && !self.settled.contains_key(&(*node, cell.first))
                        && !self.failed.contains(&(*node, cell.first))
                })
                .map(|cell| cell.first);
            if let Some(buffer) = self.buffers.get_mut(node) {
                match oldest_open {
                    Some(first) => buffer.retain(|cell| cell.last >= first),
                    None => buffer.retain(|cell| cell.first + 1 == self.events.len()),
                }
            }
        }
        let mut memo = Memo::new();
        let mut results = Vec::new();
        for (node, position) in self.reported() {
            if self.failed.contains(&(node, position)) {
                continue;
            }
            let outcome = self
                .eval(node, position, &mut memo)
                .outcome(&self.spec.capacities);
            if self.emitted.get(&(node, position)) != Some(&outcome) {
                self.emitted.insert((node, position), outcome);
                results.push(TickResult {
                    subformula: node,
                    position,
                    origin: self.events[position].tick,
                    outcome,
                });
            }
        }
        results
    }

    /// The reading-gap fault: every instance still `Pending` fails.
    fn gap(&mut self, gap: u64) -> Vec<TickResult> {
        let mut memo = Memo::new();
        let mut open = Vec::new();
        for (node, position) in self.reported() {
            if self.failed.contains(&(node, position)) {
                continue;
            }
            let outcome = self
                .eval(node, position, &mut memo)
                .outcome(&self.spec.capacities);
            if outcome == TickOutcome::Pending {
                open.push((node, position));
            }
        }
        let mut results = Vec::new();
        for (node, position) in open {
            self.failed.insert((node, position));
            let outcome = TickOutcome::Failed {
                gap,
                max_reading_gap: self.spec.max_reading_gap,
            };
            self.emitted.insert((node, position), outcome);
            results.push(TickResult {
                subformula: node,
                position,
                origin: self.events[position].tick,
                outcome,
            });
        }
        results
    }

    /// The current outcome of an instance, or `None` if it does not exist.
    pub fn outcome(&self, subformula: usize, position: usize) -> Option<TickOutcome> {
        self.emitted.get(&(subformula, position)).copied()
    }

    /// The unwrapped time of the latest admitted reading.
    pub fn now(&self) -> i128 {
        self.now
    }
}

type Memo = std::collections::HashMap<(usize, usize), Val>;

/// The comparison of FR-160 "Counter-tick intervals and endpoints": whether
/// `b - a <= bound` for readings `a` and `b` under `uncertainty`. Returns
/// `Some(true)`, `Some(false)` or `None` for indeterminate.
pub fn within_bound(
    bound: i128,
    uncertainty: &Rational,
    request_tick: i128,
    ack_tick: i128,
) -> Option<bool> {
    let width = q(1).add(&q(2).mul(uncertainty));
    let d = ack_tick - request_tick;
    let lo = q(d).sub(&width);
    let hi = q(d).add(&width);
    let bound = q(bound);
    if hi <= bound {
        Some(true)
    } else if lo >= bound {
        Some(false)
    } else {
        None
    }
}

/// Whether tick `a` is definitely before tick `b` under `uncertainty`
/// (every point of `a`'s cover lies below every point of `b`'s).
pub fn definitely_before(uncertainty: &Rational, a: i128, b: i128) -> bool {
    // a's cover is [a - u, a + 1 + u), b's is [b - u, b + 1 + u): the
    // latest point of a is open at a + 1 + u.
    q(a).add(&q(1)).add(uncertainty) <= q(b).sub(uncertainty)
}
