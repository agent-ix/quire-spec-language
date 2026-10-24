// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-450 "Exact replay": a mode's polynomial flow and its solution from a
//! point. Every rate is built from constants, variables, unary `-`, `+`,
//! `-` and `*` (a quotient by a closed constant term that is not 0 counts
//! as a constant factor), and the relation "`y` occurs in `x`'s rate" has
//! no cycle, a variable occurring in its own rate included. The solution is
//! then one polynomial in time per variable, integrated in that relation's
//! order.

use std::collections::BTreeSet;

use qsl_semantics::check::ArithmeticOp;
use quire_exact::Rational;

use super::expand::Exp;
use super::poly::Poly;

/// A mode's polynomial flow: its rates and an integration order.
#[derive(Clone, Debug)]
pub(crate) struct Flow {
    pub(crate) rates: Vec<Exp>,
    order: Vec<usize>,
}

/// The value of a term with no variable, when it is a number.
fn closed_constant(exp: &Exp) -> Option<Rational> {
    match exp {
        Exp::Number(value) => Some(value.clone()),
        Exp::Negate(a) => closed_constant(a).map(|a| a.neg()),
        Exp::Arithmetic(op, a, b, _) => {
            let (a, b) = (closed_constant(a)?, closed_constant(b)?);
            match op {
                ArithmeticOp::Add => Some(a.add(&b)),
                ArithmeticOp::Sub => Some(a.sub(&b)),
                ArithmeticOp::Mul => Some(a.mul(&b)),
                ArithmeticOp::Div => a.div(&b),
            }
        }
        _ => None,
    }
}

/// Whether `exp` is in the polynomial rate forms, collecting the variables
/// it reads.
fn polynomial_form(exp: &Exp, reads: &mut BTreeSet<usize>) -> bool {
    match exp {
        Exp::Number(_) => true,
        Exp::Variable(i) => {
            reads.insert(*i);
            true
        }
        Exp::Negate(a) => polynomial_form(a, reads),
        // A call's argument and result ranges are definedness conditions;
        // the value is the wrapped term's.
        Exp::Ranged { value, .. } => polynomial_form(value, reads),
        Exp::Arithmetic(ArithmeticOp::Div, a, b, _) => {
            closed_constant(b).is_some_and(|divisor| !divisor.is_zero())
                && polynomial_form(a, reads)
        }
        Exp::Arithmetic(_, a, b, _) => polynomial_form(a, reads) && polynomial_form(b, reads),
        _ => false,
    }
}

/// `exp` as a polynomial in time, each variable read as `solved[i]`.
fn as_poly(exp: &Exp, solved: &[Option<Poly>]) -> Option<Poly> {
    match exp {
        Exp::Number(value) => Some(Poly::constant(value.clone())),
        Exp::Variable(i) => solved.get(*i).cloned().flatten(),
        Exp::Negate(a) => as_poly(a, solved).map(|a| a.neg()),
        Exp::Ranged { value, .. } => as_poly(value, solved),
        Exp::Arithmetic(ArithmeticOp::Div, a, b, _) => {
            let divisor = closed_constant(b)?;
            let inverse = Rational::from_integer(1_i64.into()).div(&divisor)?;
            as_poly(a, solved).map(|a| a.scale(&inverse))
        }
        Exp::Arithmetic(op, a, b, _) => {
            let (a, b) = (as_poly(a, solved)?, as_poly(b, solved)?);
            Some(match op {
                ArithmeticOp::Add => a.add(&b),
                ArithmeticOp::Sub => a.sub(&b),
                ArithmeticOp::Mul => a.mul(&b),
                ArithmeticOp::Div => return None,
            })
        }
        _ => None,
    }
}

impl Flow {
    /// The flow of `rates`, `rates[i]` the rate of variable `i`; `None` when
    /// it is not a polynomial flow.
    pub(crate) fn of(rates: Vec<Exp>) -> Option<Self> {
        let mut reads = Vec::with_capacity(rates.len());
        for rate in &rates {
            let mut read = BTreeSet::new();
            if !polynomial_form(rate, &mut read) || read.iter().any(|i| *i >= rates.len()) {
                return None;
            }
            reads.push(read);
        }
        // Integrate each variable after every variable its rate reads; a
        // variable left over is on a cycle.
        let mut order = Vec::with_capacity(rates.len());
        let mut placed = vec![false; rates.len()];
        while order.len() < rates.len() {
            let next = (0..rates.len())
                .find(|&i| !placed[i] && reads[i].iter().all(|&j| placed[j] && j != i))?;
            placed[next] = true;
            order.push(next);
        }
        Some(Self { rates, order })
    }

    /// The solution from `start` at local time 0, one polynomial in local
    /// time per variable.
    pub(crate) fn solve(&self, start: &[Rational]) -> Vec<Poly> {
        let mut solved: Vec<Option<Poly>> = vec![None; self.rates.len()];
        for &i in &self.order {
            let rate = as_poly(&self.rates[i], &solved).unwrap_or_else(|| {
                unreachable!("a polynomial flow's rate reads only solved variables")
            });
            let initial = start.get(i).cloned().unwrap_or_else(super::trace::zero);
            solved[i] = Some(rate.integral().add(&Poly::constant(initial)));
        }
        solved
            .into_iter()
            .map(|poly| poly.unwrap_or_else(Poly::zero))
            .collect()
    }
}
