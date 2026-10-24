// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact real algebraic numbers: a rational, or the one root of a
//! square-free rational polynomial in an isolating open interval with
//! rational ends. Comparison and sign evaluation refine the interval by
//! bisection and decide equality by a common factor, so no floating-point
//! step is taken (FR-450 "Exact replay").

use std::cmp::Ordering;
use std::fmt;

use quire_exact::Rational;

use super::poly::{midpoint, Poly};

/// An exact real number.
#[derive(Clone, Debug)]
pub enum ExactReal {
    /// A rational number.
    Rational(Rational),
    /// An irrational algebraic number.
    Root(Root),
}

/// The one root of a square-free polynomial in the open interval
/// `(lower, upper)`, where the polynomial changes sign.
#[derive(Clone, Debug)]
pub struct Root {
    poly: Poly,
    lower: Rational,
    upper: Rational,
}

impl Root {
    /// The real number the one root of `poly` in `(lower, upper)` is.
    /// `poly` is square-free, has exactly one root there and is nonzero at
    /// both ends.
    pub(super) fn real(poly: Poly, lower: Rational, upper: Rational) -> ExactReal {
        ExactReal::Root(Self { poly, lower, upper })
    }

    /// The polynomial's coefficients, lowest degree first.
    pub fn coefficients(&self) -> &[Rational] {
        self.poly.coefficients()
    }

    /// The isolating interval's lower end, exclusive.
    pub fn lower(&self) -> &Rational {
        &self.lower
    }

    /// The isolating interval's upper end, exclusive.
    pub fn upper(&self) -> &Rational {
        &self.upper
    }

    /// Halve the isolating interval; the root itself when the midpoint is
    /// it.
    fn refine(&mut self) -> Option<Rational> {
        let middle = midpoint(&self.lower, &self.upper);
        let at = self.poly.sign_at(&middle);
        if at == Ordering::Equal {
            return Some(middle);
        }
        if self.poly.sign_at(&self.lower) == at {
            self.lower = middle;
        } else {
            self.upper = middle;
        }
        None
    }

    /// Whether `poly` has a root in this interval's open span.
    fn shares_root(&self, other: &Poly, lower: &Rational, upper: &Rational) -> bool {
        let common = self.poly.gcd(other);
        common.degree().unwrap_or(0) > 0 && lower < upper && common.roots_between(lower, upper) > 0
    }
}

impl ExactReal {
    /// `self + by`.
    pub(crate) fn shifted(&self, by: &Rational) -> Self {
        match self {
            Self::Rational(q) => Self::Rational(q.add(by)),
            Self::Root(root) => Self::Root(Root {
                poly: root.poly.shifted(by),
                lower: root.lower.add(by),
                upper: root.upper.add(by),
            }),
        }
    }

    /// The rational value, when `self` is rational.
    pub fn as_rational(&self) -> Option<&Rational> {
        match self {
            Self::Rational(q) => Some(q),
            Self::Root(_) => None,
        }
    }

    /// The sign of `poly` at `self`.
    pub(crate) fn sign_of(&self, poly: &Poly) -> Ordering {
        match self {
            Self::Rational(q) => poly.sign_at(q),
            Self::Root(root) => {
                if poly.is_zero() || root.shares_root(poly, &root.lower, &root.upper) {
                    return Ordering::Equal;
                }
                let mut root = root.clone();
                let square_free = poly.square_free();
                loop {
                    if square_free.roots_between(&root.lower, &root.upper) == 0 {
                        return poly.sign_at(&midpoint(&root.lower, &root.upper));
                    }
                    if let Some(exact) = root.refine() {
                        return poly.sign_at(&exact);
                    }
                }
            }
        }
    }

    /// A rational strictly between `self` and `above`, `self < above`.
    pub(crate) fn between(&self, above: &Self) -> Rational {
        let mut low = self.clone();
        let mut high = above.clone();
        loop {
            let low_bound = match &low {
                Self::Rational(q) => q.clone(),
                Self::Root(root) => root.upper.clone(),
            };
            let high_bound = match &high {
                Self::Rational(q) => q.clone(),
                Self::Root(root) => root.lower.clone(),
            };
            if low_bound < high_bound {
                return match (&low, &high) {
                    (Self::Root(_), _) => low_bound,
                    (_, Self::Root(_)) => high_bound,
                    (Self::Rational(_), Self::Rational(_)) => midpoint(&low_bound, &high_bound),
                };
            }
            for value in [&mut low, &mut high] {
                if let Self::Root(root) = value {
                    if let Some(exact) = root.refine() {
                        *value = Self::Rational(exact);
                    }
                }
            }
        }
    }
}

fn compare_root_rational(root: &Root, q: &Rational) -> Ordering {
    if q <= &root.lower {
        return Ordering::Greater;
    }
    if q >= &root.upper {
        return Ordering::Less;
    }
    let at = root.poly.sign_at(q);
    if at == Ordering::Equal {
        Ordering::Equal
    } else if at == root.poly.sign_at(&root.lower) {
        // The sign has not changed by `q`, so the root lies above it.
        Ordering::Greater
    } else {
        Ordering::Less
    }
}

impl Ord for ExactReal {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Rational(a), Self::Rational(b)) => a.cmp(b),
            (Self::Root(a), Self::Rational(b)) => compare_root_rational(a, b),
            (Self::Rational(a), Self::Root(b)) => compare_root_rational(b, a).reverse(),
            (Self::Root(a), Self::Root(b)) => {
                let (mut a, mut b) = (a.clone(), b.clone());
                loop {
                    if a.upper <= b.lower {
                        return Ordering::Less;
                    }
                    if b.upper <= a.lower {
                        return Ordering::Greater;
                    }
                    let lower = a.lower.clone().max(b.lower.clone());
                    let upper = a.upper.clone().min(b.upper.clone());
                    if a.shares_root(&b.poly, &lower, &upper) {
                        return Ordering::Equal;
                    }
                    match (a.refine(), b.refine()) {
                        (Some(x), Some(y)) => return x.cmp(&y),
                        (Some(x), None) => return compare_root_rational(&b, &x).reverse(),
                        (None, Some(y)) => return compare_root_rational(&a, &y),
                        (None, None) => {}
                    }
                }
            }
        }
    }
}

impl PartialOrd for ExactReal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ExactReal {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for ExactReal {}

impl From<Rational> for ExactReal {
    fn from(value: Rational) -> Self {
        Self::Rational(value)
    }
}

impl fmt::Display for ExactReal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rational(q) => write!(f, "{q}"),
            Self::Root(root) => write!(
                f,
                "the root of {:?} in ({}, {})",
                root.poly.coefficients(),
                root.lower,
                root.upper
            ),
        }
    }
}
