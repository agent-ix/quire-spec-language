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
use std::sync::Arc;

use qsl_semantics::check::{ArithmeticOp, CompareOp, LogicOp};
use quire_exact::Rational;

use super::expand::{Exp, Expander, RecursiveCall, ResultKind, ValueRange};
use super::poly::sign;
use super::trace::zero;
use super::{HybridReplayMeter, HybridReplayRefusal};

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

/// A pending interval-expression step. Recursive calls push their bodies on
/// this heap stack, so their depth does not consume the Rust call stack.
enum Work {
    Eval {
        exp: Box<Exp>,
        parameters: Arc<[IValue]>,
        in_recursive_body: bool,
    },
    Negate,
    Not,
    Ranged(ValueRange),
    BinaryLeft {
        op: Binary,
        right: Box<Exp>,
        parameters: Arc<[IValue]>,
        in_recursive_body: bool,
    },
    BinaryRight {
        op: Binary,
        left: IValue,
    },
    Conditional {
        yes: Box<Exp>,
        no: Box<Exp>,
        parameters: Arc<[IValue]>,
        in_recursive_body: bool,
    },
    ConditionalUnknownLeft {
        no: Box<Exp>,
        parameters: Arc<[IValue]>,
        in_recursive_body: bool,
    },
    ConditionalUnknownRight {
        yes: IValue,
    },
    RecursiveArguments {
        call: Box<RecursiveCall>,
        count: usize,
    },
    RecursiveResult {
        whole: IValue,
    },
}

#[derive(Clone, Copy)]
enum Binary {
    Arithmetic(ArithmeticOp),
    Compare(CompareOp),
    Logic(LogicOp),
}

fn number(value: IValue) -> Interval {
    match value {
        IValue::Number(interval) => interval,
        IValue::Boolean(_) => Interval::whole(),
    }
}

fn truth(value: IValue) -> Tri {
    match value {
        IValue::Boolean(tri) => tri,
        IValue::Number(_) => Tri::Unknown,
    }
}

fn whole_result(result: &ResultKind) -> IValue {
    match result {
        ResultKind::Boolean => IValue::Boolean(Tri::Unknown),
        ResultKind::Number(range) => IValue::Number(Interval::from_range(range)),
    }
}

/// The interval evaluator over one state's enclosure.
pub(crate) struct IntervalEvaluator<'e, 'g, 'b> {
    expander: &'e mut Expander<'g>,
    meter: &'b mut HybridReplayMeter,
    delta: Rational,
    bodies: BTreeMap<usize, Exp>,
}

impl<'e, 'g, 'b> IntervalEvaluator<'e, 'g, 'b> {
    pub(crate) fn new(
        expander: &'e mut Expander<'g>,
        delta: Rational,
        meter: &'b mut HybridReplayMeter,
    ) -> Self {
        Self {
            expander,
            meter,
            delta,
            bodies: BTreeMap::new(),
        }
    }

    /// The value of `exp` over `variables`.
    pub(crate) fn evaluate(
        &mut self,
        exp: Exp,
        variables: &[Interval],
    ) -> Result<IValue, HybridReplayRefusal> {
        match self.value(exp, variables, &[], false)? {
            Ok(value) => Ok(value),
            Err(Undecided) => Ok(IValue::Boolean(Tri::Unknown)),
        }
    }

    /// The truth value of a Boolean `exp` over `variables`.
    pub(crate) fn truth(
        &mut self,
        exp: Exp,
        variables: &[Interval],
    ) -> Result<Tri, HybridReplayRefusal> {
        Ok(match self.evaluate(exp, variables)? {
            IValue::Boolean(tri) => tri,
            IValue::Number(_) => Tri::Unknown,
        })
    }

    pub(crate) fn expand(
        &mut self,
        term: &qsl_semantics::check::HybridTerm,
    ) -> Result<Exp, HybridReplayRefusal> {
        self.expander.term(term, self.meter)
    }

    fn value(
        &mut self,
        exp: Exp,
        variables: &[Interval],
        parameters: &[IValue],
        in_recursive_body: bool,
    ) -> Result<Result<IValue, Undecided>, HybridReplayRefusal> {
        let mut work = vec![Work::Eval {
            exp: Box::new(exp),
            parameters: Arc::from(parameters),
            in_recursive_body,
        }];
        let mut values: Vec<Result<IValue, Undecided>> = Vec::new();
        while let Some(step) = work.pop() {
            match step {
                Work::Eval {
                    exp,
                    parameters,
                    in_recursive_body,
                } => {
                    self.meter.charge()?;
                    match *exp {
                        Exp::Variable(i) => {
                            let interval = if in_recursive_body {
                                None
                            } else {
                                variables.get(i)
                            };
                            values.push(Ok(IValue::Number(
                                interval.cloned().unwrap_or_else(Interval::whole),
                            )));
                        }
                        // Model expressions read no activation state.
                        Exp::Activation(_) | Exp::Capture(_) => {
                            values.push(Ok(IValue::Number(Interval::whole())));
                        }
                        Exp::Parameter(i) => values.push(Ok(parameters
                            .get(i)
                            .cloned()
                            .unwrap_or(IValue::Number(Interval::whole())))),
                        Exp::Number(value) => {
                            values.push(Ok(IValue::Number(Interval::point(value))));
                        }
                        Exp::Boolean(value) => {
                            values.push(Ok(IValue::Boolean(Tri::from_bool(value))));
                        }
                        Exp::Negate(a) => {
                            work.push(Work::Negate);
                            work.push(Work::Eval {
                                exp: a,
                                parameters,
                                in_recursive_body,
                            });
                        }
                        Exp::Not(a) => {
                            work.push(Work::Not);
                            work.push(Work::Eval {
                                exp: a,
                                parameters,
                                in_recursive_body,
                            });
                        }
                        Exp::Ranged { value, range, .. } => {
                            work.push(Work::Ranged(range));
                            work.push(Work::Eval {
                                exp: value,
                                parameters,
                                in_recursive_body,
                            });
                        }
                        Exp::Arithmetic(op, a, b, _) => {
                            work.push(Work::BinaryLeft {
                                op: Binary::Arithmetic(op),
                                right: b,
                                parameters: parameters.clone(),
                                in_recursive_body,
                            });
                            work.push(Work::Eval {
                                exp: a,
                                parameters,
                                in_recursive_body,
                            });
                        }
                        Exp::Compare(op, a, b) => {
                            work.push(Work::BinaryLeft {
                                op: Binary::Compare(op),
                                right: b,
                                parameters: parameters.clone(),
                                in_recursive_body,
                            });
                            work.push(Work::Eval {
                                exp: a,
                                parameters,
                                in_recursive_body,
                            });
                        }
                        Exp::Logic(op, a, b) => {
                            work.push(Work::BinaryLeft {
                                op: Binary::Logic(op),
                                right: b,
                                parameters: parameters.clone(),
                                in_recursive_body,
                            });
                            work.push(Work::Eval {
                                exp: a,
                                parameters,
                                in_recursive_body,
                            });
                        }
                        Exp::Conditional(c, a, b) => {
                            work.push(Work::Conditional {
                                yes: a,
                                no: b,
                                parameters: parameters.clone(),
                                in_recursive_body,
                            });
                            work.push(Work::Eval {
                                exp: c,
                                parameters,
                                in_recursive_body,
                            });
                        }
                        Exp::Recursive(mut call) => {
                            let arguments = std::mem::take(&mut call.arguments);
                            let count = arguments.len();
                            work.push(Work::RecursiveArguments {
                                call: Box::new(call),
                                count,
                            });
                            for argument in arguments.into_iter().rev() {
                                work.push(Work::Eval {
                                    exp: Box::new(argument),
                                    parameters: parameters.clone(),
                                    in_recursive_body,
                                });
                            }
                        }
                    }
                }
                Work::Negate => {
                    let result = values
                        .pop()
                        .expect("operand is evaluated")
                        .map(|v| IValue::Number(number(v).neg()));
                    values.push(result);
                }
                Work::Not => {
                    let result = values
                        .pop()
                        .expect("operand is evaluated")
                        .map(|v| IValue::Boolean(truth(v).not()));
                    values.push(result);
                }
                Work::Ranged(range) => {
                    let result = values
                        .pop()
                        .expect("operand is evaluated")
                        .map(|v| match v {
                            IValue::Number(interval) => IValue::Number(
                                interval
                                    .intersect(&Interval::from_range(&range))
                                    .unwrap_or_else(Interval::whole),
                            ),
                            boolean @ IValue::Boolean(_) => boolean,
                        });
                    values.push(result);
                }
                Work::BinaryLeft {
                    op,
                    right,
                    parameters,
                    in_recursive_body,
                } => match values.pop().expect("left operand is evaluated") {
                    Err(undecided) => values.push(Err(undecided)),
                    Ok(left) => {
                        let shortcut = match (op, truth(left.clone())) {
                            (Binary::Logic(LogicOp::And), Tri::False) => Some(Tri::False),
                            (Binary::Logic(LogicOp::Or), Tri::True)
                            | (Binary::Logic(LogicOp::Implies), Tri::False) => Some(Tri::True),
                            _ => None,
                        };
                        if let Some(result) = shortcut {
                            values.push(Ok(IValue::Boolean(result)));
                        } else {
                            work.push(Work::BinaryRight { op, left });
                            work.push(Work::Eval {
                                exp: right,
                                parameters,
                                in_recursive_body,
                            });
                        }
                    }
                },
                Work::BinaryRight { op, left } => {
                    let result = values
                        .pop()
                        .expect("right operand is evaluated")
                        .map(|right| match op {
                            Binary::Arithmetic(op) => {
                                let (a, b) = (number(left), number(right));
                                IValue::Number(match op {
                                    ArithmeticOp::Add => a.add(&b),
                                    ArithmeticOp::Sub => a.add(&b.neg()),
                                    ArithmeticOp::Mul => a.mul(&b),
                                    ArithmeticOp::Div => a.div(&b),
                                })
                            }
                            Binary::Compare(op) => IValue::Boolean(compare(
                                op,
                                &number(left),
                                &number(right),
                                &self.delta,
                            )),
                            Binary::Logic(op) => {
                                let (a, b) = (truth(left), truth(right));
                                IValue::Boolean(match op {
                                    LogicOp::And => a.and(b),
                                    LogicOp::Or => a.or(b),
                                    LogicOp::Implies => a.not().or(b),
                                })
                            }
                        });
                    values.push(result);
                }
                Work::Conditional {
                    yes,
                    no,
                    parameters,
                    in_recursive_body,
                } => match values.pop().expect("condition is evaluated") {
                    Err(undecided) => values.push(Err(undecided)),
                    Ok(condition) => match truth(condition) {
                        Tri::True => work.push(Work::Eval {
                            exp: yes,
                            parameters,
                            in_recursive_body,
                        }),
                        Tri::False => work.push(Work::Eval {
                            exp: no,
                            parameters,
                            in_recursive_body,
                        }),
                        Tri::Unknown if in_recursive_body => values.push(Err(Undecided)),
                        Tri::Unknown => {
                            work.push(Work::ConditionalUnknownLeft {
                                no,
                                parameters: parameters.clone(),
                                in_recursive_body,
                            });
                            work.push(Work::Eval {
                                exp: yes,
                                parameters,
                                in_recursive_body,
                            });
                        }
                    },
                },
                Work::ConditionalUnknownLeft {
                    no,
                    parameters,
                    in_recursive_body,
                } => match values.pop().expect("first branch is evaluated") {
                    Err(undecided) => values.push(Err(undecided)),
                    Ok(yes) => {
                        work.push(Work::ConditionalUnknownRight { yes });
                        work.push(Work::Eval {
                            exp: no,
                            parameters,
                            in_recursive_body,
                        });
                    }
                },
                Work::ConditionalUnknownRight { yes } => {
                    let result =
                        values.pop().expect("second branch is evaluated").map(|no| {
                            match (yes, no) {
                                (IValue::Number(a), IValue::Number(b)) => {
                                    IValue::Number(a.hull(&b))
                                }
                                (IValue::Boolean(a), IValue::Boolean(b)) if a == b => {
                                    IValue::Boolean(a)
                                }
                                _ => IValue::Boolean(Tri::Unknown),
                            }
                        });
                    values.push(result);
                }
                Work::RecursiveArguments { call, count } => {
                    let mut arguments = Vec::with_capacity(count);
                    let mut undecided = false;
                    for _ in 0..count {
                        match values.pop().expect("call argument is evaluated") {
                            Ok(value) => arguments.push(value),
                            Err(Undecided) => undecided = true,
                        }
                    }
                    if undecided {
                        values.push(Err(Undecided));
                        continue;
                    }
                    arguments.reverse();
                    let whole = whole_result(&call.result);
                    let definite = call.measured.iter().zip(&arguments).all(|(measured, argument)| {
                        !measured
                            || matches!(argument, IValue::Number(Interval { lower: Some(l), upper: Some(u) }) if l == u)
                    });
                    if !definite {
                        values.push(Ok(whole));
                        continue;
                    }
                    self.meter.charge()?;
                    let function = call.function.get();
                    if !self.bodies.contains_key(&function) {
                        let parameters = (0..count).map(Exp::Parameter).collect();
                        let body = self.expander.body(
                            call.function,
                            parameters,
                            &call.locus,
                            self.meter,
                        )?;
                        self.bodies.insert(function, body);
                    }
                    let body = self
                        .bodies
                        .get(&function)
                        .cloned()
                        .unwrap_or(Exp::Boolean(true));
                    work.push(Work::RecursiveResult { whole });
                    work.push(Work::Eval {
                        exp: Box::new(body),
                        parameters: Arc::from(arguments),
                        in_recursive_body: true,
                    });
                }
                Work::RecursiveResult { whole } => {
                    let result = match values.pop().expect("recursive body is evaluated") {
                        Err(Undecided) => whole,
                        Ok(IValue::Number(interval)) => match &whole {
                            IValue::Number(range) => IValue::Number(
                                interval.intersect(range).unwrap_or_else(Interval::whole),
                            ),
                            IValue::Boolean(_) => IValue::Number(interval),
                        },
                        Ok(boolean) => boolean,
                    };
                    values.push(Ok(result));
                }
            }
        }
        Ok(values.pop().expect("root expression is evaluated"))
    }
}
