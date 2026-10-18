// SPDX-License-Identifier: AGPL-3.0-or-later
//! Window aggregates over an observed trace (FR-194; ADR-024 WA-1 to WA-7).
//!
//! An [`AggregateAtom`] compares one aggregate of a past window with a
//! threshold. It is computed at atom evaluation, before temporal evaluation,
//! as a deterministic function of the trace, so the temporal layer reads a
//! Boolean per position and stays free of statistics.
//!
//! **Windows.** Under event-position, `past[a, b]` at position `i` holds the
//! positions `j` with `i - b <= j <= i - a` and `j >= 0`. Under
//! timestamped-event, it holds the positions `j <= i` whose time stamp lies
//! in `[t_i - b, t_i - a]`. A window never reads a position after `i`.
//!
//! **Values.** Values are exact rationals in one unit; converting a threshold
//! to the value's unit is the checker's step (FR-142), not this module's.
//! The caller reads the value, filter and predicate of a position through
//! [`WindowSource`].
//!
//! **Work and memory.** One work unit is charged for each window position
//! visited, whether or not its filter admits it, so a filtered scan pays
//! for every position it reads. The values a window gathers are kept, and a
//! window that would hold more than the run's `max_window_values` stops the
//! evaluation. A quantile sorts the gathered values once; that sort is
//! bounded by `max_window_values` and is not charged separately.

use std::cmp::Ordering;

use qsl_foundation::InternalFault;
use quire_exact::{Charge, ChargePoint, ComparisonOperator, Incomplete, Integer, Meter, Rational};

use super::letters::{AtomReading, LetterStop, UndefinedCause};
use super::plan::fault;
use super::position::TemporalPosition;

/// The setting that raises `max_window_values` (FR-194, FR-255's naming).
pub const MAX_WINDOW_VALUES_SETTING: &str = "monitor.max_window_values";

/// The published default of `max_window_values`: 2^20 values.
pub const DEFAULT_MAX_WINDOW_VALUES: u64 = 1 << 20;

/// The aggregate an atom computes over its window (ADR-024 WA-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AggregateKind {
    /// The number of gathered positions.
    Count,
    /// The sum of the gathered values.
    Sum,
    /// The least gathered value.
    Min,
    /// The greatest gathered value.
    Max,
    /// The share of gathered positions where the predicate holds.
    Fraction,
    /// The nearest-rank quantile: the `ceil(q * n)`-th smallest of the `n`
    /// gathered values, for `0 < q < 1`.
    Quantile(Rational),
}

/// A past window `past[a, b]` with `a <= b` (ADR-024 WA-2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PastWindow {
    /// Positions `i - b` to `i - a`, under event-position.
    Positions {
        /// The nearest offset.
        a: u64,
        /// The farthest offset.
        b: u64,
    },
    /// Time stamps `t_i - b` to `t_i - a`, under timestamped-event.
    Duration {
        /// The shortest age, non-negative.
        a: Rational,
        /// The longest age, at least `a`.
        b: Rational,
    },
}

/// Why an [`AggregateTerm`] is not well formed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TermRefusal {
    /// The window's nearest offset exceeds its farthest.
    #[error("a past window has a <= b")]
    EmptyWindow,
    /// A duration window has a negative bound.
    #[error("a duration window has non-negative bounds")]
    NegativeDuration,
    /// A quantile outside the open interval from 0 to 1.
    #[error("a quantile q has 0 < q < 1")]
    QuantileOutOfRange,
    /// `min_count` is zero.
    #[error("min_count is at least 1")]
    ZeroMinCount,
}

/// One aggregate over one window with its `min_count` (FR-194 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregateTerm {
    kind: AggregateKind,
    window: PastWindow,
    min_count: u64,
}

impl AggregateTerm {
    /// A term, refused when its window, quantile or `min_count` is
    /// ill-formed.
    pub fn new(
        kind: AggregateKind,
        window: PastWindow,
        min_count: u64,
    ) -> Result<Self, TermRefusal> {
        let zero = Rational::from_integer(Integer::zero());
        match &window {
            PastWindow::Positions { a, b } if a > b => return Err(TermRefusal::EmptyWindow),
            PastWindow::Duration { a, b } if a > b => return Err(TermRefusal::EmptyWindow),
            PastWindow::Duration { a, .. } if *a < zero => {
                return Err(TermRefusal::NegativeDuration)
            }
            PastWindow::Positions { .. } | PastWindow::Duration { .. } => {}
        }
        if let AggregateKind::Quantile(q) = &kind {
            if *q <= zero || *q >= Rational::from_integer(Integer::one()) {
                return Err(TermRefusal::QuantileOutOfRange);
            }
        }
        if min_count == 0 {
            return Err(TermRefusal::ZeroMinCount);
        }
        Ok(Self {
            kind,
            window,
            min_count,
        })
    }
}

/// A window that held fewer values than `min_count` (ADR-024 WA-4).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InsufficientData {
    /// The values the window held.
    pub count: u64,
    /// The term's `min_count`.
    pub min_count: u64,
}

/// A window that would hold more than `max_window_values` values (ADR-018
/// V-7): the run stops `Incomplete`, with no truth value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowLimitReached {
    /// The limit's name, `max_window_values`.
    pub limit: &'static str,
    /// The limit's value in this run.
    pub value: u64,
    /// The setting that raises it, `monitor.max_window_values`.
    pub setting: &'static str,
}

impl WindowLimitReached {
    fn at(value: u64) -> Self {
        Self {
            limit: "max_window_values",
            value,
            setting: MAX_WINDOW_VALUES_SETTING,
        }
    }
}

/// An observed trace as windows read it: how many positions it has and,
/// when it is timestamped, each position's time stamp.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowTrace(Stamps);

#[derive(Clone, Debug, Eq, PartialEq)]
enum Stamps {
    Positions { len: u64 },
    Timed(Vec<Rational>),
}

impl WindowTrace {
    /// A trace of `len` positions with no time stamps (event-position).
    pub fn positions(len: u64) -> Self {
        Self(Stamps::Positions { len })
    }

    /// A timestamped trace, one time stamp per position, refused when the
    /// stamps decrease.
    pub fn timed(stamps: Vec<Rational>) -> Result<Self, WindowRefusal> {
        if stamps.windows(2).any(|pair| pair[0] > pair[1]) {
            return Err(WindowRefusal::TimestampsDecrease);
        }
        Ok(Self(Stamps::Timed(stamps)))
    }

    fn len(&self) -> u64 {
        match &self.0 {
            Stamps::Positions { len } => *len,
            Stamps::Timed(stamps) => u64::try_from(stamps.len()).unwrap_or(u64::MAX),
        }
    }

    /// The positions `lo..hi` (end exclusive) the window of `term` holds at
    /// `position`.
    fn window(
        &self,
        window: &PastWindow,
        position: u64,
    ) -> Result<std::ops::Range<u64>, WindowRefusal> {
        if position >= self.len() {
            return Err(WindowRefusal::OutsideTrace { position });
        }
        match (&self.0, window) {
            (Stamps::Positions { .. }, PastWindow::Positions { a, b }) => {
                Ok(match position.checked_sub(*a) {
                    Some(newest) => position.saturating_sub(*b)..newest.saturating_add(1),
                    None => 0..0,
                })
            }
            (Stamps::Timed(stamps), PastWindow::Duration { a, b }) => {
                let upto = usize::try_from(position)
                    .ok()
                    .and_then(|index| stamps.get(..=index))
                    .ok_or(WindowRefusal::OutsideTrace { position })?;
                let now = upto
                    .last()
                    .ok_or(WindowRefusal::OutsideTrace { position })?;
                let oldest = now.sub(b);
                let newest = now.sub(a);
                let lo = upto.partition_point(|stamp| *stamp < oldest);
                let hi = upto.partition_point(|stamp| *stamp <= newest);
                let widen = |index: usize| u64::try_from(index).unwrap_or(u64::MAX);
                Ok(widen(lo)..widen(hi.max(lo)))
            }
            (Stamps::Positions { .. }, PastWindow::Duration { .. })
            | (Stamps::Timed(_), PastWindow::Positions { .. }) => {
                Err(WindowRefusal::WindowKindMismatch)
            }
        }
    }
}

/// Why a window cannot be read on a trace.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WindowRefusal {
    /// The position is not on the trace.
    #[error("position {position} is outside the trace")]
    OutsideTrace {
        /// The refused position.
        position: u64,
    },
    /// A position window on a timestamped trace, or a duration window on a
    /// trace without time stamps.
    #[error("the window's kind does not match the trace's profile")]
    WindowKindMismatch,
    /// Time stamps are not in non-decreasing order.
    #[error("time stamps do not decrease along a trace")]
    TimestampsDecrease,
}

/// Reads what an aggregate gathers at a trace position. The caller owns the
/// observations and their expressions (FR-107); each method charges the
/// meter for its own work.
pub trait WindowSource {
    /// Whether the term's filter holds at `position`; `true` when the term
    /// has no filter.
    fn admits(&mut self, position: TemporalPosition, meter: &mut Meter)
        -> Result<bool, Incomplete>;

    /// The value expression at `position`. Read for `sum`, `min`, `max` and
    /// `quantile`.
    fn value(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<Rational, Incomplete>;

    /// Whether the predicate of `fraction holds(P)` holds at `position`.
    fn predicate(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<bool, Incomplete>;
}

/// An aggregate compared with a threshold: one atom of a monitor (ADR-024
/// WA-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregateAtom {
    term: AggregateTerm,
    operator: ComparisonOperator,
    threshold: Rational,
}

/// What a window has gathered so far, per aggregate kind.
enum Gathered {
    Count,
    Sum(Rational),
    Min(Option<Rational>),
    Max(Option<Rational>),
    Fraction { holding: u64 },
    Quantile(Vec<Rational>),
}

impl Gathered {
    fn new(kind: &AggregateKind) -> Self {
        match kind {
            AggregateKind::Count => Self::Count,
            AggregateKind::Sum => Self::Sum(Rational::from_integer(Integer::zero())),
            AggregateKind::Min => Self::Min(None),
            AggregateKind::Max => Self::Max(None),
            AggregateKind::Fraction => Self::Fraction { holding: 0 },
            AggregateKind::Quantile(_) => Self::Quantile(Vec::new()),
        }
    }

    /// Fold in the gathered `position`.
    fn take<S: WindowSource>(
        &mut self,
        source: &mut S,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<(), Incomplete> {
        match self {
            Self::Count => {}
            Self::Sum(total) => *total = total.add(&source.value(position, meter)?),
            Self::Min(least) => {
                let value = source.value(position, meter)?;
                if least.as_ref().is_none_or(|held| value < *held) {
                    *least = Some(value);
                }
            }
            Self::Max(greatest) => {
                let value = source.value(position, meter)?;
                if greatest.as_ref().is_none_or(|held| value > *held) {
                    *greatest = Some(value);
                }
            }
            Self::Fraction { holding } => {
                if source.predicate(position, meter)? {
                    *holding = holding.saturating_add(1);
                }
            }
            Self::Quantile(values) => values.push(source.value(position, meter)?),
        }
        Ok(())
    }

    /// The aggregate over `count >= 1` gathered values.
    fn finish(self, kind: &AggregateKind, count: u64) -> Result<Rational, InternalFault> {
        let missing = || fault("aggregate-has-a-gathered-value");
        match self {
            Self::Count => Ok(Rational::from_integer(Integer::from(count))),
            Self::Sum(total) => Ok(total),
            Self::Min(value) | Self::Max(value) => value.ok_or_else(missing),
            Self::Fraction { holding } => {
                Rational::new(Integer::from(holding), Integer::from(count)).map_err(|_| missing())
            }
            Self::Quantile(mut values) => {
                let AggregateKind::Quantile(q) = kind else {
                    return Err(fault("quantile-state-has-a-quantile"));
                };
                values.sort();
                let rank = nearest_rank(q, count).ok_or_else(missing)?;
                let index = usize::try_from(rank)
                    .ok()
                    .and_then(|rank| rank.checked_sub(1))
                    .ok_or_else(missing)?;
                slot_cloned(&values, index)
            }
        }
    }
}

fn slot_cloned(values: &[Rational], index: usize) -> Result<Rational, InternalFault> {
    values
        .get(index)
        .cloned()
        .ok_or_else(|| fault("quantile-rank-in-window"))
}

/// `ceil(q * n)`, the 1-based rank of the nearest-rank quantile.
fn nearest_rank(q: &Rational, n: u64) -> Option<u64> {
    let scaled = q.mul(&Rational::from_integer(Integer::from(n)));
    let denominator = quire_exact::NonZeroInteger::try_from(scaled.denominator()).ok()?;
    let (floor, remainder) = scaled.numerator().div_mod_floor(denominator);
    let rank = if remainder.is_zero() {
        floor
    } else {
        floor.add(&Integer::one())
    };
    rank.to_u64()
}

impl AggregateAtom {
    /// The atom `term <operator> threshold`.
    pub fn new(term: AggregateTerm, operator: ComparisonOperator, threshold: Rational) -> Self {
        Self {
            term,
            operator,
            threshold,
        }
    }

    /// The atom at `position` of `trace`: true or false, or undefined when
    /// the window holds fewer than `min_count` values. A window that would
    /// hold more than `max_window_values` values stops the evaluation.
    pub fn evaluate<S: WindowSource>(
        &self,
        trace: &WindowTrace,
        source: &mut S,
        max_window_values: u64,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let window = trace.window(&self.term.window, position.index())?;
        let mut gathered = Gathered::new(&self.term.kind);
        let mut count = 0_u64;
        for index in window {
            meter.charge(Charge::new(ChargePoint::CollectionVisit))?;
            let at = TemporalPosition::new(index);
            if !source.admits(at, meter)? {
                continue;
            }
            if count >= max_window_values {
                return Err(LetterStop::WindowLimit(WindowLimitReached::at(
                    max_window_values,
                )));
            }
            count += 1;
            gathered.take(source, at, meter)?;
        }
        if count < self.term.min_count {
            return Ok(AtomReading::Undefined(UndefinedCause::InsufficientData(
                InsufficientData {
                    count,
                    min_count: self.term.min_count,
                },
            )));
        }
        let aggregate = gathered.finish(&self.term.kind, count)?;
        let ordering: Ordering = aggregate.cmp(&self.threshold);
        Ok(if self.operator.holds(ordering) {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}
