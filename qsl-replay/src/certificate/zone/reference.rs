// SPDX-License-Identifier: AGPL-3.0-or-later
//! The reference the checker's zone code is tested and Kani-checked
//! against: zones as plain constraint lists over small machine integers,
//! read by point membership.
//!
//! Nothing here closes a matrix. Time elapse and reset are computed by
//! Fourier-Motzkin elimination of the delay or the reset clock, so a point
//! is in the reference result exactly when it satisfies the eliminated
//! constraints. Points are integer vectors read at a scale `s`: the point
//! `p` is the valuation `p / s`, with `p[0]` the reference clock at 0.

use super::{strict, Cell, Exact, LuBounds, Zone};

/// Reading a small exact integer back as a machine integer.
pub(super) trait Small: Exact {
    fn small(&self) -> i64;
}

#[cfg(test)]
impl Small for super::Integer {
    fn small(&self) -> i64 {
        i64::try_from(self.as_big()).expect("a small bound")
    }
}

#[cfg(kani)]
impl Small for i64 {
    fn small(&self) -> i64 {
        *self
    }
}

/// A plain bound on `x_i - x_j`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RefBound {
    /// `(c, <)`.
    Lt(i64),
    /// `(c, <=)`.
    Le(i64),
    /// `∞`.
    Inf,
}

impl RefBound {
    /// The sum of two bounds.
    pub(super) fn plus(self, other: Self) -> Self {
        match (self, other) {
            (Self::Inf, _) | (_, Self::Inf) => Self::Inf,
            (Self::Le(a), Self::Le(b)) => Self::Le(a + b),
            (Self::Lt(a) | Self::Le(a), Self::Lt(b) | Self::Le(b)) => Self::Lt(a + b),
        }
    }

    /// Whether `self` is strictly tighter than `other`.
    pub(super) fn tighter(self, other: Self) -> bool {
        match (self, other) {
            (Self::Inf, _) => false,
            (_, Self::Inf) => true,
            (Self::Lt(a), Self::Le(b)) => a <= b,
            (Self::Lt(a), Self::Lt(b)) | (Self::Le(a), Self::Le(b)) => a < b,
            (Self::Le(a), Self::Lt(b)) => a < b,
        }
    }

    /// Whether the scaled difference `diff` (the value `diff / s`)
    /// satisfies this bound.
    pub(super) fn holds(self, diff: i64, s: i64) -> bool {
        match self {
            Self::Inf => true,
            Self::Le(c) => diff <= c * s,
            Self::Lt(c) => diff < c * s,
        }
    }

    fn encode<W: Exact>(self) -> Cell<W> {
        match self {
            Self::Inf => None,
            Self::Lt(c) => Some(strict(&W::of(c))),
            Self::Le(c) => Some(super::non_strict(&W::of(c))),
        }
    }
}

/// A raw zone: `dim` and its row-major bounds, closed or not.
#[derive(Clone, Debug)]
pub(super) struct RawZone {
    pub(super) dim: usize,
    pub(super) bounds: Vec<RefBound>,
}

impl RawZone {
    fn at(&self, i: usize, j: usize) -> RefBound {
        self.bounds[i * self.dim + j]
    }

    /// The checker zone holding these bounds as they stand, not closed.
    pub(super) fn to_zone<W: Exact>(&self) -> Zone<W> {
        Zone {
            dim: self.dim,
            cells: self.bounds.iter().map(|b| b.encode()).collect(),
        }
    }

    /// Whether the point satisfies every bound.
    pub(super) fn contains(&self, p: &[i64], s: i64) -> bool {
        constraints_hold(self.constraints(), p, s)
    }

    fn constraints(&self) -> Vec<(usize, usize, RefBound)> {
        let mut out = Vec::new();
        for i in 0..self.dim {
            for j in 0..self.dim {
                out.push((i, j, self.at(i, j)));
            }
        }
        out
    }

    /// The constraints of `{p | ∃d >= 0. p - d ∈ Z}`, `d` eliminated.
    pub(super) fn up_constraints(&self) -> Vec<(usize, usize, RefBound)> {
        let mut out = Vec::new();
        for i in 1..self.dim {
            for j in 1..self.dim {
                out.push((i, j, self.at(i, j)));
                // d > p_i - c_i0 and d < p_j + c_0j.
                out.push((i, j, self.at(i, 0).plus(self.at(0, j))));
            }
            // 0 <= d < p_i + c_0i.
            out.push((0, i, self.at(0, i)));
        }
        if self.dim > 0 {
            out.push((0, 0, self.at(0, 0)));
        }
        out
    }

    /// The constraints of `{p | p_x = v ∧ ∃y. p[x := y] ∈ Z}`, `y`
    /// eliminated.
    pub(super) fn reset_constraints(&self, x: usize, v: i64) -> Vec<(usize, usize, RefBound)> {
        let mut out = Vec::new();
        for i in (0..self.dim).filter(|&i| i != x) {
            for j in (0..self.dim).filter(|&j| j != x) {
                out.push((i, j, self.at(i, j)));
                // y > p_i - c_ix and y < p_j + c_xj.
                out.push((i, j, self.at(i, x).plus(self.at(x, j))));
            }
        }
        out.push((x, x, self.at(x, x)));
        out.push((x, 0, RefBound::Le(v)));
        out.push((0, x, RefBound::Le(-v)));
        out
    }
}

/// Whether the point satisfies every listed constraint.
pub(super) fn constraints_hold(
    constraints: Vec<(usize, usize, RefBound)>,
    p: &[i64],
    s: i64,
) -> bool {
    constraints
        .into_iter()
        .all(|(i, j, bound)| bound.holds(p[i] - p[j], s))
}

/// Whether the point (scale `s`) is in the checker zone, read entry by
/// entry from its encoded matrix.
pub(super) fn zone_contains<W: Exact>(zone: &Zone<W>, p: &[i64], s: i64) -> bool {
    (0..zone.dim).all(|i| {
        (0..zone.dim).all(|j| match zone.cell(i, j) {
            None => true,
            // `(diff, <=)` is no looser than `(c * s, ≺)`, whose encoding
            // is `(e - n) * s + n` for `e = 2c + n`.
            Some(e) => {
                let n = if e.even() { W::of(0) } else { W::of(1) };
                let twice = e.minus(&n);
                let mut scaled = n;
                for _ in 0..s {
                    scaled = scaled.plus(&twice);
                }
                W::of(2 * (p[i] - p[j]) + 1) <= scaled
            }
        })
    })
}

/// The checker zone's bound on `x_i - x_j` as a plain bound.
pub(super) fn decode<W: Small>(zone: &Zone<W>, i: usize, j: usize) -> RefBound {
    match zone.bound(i, j) {
        Ok(Some((c, is_strict))) => {
            let c = c.small();
            if is_strict {
                RefBound::Lt(c)
            } else {
                RefBound::Le(c)
            }
        }
        _ => RefBound::Inf,
    }
}

/// Inclusion of canonical zones, entry by entry over plain bounds.
pub(super) fn includes(big: &RawZone, small: &RawZone) -> bool {
    let empty = |z: &RawZone| (0..z.dim).any(|k| z.at(k, k).tighter(RefBound::Le(0)));
    if empty(small) {
        return true;
    }
    if empty(big) {
        return false;
    }
    big.bounds
        .iter()
        .zip(&small.bounds)
        .all(|(b, s)| !b.tighter(*s))
}

/// The plain bounds of a checker zone.
pub(super) fn raw_of<W: Small>(zone: &Zone<W>) -> RawZone {
    let mut bounds = Vec::new();
    for i in 0..zone.dim {
        for j in 0..zone.dim {
            bounds.push(decode(zone, i, j));
        }
    }
    RawZone {
        dim: zone.dim,
        bounds,
    }
}

/// The aLU coverage test over plain bounds, written out from Herbreteau,
/// Srivathsan and Walukiewicz's characterisation: `Z ⊄ aLU(Z')` exactly
/// when some `x != y` have `Z[0][x] >= (-U_x, <=)`, `Z'[y][x] < Z[y][x]`
/// and `Z'[y][x] + (-L_y, <) < Z[0][x]`.
pub(super) fn alu_covered(
    z: &RawZone,
    target: &RawZone,
    lower: &[Option<i64>],
    upper: &[Option<i64>],
) -> bool {
    let empty = |z: &RawZone| (0..z.dim).any(|k| z.at(k, k).tighter(RefBound::Le(0)));
    if empty(z) {
        return true;
    }
    if empty(target) {
        return false;
    }
    let of = |bounds: &[Option<i64>], k: usize| if k == 0 { Some(0) } else { bounds[k - 1] };
    for x in 0..z.dim {
        let minus_u = of(upper, x).map_or(RefBound::Inf, |u| RefBound::Le(-u));
        if z.at(0, x).tighter(minus_u) {
            continue;
        }
        for y in 0..z.dim {
            if y == x {
                continue;
            }
            let minus_l = of(lower, y).map_or(RefBound::Inf, |l| RefBound::Lt(-l));
            if target.at(y, x).tighter(z.at(y, x))
                && target.at(y, x).plus(minus_l).tighter(z.at(0, x))
            {
                return false;
            }
        }
    }
    true
}

/// The aLU simulation by definition: `v ≼LU v'` when for every clock `c`,
/// `v'(c) < v(c)` implies `v'(c) > L_c` and `v'(c) > v(c)` implies
/// `v(c) > U_c`. Points are at scale `s`; `None` is `-∞`.
#[cfg(test)]
pub(super) fn simulated(
    v: &[i64],
    v_prime: &[i64],
    lower: &[Option<i64>],
    upper: &[Option<i64>],
    s: i64,
) -> bool {
    (1..v.len()).all(|c| {
        let below_ok = v_prime[c] >= v[c] || lower[c - 1].is_none_or(|l| v_prime[c] > l * s);
        let above_ok = v_prime[c] <= v[c] || upper[c - 1].is_none_or(|u| v[c] > u * s);
        below_ok && above_ok
    })
}

/// LU bounds as the checker reads them.
pub(super) fn lu_of<W: Exact>(lower: &[Option<i64>], upper: &[Option<i64>]) -> LuBounds<W> {
    LuBounds {
        lower: lower.iter().map(|b| b.map(W::of)).collect(),
        upper: upper.iter().map(|b| b.map(W::of)).collect(),
    }
}
