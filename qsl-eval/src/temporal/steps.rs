// SPDX-License-Identifier: AGPL-3.0-or-later
//! Interval distance in counted steps over a finite trace or a lasso
//! (ADR-027 PB-3, QSpec FR-431).
//!
//! The distance from position `i` to position `j >= i` is the number of
//! counted steps among steps `i + 1` to `j`; an uncounted step moves the
//! position and not the distance. Positions before 0 and the positions of
//! the false extension past a finite trace are each one counted step (the
//! terminal stutter step is counted, PB-3), so the distance there is the
//! distance in positions.
//!
//! On a lasso the positions past the represented ones are the loop again. The
//! step that closes the loop (from its last position back to its entry) counts
//! or not as the letter says, and each pass of the loop adds the same counted
//! steps, so the distance of a position is periodic: a loop with no counted
//! step keeps every distance from a position before it constant, and an
//! interval that needs a counted step after the loop is entered is not met on
//! it (PB-3).

use qsl_foundation::InternalFault;
use quire_exact::{Charge, ChargePoint, Incomplete, Meter};

use super::letters::TemporalLetters;
use super::plan::fault;
use super::position::TemporalPosition;

/// Why the counted steps could not be read.
pub(super) enum ReadStop {
    /// The work budget ran out.
    Incomplete(Incomplete),
    /// An invariant broke.
    Fault(InternalFault),
}

impl From<Incomplete> for ReadStop {
    fn from(incomplete: Incomplete) -> Self {
        Self::Incomplete(incomplete)
    }
}

impl From<InternalFault> for ReadStop {
    fn from(fault: InternalFault) -> Self {
        Self::Fault(fault)
    }
}

/// `numerator / denominator` rounded up, for a non-negative `numerator` and a
/// positive `denominator`.
fn ceil_div(numerator: i128, denominator: i128) -> i128 {
    numerator.div_euclid(denominator) + i128::from(numerator.rem_euclid(denominator) != 0)
}

/// The loop of a lasso whose steps are counted (ADR-027 PB-3).
#[derive(Clone, Copy, Debug)]
pub(super) struct LoopShape {
    /// The positions before the loop.
    pub(super) prefix: u64,
    /// The positions of the loop.
    pub(super) loop_len: u64,
    /// Whether the step that closes the loop counts.
    pub(super) edge_counts: bool,
}

/// The counted steps of a finite trace or a lasso with at least one
/// uncounted step.
#[derive(Debug)]
pub(super) struct Steps {
    /// `counted[i]` is the number of counted steps among steps `1..=i` of
    /// the represented positions.
    counted: Vec<i128>,
    /// The loop of a lasso; `None` for a finite trace.
    cycle: Option<Cycle>,
}

/// A lasso's loop as the distance function reads it.
#[derive(Debug)]
struct Cycle {
    prefix: i128,
    period: i128,
    /// The counted steps of one pass of the loop, its closing step included.
    per_pass: i128,
}

impl Steps {
    /// The counted steps of the `len` positions `letters` reads, or `None`
    /// when every step counts, which is the distance in positions. A trace
    /// whose steps all count is scanned without allocating, and one with an
    /// uncounted step pays `meter` one work unit per position. `lasso` is the
    /// loop of a lasso, whose closing step is read too.
    pub(super) fn read<L: TemporalLetters>(
        len: u64,
        lasso: Option<LoopShape>,
        letters: &L,
        meter: &mut Meter,
    ) -> Result<Option<Self>, ReadStop> {
        let counts = |index: u64| letters.counts_step(TemporalPosition::new(index));
        if (1..len).all(counts) && lasso.is_none_or(|shape| shape.edge_counts) {
            return Ok(None);
        }
        let size = usize::try_from(len).map_err(|_| fault("trace-length-in-usize"))?;
        let mut counted = Vec::with_capacity(size);
        let mut total = 0_i128;
        for index in 0..len {
            meter.charge(Charge::new(ChargePoint::GraphExpand))?;
            total += i128::from(index > 0 && counts(index));
            counted.push(total);
        }
        let cycle = match lasso {
            Some(shape) => {
                let prefix =
                    usize::try_from(shape.prefix).map_err(|_| fault("prefix-length-in-usize"))?;
                let at_entry = *counted
                    .get(prefix)
                    .ok_or_else(|| fault("loop-entry-is-represented"))?;
                Some(Cycle {
                    prefix: i128::from(shape.prefix),
                    period: i128::from(shape.loop_len),
                    per_pass: total - at_entry + i128::from(shape.edge_counts),
                })
            }
            None => None,
        };
        Ok(Some(Self { counted, cycle }))
    }

    /// The counted steps of one pass of the loop of a lasso, its closing step
    /// included; `None` for a finite trace.
    pub(super) fn per_pass(&self) -> Option<i128> {
        self.cycle.as_ref().map(|cycle| cycle.per_pass)
    }

    fn last(&self) -> Result<(i128, i128), InternalFault> {
        let end = self
            .counted
            .last()
            .ok_or_else(|| fault("trace-has-a-position"))?;
        let at = i128::try_from(self.counted.len() - 1).map_err(|_| fault("position-in-i128"))?;
        Ok((at, *end))
    }

    /// The counted steps from the loop entry to the position `offset` into
    /// the loop's first pass.
    fn inside(&self, cycle: &Cycle, offset: i128) -> Result<i128, InternalFault> {
        let at = |position: i128| {
            usize::try_from(position)
                .ok()
                .and_then(|index| self.counted.get(index))
                .copied()
                .ok_or_else(|| fault("position-is-represented"))
        };
        Ok(at(cycle.prefix + offset)? - at(cycle.prefix)?)
    }

    /// The counted steps from position 0 to `q`; negative before it.
    pub(super) fn to(&self, q: i128) -> Result<i128, InternalFault> {
        if q <= 0 {
            return Ok(q);
        }
        let Some(cycle) = &self.cycle else {
            let (last, end) = self.last()?;
            if q > last {
                return Ok(end + (q - last));
            }
            let index = usize::try_from(q).map_err(|_| fault("position-in-usize"))?;
            return self
                .counted
                .get(index)
                .copied()
                .ok_or_else(|| fault("position-is-represented"));
        };
        if q < cycle.prefix {
            let index = usize::try_from(q).map_err(|_| fault("position-in-usize"))?;
            return self
                .counted
                .get(index)
                .copied()
                .ok_or_else(|| fault("position-is-represented"));
        }
        let into = q - cycle.prefix;
        let (pass, offset) = (into.div_euclid(cycle.period), into.rem_euclid(cycle.period));
        let entry = self.entry(cycle)?;
        Ok(entry + pass * cycle.per_pass + self.inside(cycle, offset)?)
    }

    /// The counted steps from position 0 to the loop entry's first visit.
    fn entry(&self, cycle: &Cycle) -> Result<i128, InternalFault> {
        let index = usize::try_from(cycle.prefix).map_err(|_| fault("position-in-usize"))?;
        self.counted
            .get(index)
            .copied()
            .ok_or_else(|| fault("position-is-represented"))
    }

    /// The first position `q` with `to(q) >= count`; `None` when no position
    /// of a lasso is that far, because its loop has no counted step.
    pub(super) fn first_reaching(&self, count: i128) -> Result<Option<i128>, InternalFault> {
        if count <= 0 {
            return Ok(Some(count));
        }
        let Some(cycle) = &self.cycle else {
            let reached = self.counted.partition_point(|steps| *steps < count);
            if reached < self.counted.len() {
                return i128::try_from(reached)
                    .map(Some)
                    .map_err(|_| fault("position-in-i128"));
            }
            let (last, end) = self.last()?;
            return Ok(Some(last + (count - end)));
        };
        let entry = self.entry(cycle)?;
        if count <= entry {
            let reached = self.counted.partition_point(|steps| *steps < count);
            return i128::try_from(reached)
                .map(Some)
                .map_err(|_| fault("position-in-i128"));
        }
        // The least pass whose last position is far enough, then the first
        // position of that pass that is.
        let need = count - entry;
        let widest = self.inside(cycle, cycle.period - 1)?;
        if cycle.per_pass == 0 {
            return Ok(None);
        }
        let pass = ceil_div((need - widest).max(0), cycle.per_pass);
        let wanted = need - pass * cycle.per_pass;
        let prefix = usize::try_from(cycle.prefix).map_err(|_| fault("position-in-usize"))?;
        let period = usize::try_from(cycle.period).map_err(|_| fault("loop-in-usize"))?;
        let pass_steps = self
            .counted
            .get(prefix..prefix + period)
            .ok_or_else(|| fault("position-is-represented"))?;
        let base = pass_steps
            .first()
            .copied()
            .ok_or_else(|| fault("loop-has-a-position"))?;
        let offset = pass_steps.partition_point(|steps| steps - base < wanted);
        let offset = i128::try_from(offset).map_err(|_| fault("position-in-i128"))?;
        Ok(Some(cycle.prefix + pass * cycle.period + offset))
    }

    /// The last position `q` with `to(q) <= count`; `None` when every
    /// position of a lasso past some point is, because its loop has no
    /// counted step and `count` reaches it.
    pub(super) fn last_within(&self, count: i128) -> Result<Option<i128>, InternalFault> {
        if count < 0 {
            return Ok(Some(count));
        }
        let Some(cycle) = &self.cycle else {
            let within = self.counted.partition_point(|steps| *steps <= count);
            if within < self.counted.len() {
                return i128::try_from(within - 1)
                    .map(Some)
                    .map_err(|_| fault("position-in-i128"));
            }
            let (last, end) = self.last()?;
            return Ok(Some(last + (count - end)));
        };
        let entry = self.entry(cycle)?;
        if count < entry {
            let within = self.counted.partition_point(|steps| *steps <= count);
            return i128::try_from(within - 1)
                .map(Some)
                .map_err(|_| fault("position-in-i128"));
        }
        if cycle.per_pass == 0 {
            return Ok(None);
        }
        let rest = count - entry;
        let pass = rest.div_euclid(cycle.per_pass);
        let wanted = rest - pass * cycle.per_pass;
        let prefix = usize::try_from(cycle.prefix).map_err(|_| fault("position-in-usize"))?;
        let period = usize::try_from(cycle.period).map_err(|_| fault("loop-in-usize"))?;
        let pass_steps = self
            .counted
            .get(prefix..prefix + period)
            .ok_or_else(|| fault("position-is-represented"))?;
        let base = pass_steps
            .first()
            .copied()
            .ok_or_else(|| fault("loop-has-a-position"))?;
        let offset = pass_steps.partition_point(|steps| steps - base <= wanted);
        let offset = i128::try_from(offset - 1).map_err(|_| fault("position-in-i128"))?;
        Ok(Some(cycle.prefix + pass * cycle.period + offset))
    }
}
