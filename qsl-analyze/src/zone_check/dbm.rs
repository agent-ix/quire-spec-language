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

/// Shift a finite DBM bound by a constant without changing its strictness.
fn shift(bound: &Bound, amount: &BigInt) -> Bound {
    match bound {
        Bound::Finite { value, strict } => Bound::Finite {
            value: value + amount,
            strict: *strict,
        },
        Bound::Infinite => Bound::Infinite,
    }
}

/// Whether the finite value satisfies a DBM bound.
fn contains(value: &BigInt, bound: &Bound) -> bool {
    match bound {
        Bound::Finite {
            value: upper,
            strict,
        } => {
            if *strict {
                value < upper
            } else {
                value <= upper
            }
        }
        Bound::Infinite => true,
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
    /// A renaming mapped two clocks to one, so it is no permutation.
    #[error("the renaming maps clocks {first} and {second} to one clock")]
    NotAPermutation {
        /// The lower clock mapped twice.
        first: usize,
        /// The higher.
        second: usize,
    },
    /// A carry or reset assignment to the reference clock.
    #[error("the reference clock cannot be reassigned")]
    ReassignReferenceClock,
    /// A clock is assigned more than once by one simultaneous update.
    #[error("clock {clock} is assigned more than once")]
    DuplicateAssignment {
        /// The clock assigned more than once.
        clock: usize,
    },
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

    /// The zone with clock `i` renamed to `image[i - 1] + 1` for each `i`
    /// that `image` covers, every other clock (the reference clock first)
    /// kept: QSpec FR-430's move of an object's clock indices to those of
    /// the object a permutation maps it to. A permutation of the clocks
    /// keeps the canonical form.
    ///
    /// # Errors
    ///
    /// [`DbmError::ClockOutOfRange`] when an image is outside the matrix,
    /// and [`DbmError::NotAPermutation`] when two clocks have one image, so
    /// that no row is overwritten.
    pub fn renamed(&self, image: &[usize]) -> Result<Self, DbmError> {
        let to = |clock: usize| match clock.checked_sub(1).and_then(|place| image.get(place)) {
            Some(place) => place + 1,
            None => clock,
        };
        let mut from = vec![None; self.dim];
        for clock in 0..self.dim {
            let target = to(clock);
            self.in_range(target)?;
            if let Some(first) = from[target].replace(clock) {
                return Err(DbmError::NotAPermutation {
                    first,
                    second: clock,
                });
            }
        }
        let mut renamed = self.clone();
        for i in 0..self.dim {
            for j in 0..self.dim {
                renamed.set(to(i), to(j), self.at(i, j).clone());
            }
        }
        Ok(renamed)
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

    /// Apply simultaneous carried-clock copies and constant resets.
    /// `carries` contains `(target, source)` DBM clock indices; `resets`
    /// contains `(target, value)` pairs. Unassigned clocks retain their
    /// value. All sources are read from the pre-update zone.
    pub fn assign_clocks(
        &mut self,
        carries: &[(usize, usize)],
        resets: &[(usize, BigInt)],
    ) -> Result<(), DbmError> {
        #[derive(Clone)]
        enum Value {
            Clock(usize),
            Constant(BigInt),
        }

        let mut values: Vec<Value> = (0..self.dim).map(Value::Clock).collect();
        let mut assigned = vec![false; self.dim];
        for &(target, source) in carries {
            self.in_range(target)?;
            self.in_range(source)?;
            if target == 0 {
                return Err(DbmError::ReassignReferenceClock);
            }
            if assigned[target] {
                return Err(DbmError::DuplicateAssignment { clock: target });
            }
            assigned[target] = true;
            values[target] = Value::Clock(source);
        }
        for (target, value) in resets {
            self.in_range(*target)?;
            if *target == 0 {
                return Err(DbmError::ReassignReferenceClock);
            }
            if assigned[*target] {
                return Err(DbmError::DuplicateAssignment { clock: *target });
            }
            assigned[*target] = true;
            values[*target] = Value::Constant(value.clone());
        }
        if self.is_empty() {
            return Ok(());
        }

        let before = self.clone();
        for i in 0..self.dim {
            for j in 0..self.dim {
                let bound = match (&values[i], &values[j]) {
                    (Value::Clock(a), Value::Clock(b)) => before.at(*a, *b).clone(),
                    (Value::Clock(a), Value::Constant(b)) => before.at(*a, 0).sum(&Bound::le(-b)),
                    (Value::Constant(a), Value::Clock(b)) => {
                        Bound::le(a.clone()).sum(before.at(0, *b))
                    }
                    (Value::Constant(a), Value::Constant(b)) => Bound::le(a - b),
                };
                self.set(i, j, bound);
            }
        }
        self.close();
        Ok(())
    }

    /// The preimage of this zone under simultaneous carried-clock copies and
    /// constant resets. Assignment sources are read from the pre-state;
    /// unassigned clocks retain their value.
    pub fn preimage_clocks(
        &self,
        carries: &[(usize, usize)],
        resets: &[(usize, BigInt)],
    ) -> Result<Self, DbmError> {
        #[derive(Clone)]
        enum Value {
            Clock(usize),
            Constant(BigInt),
        }

        let mut values: Vec<Value> = (0..self.dim).map(Value::Clock).collect();
        let mut assigned = vec![false; self.dim];
        for &(target, source) in carries {
            self.in_range(target)?;
            self.in_range(source)?;
            if target == 0 {
                return Err(DbmError::ReassignReferenceClock);
            }
            if assigned[target] {
                return Err(DbmError::DuplicateAssignment { clock: target });
            }
            assigned[target] = true;
            values[target] = Value::Clock(source);
        }
        for (target, value) in resets {
            self.in_range(*target)?;
            if *target == 0 {
                return Err(DbmError::ReassignReferenceClock);
            }
            if assigned[*target] {
                return Err(DbmError::DuplicateAssignment { clock: *target });
            }
            assigned[*target] = true;
            values[*target] = Value::Constant(value.clone());
        }
        if self.is_empty() {
            return Ok(self.clone());
        }

        let mut preimage = Self::universe(self.dim);
        for i in 0..self.dim {
            for j in 0..self.dim {
                let bound = self.at(i, j);
                if matches!(bound, Bound::Infinite) {
                    continue;
                }
                match (&values[i], &values[j]) {
                    (Value::Clock(a), Value::Clock(b)) => {
                        preimage.constrain(*a, *b, bound.clone())?;
                    }
                    (Value::Clock(a), Value::Constant(b)) => {
                        preimage.constrain(*a, 0, shift(bound, b))?;
                    }
                    (Value::Constant(a), Value::Clock(b)) => {
                        preimage.constrain(0, *b, shift(bound, &-a))?;
                    }
                    (Value::Constant(a), Value::Constant(b)) => {
                        if !contains(&(a - b), bound) {
                            preimage.set_empty();
                            return Ok(preimage);
                        }
                    }
                }
            }
        }
        Ok(preimage)
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

impl Dbm {
    /// The zone of every valuation: each clock at least 0, nothing else.
    pub fn universe(dim: usize) -> Self {
        let mut bounds = vec![Bound::Infinite; dim * dim];
        for k in 0..dim {
            bounds[k * dim + k] = Bound::zero_le();
            bounds[k] = Bound::zero_le();
        }
        Self { dim, bounds }
    }

    /// This zone over `dim >= self.dim()` clocks, the added clocks
    /// unconstrained but for `x >= 0`.
    #[must_use]
    pub fn widened(&self, dim: usize) -> Self {
        let mut wide = Self::universe(dim.max(self.dim));
        for i in 0..self.dim {
            for j in 0..self.dim {
                wide.bounds[i * wide.dim + j] = self.at(i, j).clone();
            }
        }
        wide.close();
        if self.is_empty() {
            wide.set_empty();
        }
        wide
    }

    /// Every finite bound multiplied by `factor > 0`: the same zone in units
    /// `factor` times finer.
    pub fn scale_by(&mut self, factor: &BigInt) {
        for bound in &mut self.bounds {
            if let Bound::Finite { value, .. } = bound {
                *value *= factor;
            }
        }
    }

    /// This zone with one more clock, the last, equal to 0.
    #[must_use]
    pub fn with_clock_at_zero(&self) -> Self {
        let dim = self.dim + 1;
        let mut bounds = vec![Bound::zero_le(); dim * dim];
        for i in 0..dim {
            for j in 0..dim {
                // The new clock reads as the reference clock.
                let at = |k: usize| if k == self.dim { 0 } else { k };
                bounds[i * dim + j] = self.at(at(i), at(j)).clone();
            }
        }
        Self { dim, bounds }
    }

    /// This zone with its last clock projected away: the valuations of the
    /// other clocks some value of it extends into the zone. A closed matrix
    /// projects by dropping the clock's row and column.
    #[must_use]
    pub fn without_last_clock(&self) -> Self {
        let dim = self.dim.saturating_sub(1);
        let mut bounds = Vec::with_capacity(dim * dim);
        for i in 0..dim {
            for j in 0..dim {
                bounds.push(self.at(i, j).clone());
            }
        }
        let mut projected = Self { dim, bounds };
        if self.is_empty() {
            projected.set_empty();
        }
        projected
    }

    /// Free `clock`: drop every constraint on it but `clock >= 0`, the
    /// inverse image of a reset once the reset value is constrained.
    ///
    /// # Errors
    ///
    /// [`DbmError::ClockOutOfRange`] or [`DbmError::ResetReferenceClock`].
    pub fn free(&mut self, clock: usize) -> Result<(), DbmError> {
        self.in_range(clock)?;
        if clock == 0 {
            return Err(DbmError::ResetReferenceClock);
        }
        if self.is_empty() {
            return Ok(());
        }
        for k in (0..self.dim).filter(|&k| k != clock) {
            self.set(clock, k, Bound::Infinite);
            let lower = self.at(k, 0).clone();
            self.set(k, clock, lower);
        }
        self.set(0, clock, Bound::zero_le());
        Ok(())
    }

    /// The time predecessors: every valuation from which a delay reaches
    /// the zone. Each clock's lower bound against the reference clock is
    /// relaxed to the tightest one other clocks still imply.
    pub fn down(&mut self) {
        if self.is_empty() {
            return;
        }
        for i in 1..self.dim {
            let mut lower = Bound::zero_le();
            for j in 1..self.dim {
                if *self.at(j, i) < lower {
                    lower = self.at(j, i).clone();
                }
            }
            self.set(0, i, lower);
        }
    }

    /// Whether this zone is covered by `target` under the aLU simulation:
    /// `self ⊆ aLU(target)` (Herbreteau, Srivathsan and Walukiewicz). `lower`
    /// and `upper` give each clock's `L` and `U`, `None` for `-∞`. It fails
    /// exactly when some clocks `x != y` have `self[0][x] >= (-U_x, <=)`,
    /// `target[y][x] < self[y][x]` and `target[y][x] + (-L_y, <) <
    /// self[0][x]`.
    ///
    /// # Errors
    ///
    /// [`DbmError::DimensionMismatch`] when the matrices or the bounds do not
    /// have one clock count.
    pub fn alu_covered_by(
        &self,
        target: &Self,
        lower: &[Option<BigInt>],
        upper: &[Option<BigInt>],
    ) -> Result<bool, DbmError> {
        let clocks = self.dim.saturating_sub(1);
        if self.dim != target.dim || lower.len() != clocks || upper.len() != clocks {
            return Err(DbmError::DimensionMismatch {
                left: self.dim,
                right: target.dim,
            });
        }
        if self.is_empty() {
            return Ok(true);
        }
        if target.is_empty() {
            return Ok(false);
        }
        // `(-c, ≺)` for clock `k`'s bound `c`, `∞` for `-∞`; the reference
        // clock's bound is 0.
        let negated = |bounds: &[Option<BigInt>], k: usize, strict: bool| -> Bound {
            let value = if k == 0 {
                Some(BigInt::zero())
            } else {
                bounds[k - 1].as_ref().map(|c| -c)
            };
            match value {
                Some(value) => Bound::Finite { value, strict },
                None => Bound::Infinite,
            }
        };
        for x in 0..self.dim {
            if *self.at(0, x) < negated(upper, x, false) {
                continue;
            }
            for y in (0..self.dim).filter(|&y| y != x) {
                if *target.at(y, x) < *self.at(y, x)
                    && target.at(y, x).sum(&negated(lower, y, true)) < *self.at(0, x)
                {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
