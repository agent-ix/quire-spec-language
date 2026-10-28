// SPDX-License-Identifier: AGPL-3.0-or-later
//! The zone certificate checker's own difference-bound matrices (ADR-026
//! CF-3, CF-6; FR-245).
//!
//! The checker recomputes every successor of every certificate node with
//! this code, never with the zone engine's DBM in `qsl-analyze`. The two
//! share no implementation: this module holds its own bound encoding, its
//! own closure, its own operations and the aLU coverage test, so a fault in
//! the engine's DBM cannot be masked by the same fault here. It is kept
//! small on purpose; its Kani harnesses (`proofs`) check it for overflow
//! freedom and for agreement with a reference implementation.
//!
//! # Encoding
//!
//! A bound on `x_i - x_j` is held as one exact integer: `(c, <)` is `2c`
//! and `(c, <=)` is `2c + 1`, so a tighter bound is a smaller integer and
//! `None` is `∞`. A matrix of dimension `dim` covers the reference clock
//! (index 0, fixed at 0) and `dim - 1` clocks.
//!
//! The checker holds bounds in [`quire_exact::Integer`], so no bound is too
//! large. The matrix code is written once over [`Exact`], and the Kani
//! harnesses run that same code over a machine integer, where every
//! addition is checked for overflow.

use quire_exact::{Integer, NonZeroInteger};

/// Seals [`Exact`]: only this module chooses the integers a zone may hold.
mod sealed {
    /// The private supertrait of [`super::Exact`].
    pub trait Sealed {}

    impl Sealed for quire_exact::Integer {}

    #[cfg(kani)]
    impl Sealed for i64 {}
}

/// The exact integers a zone holds its encoded bounds in: [`Integer`] for
/// the checker, a machine integer only in the Kani harnesses. Sealed: no
/// other crate or module can add one.
pub trait Exact: sealed::Sealed + Clone + Ord + core::fmt::Debug {
    /// The integer `value`.
    fn of(value: i64) -> Self;
    /// `self + other`.
    fn plus(&self, other: &Self) -> Self;
    /// `self - other`.
    fn minus(&self, other: &Self) -> Self;
    /// `-self`.
    fn negated(&self) -> Self;
    /// Whether `self` is even.
    fn even(&self) -> bool;
    /// `self / 2`, for an even `self`.
    fn halved(&self) -> Self;
}

impl Exact for Integer {
    fn of(value: i64) -> Self {
        Integer::from(value)
    }

    fn plus(&self, other: &Self) -> Self {
        self.add(other)
    }

    fn minus(&self, other: &Self) -> Self {
        self.sub(other)
    }

    fn negated(&self) -> Self {
        self.neg()
    }

    fn even(&self) -> bool {
        self.is_even()
    }

    fn halved(&self) -> Self {
        let two = Integer::from(2_i64);
        let divisor = NonZeroInteger::try_from(&two).expect("the literal two is nonzero");
        self.div_mod_floor(divisor).0
    }
}

/// The machine integer the Kani harnesses run the matrix code over; `+`
/// and `-` are checked for overflow by Kani.
#[cfg(kani)]
impl Exact for i64 {
    fn of(value: i64) -> Self {
        value
    }

    fn plus(&self, other: &Self) -> Self {
        self + other
    }

    fn minus(&self, other: &Self) -> Self {
        self - other
    }

    fn negated(&self) -> Self {
        -self
    }

    fn even(&self) -> bool {
        self % 2 == 0
    }

    fn halved(&self) -> Self {
        self / 2
    }
}

/// One encoded bound: `Some(2c)` is `(c, <)`, `Some(2c + 1)` is `(c, <=)`,
/// `None` is `∞`.
type Cell<W> = Option<W>;

/// A zone: a conjunction of `x_i - x_j ≺ c` over the reference clock and
/// `dim - 1` clocks, as an encoded difference-bound matrix. Its order is an
/// arbitrary total one, for keying.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Zone<W: Exact = Integer> {
    dim: usize,
    cells: Vec<Cell<W>>,
}

/// The per-clock lower and upper bounds of the aLU simulation (ADR-026
/// EZ-4). Entry `k` is clock `k + 1`'s bound; `None` is `-∞`, a clock no
/// constant is compared with. The reference clock's bounds are 0.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LuBounds<W: Exact = Integer> {
    /// `L`: the constants each clock is compared with from below.
    pub lower: Vec<Option<W>>,
    /// `U`: the constants each clock is compared with from above.
    pub upper: Vec<Option<W>>,
}

/// A zone operation named an index or shape the zone does not have.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ZoneError {
    /// A clock index at or above the zone's dimension.
    #[error("clock {clock} is outside a zone of dimension {dim}")]
    ClockOutOfRange {
        /// The index named.
        clock: usize,
        /// The zone's dimension.
        dim: usize,
    },
    /// A reset of the reference clock, which is fixed at 0.
    #[error("the reference clock cannot be reset")]
    ResetReferenceClock,
    /// A carry or reset assignment to the reference clock.
    #[error("the reference clock cannot be reassigned")]
    ReassignReferenceClock,
    /// A clock is assigned more than once by one simultaneous update.
    #[error("clock {clock} is assigned more than once")]
    DuplicateAssignment {
        /// The clock assigned more than once.
        clock: usize,
    },
    /// Two zones of different dimensions were compared.
    #[error("zones of dimension {left} and {right} cannot be compared")]
    DimensionMismatch {
        /// The receiver's dimension.
        left: usize,
        /// The argument's dimension.
        right: usize,
    },
    /// Extrapolation constants whose length is not the zone's clock count.
    #[error("extrapolation constants for {max} clocks over {clocks} clocks")]
    ExtrapolationLength {
        /// Clocks named by the constants.
        max: usize,
        /// The zone's clock count.
        clocks: usize,
    },
    /// A matrix whose cell count is not its dimension squared.
    #[error("{cells} bounds cannot form a zone of dimension {dim}")]
    MatrixShape {
        /// The bounds given.
        cells: usize,
        /// The dimension named.
        dim: usize,
    },
    /// LU bounds whose length is not the zone's clock count.
    #[error("LU bounds for {lower} lower and {upper} upper clocks over {clocks} clocks")]
    LuLength {
        /// Clocks named by the lower bounds.
        lower: usize,
        /// Clocks named by the upper bounds.
        upper: usize,
        /// The zone's clock count.
        clocks: usize,
    },
}

/// `(c, <=)` encoded.
fn non_strict<W: Exact>(c: &W) -> W {
    c.plus(c).plus(&W::of(1))
}

/// `(c, <)` encoded.
fn strict<W: Exact>(c: &W) -> W {
    c.plus(c)
}

/// `(0, <=)`, the bound every diagonal entry of a non-empty zone holds.
fn zero_le<W: Exact>() -> Cell<W> {
    Some(W::of(1))
}

/// The sum of two bounds: values add, and the sum is strict when either
/// is. With `e = 2c + n` (`n` 1 for `<=`), `e1 + e2 - (n1 | n2)`.
fn plus<W: Exact>(a: &Cell<W>, b: &Cell<W>) -> Cell<W> {
    let (Some(a), Some(b)) = (a, b) else {
        return None;
    };
    let either_non_strict = !a.even() || !b.even();
    let sum = a.plus(b);
    Some(if either_non_strict {
        sum.minus(&W::of(1))
    } else {
        sum
    })
}

/// Whether `a` is strictly tighter than `b`.
fn tighter<W: Exact>(a: &Cell<W>, b: &Cell<W>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a < b,
        (Some(_), None) => true,
        (None, _) => false,
    }
}

impl<W: Exact> Zone<W> {
    /// The zone holding only the valuation with every clock 0.
    pub fn zero(dim: usize) -> Self {
        Self {
            dim,
            cells: vec![zero_le(); dim * dim],
        }
    }

    /// The zone of every valuation with every clock non-negative.
    pub fn universe(dim: usize) -> Self {
        let mut zone = Self {
            dim,
            cells: vec![None; dim * dim],
        };
        for k in 0..dim {
            zone.cells[k * dim + k] = zero_le();
            zone.cells[k] = zero_le();
        }
        zone
    }

    /// The zone of the matrix `bounds`, row-major, each `x_i - x_j ≺ c` as
    /// `Some((c, strict))` or `None` for `∞`, closed to canonical form: how
    /// a certificate written by another DBM implementation is read.
    ///
    /// # Errors
    ///
    /// [`ZoneError::MatrixShape`] when there are not `dim * dim` bounds.
    pub fn from_matrix(
        dim: usize,
        bounds: impl IntoIterator<Item = Option<(W, bool)>>,
    ) -> Result<Self, ZoneError> {
        let cells: Vec<Cell<W>> = bounds
            .into_iter()
            .map(|bound| {
                bound.map(|(value, is_strict)| {
                    if is_strict {
                        strict(&value)
                    } else {
                        non_strict(&value)
                    }
                })
            })
            .collect();
        if cells.len() != dim.saturating_mul(dim) {
            return Err(ZoneError::MatrixShape {
                cells: cells.len(),
                dim,
            });
        }
        let mut zone = Self { dim, cells };
        zone.close();
        Ok(zone)
    }

    /// The dimension: the clock count plus the reference clock.
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// The bound on `x_i - x_j` as `(c, strict)`, or `None` for `∞`.
    pub fn bound(&self, i: usize, j: usize) -> Result<Option<(W, bool)>, ZoneError> {
        self.check_clock(i)?;
        self.check_clock(j)?;
        Ok(self.cell(i, j).as_ref().map(|e| {
            let is_strict = e.even();
            let twice = if is_strict {
                e.clone()
            } else {
                e.minus(&W::of(1))
            };
            (twice.halved(), is_strict)
        }))
    }

    fn cell(&self, i: usize, j: usize) -> &Cell<W> {
        &self.cells[i * self.dim + j]
    }

    fn check_clock(&self, clock: usize) -> Result<(), ZoneError> {
        if clock < self.dim {
            Ok(())
        } else {
            Err(ZoneError::ClockOutOfRange {
                clock,
                dim: self.dim,
            })
        }
    }

    fn check_dim(&self, other: &Self) -> Result<(), ZoneError> {
        if self.dim == other.dim {
            Ok(())
        } else {
            Err(ZoneError::DimensionMismatch {
                left: self.dim,
                right: other.dim,
            })
        }
    }

    /// Replace the matrix by the one empty form: every entry `(0, <)`.
    fn make_empty(&mut self) {
        let empty = Some(strict(&W::of(0)));
        self.cells
            .iter_mut()
            .for_each(|cell| cell.clone_from(&empty));
    }

    /// Whether the zone holds no valuation: some diagonal entry of the
    /// canonical matrix is tighter than `(0, <=)`.
    pub fn is_empty(&self) -> bool {
        (0..self.dim).any(|k| tighter(self.cell(k, k), &zero_le()))
    }

    /// Canonical form by shortest-path closure; an empty zone takes the
    /// one empty form.
    pub fn close(&mut self) {
        let dim = self.dim;
        // `x_k - x_k` is 0: no diagonal entry is looser than `(0, <=)`.
        for k in 0..dim {
            if tighter(&zero_le(), self.cell(k, k)) {
                self.cells[k * dim + k] = zero_le();
            }
        }
        for k in 0..dim {
            for i in 0..dim {
                for j in 0..dim {
                    let via = plus(self.cell(i, k), self.cell(k, j));
                    if tighter(&via, self.cell(i, j)) {
                        self.cells[i * dim + j] = via;
                    }
                }
            }
        }
        if self.is_empty() {
            self.make_empty();
        }
    }

    /// Intersect a canonical zone with `x_i - x_j ≺ value` (`strict` for
    /// `<`), leaving it canonical.
    pub fn constrain(
        &mut self,
        i: usize,
        j: usize,
        value: &W,
        strict_bound: bool,
    ) -> Result<(), ZoneError> {
        self.check_clock(i)?;
        self.check_clock(j)?;
        let new = Some(if strict_bound {
            strict(value)
        } else {
            non_strict(value)
        });
        if self.is_empty() || !tighter(&new, self.cell(i, j)) {
            return Ok(());
        }
        if tighter(&plus(self.cell(j, i), &new), &zero_le()) {
            self.make_empty();
            return Ok(());
        }
        let dim = self.dim;
        let previous = self.clone();
        for a in 0..dim {
            for b in 0..dim {
                let via = plus(&plus(previous.cell(a, i), &new), previous.cell(j, b));
                if tighter(&via, previous.cell(a, b)) {
                    self.cells[a * dim + b] = via;
                }
            }
        }
        Ok(())
    }

    /// Reset `clock` to `value` in a canonical zone, leaving it canonical.
    pub fn reset(&mut self, clock: usize, value: &W) -> Result<(), ZoneError> {
        self.check_clock(clock)?;
        if clock == 0 {
            return Err(ZoneError::ResetReferenceClock);
        }
        if self.is_empty() {
            return Ok(());
        }
        let dim = self.dim;
        let to = Some(non_strict(value));
        let from = Some(non_strict(&value.negated()));
        for k in 0..dim {
            if k == clock {
                continue;
            }
            self.cells[clock * dim + k] = plus(&to, self.cell(0, k));
            self.cells[k * dim + clock] = plus(self.cell(k, 0), &from);
        }
        Ok(())
    }

    /// Apply simultaneous carried-clock copies and constant resets.
    /// `carries` contains `(target, source)` zone clock indices; `resets`
    /// contains `(target, value)` pairs. Unassigned clocks retain their
    /// value. All sources are read from the pre-update zone.
    pub fn assign_clocks(
        &mut self,
        carries: &[(usize, usize)],
        resets: &[(usize, W)],
    ) -> Result<(), ZoneError> {
        #[derive(Clone)]
        enum Value<W> {
            Clock(usize),
            Constant(W),
        }

        let mut values: Vec<Value<W>> = (0..self.dim).map(Value::Clock).collect();
        let mut assigned = vec![false; self.dim];
        for &(target, source) in carries {
            self.check_clock(target)?;
            self.check_clock(source)?;
            if target == 0 {
                return Err(ZoneError::ReassignReferenceClock);
            }
            if assigned[target] {
                return Err(ZoneError::DuplicateAssignment { clock: target });
            }
            assigned[target] = true;
            values[target] = Value::Clock(source);
        }
        for (target, value) in resets {
            self.check_clock(*target)?;
            if *target == 0 {
                return Err(ZoneError::ReassignReferenceClock);
            }
            if assigned[*target] {
                return Err(ZoneError::DuplicateAssignment { clock: *target });
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
                    (Value::Clock(a), Value::Clock(b)) => before.cell(*a, *b).clone(),
                    (Value::Clock(a), Value::Constant(b)) => {
                        plus(before.cell(*a, 0), &Some(non_strict(&b.negated())))
                    }
                    (Value::Constant(a), Value::Clock(b)) => {
                        plus(&Some(non_strict(a)), before.cell(0, *b))
                    }
                    (Value::Constant(a), Value::Constant(b)) => Some(non_strict(&a.minus(b))),
                };
                self.cells[i * self.dim + j] = bound;
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
        resets: &[(usize, W)],
    ) -> Result<Self, ZoneError> {
        #[derive(Clone)]
        enum Value<W> {
            Clock(usize),
            Constant(W),
        }

        let mut values: Vec<Value<W>> = (0..self.dim).map(Value::Clock).collect();
        let mut assigned = vec![false; self.dim];
        for &(target, source) in carries {
            self.check_clock(target)?;
            self.check_clock(source)?;
            if target == 0 {
                return Err(ZoneError::ReassignReferenceClock);
            }
            if assigned[target] {
                return Err(ZoneError::DuplicateAssignment { clock: target });
            }
            assigned[target] = true;
            values[target] = Value::Clock(source);
        }
        for (target, value) in resets {
            self.check_clock(*target)?;
            if *target == 0 {
                return Err(ZoneError::ReassignReferenceClock);
            }
            if assigned[*target] {
                return Err(ZoneError::DuplicateAssignment { clock: *target });
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
                let Some((bound, strict_bound)) = self.bound(i, j)? else {
                    continue;
                };
                match (&values[i], &values[j]) {
                    (Value::Clock(a), Value::Clock(b)) => {
                        preimage.constrain(*a, *b, &bound, strict_bound)?;
                    }
                    (Value::Clock(a), Value::Constant(b)) => {
                        let shifted = bound.plus(b);
                        preimage.constrain(*a, 0, &shifted, strict_bound)?;
                    }
                    (Value::Constant(a), Value::Clock(b)) => {
                        let shifted = bound.minus(a);
                        preimage.constrain(0, *b, &shifted, strict_bound)?;
                    }
                    (Value::Constant(a), Value::Constant(b)) => {
                        let actual = a.minus(b);
                        let satisfies = if strict_bound {
                            actual < bound
                        } else {
                            actual <= bound
                        };
                        if !satisfies {
                            preimage.make_empty();
                            return Ok(preimage);
                        }
                    }
                }
            }
        }
        Ok(preimage)
    }

    /// Free `clock` in a canonical zone: every constraint on it is dropped
    /// but `clock >= 0`, leaving it canonical. With the reset value
    /// constrained first, this is the inverse image of a reset.
    pub fn free(&mut self, clock: usize) -> Result<(), ZoneError> {
        self.check_clock(clock)?;
        if clock == 0 {
            return Err(ZoneError::ResetReferenceClock);
        }
        if self.is_empty() {
            return Ok(());
        }
        let dim = self.dim;
        for k in 0..dim {
            if k == clock {
                continue;
            }
            self.cells[clock * dim + k] = None;
            self.cells[k * dim + clock] = self.cell(k, 0).clone();
        }
        self.cells[clock] = zero_le();
        Ok(())
    }

    /// Let time elapse in a canonical zone: every clock's upper bound
    /// against the reference clock becomes `∞`.
    pub fn up(&mut self) {
        if self.is_empty() {
            return;
        }
        for k in 1..self.dim {
            self.cells[k * self.dim] = None;
        }
    }

    /// The valuations from which time elapsing reaches this canonical zone:
    /// every clock's lower bound against the reference clock is dropped to
    /// what the other clocks' differences imply, left canonical.
    pub fn down(&mut self) {
        if self.is_empty() {
            return;
        }
        let dim = self.dim;
        for i in 1..dim {
            let mut lower = zero_le();
            for j in 1..dim {
                if tighter(self.cell(j, i), &lower) {
                    lower.clone_from(self.cell(j, i));
                }
            }
            self.cells[i] = lower;
        }
        self.close();
    }

    /// This canonical zone without canonical `other`, as disjoint canonical
    /// zones: each the part outside one more of `other`'s bounds.
    ///
    /// # Errors
    ///
    /// [`ZoneError::DimensionMismatch`] for zones of different dimensions.
    pub fn minus(&self, other: &Self) -> Result<Vec<Self>, ZoneError> {
        self.check_dim(other)?;
        if other.is_empty() || self.is_empty() {
            return Ok(if self.is_empty() {
                Vec::new()
            } else {
                vec![self.clone()]
            });
        }
        let mut pieces = Vec::new();
        let mut rest = self.clone();
        for i in 0..self.dim {
            for j in (0..self.dim).filter(|&j| j != i) {
                let Some((value, is_strict)) = other.bound(i, j)? else {
                    continue;
                };
                // Not `x_i - x_j ≺ c`: `x_j - x_i ≺' -c`, `<` for `<=`.
                let mut outside = rest.clone();
                outside.constrain(j, i, &value.negated(), !is_strict)?;
                if !outside.is_empty() {
                    pieces.push(outside);
                }
                rest.constrain(i, j, &value, is_strict)?;
                if rest.is_empty() {
                    return Ok(pieces);
                }
            }
        }
        Ok(pieces)
    }

    /// Whether this canonical zone includes canonical `other`.
    pub fn includes(&self, other: &Self) -> Result<bool, ZoneError> {
        self.check_dim(other)?;
        if other.is_empty() {
            return Ok(true);
        }
        if self.is_empty() {
            return Ok(false);
        }
        Ok(self
            .cells
            .iter()
            .zip(&other.cells)
            .all(|(mine, theirs)| !tighter(mine, theirs)))
    }

    /// The classical `Extra_M` extrapolation (Behrmann, Bouyer, Larsen and
    /// Pelánek) of a canonical zone, left canonical: entry `k` of `max` is
    /// clock `k + 1`'s largest constant, `None` for a clock no constant is
    /// compared with. A bound `x_i - x_j ≺ c` with `c > M_i` becomes `∞`,
    /// and one with `c < -M_j` becomes `< -M_j`, so the zones a run reaches
    /// are finitely many (FR-237's fresh exploration).
    pub fn extrapolate(&mut self, max: &[Option<W>]) -> Result<(), ZoneError> {
        let clocks = self.dim.saturating_sub(1);
        if max.len() != clocks {
            return Err(ZoneError::ExtrapolationLength {
                max: max.len(),
                clocks,
            });
        }
        if self.is_empty() {
            return Ok(());
        }
        let ceiling = |k: usize| if k == 0 { None } else { max[k - 1].as_ref() };
        let dim = self.dim;
        for i in 0..dim {
            for j in (0..dim).filter(|&j| j != i) {
                let Some((value, _)) = self.bound(i, j)? else {
                    continue;
                };
                let above = match ceiling(i) {
                    Some(m) => &value > m,
                    None => i != 0,
                };
                let below = match ceiling(j) {
                    Some(m) => value < m.negated(),
                    None => j != 0 && value < W::of(0),
                };
                if above {
                    self.cells[i * dim + j] = None;
                } else if below {
                    let m = ceiling(j).cloned().unwrap_or_else(|| W::of(0));
                    self.cells[i * dim + j] = Some(strict(&m.negated()));
                }
            }
        }
        self.close();
        Ok(())
    }

    /// Whether this canonical zone is covered by canonical `target` under
    /// the aLU simulation: `self ⊆ aLU(target)` (Herbreteau, Srivathsan and
    /// Walukiewicz). It fails exactly when some clocks `x != y` have
    /// `self[0][x] >= (-U_x, <=)`, `target[y][x] < self[y][x]` and
    /// `target[y][x] + (-L_y, <) < self[0][x]`.
    pub fn alu_covered_by(&self, target: &Self, lu: &LuBounds<W>) -> Result<bool, ZoneError> {
        self.check_dim(target)?;
        let clocks = self.dim.saturating_sub(1);
        if lu.lower.len() != clocks || lu.upper.len() != clocks {
            return Err(ZoneError::LuLength {
                lower: lu.lower.len(),
                upper: lu.upper.len(),
                clocks,
            });
        }
        if self.is_empty() {
            return Ok(true);
        }
        if target.is_empty() {
            return Ok(false);
        }
        // `(-U_x, <=)` and `(-L_y, <)`; `-(-∞)` is `∞`.
        let bound_of = |bounds: &[Option<W>], k: usize, encode: fn(&W) -> W| {
            if k == 0 {
                Some(encode(&W::of(0)))
            } else {
                bounds[k - 1].as_ref().map(|c| encode(&c.negated()))
            }
        };
        for x in 0..self.dim {
            let minus_u = bound_of(&lu.upper, x, non_strict);
            if tighter(self.cell(0, x), &minus_u) {
                continue;
            }
            for y in (0..self.dim).filter(|&y| y != x) {
                let minus_l = bound_of(&lu.lower, y, strict);
                if tighter(target.cell(y, x), self.cell(y, x))
                    && tighter(&plus(target.cell(y, x), &minus_l), self.cell(0, x))
                {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

#[cfg(any(test, kani))]
mod reference;

#[cfg(test)]
mod tests;

#[cfg(kani)]
mod proofs;
