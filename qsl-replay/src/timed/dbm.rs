// SPDX-License-Identifier: AGPL-3.0-or-later
//! The core's own difference-bound matrices (ADR-026 CF-3), written apart
//! from EN-6's so that FR-237's fresh exploration and FR-245's certificate
//! checker trust no code of the engine they check.
//!
//! A DBM over `n` clocks has `n + 1` rows and columns: index 0 is the
//! reference clock, fixed at 0, and clock `i` is index `i`. Cell `(i, j)`
//! bounds `x_i - x_j`. Bounds are integers: a caller scales every rational
//! constant by one common factor first. A DBM is kept canonical (every
//! cell the tightest bound its constraints imply), so emptiness is a
//! negative diagonal and inclusion is cell-wise comparison.

use std::cmp::Ordering;

use num_bigint::BigInt;

/// One cell: `< value`, `<= value`, or no bound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Bound {
    /// `x_i - x_j < value`, or `<=` when not `strict`.
    Finite {
        /// The constant.
        value: BigInt,
        /// Whether the bound is strict.
        strict: bool,
    },
    /// No bound.
    Infinite,
}

impl Bound {
    /// `<= value`.
    pub(crate) fn le(value: BigInt) -> Self {
        Self::Finite {
            value,
            strict: false,
        }
    }

    /// `< value`.
    pub(crate) fn lt(value: BigInt) -> Self {
        Self::Finite {
            value,
            strict: true,
        }
    }

    /// The bound of a sum of two differences.
    fn add(&self, other: &Self) -> Self {
        match (self, other) {
            (
                Self::Finite { value, strict },
                Self::Finite {
                    value: other,
                    strict: other_strict,
                },
            ) => Self::Finite {
                value: value + other,
                strict: *strict || *other_strict,
            },
            _ => Self::Infinite,
        }
    }

    /// Whether the bound admits a negative difference only: below `<= 0`.
    fn below_zero(&self) -> bool {
        *self < Self::le(BigInt::from(0))
    }
}

impl PartialOrd for Bound {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Bound {
    /// Tighter is smaller: `< c` is below `<= c`, and both are below every
    /// larger constant and below no bound.
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Infinite, Self::Infinite) => Ordering::Equal,
            (Self::Infinite, Self::Finite { .. }) => Ordering::Greater,
            (Self::Finite { .. }, Self::Infinite) => Ordering::Less,
            (
                Self::Finite { value, strict },
                Self::Finite {
                    value: other,
                    strict: other_strict,
                },
            ) => value.cmp(other).then_with(|| other_strict.cmp(strict)),
        }
    }
}

/// A canonical DBM. Its order is an arbitrary total one, for keying.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Dbm {
    /// Rows and columns: clocks plus the reference clock.
    size: usize,
    cells: Vec<Bound>,
}

impl Dbm {
    /// The one valuation `values`, each clock at its value.
    pub(crate) fn point(values: &[BigInt]) -> Self {
        let size = values.len() + 1;
        let at = |i: usize| {
            if i == 0 {
                BigInt::from(0)
            } else {
                values[i - 1].clone()
            }
        };
        let cells = (0..size)
            .flat_map(|i| (0..size).map(move |j| (i, j)))
            .map(|(i, j)| Bound::le(at(i) - at(j)))
            .collect();
        Self { size, cells }
    }

    /// The bound on `x_i - x_j`.
    pub(crate) fn get(&self, i: usize, j: usize) -> &Bound {
        &self.cells[i * self.size + j]
    }

    fn set(&mut self, i: usize, j: usize, bound: Bound) {
        let size = self.size;
        self.cells[i * size + j] = bound;
    }

    /// Whether no valuation satisfies the DBM.
    pub(crate) fn is_empty(&self) -> bool {
        (0..self.size).any(|i| self.get(i, i).below_zero())
    }

    /// Intersect with `x_i - x_j` bounded by `bound`, keeping the DBM
    /// canonical. An empty result keeps a negative diagonal.
    pub(crate) fn constrain(&mut self, i: usize, j: usize, bound: Bound) {
        if bound >= *self.get(i, j) {
            return;
        }
        if bound.add(self.get(j, i)).below_zero() {
            self.set(0, 0, Bound::lt(BigInt::from(0)));
            return;
        }
        self.set(i, j, bound);
        // Tighten every path through the new edge `i -> j`.
        let size = self.size;
        for a in 0..size {
            for b in 0..size {
                let through = self.get(a, i).add(self.get(i, j)).add(self.get(j, b));
                if through < *self.get(a, b) {
                    self.set(a, b, through);
                }
            }
        }
    }

    /// Let time pass: drop every upper bound of a clock.
    pub(crate) fn up(&mut self) {
        for i in 1..self.size {
            self.set(i, 0, Bound::Infinite);
        }
    }

    /// Set clock `clock` to `value`.
    pub(crate) fn reset(&mut self, clock: usize, value: &BigInt) {
        for j in 0..self.size {
            if j == clock {
                continue;
            }
            let to = Bound::le(value.clone()).add(self.get(0, j));
            let from = self.get(j, 0).add(&Bound::le(-value));
            self.set(clock, j, to);
            self.set(j, clock, from);
        }
        self.set(clock, clock, Bound::le(BigInt::from(0)));
    }

    /// The classical `Extra_M` extrapolation (Behrmann, Bouyer, Larsen and
    /// Pelánek): each clock `i` compared with no constant above `max[i]`
    /// (`None` for a clock never compared) loses the bounds that only
    /// distinguish valuations above it. It keeps the DBM canonical and
    /// makes the zones a run reaches finitely many.
    pub(crate) fn extrapolate(&mut self, max: &[Option<BigInt>]) {
        let ceiling = |i: usize| -> Option<&BigInt> {
            if i == 0 {
                None
            } else {
                max.get(i - 1).and_then(Option::as_ref)
            }
        };
        let size = self.size;
        let zero = BigInt::from(0);
        for i in 0..size {
            for j in 0..size {
                if i == j {
                    continue;
                }
                let bound = self.get(i, j).clone();
                let Bound::Finite { value, .. } = &bound else {
                    continue;
                };
                // Above `M_i` the difference is unbounded; below `-M_j` it
                // is bounded by `< -M_j`.
                let above = match ceiling(i) {
                    Some(m) => value > m,
                    None => i != 0,
                };
                let below = match ceiling(j) {
                    Some(m) => *value < -m,
                    None => j != 0 && *value < zero,
                };
                if above {
                    self.set(i, j, Bound::Infinite);
                } else if below {
                    let m = ceiling(j).cloned().unwrap_or_default();
                    self.set(i, j, Bound::lt(-m));
                }
            }
        }
        self.close();
    }

    /// Floyd–Warshall: every cell the tightest bound of a path.
    fn close(&mut self) {
        let size = self.size;
        for k in 0..size {
            for a in 0..size {
                for b in 0..size {
                    let through = self.get(a, k).add(self.get(k, b));
                    if through < *self.get(a, b) {
                        self.set(a, b, through);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Bound, Dbm};
    use num_bigint::BigInt;

    fn n(value: i64) -> BigInt {
        BigInt::from(value)
    }

    /// `x` and `y` from 0, time passing, constrained and reset: the
    /// difference stays, emptiness shows as a negative diagonal, and a
    /// reset fixes one clock.
    #[test]
    fn elapse_constrain_and_reset_keep_a_canonical_zone() {
        let mut zone = Dbm::point(&[n(0), n(0)]);
        zone.up();
        // x <= 3.
        zone.constrain(1, 0, Bound::le(n(3)));
        assert_eq!(zone.get(2, 0), &Bound::le(n(3)), "y = x");
        assert_eq!(zone.get(1, 2), &Bound::le(n(0)));
        zone.reset(1, &n(0));
        // y - x is now y, in [0, 3].
        assert_eq!(zone.get(2, 1), &Bound::le(n(3)));
        assert_eq!(zone.get(1, 0), &Bound::le(n(0)));
        let mut empty = zone.clone();
        // y > 3 meets y <= 3.
        empty.constrain(0, 2, Bound::lt(n(-3)));
        assert!(empty.is_empty());
        assert!(!zone.is_empty());
    }

    /// `Extra_M` drops what only separates values above a clock's constant,
    /// so a clock that grows without bound reaches finitely many zones.
    #[test]
    fn extrapolation_bounds_the_zones_of_a_growing_clock() {
        let mut seen: Vec<Dbm> = Vec::new();
        for value in 0..10 {
            let mut zone = Dbm::point(&[n(value)]);
            zone.extrapolate(&[Some(n(2))]);
            if !seen.contains(&zone) {
                seen.push(zone);
            }
        }
        // 0, 1, 2, and every value above 2.
        assert_eq!(seen.len(), 4);
        let above = seen.last().expect("a zone");
        assert_eq!(above.get(1, 0), &Bound::Infinite);
        assert_eq!(above.get(0, 1), &Bound::lt(n(-2)));
    }
}
