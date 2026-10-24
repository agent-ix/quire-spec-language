// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact evaluation of an expanded expression along a dwell, where every
//! variable is a polynomial in time (FR-450 "Exact replay"): at an instant
//! every numeric subterm is a rational function of time and every decision
//! the sign of a polynomial there, decided by exact real root isolation. A
//! stretch of instants is split at the roots of every polynomial a decision
//! read, so each resulting cell takes one value throughout.

use std::cmp::Ordering;

use qsl_semantics::check::{ArithmeticOp, CompareOp, LogicOp};
use quire_exact::Rational;

use super::expand::{Exp, RangeRole, TermLocus};
use super::poly::{Poly, RatFunc};
use super::real::ExactReal;
use super::{ExactGap, UndefinedCause};

/// A capture's value, read throughout its obligation.
#[derive(Clone, Debug)]
pub(crate) enum Constant {
    Number(Rational),
    Boolean(bool),
}

/// What an evaluation reads: each variable as a polynomial in the clock,
/// each capture, and each activation variable.
pub(crate) struct Env<'a> {
    pub(crate) variables: &'a [Poly],
    pub(crate) captures: &'a [Constant],
    pub(crate) activation: &'a [Rational],
}

/// An exact value at one instant.
#[derive(Clone, Debug)]
pub(crate) enum Ev {
    Number(RatFunc),
    Boolean(bool),
    /// The expression has no value there.
    Undefined {
        locus: TermLocus,
        cause: UndefinedCause,
    },
}

/// The value of `exp` at the instant `at`, pushing every polynomial whose
/// sign a decision read.
pub(crate) fn eval(
    exp: &Exp,
    env: &Env<'_>,
    at: &ExactReal,
    decisions: &mut Vec<Poly>,
) -> Result<Ev, ExactGap> {
    macro_rules! number {
        ($e:expr) => {
            match eval($e, env, at, decisions)? {
                Ev::Number(value) => value,
                undefined @ Ev::Undefined { .. } => return Ok(undefined),
                Ev::Boolean(_) => return Err(ExactGap::IllKindedTerm),
            }
        };
    }
    macro_rules! truth {
        ($e:expr) => {
            match eval($e, env, at, decisions)? {
                Ev::Boolean(value) => value,
                undefined @ Ev::Undefined { .. } => return Ok(undefined),
                Ev::Number(_) => return Err(ExactGap::IllKindedTerm),
            }
        };
    }
    let sign = |value: &RatFunc, decisions: &mut Vec<Poly>| {
        let poly = value.sign_poly();
        let sign = at.sign_of(&poly);
        decisions.push(poly);
        sign
    };
    Ok(match exp {
        Exp::Variable(i) => Ev::Number(RatFunc::poly(
            env.variables
                .get(*i)
                .cloned()
                .ok_or(ExactGap::IllKindedTerm)?,
        )),
        Exp::Activation(i) => Ev::Number(RatFunc::constant(
            env.activation
                .get(*i)
                .cloned()
                .ok_or(ExactGap::ActivationState)?,
        )),
        Exp::Capture(i) => match env.captures.get(*i).ok_or(ExactGap::ActivationState)? {
            Constant::Number(value) => Ev::Number(RatFunc::constant(value.clone())),
            Constant::Boolean(value) => Ev::Boolean(*value),
        },
        Exp::Parameter(_) | Exp::Recursive(_) => return Err(ExactGap::RecursiveCall),
        Exp::Number(value) => Ev::Number(RatFunc::constant(value.clone())),
        Exp::Boolean(value) => Ev::Boolean(*value),
        Exp::Negate(a) => Ev::Number(number!(a).neg()),
        Exp::Arithmetic(op, a, b, locus) => {
            let (a, b) = (number!(a), number!(b));
            Ev::Number(match op {
                ArithmeticOp::Add => a.add(&b),
                ArithmeticOp::Sub => a.sub(&b),
                ArithmeticOp::Mul => a.mul(&b),
                ArithmeticOp::Div => {
                    let divisor = b.numerator.clone();
                    let zero = at.sign_of(&divisor) == Ordering::Equal;
                    decisions.push(divisor);
                    if zero {
                        return Ok(Ev::Undefined {
                            locus: locus.clone(),
                            cause: UndefinedCause::DivisionByZero,
                        });
                    }
                    a.div(&b)
                }
            })
        }
        Exp::Compare(op, a, b) => {
            let difference = number!(a).sub(&number!(b));
            let s = sign(&difference, decisions);
            Ev::Boolean(match op {
                CompareOp::Eq => s == Ordering::Equal,
                CompareOp::Ne => s != Ordering::Equal,
                CompareOp::Lt => s == Ordering::Less,
                CompareOp::Le => s != Ordering::Greater,
                CompareOp::Gt => s == Ordering::Greater,
                CompareOp::Ge => s != Ordering::Less,
            })
        }
        Exp::Not(a) => Ev::Boolean(!truth!(a)),
        Exp::Logic(op, a, b) => {
            let left = truth!(a);
            match (op, left) {
                (LogicOp::And, false) => Ev::Boolean(false),
                (LogicOp::Or, true) | (LogicOp::Implies, false) => Ev::Boolean(true),
                (LogicOp::And | LogicOp::Or | LogicOp::Implies, _) => Ev::Boolean(truth!(b)),
            }
        }
        Exp::Conditional(c, a, b) => {
            if truth!(c) {
                eval(a, env, at, decisions)?
            } else {
                eval(b, env, at, decisions)?
            }
        }
        Exp::Ranged {
            value,
            range,
            role,
            locus,
        } => {
            let value = number!(value);
            let mut inside = true;
            if let Some(lower) = &range.lower {
                inside &= sign(&value.sub(&RatFunc::constant(lower.clone())), decisions)
                    != Ordering::Less;
            }
            if let Some(upper) = &range.upper {
                inside &= sign(&RatFunc::constant(upper.clone()).sub(&value), decisions)
                    != Ordering::Less;
            }
            if !inside {
                return Ok(Ev::Undefined {
                    locus: locus.clone(),
                    cause: match role {
                        RangeRole::Parameter {
                            function,
                            parameter,
                        } => UndefinedCause::OutsideParameterRange {
                            function: *function,
                            parameter: *parameter,
                        },
                        RangeRole::Result { function } => UndefinedCause::OutsideResultRange {
                            function: *function,
                        },
                    },
                });
            }
            Ev::Number(value)
        }
    })
}

/// A stretch of a segment over which every evaluated expression takes one
/// value: the instant `low` when `point`, else the open interval
/// `(low, high)`.
#[derive(Clone, Debug)]
pub(crate) struct Cell {
    pub(crate) low: ExactReal,
    pub(crate) high: ExactReal,
    pub(crate) point: bool,
    pub(crate) values: Vec<Ev>,
}

/// A rational at or below `x`.
fn floor_bound(x: &ExactReal) -> Rational {
    match x {
        ExactReal::Rational(q) => q.clone(),
        ExactReal::Root(root) => root.lower().clone(),
    }
}

/// A rational at or above `x`.
fn ceiling_bound(x: &ExactReal) -> Rational {
    match x {
        ExactReal::Rational(q) => q.clone(),
        ExactReal::Root(root) => root.upper().clone(),
    }
}

fn eval_all(exps: &[Exp], env: &Env<'_>, at: &ExactReal) -> Result<(Vec<Ev>, Vec<Poly>), ExactGap> {
    let mut decisions = Vec::new();
    let values = exps
        .iter()
        .map(|exp| eval(exp, env, at, &mut decisions))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((values, decisions))
}

/// The cells of `[low, high]` (`[low, high)` when `!high_closed`), in
/// order, over which each of `exps` takes one value.
pub(crate) fn cells(
    exps: &[Exp],
    env: &Env<'_>,
    low: &ExactReal,
    high: &ExactReal,
    high_closed: bool,
) -> Result<Vec<Cell>, ExactGap> {
    let mut done = Vec::new();
    let point = |at: &ExactReal, done: &mut Vec<Cell>| -> Result<(), ExactGap> {
        let (values, _) = eval_all(exps, env, at)?;
        done.push(Cell {
            low: at.clone(),
            high: at.clone(),
            point: true,
            values,
        });
        Ok(())
    };
    point(low, &mut done)?;
    let mut pending = Vec::new();
    if low < high {
        if high_closed {
            point(high, &mut done)?;
        }
        pending.push((low.clone(), high.clone()));
    }
    while let Some((a, b)) = pending.pop() {
        let sample = ExactReal::Rational(a.between(&b));
        let (values, decisions) = eval_all(exps, env, &sample)?;
        let mut splits: Vec<ExactReal> = Vec::new();
        for decision in &decisions {
            for root in decision.roots_in(&floor_bound(&a), &ceiling_bound(&b)) {
                if root > a && root < b {
                    splits.push(root);
                }
            }
        }
        if splits.is_empty() {
            done.push(Cell {
                low: a,
                high: b,
                point: false,
                values,
            });
            continue;
        }
        splits.sort();
        splits.dedup();
        let mut from = a;
        for split in splits {
            point(&split, &mut done)?;
            pending.push((from, split.clone()));
            from = split;
        }
        pending.push((from, b));
    }
    done.sort_by(|x, y| x.low.cmp(&y.low).then(y.point.cmp(&x.point)));
    Ok(done)
}
