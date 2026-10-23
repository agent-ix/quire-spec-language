// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-450 "Exact replay": QSpec FR-193 "Trace replay" exact checks 1 to 5
//! over a point trace through modes with polynomial flows, and the decisive
//! instant that makes a reproduced trace a `formula` or an
//! `undefined-evaluation` counterexample.

use std::collections::BTreeMap;

use qsl_semantics::check::{
    CheckedHybridClaim, CheckedHybridModel, CompareOp, HybridActivation, HybridClock,
    HybridFormula, HybridInterval, LogicOp,
};
use quire_exact::Rational;

use super::eval::{cells, eval, Constant, Env, Ev};
use super::expand::{result_kind, Exp, Expander, ResultKind};
use super::flow::Flow;
use super::interval::Tri;
use super::poly::{Poly, RatFunc};
use super::real::ExactReal;
use super::signal::{Piece, Signal, Span, Timeline, Window};
use super::trace::{before_pointer, jump_pointer, loop_pointer, state_pointer, zero};
use super::{
    ExactCheck, ExactGap, ExactReplay, ExpressionLocus, ExpressionSite, HybridCounterexample,
    HybridCounterexampleKind, HybridReplayRefusal, HybridTrace, JsonPointer, UndefinedCause,
    UndefinedInstant,
};

/// One state of a point trace: its time and point.
#[derive(Clone, Debug)]
struct Point {
    time: Rational,
    values: Vec<Rational>,
}

impl Point {
    fn polys(&self) -> Vec<Poly> {
        self.values.iter().cloned().map(Poly::constant).collect()
    }
}

/// A point trace: every state and every `before`, each a point.
struct PointTrace {
    states: Vec<(usize, Point)>,
    befores: Vec<Point>,
    transitions: Vec<usize>,
}

impl PointTrace {
    /// The trace's points, when it is a point trace (QSpec FR-193 "Trace
    /// replay": `delta` 0, every jump carrying `before`, every interval a
    /// point).
    fn of(trace: &HybridTrace) -> Option<Self> {
        if !trace.tolerance().is_zero() {
            return None;
        }
        let point = |time: &super::ExactInterval, enclosure: &[super::ExactInterval]| {
            Some(Point {
                time: time.as_point()?.clone(),
                values: enclosure
                    .iter()
                    .map(|interval| interval.as_point().cloned())
                    .collect::<Option<Vec<_>>>()?,
            })
        };
        let mut states = vec![(
            trace.start.mode.get(),
            point(&trace.start.time, &trace.start.enclosure)?,
        )];
        let mut befores = Vec::with_capacity(trace.jumps.len());
        let mut transitions = Vec::with_capacity(trace.jumps.len());
        for jump in &trace.jumps {
            let before = jump.before.as_ref()?;
            befores.push(point(&before.time, &before.enclosure)?);
            states.push((
                jump.state.mode.get(),
                point(&jump.state.time, &jump.state.enclosure)?,
            ));
            transitions.push(usize::try_from(jump.transition).ok()?);
        }
        Some(Self {
            states,
            befores,
            transitions,
        })
    }
}

/// An expression with no value, at a position of the replayed behaviour:
/// segment `segment` (the dwell from state `segment`, or the last state's
/// point), its instant `time`, `open` when it has no value just after
/// `time` only.
#[derive(Clone, Debug)]
struct Record {
    segment: usize,
    time: ExactReal,
    open: bool,
    state: usize,
    locus: ExpressionLocus,
    cause: UndefinedCause,
}

impl Record {
    fn before(&self, other: &Self) -> bool {
        (self.segment, &self.time, self.open) < (other.segment, &other.time, other.open)
    }
}

#[derive(Default)]
struct Records(Option<Record>);

impl Records {
    /// Keep `record` when it is the earliest so far.
    fn keep(&mut self, record: Record) {
        if self.0.as_ref().is_none_or(|kept| record.before(kept)) {
            self.0 = Some(record);
        }
    }

    /// Keep the undefined value `value` at a cell.
    fn cell(
        &mut self,
        value: &Ev,
        segment: usize,
        time: &ExactReal,
        open: bool,
        state: usize,
        site: ExpressionSite,
    ) {
        if let Ev::Undefined { locus, cause } = value {
            self.keep(Record {
                segment,
                time: time.clone(),
                open,
                state,
                locus: ExpressionLocus {
                    site,
                    term: locus.clone(),
                },
                cause: *cause,
            });
        }
    }
}

/// A claim formula over expanded atoms.
#[derive(Clone, Debug)]
enum Formula {
    Atom(usize),
    Not(Box<Formula>),
    Logic(LogicOp, Box<Formula>, Box<Formula>),
    /// `eventually`/`always` (`past`: `once`/`historically`).
    Unary {
        window: Window,
        past: bool,
        universal: bool,
        operand: Box<Formula>,
    },
    /// `until`/`release` (`past`: `since`/`triggered`); `dual` for
    /// `release` and `triggered`.
    Binary {
        window: Window,
        past: bool,
        dual: bool,
        left: Box<Formula>,
        right: Box<Formula>,
    },
}

/// A lasso's period in clock units: from `onset` on, a signal repeats with
/// `period`.
#[derive(Clone, Debug)]
struct Period {
    onset: Rational,
    period: Rational,
}

fn window(interval: Option<&HybridInterval>) -> Window {
    interval.map_or(
        Window {
            lower: zero(),
            lower_open: false,
            upper: None,
            upper_open: false,
        },
        |interval| Window {
            lower: interval.lower.clone(),
            lower_open: interval.lower_open,
            upper: interval.upper.clone(),
            upper_open: interval.upper_open,
        },
    )
}

struct Converter<'x, 'g> {
    expander: &'x mut Expander<'g>,
    atoms: Vec<Exp>,
    period: Option<Period>,
}

impl Converter<'_, '_> {
    /// `formula` over expanded atoms, and the clock from which its signal
    /// repeats on a lasso. On a lasso, an unbounded future window reads a
    /// bounded one that covers a whole period past its operand's onset,
    /// which reads the same positions' values.
    fn convert(
        &mut self,
        formula: &HybridFormula,
    ) -> Result<(Formula, Rational), HybridReplayRefusal> {
        let onset = || self.period.as_ref().map_or_else(zero, |p| p.onset.clone());
        Ok(match formula {
            HybridFormula::Holds(term) => {
                let exp = self.expander.term(term)?;
                self.atoms.push(exp);
                (Formula::Atom(self.atoms.len() - 1), onset())
            }
            HybridFormula::Not(f) => {
                let (f, onset) = self.convert(f)?;
                (Formula::Not(Box::new(f)), onset)
            }
            HybridFormula::Logic(op, a, b) => {
                let (a, onset_a) = self.convert(a)?;
                let (b, onset_b) = self.convert(b)?;
                (
                    Formula::Logic(*op, Box::new(a), Box::new(b)),
                    onset_a.max(onset_b),
                )
            }
            HybridFormula::Always(i, f)
            | HybridFormula::Eventually(i, f)
            | HybridFormula::Historically(i, f)
            | HybridFormula::Once(i, f) => {
                let past = matches!(
                    formula,
                    HybridFormula::Historically(..) | HybridFormula::Once(..)
                );
                let universal = matches!(
                    formula,
                    HybridFormula::Always(..) | HybridFormula::Historically(..)
                );
                let (operand, inner) = self.convert(f)?;
                let (window, onset) = self.bound(window(i.as_ref()), inner, past);
                (
                    Formula::Unary {
                        window,
                        past,
                        universal,
                        operand: Box::new(operand),
                    },
                    onset,
                )
            }
            HybridFormula::Until(i, a, b)
            | HybridFormula::Release(i, a, b)
            | HybridFormula::Since(i, a, b)
            | HybridFormula::Triggered(i, a, b) => {
                let past = matches!(
                    formula,
                    HybridFormula::Since(..) | HybridFormula::Triggered(..)
                );
                let dual = matches!(
                    formula,
                    HybridFormula::Release(..) | HybridFormula::Triggered(..)
                );
                let (left, onset_a) = self.convert(a)?;
                let (right, onset_b) = self.convert(b)?;
                let (window, onset) = self.bound(window(i.as_ref()), onset_a.max(onset_b), past);
                (
                    Formula::Binary {
                        window,
                        past,
                        dual,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    onset,
                )
            }
        })
    }

    /// The window an operator reads on a lasso, and its output's onset.
    fn bound(&self, mut window: Window, inner: Rational, past: bool) -> (Window, Rational) {
        let Some(period) = &self.period else {
            return (window, zero());
        };
        if past {
            let onset = match &window.upper {
                Some(upper) => inner.add(upper),
                None => inner.add(&window.lower).add(&period.period),
            };
            return (window, onset);
        }
        if window.upper.is_none() {
            window.upper = Some(window.lower.add(&inner).add(&period.period));
            window.upper_open = false;
        }
        (window, inner)
    }
}

/// The latest clock offset past a position whose state `formula` reads, its
/// future windows all bounded.
fn reach(formula: &Formula) -> Rational {
    let positive = |q: Rational| q.max(zero());
    match formula {
        Formula::Atom(_) => zero(),
        Formula::Not(f) => reach(f),
        Formula::Logic(_, a, b) => reach(a).max(reach(b)),
        Formula::Unary {
            window,
            past,
            operand,
            ..
        } => {
            if *past {
                positive(reach(operand).sub(&window.lower))
            } else {
                window
                    .upper
                    .clone()
                    .unwrap_or_else(zero)
                    .add(&reach(operand))
            }
        }
        Formula::Binary {
            window,
            past,
            left,
            right,
            ..
        } => {
            if *past {
                reach(left).max(positive(reach(right).sub(&window.lower)))
            } else {
                window
                    .upper
                    .clone()
                    .unwrap_or_else(zero)
                    .add(&reach(left).max(reach(right)))
            }
        }
    }
}

/// One known segment of the claim's positions.
#[derive(Clone, Debug)]
struct ClaimSegment {
    span: Span,
    values: Vec<Poly>,
    /// The dwell-timeline segment the positions are in.
    segment: usize,
    /// The trace state the positions belong to.
    state: usize,
    /// Under `model-steps`, the state's time: a position's clock counts
    /// positions there.
    time: Option<Rational>,
}

/// A cut of the replayed behaviour: the positions up to an instant of a
/// dwell-timeline segment, the instant included when `inclusive`.
#[derive(Clone, Debug)]
struct Cut {
    segment: usize,
    time: ExactReal,
    inclusive: bool,
}

struct Claim<'c> {
    claim: &'c CheckedHybridClaim,
    formula: Formula,
    atoms: Vec<Exp>,
    guard: Option<Exp>,
    captures: Vec<Exp>,
}

fn substitute(exp: &Exp, posts: &[Option<Exp>]) -> Exp {
    let map = |e: &Exp| Box::new(substitute(e, posts));
    match exp {
        Exp::Variable(i) => posts
            .get(*i)
            .cloned()
            .flatten()
            .unwrap_or_else(|| exp.clone()),
        Exp::Activation(_)
        | Exp::Capture(_)
        | Exp::Parameter(_)
        | Exp::Number(_)
        | Exp::Boolean(_)
        | Exp::Recursive(_) => exp.clone(),
        Exp::Negate(a) => Exp::Negate(map(a)),
        Exp::Not(a) => Exp::Not(map(a)),
        Exp::Arithmetic(op, a, b, locus) => Exp::Arithmetic(*op, map(a), map(b), locus.clone()),
        Exp::Compare(op, a, b) => Exp::Compare(*op, map(a), map(b)),
        Exp::Logic(op, a, b) => Exp::Logic(*op, map(a), map(b)),
        Exp::Conditional(a, b, c) => Exp::Conditional(map(a), map(b), map(c)),
        Exp::Ranged {
            value,
            range,
            role,
            locus,
        } => Exp::Ranged {
            value: map(value),
            range: range.clone(),
            role: *role,
            locus: locus.clone(),
        },
    }
}

fn rational_of(value: &RatFunc) -> Option<Rational> {
    value
        .numerator
        .as_constant()?
        .div(&value.denominator.as_constant()?)
}

/// The replay's state: the model, the trace's points, and every
/// expression it evaluates, expanded.
struct Replay<'m, 'x, 'g> {
    model: &'m CheckedHybridModel,
    expander: &'x mut Expander<'g>,
    points: PointTrace,
    flows: BTreeMap<usize, Flow>,
    invariants: BTreeMap<usize, Exp>,
    records: Records,
}

impl Replay<'_, '_, '_> {
    fn invariant(&self, mode: usize) -> &Exp {
        self.invariants
            .get(&mode)
            .unwrap_or_else(|| unreachable!("every visited mode's invariant is expanded"))
    }

    fn point_value(exp: &Exp, point: &Point) -> Result<Ev, ExactGap> {
        let variables = point.polys();
        let env = Env {
            variables: &variables,
            captures: &[],
            activation: &[],
        };
        eval(
            exp,
            &env,
            &ExactReal::Rational(point.time.clone()),
            &mut Vec::new(),
        )
    }

    /// The `n`-th state's point.
    fn state(&self, k: usize) -> &(usize, Point) {
        &self.points.states[k]
    }

    /// Check 1.
    fn initial(&mut self) -> Result<bool, HybridReplayRefusal> {
        let (mode, start) = self.state(0).clone();
        if !start.time.is_zero() {
            return Ok(false);
        }
        let invariant = Self::point_value(self.invariant(mode), &start)?;
        let mut undefined: Option<(Ev, ExpressionSite)> = None;
        let mut satisfied = false;
        for (entry, initial) in self.model.initial.iter().enumerate() {
            if initial.mode.get() != mode {
                continue;
            }
            let condition = self.expander.term(&initial.condition)?;
            let condition = Self::point_value(&condition, &start)?;
            match (&condition, &invariant) {
                (Ev::Boolean(true), Ev::Boolean(true)) => {
                    satisfied = true;
                    undefined = None;
                    break;
                }
                (Ev::Boolean(false), _) | (_, Ev::Boolean(false)) => {}
                (Ev::Undefined { .. }, _) => {
                    satisfied = true;
                    undefined.get_or_insert((condition.clone(), ExpressionSite::Initial { entry }));
                }
                (_, Ev::Undefined { .. }) => {
                    satisfied = true;
                    undefined
                        .get_or_insert((invariant.clone(), ExpressionSite::Invariant { mode }));
                }
                (Ev::Number(_), _) | (_, Ev::Number(_)) => {
                    return Err(ExactGap::IllKindedTerm.into())
                }
            }
        }
        if let Some((value, site)) = undefined {
            self.records.cell(
                &value,
                0,
                &ExactReal::Rational(start.time.clone()),
                false,
                0,
                site,
            );
        }
        Ok(satisfied)
    }

    /// The dwell from state `k`: its span and its solution in absolute
    /// time.
    fn dwell(&self, k: usize) -> (Rational, Rational, Vec<Poly>) {
        let (mode, state) = self.state(k);
        let before = &self.points.befores[k];
        let flow = self
            .flows
            .get(mode)
            .unwrap_or_else(|| unreachable!("every dwelled mode has a flow"));
        let solution = flow
            .solve(&state.values)
            .into_iter()
            .map(|poly| poly.shifted(&state.time))
            .collect();
        (state.time.clone(), before.time.clone(), solution)
    }

    /// Check 2 for the dwell from state `k`.
    fn dwell_holds(&mut self, k: usize) -> Result<bool, HybridReplayRefusal> {
        let (low, high, solution) = self.dwell(k);
        if high < low {
            return Ok(false);
        }
        let before = &self.points.befores[k];
        if solution
            .iter()
            .zip(&before.values)
            .any(|(poly, value)| poly.eval(&high) != *value)
        {
            return Ok(false);
        }
        let mode = self.state(k).0;
        let mut exps = vec![self.invariant(mode).clone()];
        exps.extend(self.flows[&mode].rates.iter().cloned());
        let env = Env {
            variables: &solution,
            captures: &[],
            activation: &[],
        };
        let cells = cells(&exps, &env, &low.clone().into(), &high.into(), true)?;
        for cell in &cells {
            for (position, value) in cell.values.iter().enumerate() {
                if position == 0 {
                    match value {
                        Ev::Boolean(false) => return Ok(false),
                        Ev::Number(_) => return Err(ExactGap::IllKindedTerm.into()),
                        Ev::Boolean(true) | Ev::Undefined { .. } => {}
                    }
                }
                let site = if position == 0 {
                    ExpressionSite::Invariant { mode }
                } else {
                    ExpressionSite::Rate {
                        mode,
                        variable: position - 1,
                    }
                };
                self.records.cell(value, k, &cell.low, !cell.point, k, site);
            }
        }
        Ok(true)
    }

    /// Check 3 for jump `k`.
    fn jump_holds(&mut self, k: usize) -> Result<bool, HybridReplayRefusal> {
        let model = self.model;
        let mode = self.state(k).0;
        let transition_index = self.points.transitions[k];
        let transition = &model.modes[mode].transitions[transition_index];
        let before = self.points.befores[k].clone();
        let (target, reached) = self.state(k + 1).clone();
        let at = ExactReal::Rational(before.time.clone());
        let guard = self.expander.term(&transition.guard)?;
        match Self::point_value(&guard, &before)? {
            Ev::Boolean(false) => return Ok(false),
            Ev::Number(_) => return Err(ExactGap::IllKindedTerm.into()),
            value => self.records.cell(
                &value,
                k,
                &at,
                false,
                k,
                ExpressionSite::Guard {
                    mode,
                    transition: transition_index,
                },
            ),
        }
        if reached.time != before.time {
            return Ok(false);
        }
        let mut constrained = vec![false; model.variables.len()];
        for (constraint, reset) in transition.resets.iter().enumerate() {
            let variable = reset.variable.get();
            constrained[variable] = true;
            let value = self.expander.term(&reset.value)?;
            match Self::point_value(&value, &before)? {
                Ev::Number(value) => {
                    let value = rational_of(&value).ok_or(ExactGap::IllKindedTerm)?;
                    let after = &reached.values[variable];
                    let holds = match reset.relation {
                        CompareOp::Eq => after == &value,
                        CompareOp::Ne => after != &value,
                        CompareOp::Lt => after < &value,
                        CompareOp::Le => after <= &value,
                        CompareOp::Ge => after >= &value,
                        CompareOp::Gt => after > &value,
                    };
                    if !holds {
                        return Ok(false);
                    }
                }
                Ev::Boolean(_) => return Err(ExactGap::IllKindedTerm.into()),
                undefined @ Ev::Undefined { .. } => self.records.cell(
                    &undefined,
                    k,
                    &at,
                    false,
                    k,
                    ExpressionSite::Reset {
                        mode,
                        transition: transition_index,
                        constraint,
                    },
                ),
            }
        }
        if constrained
            .iter()
            .zip(before.values.iter().zip(&reached.values))
            .any(|(constrained, (before, after))| !constrained && before != after)
        {
            return Ok(false);
        }
        match Self::point_value(&self.invariant(target).clone(), &reached)? {
            Ev::Boolean(false) => Ok(false),
            Ev::Number(_) => Err(ExactGap::IllKindedTerm.into()),
            value => {
                self.records.cell(
                    &value,
                    k + 1,
                    &at,
                    false,
                    k + 1,
                    ExpressionSite::Invariant { mode: target },
                );
                Ok(true)
            }
        }
    }

    /// Under `model-steps`, state `k`'s position at clock `clock`.
    fn step_segment(&self, k: usize, clock: Rational) -> ClaimSegment {
        let (_, point) = self.state(k);
        let at: ExactReal = clock.into();
        ClaimSegment {
            span: Span {
                low: at.clone(),
                high: at,
                high_closed: true,
            },
            values: point.polys(),
            segment: k,
            state: k,
            time: Some(point.time.clone()),
        }
    }

    /// Under `model-time`, the dwell from state `k`, `shift` seconds later.
    fn dwell_segment(&self, k: usize, shift: &Rational) -> ClaimSegment {
        let (low, high, solution) = self.dwell(k);
        ClaimSegment {
            span: Span {
                low: low.add(shift).into(),
                high: high.add(shift).into(),
                high_closed: true,
            },
            values: solution.iter().map(|p| p.shifted(shift)).collect(),
            segment: k,
            state: k,
            time: None,
        }
    }

    /// The claim's known positions, and the clock value the unknown tail
    /// starts at: the whole finite trace, or, on a lasso, its periodic
    /// behaviour unrolled past `horizon` clock units.
    fn claim_segments(
        &self,
        clock: HybridClock,
        lasso: Option<usize>,
        horizon: &Rational,
    ) -> (Vec<ClaimSegment>, ExactReal) {
        let n = self.points.befores.len();
        let steps = clock == HybridClock::ModelSteps;
        let count = |k: usize| Rational::from_integer(k.into());
        let mut segments: Vec<ClaimSegment> = (0..n)
            .map(|k| {
                if steps {
                    self.step_segment(k, count(k))
                } else {
                    self.dwell_segment(k, &zero())
                }
            })
            .collect();
        let Some(loop_to) = lasso else {
            if steps {
                segments.push(self.step_segment(n, count(n)));
                return (segments, count(n + 1).into());
            }
            let (_, last) = self.state(n);
            let at: ExactReal = last.time.clone().into();
            segments.push(ClaimSegment {
                span: Span {
                    low: at.clone(),
                    high: at.clone(),
                    high_closed: true,
                },
                values: last.polys(),
                segment: n,
                state: n,
                time: None,
            });
            return (segments, at);
        };
        let period = if steps {
            count(n - loop_to)
        } else {
            self.state(n).1.time.sub(&self.state(loop_to).1.time)
        };
        let enough: ExactReal = horizon.add(&period).into();
        let mut copy = 1_usize;
        loop {
            let shift = period.mul(&count(copy));
            for k in loop_to..n {
                segments.push(if steps {
                    self.step_segment(k, count(k).add(&shift))
                } else {
                    self.dwell_segment(k, &shift)
                });
            }
            let end = segments
                .last()
                .map_or_else(|| zero().into(), |segment| segment.span.high.clone());
            if end > enough {
                return (segments, end.shifted(&Rational::from_integer(1_i64.into())));
            }
            copy += 1;
        }
    }
}

/// `segments` cut at `cut`, and the clock value the unknown tail starts at.
fn cut_segments(
    segments: &[ClaimSegment],
    cut: &Cut,
    steps: bool,
) -> (Vec<ClaimSegment>, ExactReal) {
    let mut kept = Vec::new();
    for segment in segments {
        if segment.segment > cut.segment {
            break;
        }
        if segment.segment < cut.segment {
            kept.push(segment.clone());
            continue;
        }
        if steps {
            let time: ExactReal = segment.time.clone().unwrap_or_else(zero).into();
            if time < cut.time || (time == cut.time && cut.inclusive) {
                kept.push(segment.clone());
            }
            break;
        }
        let mut cut_segment = segment.clone();
        if cut.time < segment.span.high || (cut.time == segment.span.high && !cut.inclusive) {
            if cut.time == segment.span.low && !cut.inclusive {
                break;
            }
            cut_segment.span.high = cut.time.clone();
            cut_segment.span.high_closed = cut.inclusive;
        }
        kept.push(cut_segment);
        break;
    }
    let tail = if steps {
        Rational::from_integer(kept.len().into()).into()
    } else {
        cut.time.clone()
    };
    (kept, tail)
}

impl Claim<'_> {
    /// The claim's value over `segments`, keeping each undefined atom.
    fn value(
        &self,
        segments: &[ClaimSegment],
        tail: ExactReal,
        records: &mut Records,
    ) -> Result<Tri, HybridReplayRefusal> {
        let timeline = Timeline {
            spans: segments.iter().map(|s| s.span.clone()).collect(),
            tail: Some(tail),
        };
        match &self.claim.activation {
            HybridActivation::Each { .. } if self.claim.clock == HybridClock::ModelSteps => {
                let mut claim = Tri::True;
                for (j, segment) in segments.iter().enumerate() {
                    let activation: Vec<Rational> = segment
                        .values
                        .iter()
                        .map(|p| p.as_constant().unwrap_or_else(zero))
                        .collect();
                    let Some(captures) = self.captures(segment, &activation, records)? else {
                        continue;
                    };
                    let guard = match &self.guard {
                        None => Tri::True,
                        Some(guard) => self.point_truth(
                            guard,
                            segment,
                            &captures,
                            &activation,
                            records,
                            ExpressionSite::Activation,
                        )?,
                    };
                    if guard == Tri::False {
                        continue;
                    }
                    let signal =
                        self.formula_signal(&timeline, segments, &captures, &activation, records)?;
                    let body = Timeline::value(&signal, j, &segment.span.low);
                    claim = claim.and(guard.not().or(body));
                }
                Ok(claim)
            }
            HybridActivation::Each { .. } | HybridActivation::Origin => {
                let Some(first) = segments.first() else {
                    return Ok(Tri::Unknown);
                };
                let Some(captures) = self.captures(first, &[], records)? else {
                    return Ok(Tri::Unknown);
                };
                let signal = self.formula_signal(&timeline, segments, &captures, &[], records)?;
                Ok(timeline.at_origin(&signal))
            }
        }
    }

    /// The obligation's captures at `segment`'s first position, in order;
    /// `None` when one has no value.
    fn captures(
        &self,
        segment: &ClaimSegment,
        activation: &[Rational],
        records: &mut Records,
    ) -> Result<Option<Vec<Constant>>, HybridReplayRefusal> {
        let mut values = Vec::with_capacity(self.captures.len());
        for (capture, exp) in self.captures.iter().enumerate() {
            let env = Env {
                variables: &segment.values,
                captures: &values,
                activation,
            };
            let at = &segment.span.low;
            let site = ExpressionSite::Capture { capture };
            let time = segment
                .time
                .clone()
                .map_or_else(|| at.clone(), ExactReal::from);
            match eval(exp, &env, at, &mut Vec::new())? {
                Ev::Boolean(value) => values.push(Constant::Boolean(value)),
                Ev::Number(value) => {
                    let value = rational_of(&value).ok_or(ExactGap::ActivationState)?;
                    let inside = match self
                        .claim
                        .captures
                        .get(capture)
                        .and_then(|c| result_kind(&c.ty))
                    {
                        Some(ResultKind::Number(range)) => {
                            range.lower.as_ref().is_none_or(|lower| &value >= lower)
                                && range.upper.as_ref().is_none_or(|upper| &value <= upper)
                        }
                        Some(ResultKind::Boolean) | None => true,
                    };
                    if !inside {
                        records.keep(Record {
                            segment: segment.segment,
                            time,
                            open: false,
                            state: segment.state,
                            locus: ExpressionLocus {
                                site,
                                term: super::TermLocus::default(),
                            },
                            cause: UndefinedCause::OutsideCaptureRange { capture },
                        });
                        return Ok(None);
                    }
                    values.push(Constant::Number(value));
                }
                undefined @ Ev::Undefined { .. } => {
                    records.cell(
                        &undefined,
                        segment.segment,
                        &time,
                        false,
                        segment.state,
                        site,
                    );
                    return Ok(None);
                }
            }
        }
        Ok(Some(values))
    }

    fn point_truth(
        &self,
        exp: &Exp,
        segment: &ClaimSegment,
        captures: &[Constant],
        activation: &[Rational],
        records: &mut Records,
        site: ExpressionSite,
    ) -> Result<Tri, HybridReplayRefusal> {
        let env = Env {
            variables: &segment.values,
            captures,
            activation,
        };
        let time = segment
            .time
            .clone()
            .map_or_else(|| segment.span.low.clone(), ExactReal::from);
        Ok(match eval(exp, &env, &segment.span.low, &mut Vec::new())? {
            Ev::Boolean(value) => Tri::from_bool(value),
            Ev::Number(_) => return Err(ExactGap::IllKindedTerm.into()),
            undefined @ Ev::Undefined { .. } => {
                records.cell(
                    &undefined,
                    segment.segment,
                    &time,
                    false,
                    segment.state,
                    site,
                );
                Tri::Unknown
            }
        })
    }

    /// The formula's signal over the claim's positions.
    fn formula_signal(
        &self,
        timeline: &Timeline,
        segments: &[ClaimSegment],
        captures: &[Constant],
        activation: &[Rational],
        records: &mut Records,
    ) -> Result<Signal, HybridReplayRefusal> {
        let mut atoms: Vec<Signal> = vec![Vec::with_capacity(segments.len()); self.atoms.len()];
        for segment in segments {
            let env = Env {
                variables: &segment.values,
                captures,
                activation,
            };
            let cells = cells(
                &self.atoms,
                &env,
                &segment.span.low,
                &segment.span.high,
                segment.span.high_closed,
            )?;
            for signal in &mut atoms {
                signal.push(Vec::with_capacity(cells.len()));
            }
            for cell in &cells {
                for (atom, value) in cell.values.iter().enumerate() {
                    let truth = match value {
                        Ev::Boolean(value) => Tri::from_bool(*value),
                        Ev::Number(_) => return Err(ExactGap::IllKindedTerm.into()),
                        Ev::Undefined { .. } => {
                            let time = segment
                                .time
                                .clone()
                                .map_or_else(|| cell.low.clone(), ExactReal::from);
                            let site = if Some(atom) == self.guard_atom() {
                                ExpressionSite::Activation
                            } else {
                                ExpressionSite::ClaimAtom { atom }
                            };
                            records.cell(
                                value,
                                segment.segment,
                                &time,
                                !cell.point,
                                segment.state,
                                site,
                            );
                            Tri::Unknown
                        }
                    };
                    if let Some(pieces) = atoms[atom].last_mut() {
                        pieces.push(Piece {
                            low: cell.low.clone(),
                            high: cell.high.clone(),
                            point: cell.point,
                            value: truth,
                        });
                    }
                }
            }
        }
        Ok(signal_of(&self.formula, &atoms, timeline))
    }

    /// Under `model-time`, the `on each` guard is the formula's last atom.
    fn guard_atom(&self) -> Option<usize> {
        (self.claim.clock == HybridClock::ModelTime && self.guard.is_some())
            .then(|| self.atoms.len() - 1)
    }
}

fn signal_of(formula: &Formula, atoms: &[Signal], timeline: &Timeline) -> Signal {
    match formula {
        Formula::Atom(i) => atoms[*i].clone(),
        Formula::Not(f) => Timeline::map(&signal_of(f, atoms, timeline), Tri::not),
        Formula::Logic(op, a, b) => {
            let (a, b) = (signal_of(a, atoms, timeline), signal_of(b, atoms, timeline));
            timeline.combine(&a, &b, |a, b| match op {
                LogicOp::And => a.and(b),
                LogicOp::Or => a.or(b),
                LogicOp::Implies => a.not().or(b),
            })
        }
        Formula::Unary {
            window,
            past,
            universal,
            operand,
        } => timeline.unary(
            &signal_of(operand, atoms, timeline),
            window,
            *past,
            *universal,
        ),
        Formula::Binary {
            window,
            past,
            dual,
            left,
            right,
        } => {
            let (left, right) = (
                signal_of(left, atoms, timeline),
                signal_of(right, atoms, timeline),
            );
            if *dual {
                let (left, right) = (
                    Timeline::map(&left, Tri::not),
                    Timeline::map(&right, Tri::not),
                );
                Timeline::map(&timeline.binary(&left, &right, window, *past), Tri::not)
            } else {
                timeline.binary(&left, &right, window, *past)
            }
        }
    }
}

/// Whether a lasso's periodic behaviour satisfies the claim's fairness set
/// (QSpec FR-193 "Fairness").
fn fair(
    replay: &mut Replay<'_, '_, '_>,
    claim: &CheckedHybridClaim,
    loop_to: usize,
) -> Result<bool, HybridReplayRefusal> {
    let n = replay.points.befores.len();
    let model = replay.model;
    let taken: Vec<(usize, usize)> = (loop_to..n)
        .map(|j| (replay.state(j).0, replay.points.transitions[j]))
        .collect();
    for constraint in &claim.fairness {
        let groups: Vec<Vec<(usize, usize)>> = match constraint.granularity {
            quire_semantic_value::fairness::FairnessGranularity::Whole => vec![constraint
                .transitions
                .iter()
                .map(|(m, t)| (m.get(), t.get()))
                .collect()],
            quire_semantic_value::fairness::FairnessGranularity::Each => constraint
                .transitions
                .iter()
                .map(|(m, t)| vec![(m.get(), t.get())])
                .collect(),
        };
        let strong = matches!(
            constraint.kind,
            quire_semantic_value::fairness::FairnessKind::Strong
        );
        for group in groups {
            if group.iter().any(|named| taken.contains(named)) {
                continue;
            }
            let mut enabled_somewhere = false;
            let mut disabled_somewhere = false;
            for k in loop_to..n {
                let mode = replay.state(k).0;
                let mut exps = Vec::new();
                for &(source, t) in group.iter().filter(|(source, _)| *source == mode) {
                    let transition = &model.modes[source].transitions[t];
                    let mut posts: Vec<Option<Exp>> = vec![None; model.variables.len()];
                    for reset in &transition.resets {
                        if reset.relation != CompareOp::Eq {
                            return Err(ExactGap::NondeterministicReset {
                                mode: source,
                                transition: t,
                            }
                            .into());
                        }
                        posts[reset.variable.get()] = Some(replay.expander.term(&reset.value)?);
                    }
                    let guard = replay.expander.term(&transition.guard)?;
                    let target = replay
                        .expander
                        .term(&model.modes[transition.target.get()].invariant)?;
                    exps.push(Exp::Logic(
                        LogicOp::And,
                        Box::new(guard),
                        Box::new(substitute(&target, &posts)),
                    ));
                }
                if exps.is_empty() {
                    disabled_somewhere = true;
                    continue;
                }
                let (low, high, solution) = replay.dwell(k);
                let env = Env {
                    variables: &solution,
                    captures: &[],
                    activation: &[],
                };
                for cell in cells(&exps, &env, &low.into(), &high.into(), true)? {
                    let any = cell.values.iter().any(|v| matches!(v, Ev::Boolean(true)));
                    enabled_somewhere |= any;
                    disabled_somewhere |= !any;
                }
            }
            let violated = if strong {
                enabled_somewhere
            } else {
                !disabled_somewhere
            };
            if violated {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// FR-450 "Exact replay" of an admitted trace.
pub(super) fn replay(
    model: &CheckedHybridModel,
    claim: &CheckedHybridClaim,
    expander: &mut Expander<'_>,
    trace: &HybridTrace,
) -> Result<ExactReplay, HybridReplayRefusal> {
    let Some(points) = PointTrace::of(trace) else {
        return Ok(ExactReplay::NotExact);
    };
    let n = points.befores.len();
    let lasso = trace.loop_to.and_then(|l| usize::try_from(l).ok());
    // Every mode the trace dwells in has a polynomial flow.
    let mut flows = BTreeMap::new();
    for k in 0..n {
        let mode = points.states[k].0;
        if flows.contains_key(&mode) {
            continue;
        }
        let rates = model.modes[mode]
            .rates
            .iter()
            .map(|rate| expander.term(rate))
            .collect::<Result<Vec<_>, _>>()?;
        let Some(flow) = Flow::of(rates) else {
            return Ok(ExactReplay::NotExact);
        };
        flows.insert(mode, flow);
    }
    // No expression the replay evaluates calls a recursive function.
    let mut evaluated: Vec<Exp> = Vec::new();
    let mut invariants = BTreeMap::new();
    for (mode, _) in &points.states {
        if !invariants.contains_key(mode) {
            invariants.insert(*mode, expander.term(&model.modes[*mode].invariant)?);
        }
    }
    evaluated.extend(invariants.values().cloned());
    for initial in model
        .initial
        .iter()
        .filter(|e| e.mode.get() == points.states[0].0)
    {
        evaluated.push(expander.term(&initial.condition)?);
    }
    for k in 0..n {
        let transition = &model.modes[points.states[k].0].transitions[points.transitions[k]];
        evaluated.push(expander.term(&transition.guard)?);
        for reset in &transition.resets {
            evaluated.push(expander.term(&reset.value)?);
        }
    }
    let captures = claim
        .captures
        .iter()
        .map(|capture| expander.term(&capture.value))
        .collect::<Result<Vec<_>, _>>()?;
    let guard = match &claim.activation {
        HybridActivation::Each {
            guard: Some(guard), ..
        } => Some(expander.term(guard)?),
        HybridActivation::Each { guard: None, .. } | HybridActivation::Origin => None,
    };
    let period = match lasso {
        None => None,
        Some(loop_to) if claim.clock == HybridClock::ModelSteps => Some(Period {
            onset: Rational::from_integer(loop_to.into()),
            period: Rational::from_integer((n - loop_to).into()),
        }),
        Some(loop_to) => Some(Period {
            onset: points.states[loop_to].1.time.clone(),
            period: points.states[n].1.time.sub(&points.states[loop_to].1.time),
        }),
    };
    let mut converter = Converter {
        expander,
        atoms: Vec::new(),
        period: period.clone(),
    };
    let (mut formula, _) = converter.convert(&claim.formula)?;
    let mut atoms = converter.atoms;
    let time_each = matches!(claim.activation, HybridActivation::Each { .. })
        && claim.clock == HybridClock::ModelTime;
    if time_each {
        // Under `model-time`, one obligation at every instant: the claim is
        // `always (g implies body)`, read when no obligation reads its own
        // activation state.
        if !captures.is_empty() || atoms.iter().any(Exp::reads_activation) {
            return Err(ExactGap::ActivationState.into());
        }
        let body = match &guard {
            Some(guard) => {
                atoms.push(guard.activation_as_current());
                Formula::Logic(
                    LogicOp::Implies,
                    Box::new(Formula::Atom(atoms.len() - 1)),
                    Box::new(formula),
                )
            }
            None => formula,
        };
        let mut always = window(None);
        if let Some(period) = &period {
            always.upper = Some(period.onset.add(&period.period));
        }
        formula = Formula::Unary {
            window: always,
            past: false,
            universal: true,
            operand: Box::new(body),
        };
    }
    evaluated.extend(captures.iter().cloned());
    evaluated.extend(guard.iter().cloned());
    evaluated.extend(atoms.iter().cloned());
    if evaluated.iter().any(Exp::calls_recursive) {
        return Ok(ExactReplay::NotExact);
    }

    let mut replay = Replay {
        model,
        expander,
        points,
        flows,
        invariants,
        records: Records::default(),
    };
    let not_reproduced =
        |at: JsonPointer, check: ExactCheck| Ok(ExactReplay::NotReproduced { at, check });
    // Checks 1 to 3, in trace order.
    if !replay.initial()? {
        return not_reproduced(state_pointer(0), ExactCheck::Initial);
    }
    for k in 0..n {
        if !replay.dwell_holds(k)? {
            return not_reproduced(before_pointer(k), ExactCheck::Dwell);
        }
        if !replay.jump_holds(k)? {
            return not_reproduced(jump_pointer(k), ExactCheck::Jump);
        }
    }
    // Check 4.
    if let Some(loop_to) = lasso {
        let (last_mode, last) = replay.state(n).clone();
        let (mode, target) = replay.state(loop_to).clone();
        if last_mode != mode || last.values != target.values || last.time <= target.time {
            return not_reproduced(loop_pointer(), ExactCheck::Loop);
        }
    }
    // Check 5 and the decisive instant.
    let horizon_clock = match (&claim.activation, claim.clock) {
        (HybridActivation::Each { .. }, HybridClock::ModelSteps) => {
            Rational::from_integer(n.into())
        }
        _ => zero(),
    };
    let horizon = horizon_clock.add(&reach(&formula));
    let (segments, tail) = replay.claim_segments(claim.clock, lasso, &horizon);
    let claim_reading = Claim {
        claim,
        formula,
        atoms,
        guard,
        captures,
    };
    let mut records = std::mem::take(&mut replay.records);
    let value = claim_reading.value(&segments, tail, &mut records)?;
    let reproduced = |kind| {
        Ok(ExactReplay::Reproduced(Box::new(HybridCounterexample {
            kind,
            trace: trace.clone(),
        })))
    };
    let undefined = |record: Record| HybridCounterexampleKind::UndefinedEvaluation {
        at: Box::new(UndefinedInstant {
            state: record.state,
            time: record.time,
            locus: record.locus,
        }),
        cause: record.cause,
    };
    if let Some(loop_to) = lasso {
        if let Some(record) = records.0 {
            return reproduced(undefined(record));
        }
        if value == Tri::False && fair(&mut replay, claim, loop_to)? {
            return reproduced(HybridCounterexampleKind::Formula);
        }
        return not_reproduced(JsonPointer::root(), ExactCheck::Claim);
    }
    match records.0 {
        Some(record) => {
            let cut = Cut {
                segment: record.segment,
                time: record.time.clone(),
                inclusive: record.open,
            };
            let (kept, tail) =
                cut_segments(&segments, &cut, claim.clock == HybridClock::ModelSteps);
            let before = claim_reading.value(&kept, tail, &mut Records::default())?;
            if before == Tri::False {
                reproduced(HybridCounterexampleKind::Formula)
            } else {
                reproduced(undefined(record))
            }
        }
        None if value == Tri::False => reproduced(HybridCounterexampleKind::Formula),
        None => not_reproduced(JsonPointer::root(), ExactCheck::Claim),
    }
}

#[cfg(test)]
pub(super) mod probe {
    //! Crate-internal views the tests read: a dwell's solution, and the
    //! claim's value over a prefix cut at an instant.

    use super::*;

    /// The solution of `mode`'s flow from `start`, in local time, or `None`
    /// when the flow is not polynomial.
    pub(in crate::hybrid_replay) fn solution(
        model: &CheckedHybridModel,
        expander: &mut Expander<'_>,
        mode: usize,
        start: &[Rational],
    ) -> Option<Vec<Poly>> {
        let rates = model.modes[mode]
            .rates
            .iter()
            .map(|rate| expander.term(rate))
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        Flow::of(rates).map(|flow| flow.solve(start))
    }
}
