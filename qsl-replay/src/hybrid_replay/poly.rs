// SPDX-License-Identifier: AGPL-3.0-or-later
//! Univariate polynomials and rational functions over the exact rationals,
//! Sturm root counting and exact real root isolation (FR-450 "Exact
//! replay": an atom along a dwell is a univariate rational function of time
//! whose sign on an interval QSL decides with no floating-point step).

use std::cmp::Ordering;

use quire_exact::Rational;

use super::real::{ExactReal, Root};
use super::trace::zero;

fn one() -> Rational {
    Rational::from_integer(1_i64.into())
}

fn two() -> Rational {
    Rational::from_integer(2_i64.into())
}

/// The midpoint of `a` and `b`.
pub(super) fn midpoint(a: &Rational, b: &Rational) -> Rational {
    a.add(b)
        .div(&two())
        .unwrap_or_else(|| unreachable!("2 is not zero"))
}

/// The sign of `value`.
pub(super) fn sign(value: &Rational) -> Ordering {
    if value.is_zero() {
        Ordering::Equal
    } else if value.numerator().is_negative() {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

/// A polynomial in one variable with exact rational coefficients, lowest
/// degree first, with no trailing zero coefficient.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Poly(Vec<Rational>);

impl Poly {
    /// The polynomial with these coefficients, lowest degree first.
    pub(crate) fn new(mut coefficients: Vec<Rational>) -> Self {
        while coefficients.last().is_some_and(Rational::is_zero) {
            coefficients.pop();
        }
        Self(coefficients)
    }

    pub(crate) fn zero() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn constant(value: Rational) -> Self {
        Self::new(vec![value])
    }

    /// The coefficients, lowest degree first.
    pub(crate) fn coefficients(&self) -> &[Rational] {
        &self.0
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    /// The degree; `None` for the zero polynomial.
    pub(crate) fn degree(&self) -> Option<usize> {
        self.0.len().checked_sub(1)
    }

    /// The constant value, when the degree is at most 0.
    pub(crate) fn as_constant(&self) -> Option<Rational> {
        match self.0.as_slice() {
            [] => Some(zero()),
            [value] => Some(value.clone()),
            _ => None,
        }
    }

    fn lead(&self) -> Option<&Rational> {
        self.0.last()
    }

    fn coefficient(&self, degree: usize) -> Rational {
        self.0.get(degree).cloned().unwrap_or_else(zero)
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        let len = self.0.len().max(other.0.len());
        Self::new(
            (0..len)
                .map(|i| self.coefficient(i).add(&other.coefficient(i)))
                .collect(),
        )
    }

    pub(crate) fn neg(&self) -> Self {
        Self(self.0.iter().map(Rational::neg).collect())
    }

    pub(crate) fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    pub(crate) fn scale(&self, factor: &Rational) -> Self {
        Self::new(self.0.iter().map(|c| c.mul(factor)).collect())
    }

    pub(crate) fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }
        let mut product = vec![zero(); self.0.len() + other.0.len() - 1];
        for (i, a) in self.0.iter().enumerate() {
            for (j, b) in other.0.iter().enumerate() {
                product[i + j] = product[i + j].add(&a.mul(b));
            }
        }
        Self::new(product)
    }

    /// The value at `x`, by Horner's rule.
    pub(crate) fn eval(&self, x: &Rational) -> Rational {
        self.0
            .iter()
            .rev()
            .fold(zero(), |value, coefficient| value.mul(x).add(coefficient))
    }

    /// The sign at `x`.
    pub(super) fn sign_at(&self, x: &Rational) -> Ordering {
        sign(&self.eval(x))
    }

    /// `self(inner(t))`.
    pub(crate) fn compose(&self, inner: &Self) -> Self {
        self.0
            .iter()
            .rev()
            .fold(Self::zero(), |value, coefficient| {
                value.mul(inner).add(&Self::constant(coefficient.clone()))
            })
    }

    /// `self(t - shift)`.
    pub(crate) fn shifted(&self, shift: &Rational) -> Self {
        self.compose(&Self::new(vec![shift.neg(), one()]))
    }

    pub(super) fn derivative(&self) -> Self {
        Self::new(
            self.0
                .iter()
                .enumerate()
                .skip(1)
                .map(|(degree, c)| c.mul(&Rational::from_integer(degree.into())))
                .collect(),
        )
    }

    /// The antiderivative with constant term 0.
    pub(crate) fn integral(&self) -> Self {
        let mut coefficients = vec![zero()];
        for (degree, c) in self.0.iter().enumerate() {
            let divisor = Rational::from_integer((degree + 1).into());
            coefficients.push(
                c.div(&divisor)
                    .unwrap_or_else(|| unreachable!("degree + 1 is positive")),
            );
        }
        Self::new(coefficients)
    }

    /// Quotient and remainder by a nonzero divisor.
    fn div_rem(&self, divisor: &Self) -> (Self, Self) {
        let (Some(lead), Some(degree)) = (divisor.lead(), divisor.degree()) else {
            return (Self::zero(), self.clone());
        };
        if self.0.len() <= degree {
            return (Self::zero(), self.clone());
        }
        let mut remainder = self.0.clone();
        let mut quotient = vec![zero(); self.0.len() - degree];
        while remainder.len() > degree {
            let shift = remainder.len() - 1 - degree;
            let factor = remainder[remainder.len() - 1]
                .div(lead)
                .unwrap_or_else(|| unreachable!("a leading coefficient is nonzero"));
            for (i, c) in divisor.0.iter().enumerate() {
                remainder[shift + i] = remainder[shift + i].sub(&factor.mul(c));
            }
            quotient[shift] = factor;
            remainder.pop();
            while remainder.last().is_some_and(Rational::is_zero) {
                remainder.pop();
            }
        }
        (Self::new(quotient), Self::new(remainder))
    }

    fn monic(&self) -> Self {
        match self.lead() {
            Some(lead) => Self(
                self.0
                    .iter()
                    .map(|c| {
                        c.div(lead)
                            .unwrap_or_else(|| unreachable!("a leading coefficient is nonzero"))
                    })
                    .collect(),
            ),
            None => Self::zero(),
        }
    }

    /// The monic greatest common divisor.
    pub(super) fn gcd(&self, other: &Self) -> Self {
        let (mut a, mut b) = (self.clone(), other.clone());
        while !b.is_zero() {
            let (_, remainder) = a.div_rem(&b);
            a = b;
            b = remainder;
        }
        a.monic()
    }

    /// The exact quotient by a divisor of `self`.
    pub(super) fn exact_div(&self, divisor: &Self) -> Self {
        self.div_rem(divisor).0
    }

    /// The square-free part: the product of the distinct irreducible
    /// factors, monic.
    pub(super) fn square_free(&self) -> Self {
        if self.degree().unwrap_or(0) == 0 {
            return self.monic();
        }
        let common = self.gcd(&self.derivative());
        self.exact_div(&common).monic()
    }

    /// `self` with the factor `t - root` removed, `root` being a root.
    fn without_root(&self, root: &Rational) -> Self {
        self.exact_div(&Self::new(vec![root.neg(), one()]))
    }

    fn sturm_sequence(&self) -> Vec<Self> {
        let mut sequence = vec![self.clone(), self.derivative()];
        loop {
            let len = sequence.len();
            let (_, remainder) = sequence[len - 2].div_rem(&sequence[len - 1]);
            if remainder.is_zero() {
                break;
            }
            sequence.push(remainder.neg());
        }
        sequence
    }

    fn sign_changes(sequence: &[Self], x: &Rational) -> usize {
        let signs: Vec<Ordering> = sequence
            .iter()
            .map(|p| p.sign_at(x))
            .filter(|s| *s != Ordering::Equal)
            .collect();
        signs.windows(2).filter(|w| w[0] != w[1]).count()
    }

    /// The number of distinct real roots in the open interval `(a, b)`, for
    /// a square-free `self` and `a < b`.
    pub(super) fn roots_between(&self, a: &Rational, b: &Rational) -> usize {
        if self.degree().unwrap_or(0) == 0 {
            return 0;
        }
        let mut p = self.clone();
        if p.eval(a).is_zero() {
            p = p.without_root(a);
        }
        if p.eval(b).is_zero() {
            p = p.without_root(b);
        }
        if p.degree().unwrap_or(0) == 0 {
            return 0;
        }
        let sequence = p.sturm_sequence();
        Self::sign_changes(&sequence, a).saturating_sub(Self::sign_changes(&sequence, b))
    }

    /// Every distinct real root in the closed interval `[low, high]`,
    /// ascending; none for the zero polynomial, which the caller handles.
    pub(super) fn roots_in(&self, low: &Rational, high: &Rational) -> Vec<ExactReal> {
        if self.is_zero() || low > high {
            return Vec::new();
        }
        let square_free = self.square_free();
        let mut roots = Vec::new();
        if square_free.eval(low).is_zero() {
            roots.push(ExactReal::Rational(low.clone()));
        }
        if low < high {
            square_free.isolate(low, high, &mut roots);
            if square_free.eval(high).is_zero() {
                roots.push(ExactReal::Rational(high.clone()));
            }
        }
        roots
    }

    /// Push the roots in `(a, b)`, ascending.
    fn isolate(&self, a: &Rational, b: &Rational, roots: &mut Vec<ExactReal>) {
        match self.roots_between(a, b) {
            0 => {}
            1 => roots.push(self.single_root(a, b)),
            _ => {
                let m = midpoint(a, b);
                self.isolate(a, &m, roots);
                if self.eval(&m).is_zero() {
                    roots.push(ExactReal::Rational(m.clone()));
                }
                self.isolate(&m, b, roots);
            }
        }
    }

    /// The one root in `(a, b)`.
    fn single_root(&self, a: &Rational, b: &Rational) -> ExactReal {
        if self.degree() == Some(1) {
            let root = self.0[0]
                .neg()
                .div(&self.0[1])
                .unwrap_or_else(|| unreachable!("a degree-1 lead is nonzero"));
            return ExactReal::Rational(root);
        }
        let mut p = self.clone();
        if p.eval(a).is_zero() {
            p = p.without_root(a);
        }
        if p.eval(b).is_zero() {
            p = p.without_root(b);
        }
        Root::real(p, a.clone(), b.clone())
    }
}

/// A rational function `numerator / denominator`, the denominator nonzero.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RatFunc {
    pub(crate) numerator: Poly,
    pub(crate) denominator: Poly,
}

impl RatFunc {
    pub(crate) fn poly(numerator: Poly) -> Self {
        Self {
            numerator,
            denominator: Poly::constant(one()),
        }
    }

    pub(crate) fn constant(value: Rational) -> Self {
        Self::poly(Poly::constant(value))
    }

    fn reduced(numerator: Poly, denominator: Poly) -> Self {
        if numerator.is_zero() {
            return Self::poly(Poly::zero());
        }
        let common = numerator.gcd(&denominator);
        let (numerator, denominator) = if common.degree().unwrap_or(0) > 0 {
            (numerator.exact_div(&common), denominator.exact_div(&common))
        } else {
            (numerator, denominator)
        };
        // A monic denominator keeps one representation per function.
        let lead = denominator
            .lead()
            .cloned()
            .unwrap_or_else(|| unreachable!("a denominator is nonzero"));
        let inverse = one()
            .div(&lead)
            .unwrap_or_else(|| unreachable!("a leading coefficient is nonzero"));
        Self {
            numerator: numerator.scale(&inverse),
            denominator: denominator.scale(&inverse),
        }
    }

    pub(crate) fn add(&self, other: &Self) -> Self {
        Self::reduced(
            self.numerator
                .mul(&other.denominator)
                .add(&other.numerator.mul(&self.denominator)),
            self.denominator.mul(&other.denominator),
        )
    }

    pub(crate) fn neg(&self) -> Self {
        Self {
            numerator: self.numerator.neg(),
            denominator: self.denominator.clone(),
        }
    }

    pub(crate) fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    pub(crate) fn mul(&self, other: &Self) -> Self {
        Self::reduced(
            self.numerator.mul(&other.numerator),
            self.denominator.mul(&other.denominator),
        )
    }

    /// `self / other`, `other` not the zero function.
    pub(crate) fn div(&self, other: &Self) -> Self {
        Self::reduced(
            self.numerator.mul(&other.denominator),
            self.denominator.mul(&other.numerator),
        )
    }

    /// The polynomial whose sign is the sign of `self` wherever `self` is
    /// defined: `numerator * denominator`.
    pub(crate) fn sign_poly(&self) -> Poly {
        self.numerator.mul(&self.denominator)
    }
}
