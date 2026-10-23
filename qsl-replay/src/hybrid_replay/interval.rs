// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact rational interval evaluation of an expanded hybrid expression, for
//! static replay (FR-450 "Static replay"): `Variable` over the state's
//! interval, `Constant` as a point, `Arithmetic` and `Negate` by exact
//! interval arithmetic, a division whose divisor contains 0 as the whole
//! line, `Compare` as definitely true, definitely false or undecided, each
//! comparison widened by the trace's `delta`, `Logic` and `Not` over those
//! three values, `Conditional` as the hull of the branches its condition
//! does not exclude, and a call by its checked body over the argument
//! intervals.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use qsl_semantics::check::{ArithmeticOp, CompareOp, LogicOp};
use quire_exact::Rational;

use super::expand::{Exp, Expander, ExpansionGap, RecursiveCall, ResultKind, ValueRange};
use super::poly::sign;
use super::trace::zero;

/// How many recursive call evaluations one interval evaluation makes before
/// every further recursive call takes its result type's whole range, which
/// encloses its value, so static replay always terminates.
const RECURSIVE_EVALUATIONS: usize = 1 << 14;

/// A closed interval of the reals, either end absent when unbounded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Interval {
    pub(crate) lower: Option<Rational>,
    pub(crate) upper: Option<Rational>,
}

/// A three-valued truth value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Tri {
    True,
    False,
    Unknown,
}

impl Tri {
    pub(crate) fn from_bool(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::False
        }
    }

    pub(crate) fn not(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }

    pub(crate) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    pub(crate) fn or(self, other: Self) -> Self {
        self.not().and(other.not()).not()
    }
}

/// An interval value: a number in an interval, or a truth value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum IValue {
    Number(Interval),
    Boolean(Tri),
}

/// An extended rational, for interval end products.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Extended {
    NegativeInfinity,
    Finite(Rational),
    PositiveInfinity,
}

impl Extended {
    fn sign(&self) -> Ordering {
        match self {
            Self::NegativeInfinity => Ordering::Less,
            Self::Finite(q) => sign(q),
            Self::PositiveInfinity => Ordering::Greater,
        }
    }

    fn mul(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Finite(a), Self::Finite(b)) => Self::Finite(a.mul(b)),
            // An infinite end times a zero end is 0 in interval end products.
            _ if self.sign() == Ordering::Equal || other.sign() == Ordering::Equal => {
                Self::Finite(zero())
            }
            _ if self.sign() == other.sign() => Self::PositiveInfinity,
            _ => Self::NegativeInfinity,
        }
    }
}

impl Interval {
    pub(crate) fn point(value: Rational) -> Self {
        Self {
            lower: Some(value.clone()),
            upper: Some(value),
        }
    }

    pub(crate) fn whole() -> Self {
        Self {
            lower: None,
            upper: None,
        }
    }

    fn from_range(range: &ValueRange) -> Self {
        Self {
            lower: range.lower.clone(),
            upper: range.upper.clone(),
        }
    }

    fn low(&self) -> Extended {
        self.lower
            .clone()
            .map_or(Extended::NegativeInfinity, Extended::Finite)
    }

    fn high(&self) -> Extended {
        self.upper
            .clone()
            .map_or(Extended::PositiveInfinity, Extended::Finite)
    }

    fn from_extended(low: Extended, high: Extended) -> Self {
        let finite = |e: Extended| match e {
            Extended::Finite(q) => Some(q),
            Extended::NegativeInfinity | Extended::PositiveInfinity => None,
        };
        Self {
            lower: finite(low),
            upper: finite(high),
        }
    }

    fn add(&self, other: &Self) -> Self {
        let both = |a: &Option<Rational>, b: &Option<Rational>| match (a, b) {
            (Some(a), Some(b)) => Some(a.add(b)),
            _ => None,
        };
        Self {
            lower: both(&self.lower, &other.lower),
            upper: both(&self.upper, &other.upper),
        }
    }

    fn neg(&self) -> Self {
        Self {
            lower: self.upper.as_ref().map(Rational::neg),
            upper: self.lower.as_ref().map(Rational::neg),
        }
    }

    fn mul(&self, other: &Self) -> Self {
        let products = [
            self.low().mul(&other.low()),
            self.low().mul(&other.high()),
            self.high().mul(&other.low()),
            self.high().mul(&other.high()),
        ];
        let low = products.iter().min().cloned();
        let high = products.iter().max().cloned();
        match (low, high) {
            (Some(low), Some(high)) => Self::from_extended(low, high),
            _ => Self::whole(),
        }
    }

    fn contains_zero(&self) -> bool {
        self.low().sign() != Ordering::Greater && self.high().sign() != Ordering::Less
    }

    fn div(&self, divisor: &Self) -> Self {
        if divisor.contains_zero() {
            return Self::whole();
        }
        let reciprocal = |e: &Extended| match e {
            Extended::Finite(q) => Extended::Finite(
                Rational::from_integer(1_i64.into())
                    .div(q)
                    .unwrap_or_else(|| unreachable!("the divisor excludes 0")),
            ),
            Extended::NegativeInfinity | Extended::PositiveInfinity => Extended::Finite(zero()),
        };
        let inverse = Self::from_extended(reciprocal(&divisor.high()), reciprocal(&divisor.low()));
        self.mul(&inverse)
    }

    fn hull(&self, other: &Self) -> Self {
        let lower = match (&self.lower, &other.lower) {
            (Some(a), Some(b)) => Some(a.clone().min(b.clone())),
            _ => None,
        };
        let upper = match (&self.upper, &other.upper) {
            (Some(a), Some(b)) => Some(a.clone().max(b.clone())),
            _ => None,
        };
        Self { lower, upper }
    }

    /// The intersection, `None` when empty.
    fn intersect(&self, other: &Self) -> Option<Self> {
        let lower = match (&self.lower, &other.lower) {
            (Some(a), Some(b)) => Some(a.clone().max(b.clone())),
            (a, b) => a.clone().or_else(|| b.clone()),
        };
        let upper = match (&self.upper, &other.upper) {
            (Some(a), Some(b)) => Some(a.clone().min(b.clone())),
            (a, b) => a.clone().or_else(|| b.clone()),
        };
        match (&lower, &upper) {
            (Some(l), Some(u)) if l > u => None,
            _ => Some(Self { lower, upper }),
        }
    }
}

/// `a op b`, widened by `delta >= 0` (QSpec FR-193 "Trace replay"): `a < b`
/// reads `a < b + delta`, `a = b` reads `|a - b| <= delta`, `>` and `>=`
/// likewise, and `a != b` holds whenever `delta > 0`.
pub(crate) fn compare(op: CompareOp, a: &Interval, b: &Interval, delta: &Rational) -> Tri {
    let difference = a.add(&b.neg());
    let negative_delta = delta.neg();
    let below = |bound: &Option<Rational>, limit: &Rational, strict: bool| {
        bound
            .as_ref()
            .is_some_and(|q| if strict { q < limit } else { q <= limit })
    };
    let above = |bound: &Option<Rational>, limit: &Rational, strict: bool| {
        bound
            .as_ref()
            .is_some_and(|q| if strict { q > limit } else { q >= limit })
    };
    let (low, high) = (&difference.lower, &difference.upper);
    let decide = |holds: bool, fails: bool| {
        if holds {
            Tri::True
        } else if fails {
            Tri::False
        } else {
            Tri::Unknown
        }
    };
    match op {
        CompareOp::Lt => decide(below(high, delta, true), above(low, delta, false)),
        CompareOp::Le => decide(below(high, delta, false), above(low, delta, true)),
        CompareOp::Gt => decide(
            above(low, &negative_delta, true),
            below(high, &negative_delta, false),
        ),
        CompareOp::Ge => decide(
            above(low, &negative_delta, false),
            below(high, &negative_delta, true),
        ),
        CompareOp::Eq => decide(
            above(low, &negative_delta, false) && below(high, delta, false),
            above(low, delta, true) || below(high, &negative_delta, true),
        ),
        CompareOp::Ne => {
            if !delta.is_zero() {
                return Tri::True;
            }
            let point_zero = low.as_ref().is_some_and(Rational::is_zero)
                && high.as_ref().is_some_and(Rational::is_zero);
            decide(
                above(low, &zero(), true) || below(high, &zero(), true),
                point_zero,
            )
        }
    }
}

/// A recursive body's undecided base case: the call takes its result range.
struct Undecided;

/// The interval evaluator over one state's enclosure.
pub(crate) struct IntervalEvaluator<'e, 'g> {
    expander: &'e mut Expander<'g>,
    delta: Rational,
    bodies: BTreeMap<usize, Exp>,
    evaluations: usize,
}

impl<'e, 'g> IntervalEvaluator<'e, 'g> {
    pub(crate) fn new(expander: &'e mut Expander<'g>, delta: Rational) -> Self {
        Self {
            expander,
            delta,
            bodies: BTreeMap::new(),
            evaluations: 0,
        }
    }

    /// The value of `exp` over `variables`.
    pub(crate) fn evaluate(
        &mut self,
        exp: &Exp,
        variables: &[Interval],
    ) -> Result<IValue, ExpansionGap> {
        match self.value(exp, variables, &[], false)? {
            Ok(value) => Ok(value),
            Err(Undecided) => Ok(IValue::Boolean(Tri::Unknown)),
        }
    }

    /// The truth value of a Boolean `exp` over `variables`.
    pub(crate) fn truth(&mut self, exp: &Exp, variables: &[Interval]) -> Result<Tri, ExpansionGap> {
        Ok(match self.evaluate(exp, variables)? {
            IValue::Boolean(tri) => tri,
            IValue::Number(_) => Tri::Unknown,
        })
    }

    pub(crate) fn expander(&mut self) -> &mut Expander<'g> {
        self.expander
    }

    fn value(
        &mut self,
        exp: &Exp,
        variables: &[Interval],
        parameters: &[IValue],
        in_recursive_body: bool,
    ) -> Result<Result<IValue, Undecided>, ExpansionGap> {
        macro_rules! sub {
            ($e:expr) => {
                match self.value($e, variables, parameters, in_recursive_body)? {
                    Ok(value) => value,
                    Err(Undecided) => return Ok(Err(Undecided)),
                }
            };
        }
        let number = |value: IValue| match value {
            IValue::Number(interval) => interval,
            IValue::Boolean(_) => Interval::whole(),
        };
        let truth = |value: IValue| match value {
            IValue::Boolean(tri) => tri,
            IValue::Number(_) => Tri::Unknown,
        };
        Ok(Ok(match exp {
            Exp::Variable(i) => {
                IValue::Number(variables.get(*i).cloned().unwrap_or_else(Interval::whole))
            }
            // Static replay checks the model's expressions, which read no
            // activation state; such a read is unconstrained.
            Exp::Activation(_) | Exp::Capture(_) => IValue::Number(Interval::whole()),
            Exp::Parameter(i) => parameters
                .get(*i)
                .cloned()
                .unwrap_or(IValue::Number(Interval::whole())),
            Exp::Number(value) => IValue::Number(Interval::point(value.clone())),
            Exp::Boolean(value) => IValue::Boolean(Tri::from_bool(*value)),
            Exp::Negate(a) => IValue::Number(number(sub!(a)).neg()),
            Exp::Arithmetic(op, a, b, _) => {
                let (a, b) = (number(sub!(a)), number(sub!(b)));
                IValue::Number(match op {
                    ArithmeticOp::Add => a.add(&b),
                    ArithmeticOp::Sub => a.add(&b.neg()),
                    ArithmeticOp::Mul => a.mul(&b),
                    ArithmeticOp::Div => a.div(&b),
                })
            }
            Exp::Compare(op, a, b) => {
                let (a, b) = (number(sub!(a)), number(sub!(b)));
                IValue::Boolean(compare(*op, &a, &b, &self.delta))
            }
            Exp::Not(a) => IValue::Boolean(truth(sub!(a)).not()),
            Exp::Logic(op, a, b) => {
                let left = truth(sub!(a));
                match (op, left) {
                    (LogicOp::And, Tri::False) => IValue::Boolean(Tri::False),
                    (LogicOp::Or, Tri::True) | (LogicOp::Implies, Tri::False) => {
                        IValue::Boolean(Tri::True)
                    }
                    _ => {
                        let right = truth(sub!(b));
                        IValue::Boolean(match op {
                            LogicOp::And => left.and(right),
                            LogicOp::Or => left.or(right),
                            LogicOp::Implies => left.not().or(right),
                        })
                    }
                }
            }
            Exp::Conditional(c, a, b) => match truth(sub!(c)) {
                Tri::True => sub!(a),
                Tri::False => sub!(b),
                Tri::Unknown if in_recursive_body => return Ok(Err(Undecided)),
                Tri::Unknown => match (sub!(a), sub!(b)) {
                    (IValue::Number(a), IValue::Number(b)) => IValue::Number(a.hull(&b)),
                    (IValue::Boolean(a), IValue::Boolean(b)) if a == b => IValue::Boolean(a),
                    (IValue::Boolean(_), _) | (_, IValue::Boolean(_)) => {
                        IValue::Boolean(Tri::Unknown)
                    }
                },
            },
            Exp::Ranged { value, range, .. } => match sub!(value) {
                IValue::Number(interval) => IValue::Number(
                    interval
                        .intersect(&Interval::from_range(range))
                        // No argument in range: the call has no value, which
                        // decides no check.
                        .unwrap_or_else(Interval::whole),
                ),
                boolean @ IValue::Boolean(_) => boolean,
            },
            Exp::Recursive(call) => {
                let mut arguments = Vec::with_capacity(call.arguments.len());
                for argument in &call.arguments {
                    arguments.push(sub!(argument));
                }
                self.recursive(call, arguments)?
            }
        }))
    }

    /// A recursive call over argument intervals: its body while every
    /// parameter a measure can name is a point and the body's conditions
    /// are decided, else the result type's whole range (FR-450 "Static
    /// replay").
    fn recursive(
        &mut self,
        call: &RecursiveCall,
        arguments: Vec<IValue>,
    ) -> Result<IValue, ExpansionGap> {
        let whole = match &call.result {
            ResultKind::Boolean => IValue::Boolean(Tri::Unknown),
            ResultKind::Number(range) => IValue::Number(Interval::from_range(range)),
        };
        let definite = call.measured.iter().zip(&arguments).all(|(measured, argument)| {
            !measured
                || matches!(argument, IValue::Number(Interval { lower: Some(l), upper: Some(u) }) if l == u)
        });
        if !definite || self.evaluations >= RECURSIVE_EVALUATIONS {
            return Ok(whole);
        }
        self.evaluations += 1;
        let function = call.function.get();
        if !self.bodies.contains_key(&function) {
            let parameters = (0..call.arguments.len()).map(Exp::Parameter).collect();
            let body = self.expander.body(call.function, parameters, &call.locus)?;
            self.bodies.insert(function, body);
        }
        let body = self
            .bodies
            .get(&function)
            .cloned()
            .unwrap_or(Exp::Boolean(true));
        Ok(match self.value(&body, &[], &arguments, true)? {
            Err(Undecided) => whole,
            Ok(IValue::Number(interval)) => match &whole {
                IValue::Number(range) => {
                    IValue::Number(interval.intersect(range).unwrap_or_else(Interval::whole))
                }
                IValue::Boolean(_) => IValue::Number(interval),
            },
            Ok(boolean) => boolean,
        })
    }
}
