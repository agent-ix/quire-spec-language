// SPDX-License-Identifier: AGPL-3.0-or-later
//! The timestamped trace and its progress (FR-501 Inputs).

use super::cover::ClockCover;
use crate::temporal::outcome::TemporalRefusal;
use crate::temporal::position::TemporalPosition;

/// The completeness assertion of a trace (FR-501 Inputs).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Completeness {
    /// Every observation stamped at or before `through` has been supplied.
    Complete {
        /// The last tick the assertion covers. It MAY exceed the watermark.
        through: u64,
    },
    /// No completeness is asserted.
    Incomplete,
}

impl Completeness {
    /// Whether the assertion covers every tick up to `tick`.
    pub(super) fn covers(self, tick: i128) -> bool {
        match self {
            Self::Complete { through } => i128::from(through) >= tick,
            Self::Incomplete => false,
        }
    }
}

/// The closure of a decision scope or of the surrounding execution.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Closure {
    /// Nothing more will be added.
    Closed,
    /// More may still be added.
    Open,
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

/// An order key: an order-authority identity and a key within it. Keys of
/// one authority compare; keys of different authorities do not (QSpec
/// FR-090-AC-4).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OrderKey {
    /// The order authority.
    pub authority: String,
    /// The key within the authority.
    pub key: u64,
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
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WindowProgress {
    /// The watermark `W`.
    pub watermark: Watermark,
    /// The completeness assertion.
    pub completeness: Completeness,
    /// The decision scope's closure.
    pub scope: Closure,
    /// The surrounding execution's closure.
    pub execution: Closure,
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

/// A non-empty timestamped trace in admitted order whose stamps do not
/// decrease (FR-501 Inputs).
#[derive(Clone, Debug)]
pub struct TimestampedTrace {
    pub(super) positions: Vec<TimestampedObservation>,
    origin: Vec<usize>,
    pub(super) progress: WindowProgress,
    pub(super) clock: WindowClock,
    pub(super) clause_identity: String,
}

impl TimestampedTrace {
    /// Admit `positions`, given in the order they were supplied. A trace with
    /// no position is [`TemporalRefusal::EmptyTrace`] and one whose stamps
    /// decrease is [`TemporalRefusal::DecreasingStamp`] at the first
    /// position below its predecessor, both before any position is visited.
    ///
    /// Positions of equal stamp whose keys all come from one order authority
    /// are admitted in key order, so the supply order of such a run changes
    /// nothing; a run without keys, or with keys of several authorities,
    /// keeps its supply order.
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
        for (index, pair) in positions.windows(2).enumerate() {
            if let [before, after] = pair {
                if after.stamp < before.stamp {
                    return Err(TemporalRefusal::DecreasingStamp {
                        position: TemporalPosition::new(
                            u64::try_from(index + 1).unwrap_or(u64::MAX),
                        ),
                    });
                }
            }
        }
        let mut start = 0;
        while start < order.len() {
            let stamp = positions.get(start).map(|position| position.stamp);
            let end = order
                .iter()
                .skip(start)
                .take_while(|index| positions.get(**index).map(|p| p.stamp) == stamp)
                .count()
                + start;
            if let Some(run) = order.get_mut(start..end) {
                let keyed =
                    |index: &usize| positions.get(*index).and_then(|p| p.order_key.as_ref());
                let authority = run.first().and_then(keyed).map(|key| &key.authority);
                let comparable = authority.is_some()
                    && run
                        .iter()
                        .all(|index| keyed(index).map(|key| &key.authority) == authority);
                if comparable {
                    run.sort_by_key(|index| keyed(index).map(|key| key.key));
                }
            }
            start = end.max(start + 1);
        }
        let admitted = order
            .iter()
            .filter_map(|index| positions.get(*index).cloned())
            .collect();
        Ok(Self {
            positions: admitted,
            origin: order,
            progress,
            clock,
            clause_identity: clause_identity.into(),
        })
    }

    /// The number of positions.
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// Whether the trace has no position; an admitted trace never has none.
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// The supply index of the admitted `position`: the observation a
    /// letter reads at an admitted position is the one supplied there.
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
