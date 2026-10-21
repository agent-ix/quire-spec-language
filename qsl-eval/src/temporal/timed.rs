// SPDX-License-Identifier: AGPL-3.0-or-later
//! The layer-5 reference evaluator of a clause under the timed profile
//! `quire.temporal.timed/v1` (QSL FR-234 "Pointwise meaning"; QSpec FR-416;
//! ADR-026 DF-3, DF-7).
//!
//! A formula is read at positions. Each position carries an exact rational
//! time stamp, non-negative and non-decreasing; positions with equal stamps
//! keep their step order. `eventually_I p` holds at `i` when some `j >= i`
//! has `τ_j − τ_i` in `I` and `p` at `j`; `p until_I q` when some such `j`
//! has `q` and every `k` with `i <= k < j` has `p`; `once`, `historically`,
//! `since` and `triggered` read `τ_i − τ_j` over `j <= i`; `always` and
//! `release` are the duals. An operator with no interval reads `[0, ∞)`.
//! Membership of a difference in an interval is decided in exact arithmetic
//! with each end's openness ([`TimedInterval::contains`]).
//!
//! The trace is a finite prefix of a time-divergent behaviour whose
//! continuation is unknown (FR-231 "Inputs"): a position past the prefix
//! has a stamp no less than the last one and an unknown letter, so values
//! are Kleene's three. A future operator whose interval still reaches past
//! the last stamp is `Unknown` unless the prefix decides it; past operators
//! read only represented positions, and no position precedes 0.
//!
//! A timed lasso (FR-231 "Evaluation", ADR-026 CT-2, CT-3) repeats its loop
//! forever: the position `k` past the represented trace is the loop position
//! it wraps to, with its stamp grown by the loop's total delay `D` once per
//! repetition. The evaluator unrolls the loop enough times that every node's
//! value is the same on every later repetition, then reads a future scan
//! that runs off the unrolled trace through the last repetition, shifting
//! stamps by `D`; a scan over an unbounded window stops once it has covered
//! one full period past the window's lower end. Every value of a lasso is
//! two-valued except an undefined atom's.
//!
//! Work: one unit for each atom read at a position, one for each (node,
//! position) value and one for each window position a scan reads.

use qsl_foundation::bound::TimedInterval;
use qsl_semantics::check::{
    BinaryTemporal, CheckedFormula, CheckedInterval, CheckedTemporalClause, FormulaNode,
    TemporalProfile, UnaryTemporal,
};
use quire_exact::{Charge, ChargePoint, Incomplete, Integer, Meter, NonZeroInteger, Rational};

use super::letters::{AtomReading, LetterStop, TemporalLetters};
use super::outcome::{TemporalOutcome, TemporalRefusal, UndefinedEvaluation};
use super::plan::fault;
use super::position::{PositionRefusal, TemporalPosition};

/// The named point every unit of work charges, as the step evaluator does.
const VISIT: ChargePoint = ChargePoint::GraphExpand;

/// A finite timed trace (FR-231 "Inputs"): the time stamp of each position,
/// in the clause binding's unit, non-negative and non-decreasing. It is a
/// prefix of a time-divergent behaviour whose continuation is unknown.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimedTrace {
    stamps: Vec<Rational>,
    /// The loop a timed lasso repeats forever; none for a finite prefix.
    looping: Option<TimedLoop>,
}

/// The loop of a timed lasso (ADR-026 CT-2): the position its end re-enters
/// and its total delay.
#[derive(Clone, Debug, Eq, PartialEq)]
struct TimedLoop {
    start: usize,
    delay: Rational,
}

impl TimedTrace {
    /// The trace whose position `i` has the stamp `stamps[i]`, refusing an
    /// empty trace, a negative stamp and a stamp below its predecessor.
    pub fn new(stamps: Vec<Rational>) -> Result<Self, TemporalRefusal> {
        let zero = Rational::from_integer(Integer::zero());
        let mut previous = &zero;
        for (index, stamp) in stamps.iter().enumerate() {
            if stamp < previous {
                let position = TemporalPosition::new(u64::try_from(index).unwrap_or(u64::MAX));
                return Err(if stamp < &zero {
                    TemporalRefusal::NegativeStamp { position }
                } else {
                    TemporalRefusal::DecreasingStamp { position }
                });
            }
            previous = stamp;
        }
        if stamps.is_empty() {
            return Err(TemporalRefusal::EmptyTrace);
        }
        Ok(Self {
            stamps,
            looping: None,
        })
    }

    /// The timed lasso whose represented positions have `stamps` and whose
    /// end re-enters position `loop_start` after the loop's total delay
    /// `loop_delay` (ADR-026 CT-2): the position after the last is
    /// `loop_start` again, stamped `stamps[loop_start] + loop_delay`. Refuses
    /// what [`Self::new`] refuses, a `loop_start` that is not a represented
    /// position, a non-positive delay (the behaviour would not be
    /// time-divergent), and a delay that puts the re-entered position before
    /// the last represented one.
    pub fn lasso(
        stamps: Vec<Rational>,
        loop_start: u64,
        loop_delay: Rational,
    ) -> Result<Self, TemporalRefusal> {
        let trace = Self::new(stamps)?;
        let start = usize::try_from(loop_start)
            .ok()
            .filter(|start| *start < trace.stamps.len())
            .ok_or(TemporalRefusal::EmptyLoop)?;
        if loop_delay <= Rational::from_integer(Integer::zero()) {
            return Err(TemporalRefusal::NonPositiveLoopDelay);
        }
        let reentry = trace.stamps[start].add(&loop_delay);
        if trace.stamps.last().is_some_and(|last| &reentry < last) {
            return Err(TemporalRefusal::LoopReentersBeforeLastStamp);
        }
        Ok(Self {
            looping: Some(TimedLoop {
                start,
                delay: loop_delay,
            }),
            ..trace
        })
    }

    /// The time stamp of each represented position, in position order.
    pub fn stamps(&self) -> &[Rational] {
        &self.stamps
    }

    /// Whether this is a timed lasso.
    pub fn is_lasso(&self) -> bool {
        self.looping.is_some()
    }
}

/// Kleene's three values, ordered `False < Unknown < True`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Tri {
    False,
    Unknown,
    True,
}

impl Tri {
    fn of(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::False
        }
    }

    fn negate(self) -> Self {
        match self {
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
            Self::True => Self::False,
        }
    }

    fn flip(self, dual: bool) -> Self {
        if dual {
            self.negate()
        } else {
            self
        }
    }
}

/// Why evaluation stopped before a value.
enum Stop {
    Incomplete(Incomplete),
    Refused(TemporalRefusal),
}

impl From<Incomplete> for Stop {
    fn from(incomplete: Incomplete) -> Self {
        Self::Incomplete(incomplete)
    }
}

impl From<LetterStop> for Stop {
    fn from(stop: LetterStop) -> Self {
        match stop {
            LetterStop::Incomplete(incomplete) => Self::Incomplete(incomplete),
            LetterStop::Window(refusal) => Self::Refused(TemporalRefusal::Window(refusal)),
            LetterStop::Fault(internal) => Self::Refused(TemporalRefusal::Fault(internal)),
            // A timed clause's atoms are `holds` conditions, which hold no
            // window aggregate.
            LetterStop::WindowLimit(_) => {
                Self::Refused(TemporalRefusal::Fault(fault("timed-atom-holds-no-window")))
            }
        }
    }
}

fn broken(invariant: &'static str) -> Stop {
    Stop::Refused(TemporalRefusal::Fault(fault(invariant)))
}

/// The interval an operator reads: its timed interval, or `[0, ∞)` for an
/// unbounded operator.
#[derive(Clone, Copy)]
struct Window<'a>(Option<&'a TimedInterval>);

impl<'a> Window<'a> {
    fn of(interval: Option<&'a CheckedInterval>) -> Result<Self, Stop> {
        match interval {
            None => Ok(Self(None)),
            Some(CheckedInterval::Timed(interval)) => Ok(Self(Some(interval))),
            Some(CheckedInterval::Steps(_)) => Err(broken("timed-clause-holds-timed-intervals")),
        }
    }

    /// Whether the difference `offset >= 0` lies in the window.
    fn contains(self, offset: &Rational) -> bool {
        self.0.is_none_or(|interval| interval.contains(offset))
    }

    /// Whether some difference of the window is at least `offset`.
    fn reaches(self, offset: &Rational) -> bool {
        self.0.is_none_or(|interval| interval.reaches(offset))
    }

    /// Whether `offset` lies a full `period` past the window's lower end,
    /// so a periodic scan has met every position class of the loop.
    fn covers_a_period(self, offset: &Rational, period: &Rational) -> bool {
        let lower = self.0.map(TimedInterval::lower);
        let end = lower.map_or_else(|| period.clone(), |lower| lower.add(period));
        offset >= &end
    }
}

/// The repetition of a timed lasso's last unrolled loop copy (FR-231
/// "Evaluation"): a position past the unrolled trace is its image in that
/// copy, `loop_len` positions long, its stamp grown by `delay` per
/// repetition.
#[derive(Clone, Copy)]
struct Tail<'a> {
    loop_len: usize,
    delay: &'a Rational,
}

/// The reference evaluator over one trace.
struct Evaluator<'a> {
    stamps: &'a [Rational],
    /// The lasso's repetition past the last stamp; none for a finite prefix.
    tail: Option<Tail<'a>>,
    meter: &'a mut Meter,
    /// The value of each formula node at each position, once computed.
    values: Vec<Vec<Tri>>,
}

impl Evaluator<'_> {
    fn charge(&mut self) -> Result<(), Stop> {
        self.meter.charge(Charge::new(VISIT))?;
        Ok(())
    }

    fn value(&self, node: usize, position: usize) -> Result<Tri, Stop> {
        self.values
            .get(node)
            .and_then(|values| values.get(position))
            .copied()
            .ok_or_else(|| broken("operand-valued-first"))
    }

    fn stamp(&self, position: usize) -> Result<&Rational, Stop> {
        self.stamps
            .get(position)
            .ok_or_else(|| broken("position-has-a-stamp"))
    }

    /// `eventually`/`until` at `i` (`always`/`release` when `dual`): the
    /// witness `witness`, guarded by `guard` over the positions before it.
    fn future(
        &mut self,
        i: usize,
        window: Window<'_>,
        guard: Option<usize>,
        witness: usize,
        dual: bool,
    ) -> Result<Tri, Stop> {
        let here = self.stamp(i)?.clone();
        let mut acc = Tri::False;
        let mut held = Tri::True;
        for j in i..self.stamps.len() {
            self.charge()?;
            let offset = self.stamp(j)?.sub(&here);
            if !window.reaches(&offset) {
                // Every later position, represented or not, lies past the
                // window.
                return Ok(acc.flip(dual));
            }
            if window.contains(&offset) {
                let found = self.value(witness, j)?.flip(dual);
                acc = acc.max(found.min(held));
            }
            if let Some(guard) = guard {
                held = held.min(self.value(guard, j)?.flip(dual));
            }
            if acc == Tri::True || held == Tri::False {
                return Ok(acc.flip(dual));
            }
        }
        if let Some(tail) = self.tail {
            return self.future_past_the_trace(tail, here, window, guard, witness, dual, acc, held);
        }
        // A position past the prefix has a stamp of at least the last one.
        let last = self
            .stamps
            .last()
            .ok_or_else(|| broken("trace-not-empty"))?
            .sub(&here);
        if window.reaches(&last) {
            acc = acc.max(held.min(Tri::Unknown));
        }
        Ok(acc.flip(dual))
    }

    /// The rest of a future scan over a lasso, from the position after the
    /// unrolled trace: each position is its image in the last loop copy,
    /// shifted by whole delays. `acc` and `held` are the scan so far.
    #[allow(clippy::too_many_arguments)]
    fn future_past_the_trace(
        &mut self,
        tail: Tail<'_>,
        here: Rational,
        window: Window<'_>,
        guard: Option<usize>,
        witness: usize,
        dual: bool,
        mut acc: Tri,
        mut held: Tri,
    ) -> Result<Tri, Stop> {
        let len = self.stamps.len();
        let first = len
            .checked_sub(tail.loop_len)
            .ok_or_else(|| broken("loop-within-trace"))?;
        let mut shift = Integer::one();
        let mut copy = first;
        loop {
            self.charge()?;
            let stamp = self
                .stamp(copy)?
                .add(&tail.delay.mul(&Rational::from_integer(shift.clone())));
            let offset = stamp.sub(&here);
            if !window.reaches(&offset) {
                return Ok(acc.flip(dual));
            }
            if window.contains(&offset) {
                let found = self.value(witness, copy)?.flip(dual);
                acc = acc.max(found.min(held));
            }
            if let Some(guard) = guard {
                held = held.min(self.value(guard, copy)?.flip(dual));
            }
            if acc == Tri::True || held == Tri::False || window.covers_a_period(&offset, tail.delay)
            {
                return Ok(acc.flip(dual));
            }
            copy += 1;
            if copy == len {
                copy = first;
                shift = shift.add(&Integer::one());
            }
        }
    }

    /// `once`/`since` at `i` (`historically`/`triggered` when `dual`).
    fn past(
        &mut self,
        i: usize,
        window: Window<'_>,
        guard: Option<usize>,
        witness: usize,
        dual: bool,
    ) -> Result<Tri, Stop> {
        let here = self.stamp(i)?.clone();
        let mut acc = Tri::False;
        let mut held = Tri::True;
        for j in (0..=i).rev() {
            self.charge()?;
            let offset = here.sub(self.stamp(j)?);
            if !window.reaches(&offset) {
                break;
            }
            if window.contains(&offset) {
                let found = self.value(witness, j)?.flip(dual);
                acc = acc.max(found.min(held));
            }
            if let Some(guard) = guard {
                held = held.min(self.value(guard, j)?.flip(dual));
            }
            if acc == Tri::True || held == Tri::False {
                break;
            }
        }
        Ok(acc.flip(dual))
    }

    /// The value of the non-atom `node` at position `i`, its operands
    /// valued at every position already.
    fn node(&mut self, node: &FormulaNode, i: usize) -> Result<Tri, Stop> {
        self.charge()?;
        Ok(match node {
            FormulaNode::Constant(value) => Tri::of(*value),
            FormulaNode::Holds { .. } => return Err(broken("atom-read-from-letters")),
            FormulaNode::Not(operand) => self.value(operand.index(), i)?.negate(),
            FormulaNode::And(left, right) => self
                .value(left.index(), i)?
                .min(self.value(right.index(), i)?),
            FormulaNode::Or(left, right) => self
                .value(left.index(), i)?
                .max(self.value(right.index(), i)?),
            FormulaNode::Implies(left, right) => self
                .value(left.index(), i)?
                .negate()
                .max(self.value(right.index(), i)?),
            FormulaNode::Unary {
                operator,
                interval,
                operand,
            } => {
                let window = Window::of(interval.as_ref())?;
                let operand = operand.index();
                match operator {
                    UnaryTemporal::Eventually => self.future(i, window, None, operand, false)?,
                    UnaryTemporal::Always => self.future(i, window, None, operand, true)?,
                    UnaryTemporal::Once => self.past(i, window, None, operand, false)?,
                    UnaryTemporal::Historically => self.past(i, window, None, operand, true)?,
                }
            }
            FormulaNode::Binary {
                operator,
                interval,
                left,
                right,
            } => {
                let window = Window::of(interval.as_ref())?;
                let (left, right) = (Some(left.index()), right.index());
                match operator {
                    BinaryTemporal::Until => self.future(i, window, left, right, false)?,
                    BinaryTemporal::Release => self.future(i, window, left, right, true)?,
                    BinaryTemporal::Since => self.past(i, window, left, right, false)?,
                    BinaryTemporal::Triggered => self.past(i, window, left, right, true)?,
                }
            }
        })
    }
}

/// Read every atom of `formula` at every represented position of the
/// trace, in position order and then atom order, stopping at the first
/// undefined atom: it and every later position read `Unknown`. The unrolled
/// copies of a lasso's loop read the letters of the positions they repeat.
fn read_atoms<L: TemporalLetters>(
    formula: &CheckedFormula,
    evaluator: &mut Evaluator<'_>,
    letters: &mut L,
    represented: usize,
    loop_start: usize,
) -> Result<Option<UndefinedEvaluation>, Stop> {
    let atoms: Vec<usize> = formula
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node, FormulaNode::Holds { .. }))
        .map(|(index, _)| index)
        .collect();
    for position in 0..represented {
        let at =
            TemporalPosition::new(u64::try_from(position).map_err(|_| broken("position-in-u64"))?);
        for &atom in &atoms {
            evaluator.charge()?;
            let id = formula
                .node_id(atom)
                .ok_or_else(|| broken("atom-has-node-id"))?;
            let value = match letters.atom(id, at, &mut *evaluator.meter)? {
                AtomReading::True => Tri::True,
                AtomReading::False => Tri::False,
                AtomReading::Undefined(cause) => {
                    for &atom in &atoms {
                        if let Some(values) = evaluator.values.get_mut(atom) {
                            values.truncate(position);
                            values.resize(evaluator.stamps.len(), Tri::Unknown);
                        }
                    }
                    return Ok(Some(UndefinedEvaluation { at, cause }));
                }
            };
            let slot = evaluator
                .values
                .get_mut(atom)
                .and_then(|values| values.get_mut(position))
                .ok_or_else(|| broken("atom-slot-allocated"))?;
            *slot = value;
        }
    }
    // Position `k >= represented` repeats `loop_start + (k - loop_start) %
    // loop_len`.
    let loop_len = represented - loop_start;
    for &atom in &atoms {
        if let Some(values) = evaluator.values.get_mut(atom) {
            for position in represented..values.len() {
                let image = loop_start + (position - loop_start) % loop_len;
                values[position] = values[image];
            }
        }
    }
    Ok(None)
}

/// Evaluate `clause`'s formula at `position` of the timed `trace`, ignoring
/// its activation and fairness (FR-234 "Pointwise meaning"): `Holds` and
/// `Violated` are the formula's value there, `Pending` a value the prefix
/// does not decide, and `Undefined` an atom with no value at a position
/// the value depends on.
///
/// Refuses a clause not checked under the timed profile and a position the
/// trace does not represent.
pub fn evaluate_timed_at<L: TemporalLetters>(
    clause: &CheckedTemporalClause,
    trace: &TimedTrace,
    position: TemporalPosition,
    letters: &mut L,
    meter: &mut Meter,
) -> Result<TemporalOutcome, TemporalRefusal> {
    if clause.profile() != TemporalProfile::Timed {
        return Err(TemporalRefusal::NotTimedProfile {
            profile: clause.profile(),
        });
    }
    let formula = clause.formula();
    let represented = trace.stamps.len();
    let (stamps, tail, loop_start) = match &trace.looping {
        None => (trace.stamps.clone(), None, 0),
        Some(looping) => match unrolled(trace, looping, formula, meter) {
            Ok(stamps) => (stamps, Some(looping), looping.start),
            Err(Stop::Incomplete(incomplete)) => {
                return Ok(TemporalOutcome::Incomplete(incomplete))
            }
            Err(Stop::Refused(refusal)) => return Err(refusal),
        },
    };
    let loop_len = represented - loop_start;
    let at = match &trace.looping {
        None => usize::try_from(position.index())
            .ok()
            .filter(|at| *at < represented)
            .ok_or(PositionRefusal::OutsideTrace { position })?,
        // A position past the represented trace is read at the same place
        // of the last unrolled copy, where every value has settled.
        Some(_) => {
            let position =
                usize::try_from(position.index()).map_err(|_| TemporalRefusal::TraceTooLong)?;
            if position < represented {
                position
            } else {
                stamps.len() - loop_len + (position - loop_start) % loop_len
            }
        }
    };
    let mut evaluator = Evaluator {
        stamps: &stamps,
        tail: tail.map(|looping| Tail {
            loop_len,
            delay: &looping.delay,
        }),
        meter,
        values: vec![vec![Tri::Unknown; stamps.len()]; formula.nodes().len()],
    };
    match evaluate(
        formula,
        &mut evaluator,
        letters,
        at,
        represented,
        loop_start,
    ) {
        Ok((value, undefined)) => Ok(match (value, undefined) {
            (Tri::False, _) => TemporalOutcome::Violated { at: position },
            (_, Some(undefined)) => TemporalOutcome::Undefined(undefined),
            (Tri::True, None) => TemporalOutcome::Holds,
            (Tri::Unknown, None) => TemporalOutcome::Pending,
        }),
        Err(Stop::Incomplete(incomplete)) => Ok(TemporalOutcome::Incomplete(incomplete)),
        Err(Stop::Refused(refusal)) => Err(refusal),
    }
}

/// The value of `formula`'s root at `at`, and the first undefined atom.
/// The arena puts an operator before its operands, so a reverse pass values
/// every operand first.
fn evaluate<L: TemporalLetters>(
    formula: &CheckedFormula,
    evaluator: &mut Evaluator<'_>,
    letters: &mut L,
    at: usize,
    represented: usize,
    loop_start: usize,
) -> Result<(Tri, Option<UndefinedEvaluation>), Stop> {
    let undefined = read_atoms(formula, evaluator, letters, represented, loop_start)?;
    for (index, node) in formula.nodes().iter().enumerate().rev() {
        if matches!(node, FormulaNode::Holds { .. }) {
            continue;
        }
        for position in 0..evaluator.stamps.len() {
            let value = evaluator.node(node, position)?;
            let slot = evaluator
                .values
                .get_mut(index)
                .and_then(|values| values.get_mut(position))
                .ok_or_else(|| broken("node-slot-allocated"))?;
            *slot = value;
        }
    }
    Ok((evaluator.value(formula.root().index(), at)?, undefined))
}

/// The stamps of `trace`'s lasso with its loop repeated until every node's
/// value is the same on every later repetition (ADR-026 CT-3): one
/// repetition for each temporal node, for the window of each finite bound,
/// and two more. Each repetition adds the loop's total delay to the stamps
/// of the loop's positions.
fn unrolled(
    trace: &TimedTrace,
    looping: &TimedLoop,
    formula: &CheckedFormula,
    meter: &mut Meter,
) -> Result<Vec<Rational>, Stop> {
    let reach = formula
        .nodes()
        .iter()
        .filter_map(|node| match node {
            FormulaNode::Unary { interval, .. } | FormulaNode::Binary { interval, .. } => {
                match interval {
                    Some(CheckedInterval::Timed(interval)) => Some(interval.upper()),
                    Some(CheckedInterval::Steps(_)) | None => None,
                }
            }
            _ => None,
        })
        .fold(Rational::from_integer(Integer::zero()), |sum, upper| {
            sum.add(upper)
        });
    let temporal = formula
        .nodes()
        .iter()
        .filter(|node| matches!(node, FormulaNode::Unary { .. } | FormulaNode::Binary { .. }))
        .count();
    let periods = reach
        .div(&looping.delay)
        .and_then(|periods| {
            let denominator = NonZeroInteger::try_from(periods.denominator()).ok()?;
            Some(periods.numerator().div_mod_floor(denominator).0)
        })
        .and_then(|periods| periods.to_u64())
        .and_then(|periods| periods.checked_add(u64::try_from(temporal).ok()?))
        .and_then(|copies| copies.checked_add(3))
        .ok_or(Stop::Refused(TemporalRefusal::TraceTooLong))?;
    let loop_stamps = &trace.stamps[looping.start..];
    let mut stamps = trace.stamps.clone();
    let mut shift = Rational::from_integer(Integer::zero());
    for _ in 0..periods {
        shift = shift.add(&looping.delay);
        for _ in loop_stamps {
            meter.charge(Charge::new(VISIT))?;
        }
        stamps.extend(loop_stamps.iter().map(|stamp| stamp.add(&shift)));
    }
    Ok(stamps)
}
