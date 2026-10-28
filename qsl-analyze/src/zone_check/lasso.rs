// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-241 for a lasso's loop: its delays, solved exactly over the rationals
//! (ADR-026 EZ-8, CT-4).
//!
//! A loop of `m` positions is read as repeated forever. A clock the loop
//! resets takes its value from the loop's own delays: at any point it reads
//! the value of its last reset, cyclically before the point, plus the
//! delays since. A clock the loop never resets is, at the loop's entry,
//! above every constant of the model (FR-236 "Timed lasso"), so a lower
//! bound on it holds and an upper bound fails. Every guard, time
//! invariant and symbolic piece of the loop is then a linear constraint on
//! the prefix sums `P_k = d_1 + ... + d_k` of its delays, with `P_m` the
//! loop's total delay `D`, which must be positive.
//!
//! The constraints are solved by Fourier-Motzkin elimination with strict
//! and non-strict inequalities, exact over the rationals, and the delays
//! are then fixed in order by FR-241's rule: the least admissible delay
//! when the delay set has a least element, otherwise the midpoint of a
//! bounded set, otherwise its lower end plus one unit. The solution is a
//! function of the loop.

use std::collections::BTreeSet;

use num_bigint::BigInt;
use qsl_eval::simulation::model::PlaceConstraint;
use qsl_semantics::check::ClockOp;
use quire_exact::{Integer, Rational};

use super::concretise::NotConcretised;
use super::{Bound, Dbm};

/// One position of a loop, with the members its delay and step obey.
#[derive(Clone, Debug)]
pub(super) struct LoopStage {
    /// Whether time may elapse before the position.
    pub elapses: bool,
    /// The time invariants of the state the delay is spent in.
    pub invariants: Vec<PlaceConstraint>,
    /// The convex guard part the position's step takes; empty for a
    /// stutter.
    pub part: Vec<PlaceConstraint>,
    /// The step's resets, place and value, in order.
    pub resets: Vec<(usize, Rational)>,
    /// The step's simultaneous clock copies, destination and source place.
    pub carries: Vec<(usize, usize)>,
    /// The time invariants after the position.
    pub target: Vec<PlaceConstraint>,
    /// A claim automaton's guard on its own clocks at the position, read
    /// before its resets (FR-242); its resets are among `resets`, and no
    /// step resets its clocks, so the guard reads their values before the
    /// position.
    pub position: Vec<PlaceConstraint>,
    /// The zone over clock places, in units of `1 / scale`, the valuation
    /// before the step, after the delay, lies in.
    pub before: Option<Dbm>,
    /// The zone over clock places, in units of `1 / scale`, the valuation
    /// after the position lies in.
    pub piece: Option<Dbm>,
}

/// A linear form `constant + Σ coefficient_k · P_k`.
#[derive(Clone, Debug, PartialEq)]
struct Form {
    constant: Rational,
    coefficients: Vec<Rational>,
}

/// `form < 0` when `strict`, otherwise `form <= 0`.
#[derive(Clone, Debug)]
struct Inequality {
    form: Form,
    strict: bool,
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

fn one() -> Rational {
    Rational::from_integer(Integer::one())
}

impl Form {
    fn constant(m: usize, value: Rational) -> Self {
        Self {
            constant: value,
            coefficients: vec![zero(); m],
        }
    }

    /// `P_k`, or 0 for `k = None` (the empty prefix).
    fn prefix(m: usize, k: Option<usize>) -> Self {
        let mut form = Self::constant(m, zero());
        if let Some(k) = k {
            form.coefficients[k] = one();
        }
        form
    }

    fn plus(&self, other: &Self) -> Self {
        Self {
            constant: self.constant.add(&other.constant),
            coefficients: self
                .coefficients
                .iter()
                .zip(&other.coefficients)
                .map(|(a, b)| a.add(b))
                .collect(),
        }
    }

    fn minus(&self, other: &Self) -> Self {
        self.plus(&other.times(&one().neg()))
    }

    fn times(&self, factor: &Rational) -> Self {
        Self {
            constant: self.constant.mul(factor),
            coefficients: self.coefficients.iter().map(|c| c.mul(factor)).collect(),
        }
    }

    fn shifted(&self, value: &Rational) -> Self {
        Self {
            constant: self.constant.add(value),
            coefficients: self.coefficients.clone(),
        }
    }
}

/// A clock's value at a point of the loop: a form, or above every
/// constant for a clock the loop never resets.
enum Value {
    Form(Form),
    Above,
}

/// The loop's constraints, built position by position.
struct System<'s> {
    stages: &'s [LoopStage],
    scale: &'s BigInt,
    inequalities: Vec<Inequality>,
    infeasible: bool,
}

impl System<'_> {
    fn m(&self) -> usize {
        self.stages.len()
    }

    /// The time over `distance` positions ending at `end`, where the span
    /// may cross the loop more than once because a carried clock can follow
    /// another clock's earlier value.
    fn delays_back(&self, distance: usize, end: usize) -> Form {
        let m = self.m();
        let laps = distance / m;
        let remainder = distance % m;
        let mut span = Form::constant(m, zero());
        if laps > 0 {
            let whole = Form::prefix(m, Some(m - 1));
            let factor =
                Rational::from_integer(Integer::from(u64::try_from(laps).unwrap_or(u64::MAX)));
            span = span.plus(&whole.times(&factor));
        }
        for offset in 0..remainder {
            let at = (end + m - offset) % m;
            let current = Form::prefix(m, Some(at));
            let previous = at
                .checked_sub(1)
                .map(|index| Form::prefix(m, Some(index)))
                .unwrap_or_else(|| Form::constant(m, zero()));
            span = span.plus(&current.minus(&previous));
        }
        span
    }

    /// Clock `place`'s value just before position `k`'s step, after its
    /// delay.
    fn before(&self, place: usize, k: usize) -> Value {
        let m = self.m();
        let mut clock = place;
        let mut at = k;
        let mut distance = 0;
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert((at, clock)) {
                return Value::Above;
            }
            let j = (at + m - 1) % m;
            distance += 1;
            if let Some((_, value)) = self.stages[j]
                .resets
                .iter()
                .rev()
                .find(|(reset, _)| *reset == clock)
            {
                return Value::Form(self.delays_back(distance, k).shifted(value));
            }
            if let Some((_, source)) = self.stages[j]
                .carries
                .iter()
                .rev()
                .find(|(target, _)| *target == clock)
            {
                clock = *source;
            }
            at = j;
        }
    }

    /// Clock `place`'s value just after position `k`'s step.
    fn after(&self, place: usize, k: usize) -> Value {
        match self.stages[k]
            .resets
            .iter()
            .rev()
            .find(|(reset, _)| *reset == place)
        {
            Some((_, value)) => Value::Form(Form::constant(self.m(), value.clone())),
            None => match self.stages[k]
                .carries
                .iter()
                .rev()
                .find(|(target, _)| *target == place)
            {
                Some((_, source)) => self.before(*source, k),
                None => self.before(place, k),
            },
        }
    }

    /// Clock `place`'s value at the start of position `k`'s delay.
    fn start(&self, place: usize, k: usize) -> Value {
        self.after(place, (k + self.m() - 1) % self.m())
    }

    fn push(&mut self, form: Form, strict: bool) {
        self.inequalities.push(Inequality { form, strict });
    }

    /// `value op bound`.
    fn compare(&mut self, value: Value, op: ClockOp, bound: &Rational) {
        let form = match value {
            Value::Form(form) => form,
            Value::Above => {
                if matches!(op, ClockOp::Less | ClockOp::LessOrEqual | ClockOp::Equal) {
                    self.infeasible = true;
                }
                return;
            }
        };
        let below = form.shifted(&bound.neg());
        match op {
            ClockOp::Less => self.push(below, true),
            ClockOp::LessOrEqual => self.push(below, false),
            ClockOp::Equal => {
                self.push(below.clone(), false);
                self.push(below.times(&one().neg()), false);
            }
            ClockOp::GreaterOrEqual => self.push(below.times(&one().neg()), false),
            ClockOp::Greater => self.push(below.times(&one().neg()), true),
        }
    }

    fn constraints(
        &mut self,
        constraints: &[PlaceConstraint],
        value: impl Fn(&Self, usize) -> Value,
    ) {
        for constraint in constraints {
            let read = value(self, constraint.place());
            self.compare(read, constraint.op(), constraint.bound());
        }
    }

    /// The valuation `value` reads lies in `piece`.
    fn piece(
        &mut self,
        piece: &Dbm,
        clock_value: impl Fn(&Self, usize) -> Value,
    ) -> Result<(), NotConcretised> {
        let scale = Rational::from_integer(Integer::from_big(self.scale.clone()));
        for i in 0..piece.dim() {
            for j in (0..piece.dim()).filter(|&j| j != i) {
                let Bound::Finite { value, strict } = piece.bound(i, j)? else {
                    continue;
                };
                let bound = Rational::from_integer(Integer::from_big(value.clone()))
                    .div(&scale)
                    .ok_or(NotConcretised::Infeasible)?;
                let read = |clock: usize| {
                    if clock == 0 {
                        Value::Form(Form::constant(self.m(), zero()))
                    } else {
                        clock_value(self, clock - 1)
                    }
                };
                match (read(i), read(j)) {
                    (Value::Form(left), Value::Form(right)) => {
                        self.push(left.minus(&right).shifted(&bound.neg()), *strict);
                    }
                    // A clock the loop never resets is above every constant:
                    // a bound from above on it fails, one from below holds.
                    // Two such clocks keep the difference the stem gives
                    // them, which the whole lasso's concretisation checks.
                    (Value::Above, Value::Form(_)) => self.infeasible = true,
                    (Value::Form(_) | Value::Above, Value::Above) => {}
                }
            }
        }
        Ok(())
    }
}

/// FR-241: the delays of the loop `stages`, repeated forever, in time
/// units; zone bounds in pieces are in units of `1 / scale`. The loop's
/// total delay must be positive under a dense source (`dense`); under a
/// tick source no time elapses between positions and its ticks advance
/// time. The elimination keeps at most `cap` constraints at once, and
/// past it returns [`NotConcretised::Exhausted`].
pub(super) fn solve_loop(
    stages: &[LoopStage],
    scale: &BigInt,
    dense: bool,
    cap: u64,
) -> Result<Vec<Rational>, NotConcretised> {
    let m = stages.len();
    if m == 0 {
        return Err(NotConcretised::Infeasible);
    }
    let mut system = System {
        stages,
        scale,
        inequalities: Vec::new(),
        infeasible: false,
    };
    for (k, stage) in stages.iter().enumerate() {
        let delay = Form::prefix(m, Some(k)).minus(&Form::prefix(m, k.checked_sub(1)));
        // `d_k >= 0`, and `= 0` where time cannot pass.
        system.push(delay.times(&one().neg()), false);
        if !stage.elapses {
            system.push(delay.clone(), false);
        }
        system.constraints(&stage.invariants, |system, place| system.start(place, k));
        system.constraints(&stage.invariants, |system, place| system.before(place, k));
        system.constraints(&stage.part, |system, place| system.before(place, k));
        system.constraints(&stage.target, |system, place| system.after(place, k));
        system.constraints(&stage.position, |system, place| system.before(place, k));
        if let Some(before) = &stage.before {
            system.piece(before, |system, place| system.before(place, k))?;
        }
        if let Some(piece) = &stage.piece {
            system.piece(piece, |system, place| system.after(place, k))?;
        }
    }
    if dense {
        // `D > 0`.
        system.push(Form::prefix(m, Some(m - 1)).times(&one().neg()), true);
    }
    if system.infeasible {
        return Err(NotConcretised::Infeasible);
    }
    let prefixes = solve(system.inequalities, m, cap)?;
    let mut delays = Vec::with_capacity(m);
    let mut previous = zero();
    for prefix in prefixes {
        delays.push(prefix.sub(&previous));
        previous = prefix;
    }
    Ok(delays)
}

/// Solve `inequalities` over `m` variables: eliminate the last variable
/// first, then fix the first variable first, each by FR-241's rule.
fn solve(
    inequalities: Vec<Inequality>,
    m: usize,
    cap: u64,
) -> Result<Vec<Rational>, NotConcretised> {
    // `levels[k]` holds the constraints over the variables `0..=k`.
    let mut levels: Vec<Vec<Inequality>> = vec![Vec::new(); m];
    let mut current = inequalities;
    for k in (0..m).rev() {
        let (with, without): (Vec<_>, Vec<_>) = current
            .into_iter()
            .partition(|inequality| inequality.form.coefficients[k] != zero());
        let mut next = without.clone();
        let (positive, negative): (Vec<_>, Vec<_>) = with
            .iter()
            .partition(|inequality| inequality.form.coefficients[k] > zero());
        for upper in &positive {
            for lower in &negative {
                let a = upper.form.coefficients[k].clone();
                let b = lower.form.coefficients[k].neg();
                // b · upper + a · lower cancels variable k.
                let form = upper.form.times(&b).plus(&lower.form.times(&a));
                next.push(Inequality {
                    form,
                    strict: upper.strict || lower.strict,
                });
                if u64::try_from(next.len()).unwrap_or(u64::MAX) > cap {
                    return Err(NotConcretised::Exhausted);
                }
            }
        }
        levels[k] = with.into_iter().chain(without).collect();
        current = next;
    }
    for inequality in &current {
        let holds = if inequality.strict {
            inequality.form.constant < zero()
        } else {
            inequality.form.constant <= zero()
        };
        if !holds {
            return Err(NotConcretised::Infeasible);
        }
    }
    let mut values: Vec<Rational> = Vec::with_capacity(m);
    for (k, level) in levels.iter().enumerate() {
        // Bounds on variable k, the earlier ones fixed.
        let mut lower: Option<(Rational, bool)> = None;
        let mut upper: Option<(Rational, bool)> = None;
        for inequality in level {
            let a = &inequality.form.coefficients[k];
            if a == &zero() {
                continue;
            }
            let mut rest = inequality.form.constant.clone();
            for (j, value) in values.iter().enumerate() {
                rest = rest.add(&inequality.form.coefficients[j].mul(value));
            }
            // a · v + rest ≺ 0.
            let bound = rest.neg().div(a).ok_or(NotConcretised::Infeasible)?;
            if a > &zero() {
                let tighter = upper.as_ref().is_none_or(|(held, strict)| {
                    bound < *held || (bound == *held && inequality.strict && !strict)
                });
                if tighter {
                    upper = Some((bound, inequality.strict));
                }
            } else {
                let tighter = lower.as_ref().is_none_or(|(held, strict)| {
                    bound > *held || (bound == *held && inequality.strict && !strict)
                });
                if tighter {
                    lower = Some((bound, inequality.strict));
                }
            }
        }
        let floor = values.last().cloned().unwrap_or_else(zero);
        let (low, open) = lower.unwrap_or((floor, false));
        let value = if !open {
            low
        } else {
            match &upper {
                Some((high, _)) => low
                    .add(high)
                    .div(&Rational::from_integer(Integer::from(2_i64)))
                    .ok_or(NotConcretised::Infeasible)?,
                None => low.add(&one()),
            }
        };
        values.push(value);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact(n: i64, d: i64) -> Rational {
        Rational::new(Integer::from(n), Integer::from(d)).expect("a rational")
    }

    fn stage(part: Vec<PlaceConstraint>, resets: Vec<(usize, Rational)>) -> LoopStage {
        LoopStage {
            elapses: true,
            invariants: Vec::new(),
            part,
            resets,
            carries: Vec::new(),
            target: Vec::new(),
            position: Vec::new(),
            before: None,
            piece: None,
        }
    }

    /// Part of FR-241-AC-4 (TC-696): a loop of one step that resets `x`,
    /// with the delay constraints `d > 1` and `d < 1`, has no rational
    /// solution. The engine's `Undecided(LassoNotConcretized)` over a
    /// model is `zone_liveness::a_drifting_violation_has_no_periodic_counterexample`.
    #[ix_trace_rs::trace("TC-696")]
    #[test]
    fn an_unsatisfiable_loop_has_no_delays() {
        let loop_ = [stage(
            vec![
                PlaceConstraint::new(0, ClockOp::Greater, exact(1, 1)),
                PlaceConstraint::new(0, ClockOp::Less, exact(1, 1)),
            ],
            vec![(0, exact(0, 1))],
        )];
        assert!(matches!(
            solve_loop(&loop_, &BigInt::from(1), true, u64::MAX),
            Err(NotConcretised::Infeasible)
        ));
    }

    /// A carried clock retains the source clock's pre-step value through a
    /// loop, while a simultaneous reset starts the source clock afresh.
    /// Thus `x <- y; y = 0` and the guard `x >= 3` require a loop delay of
    /// at least 3/2: the carried `x` accumulates both loop delays since the
    /// prior reset of `y`.
    #[ix_trace_rs::trace("TC-696", "FR-241-AC-4")]
    #[test]
    fn a_carried_clock_uses_the_source_value_in_a_loop() {
        let mut loop_ = stage(
            vec![PlaceConstraint::new(
                0,
                ClockOp::GreaterOrEqual,
                exact(3, 1),
            )],
            vec![(1, exact(0, 1))],
        );
        loop_.carries.push((0, 1));
        assert_eq!(
            solve_loop(&[loop_], &BigInt::from(1), true, u64::MAX)
                .expect("the carried-clock loop is feasible"),
            [exact(3, 2)]
        );
    }

    /// FR-241's rule on a loop: a step resetting `x` guarded by `x >= 1`
    /// takes the least delay 1; guarded by `x > 1` and `x < 2`, the
    /// midpoint 3/2; a clock never reset bounded from above fails.
    #[ix_trace_rs::trace("TC-696")]
    #[test]
    fn loop_delays_follow_the_canonical_rule() {
        let closed = [stage(
            vec![PlaceConstraint::new(
                0,
                ClockOp::GreaterOrEqual,
                exact(1, 1),
            )],
            vec![(0, exact(0, 1))],
        )];
        assert_eq!(
            solve_loop(&closed, &BigInt::from(1), true, u64::MAX).expect("solved"),
            [exact(1, 1)]
        );
        let open = [stage(
            vec![
                PlaceConstraint::new(0, ClockOp::Greater, exact(1, 1)),
                PlaceConstraint::new(0, ClockOp::Less, exact(2, 1)),
            ],
            vec![(0, exact(0, 1))],
        )];
        assert_eq!(
            solve_loop(&open, &BigInt::from(1), true, u64::MAX).expect("solved"),
            [exact(3, 2)]
        );
        let never = [stage(
            vec![PlaceConstraint::new(0, ClockOp::LessOrEqual, exact(5, 1))],
            Vec::new(),
        )];
        assert!(solve_loop(&never, &BigInt::from(1), true, u64::MAX).is_err());
    }

    /// SR-1643001 FND-003: an elimination that needs more constraints than
    /// its cap is a budget stop, never an unsatisfiable loop.
    #[test]
    fn an_elimination_past_its_cap_is_exhausted_not_infeasible() {
        let open = [stage(
            vec![
                PlaceConstraint::new(0, ClockOp::Greater, exact(1, 1)),
                PlaceConstraint::new(0, ClockOp::Less, exact(2, 1)),
            ],
            vec![(0, exact(0, 1))],
        )];
        assert!(matches!(
            solve_loop(&open, &BigInt::from(1), true, 0),
            Err(NotConcretised::Exhausted)
        ));
    }
}
