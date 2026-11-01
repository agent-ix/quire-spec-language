// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 "Refinement queries", normal form: the rewriting a refinement
//! query's assertions pass through before they are printed. It removes the
//! shapes its rules name and no others: an arithmetic atom keeps the
//! orientation it is built in.
//!
//! [`Term::normal`] works bottom up and preserves the meaning of the term:
//!
//! - a Boolean connective over a constant folds, `(not (not t))` is `t`,
//!   and `(=> l r)` is `(or (not l) r)`;
//! - a conjunction or disjunction splices its nested conjunctions or
//!   disjunctions in place, drops its neutral constant and every operand
//!   equal to an earlier one, and is its absorbing constant when it holds
//!   one; with one operand left it is that operand, with none its neutral
//!   constant;
//! - an equality with a Boolean constant is the other side or its
//!   negation, an equality or comparison of two literals or of a term with
//!   itself folds, and a comparison or equality of a sum ending in a literal
//!   with a literal moves that summand to the literal side;
//! - an `ite` with a constant condition, equal branches or a Boolean
//!   constant branch is its branch or the connective it stands for;
//! - a sum splices its nested sums and gathers its literals into one last
//!   summand, dropped when it is 0; linear sums use the coefficient map
//!   below, whose only subtraction form is the exact +1/-1 atom pair; a
//!   difference, negation or product of literals folds, `(- 0 t)` is
//!   `(- t)`, `(* 1 t)` is `t`, `(* 0 t)` is 0 and `(- (- t))` is `t`;
//! - `mod` of two literals is SMT-LIB's Euclidean remainder, in `[0, |n|)`.

use quire_exact::{Integer, NonZeroInteger};

use crate::smt::script::{Comparison, Term};

impl Term {
    /// This term in FR-315's refinement-query normal form.
    #[must_use]
    pub fn normal(self) -> Self {
        match self {
            Self::Cond(value) => Self::Bool(value),
            Self::Bool(_) | Self::Int(_) | Self::Const(_) => self,
            Self::Not(inner) => negation(inner.normal()),
            Self::And(terms) => junction(true, terms.into_iter().map(Self::normal)),
            Self::Or(terms) => junction(false, terms.into_iter().map(Self::normal)),
            Self::Implies(left, right) => {
                junction(false, [negation(left.normal()), right.normal()])
            }
            Self::Equal(left, right) => equality(left.normal(), right.normal()),
            Self::Compare(op, left, right) => comparison(op, left.normal(), right.normal()),
            Self::Add(terms) => linear_form(sum(terms.into_iter().map(Self::normal))),
            Self::Subtract(left, right) => linear_form(difference(left.normal(), right.normal())),
            Self::Negate(inner) => linear_form(opposite(inner.normal())),
            Self::Scale(factor, inner) => linear_form(product(factor, inner.normal())),
            Self::Modulo(inner, divisor) => remainder(inner.normal(), divisor),
            Self::If(condition, then, otherwise) => {
                choice(condition.normal(), then.normal(), otherwise.normal())
            }
        }
    }

    fn constant_value(&self) -> Option<bool> {
        match self {
            Self::Bool(value) | Self::Cond(value) => Some(*value),
            _ => None,
        }
    }
}

/// `(not t)` of a normal `t`.
fn negation(term: Term) -> Term {
    match term {
        Term::Bool(value) | Term::Cond(value) => Term::Bool(!value),
        Term::Not(inner) => *inner,
        other => Term::Not(Box::new(other)),
    }
}

/// The conjunction (`conjunction`) or disjunction of normal `terms`.
fn junction(conjunction: bool, terms: impl IntoIterator<Item = Term>) -> Term {
    let mut kept: Vec<Term> = Vec::new();
    let mut pending: Vec<Term> = terms.into_iter().collect();
    pending.reverse();
    while let Some(term) = pending.pop() {
        match term {
            Term::And(inner) if conjunction => pending.extend(inner.into_iter().rev()),
            Term::Or(inner) if !conjunction => pending.extend(inner.into_iter().rev()),
            constant @ (Term::Bool(_) | Term::Cond(_)) => {
                if constant.constant_value() != Some(conjunction) {
                    return Term::Bool(!conjunction);
                }
            }
            other => {
                if !kept.contains(&other) {
                    kept.push(other);
                }
            }
        }
    }
    match kept.len() {
        0 => Term::Bool(conjunction),
        1 => kept.remove(0),
        _ if conjunction => Term::And(kept),
        _ => Term::Or(kept),
    }
}

/// `(= l r)` of normal `l` and `r`.
fn equality(left: Term, right: Term) -> Term {
    if left == right {
        return Term::Bool(true);
    }
    match (left.constant_value(), right.constant_value()) {
        (Some(l), Some(r)) => return Term::Bool(l == r),
        (Some(value), None) => return if value { right } else { negation(right) },
        (None, Some(value)) => return if value { left } else { negation(left) },
        (None, None) => {}
    }
    if let (Term::Int(l), Term::Int(r)) = (&left, &right) {
        return Term::Bool(l == r);
    }
    let (left, right) = shifted(left, right);
    Term::Equal(Box::new(left), Box::new(right))
}

/// `(op l r)` of normal `l` and `r`.
fn comparison(op: Comparison, left: Term, right: Term) -> Term {
    if let (Term::Int(l), Term::Int(r)) = (&left, &right) {
        return Term::Bool(match op {
            Comparison::Less => l < r,
            Comparison::LessOrEqual => l <= r,
            Comparison::Greater => l > r,
            Comparison::GreaterOrEqual => l >= r,
        });
    }
    if left == right {
        return Term::Bool(matches!(
            op,
            Comparison::LessOrEqual | Comparison::GreaterOrEqual
        ));
    }
    let (left, right) = shifted(left, right);
    Term::Compare(op, Box::new(left), Box::new(right))
}

/// A relation's sides with a sum's last literal summand moved to a literal
/// on the other side: `(+ t c) ~ d` is `t ~ d-c`, `d ~ (+ t c)` is
/// `d-c ~ t`.
fn shifted(left: Term, right: Term) -> (Term, Term) {
    match (left, right) {
        (Term::Add(terms), Term::Int(bound)) => {
            let (rest, moved) = split_literal(terms);
            (rest, Term::Int(bound.sub(&moved)))
        }
        (Term::Int(bound), Term::Add(terms)) => {
            let (rest, moved) = split_literal(terms);
            (Term::Int(bound.sub(&moved)), rest)
        }
        sides => sides,
    }
}

/// A normal sum's non-literal part and its literal summand (0 when it has
/// none).
fn split_literal(mut terms: Vec<Term>) -> (Term, Integer) {
    let moved = match terms.last() {
        Some(Term::Int(value)) => {
            let value = value.clone();
            terms.pop();
            value
        }
        _ => Integer::zero(),
    };
    (sum(terms), moved)
}

/// The sum of normal `terms`.
fn sum(terms: impl IntoIterator<Item = Term>) -> Term {
    let mut kept = Vec::new();
    let mut literal = Integer::zero();
    let mut pending: Vec<Term> = terms.into_iter().collect();
    pending.reverse();
    while let Some(term) = pending.pop() {
        match term {
            Term::Add(inner) => pending.extend(inner.into_iter().rev()),
            Term::Int(value) => literal = literal.add(&value),
            other => kept.push(other),
        }
    }
    if !literal.is_zero() {
        kept.push(Term::Int(literal));
    }
    match kept.len() {
        0 => Term::Int(Integer::zero()),
        1 => kept.remove(0),
        2 => {
            let second = kept.remove(1);
            let first = kept.remove(0);
            match second {
                Term::Negate(subtrahend) => difference(first, *subtrahend),
                second => Term::Add(vec![first, second]),
            }
        }
        _ => Term::Add(kept),
    }
}

/// `(- t)` of a normal `t`.
fn opposite(term: Term) -> Term {
    match term {
        Term::Int(value) => Term::Int(value.neg()),
        Term::Negate(twice) => *twice,
        other => Term::Negate(Box::new(other)),
    }
}

/// `(mod t divisor)` of a normal `t`: SMT-LIB's Euclidean remainder, in
/// `[0, |divisor|)`, when `t` is a literal and `divisor` is not 0.
fn remainder(term: Term, divisor: Integer) -> Term {
    let magnitude = divisor.abs();
    match (&term, NonZeroInteger::try_from(&magnitude)) {
        (Term::Int(value), Ok(modulus)) => Term::Int(value.div_mod_floor(modulus).1),
        _ => Term::Modulo(Box::new(term), divisor),
    }
}

/// `(- l r)` of normal `l` and `r`.
fn difference(left: Term, right: Term) -> Term {
    match (left, right) {
        (Term::Int(l), Term::Int(r)) => Term::Int(l.sub(&r)),
        (left, Term::Int(r)) if r.is_zero() => left,
        (Term::Int(l), right) if l.is_zero() => opposite(right),
        (left, right) if left == right => Term::Int(Integer::zero()),
        (left, right) => Term::Subtract(Box::new(left), Box::new(right)),
    }
}

/// `(* factor t)` of a normal `t`.
fn product(factor: Integer, term: Term) -> Term {
    if factor.is_zero() {
        return Term::Int(Integer::zero());
    }
    if factor == Integer::one() {
        return term;
    }
    match term {
        Term::Int(value) => Term::Int(factor.mul(&value)),
        Term::Scale(inner, base) => product(factor.mul(&inner), *base),
        other => Term::Scale(factor, Box::new(other)),
    }
}

/// Collect an integer-linear expression into coefficients of its atoms.
/// Unsupported arithmetic shapes remain opaque atoms, and no product of two
/// non-constant terms is formed.
fn linear_form(term: Term) -> Term {
    fn collect(
        term: Term,
        factor: Integer,
        atoms: &mut Vec<(Term, Integer)>,
        constant: &mut Integer,
    ) {
        if factor.is_zero() {
            return;
        }
        match term {
            Term::Int(value) => *constant = constant.add(&factor.mul(&value)),
            Term::Add(terms) => {
                for term in terms {
                    collect(term, factor.clone(), atoms, constant);
                }
            }
            Term::Subtract(left, right) => {
                collect(*left, factor.clone(), atoms, constant);
                collect(*right, factor.neg(), atoms, constant);
            }
            Term::Negate(inner) => collect(*inner, factor.neg(), atoms, constant),
            Term::Scale(scale, inner) => collect(*inner, factor.mul(&scale), atoms, constant),
            atom => {
                if let Some((_, coefficient)) = atoms.iter_mut().find(|(seen, _)| *seen == atom) {
                    *coefficient = coefficient.add(&factor);
                } else {
                    atoms.push((atom, factor));
                }
            }
        }
    }

    let mut atoms = Vec::new();
    let mut constant = Integer::zero();
    collect(term, Integer::one(), &mut atoms, &mut constant);
    atoms.sort_by(|(left, _), (right, _)| left.text().as_bytes().cmp(right.text().as_bytes()));
    let mut terms = Vec::new();
    for (atom, coefficient) in atoms {
        if coefficient.is_zero() {
            continue;
        }
        terms.push(if coefficient == Integer::one() {
            atom
        } else if coefficient == Integer::from(-1_i64) {
            Term::Negate(Box::new(atom))
        } else {
            Term::Scale(coefficient, Box::new(atom))
        });
    }
    if !constant.is_zero() {
        terms.push(Term::Int(constant));
    }
    match terms.len() {
        0 => Term::Int(Integer::zero()),
        1 => terms.remove(0),
        2 => {
            let second = terms.remove(1);
            let first = terms.remove(0);
            match (first, second) {
                (Term::Negate(negative), positive) if is_unit_coefficient(&positive) => {
                    difference(positive, *negative)
                }
                (positive, Term::Negate(negative)) if is_unit_coefficient(&positive) => {
                    difference(positive, *negative)
                }
                (first, second) => Term::Add(vec![first, second]),
            }
        }
        _ => Term::Add(terms),
    }
}

/// Whether `term` is an atom printed with coefficient +1 by the linear form.
fn is_unit_coefficient(term: &Term) -> bool {
    !matches!(
        term,
        Term::Int(_) | Term::Negate(_) | Term::Scale(_, _) | Term::Add(_) | Term::Subtract(_, _)
    )
}

/// `(ite c t e)` of normal `c`, `t` and `e`.
fn choice(condition: Term, then: Term, otherwise: Term) -> Term {
    if let Some(value) = condition.constant_value() {
        return if value { then } else { otherwise };
    }
    if then == otherwise {
        return then;
    }
    match (then.constant_value(), otherwise.constant_value()) {
        (Some(true), _) => junction(false, [condition, otherwise]),
        (Some(false), _) => junction(true, [negation(condition), otherwise]),
        (_, Some(true)) => junction(false, [negation(condition), then]),
        (_, Some(false)) => junction(true, [condition, then]),
        (None, None) => Term::If(Box::new(condition), Box::new(then), Box::new(otherwise)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn normal(term: Term) -> String {
        term.normal().text()
    }

    fn c(name: &str) -> Term {
        Term::constant(name)
    }

    /// Linear integer expressions collect exact coefficients without
    /// multiplying non-constant terms.
    #[trace("TC-900", "FR-315-AC-56", "FR-315-AC-57")]
    #[test]
    fn linear_integer_normal_form_collects_terms() {
        let x = c("x");
        let y = c("y");
        let left = Term::Scale(
            Integer::one(),
            Box::new(Term::Subtract(
                Box::new(Term::Add(vec![Term::int(1), x.clone()])),
                Box::new(y.clone()),
            )),
        );
        let right = Term::Subtract(
            Box::new(x.clone()),
            Box::new(Term::Add(vec![Term::int(-1), y.clone()])),
        );
        assert_eq!(normal(left.clone()), normal(right));
        assert_eq!(normal(left), "(+ x (- y) 1)");
        assert_eq!(normal(Term::Add(vec![y.clone(), x.clone()])), "(+ x y)");
        assert_eq!(normal(Term::Add(vec![x.clone(), y.clone()])), "(+ x y)");
        assert_eq!(normal(x.clone().plus(y.clone().negated())), "(- x y)");
        assert_eq!(
            normal(Term::Add(vec![x.clone().negated(), y.clone().negated()])),
            "(+ (- x) (- y))"
        );
        assert_eq!(
            normal(Term::Add(vec![
                Term::Scale(Integer::from(2_i64), Box::new(x.clone())),
                y.clone().negated(),
            ])),
            "(+ (* 2 x) (- y))"
        );
        assert_eq!(
            normal(Term::Add(vec![
                x.clone(),
                x.clone(),
                Term::Scale(Integer::from(2_i64), Box::new(x.clone()))
            ])),
            "(* 4 x)"
        );
        assert_eq!(
            normal(Term::Scale(
                Integer::from(2_i64),
                Box::new(Term::Scale(Integer::from(3_i64), Box::new(x)))
            )),
            "(* 6 x)"
        );
    }

    /// Double negation, negated constants and implication are rewritten.
    #[trace("TC-900", "FR-315-AC-56")]
    #[test]
    fn negations_and_implications_are_normal() {
        let not = |t: Term| Term::Not(Box::new(t));
        assert_eq!(normal(not(not(c("a")))), "a");
        assert_eq!(normal(not(Term::literal(true))), "false");
        assert_eq!(normal(not(not(not(c("a"))))), "(not a)");
        assert_eq!(
            normal(Term::Implies(Box::new(c("a")), Box::new(c("b")))),
            "(or (not a) b)"
        );
        assert_eq!(
            normal(Term::Or(vec![not(Term::literal(true)), not(c("r")),])),
            "(not r)"
        );
    }

    /// Conjunctions and disjunctions are flat, without constants or
    /// repeated operands.
    #[trace("TC-900", "FR-315-AC-56")]
    #[test]
    fn junctions_are_flat_and_free_of_constants() {
        let nested = Term::And(vec![
            Term::And(vec![c("a"), Term::And(vec![c("b"), c("a")])]),
            Term::literal(true),
            Term::Or(vec![c("x"), Term::Or(vec![c("y"), c("x")])]),
        ]);
        assert_eq!(normal(nested), "(and a b (or x y))");
        assert_eq!(
            normal(Term::And(vec![c("a"), Term::literal(false)])),
            "false"
        );
        assert_eq!(normal(Term::Or(vec![c("a"), Term::literal(true)])), "true");
        assert_eq!(normal(Term::And(vec![c("a"), c("a")])), "a");
    }

    /// Equalities with Boolean constants and comparisons of literals fold;
    /// a literal summand moves across a relation with a literal.
    #[trace("TC-900", "FR-315-AC-56")]
    #[test]
    fn relations_fold_constants() {
        assert_eq!(normal(c("b").equal(Term::literal(false))), "(not b)");
        assert_eq!(normal(c("b").equal(Term::literal(true))), "b");
        assert_eq!(normal(Term::int(1).equal(Term::int(1))), "true");
        assert_eq!(
            normal(Term::int(0).compare(Comparison::LessOrEqual, c("x").plus(Term::int(1)))),
            "(<= (- 1) x)"
        );
        assert_eq!(
            normal(
                c("x")
                    .plus(Term::int(1))
                    .compare(Comparison::LessOrEqual, Term::int(2))
            ),
            "(<= x 1)"
        );
        assert_eq!(
            normal(c("y").equal(c("x").plus(Term::int(1)))),
            "(= y (+ x 1))"
        );
        assert_eq!(normal(c("x").compare(Comparison::Less, c("x"))), "false");
        assert_eq!(
            normal(c("x").plus(Term::int(1)).equal(Term::int(3))),
            "(= x 2)"
        );
        assert_eq!(
            normal(Term::int(3).equal(c("x").plus(Term::int(1)))),
            "(= 2 x)"
        );
        assert_eq!(
            normal(
                c("x")
                    .plus(Term::int(2))
                    .compare(Comparison::Less, Term::int(5))
            ),
            "(< x 3)"
        );
        assert_eq!(normal(Term::literal(false).equal(c("b"))), "(not b)");
        assert_eq!(normal(Term::literal(true).equal(c("b"))), "b");
    }

    /// Arithmetic over literals folds.
    #[trace("TC-900", "FR-315-AC-56")]
    #[test]
    fn arithmetic_over_literals_folds() {
        let sum = Term::Add(vec![
            Term::int(1),
            c("x"),
            Term::Add(vec![Term::int(2), c("y")]),
        ]);
        assert_eq!(normal(sum), "(+ x y 3)");
        assert_eq!(normal(Term::int(3).minus(Term::int(5))), "(- 2)");
        assert_eq!(normal(c("x").negated().negated()), "x");
        assert_eq!(normal(c("x").minus(Term::int(0))), "x");
        assert_eq!(normal(c("x").minus(c("x"))), "0");
        assert_eq!(
            normal(Term::Scale(
                Integer::from(2_i64),
                Box::new(Term::Scale(Integer::from(3_i64), Box::new(c("x"))))
            )),
            "(* 6 x)"
        );
        assert_eq!(normal(Term::Scale(Integer::zero(), Box::new(c("x")))), "0");
        assert_eq!(normal(Term::int(0).minus(c("x"))), "(- x)");
        assert_eq!(normal(Term::int(0).minus(c("x").negated())), "x");
        assert_eq!(normal(c("x").plus(c("y").negated())), "(- x y)");
        assert_eq!(
            normal(Term::Add(vec![c("x"), c("y").negated(), Term::int(1)])),
            "(+ x (- y) 1)"
        );
        assert_eq!(normal(Term::Scale(Integer::one(), Box::new(c("x")))), "x");
        assert_eq!(
            normal(Term::Scale(Integer::from(2_i64), Box::new(Term::int(3)))),
            "6"
        );
    }

    /// `mod` of two literals is the Euclidean remainder, in `[0, |n|)`, for
    /// every sign of dividend and divisor; `mod` of a non-literal stays.
    #[trace("TC-900", "FR-315-AC-56")]
    #[test]
    fn remainders_of_literals_are_euclidean() {
        let modulo = |m: i64, n: i64| normal(Term::int(m).modulo(Integer::from(n)));
        assert_eq!(modulo(7, 3), "1");
        assert_eq!(modulo(-7, 3), "2");
        assert_eq!(modulo(7, -3), "1");
        assert_eq!(modulo(-7, -3), "2");
        assert_eq!(modulo(-6, 3), "0");
        assert_eq!(modulo(-1, -5), "4");
        assert_eq!(normal(c("x").modulo(Integer::from(3_i64))), "(mod x 3)");
        assert_eq!(
            normal(
                Term::int(2)
                    .plus(Term::int(3))
                    .modulo(Integer::from(-4_i64))
            ),
            "1"
        );
    }

    /// `ite` with a constant condition, equal branches or a constant branch
    /// is rewritten.
    #[trace("TC-900", "FR-315-AC-56")]
    #[test]
    fn choices_fold() {
        let ite = |a: Term, b: Term, e: Term| Term::If(Box::new(a), Box::new(b), Box::new(e));
        assert_eq!(normal(ite(Term::literal(true), c("a"), c("b"))), "a");
        assert_eq!(normal(ite(Term::literal(false), c("a"), c("b"))), "b");
        assert_eq!(normal(ite(c("p"), c("a"), c("a"))), "a");
        assert_eq!(
            normal(ite(c("p"), Term::literal(true), Term::literal(false))),
            "p"
        );
        assert_eq!(
            normal(ite(c("p"), Term::literal(false), c("e"))),
            "(and (not p) e)"
        );
        assert_eq!(normal(ite(c("p"), Term::literal(true), c("e"))), "(or p e)");
        assert_eq!(
            normal(ite(c("p"), c("t"), Term::literal(true))),
            "(or (not p) t)"
        );
        assert_eq!(
            normal(ite(c("p"), c("t"), Term::literal(false))),
            "(and p t)"
        );
        assert_eq!(normal(ite(c("p"), c("x"), c("y"))), "(ite p x y)");
    }
}
