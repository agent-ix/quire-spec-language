// SPDX-License-Identifier: AGPL-3.0-or-later
//! The memory of a past operator (ADR-018 SM-6, IV-3): the state an automaton
//! keeps so a past operator needs no unrolling.
//!
//! An operator with lower bound `a` is the operator with lower bound 0 read
//! `a` positions earlier (IV-3). The lower-bound-0 part keeps a
//! [`PastAgeCounter`] for a closed interval and one flag for an open one; the
//! shift keeps the ages, among `1..=a`, at which the lower-bound-0 part held.
//! Ages are kept as inclusive ranges, so a lower bound as large as `u64::MAX`
//! costs one range.

use crate::automaton::counters::PastAgeCounter;
use crate::automaton::dnf::Dnf;
use crate::automaton::formula::{PastKind, Window};

/// The lower-bound-0 part of a past operator.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Base {
    /// An open interval: whether the operator holds.
    Flag(bool),
    /// A closed interval: the positions since the operand last held.
    Age(PastAgeCounter),
}

/// What a past operator remembers of the positions already read.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Memory {
    base: Base,
    /// The ages in `1..=lower` at which the lower-bound-0 part held, as
    /// disjoint inclusive ranges in ascending order.
    ages: Vec<(u64, u64)>,
}

/// One case of a past operator's step: the letters that cause it, whether
/// the operator holds at the position read, and its memory afterwards.
#[derive(Debug)]
pub(super) struct Outcome {
    pub(super) condition: Dnf,
    pub(super) holds: bool,
    pub(super) next: Memory,
}

/// A past operator's kind and interval.
#[derive(Clone, Copy, Debug)]
pub(super) struct Past {
    kind: PastKind,
    lower: u64,
    /// The upper bound of the lower-bound-0 part; `None` for an open
    /// interval.
    bound: Option<u64>,
}

impl Past {
    pub(super) fn new(kind: PastKind, window: Window) -> Self {
        match window {
            Window::Closed { lower, upper } => Self {
                kind,
                lower,
                bound: Some(upper.saturating_sub(lower)),
            },
            Window::From { lower } => Self {
                kind,
                lower,
                bound: None,
            },
        }
    }

    /// The memory in the stationary history before position 0, where every
    /// atom reads false and so every operand is constant (QSpec FR-092);
    /// `witness` is the constant value of the operand that witnesses the
    /// operator.
    pub(super) fn stationary(self, witness: bool) -> Memory {
        let base = match self.bound {
            Some(bound) if witness => Base::Age(PastAgeCounter::new(bound).step(true)),
            Some(bound) => Base::Age(PastAgeCounter::new(bound)),
            None => Base::Flag(witness),
        };
        let ages = if witness && self.lower != 0 {
            vec![(1, self.lower)]
        } else {
            Vec::new()
        };
        Memory { base, ages }
    }

    /// The memory after one position at which the guard (the left operand of
    /// `since`) and the witness hold or not, and whether the operator holds
    /// at that position.
    fn step(self, memory: &Memory, guard: bool, witness: bool) -> (bool, Memory) {
        let base = match (self.kind, memory.base) {
            (PastKind::Once, Base::Age(counter)) => Base::Age(counter.step(witness)),
            (PastKind::Once, Base::Flag(held)) => Base::Flag(held || witness),
            (PastKind::Since, Base::Age(counter)) => Base::Age(if witness {
                counter.step(true)
            } else if guard {
                counter.step(false)
            } else {
                // The guard broke every witness the counter was counting.
                self.bound.map_or(counter, PastAgeCounter::new)
            }),
            (PastKind::Since, Base::Flag(alive)) => Base::Flag(witness || (guard && alive)),
        };
        let part = match base {
            Base::Flag(holds) => holds,
            Base::Age(counter) => counter.within_bound(),
        };
        if self.lower == 0 {
            return (
                part,
                Memory {
                    base,
                    ages: Vec::new(),
                },
            );
        }
        let holds = memory
            .ages
            .iter()
            .any(|&(first, last)| first <= self.lower && self.lower <= last);
        let mut ages: Vec<(u64, u64)> = memory
            .ages
            .iter()
            .filter_map(|&(first, last)| {
                let first = first.checked_add(1)?;
                (first <= self.lower).then(|| (first, last.saturating_add(1).min(self.lower)))
            })
            .collect();
        if part {
            ages.insert(0, (1, 1));
        }
        (
            holds,
            Memory {
                base,
                ages: merged(ages),
            },
        )
    }

    /// Every case of the step from `memory`, given the conditions under
    /// which the guard and the witness hold; the cases partition the letters.
    pub(super) fn outcomes(
        self,
        memory: &Memory,
        guard: Option<&Dnf>,
        witness: &Dnf,
    ) -> Vec<Outcome> {
        let guards = match guard {
            Some(guard) => vec![(true, guard.clone()), (false, guard.negate())],
            None => vec![(true, Dnf::always())],
        };
        let witnesses = [(true, witness.clone()), (false, witness.negate())];
        let mut outcomes: Vec<Outcome> = Vec::new();
        for (guard_holds, guard_condition) in &guards {
            for (witness_holds, witness_condition) in &witnesses {
                let condition = guard_condition.and(witness_condition);
                if condition.is_never() {
                    continue;
                }
                let (holds, next) = self.step(memory, *guard_holds, *witness_holds);
                match outcomes
                    .iter_mut()
                    .find(|seen| seen.holds == holds && seen.next == next)
                {
                    Some(seen) => seen.condition = seen.condition.or(&condition),
                    None => outcomes.push(Outcome {
                        condition,
                        holds,
                        next,
                    }),
                }
            }
        }
        outcomes
    }
}

/// `ranges` as disjoint, non-adjacent inclusive ranges in ascending order.
fn merged(mut ranges: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
    ranges.sort_unstable();
    let mut result: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (first, last) in ranges {
        if let Some(previous) = result.last_mut() {
            if first <= previous.1.saturating_add(1) {
                previous.1 = previous.1.max(last);
                continue;
            }
        }
        result.push((first, last));
    }
    result
}
