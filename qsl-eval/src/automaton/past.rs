// SPDX-License-Identifier: AGPL-3.0-or-later
//! The memory of a past operator (ADR-018 SM-6, IV-3): the state an automaton
//! keeps so a past operator needs no unrolling.
//!
//! An operator with lower bound `a` is the operator with lower bound 0 read
//! `a` steps earlier (IV-3): at the last position whose distance back is
//! exactly `a`. The lower-bound-0 part keeps a [`PastAgeCounter`] for a closed
//! interval and one flag for an open one; the shift keeps the distances,
//! among `1..=a`, at which the lower-bound-0 part held at the last position
//! of the run of positions that distance back. Distances are measured from
//! the last position read and are kept as inclusive ranges, so a lower bound
//! as large as `u64::MAX` costs one range.
//!
//! A step that does not count (ADR-027 PB-3) leaves every distance where it
//! was: the new position joins the run of the position before it, the age
//! counter holds unless the operand is read, and the shift does not move.
//! When every step counts this is ADR-018's reading.

use crate::automaton::counters::PastAgeCounter;
use crate::automaton::dnf::Dnf;
use crate::automaton::formula::{PastKind, Window};
use crate::automaton::key::SubjectKind;
use crate::automaton::key_state::{DistanceBucket, HistoryOperand, PastMemory, PastOperator};
use crate::simulation::model::encode::Enc;

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
    /// The distances back from the last position read, in `1..=lower`, at
    /// which the lower-bound-0 part held at the last position of the run
    /// that far back, as disjoint inclusive ranges in ascending order.
    ages: Vec<(u64, u64)>,
    /// Run-length encoded operand truth assignments retained for FR-338's
    /// structural past key. Distances are from the next position for
    /// ordinary history entries.
    pub(super) history: Vec<(u64, u64, bool, bool)>,
    /// Composed one-position updates by counted distance for protocol
    /// history entries. Distances are from the current distance-zero region.
    pub(super) buckets: Vec<(u64, u64, [bool; 2])>,
    /// The recurrence result before the oldest retained open-shift bucket.
    pub(super) tail: bool,
}

impl Memory {
    /// Canonical state data, independent of the transition system's
    /// materialization order.
    pub(super) fn key_value(&self) -> Enc {
        let base = match self.base {
            Base::Flag(value) => Enc::Array(vec![Enc::text("flag"), Enc::Bool(value)]),
            Base::Age(counter) => Enc::Array(vec![
                Enc::text("age"),
                counter
                    .age()
                    .map_or(Enc::Null, |age| Enc::text(age.to_string())),
            ]),
        };
        let ranges = |ranges: &[(u64, u64)]| {
            Enc::Array(
                ranges
                    .iter()
                    .map(|(first, last)| {
                        Enc::Array(vec![
                            Enc::text(first.to_string()),
                            Enc::text(last.to_string()),
                        ])
                    })
                    .collect(),
            )
        };
        let history = Enc::Array(
            self.history
                .iter()
                .map(|(first, last, guard, witness)| {
                    Enc::Array(vec![
                        Enc::text(first.to_string()),
                        Enc::text(last.to_string()),
                        Enc::Bool(*guard),
                        Enc::Bool(*witness),
                    ])
                })
                .collect(),
        );
        let buckets = Enc::Array(
            self.buckets
                .iter()
                .map(|(first, last, transform)| {
                    Enc::Array(vec![
                        Enc::text(first.to_string()),
                        Enc::text(last.to_string()),
                        Enc::Array(transform.iter().copied().map(Enc::Bool).collect()),
                    ])
                })
                .collect(),
        );
        Enc::Array(vec![
            base,
            ranges(&self.ages),
            history,
            buckets,
            Enc::Bool(self.tail),
        ])
    }
}

/// One case of a past operator's step: the letters that cause it, whether
/// the operator holds at the position read, and its memory afterwards.
#[derive(Debug)]
pub(super) struct Outcome {
    pub(super) condition: Dnf,
    pub(super) holds: bool,
    pub(super) guard: bool,
    pub(super) witness: bool,
    pub(super) next: Memory,
}

/// A past operator's kind and interval.
#[derive(Clone, Copy, Debug)]
pub(super) struct Past {
    kind: PastKind,
    subject: SubjectKind,
    lower: u64,
    /// The upper bound of the lower-bound-0 part; `None` for an open
    /// interval.
    bound: Option<u64>,
    /// The greatest retained distance for FR-338's general history forms.
    retained: Option<u64>,
}

impl Past {
    pub(super) fn new(kind: PastKind, window: Window, subject: SubjectKind) -> Self {
        match window {
            Window::Closed { lower, upper } => Self {
                kind,
                subject,
                lower,
                bound: Some(upper.saturating_sub(lower)),
                retained: (lower != 0).then_some(upper),
            },
            Window::From { lower } => Self {
                kind,
                subject,
                lower,
                bound: None,
                retained: (lower != 0).then_some(lower),
            },
        }
    }

    /// The memory in the stationary history before position 0, where every
    /// atom reads false and so every operand is constant (QSpec FR-092);
    /// `witness` is the constant value of the operand that witnesses the
    /// operator.
    pub(super) fn stationary(self, guard: bool, witness: bool) -> Memory {
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
        let mut history = Vec::new();
        let mut buckets = Vec::new();
        if let Some(retained) = self.retained {
            if self.subject == SubjectKind::Ordinary {
                if guard || witness {
                    history.push((1, retained, guard, witness));
                }
            } else {
                let transform = self.transform(guard, witness);
                if transform != [false, true] {
                    buckets.push((0, retained, transform));
                }
            }
        }
        let tail = match base {
            Base::Flag(holds) => holds,
            Base::Age(counter) => counter.within_bound(),
        };
        Memory {
            base,
            ages,
            history,
            buckets,
            tail,
        }
    }

    /// The memory after one position at which the guard (the left operand of
    /// `since`) and the witness hold or not, reached by a step that
    /// `counted` or not, and whether the operator holds at that position.
    fn step(self, memory: &Memory, guard: bool, witness: bool, counted: bool) -> (bool, Memory) {
        let previous = match memory.base {
            Base::Flag(held) => held,
            Base::Age(counter) => counter.within_bound(),
        };
        let base = match (self.kind, memory.base) {
            (PastKind::Once, Base::Age(counter)) => {
                Base::Age(counter.step_counted(witness, counted))
            }
            (PastKind::Once, Base::Flag(held)) => Base::Flag(held || witness),
            (PastKind::Since, Base::Age(counter)) => Base::Age(if witness {
                counter.step_counted(true, counted)
            } else if guard {
                counter.step_counted(false, counted)
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
                    history: memory.history.clone(),
                    buckets: memory.buckets.clone(),
                    tail: memory.tail,
                },
            );
        }
        let ages = if counted {
            // The run the last position ended is now one step back.
            let mut ages: Vec<(u64, u64)> = memory
                .ages
                .iter()
                .filter_map(|&(first, last)| {
                    let first = first.checked_add(1)?;
                    (first <= self.lower).then(|| (first, last.saturating_add(1).min(self.lower)))
                })
                .collect();
            if previous {
                ages.insert(0, (1, 1));
            }
            merged(ages)
        } else {
            memory.ages.clone()
        };
        let mut history = memory.history.clone();
        let mut buckets = memory.buckets.clone();
        let mut tail = memory.tail;
        if let Some(retained) = self.retained {
            if self.subject == SubjectKind::Ordinary && counted {
                for (first, last, _, _) in &mut history {
                    *first = first.saturating_add(1);
                    *last = last.saturating_add(1);
                }
                history.retain(|(first, _, _, _)| *first <= retained);
                for (_, last, _, _) in &mut history {
                    *last = (*last).min(retained);
                }
                if guard || witness {
                    history.push((1, 1, guard, witness));
                }
                history = merged_history(history);
            }
            if self.subject == SubjectKind::Protocol && counted {
                for (first, last, _) in &mut buckets {
                    *first = first.saturating_add(1);
                    *last = last.saturating_add(1);
                }
                let evicted: Vec<_> = buckets
                    .iter()
                    .filter(|(_, last, _)| *last > retained)
                    .map(|(_, _, transform)| *transform)
                    .collect();
                buckets.retain(|(first, _, _)| *first <= retained);
                for (_, last, _) in &mut buckets {
                    *last = (*last).min(retained);
                }
                for transform in evicted {
                    tail = apply(transform, tail);
                }
                let transform = self.transform(guard, witness);
                if transform != [false, true] {
                    buckets.insert(0, (0, 0, transform));
                }
                buckets = merged_buckets(buckets);
            } else if self.subject == SubjectKind::Protocol {
                let transform = self.transform(guard, witness);
                if let Some((index, &(_, last, previous))) = buckets
                    .iter()
                    .enumerate()
                    .find(|(_, (first, _, _))| *first == 0)
                {
                    let composed = compose(transform, previous);
                    buckets.remove(index);
                    if composed != [false, true] {
                        buckets.insert(0, (0, 0, composed));
                    }
                    if last > 0 {
                        buckets.push((1, last, previous));
                    }
                    buckets = merged_buckets(buckets);
                } else if transform != [false, true] {
                    buckets.insert(0, (0, 0, transform));
                }
            }
        }
        let holds = ages
            .iter()
            .any(|&(first, last)| first <= self.lower && self.lower <= last);
        (
            holds,
            Memory {
                base,
                ages,
                history,
                buckets,
                tail,
            },
        )
    }

    fn transform(self, guard: bool, witness: bool) -> [bool; 2] {
        match self.kind {
            PastKind::Once => [witness, true],
            PastKind::Since => [witness, witness || guard],
        }
    }

    /// Every case of the step from `memory`, given the conditions under
    /// which the guard and the witness hold; the cases partition the letters.
    pub(super) fn outcomes(
        self,
        memory: &Memory,
        guard: Option<&Dnf>,
        witness: &Dnf,
        counted: bool,
    ) -> Vec<Outcome> {
        let guards = match guard {
            Some(guard) => vec![(true, guard.clone()), (false, guard.negate())],
            None => vec![(true, Dnf::always())],
        };
        let witnesses = [(true, witness.clone()), (false, witness.negate())];
        let records_history =
            self.subject == SubjectKind::Ordinary && self.lower != 0 && self.retained.is_some();
        let mut outcomes: Vec<Outcome> = Vec::new();
        for (guard_holds, guard_condition) in &guards {
            for (witness_holds, witness_condition) in &witnesses {
                let condition = guard_condition.and(witness_condition);
                if condition.is_never() {
                    continue;
                }
                let (holds, next) = self.step(memory, *guard_holds, *witness_holds, counted);
                match outcomes.iter_mut().find(|seen| {
                    seen.holds == holds
                        && (!records_history
                            || (seen.guard == *guard_holds && seen.witness == *witness_holds))
                        && seen.next == next
                }) {
                    Some(seen) => seen.condition = seen.condition.or(&condition),
                    None => outcomes.push(Outcome {
                        condition,
                        holds,
                        guard: *guard_holds,
                        witness: *witness_holds,
                        next,
                    }),
                }
            }
        }
        outcomes
    }

    /// Encode this recurrence's history using FR-338's path-addressed form.
    pub(super) fn key_memory(
        self,
        memory: &Memory,
        path: &[u64],
        operator: PastOperator,
        subject: SubjectKind,
        max_key_bytes: u64,
    ) -> Result<PastMemory, crate::automaton::error::AutomatonError> {
        let dual = matches!(
            operator,
            PastOperator::Historically | PastOperator::Triggered
        );
        let source_truth = |value: bool| if dual { !value } else { value };
        let operand_paths = || {
            let positions = match operator {
                PastOperator::Once | PastOperator::Historically => &[0][..],
                PastOperator::Since | PastOperator::Triggered => &[0, 1][..],
            };
            positions
                .iter()
                .map(|position| {
                    let mut operand = path.to_vec();
                    operand.push(*position);
                    operand
                })
                .collect::<Vec<_>>()
        };
        let make_operands = |history: &[(u64, u64, bool, bool)]| {
            let paths = operand_paths();
            paths
                .into_iter()
                .enumerate()
                .map(|(index, path)| {
                    let operand_value = |guard: bool, witness: bool| {
                        if index == 0
                            && matches!(operator, PastOperator::Since | PastOperator::Triggered)
                        {
                            guard
                        } else {
                            witness
                        }
                    };
                    let internal_true: Vec<_> = history
                        .iter()
                        .filter_map(|(first, last, guard, witness)| {
                            operand_value(*guard, *witness).then_some((*first, *last))
                        })
                        .collect();
                    let true_count: u128 = internal_true
                        .iter()
                        .map(|(first, last)| u128::from(last - first) + 1)
                        .sum();
                    let retained = u128::from(self.retained.unwrap_or(0));
                    let count = if dual {
                        retained.saturating_sub(true_count)
                    } else {
                        true_count
                    };
                    // Every offset is a quoted decimal string (at least 3
                    // octets) and every following offset adds a comma.
                    // Refuse before materializing an impossible key.
                    if count.saturating_mul(4).saturating_add(1) > u128::from(max_key_bytes) {
                        return Err(crate::automaton::error::AutomatonError::KeyEncodingLimit {
                            limit: max_key_bytes,
                        });
                    }
                    let offsets = if dual {
                        let mut offsets =
                            Vec::with_capacity(usize::try_from(count).unwrap_or_default());
                        let mut next = 1;
                        for (first, last) in &internal_true {
                            if next < *first {
                                offsets.extend(next..*first);
                            }
                            next = last.saturating_add(1);
                        }
                        if next <= self.retained.unwrap_or(0) {
                            offsets.extend(next..=self.retained.unwrap_or(0));
                        }
                        offsets
                    } else {
                        internal_true
                            .iter()
                            .flat_map(|(first, last)| *first..=*last)
                            .collect()
                    };
                    Ok(HistoryOperand { path, offsets })
                })
                .collect::<Result<Vec<_>, _>>()
        };
        let source_transform = |transform: [bool; 2]| {
            if dual {
                [!transform[1], !transform[0]]
            } else {
                transform
            }
        };

        match (self.lower, self.bound, subject) {
            (0, None, _) => Ok(PastMemory::Flag {
                path: path.to_vec(),
                operator,
                value: source_truth(memory.value()),
            }),
            (0, Some(bound), _) => {
                let age = memory.age().map_or(u128::from(bound) + 1, u128::from);
                Ok(PastMemory::Age {
                    path: path.to_vec(),
                    operator,
                    age,
                })
            }
            (_, Some(_), SubjectKind::Ordinary) => Ok(PastMemory::History {
                path: path.to_vec(),
                operands: make_operands(&memory.history)?,
            }),
            (lower, None, SubjectKind::Ordinary) => Ok(PastMemory::ShiftHistory {
                path: path.to_vec(),
                operands: make_operands(&memory.history)?,
                tail: source_truth(memory.age_at(lower)),
            }),
            (_, Some(_), SubjectKind::Protocol) => Ok(PastMemory::DistanceHistory {
                path: path.to_vec(),
                buckets: key_buckets(&memory.buckets, source_transform, max_key_bytes)?,
                tail: None,
            }),
            (_, None, SubjectKind::Protocol) => Ok(PastMemory::DistanceHistory {
                path: path.to_vec(),
                buckets: key_buckets(&memory.buckets, source_transform, max_key_bytes)?,
                tail: Some(source_truth(memory.tail)),
            }),
        }
    }
}

impl Memory {
    fn value(&self) -> bool {
        match self.base {
            Base::Flag(value) => value,
            Base::Age(counter) => counter.within_bound(),
        }
    }

    fn age(&self) -> Option<u64> {
        match self.base {
            Base::Age(counter) => counter.age(),
            Base::Flag(_) => None,
        }
    }

    fn age_at(&self, distance: u64) -> bool {
        self.ages
            .iter()
            .any(|(first, last)| *first <= distance && distance <= *last)
    }
}

fn apply(transform: [bool; 2], input: bool) -> bool {
    transform[usize::from(input)]
}

/// Compose the newer update after the older one.
fn compose(newer: [bool; 2], older: [bool; 2]) -> [bool; 2] {
    [apply(newer, older[0]), apply(newer, older[1])]
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

/// Merge adjacent ordinary-history runs with equal operand truths.
fn merged_history(mut runs: Vec<(u64, u64, bool, bool)>) -> Vec<(u64, u64, bool, bool)> {
    runs.sort_unstable_by_key(|(first, _, guard, witness)| (*guard, *witness, *first));
    let mut result: Vec<(u64, u64, bool, bool)> = Vec::with_capacity(runs.len());
    for (first, last, guard, witness) in runs {
        if let Some((_, previous_last, previous_guard, previous_witness)) = result.last_mut() {
            if *previous_guard == guard
                && *previous_witness == witness
                && first <= previous_last.saturating_add(1)
            {
                *previous_last = (*previous_last).max(last);
                continue;
            }
        }
        result.push((first, last, guard, witness));
    }
    result.sort_unstable_by_key(|(first, _, _, _)| *first);
    result
}

/// Merge adjacent protocol-distance buckets with the same transformation.
fn merged_buckets(mut buckets: Vec<(u64, u64, [bool; 2])>) -> Vec<(u64, u64, [bool; 2])> {
    buckets.sort_unstable_by_key(|(first, _, _)| *first);
    let mut result: Vec<(u64, u64, [bool; 2])> = Vec::with_capacity(buckets.len());
    for (first, last, transform) in buckets {
        if let Some((_, previous_last, previous_transform)) = result.last_mut() {
            if *previous_transform == transform && first <= previous_last.saturating_add(1) {
                *previous_last = (*previous_last).max(last);
                continue;
            }
        }
        result.push((first, last, transform));
    }
    result
}

/// Expand compressed protocol buckets only if their canonical form can fit.
fn key_buckets(
    runs: &[(u64, u64, [bool; 2])],
    source_transform: impl Fn([bool; 2]) -> [bool; 2],
    max_key_bytes: u64,
) -> Result<Vec<DistanceBucket>, crate::automaton::error::AutomatonError> {
    let count: u128 = runs
        .iter()
        .map(|(first, last, _)| u128::from(last - first) + 1)
        .sum();
    // `["0",[false,true]]` is 18 bytes; the outer array also needs its
    // separators. This lower bound avoids expanding an impossible key.
    if count.saturating_mul(18).saturating_add(2) > u128::from(max_key_bytes) {
        return Err(crate::automaton::error::AutomatonError::KeyEncodingLimit {
            limit: max_key_bytes,
        });
    }
    Ok(runs
        .iter()
        .flat_map(|(first, last, transform)| {
            (*first..=*last).map(|distance| DistanceBucket {
                distance,
                transform: source_transform(*transform),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automaton::key_state::{HistoryOperand, PastOperator};

    #[test]
    fn a_historically_key_complements_the_internal_once_history() {
        let past = Past::new(
            PastKind::Once,
            Window::Closed { lower: 1, upper: 2 },
            SubjectKind::Ordinary,
        );
        let memory = Memory {
            base: Base::Age(PastAgeCounter::new(1)),
            ages: Vec::new(),
            history: vec![(2, 2, false, true)],
            buckets: Vec::new(),
            tail: false,
        };

        let PastMemory::History { operands, .. } = past
            .key_memory(
                &memory,
                &[4],
                PastOperator::Historically,
                SubjectKind::Ordinary,
                1024,
            )
            .expect("the structural key is within its budget")
        else {
            panic!("a finite ordinary past maps to operand history");
        };
        assert_eq!(
            operands,
            vec![HistoryOperand {
                path: vec![4, 0],
                offsets: vec![1],
            }]
        );
    }

    #[test]
    fn a_past_key_refuses_a_dense_offset_set_before_expanding_it() {
        let past = Past::new(
            PastKind::Once,
            Window::Closed {
                lower: 1,
                upper: u64::MAX,
            },
            SubjectKind::Ordinary,
        );
        let memory = Memory {
            base: Base::Age(PastAgeCounter::new(1)),
            ages: Vec::new(),
            history: Vec::new(),
            buckets: Vec::new(),
            tail: false,
        };
        assert_eq!(
            past.key_memory(
                &memory,
                &[4],
                PastOperator::Historically,
                SubjectKind::Ordinary,
                1024,
            ),
            Err(crate::automaton::error::AutomatonError::KeyEncodingLimit { limit: 1024 })
        );
    }

    #[test]
    fn a_protocol_past_key_refuses_dense_distance_buckets_before_expanding_them() {
        let past = Past::new(
            PastKind::Once,
            Window::Closed {
                lower: 1,
                upper: u64::MAX,
            },
            SubjectKind::Protocol,
        );
        let memory = Memory {
            base: Base::Age(PastAgeCounter::new(1)),
            ages: Vec::new(),
            history: Vec::new(),
            buckets: vec![(0, u64::MAX, [true, true])],
            tail: false,
        };
        assert_eq!(
            past.key_memory(
                &memory,
                &[4],
                PastOperator::Once,
                SubjectKind::Protocol,
                1024,
            ),
            Err(crate::automaton::error::AutomatonError::KeyEncodingLimit { limit: 1024 })
        );
    }

    #[test]
    fn stationary_history_compresses_a_u64_max_window() {
        let ordinary = Past::new(
            PastKind::Once,
            Window::Closed {
                lower: 1,
                upper: u64::MAX,
            },
            SubjectKind::Ordinary,
        )
        .stationary(false, true);
        assert_eq!(ordinary.history, vec![(1, u64::MAX, false, true)]);

        let protocol = Past::new(
            PastKind::Once,
            Window::Closed {
                lower: 1,
                upper: u64::MAX,
            },
            SubjectKind::Protocol,
        )
        .stationary(false, true);
        assert_eq!(protocol.buckets, vec![(0, u64::MAX, [true, true])]);
    }
}
