// SPDX-License-Identifier: AGPL-3.0-or-later
//! Zones as difference-bound matrices in exact integer arithmetic (ADR-026
//! EZ-2, FR-238; Bengtsson and Yi).
//!
//! A [`Dbm`] of dimension `dim` constrains the reference clock (index 0,
//! fixed at 0) and `dim - 1` clocks; entry `(i, j)` bounds `x_i - x_j`.
//! Bounds are [`BigInt`]s, so no constant is too large and no sum
//! overflows. Every operation takes a canonical matrix and leaves it
//! canonical, or empty; an empty matrix holds the one empty form, every
//! entry `(0, <)`.

use core::cmp::Ordering;

use num_bigint::BigInt;
use num_traits::Zero;

/// A bound on a clock difference: `(value, <)`, `(value, <=)` or `∞`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Bound {
    /// `x_i - x_j < value` when `strict`, `x_i - x_j <= value` otherwise.
    Finite {
        /// The constant.
        value: BigInt,
        /// `<` rather than `<=`.
        strict: bool,
    },
    /// No bound.
    Infinite,
}

impl Bound {
    /// `(value, <=)`.
    pub fn le(value: BigInt) -> Self {
        Self::Finite {
            value,
            strict: false,
        }
    }

    /// `(value, <)`.
    pub fn lt(value: BigInt) -> Self {
        Self::Finite {
            value,
            strict: true,
        }
    }

    fn zero_le() -> Self {
        Self::le(BigInt::zero())
    }

    /// The bound on `a + b` given bounds on `a` and `b`: values add, and
    /// the sum is strict when either is.
    fn sum(&self, other: &Self) -> Self {
        match (self, other) {
            (
                Self::Finite {
                    value: a,
                    strict: sa,
                },
                Self::Finite {
                    value: b,
                    strict: sb,
                },
            ) => Self::Finite {
                value: a + b,
                strict: *sa || *sb,
            },
            _ => Self::Infinite,
        }
    }
}

impl Ord for Bound {
    /// Tighter is smaller: `(c, <)` before `(c, <=)`, `∞` last.
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Infinite, Self::Infinite) => Ordering::Equal,
            (Self::Infinite, Self::Finite { .. }) => Ordering::Greater,
            (Self::Finite { .. }, Self::Infinite) => Ordering::Less,
            (
                Self::Finite {
                    value: a,
                    strict: sa,
                },
                Self::Finite {
                    value: b,
                    strict: sb,
                },
            ) => a.cmp(b).then_with(|| sb.cmp(sa)),
        }
    }
}

impl PartialOrd for Bound {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A DBM operation named an index or shape the matrix does not have.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DbmError {
    /// A clock index at or above the matrix's dimension.
    #[error("clock {clock} is outside a DBM of dimension {dim}")]
    ClockOutOfRange {
        /// The index named.
        clock: usize,
        /// The matrix's dimension.
        dim: usize,
    },
    /// A reset of the reference clock, which is fixed at 0.
    #[error("the reference clock cannot be reset")]
    ResetReferenceClock,
    /// Two matrices of different dimensions were compared.
    #[error("DBMs of dimension {left} and {right} cannot be compared")]
    DimensionMismatch {
        /// The receiver's dimension.
        left: usize,
        /// The argument's dimension.
        right: usize,
    },
}

/// A zone: a conjunction of `x_i - x_j ≺ c` over the reference clock and
/// `dim - 1` clocks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dbm {
    dim: usize,
    bounds: Vec<Bound>,
}

impl Dbm {
    /// The zone holding only the valuation with every clock 0; `dim`
    /// counts the reference clock.
    pub fn zero(dim: usize) -> Self {
        Self {
            dim,
            bounds: vec![Bound::zero_le(); dim * dim],
        }
    }

    /// The dimension: the clock count plus the reference clock.
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// The bound on `x_i - x_j`.
    pub fn bound(&self, i: usize, j: usize) -> Result<&Bound, DbmError> {
        self.in_range(i)?;
        self.in_range(j)?;
        Ok(self.at(i, j))
    }

    fn at(&self, i: usize, j: usize) -> &Bound {
        &self.bounds[i * self.dim + j]
    }

    fn set(&mut self, i: usize, j: usize, bound: Bound) {
        self.bounds[i * self.dim + j] = bound;
    }

    fn in_range(&self, clock: usize) -> Result<(), DbmError> {
        if clock < self.dim {
            Ok(())
        } else {
            Err(DbmError::ClockOutOfRange {
                clock,
                dim: self.dim,
            })
        }
    }

    fn set_empty(&mut self) {
        let empty = Bound::lt(BigInt::zero());
        for bound in &mut self.bounds {
            bound.clone_from(&empty);
        }
    }

    /// Lower `(i, j)` to the path through `k` where that is tighter.
    fn relax(&mut self, i: usize, k: usize, j: usize) {
        let through = self.at(i, k).sum(self.at(k, j));
        if through < *self.at(i, j) {
            self.set(i, j, through);
        }
    }

    /// Canonical form by shortest-path closure (Floyd-Warshall); a matrix
    /// with a negative cycle takes the empty form.
    pub fn close(&mut self) {
        let zero = Bound::zero_le();
        for k in 0..self.dim {
            if *self.at(k, k) > zero {
                self.set(k, k, zero.clone());
            }
        }
        for k in 0..self.dim {
            for i in 0..self.dim {
                for j in 0..self.dim {
                    self.relax(i, k, j);
                }
                if *self.at(i, i) < zero {
                    self.set_empty();
                    return;
                }
            }
        }
    }

    /// Whether the zone is empty: some diagonal entry is tighter than
    /// `(0, <=)`, a negative cycle in the closed matrix.
    pub fn is_empty(&self) -> bool {
        let zero = Bound::zero_le();
        (0..self.dim).any(|k| *self.at(k, k) < zero)
    }

    /// Intersect with `x_i - x_j ≺ b`.
    pub fn constrain(&mut self, i: usize, j: usize, b: Bound) -> Result<(), DbmError> {
        self.in_range(i)?;
        self.in_range(j)?;
        if self.is_empty() || b >= *self.at(i, j) {
            return Ok(());
        }
        if b.sum(self.at(j, i)) < Bound::zero_le() {
            self.set_empty();
            return Ok(());
        }
        self.set(i, j, b);
        // Only paths through the new edge can shorten, so closing over its
        // two endpoints restores canonical form.
        for k in [i, j] {
            for a in 0..self.dim {
                for c in 0..self.dim {
                    self.relax(a, k, c);
                }
            }
        }
        Ok(())
    }

    /// Reset `clock` to `value`.
    pub fn reset(&mut self, clock: usize, value: &BigInt) -> Result<(), DbmError> {
        self.in_range(clock)?;
        if clock == 0 {
            return Err(DbmError::ResetReferenceClock);
        }
        if self.is_empty() {
            return Ok(());
        }
        let to = Bound::le(value.clone());
        let from = Bound::le(-value);
        for k in (0..self.dim).filter(|&k| k != clock) {
            self.set(clock, k, to.sum(self.at(0, k)));
            self.set(k, clock, self.at(k, 0).sum(&from));
        }
        Ok(())
    }

    /// Let time elapse: every clock's upper bound against the reference
    /// clock becomes `∞`.
    pub fn up(&mut self) {
        if self.is_empty() {
            return;
        }
        for k in 1..self.dim {
            self.set(k, 0, Bound::Infinite);
        }
    }

    /// Whether this zone includes `other`.
    pub fn includes(&self, other: &Self) -> Result<bool, DbmError> {
        if self.dim != other.dim {
            return Err(DbmError::DimensionMismatch {
                left: self.dim,
                right: other.dim,
            });
        }
        if other.is_empty() {
            return Ok(true);
        }
        if self.is_empty() {
            return Ok(false);
        }
        Ok(self
            .bounds
            .iter()
            .zip(&other.bounds)
            .all(|(mine, theirs)| theirs <= mine))
    }
}

#[cfg(test)]
mod tests;
