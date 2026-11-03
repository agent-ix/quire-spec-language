// SPDX-License-Identifier: AGPL-3.0-or-later
//! The timestamped trace and its progress (FR-501 Inputs).

use super::cover::ClockCover;
use crate::temporal::ledger::{ProgressClosure, ProgressCompleteness};
use crate::temporal::outcome::TemporalRefusal;
use crate::temporal::position::TemporalPosition;

/// Whether the completeness assertion covers every tick up to `tick`.
pub(super) fn covers(completeness: ProgressCompleteness, tick: i128) -> bool {
    match completeness {
        ProgressCompleteness::Complete { through } => i128::from(through) >= tick,
        ProgressCompleteness::Incomplete => false,
    }
}

/// Whether the trace's lower boundary is the origin of the history or only a
/// cutoff within it (FR-501 "Verdicts").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LowerBoundary {
    /// History before the first position is empty.
    AuthoritativeOrigin,
    /// History before the first position exists but was not supplied.
    HistoryCutoff,
}

/// A progress watermark as the supplier states it (QSpec FR-094).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Watermark {
    /// Progress covers every tick up to and including the value.
    Inclusive(u64),
    /// Progress covers every tick strictly below `end`, supplied without a
    /// QSpec FR-289 mapping that proves inclusive coverage of `end`: it is
    /// read as the inclusive watermark `end - 1`, and as no progress at all
    /// when `end` is 0.
    HalfOpen {
        /// The exclusive end.
        end: u64,
    },
}

impl Watermark {
    /// The greatest tick progress covers, or -1 when it covers none.
    pub(super) fn inclusive(self) -> i128 {
        match self {
            Self::Inclusive(tick) => i128::from(tick),
            Self::HalfOpen { end } => i128::from(end) - 1,
        }
    }
}

/// An order key: an order-authority identity and a key within it. Distinct
/// keys of one authority order two observations; keys of different
/// authorities, and a key repeated within one authority, do not (QSpec
/// FR-090-AC-4, FR-501 "usable order").
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OrderKey {
    /// The order authority.
    pub authority: String,
    /// The key within the authority.
    pub key: i64,
}

/// One admitted observation of a timestamped trace.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TimestampedObservation {
    /// The stamp: a non-negative count of ticks of the clock unit.
    pub stamp: u64,
    /// The order key, when the observation has one.
    pub order_key: Option<OrderKey>,
    /// The identity of the admitted observation; it enters the result
    /// identity so that a change of any valuation changes it.
    pub observation: String,
}

/// The progress a timestamped trace asserts (FR-501 Inputs).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowProgress {
    /// The watermark `W`.
    pub watermark: Watermark,
    /// The completeness assertion.
    pub completeness: ProgressCompleteness,
    /// The decision scope's closure.
    pub scope: ProgressClosure,
    /// The surrounding execution's closure.
    pub execution: ProgressClosure,
    /// The lower-boundary kind.
    pub lower_boundary: LowerBoundary,
}

/// The clock binding's unit and cover.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WindowClock {
    /// The declared timestamp unit.
    pub unit: String,
    /// The declared cover of a stamp.
    pub cover: ClockCover,
}

/// A non-empty timestamped trace in canonical order (FR-501 Inputs, "Canonical
/// order").
#[derive(Clone, Debug)]
pub struct TimestampedTrace {
    pub(super) positions: Vec<TimestampedObservation>,
    origin: Vec<usize>,
    pub(super) progress: WindowProgress,
    /// The progress the request asserted, which the report and the result
    /// identity name. It is `progress` itself for a timestamped trace; a
    /// progressed trace (FR-504) reads an effective `progress` of its own.
    pub(super) asserted: WindowProgress,
    /// How many leading positions are observations: the positions after them
    /// are silent samples (FR-504), which no activation anchors on.
    pub(super) observed: usize,
    pub(super) clock: WindowClock,
    pub(super) clause_identity: String,
}

impl TimestampedTrace {
    /// Admit `positions`, given in any order. A trace with no position is
    /// [`TemporalRefusal::EmptyTrace`], refused before any position is
    /// visited. The positions are put in canonical order (FR-501 "Canonical
    /// order"): ascending stamp, then ascending order-authority identity (an
    /// observation without an order key after every keyed one), then
    /// ascending key within the authority, then ascending observation
    /// identity. The listing order decides nothing: the admitted position of
    /// an observation is its index in that order.
    pub fn admit(
        clause_identity: impl Into<String>,
        clock: WindowClock,
        positions: Vec<TimestampedObservation>,
        progress: WindowProgress,
    ) -> Result<Self, TemporalRefusal> {
        if positions.is_empty() {
            return Err(TemporalRefusal::EmptyTrace);
        }
        let mut order: Vec<usize> = (0..positions.len()).collect();
        order.sort_by(|left, right| {
            let (Some(left), Some(right)) = (positions.get(*left), positions.get(*right)) else {
                return std::cmp::Ordering::Equal;
            };
            left.stamp
                .cmp(&right.stamp)
                .then_with(|| match (&left.order_key, &right.order_key) {
                    (Some(left), Some(right)) => left.cmp(right),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                })
                .then_with(|| left.observation.cmp(&right.observation))
        });
        let admitted: Vec<TimestampedObservation> = order
            .iter()
            .filter_map(|index| positions.get(*index).cloned())
            .collect();
        Ok(Self {
            observed: admitted.len(),
            positions: admitted,
            origin: order,
            progress,
            asserted: progress,
            clock,
            clause_identity: clause_identity.into(),
        })
    }

    /// The number of positions that are observations; the rest are silent
    /// samples.
    pub fn observed(&self) -> usize {
        self.observed
    }

    /// The number of positions.
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// Whether the trace has no position; an admitted trace never has none.
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// The listing index of the observation at the canonical `position`: the
    /// observation a letter reads there is the one listed at that index.
    pub fn supplied_index(&self, position: TemporalPosition) -> Option<usize> {
        usize::try_from(position.index())
            .ok()
            .and_then(|index| self.origin.get(index).copied())
    }

    /// The admitted positions, in order.
    pub fn positions(&self) -> &[TimestampedObservation] {
        &self.positions
    }

    /// The progress asserted.
    pub fn progress(&self) -> &WindowProgress {
        &self.progress
    }

    /// The clock binding.
    pub fn clock(&self) -> &WindowClock {
        &self.clock
    }
}
