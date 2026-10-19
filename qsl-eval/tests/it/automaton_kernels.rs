// SPDX-License-Identifier: AGPL-3.0-or-later
//! The property-automaton kernels (ADR-018 SM-6, IV-3, IV-5, IV-6): the
//! generalized Büchi automaton, the deterministic bad-prefix monitor, state
//! numbering under the `max_automaton_states` bound, and the interval
//! counters. The counters are each checked against IV-3's expansion into
//! next-position steps on every trace of up to six positions.

use std::collections::BTreeSet;

use qsl_eval::automaton::{
    AlwaysOffsets, Atom, AutomatonError, DeadlineCounter, EventuallyWindows, Expired,
    ExtentCounter, Gba, GbaBuilder, Guard, Monitor, Numbering, PastAgeCounter, ShiftCounter,
    StateId, Violated, MAX_ATOMS,
};

use crate::support::automata::{atom, letter, negated_liveness};

/// A state of a four-state automaton that no one-state automaton has.
fn foreign_state() -> StateId {
    let mut builder = GbaBuilder::new();
    builder.add_state();
    builder.add_state();
    builder.add_state()
}

#[test]
fn a_guard_admits_the_letters_that_satisfy_its_literals() {
    let guard = Guard::always().requiring(atom(0)).forbidding(atom(1));
    assert!(guard.admits(letter(&[0])));
    assert!(guard.admits(letter(&[0, 2])));
    assert!(!guard.admits(letter(&[0, 1])));
    assert!(!guard.admits(letter(&[])));
    assert!(Guard::always().admits(letter(&[])));
    assert!(guard.is_satisfiable());
    assert!(!Guard::always()
        .requiring(atom(0))
        .forbidding(atom(0))
        .is_satisfiable());
}

#[test]
fn an_atom_beyond_the_letter_width_is_refused() {
    assert_eq!(
        Atom::new(MAX_ATOMS),
        Err(AutomatonError::AtomOutOfRange {
            index: MAX_ATOMS,
            max: MAX_ATOMS
        })
    );
    assert_eq!(Atom::new(MAX_ATOMS - 1).map(Atom::index), Ok(MAX_ATOMS - 1));
    assert!(letter(&[MAX_ATOMS - 1]).holds(atom(MAX_ATOMS - 1)));
    assert!(!letter(&[0]).holds(atom(1)));
}

#[test]
fn the_negated_liveness_automaton_reads_position_zero_into_its_documented_states() {
    let (gba, q0, q1) = negated_liveness();
    assert_eq!(gba.state_count(), 3);
    assert_eq!(gba.successors(Gba::INITIAL, letter(&[0])), Ok(vec![q0, q1]));
    assert_eq!(gba.successors(Gba::INITIAL, letter(&[])), Ok(vec![q0]));
    assert_eq!(gba.successors(q0, letter(&[0])), Ok(vec![q0, q1]));
    assert_eq!(gba.successors(q1, letter(&[0])), Ok(vec![q1]));
    assert_eq!(gba.successors(q1, letter(&[])), Ok(vec![]));
    assert_eq!(gba.acceptance_sets().len(), 1);
    assert!(gba.is_accepting_in(0, q1));
    assert!(!gba.is_accepting_in(0, q0));
    assert!(!gba.is_accepting_in(1, q1));
}

#[test]
fn an_automaton_refuses_a_state_it_does_not_have() {
    let (gba, _, _) = negated_liveness();
    let foreign = foreign_state();
    assert_eq!(
        gba.successors(foreign, letter(&[])),
        Err(AutomatonError::UnknownState { state: foreign })
    );
    let mut builder = GbaBuilder::new();
    assert_eq!(
        builder.add_transition(Gba::INITIAL, Guard::always(), foreign),
        Err(AutomatonError::UnknownState { state: foreign })
    );
    assert_eq!(
        builder.add_acceptance_set([foreign]),
        Err(AutomatonError::UnknownState { state: foreign })
    );
}

#[test]
fn transitions_are_kept_in_canonical_order_without_repeats() {
    let mut builder = GbaBuilder::new();
    let first = builder.add_state();
    let second = builder.add_state();
    for target in [second, first, second] {
        builder
            .add_transition(Gba::INITIAL, Guard::always(), target)
            .expect("both states were added");
    }
    let gba = builder.build();
    let targets: Vec<StateId> = gba
        .transitions(Gba::INITIAL)
        .expect("the initial state exists")
        .iter()
        .map(|transition| transition.target)
        .collect();
    assert_eq!(targets, vec![first, second]);
}

/// `always P` with atom 0 the literal `P`.
fn always_p() -> Gba {
    let p = Guard::always().requiring(atom(0));
    let mut builder = GbaBuilder::new();
    let ok = builder.add_state();
    builder
        .add_transition(Gba::INITIAL, p, ok)
        .expect("both states were added");
    builder
        .add_transition(ok, p, ok)
        .expect("both states were added");
    builder.build()
}

#[test]
fn the_monitor_of_always_rejects_exactly_the_bad_prefixes() {
    let mut monitor = Monitor::bad_prefix(always_p(), 16).expect("a safety automaton");
    let start = monitor.initial().expect("within the limit");
    assert_eq!(monitor.is_rejecting(start), Ok(false));
    let good = monitor.step(start, letter(&[0])).expect("within the limit");
    assert_eq!(monitor.is_rejecting(good), Ok(false));
    assert_eq!(monitor.step(good, letter(&[0])), Ok(good));
    let bad = monitor.step(good, letter(&[])).expect("within the limit");
    assert_eq!(monitor.is_rejecting(bad), Ok(true));
    assert_eq!(monitor.step(bad, letter(&[0])), Ok(bad));
    assert_eq!(monitor.step(bad, letter(&[])), Ok(bad));
    assert_eq!(monitor.state_count(), 3);
}

#[test]
fn the_monitor_determinizes_by_subset_construction() {
    // `eventually[0,1] P` on origin: P at position 0 or 1. Atom 0 is `P`.
    // After one miss the monitor is in {b}; a second miss is the bad prefix.
    let p = Guard::always().requiring(atom(0));
    let not_p = Guard::always().forbidding(atom(0));
    let mut builder = GbaBuilder::new();
    let done = builder.add_state();
    let waiting = builder.add_state();
    for (from, guard, to) in [
        (Gba::INITIAL, p, done),
        (Gba::INITIAL, not_p, waiting),
        (waiting, p, done),
        (done, Guard::always(), done),
    ] {
        builder
            .add_transition(from, guard, to)
            .expect("both states were added");
    }
    let mut monitor = Monitor::bad_prefix(builder.build(), 16).expect("a safety automaton");
    let start = monitor.initial().expect("within the limit");
    let miss = monitor.step(start, letter(&[])).expect("within the limit");
    assert_eq!(monitor.members(miss), Some(&BTreeSet::from([waiting])));
    let rescued = monitor.step(miss, letter(&[0])).expect("within the limit");
    assert_eq!(monitor.members(rescued), Some(&BTreeSet::from([done])));
    assert_eq!(monitor.step(start, letter(&[0])), Ok(rescued));
    let rejected = monitor.step(miss, letter(&[])).expect("within the limit");
    assert_eq!(monitor.members(rejected), Some(&BTreeSet::new()));
    assert_eq!(monitor.is_rejecting(rejected), Ok(true));
    assert_eq!(monitor.is_rejecting(rescued), Ok(false));
}

#[test]
fn the_monitor_trims_states_with_no_infinite_run() {
    // `P` then nothing: no infinite run exists, so the initial state is
    // already dead and the empty prefix is a bad prefix.
    let mut dead_end = GbaBuilder::new();
    let end = dead_end.add_state();
    dead_end
        .add_transition(Gba::INITIAL, Guard::always().requiring(atom(0)), end)
        .expect("both states were added");
    let mut monitor = Monitor::bad_prefix(dead_end.build(), 16).expect("a safety automaton");
    let start = monitor.initial().expect("within the limit");
    assert_eq!(monitor.is_rejecting(start), Ok(true));

    // A loop whose guard no letter satisfies is no run either.
    let mut impossible = GbaBuilder::new();
    impossible
        .add_transition(
            Gba::INITIAL,
            Guard::always().requiring(atom(0)).forbidding(atom(0)),
            Gba::INITIAL,
        )
        .expect("the initial state exists");
    let mut monitor = Monitor::bad_prefix(impossible.build(), 16).expect("a safety automaton");
    let start = monitor.initial().expect("within the limit");
    assert_eq!(monitor.is_rejecting(start), Ok(true));
}

#[test]
fn a_monitor_is_refused_for_an_automaton_with_acceptance_sets() {
    let (gba, _, _) = negated_liveness();
    assert_eq!(
        Monitor::bad_prefix(&gba, 16).err(),
        Some(AutomatonError::NotSafetyAutomaton { sets: 1 })
    );
}

#[test]
fn the_monitor_stops_at_the_state_limit_with_the_count_it_reached() {
    let mut monitor = Monitor::bad_prefix(always_p(), 2).expect("a safety automaton");
    let start = monitor.initial().expect("one state fits");
    let good = monitor.step(start, letter(&[0])).expect("two states fit");
    assert_eq!(
        monitor.step(good, letter(&[])),
        Err(AutomatonError::StateLimit { limit: 2 })
    );
    assert_eq!(monitor.state_count(), 2);
    // A state already numbered is still reachable at the limit.
    assert_eq!(monitor.step(good, letter(&[0])), Ok(good));

    let mut none = Monitor::bad_prefix(always_p(), 0).expect("a safety automaton");
    assert_eq!(none.initial(), Err(AutomatonError::StateLimit { limit: 0 }));
}

#[test]
fn a_monitor_refuses_a_state_it_did_not_number() {
    let mut monitor = Monitor::bad_prefix(always_p(), 16).expect("a safety automaton");
    monitor.initial().expect("within the limit");
    let foreign = foreign_state();
    assert_eq!(
        monitor.step(foreign, letter(&[])),
        Err(AutomatonError::UnknownState { state: foreign })
    );
    assert_eq!(
        monitor.is_rejecting(foreign),
        Err(AutomatonError::UnknownState { state: foreign })
    );
}

#[test]
fn numbering_assigns_first_reached_order_and_stops_at_its_limit() {
    let mut numbering = Numbering::new(3);
    let a = numbering.intern("a").expect("within the limit");
    let b = numbering.intern("b").expect("within the limit");
    let again = numbering.intern("a").expect("already numbered");
    let c = numbering.intern("c").expect("within the limit");
    assert_eq!((a.state.index(), a.created), (0, true));
    assert_eq!((b.state.index(), b.created), (1, true));
    assert_eq!((again.state, again.created), (a.state, false));
    assert_eq!((c.state.index(), c.created), (2, true));
    assert_eq!(
        numbering.intern("d"),
        Err(AutomatonError::StateLimit { limit: 3 })
    );
    assert_eq!(numbering.len(), 3);
    assert_eq!(numbering.key(b.state), Some(&"b"));
    assert!(!numbering.is_empty());
}

#[test]
fn a_numbering_of_limit_fifty_stops_at_exactly_fifty_states() {
    let mut numbering = Numbering::new(50);
    for key in 0u32..50 {
        numbering.intern(key).expect("within the limit");
    }
    assert_eq!(
        numbering.intern(50),
        Err(AutomatonError::StateLimit { limit: 50 })
    );
    assert_eq!(numbering.len(), 50);
}

const LONGEST: usize = 6;

/// Every `(opens, holds)` pair of vectors of `len` positions.
fn traces(len: usize) -> impl Iterator<Item = (Vec<bool>, Vec<bool>)> {
    (0u32..(1u32 << (2 * len))).map(move |bits| {
        let opens: Vec<bool> = (0..len).map(|i| ((bits >> i) & 1) == 1).collect();
        let holds: Vec<bool> = (0..len).map(|i| ((bits >> (len + i)) & 1) == 1).collect();
        (opens, holds)
    })
}

fn wide(n: usize) -> u64 {
    u64::try_from(n).expect("a small number")
}

/// IV-3: `eventually[a,b]` opened at `open` is expanded to a disjunction over
/// `[open + a, open + b]`. The first position at which such a window has
/// closed with the operand false at every position of it.
fn first_expiry(lower: usize, upper: usize, opens: &[bool], holds: &[bool]) -> Option<usize> {
    (0..opens.len()).find(|&end| {
        (0..=end).any(|open| {
            opens[open]
                && open + upper == end
                && !holds[open + lower..=end].iter().any(|held| *held)
        })
    })
}

/// Whether a window opened by some position is still undecided after the
/// last position: it ends later and no position of it so far held.
fn open_windows(lower: usize, upper: usize, opens: &[bool], holds: &[bool]) -> Vec<usize> {
    let last = opens.len() - 1;
    (0..=last)
        .filter(|&open| {
            opens[open]
                && open + upper > last
                && !holds[(open + lower).min(last + 1)..=last]
                    .iter()
                    .any(|held| *held)
        })
        .collect()
}

#[test]
fn a_deadline_counter_matches_the_expansion_of_eventually_on_every_short_trace() {
    for upper in 0..=3usize {
        for len in 1..=LONGEST {
            for (opens, holds) in traces(len) {
                let mut counter = DeadlineCounter::new(wide(upper));
                let mut expired_at = None;
                for (position, (open, held)) in opens.iter().zip(&holds).enumerate() {
                    match counter.step(*open, *held) {
                        Ok(next) => counter = next,
                        Err(Expired) => {
                            expired_at = Some(position);
                            break;
                        }
                    }
                }
                assert_eq!(expired_at, first_expiry(0, upper, &opens, &holds));
                if expired_at.is_none() {
                    let last = len - 1;
                    let remaining = open_windows(0, upper, &opens, &holds)
                        .into_iter()
                        .map(|open| open + upper - last)
                        .min()
                        .unwrap_or(0);
                    assert_eq!(counter.remaining(), wide(remaining));
                    assert_eq!(counter.is_pending(), remaining != 0);
                }
            }
        }
    }
}

#[test]
fn eventually_windows_match_the_expansion_for_every_interval_on_every_short_trace() {
    for upper in 0..=3usize {
        for lower in 0..=upper {
            for len in 1..=LONGEST {
                for (opens, holds) in traces(len) {
                    let mut windows = EventuallyWindows::new(wide(lower), wide(upper))
                        .expect("lower does not exceed upper");
                    let mut expired_at = None;
                    for (position, (open, held)) in opens.iter().zip(&holds).enumerate() {
                        match windows.step(*open, *held) {
                            Ok(next) => windows = next,
                            Err(Expired) => {
                                expired_at = Some(position);
                                break;
                            }
                        }
                    }
                    assert_eq!(expired_at, first_expiry(lower, upper, &opens, &holds));
                    if expired_at.is_none() {
                        assert_eq!(
                            windows.is_pending(),
                            !open_windows(lower, upper, &opens, &holds).is_empty()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn eventually_windows_drop_a_window_that_contains_another() {
    let mut windows = EventuallyWindows::new(0, 3).expect("lower does not exceed upper");
    windows = windows.step(true, false).expect("the window is open");
    assert_eq!(windows.windows(), &BTreeSet::from([(0, 2)]));
    windows = windows.step(true, false).expect("both windows are open");
    // The window opened now reads (0,2) after this position and the one
    // opened before reads (0,1); the wider contains the narrower and is
    // dropped.
    assert_eq!(windows.windows(), &BTreeSet::from([(0, 1)]));
    let aged = windows
        .step(false, false)
        .expect("the window still has a position");
    assert_eq!(aged.windows(), &BTreeSet::from([(0, 0)]));
    assert!(matches!(aged.step(false, false), Err(Expired)));
}

/// The first position at which an `always[a,b]` obligation opened earlier
/// requires the operand and it is false.
fn first_violation(lower: usize, upper: usize, opens: &[bool], holds: &[bool]) -> Option<usize> {
    (0..opens.len()).find(|&at| {
        !holds[at] && (0..=at).any(|open| opens[open] && open + lower <= at && at <= open + upper)
    })
}

/// The obligations still requiring a later position after the last one.
fn requiring_later(upper: usize, opens: &[bool]) -> Vec<usize> {
    let last = opens.len() - 1;
    (0..=last)
        .filter(|&open| opens[open] && open + upper > last)
        .collect()
}

#[test]
fn an_extent_counter_matches_the_expansion_of_always_on_every_short_trace() {
    for upper in 0..=3usize {
        for len in 1..=LONGEST {
            for (opens, holds) in traces(len) {
                let mut counter = ExtentCounter::new(wide(upper));
                let mut violated_at = None;
                for (position, (open, held)) in opens.iter().zip(&holds).enumerate() {
                    match counter.step(*open, *held) {
                        Ok(next) => counter = next,
                        Err(Violated) => {
                            violated_at = Some(position);
                            break;
                        }
                    }
                }
                assert_eq!(violated_at, first_violation(0, upper, &opens, &holds));
                if violated_at.is_none() {
                    let last = len - 1;
                    let furthest = requiring_later(upper, &opens)
                        .into_iter()
                        .map(|open| open + upper - last)
                        .max()
                        .unwrap_or(0);
                    assert_eq!(counter.remaining(), wide(furthest));
                }
            }
        }
    }
}

#[test]
fn always_offsets_match_the_expansion_for_every_interval_on_every_short_trace() {
    for upper in 0..=3usize {
        for lower in 0..=upper {
            for len in 1..=LONGEST {
                for (opens, holds) in traces(len) {
                    let mut offsets = AlwaysOffsets::new(wide(lower), wide(upper))
                        .expect("lower does not exceed upper");
                    let mut violated_at = None;
                    for (position, (open, held)) in opens.iter().zip(&holds).enumerate() {
                        match offsets.step(*open, *held) {
                            Ok(next) => offsets = next,
                            Err(Violated) => {
                                violated_at = Some(position);
                                break;
                            }
                        }
                    }
                    assert_eq!(violated_at, first_violation(lower, upper, &opens, &holds));
                    if violated_at.is_none() {
                        assert_eq!(
                            offsets.is_pending(),
                            !requiring_later(upper, &opens).is_empty()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn always_offsets_hold_an_upper_bound_of_u64_max_as_one_range() {
    let offsets = AlwaysOffsets::new(0, u64::MAX).expect("lower does not exceed upper");
    let opened = offsets.step(true, true).expect("the operand holds");
    assert_eq!(opened.required(), &[(0, u64::MAX - 1)]);
    assert_eq!(opened.step(false, false), Err(Violated));
}

#[test]
fn a_past_age_counter_counts_positions_since_the_operand_held() {
    for bound in 0..=3usize {
        for len in 1..=LONGEST {
            for (_, holds) in traces(len) {
                let mut counter = PastAgeCounter::new(wide(bound));
                for (position, held) in holds.iter().enumerate() {
                    counter = counter.step(*held);
                    let since = (0..=position)
                        .rev()
                        .find(|&earlier| holds[earlier])
                        .map(|earlier| position - earlier)
                        .filter(|since| *since <= bound);
                    assert_eq!(counter.age(), since.map(wide));
                    assert_eq!(counter.within_bound(), since.is_some());
                }
            }
        }
    }
}

#[test]
fn a_shift_counter_opens_after_exactly_its_lower_bound() {
    let mut shift = ShiftCounter::new(2);
    assert!(!shift.is_open());
    shift = shift.advance();
    assert!(!shift.is_open());
    shift = shift.advance();
    assert!(shift.is_open());
    assert!(shift.advance().is_open());
    assert!(ShiftCounter::new(0).is_open());
}

#[test]
fn counters_report_the_value_counts_of_ir_5() {
    assert_eq!(DeadlineCounter::new(10).value_count(), 11);
    assert_eq!(ExtentCounter::new(10).value_count(), 11);
    assert_eq!(PastAgeCounter::new(10).value_count(), 12);
    assert_eq!(ShiftCounter::new(4).value_count(), 5);
    assert_eq!(
        PastAgeCounter::new(u64::MAX).value_count(),
        u128::from(u64::MAX) + 2
    );
    assert_eq!(
        DeadlineCounter::new(u64::MAX).value_count(),
        u128::from(u64::MAX) + 1
    );
}

#[test]
fn an_interval_whose_lower_bound_exceeds_its_upper_bound_is_refused() {
    let inverted = AutomatonError::InvertedInterval { lower: 3, upper: 2 };
    assert_eq!(EventuallyWindows::new(3, 2), Err(inverted));
    assert_eq!(AlwaysOffsets::new(3, 2), Err(inverted));
}
