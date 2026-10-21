// SPDX-License-Identifier: AGPL-3.0-or-later
//! Interval distance in counted steps over a finite trace (ADR-027 PB-3,
//! QSpec FR-431).
//!
//! The distance from position `i` to position `j >= i` is the number of
//! counted steps among steps `i + 1` to `j`; an uncounted step moves the
//! position and not the distance. Positions before 0 and the positions of
//! the false extension past a finite trace are each one counted step (the
//! terminal stutter step is counted, PB-3), so the distance there is the
//! distance in positions.

use qsl_foundation::InternalFault;

use super::letters::TemporalLetters;
use super::plan::fault;
use super::position::TemporalPosition;

/// The counted steps of a finite trace with at least one uncounted step.
#[derive(Debug)]
pub(super) struct Steps {
    /// `counted[i]` is the number of counted steps among steps `1..=i`.
    counted: Vec<i128>,
}

impl Steps {
    /// The counted steps of the `len` positions `letters` reads, or `None`
    /// when every step counts, which is the distance in positions.
    pub(super) fn read<L: TemporalLetters>(
        len: u64,
        letters: &L,
    ) -> Result<Option<Self>, InternalFault> {
        let len = usize::try_from(len).map_err(|_| fault("trace-length-in-usize"))?;
        let mut counted = Vec::with_capacity(len);
        let mut all = true;
        let mut total = 0_i128;
        for index in 0..len {
            if index > 0 {
                let position = u64::try_from(index).map_err(|_| fault("position-in-u64"))?;
                if letters.counts_step(TemporalPosition::new(position)) {
                    total += 1;
                } else {
                    all = false;
                }
            }
            counted.push(total);
        }
        Ok((!all).then_some(Self { counted }))
    }

    fn last(&self) -> Result<(i128, i128), InternalFault> {
        let end = self
            .counted
            .last()
            .ok_or_else(|| fault("trace-has-a-position"))?;
        let at = i128::try_from(self.counted.len() - 1).map_err(|_| fault("position-in-i128"))?;
        Ok((at, *end))
    }

    /// The counted steps from position 0 to `q`; negative before it.
    pub(super) fn to(&self, q: i128) -> Result<i128, InternalFault> {
        if q <= 0 {
            return Ok(q);
        }
        let (last, end) = self.last()?;
        if q > last {
            return Ok(end + (q - last));
        }
        let index = usize::try_from(q).map_err(|_| fault("position-in-usize"))?;
        self.counted
            .get(index)
            .copied()
            .ok_or_else(|| fault("position-is-represented"))
    }

    /// The first position `q` with `to(q) >= count`.
    pub(super) fn first_reaching(&self, count: i128) -> Result<i128, InternalFault> {
        if count <= 0 {
            return Ok(count);
        }
        let reached = self.counted.partition_point(|steps| *steps < count);
        if reached < self.counted.len() {
            return i128::try_from(reached).map_err(|_| fault("position-in-i128"));
        }
        let (last, end) = self.last()?;
        Ok(last + (count - end))
    }

    /// The last position `q` with `to(q) <= count`.
    pub(super) fn last_within(&self, count: i128) -> Result<i128, InternalFault> {
        if count < 0 {
            return Ok(count);
        }
        let within = self.counted.partition_point(|steps| *steps <= count);
        if within < self.counted.len() {
            return i128::try_from(within - 1).map_err(|_| fault("position-in-i128"));
        }
        let (last, end) = self.last()?;
        Ok(last + (count - end))
    }
}
