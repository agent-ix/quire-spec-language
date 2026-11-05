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
//! [`WindowSource`], which may find any of them undefined.
//!
//! **Undefined positions.** A window is read oldest position first and the
//! read stops at the first position whose filter, value or predicate is
//! undefined: the atom is undefined with that cause, whatever the values
//! before it would have summed to and whatever `min_count` asks, and the
//! positions after it are neither read nor charged. A filter that rejects a
//! position keeps that position's value from being read.
//!
//! **Work and memory.** One work unit is charged at `temporal.visit` for each window position
//! visited, whether or not its filter admits it, so a filtered scan pays
//! for every position it reads. The values a window gathers are kept, and a
//! window that would hold more than the run's `max_window_values` stops the
//! evaluation. A quantile sorts the gathered values once; that sort is
//! bounded by `max_window_values` and is not charged separately. A position
//! whose value is undefined holds no value, so the undefined position is
//! reported and not the overflow it would otherwise have been the last to
//! reach.

use std::cmp::Ordering;

use qsl_semantics::check::{
    AggregateComparison, AggregateKind as CheckedKind, AggregateWindow, CheckedAggregate,
};
use quire_exact::{Charge, ChargePoint, ComparisonOperator, Integer, Meter, Rational};
use quire_semantic_value::catalog::InternalFault;

use super::finite_window::Valuation;
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
    /// `min_count` is zero on a term that has no value over an empty set.
    #[error("min_count is at least 1 on a min, max, fraction or quantile term")]
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
        // QSpec FR-414: a `count` or `sum` over no values is 0, so its
        // `min_count` may be 0; `min`, `max`, `fraction` and `quantile` have
        // no value over an empty set.
        if min_count == 0 && !matches!(kind, AggregateKind::Count | AggregateKind::Sum) {
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
    /// The maximum number of values the window may hold.
    pub limit: u64,
    /// The first value count that exceeds the limit.
    pub value: u64,
    /// The setting that raises it, `monitor.max_window_values`.
    pub setting: &'static str,
    /// The position at which the window tried to exceed the limit.
    pub position: TemporalPosition,
}

impl WindowLimitReached {
    fn at(limit: u64, value: u64, position: TemporalPosition) -> Self {
        Self {
            limit,
            value,
            setting: MAX_WINDOW_VALUES_SETTING,
            position,
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

/// What a [`WindowSource`] read at one position: the value, or the cause
/// for which the filter, value or predicate there is undefined.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WindowRead<T> {
    /// The read is defined.
    Defined(T),
    /// The read is undefined (ADR-018 UE-2).
    Undefined(UndefinedCause),
    /// The observation owner supplied no sample at this window position.
    Missing,
}

/// Reads what an aggregate gathers at a trace position. The caller owns the
/// observations and their expressions (FR-107); each method charges the
/// meter for its own work. A run stop is `Err`; an undefined read is
/// [`WindowRead::Undefined`] and stops only the atom.
pub trait WindowSource {
    /// Whether the term's filter holds at `position`; `true` when the term
    /// has no filter.
    fn admits(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop>;

    /// The value expression at `position`. Read for `sum`, `min`, `max` and
    /// `quantile`.
    fn value(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<Rational>, LetterStop>;

    /// Whether the predicate of `fraction holds(P)` holds at `position`.
    fn predicate(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop>;
}

/// What the observation owner supplies for an aggregate atom at one
/// position (FR-516 "Window aggregate atoms"): the filter, value and predicate
/// are the owner's, so the evaluator is handed their result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AggregateSample {
    /// The filter is false at the position, or the position is not admitted.
    Excluded,
    /// The value expression's exact value, in the aggregate's value unit.
    Value(Rational),
    /// The `fraction` predicate's value.
    Holds(bool),
    /// The filter holds, for `count`.
    Counted,
    /// The filter, the value or the predicate is undefined there.
    Undefined(UndefinedCause),
}

impl AggregateSample {
    /// Whether a term of `kind` takes this sample: `Excluded` and `Undefined`
    /// fit every kind, `Counted` only `count`, `Holds` only `fraction`, and
    /// `Value` the kinds that read a value expression.
    pub fn is_taken_by(&self, kind: &AggregateKind) -> bool {
        match self {
            Self::Excluded | Self::Undefined(_) => true,
            Self::Counted => matches!(kind, AggregateKind::Count),
            Self::Holds(_) => matches!(kind, AggregateKind::Fraction),
            Self::Value(_) => matches!(
                kind,
                AggregateKind::Sum
                    | AggregateKind::Min
                    | AggregateKind::Max
                    | AggregateKind::Quantile(_)
            ),
        }
    }
}

/// A [`WindowSource`] over the samples the observation owner supplied, one
/// per trace position, indexed by position, for a term of one kind: a sample
/// the kind does not take, and a position with no sample, are faults.
///
/// Nothing outside tests builds one yet: the package route's request carries
/// no `aggregate_samples` until the package-route aggregates land (FR-516).
#[derive(Clone, Debug)]
pub struct SampledWindow<'a> {
    samples: &'a [AggregateSample],
    kind: AggregateKind,
}

impl<'a> SampledWindow<'a> {
    /// A source over `samples`, `samples[i]` being the sample at position
    /// `i`, for a term of `kind`.
    pub fn new(samples: &'a [AggregateSample], kind: &AggregateKind) -> Self {
        Self {
            samples,
            kind: kind.clone(),
        }
    }

    fn at(&self, position: TemporalPosition) -> Result<&'a AggregateSample, LetterStop> {
        let sample = usize::try_from(position.index())
            .ok()
            .and_then(|index| self.samples.get(index))
            .ok_or_else(|| fault("aggregate-sample-unsupplied-position"))?;
        if sample.is_taken_by(&self.kind) {
            Ok(sample)
        } else {
            Err(fault("aggregate-sample-kind").into())
        }
    }
}

impl WindowSource for SampledWindow<'_> {
    fn admits(
        &mut self,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        Ok(match self.at(position)? {
            AggregateSample::Excluded => WindowRead::Defined(false),
            AggregateSample::Undefined(cause) => WindowRead::Undefined(cause.clone()),
            AggregateSample::Value(_) | AggregateSample::Holds(_) | AggregateSample::Counted => {
                WindowRead::Defined(true)
            }
        })
    }

    fn value(
        &mut self,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<WindowRead<Rational>, LetterStop> {
        match self.at(position)? {
            AggregateSample::Value(value) => Ok(WindowRead::Defined(value.clone())),
            AggregateSample::Undefined(cause) => Ok(WindowRead::Undefined(cause.clone())),
            AggregateSample::Excluded | AggregateSample::Holds(_) | AggregateSample::Counted => {
                Err(fault("aggregate-sample-kind").into())
            }
        }
    }

    fn predicate(
        &mut self,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        match self.at(position)? {
            AggregateSample::Holds(held) => Ok(WindowRead::Defined(*held)),
            AggregateSample::Undefined(cause) => Ok(WindowRead::Undefined(cause.clone())),
            AggregateSample::Excluded | AggregateSample::Value(_) | AggregateSample::Counted => {
                Err(fault("aggregate-sample-kind").into())
            }
        }
    }
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

/// What one position adds to a [`Gathered`].
enum Reading {
    Nothing,
    Value(Rational),
    Holds(bool),
}

struct AggregateEvaluationCallbacks<'a> {
    retained: &'a mut dyn FnMut(usize),
    visit: &'a mut dyn FnMut() -> Result<(), LetterStop>,
    charge_meter: bool,
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

    /// What the aggregate reads at `position`, not yet folded in: the
    /// value, the predicate, or nothing for `count`.
    fn read<S: WindowSource>(
        &self,
        source: &mut S,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<Reading>, LetterStop> {
        Ok(match self {
            Self::Count => WindowRead::Defined(Reading::Nothing),
            Self::Sum(_) | Self::Min(_) | Self::Max(_) | Self::Quantile(_) => {
                match source.value(position, meter)? {
                    WindowRead::Defined(value) => WindowRead::Defined(Reading::Value(value)),
                    WindowRead::Undefined(cause) => WindowRead::Undefined(cause),
                    WindowRead::Missing => WindowRead::Missing,
                }
            }
            Self::Fraction { .. } => match source.predicate(position, meter)? {
                WindowRead::Defined(holds) => WindowRead::Defined(Reading::Holds(holds)),
                WindowRead::Undefined(cause) => WindowRead::Undefined(cause),
                WindowRead::Missing => WindowRead::Missing,
            },
        })
    }

    /// The values this state retains: a quantile keeps every gathered
    /// value, the other aggregates none.
    fn retained(&self) -> usize {
        match self {
            Self::Quantile(values) => values.len(),
            Self::Count | Self::Sum(_) | Self::Min(_) | Self::Max(_) | Self::Fraction { .. } => 0,
        }
    }

    /// Fold in a reading of the kind [`Self::read`] returned.
    fn fold(&mut self, reading: Reading) -> Result<(), InternalFault> {
        match (self, reading) {
            (Self::Count, Reading::Nothing) => {}
            (Self::Sum(total), Reading::Value(value)) => *total = total.add(&value),
            (Self::Min(least), Reading::Value(value)) => {
                if least.as_ref().is_none_or(|held| value < *held) {
                    *least = Some(value);
                }
            }
            (Self::Max(greatest), Reading::Value(value)) => {
                if greatest.as_ref().is_none_or(|held| value > *held) {
                    *greatest = Some(value);
                }
            }
            (Self::Fraction { holding }, Reading::Holds(true)) => {
                *holding = holding.saturating_add(1);
            }
            (Self::Fraction { .. }, Reading::Holds(false)) => {}
            (Self::Quantile(values), Reading::Value(value)) => values.push(value),
            (
                Self::Count
                | Self::Sum(_)
                | Self::Min(_)
                | Self::Max(_)
                | Self::Fraction { .. }
                | Self::Quantile(_),
                Reading::Nothing | Reading::Value(_) | Reading::Holds(_),
            ) => return Err(fault("reading-matches-aggregate")),
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
    /// The atom a checked clause's aggregate `checked` states, compared with
    /// `threshold`: its kind, past window, `min_count` and comparison are the
    /// checked term's (FR-337-AC-15). No production caller yet: the package
    /// route that will build it from a package entry is a follow-up.
    pub fn of_checked(
        checked: &CheckedAggregate,
        threshold: Rational,
    ) -> Result<Self, TermRefusal> {
        let kind = match &checked.kind {
            CheckedKind::Count => AggregateKind::Count,
            CheckedKind::Sum => AggregateKind::Sum,
            CheckedKind::Min => AggregateKind::Min,
            CheckedKind::Max => AggregateKind::Max,
            CheckedKind::Fraction => AggregateKind::Fraction,
            CheckedKind::Quantile(level) => AggregateKind::Quantile(level.clone()),
        };
        let window = match &checked.window {
            AggregateWindow::Positions { lower, upper } => PastWindow::Positions {
                a: *lower,
                b: *upper,
            },
            AggregateWindow::Duration(window) => PastWindow::Duration {
                a: window.lower.clone(),
                b: window.upper.clone(),
            },
        };
        let operator = match checked.comparison {
            AggregateComparison::Less => ComparisonOperator::Less,
            AggregateComparison::LessEqual => ComparisonOperator::LessOrEqual,
            AggregateComparison::Equal => ComparisonOperator::Equal,
            AggregateComparison::GreaterEqual => ComparisonOperator::GreaterOrEqual,
            AggregateComparison::Greater => ComparisonOperator::Greater,
        };
        Ok(Self::new(
            AggregateTerm::new(kind, window, checked.min_count)?,
            operator,
            threshold,
        ))
    }

    /// The term's kind, which decides the samples it takes.
    pub fn kind(&self) -> &AggregateKind {
        &self.term.kind
    }

    /// The past window whose samples this atom reads.
    pub(crate) fn window(&self) -> &PastWindow {
        &self.term.window
    }

    /// The atom `term <operator> threshold`.
    pub fn new(term: AggregateTerm, operator: ComparisonOperator, threshold: Rational) -> Self {
        Self {
            term,
            operator,
            threshold,
        }
    }

    /// The atom at `position` of `trace`: true or false; undefined with the
    /// cause of the first position, oldest first, whose filter, value or
    /// predicate is undefined; or else undefined when the window holds
    /// fewer than `min_count` values. A window that would hold more than
    /// `max_window_values` values stops the evaluation.
    pub fn evaluate<S: WindowSource>(
        &self,
        trace: &WindowTrace,
        source: &mut S,
        max_window_values: u64,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        match self.evaluate_valuation(trace, source, max_window_values, position, meter)? {
            Valuation::True => Ok(AtomReading::True),
            Valuation::False => Ok(AtomReading::False),
            Valuation::Undefined(cause) => Ok(AtomReading::Undefined(cause)),
            Valuation::Missing => Err(fault("aggregate-sample-is-present").into()),
        }
    }

    /// Evaluate an aggregate as a valuation, preserving an absent source
    /// sample as `Missing` for evidence calls (FR-516-AC-49).
    pub(crate) fn evaluate_valuation<S: WindowSource>(
        &self,
        trace: &WindowTrace,
        source: &mut S,
        max_window_values: u64,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<Valuation, LetterStop> {
        self.evaluate_observed(
            trace,
            source,
            max_window_values,
            position,
            meter,
            &mut |_| {},
        )
    }

    /// [`Self::evaluate_valuation`] with a callback charged once for every
    /// position in the window, including an absent sample.
    pub(crate) fn evaluate_valuation_with_visits<S: WindowSource>(
        &self,
        trace: &WindowTrace,
        source: &mut S,
        max_window_values: u64,
        position: TemporalPosition,
        meter: &mut Meter,
        visit: &mut dyn FnMut() -> Result<(), LetterStop>,
    ) -> Result<Valuation, LetterStop> {
        let mut retained = |_| {};
        let mut callbacks = AggregateEvaluationCallbacks {
            retained: &mut retained,
            visit,
            charge_meter: false,
        };
        self.evaluate_observed_with_visits(
            trace,
            source,
            max_window_values,
            position,
            meter,
            &mut callbacks,
        )
    }

    /// [`Self::evaluate`], telling `retained` how many values the window
    /// retains after each position.
    ///
    /// A position's value or predicate is read before the limit is checked,
    /// because an undefined one holds nothing and so is no overflow. That
    /// read is the one evaluation a window pays for the value past the
    /// limit; it is held apart and folded in only once admitted, so the
    /// window never retains more than `max_window_values` values. If that
    /// read exhausts the work budget the run stops `Incomplete` on the
    /// budget, not on the limit.
    fn evaluate_observed<S: WindowSource>(
        &self,
        trace: &WindowTrace,
        source: &mut S,
        max_window_values: u64,
        position: TemporalPosition,
        meter: &mut Meter,
        retained: &mut dyn FnMut(usize),
    ) -> Result<Valuation, LetterStop> {
        let mut visit = || Ok(());
        let mut callbacks = AggregateEvaluationCallbacks {
            retained,
            visit: &mut visit,
            charge_meter: true,
        };
        self.evaluate_observed_with_visits(
            trace,
            source,
            max_window_values,
            position,
            meter,
            &mut callbacks,
        )
    }

    fn evaluate_observed_with_visits<S: WindowSource>(
        &self,
        trace: &WindowTrace,
        source: &mut S,
        max_window_values: u64,
        position: TemporalPosition,
        meter: &mut Meter,
        callbacks: &mut AggregateEvaluationCallbacks<'_>,
    ) -> Result<Valuation, LetterStop> {
        let window = trace.window(&self.term.window, position.index())?;
        let mut gathered = Gathered::new(&self.term.kind);
        let mut count = 0_u64;
        for index in window {
            (callbacks.visit)()?;
            if callbacks.charge_meter {
                meter.charge(Charge::new(ChargePoint::TemporalVisit))?;
            }
            let at = TemporalPosition::new(index);
            match source.admits(at, meter)? {
                WindowRead::Defined(true) => {}
                WindowRead::Defined(false) => continue,
                WindowRead::Undefined(cause) => return Ok(Valuation::Undefined(cause)),
                WindowRead::Missing => return Ok(Valuation::Missing),
            }
            let reading = match gathered.read(source, at, meter)? {
                WindowRead::Defined(reading) => reading,
                WindowRead::Undefined(cause) => return Ok(Valuation::Undefined(cause)),
                WindowRead::Missing => return Ok(Valuation::Missing),
            };
            if count >= max_window_values {
                return Err(LetterStop::WindowLimit(WindowLimitReached::at(
                    max_window_values,
                    count.saturating_add(1),
                    position,
                )));
            }
            gathered.fold(reading)?;
            (callbacks.retained)(gathered.retained());
            count += 1;
        }
        if count < self.term.min_count {
            return Ok(Valuation::Undefined(UndefinedCause::InsufficientData(
                InsufficientData {
                    count,
                    min_count: self.term.min_count,
                },
            )));
        }
        let aggregate = gathered.finish(&self.term.kind, count)?;
        let ordering: Ordering = aggregate.cmp(&self.threshold);
        Ok(if self.operator.holds(ordering) {
            Valuation::True
        } else {
            Valuation::False
        })
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_semantics::check::SCALAR_LIMITS_UNLIMITED;

    use super::*;

    struct OneMissing;

    impl WindowSource for OneMissing {
        fn admits(
            &mut self,
            position: TemporalPosition,
            _meter: &mut Meter,
        ) -> Result<WindowRead<bool>, LetterStop> {
            Ok(if position.index() == 0 {
                WindowRead::Missing
            } else {
                WindowRead::Defined(true)
            })
        }

        fn value(
            &mut self,
            _position: TemporalPosition,
            _meter: &mut Meter,
        ) -> Result<WindowRead<Rational>, LetterStop> {
            Ok(WindowRead::Defined(Rational::from_integer(Integer::one())))
        }

        fn predicate(
            &mut self,
            _position: TemporalPosition,
            _meter: &mut Meter,
        ) -> Result<WindowRead<bool>, LetterStop> {
            Ok(WindowRead::Defined(true))
        }
    }

    #[test]
    fn an_unlisted_aggregate_window_position_stays_missing() {
        let term = AggregateTerm::new(AggregateKind::Sum, PastWindow::Positions { a: 0, b: 1 }, 1)
            .expect("valid sum term");
        let aggregate = AggregateAtom::new(
            term,
            ComparisonOperator::GreaterOrEqual,
            Rational::from_integer(Integer::one()),
        );
        let value = aggregate
            .evaluate_valuation(
                &WindowTrace::positions(2),
                &mut OneMissing,
                DEFAULT_MAX_WINDOW_VALUES,
                TemporalPosition::new(1),
                &mut Meter::new(SCALAR_LIMITS_UNLIMITED),
            )
            .expect("missing is a valuation, not a fault");
        assert_eq!(value, Valuation::Missing);
    }

    /// Every position holds the value 1 and every filter admits.
    struct Ones;

    impl WindowSource for Ones {
        fn admits(
            &mut self,
            _: TemporalPosition,
            _: &mut Meter,
        ) -> Result<WindowRead<bool>, LetterStop> {
            Ok(WindowRead::Defined(true))
        }

        fn value(
            &mut self,
            _: TemporalPosition,
            _: &mut Meter,
        ) -> Result<WindowRead<Rational>, LetterStop> {
            Ok(WindowRead::Defined(Rational::from_integer(Integer::one())))
        }

        fn predicate(
            &mut self,
            _: TemporalPosition,
            _: &mut Meter,
        ) -> Result<WindowRead<bool>, LetterStop> {
            Ok(WindowRead::Defined(true))
        }
    }

    /// A window never retains more than `max_window_values` values, even
    /// on the evaluation that stops at the value past the limit.
    #[trace("TC-629")]
    #[test]
    fn a_quantile_window_never_retains_more_than_the_limit() {
        let half = Rational::new(Integer::one(), Integer::from(2_i64)).expect("nonzero");
        let term = AggregateTerm::new(
            AggregateKind::Quantile(half),
            PastWindow::Positions { a: 0, b: 3 },
            1,
        )
        .expect("a well-formed term");
        let atom = AggregateAtom::new(
            term,
            ComparisonOperator::GreaterOrEqual,
            Rational::from_integer(Integer::zero()),
        );
        let mut retained = Vec::new();
        let mut meter = Meter::new(quire_exact::ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        });
        let stop = atom.evaluate_observed(
            &WindowTrace::positions(4),
            &mut Ones,
            2,
            TemporalPosition::new(3),
            &mut meter,
            &mut |held| retained.push(held),
        );
        assert_eq!(
            stop,
            Err(LetterStop::WindowLimit(WindowLimitReached::at(
                2,
                3,
                TemporalPosition::new(3),
            )))
        );
        assert_eq!(retained, vec![1, 2]);
    }
}
