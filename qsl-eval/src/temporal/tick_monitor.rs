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
//! Every interval operator, at any depth, keeps its entries in a buffer of
//! the plan's capacity and retention ([`TickShape`]). An event that would
//! exceed a buffer is merged with the newest entry into one `lost` entry that
//! spans both: no event disappears, and any result that depends on a lost
//! entry and is not otherwise decided settles `Incomplete`. An instance
//! settles when its value is final or its lookahead has passed, and an entry
//! is dropped once it is settled, reported and past its retention. Unbounded
//! operators keep a constant-size accumulator per live instance, bounded by
//! the shared activation capacity.

use std::collections::BTreeMap;

use qsl_semantics::check::{Activation, CheckedFormula, FormulaNode};
use quire_exact::{Integer, Rational};

use super::tick_shape::{temporal_leaves, TickShape};

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
    /// The bound on live activations of the root (`monitor.max_activations`);
    /// an `on origin` root with capacity 0 is never started.
    pub activation_capacity: u64,
    /// The `holds` atoms with no admitted projection, by formula index.
    pub unprojectable: std::collections::BTreeSet<usize>,
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
    /// An atom of the formula has no admitted projection, so the instances
    /// that read it settle `Incomplete(LimitReached{atom_projection})`
    /// (QSpec FR-487-AC-12).
    AtomProjection {
        /// The formula index of the atom.
        atom: usize,
    },
    /// The root activation exceeded the activation capacity and was not
    /// started.
    ActivationLimit {
        /// The activation capacity.
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

/// What hid evidence from a value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Lost {
    /// The buffer of this interval operator overflowed.
    Buffer(usize),
    /// The activation capacity refused the instance.
    Activation,
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
    /// What hid evidence that this value depends on; the left operand's
    /// wins.
    lost: Option<Lost>,
    /// The lowest atom without an admitted projection that this value
    /// depends on (QSpec FR-487-AC-12).
    atom: Option<usize>,
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
            atom: None,
        }
    }
    fn of(value: bool) -> Self {
        if value {
            Self::TRUE
        } else {
            Self::FALSE
        }
    }
    fn lost_by(lost: Lost) -> Self {
        Self {
            lost: Some(lost),
            ..Self::UNKNOWN
        }
    }
    fn not(self) -> Self {
        let flip = |p: Poss| Poss { t: p.f, f: p.t };
        Self {
            open: flip(self.open),
            closed: flip(self.closed),
            lost: self.lost,
            atom: self.atom,
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
            atom: match (self.atom, other.atom) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            },
        }
    }
    fn or(self, other: Self) -> Self {
        self.not().and(other.not()).not()
    }
    fn outcome(self, capacities: &BTreeMap<usize, u64>, activation: u64) -> TickOutcome {
        match (self.open.t, self.open.f) {
            (true, false) => TickOutcome::True,
            (false, _) => TickOutcome::False,
            _ if self.atom.is_some() => TickOutcome::AtomProjection {
                atom: self.atom.unwrap_or(0),
            },
            _ => match self.lost {
                Some(Lost::Buffer(node)) => TickOutcome::Incomplete {
                    capacity: capacities.get(&node).copied().unwrap_or(0),
                },
                Some(Lost::Activation) => TickOutcome::ActivationLimit {
                    capacity: activation,
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

/// One entry of an interval operator's buffer: one event, or, once the buffer
/// overflowed, a run of events merged into one `lost` entry.
#[derive(Clone, Copy, Debug)]
struct Cell {
    first: usize,
    last: usize,
    t_first: i128,
    t_last: i128,
    lost: bool,
    /// The value, once the instance can no longer change.
    set: Option<Val>,
    /// The outcome last reported.
    rep: Option<TickOutcome>,
}

struct Event {
    tick: i128,
    atoms: BTreeMap<usize, bool>,
    guard: bool,
}

/// A live instance of an unbounded operator, or of a root that is not an
/// interval operator: a constant-size accumulator (QSL FR-251 "State").
#[derive(Clone, Copy, Debug)]
struct Elem {
    node: usize,
    pos: usize,
    acc: Val,
    pfx: Val,
    rep: Option<TickOutcome>,
}

fn q(value: i128) -> Rational {
    Rational::from_integer(Integer::from(value))
}

/// The streaming reference evaluator of one plan.
pub struct TickMonitorReference<'f> {
    spec: TickMonitorSpec<'f>,
    shape: TickShape,
    events: Vec<Event>,
    watermark: i128,
    now: i128,
    hz: i128,
    baseline: Option<u64>,
    cells: BTreeMap<usize, Vec<Cell>>,
    elems: Vec<Elem>,
    /// Values of unbounded or root instances that finished or were refused.
    ufinal: BTreeMap<(usize, usize), Val>,
    /// The unwrapped tick history restarts at after a reading-gap fault
    /// (FR-251-AC-15); past operators reaching before it are indeterminate.
    history_start: Option<i128>,
    emitted: BTreeMap<(usize, usize), TickOutcome>,
}

impl<'f> TickMonitorReference<'f> {
    /// A fresh evaluator for `spec` with the unwrapped time at 0.
    pub fn new(spec: TickMonitorSpec<'f>) -> Self {
        let shape = TickShape::of(spec.formula, &spec.bounds, &spec.uncertainty);
        let cells = spec.bounds.keys().map(|node| (*node, Vec::new())).collect();
        Self {
            spec,
            shape,
            events: Vec::new(),
            watermark: -1,
            now: 0,
            hz: 0,
            baseline: None,
            cells,
            elems: Vec::new(),
            ufinal: BTreeMap::new(),
            history_start: None,
            emitted: BTreeMap::new(),
        }
    }

    /// The formula nodes that keep a buffer: the interval operators.
    pub fn buffered_nodes(spec: &TickMonitorSpec<'_>) -> Vec<usize> {
        spec.bounds.keys().copied().collect()
    }

    fn width(&self) -> &Rational {
        self.shape.width()
    }

    fn is_future(&self, node: usize) -> bool {
        self.shape.node(node).is_some_and(|shape| shape.future)
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
        let lo = q(dmin).sub(self.width());
        let hi = q(dmax).add(self.width());
        if lo >= bound.lower && hi <= bound.upper {
            Member::All
        } else if hi <= bound.lower || lo >= bound.upper {
            Member::No
        } else {
            Member::Some
        }
    }

    /// Whether an instance of `node` at `tick` can gain no more events.
    fn closed(&self, node: usize, tick: i128) -> bool {
        self.spec
            .bounds
            .get(&node)
            .is_some_and(|bound| q(self.hz).sub(&q(tick)).sub(self.width()) >= bound.upper)
    }

    fn member_val(member: Member) -> Val {
        match member {
            Member::All => Val::TRUE,
            Member::Some => Val::UNKNOWN,
            Member::No => Val::FALSE,
        }
    }

    /// The value of the Boolean expression `index` at event `pos`.
    fn expr(&self, index: usize, pos: usize) -> Val {
        match &self.spec.formula.nodes()[index] {
            FormulaNode::Constant(value) => Val::of(*value),
            FormulaNode::Holds { .. } if self.spec.unprojectable.contains(&index) => Val {
                atom: Some(index),
                ..Val::UNKNOWN
            },
            FormulaNode::Holds { .. } => self.events[pos]
                .atoms
                .get(&index)
                .map_or(Val::UNKNOWN, |value| Val::of(*value)),
            FormulaNode::Aggregate { .. } => Val::UNKNOWN,
            FormulaNode::Not(operand) => self.expr(operand.index(), pos).not(),
            FormulaNode::And(a, b) => self.expr(a.index(), pos).and(self.expr(b.index(), pos)),
            FormulaNode::Or(a, b) => self.expr(a.index(), pos).or(self.expr(b.index(), pos)),
            FormulaNode::Implies(a, b) => {
                self.expr(a.index(), pos).not().or(self.expr(b.index(), pos))
            }
            FormulaNode::Unary { .. } | FormulaNode::Binary { .. } => {
                self.temporal_val(index, pos)
            }
        }
    }

    /// The value of the temporal operator `node` at event `pos`.
    fn temporal_val(&self, node: usize, pos: usize) -> Val {
        if self.cells.contains_key(&node) {
            return self.cell_val(node, pos);
        }
        if let Some(elem) = self.elems.iter().find(|e| e.node == node && e.pos == pos) {
            return self.elem_val(elem);
        }
        self.ufinal
            .get(&(node, pos))
            .copied()
            .unwrap_or(Val::UNKNOWN)
    }

    fn cell_val(&self, node: usize, pos: usize) -> Val {
        let Some(cells) = self.cells.get(&node) else {
            return Val::UNKNOWN;
        };
        let Some(index) = cells.iter().position(|c| c.first <= pos && pos <= c.last) else {
            return Val::UNKNOWN;
        };
        let cell = &cells[index];
        if cell.lost {
            return Val::lost_by(Lost::Buffer(node));
        }
        match cell.set {
            Some(value) => value,
            None => self.compute(node, pos, cell.t_first),
        }
    }

    /// An operand of `owner` at `cell`: lost when the cell was merged.
    fn operand(&self, owner: usize, child: usize, cell: &Cell) -> Val {
        if cell.lost {
            return Val::lost_by(Lost::Buffer(owner));
        }
        self.expr(child, cell.first)
    }

    /// Evaluate the interval operator `node` for the instance at event `pos`
    /// (tick `tick`) over its buffer.
    fn compute(&self, node: usize, pos: usize, tick: i128) -> Val {
        let (Some(info), Some(cells)) = (self.shape.node(node), self.cells.get(&node)) else {
            return Val::UNKNOWN;
        };
        let polar = |value: Val| if info.dual { value.not() } else { value };
        let phi = |cell: &Cell| match info.left {
            Some(child) => polar(self.operand(node, child, cell)),
            None => Val::TRUE,
        };
        let psi = |cell: &Cell| polar(self.operand(node, info.right, cell));
        let mut acc = Val::FALSE;
        if info.future {
            let mut prefix = Val::TRUE;
            for cell in cells.iter().filter(|cell| cell.first >= pos) {
                let same = cell.first == pos && cell.last == pos;
                let member = self.member(node, cell.t_first - tick, cell.t_last - tick, same);
                acc = acc.or(Self::member_val(member).and(psi(cell)).and(prefix));
                prefix = prefix.and(phi(cell));
            }
            if !self.closed(node, tick) {
                acc = acc.or(Val {
                    open: BOTH,
                    closed: ONLY_F,
                    lost: None,
                    atom: None,
                }
                .and(prefix));
            }
        } else {
            for cell in cells.iter().filter(|cell| cell.last <= pos) {
                let same = cell.first == pos && cell.last == pos;
                let member = self.member(node, tick - cell.t_last, tick - cell.t_first, same);
                acc = acc
                    .and(phi(cell))
                    .or(Self::member_val(member).and(psi(cell)));
            }
            if let Some(start) = self.history_start {
                // History before the marker is gone: an event there could have
                // been in the window, and nothing says whether it was.
                let before = start - 1;
                if self.member(node, tick - before, tick - before, false) != Member::No {
                    acc = acc.or(Val::UNKNOWN);
                }
            }
        }
        polar(acc)
    }

    /// The interval operator whose buffer carries the operands of the
    /// unbounded operator `node`: its lowest-numbered interval operand.
    fn carrier(&self, node: usize) -> Option<usize> {
        let info = self.shape.node(node)?;
        info.left
            .into_iter()
            .chain([info.right])
            .flat_map(|operand| temporal_leaves(self.spec.formula, operand))
            .find(|leaf| self.cells.contains_key(leaf))
    }

    fn fold_cell(&self, node: usize, cell: &Cell, acc: Val, pfx: Val, carrier: usize) -> (Val, Val) {
        let Some(info) = self.shape.node(node) else {
            return (acc, pfx);
        };
        let polar = |value: Val| if info.dual { value.not() } else { value };
        let (psi, phi) = if cell.lost {
            (Val::lost_by(Lost::Buffer(carrier)), Val::lost_by(Lost::Buffer(carrier)))
        } else {
            (
                polar(self.expr(info.right, cell.first)),
                info.left
                    .map_or(Val::TRUE, |left| polar(self.expr(left, cell.first))),
            )
        };
        (acc.or(psi.and(pfx)), pfx.and(phi))
    }

    fn elem_val(&self, elem: &Elem) -> Val {
        let Some(info) = self.shape.node(elem.node) else {
            return self.expr(elem.node, elem.pos);
        };
        let polar = |value: Val| if info.dual { value.not() } else { value };
        let (mut acc, mut pfx) = (elem.acc, elem.pfx);
        if let Some(carrier) = self.carrier(elem.node) {
            for cell in self
                .cells
                .get(&carrier)
                .into_iter()
                .flatten()
                .filter(|cell| cell.last >= elem.pos)
            {
                (acc, pfx) = self.fold_cell(elem.node, cell, acc, pfx, carrier);
            }
        }
        let open = Val {
            open: BOTH,
            closed: ONLY_F,
            lost: None,
            atom: None,
        };
        polar(acc.or(open.and(pfx)))
    }

    fn outcome_of(&self, value: Val) -> TickOutcome {
        value.outcome(&self.spec.capacities, self.spec.activation_capacity)
    }

    /// Apply one step and return the results whose outcome changed.
    pub fn step(&mut self, step: &TickStep) -> Result<Vec<TickResult>, TickStepError> {
        match step {
            TickStep::Poll { .. } => Ok(Vec::new()),
            TickStep::Watermark { boundary } => {
                self.watermark = self.watermark.max(i128::from(*boundary));
                self.hz = self.now.max(self.watermark + 1);
                self.settle();
                let mut results = self.report(true);
                self.drop_cells();
                results.sort_by_key(|r| (r.subformula, r.position));
                Ok(results)
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
            if matches!(node, FormulaNode::Holds { .. })
                && !self.spec.unprojectable.contains(&index)
                && !atoms.contains_key(&index)
            {
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
        self.hz = tick;
        self.settle();
        self.drop_cells();
        let position = self.events.len();
        self.events.push(Event {
            tick,
            atoms: atoms.clone(),
            guard,
        });
        let mut results = self.push(position);
        self.settle();
        results.extend(self.start(position));
        results.extend(self.report(false));
        results.sort_by_key(|r| (r.subformula, r.position));
        Ok(results)
    }

    /// Settle every instance that can no longer change: its value is final,
    /// or its lookahead has passed.
    fn settle(&mut self) {
        let nodes: Vec<usize> = self.cells.keys().rev().copied().collect();
        for node in nodes {
            let Some(lookahead) = self.shape.node(node).map(|info| info.lookahead.clone()) else {
                continue;
            };
            let cells = self.cells.get(&node).cloned().unwrap_or_default();
            let mut updates = Vec::new();
            for (index, cell) in cells.iter().enumerate() {
                if cell.set.is_some() || cell.lost {
                    continue;
                }
                let value = self.compute(node, cell.first, cell.t_first);
                let elapsed = lookahead
                    .as_ref()
                    .is_some_and(|look| &q(self.hz - cell.t_first) >= look);
                if value.is_final() || elapsed {
                    updates.push((index, value));
                }
            }
            if let Some(buffer) = self.cells.get_mut(&node) {
                for (index, value) in updates {
                    buffer[index].set = Some(value);
                }
            }
        }
    }

    /// Whether an unsettled instance of `node` can still read cell `index`.
    fn needed(&self, node: usize, index: usize) -> bool {
        let Some(cells) = self.cells.get(&node) else {
            return false;
        };
        let cell = &cells[index];
        let future = self.is_future(node);
        cells.iter().enumerate().any(|(other, open)| {
            if open.set.is_some() || open.lost || other == index {
                return false;
            }
            if future {
                other < index
                    && self.member(node, cell.t_first - open.t_first, cell.t_last - open.t_first, false)
                        != Member::No
            } else {
                other > index
                    && self.member(node, open.t_first - cell.t_last, open.t_first - cell.t_first, false)
                        != Member::No
            }
        })
    }

    /// Drop the entries nothing reads any more, folding each into the
    /// instances of unbounded operators that it carries. Every fold reads the
    /// buffers as they were before any entry is dropped.
    fn drop_cells(&mut self) {
        let nodes: Vec<usize> = self.cells.keys().copied().collect();
        let mut dropped: Vec<(usize, Vec<usize>)> = Vec::new();
        for node in &nodes {
            let Some(retention) = self.shape.node(*node).map(|info| info.retention.clone()) else {
                continue;
            };
            let cells = self.cells.get(node).cloned().unwrap_or_default();
            let mut gone = Vec::new();
            for (index, cell) in cells.iter().enumerate() {
                let reported = match (cell.set, cell.rep) {
                    (Some(value), Some(rep)) => self.outcome_of(value) == rep,
                    _ => false,
                };
                if reported
                    && q(self.hz - cell.t_last) >= retention
                    && !self.needed(*node, index)
                {
                    gone.push(index);
                }
            }
            dropped.push((*node, gone));
        }
        let mut folded = Vec::new();
        for (node, gone) in &dropped {
            let cells = self.cells.get(node).cloned().unwrap_or_default();
            for index in gone {
                let cell = cells[*index];
                for (at, elem) in self.elems.iter().enumerate() {
                    if self.carrier(elem.node) == Some(*node) && cell.last >= elem.pos {
                        folded.push((at, *node, cell));
                    }
                }
            }
        }
        for (at, node, cell) in folded {
            let elem = self.elems[at];
            let (acc, pfx) = self.fold_cell(elem.node, &cell, elem.acc, elem.pfx, node);
            self.elems[at].acc = acc;
            self.elems[at].pfx = pfx;
        }
        for (node, gone) in dropped {
            if let Some(buffer) = self.cells.get_mut(&node) {
                let mut index = 0;
                buffer.retain(|_| {
                    let keep = !gone.contains(&index);
                    index += 1;
                    keep
                });
            }
        }
    }

    /// Add event `position` to every buffer; returns the results of the
    /// instances that the overflow made incomplete.
    fn push(&mut self, position: usize) -> Vec<TickResult> {
        let tick = self.events[position].tick;
        let nodes: Vec<usize> = self.cells.keys().copied().collect();
        let mut results = Vec::new();
        let mut merged_past = Vec::new();
        for node in nodes {
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
                set: None,
                rep: None,
            };
            let len = self.cells.get(&node).map_or(0, Vec::len);
            if len < capacity.max(1) {
                if let Some(buffer) = self.cells.get_mut(&node) {
                    buffer.push(cell);
                }
                continue;
            }
            let outcome = self.outcome_of(Val::lost_by(Lost::Buffer(node)));
            let Some(buffer) = self.cells.get_mut(&node) else {
                continue;
            };
            let Some(last) = buffer.last_mut() else {
                continue;
            };
            let old = *last;
            last.last = position;
            last.t_last = tick;
            last.lost = true;
            last.set = Some(Val::lost_by(Lost::Buffer(node)));
            last.rep = Some(outcome);
            if future {
                if !old.lost && old.set.is_none() {
                    results.push(TickResult {
                        subformula: node,
                        position: old.first,
                        origin: old.t_first,
                        outcome,
                    });
                }
                results.push(TickResult {
                    subformula: node,
                    position,
                    origin: tick,
                    outcome,
                });
            } else {
                merged_past.push(node);
            }
        }
        // The new instance of a past operator reads the merged entry as lost;
        // its own result is decided by the entries that remain.
        for node in merged_past {
            let value = self.compute(node, position, tick);
            results.push(TickResult {
                subformula: node,
                position,
                origin: tick,
                outcome: self.outcome_of(value),
            });
        }
        for result in &results {
            self.emitted
                .insert((result.subformula, result.position), result.outcome);
        }
        results
    }

    /// Start the instances of unbounded operators and non-interval roots at
    /// `position`, and fold the event into the live ones whose operands are
    /// temporal-free.
    fn start(&mut self, position: usize) -> Vec<TickResult> {
        let formula = self.spec.formula;
        let root = formula.root().index();
        let activated = match self.spec.activation {
            Activation::Origin => position == 0,
            Activation::Each { guarded } => !guarded || self.events[position].guard,
        };
        // Nested unbounded operators start at every position; the root starts
        // where the clause activates it, unless it is an interval operator,
        // which keeps a buffer instead.
        let starting: Vec<usize> = (0..formula.nodes().len())
            .filter(|index| {
                let temporal = self.shape.node(*index).is_some();
                let buffered = self.cells.contains_key(index);
                if *index == root {
                    !buffered && activated
                } else {
                    temporal && !buffered
                }
            })
            .collect();
        let mut results = Vec::new();
        for node in starting {
            if u64::try_from(self.elems.len()).unwrap_or(u64::MAX) >= self.spec.activation_capacity {
                self.ufinal
                    .insert((node, position), Val::lost_by(Lost::Activation));
                let outcome = self.outcome_of(Val::lost_by(Lost::Activation));
                self.emitted.insert((node, position), outcome);
                results.push(TickResult {
                    subformula: node,
                    position,
                    origin: self.events[position].tick,
                    outcome,
                });
                continue;
            }
            self.elems.push(Elem {
                node,
                pos: position,
                acc: Val::FALSE,
                pfx: Val::TRUE,
                rep: None,
            });
        }
        // Temporal-free operands fold at event time.
        let mut folded = Vec::new();
        for (at, elem) in self.elems.iter().enumerate() {
            let Some(info) = self.shape.node(elem.node) else {
                continue;
            };
            if self.carrier(elem.node).is_some() || elem.pos > position {
                continue;
            }
            let polar = |value: Val| if info.dual { value.not() } else { value };
            let psi = polar(self.expr(info.right, position));
            let phi = info
                .left
                .map_or(Val::TRUE, |left| polar(self.expr(left, position)));
            folded.push((at, elem.acc.or(psi.and(elem.pfx)), elem.pfx.and(phi)));
        }
        for (at, acc, pfx) in folded {
            self.elems[at].acc = acc;
            self.elems[at].pfx = pfx;
        }
        results
    }

    fn leaves_settled(&self, node: usize, pos: usize) -> bool {
        temporal_leaves(self.spec.formula, node).into_iter().all(|leaf| {
            match self.cells.get(&leaf) {
                Some(cells) => cells
                    .iter()
                    .find(|c| c.first <= pos && pos <= c.last)
                    .is_none_or(|c| c.lost || c.set.is_some()),
                None => self.ufinal.contains_key(&(leaf, pos)),
            }
        })
    }

    /// Report what changed. An instance is reported once when its outcome
    /// settles; the pending ones are reported again at every watermark step
    /// (FR-251-AC-14).
    fn report(&mut self, watermark: bool) -> Vec<TickResult> {
        let mut results = Vec::new();
        let nodes: Vec<usize> = self.cells.keys().copied().collect();
        for node in nodes {
            let cells = self.cells.get(&node).cloned().unwrap_or_default();
            let mut reps = Vec::new();
            for (index, cell) in cells.iter().enumerate() {
                if cell.lost {
                    continue;
                }
                let value = cell
                    .set
                    .unwrap_or_else(|| self.compute(node, cell.first, cell.t_first));
                let outcome = self.outcome_of(value);
                self.emitted.insert((node, cell.first), outcome);
                let report = if outcome == TickOutcome::Pending {
                    watermark
                } else {
                    cell.rep != Some(outcome)
                };
                reps.push((index, outcome));
                if report {
                    results.push(TickResult {
                        subformula: node,
                        position: cell.first,
                        origin: cell.t_first,
                        outcome,
                    });
                }
            }
            if let Some(buffer) = self.cells.get_mut(&node) {
                for (index, outcome) in reps {
                    buffer[index].rep = Some(outcome);
                }
            }
        }
        let mut finished = Vec::new();
        for (at, elem) in self.elems.clone().iter().enumerate() {
            let value = self.elem_val(elem);
            let outcome = self.outcome_of(value);
            self.emitted.insert((elem.node, elem.pos), outcome);
            let report = if outcome == TickOutcome::Pending {
                watermark
            } else {
                elem.rep != Some(outcome)
            };
            if report {
                results.push(TickResult {
                    subformula: elem.node,
                    position: elem.pos,
                    origin: self.events[elem.pos].tick,
                    outcome,
                });
            }
            self.elems[at].rep = Some(outcome);
            let done = value.is_final()
                || (self.shape.node(elem.node).is_none()
                    && self.leaves_settled(elem.node, elem.pos));
            if done {
                finished.push((at, value));
            }
        }
        for (at, value) in finished.into_iter().rev() {
            let elem = self.elems.remove(at);
            self.ufinal.insert((elem.node, elem.pos), value);
        }
        results
    }

    /// The reading-gap fault: every instance still `Pending` fails.
    fn gap(&mut self, gap: u64) -> Vec<TickResult> {
        let failed = TickOutcome::Failed {
            gap,
            max_reading_gap: self.spec.max_reading_gap,
        };
        let mut results = Vec::new();
        let nodes: Vec<usize> = self.cells.keys().copied().collect();
        for node in nodes {
            for cell in self.cells.get(&node).cloned().unwrap_or_default() {
                if !cell.lost && cell.rep == Some(TickOutcome::Pending) {
                    self.emitted.insert((node, cell.first), failed);
                    results.push(TickResult {
                        subformula: node,
                        position: cell.first,
                        origin: cell.t_first,
                        outcome: failed,
                    });
                }
            }
        }
        for elem in std::mem::take(&mut self.elems) {
            if elem.rep == Some(TickOutcome::Pending) {
                self.emitted.insert((elem.node, elem.pos), failed);
                results.push(TickResult {
                    subformula: elem.node,
                    position: elem.pos,
                    origin: self.events[elem.pos].tick,
                    outcome: failed,
                });
            }
        }
        for cells in self.cells.values_mut() {
            cells.clear();
        }
        self.history_start = Some(self.now);
        results.sort_by_key(|r| (r.subformula, r.position));
        results
    }

    /// The current outcome of an instance, or `None` if it does not exist.
    pub fn outcome(&self, subformula: usize, position: usize) -> Option<TickOutcome> {
        self.emitted.get(&(subformula, position)).copied()
    }

    /// The outcome of an instance evaluated afresh from the current state.
    pub fn current(&self, subformula: usize, position: usize) -> TickOutcome {
        self.outcome_of(self.temporal_val(subformula, position))
    }

    /// The `(first tick, last tick, lost)` of each cell of a buffered
    /// node's buffer, oldest first.
    pub fn cells_of(&self, node: usize) -> Vec<(i128, i128, bool)> {
        self.cells.get(&node).map_or_else(Vec::new, |cells| {
            cells
                .iter()
                .map(|cell| (cell.t_first, cell.t_last, cell.lost))
                .collect()
        })
    }

    /// The unwrapped time of the latest admitted reading.
    pub fn now(&self) -> i128 {
        self.now
    }
}

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
