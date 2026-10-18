// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 IV-3's counter construction for interval operators: the state an
//! automaton keeps per occurrence so it never keeps one state per pending
//! next-position obligation.
//!
//! Every counter is a small value type whose `step` consumes one position:
//! whether the occurrence opens an obligation at that position and whether
//! its operand holds there. A counter's meaning is the expansion of IV-3 into
//! nested next-position steps, and the tests check each counter against that
//! expansion on every short trace.
//!
//! - For lower bound 0 over an operand with no interval operator, one counter
//!   with `b + 1` values: [`DeadlineCounter`] (`eventually`, `until`: the
//!   earliest open deadline subsumes the later ones) and [`ExtentCounter`]
//!   (`always`, `release`: the furthest required position subsumes the
//!   nearer ones).
//! - A past operator with lower bound 0: [`PastAgeCounter`], the positions
//!   since the operand last held, saturating past `b`.
//! - An `[a,*]` occurrence: [`ShiftCounter`] with `a + 1` values, then the
//!   unbounded operator's own states.
//! - Any other occurrence keeps its open obligations as a set:
//!   [`EventuallyWindows`] and [`AlwaysOffsets`]. Offsets are kept as
//!   inclusive ranges, so an upper bound as large as `u64::MAX` costs one
//!   range, not one entry per offset.

use std::collections::BTreeSet;

use thiserror::Error;

use crate::automaton::error::AutomatonError;

/// An `eventually`-style obligation's window closed with its operand false.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("an obligation's window closed with its operand false")]
pub struct Expired;

/// An `always`-style obligation's operand was false inside its window.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("an operand was false inside an obligation's window")]
pub struct Violated;

/// `eventually[0,b]` and `until[0,b]` over an operand with no interval
/// operator: the earliest open deadline. An obligation opened at position
/// `i` needs the operand at some position in `[i, i + b]`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeadlineCounter {
    bound: u64,
    /// The positions after the current one that may still discharge the
    /// earliest open obligation; 0 when none is open.
    remaining: u64,
}

impl DeadlineCounter {
    /// A counter with nothing open, for upper bound `bound`.
    pub const fn new(bound: u64) -> Self {
        Self {
            bound,
            remaining: 0,
        }
    }

    /// Whether an obligation is open.
    pub const fn is_pending(self) -> bool {
        self.remaining != 0
    }

    /// The positions after the current one that may still discharge the
    /// earliest open obligation; 0 when none is open.
    pub const fn remaining(self) -> u64 {
        self.remaining
    }

    /// The number of values the counter takes: `b + 1`.
    pub fn value_count(self) -> u128 {
        u128::from(self.bound) + 1
    }

    /// The counter after one position.
    ///
    /// # Errors
    ///
    /// [`Expired`] when an open obligation's window closes at this position
    /// with the operand false.
    pub fn step(self, opens: bool, operand_holds: bool) -> Result<Self, Expired> {
        if operand_holds {
            return Ok(Self {
                remaining: 0,
                ..self
            });
        }
        let aged = self.remaining.checked_sub(1);
        let fresh = opens.then_some(self.bound);
        let next = match (aged, fresh) {
            (Some(aged), Some(fresh)) => aged.min(fresh),
            (Some(only), None) | (None, Some(only)) => only,
            (None, None) => return Ok(self),
        };
        if next == 0 {
            Err(Expired)
        } else {
            Ok(Self {
                remaining: next,
                ..self
            })
        }
    }
}

/// `always[0,b]` and `release[0,b]` over an operand with no interval
/// operator: the furthest required position. An obligation opened at
/// position `i` needs the operand at every position in `[i, i + b]`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ExtentCounter {
    bound: u64,
    /// The positions after the current one that still need the operand; 0
    /// when none does.
    remaining: u64,
}

impl ExtentCounter {
    /// A counter with nothing required, for upper bound `bound`.
    pub const fn new(bound: u64) -> Self {
        Self {
            bound,
            remaining: 0,
        }
    }

    /// Whether some later position still needs the operand.
    pub const fn is_pending(self) -> bool {
        self.remaining != 0
    }

    /// The positions after the current one that still need the operand.
    pub const fn remaining(self) -> u64 {
        self.remaining
    }

    /// The number of values the counter takes: `b + 1`.
    pub fn value_count(self) -> u128 {
        u128::from(self.bound) + 1
    }

    /// The counter after one position.
    ///
    /// # Errors
    ///
    /// [`Violated`] when the operand is false at a position an open
    /// obligation requires.
    pub fn step(self, opens: bool, operand_holds: bool) -> Result<Self, Violated> {
        let required_here = opens || self.remaining != 0;
        if required_here && !operand_holds {
            return Err(Violated);
        }
        let carried = self.remaining.saturating_sub(1);
        let fresh = if opens { self.bound } else { 0 };
        Ok(Self {
            remaining: carried.max(fresh),
            ..self
        })
    }
}

/// `once[0,b]` and `since[0,b]` (and, over the negated operand,
/// `historically` and `triggered`): the positions since the operand last
/// held, saturating past `b`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PastAgeCounter {
    bound: u64,
    /// Positions since the operand last held; `None` when it never has or
    /// that is more than `bound` positions ago.
    age: Option<u64>,
}

impl PastAgeCounter {
    /// A counter for which the operand has never held, for upper bound
    /// `bound`: the operand reads false before position 0 (ADR-018 IV-2).
    pub const fn new(bound: u64) -> Self {
        Self { bound, age: None }
    }

    /// Whether the operand held within the last `b` positions, the current
    /// one included.
    pub const fn within_bound(self) -> bool {
        self.age.is_some()
    }

    /// Positions since the operand last held, when that is at most `b`.
    pub const fn age(self) -> Option<u64> {
        self.age
    }

    /// The number of values the counter takes: `b + 2`, the ages `0..=b` and
    /// the saturated one. The count is exact for `b = u64::MAX`.
    pub fn value_count(self) -> u128 {
        u128::from(self.bound) + 2
    }

    /// The counter after one position.
    #[must_use]
    pub fn step(self, operand_holds: bool) -> Self {
        let age = if operand_holds {
            Some(0)
        } else {
            self.age
                .and_then(|age| age.checked_add(1))
                .filter(|age| *age <= self.bound)
        };
        Self { age, ..self }
    }
}

/// The shift of an `[a,*]` occurrence: the positions to wait before the
/// unbounded operator's own states begin.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ShiftCounter {
    shift: u64,
    remaining: u64,
}

impl ShiftCounter {
    /// A counter opened at the current position, for lower bound `shift`.
    pub const fn new(shift: u64) -> Self {
        Self {
            shift,
            remaining: shift,
        }
    }

    /// Whether the current position is the first of the unbounded operator:
    /// `shift` positions have passed since the counter was opened.
    pub const fn is_open(self) -> bool {
        self.remaining == 0
    }

    /// The number of values the counter takes: `a + 1`.
    pub fn value_count(self) -> u128 {
        u128::from(self.shift) + 1
    }

    /// The counter one position later.
    #[must_use]
    pub const fn advance(self) -> Self {
        Self {
            remaining: self.remaining.saturating_sub(1),
            ..self
        }
    }
}

/// `eventually[a,b]` in general (a nonzero lower bound, or an operand that
/// holds another interval operator): the set of open obligation windows. An
/// obligation opened at position `i` needs the operand at some position in
/// `[i + a, i + b]`. A window that contains another is dropped, because
/// discharging the inner window discharges it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventuallyWindows {
    lower: u64,
    upper: u64,
    /// Open windows as inclusive offsets from the current position.
    windows: BTreeSet<(u64, u64)>,
}

impl EventuallyWindows {
    /// A set with nothing open, for the interval `[lower, upper]`.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::InvertedInterval`] when `lower > upper`.
    pub fn new(lower: u64, upper: u64) -> Result<Self, AutomatonError> {
        if lower > upper {
            return Err(AutomatonError::InvertedInterval { lower, upper });
        }
        Ok(Self {
            lower,
            upper,
            windows: BTreeSet::new(),
        })
    }

    /// Whether an obligation is open.
    pub fn is_pending(&self) -> bool {
        !self.windows.is_empty()
    }

    /// The open windows as inclusive offsets from the position after the one
    /// last read.
    pub fn windows(&self) -> &BTreeSet<(u64, u64)> {
        &self.windows
    }

    /// The set after one position.
    ///
    /// # Errors
    ///
    /// [`Expired`] when an open window closes at this position with the
    /// operand false.
    pub fn step(&self, opens: bool, operand_holds: bool) -> Result<Self, Expired> {
        let mut windows = self.windows.clone();
        if opens {
            windows.insert((self.lower, self.upper));
        }
        if operand_holds {
            windows.retain(|&(start, _)| start != 0);
        } else if windows.iter().any(|&(start, end)| start == 0 && end == 0) {
            return Err(Expired);
        }
        let shifted: BTreeSet<(u64, u64)> = windows
            .into_iter()
            .map(|(start, end)| (start.saturating_sub(1), end.saturating_sub(1)))
            .collect();
        Ok(Self {
            windows: without_enclosing(&shifted),
            ..self.clone()
        })
    }
}

/// `windows` without any window that strictly contains another.
fn without_enclosing(windows: &BTreeSet<(u64, u64)>) -> BTreeSet<(u64, u64)> {
    windows
        .iter()
        .copied()
        .filter(|&(start, end)| {
            !windows.iter().any(|&(inner_start, inner_end)| {
                (inner_start, inner_end) != (start, end) && inner_start >= start && inner_end <= end
            })
        })
        .collect()
}

/// `always[a,b]` in general: the set of offsets, from the current position,
/// at which the operand is still required, kept as disjoint inclusive ranges
/// in ascending order. An obligation opened at position `i` requires the
/// operand at every position in `[i + a, i + b]`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlwaysOffsets {
    lower: u64,
    upper: u64,
    required: Vec<(u64, u64)>,
}

impl AlwaysOffsets {
    /// A set with nothing required, for the interval `[lower, upper]`.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::InvertedInterval`] when `lower > upper`.
    pub fn new(lower: u64, upper: u64) -> Result<Self, AutomatonError> {
        if lower > upper {
            return Err(AutomatonError::InvertedInterval { lower, upper });
        }
        Ok(Self {
            lower,
            upper,
            required: Vec::new(),
        })
    }

    /// Whether some later position still requires the operand.
    pub fn is_pending(&self) -> bool {
        !self.required.is_empty()
    }

    /// The required offsets as disjoint inclusive ranges in ascending order,
    /// from the position after the one last read.
    pub fn required(&self) -> &[(u64, u64)] {
        &self.required
    }

    /// The set after one position.
    ///
    /// # Errors
    ///
    /// [`Violated`] when the operand is false at a required position.
    pub fn step(&self, opens: bool, operand_holds: bool) -> Result<Self, Violated> {
        let mut ranges = self.required.clone();
        if opens {
            ranges.push((self.lower, self.upper));
        }
        let ranges = merged(ranges);
        let required_here = ranges.first().is_some_and(|&(start, _)| start == 0);
        if required_here && !operand_holds {
            return Err(Violated);
        }
        let required = ranges
            .into_iter()
            .filter(|&(_, end)| end != 0)
            .map(|(start, end)| (start.saturating_sub(1), end.saturating_sub(1)))
            .collect();
        Ok(Self {
            required,
            ..self.clone()
        })
    }
}

/// `ranges` as disjoint, non-adjacent inclusive ranges in ascending order.
fn merged(mut ranges: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
    ranges.sort_unstable();
    let mut result: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (start, end) in ranges {
        if let Some(last) = result.last_mut() {
            if start <= last.1.saturating_add(1) {
                last.1 = last.1.max(end);
                continue;
            }
        }
        result.push((start, end));
    }
    result
}
