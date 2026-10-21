// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 SM-6 and IV-3: a checked temporal clause translates to a property
//! automaton, and the automaton answers to the trace evaluator (SM-1, SM-7).
//!
//! Each clause is real complete-V1 source taken through S1, S2, the assembler
//! and the `TemporalTrace` family's check. A clause's automaton is compared
//! with [`evaluate`] on every short word: a closed finite trace under a
//! bounded profile, a lasso under infinite-trace. A bounded or safety
//! automaton accepts the word exactly when the clause holds on it; a liveness
//! automaton is the automaton of the negation, so it accepts exactly when the
//! clause does not.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ix_trace_rs::trace;
use qsl_eval::automaton::{
    translate, translate_negation, Atom, AtomSymmetry, AutomatonError, AutomatonKind, Gba, Letter,
    Monitor, PropertyAutomaton, StateId,
};
use qsl_eval::temporal::{
    evaluate, AtomReading, LetterStop, TemporalLetters, TemporalOutcome, TemporalPosition,
    TemporalTrace,
};
use qsl_semantics::check::{FormulaNodeId, SCALAR_LIMITS_UNLIMITED};
use quire_exact::Meter;

use crate::temporal_trace::{fixture, Fixture, EVENT, INFINITE};

const LIMIT: u64 = 1 << 16;
const WHEN: &str = "on each (q: Counter) when (true)";
const EACH: &str = "on each (q: Counter)";
const ORIGIN: &str = "on origin";

/// A clause, its automaton and the letter it reads. `gba` is the explicit
/// automaton of every reachable state, which the oracle batteries compare;
/// a product reaches states lazily instead.
struct Subject {
    body: String,
    fixture: Fixture,
    property: PropertyAutomaton,
    gba: Gba,
}

fn subject_with(profile: &str, activation: &str, body: &str) -> Subject {
    let fixture = fixture(profile, "c: Counter", activation, body);
    let mut property =
        translate(&fixture.clause, LIMIT).unwrap_or_else(|why| panic!("{body}: {why}"));
    let gba = property
        .explore()
        .unwrap_or_else(|why| panic!("{body}: {why}"));
    Subject {
        body: body.to_owned(),
        fixture,
        property,
        gba,
    }
}

fn infinite(body: &str) -> Subject {
    subject_with(INFINITE, ORIGIN, body)
}

fn bounded(body: &str) -> Subject {
    subject_with(EVENT, ORIGIN, body)
}

fn translation_of(body: &str) -> Result<PropertyAutomaton, AutomatonError> {
    translate(&fixture(INFINITE, "c: Counter", ORIGIN, body).clause, LIMIT)
}

fn atom(index: usize) -> Atom {
    Atom::new(index).expect("the index is below the letter width")
}

/// The letters the evaluator reads out of a word.
struct WordLetters<'a> {
    property: &'a PropertyAutomaton,
    word: &'a [Letter],
}

impl WordLetters<'_> {
    fn letter(&self, position: TemporalPosition) -> Letter {
        let index = usize::try_from(position.index()).expect("a small position");
        *self.word.get(index).expect("the position is represented")
    }
}

impl TemporalLetters for WordLetters<'_> {
    fn atom(
        &mut self,
        node: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let index = self
            .property
            .atoms()
            .iter()
            .position(|known| *known == node)
            .expect("the evaluator reads only the clause's atoms");
        Ok(reading(self.letter(position).holds(atom(index))))
    }

    fn activation(
        &mut self,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let when = self.property.when().expect("the clause has a `when`");
        Ok(reading(self.letter(position).holds(when)))
    }
}

fn reading(holds: bool) -> AtomReading {
    if holds {
        AtomReading::True
    } else {
        AtomReading::False
    }
}

/// Whether the clause holds on `trace` over `word`, by the evaluator.
fn evaluator_holds(subject: &Subject, word: &[Letter], trace: TemporalTrace) -> bool {
    let mut letters = WordLetters {
        property: &subject.property,
        word,
    };
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    match evaluate(&subject.fixture.clause, trace, &mut letters, &mut meter) {
        Ok(TemporalOutcome::Holds) => true,
        Ok(TemporalOutcome::Violated { .. }) => false,
        other => panic!("{}: the evaluator settled {other:?}", subject.body),
    }
}

/// Whether `gba` accepts the lasso `word`, whose loop starts at `prefix`: a
/// reachable cycle of the product visits every acceptance set.
fn accepts(gba: &Gba, word: &[Letter], prefix: usize) -> bool {
    let after = |position: usize| {
        if position + 1 == word.len() {
            prefix
        } else {
            position + 1
        }
    };
    let mut numbers: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let mut nodes = Vec::new();
    let mut edges: Vec<Vec<usize>> = Vec::new();
    let mut queue = VecDeque::new();
    numbers.insert((0, 0), 0);
    nodes.push((Gba::INITIAL, 0));
    edges.push(Vec::new());
    queue.push_back(0);
    while let Some(node) = queue.pop_front() {
        let (state, position) = *nodes.get(node).expect("a numbered node");
        let letter = *word.get(position).expect("a position of the word");
        for target in gba
            .successors(state, letter)
            .expect("a state of the automaton")
        {
            let key = (target.index(), after(position));
            let next = *numbers.entry(key).or_insert_with(|| {
                nodes.push((target, key.1));
                edges.push(Vec::new());
                queue.push_back(nodes.len() - 1);
                nodes.len() - 1
            });
            edges.get_mut(node).expect("a numbered node").push(next);
        }
    }
    let reach: Vec<BTreeSet<usize>> = (0..nodes.len())
        .map(|start| {
            let mut seen = BTreeSet::new();
            let mut stack = edges.get(start).cloned().unwrap_or_default();
            while let Some(node) = stack.pop() {
                if seen.insert(node) {
                    stack.extend(edges.get(node).cloned().unwrap_or_default());
                }
            }
            seen
        })
        .collect();
    (0..nodes.len()).any(|start| {
        let Some(reached) = reach.get(start) else {
            return false;
        };
        if !reached.contains(&start) {
            return false;
        }
        let component: Vec<usize> = reached
            .iter()
            .copied()
            .filter(|node| reach.get(*node).is_some_and(|back| back.contains(&start)))
            .collect();
        (0..gba.acceptance_sets().len()).all(|set| {
            component.iter().any(|node| {
                let (state, _) = *nodes.get(*node).expect("a numbered node");
                gba.is_accepting_in(set, state)
            })
        })
    })
}

/// Every word of `1..=longest` letters over `width` atoms.
fn words(width: usize, longest: usize) -> Vec<Vec<Letter>> {
    let mut all = Vec::new();
    let mut layer: Vec<Vec<Letter>> = vec![Vec::new()];
    for _ in 0..longest {
        layer = layer
            .iter()
            .flat_map(|word| {
                (0..1_u64 << width).map(move |bits| {
                    let mut longer = word.clone();
                    longer.push(Letter::from_bits(bits));
                    longer
                })
            })
            .collect();
        all.extend(layer.iter().cloned());
    }
    all
}

/// The number of letter atoms the evaluator supplies: the clause's atoms
/// and its `when`.
fn width(subject: &Subject) -> usize {
    subject.property.atoms().len() + usize::from(subject.property.when().is_some())
}

/// Compare the automaton of `subject` with the evaluator on every word of up
/// to `longest` positions, and return how many words were compared. The
/// clause both holds on some word and fails on another, so neither answer is
/// trivially right.
fn agree(subject: &Subject, longest: usize) -> usize {
    agree_on(subject, longest, true)
}

fn agree_on(subject: &Subject, longest: usize, both: bool) -> usize {
    let property = &subject.property;
    let mut compared = 0;
    let mut verdicts = BTreeSet::new();
    for word in words(width(subject), longest) {
        match property.kind() {
            AutomatonKind::Bounded => {
                let expected = evaluator_holds(
                    subject,
                    &word,
                    TemporalTrace::Finite {
                        len: word.len() as u64,
                    },
                );
                // The false extension: every atom false after the last
                // position, and the activations stop there.
                let mut extended: Vec<Letter> = word
                    .iter()
                    .map(|letter| match property.live() {
                        Some(live) => letter.with(live),
                        None => *letter,
                    })
                    .collect();
                extended.push(Letter::default());
                let accepted = accepts(&subject.gba, &extended, word.len());
                assert_eq!(
                    accepted,
                    expected,
                    "{}: the automaton and the evaluator disagree on the closed trace {:?}",
                    subject.body,
                    bits(&word)
                );
                verdicts.insert(expected);
                compared += 1;
            }
            AutomatonKind::Safety | AutomatonKind::Liveness => {
                for prefix in 0..word.len() {
                    let expected = evaluator_holds(
                        subject,
                        &word,
                        TemporalTrace::Lasso {
                            prefix: prefix as u64,
                            loop_len: (word.len() - prefix) as u64,
                        },
                    );
                    let accepted = accepts(&subject.gba, &word, prefix);
                    let violating = property.kind() == AutomatonKind::Liveness;
                    assert_eq!(
                        accepted,
                        expected != violating,
                        "{}: the automaton and the evaluator disagree on the lasso {:?} \
                         with the loop from {prefix}",
                        subject.body,
                        bits(&word)
                    );
                    verdicts.insert(expected);
                    compared += 1;
                }
            }
        }
    }
    assert!(
        !both || verdicts.len() == 2,
        "{}: one verdict on every word",
        subject.body
    );
    compared
}

fn bits(word: &[Letter]) -> Vec<u64> {
    word.iter().map(|letter| letter.bits()).collect()
}

/// `body` under infinite-trace has the `kind` automaton, which answers to the
/// evaluator on every lasso of up to `longest` positions.
fn infinite_agrees(body: &str, kind: AutomatonKind, longest: usize) {
    let subject = infinite(body);
    assert_eq!(subject.property.kind(), kind, "{body}");
    assert!(agree(&subject, longest) > 0);
}

fn bounded_agrees(body: &str, longest: usize) {
    let subject = bounded(body);
    assert_eq!(subject.property.kind(), AutomatonKind::Bounded, "{body}");
    assert!(agree(&subject, longest) > 0);
}

/// SM-6: a bounded formula gets a finite monitor that answers to the closed
/// trace evaluation, past operators and every interval shape included.
#[test]
fn a_bounded_clause_translates_to_a_finite_monitor_that_answers_to_the_closed_trace() {
    for body in [
        "eventually[0,2] holds(p)",
        "eventually[1,3] holds(p)",
        "always[0,2] holds(p)",
        "always[1,2] (holds(p) or holds(q))",
        "holds(p) until[0,2] holds(q)",
        "holds(p) until[1,3] holds(q)",
        "holds(p) release[0,2] holds(q)",
        "holds(p) release[1,3] holds(q)",
        "eventually[0,1] (holds(p) and eventually[0,1] holds(q))",
        "always[0,3] (holds(p) implies eventually[1,2] holds(q))",
        "eventually[1,2] always[0,1] holds(p)",
        "always[0,4] (holds(p) implies once[0,2] holds(q))",
        "eventually[2,4] (holds(p) and once[1,2] holds(q))",
        "eventually[0,4] (holds(p) and (holds(q) since[1,3] holds(r)))",
        "always[0,3] (holds(p) implies historically[0,1] holds(q))",
        "always[0,3] (holds(p) implies (holds(q) triggered[0,2] holds(r)))",
        "holds(p) and not eventually[0,1] holds(q)",
    ] {
        bounded_agrees(body, if body.contains("holds(r)") { 4 } else { 6 });
    }
}

/// SM-6: a bounded `on each` clause activates at every position of the
/// closed trace and at none of the false extension, and an activation's
/// `when` condition gates it.
#[test]
fn a_bounded_each_clause_activates_only_at_the_positions_of_the_closed_trace() {
    for activation in [EACH, WHEN] {
        for body in [
            "holds(p)",
            "not holds(p) implies holds(q)",
            "holds(p) implies once[0,1] holds(q)",
            "eventually[0,1] holds(p)",
        ] {
            let subject = subject_with(EVENT, activation, body);
            assert_eq!(subject.property.kind(), AutomatonKind::Bounded);
            assert!(subject.property.live().is_some());
            assert_eq!(subject.property.when().is_some(), activation == WHEN);
            assert!(agree(&subject, 5) > 0);
        }
    }
}

/// SM-6: an infinite-trace safety formula (TP-1, TP-3) gets the automaton of
/// the formula itself, with no acceptance set, which answers to the lasso
/// evaluation.
#[test]
fn a_safety_clause_translates_to_the_automaton_of_the_formula_itself() {
    for body in [
        "always holds(p)",
        "always (holds(p) implies holds(q))",
        "holds(p) release holds(q)",
        "always (holds(p) implies once holds(q))",
        "always (holds(p) implies (holds(q) triggered holds(r)))",
        "always (holds(p) implies historically holds(q))",
        "always (holds(p) implies eventually[0,2] holds(q))",
        "always (holds(p) implies eventually[1,3] holds(q))",
        "always (holds(p) implies always[1,2] holds(q))",
        "always (holds(p) implies (holds(q) until[0,2] holds(r)))",
        "always (holds(p) implies (holds(q) release[0,2] holds(r)))",
        "always (holds(p) implies (holds(q) until[1,2] holds(r)))",
        "always (holds(p) implies (holds(q) release[1,2] holds(r)))",
        "always (holds(p) implies eventually[0,2] always[0,1] holds(q))",
        "always[2,*] holds(p)",
        "always (holds(p) implies always[1,*] holds(q))",
        "always (holds(p) implies (holds(q) release[1,*] holds(r)))",
        "always (holds(p) implies once[1,2] holds(q))",
        "always (holds(p) implies once[0,2] holds(q))",
        "always (holds(p) implies once[1,*] holds(q))",
        "always (holds(p) implies (holds(q) since[1,3] holds(r)))",
        "always (holds(p) implies (holds(q) since[0,2] holds(r)))",
        "always (holds(p) implies historically[1,2] holds(q))",
        "always (holds(p) implies (holds(q) triggered[1,2] holds(r)))",
    ] {
        let width = if body.contains("holds(r)") { 3 } else { 2 };
        infinite_agrees(body, AutomatonKind::Safety, if width == 3 { 3 } else { 5 });
    }
    for body in ["always holds(p)", "always (holds(p) implies once holds(q))"] {
        let subject = infinite(body);
        assert!(subject.gba.acceptance_sets().is_empty());
        assert_eq!(subject.property.acceptance_set_count(), 0);
    }
}

/// SM-6: a liveness formula (TP-4) gets the automaton of its negation,
/// including a formula with an interval operator under an unbounded one and
/// an `[a,*]` interval.
#[test]
fn a_liveness_clause_translates_to_the_automaton_of_its_negation() {
    for body in [
        "eventually holds(p)",
        "always eventually holds(p)",
        "eventually always holds(p)",
        "holds(p) until holds(q)",
        "always (holds(p) implies eventually holds(q))",
        "always (holds(p) implies (holds(q) until holds(r)))",
        "eventually (holds(p) and always eventually[0,2] holds(q))",
        "always (holds(p) implies eventually always[0,2] holds(q))",
        "eventually[2,*] holds(p)",
        "always (holds(p) implies eventually[1,*] holds(q))",
        "always (holds(p) implies (holds(q) until[1,*] holds(r)))",
        "always eventually (holds(p) and once[1,1] holds(q))",
        "eventually (holds(p) and (holds(q) since holds(r)))",
        "always (holds(p) implies eventually[0,1] eventually holds(q))",
        "always eventually (holds(p) and eventually[1,2] holds(q))",
        "eventually always[2,*] holds(p)",
        "eventually always[3,*] holds(p)",
        "always (holds(p) implies eventually always[2,*] holds(q))",
    ] {
        let width = if body.contains("holds(r)") { 3 } else { 2 };
        infinite_agrees(
            body,
            AutomatonKind::Liveness,
            if width == 3 { 3 } else { 5 },
        );
    }
}

/// SM-6, SM-8: past operators are automaton state, so the automaton is finite
/// and needs no unrolling; ADR-018 section 6's example reads the same on its
/// loop.
#[test]
fn a_past_operator_is_tracked_as_automaton_state_across_the_loop_edge() {
    let body = "always eventually (holds(p) and once[1,1] holds(q))";
    let subject = infinite(body);
    assert_eq!(subject.property.kind(), AutomatonKind::Liveness);
    assert!(subject.gba.state_count() < 16);
    // `p` holds at the loop's first position, `q` at its last: the position
    // before the first is the last, across the edge.
    let word = [
        Letter::from_bits(0b01),
        Letter::from_bits(0b00),
        Letter::from_bits(0b10),
    ];
    let lasso = TemporalTrace::Lasso {
        prefix: 0,
        loop_len: 3,
    };
    assert!(evaluator_holds(&subject, &word, lasso));
    assert!(!accepts(&subject.gba, &word, 0));
    let without = [Letter::from_bits(0b01), Letter::from_bits(0b00)];
    let lasso = TemporalTrace::Lasso {
        prefix: 0,
        loop_len: 2,
    };
    assert!(!evaluator_holds(&subject, &without, lasso));
    assert!(accepts(&subject.gba, &without, 0));
}

/// SM-6: the bad-prefix monitor of a safety automaton rejects exactly when
/// the prefix read is a bad prefix: no continuation satisfies the clause. A
/// prefix it does not reject has a continuation that does. The monitor over
/// the explicit automaton and the one reading the translation lazily agree
/// on every prefix; `eventually[0,2] false` is a branch with no infinite run,
/// which the monitor drops at once rather than when its deadline passes.
#[test]
fn a_safety_monitor_rejects_exactly_the_bad_prefixes() {
    for body in [
        "always (holds(p) implies holds(q))",
        "always (holds(p) implies eventually[0,2] holds(q))",
        "always (holds(p) implies once[0,1] holds(q))",
        "holds(p) release holds(q)",
        "always (holds(p) or eventually[0,2] false)",
    ] {
        let subject = infinite(body);
        assert_eq!(subject.property.kind(), AutomatonKind::Safety);
        let mut monitor =
            Monitor::bad_prefix(&subject.gba, LIMIT).expect("a safety automaton has a monitor");
        let mut lazy = Monitor::bad_prefix(
            translate(&subject.fixture.clause, LIMIT).expect("the clause translates"),
            LIMIT,
        )
        .expect("a safety automaton has a monitor");
        let continuations = words(width(&subject), 2);
        for prefix in words(width(&subject), 4) {
            let mut state = monitor.initial().expect("within the limit");
            let mut reached = lazy.initial().expect("within the limit");
            for letter in &prefix {
                state = monitor.step(state, *letter).expect("within the limit");
                reached = lazy.step(reached, *letter).expect("within the limit");
            }
            let rejects = monitor.is_rejecting(state).expect("a monitor state");
            assert_eq!(
                lazy.is_rejecting(reached),
                Ok(rejects),
                "{body}: the lazy monitor on {:?}",
                bits(&prefix)
            );
            let satisfiable = continuations.iter().any(|loop_letters| {
                let word: Vec<Letter> = prefix.iter().chain(loop_letters).copied().collect();
                evaluator_holds(
                    &subject,
                    &word,
                    TemporalTrace::Lasso {
                        prefix: prefix.len() as u64,
                        loop_len: loop_letters.len() as u64,
                    },
                )
            });
            assert_eq!(
                rejects,
                !satisfiable,
                "{body}: the prefix {:?}",
                bits(&prefix)
            );
        }
    }
}

/// SM-6: a bounded formula's monitor is deterministic and finite: reading
/// the same word twice reaches the same state, a window that passes with no
/// witness is rejecting, and the state count stays within the counter bound.
#[test]
fn a_bounded_monitor_is_deterministic_and_finite() {
    let subject = bounded("eventually[0,3] holds(p)");
    let mut monitor = Monitor::bad_prefix(&subject.gba, LIMIT).expect("no acceptance set");
    let read = |monitor: &mut Monitor<&Gba>, word: &[Letter]| {
        let mut state = monitor.initial().expect("within the limit");
        for letter in word {
            state = monitor.step(state, *letter).expect("within the limit");
        }
        state
    };
    let word = [Letter::from_bits(0b00), Letter::from_bits(0b00)];
    assert_eq!(read(&mut monitor, &word), read(&mut monitor, &word));
    let waiting = read(&mut monitor, &word);
    assert!(!monitor.is_rejecting(waiting).expect("a monitor state"));
    let silent = [Letter::default(); 8];
    let state = read(&mut monitor, &silent);
    assert!(monitor.is_rejecting(state).expect("a monitor state"));
    assert!(monitor.state_count() <= 6);
}

/// IV-3, IV-5: a lower bound of 0 over an operand keeps one counter with
/// `b + 1` values, and the negation of the recovery-stability formula has
/// `b + 2` states: one before the failure and a counter of positions since
/// the last unhealthy state.
#[test]
fn the_recovery_stability_negation_has_a_counter_of_b_plus_two_states() {
    for bound in [0_u64, 1, 3, 10] {
        let body = format!("always (holds(p) implies eventually always[0,{bound}] holds(q))");
        let subject = infinite(&body);
        assert_eq!(subject.property.kind(), AutomatonKind::Liveness);
        assert_eq!(subject.gba.state_count() as u64, bound + 2, "{body}");
        assert!(agree(&subject, if bound > 3 { 3 } else { 5 }) > 0, "{body}");
    }
}

/// IV-3, IV-5: one counter per occurrence with `b + 1` values for
/// `eventually`, `until`, `always` and `release` with lower bound 0, and
/// `b + 2` for a past interval operator with lower bound 0.
#[test]
fn an_interval_occurrence_with_lower_bound_zero_keeps_one_counter() {
    for bound in [1_u64, 3, 7] {
        for (shape, states) in [
            ("eventually[0,B] holds(q)", bound + 1),
            ("holds(r) until[0,B] holds(q)", bound + 1),
            ("always[0,B] holds(q)", bound + 1),
            ("holds(r) release[0,B] holds(q)", bound + 1),
            ("once[0,B] holds(q)", bound + 2),
            ("holds(r) since[0,B] holds(q)", bound + 2),
        ] {
            let inner = shape.replace('B', &bound.to_string());
            let body = format!("always (holds(p) implies {inner})");
            let subject = infinite(&body);
            assert_eq!(subject.property.kind(), AutomatonKind::Safety, "{body}");
            assert_eq!(subject.gba.state_count() as u64, states, "{body}");
        }
    }
}

/// IV-3: a nonzero lower bound shifts the lower-bound-0 operator, so
/// `eventually[a,b] p` is `a` steps and then the counter; the translation
/// agrees with the expansion into next-position steps for every lower bound
/// up to `b`.
#[test]
fn a_nonzero_lower_bound_is_a_shift_then_the_counter() {
    for (lower, upper) in [(1, 1), (1, 2), (2, 2), (2, 3)] {
        for shape in [
            "eventually[A,B] holds(q)",
            "always[A,B] holds(q)",
            "holds(r) until[A,B] holds(q)",
            "holds(r) release[A,B] holds(q)",
        ] {
            let inner = shape
                .replace('A', &lower.to_string())
                .replace('B', &upper.to_string());
            let body = format!("always (holds(p) implies {inner})");
            infinite_agrees(&body, AutomatonKind::Safety, 4);
        }
    }
}

/// IV-6, FR-338: the automaton-state limit counts the states the product
/// reaches, not the states the translation could reach. An interval of any
/// length translates; a run that never opens it reaches few states; a run
/// that counts it down stops at the limit.
#[test]
fn the_automaton_state_limit_counts_the_states_the_product_reaches() {
    let fixture = fixture(
        INFINITE,
        "c: Counter",
        ORIGIN,
        "always (holds(p) implies eventually[0,2000000] holds(q))",
    );
    let (p, quiet) = (Letter::from_bits(0b01), Letter::default());
    let step = |property: &mut PropertyAutomaton,
                states: &BTreeSet<StateId>,
                letter: Letter|
     -> Result<BTreeSet<StateId>, AutomatonError> {
        let mut next = BTreeSet::new();
        for state in states {
            next.extend(property.successors(*state, letter)?);
        }
        Ok(next)
    };
    let mut property = translate(&fixture.clause, 100).expect("translation reaches no state");
    assert_eq!(property.reached_states(), 0);
    let start = BTreeSet::from([property.initial().expect("within the limit")]);
    let mut states = start.clone();
    for _ in 0..1000 {
        states = step(&mut property, &states, quiet).expect("within the limit");
    }
    let idle = property.reached_states();
    assert!(idle <= 3, "{idle} states reached with `p` never holding");

    // `p` opens the interval, and `q` never arrives: each position reaches
    // a new counter value, up to the limit.
    let mut states = step(&mut property, &start, p);
    let error = loop {
        match states {
            Ok(reached) => states = step(&mut property, &reached, quiet),
            Err(error) => break error,
        }
    };
    assert_eq!(error, AutomatonError::StateLimit { limit: 100 });
    assert_eq!(property.reached_states(), 100);

    // The whole reachable graph does not fit under that limit.
    let mut whole = translate(&fixture.clause, 100).expect("translation reaches no state");
    assert_eq!(
        whole.explore().unwrap_err(),
        AutomatonError::StateLimit { limit: 100 }
    );
    let past = infinite("always (holds(p) implies once holds(q))");
    assert_eq!(past.gba.state_count(), 2);
}

/// SM-6: a past operator over a future operator has no history to track, so
/// the translation refuses it.
#[test]
fn a_past_operator_over_a_future_operator_is_refused() {
    assert_eq!(
        translation_of("always once eventually holds(p)").unwrap_err(),
        AutomatonError::PastOverFuture
    );
}

// QSL-923: the disjunction rule asserts the negation of a state operand in the
// branch that does not discharge on it, so that negation has to be evaluated
// like any other state formula. The tests below compare the automaton of an
// `or` body with a lasso semantics written here from the definitions of the
// operators, independent of the translator and of the clause evaluator.

/// A formula over numbered leaves. Leaf `n` is the `n`th `holds` of the text,
/// which is the `n`th atom of the clause: each occurrence has its own letter
/// bit.
#[derive(Clone, Debug)]
enum Form {
    Leaf(usize),
    Not(Box<Form>),
    And(Box<Form>, Box<Form>),
    Or(Box<Form>, Box<Form>),
    Next(Box<Form>),
    Always(Box<Form>),
    Eventually(Box<Form>),
    Until(Box<Form>, Box<Form>),
    Release(Box<Form>, Box<Form>),
}

impl Form {
    fn text(&self) -> String {
        match self {
            Self::Leaf(n) => format!("holds(a{n})"),
            Self::Not(a) => format!("not ({})", a.text()),
            Self::And(a, b) => format!("({}) and ({})", a.text(), b.text()),
            Self::Or(a, b) => format!("({}) or ({})", a.text(), b.text()),
            Self::Next(a) => format!("eventually[1,1] ({})", a.text()),
            Self::Always(a) => format!("always ({})", a.text()),
            Self::Eventually(a) => format!("eventually ({})", a.text()),
            Self::Until(a, b) => format!("({}) until ({})", a.text(), b.text()),
            Self::Release(a, b) => format!("({}) release ({})", a.text(), b.text()),
        }
    }

    /// The leaves renumbered in reading order, which is the order of the
    /// clause's atoms, and how many there are.
    fn numbered(&self, next: &mut usize) -> Self {
        let mut two = |a: &Self, b: &Self, make: fn(Box<Self>, Box<Self>) -> Self| {
            let a = a.numbered(next);
            let b = b.numbered(next);
            make(Box::new(a), Box::new(b))
        };
        match self {
            Self::Leaf(_) => {
                *next += 1;
                Self::Leaf(*next - 1)
            }
            Self::Not(a) => Self::Not(Box::new(a.numbered(next))),
            Self::Next(a) => Self::Next(Box::new(a.numbered(next))),
            Self::Always(a) => Self::Always(Box::new(a.numbered(next))),
            Self::Eventually(a) => Self::Eventually(Box::new(a.numbered(next))),
            Self::And(a, b) => two(a, b, Self::And),
            Self::Or(a, b) => two(a, b, Self::Or),
            Self::Until(a, b) => two(a, b, Self::Until),
            Self::Release(a, b) => two(a, b, Self::Release),
        }
    }

    /// The truth at each position of the lasso `word` whose loop starts at
    /// `prefix`, straight from the semantics: a position's successor is the
    /// next one, or the loop start after the last.
    fn truth(&self, word: &[Letter], prefix: usize) -> Vec<bool> {
        let n = word.len();
        let succ = |i: usize| if i + 1 == n { prefix } else { i + 1 };
        // The positions from `i` on, `n` of them cover every distinct one.
        let from = |i: usize| {
            let mut run = vec![i];
            for _ in 1..n {
                run.push(succ(*run.last().expect("a start")));
            }
            run
        };
        match self {
            Self::Leaf(k) => word.iter().map(|letter| letter.holds(atom(*k))).collect(),
            Self::Not(a) => a.truth(word, prefix).iter().map(|v| !v).collect(),
            Self::And(a, b) => {
                let (a, b) = (a.truth(word, prefix), b.truth(word, prefix));
                a.iter().zip(&b).map(|(x, y)| *x && *y).collect()
            }
            Self::Or(a, b) => {
                let (a, b) = (a.truth(word, prefix), b.truth(word, prefix));
                a.iter().zip(&b).map(|(x, y)| *x || *y).collect()
            }
            Self::Next(a) => {
                let a = a.truth(word, prefix);
                (0..n).map(|i| a[succ(i)]).collect()
            }
            Self::Always(a) => {
                let a = a.truth(word, prefix);
                (0..n).map(|i| from(i).iter().all(|j| a[*j])).collect()
            }
            Self::Eventually(a) => {
                let a = a.truth(word, prefix);
                (0..n).map(|i| from(i).iter().any(|j| a[*j])).collect()
            }
            Self::Until(a, b) => {
                let (a, b) = (a.truth(word, prefix), b.truth(word, prefix));
                (0..n)
                    .map(|i| {
                        let run = from(i);
                        run.iter()
                            .position(|j| b[*j])
                            .is_some_and(|at| run[..at].iter().all(|j| a[*j]))
                    })
                    .collect()
            }
            Self::Release(a, b) => {
                let (a, b) = (a.truth(word, prefix), b.truth(word, prefix));
                // `a release b` is `not (not a until not b)`.
                (0..n)
                    .map(|i| {
                        let run = from(i);
                        !run.iter()
                            .position(|j| !b[*j])
                            .is_some_and(|at| run[..at].iter().all(|j| !a[*j]))
                    })
                    .collect()
            }
        }
    }
}

fn leaf() -> Form {
    Form::Leaf(0)
}

fn not(a: Form) -> Form {
    Form::Not(Box::new(a))
}

fn or(a: Form, b: Form) -> Form {
    Form::Or(Box::new(a), Box::new(b))
}

fn and(a: Form, b: Form) -> Form {
    Form::And(Box::new(a), Box::new(b))
}

fn until(a: Form, b: Form) -> Form {
    Form::Until(Box::new(a), Box::new(b))
}

fn always(a: Form) -> Form {
    Form::Always(Box::new(a))
}

fn eventually(a: Form) -> Form {
    Form::Eventually(Box::new(a))
}

/// Compare the automaton of `form` with its semantics on every lasso with a
/// stem of at most `stem` and a loop of 1 to `looped` positions; the number of
/// lassos compared and how many of them satisfy the formula.
fn lassos_agree(form: &Form, stem: usize, looped: usize) -> (usize, usize) {
    let mut totals = (0, 0);
    for mode in [
        Mode::Clause,
        Mode::Negation,
        Mode::Itself,
        Mode::ItselfNormal,
    ] {
        let (compared, satisfying) = lassos_agree_in(form, stem, looped, mode);
        totals = (totals.0 + compared, totals.1 + satisfying);
    }
    totals
}

/// Which automaton of a body is compared with its semantics.
#[derive(Clone, Copy, Debug)]
enum Mode {
    /// The automaton of the clause, the one a safety body is checked by.
    Clause,
    /// The automaton of the negation in normal form (ADR-023 CS-2), whose
    /// `or` operands are a flattened set: what a hyper product follows.
    Negation,
    /// The automaton of the body itself, whatever its fragment, which is the
    /// negation of the clause `not body`.
    Itself,
    /// [`Self::Itself`] in normal form.
    ItselfNormal,
}

fn lassos_agree_in(form: &Form, stem: usize, looped: usize, mode: Mode) -> (usize, usize) {
    let form = form.numbered(&mut 0);
    let body = form.text();
    let (normal, itself) = match mode {
        Mode::Clause => (false, false),
        Mode::Negation => (true, false),
        Mode::Itself => (false, true),
        Mode::ItselfNormal => (true, true),
    };
    let subject = if normal || itself {
        let clause = if itself {
            format!("not ({body})")
        } else {
            body.clone()
        };
        let mut subject = infinite(&clause);
        let symmetry = AtomSymmetry { generators: &[] };
        subject.property = translate_negation(
            subject.fixture.clause.formula(),
            subject.fixture.clause.profile(),
            LIMIT,
            normal.then_some(&symmetry),
        )
        .unwrap_or_else(|why| panic!("{body}: {why}"));
        subject.gba = subject
            .property
            .explore()
            .unwrap_or_else(|why| panic!("{body}: {why}"));
        subject
    } else {
        infinite(&body)
    };
    let atoms = subject.property.atoms().len();
    let (mut compared, mut satisfying) = (0, 0);
    for length in 1..=stem + looped {
        for word in words_of(atoms, length) {
            for prefix in length.saturating_sub(looped)..=stem.min(length - 1) {
                let expected = form.truth(&word, prefix)[0];
                // A liveness automaton accepts the words that violate its
                // clause, which are the words satisfying the body for `Itself`.
                let violating = subject.property.kind() == AutomatonKind::Liveness && !itself;
                assert_eq!(
                    accepts(&subject.gba, &word, prefix),
                    expected != violating,
                    "{body} ({mode:?}): the automaton and the semantics disagree \
                     on the lasso {:?} with the loop from {prefix}",
                    bits(&word)
                );
                compared += 1;
                satisfying += usize::from(expected);
            }
        }
    }
    (compared, satisfying)
}

/// Every word of exactly `length` letters over `width` atoms.
fn words_of(width: usize, length: usize) -> Vec<Vec<Letter>> {
    words(width, length)
        .into_iter()
        .filter(|word| word.len() == length)
        .collect()
}

/// The shape of QSL-923's first effect: the negation of a plain state test is
/// the other branch of the disjunction, so `p or always q` holds on a word
/// that falsifies `p` and keeps `q`, and is not decided by `p` alone.
#[trace("TC-601")]
#[test]
fn a_disjunct_of_a_plain_state_test_is_not_dropped() {
    for form in [
        or(leaf(), always(leaf())),
        or(always(leaf()), leaf()),
        or(not(leaf()), eventually(leaf())),
        and(not(leaf()), eventually(leaf())),
        or(leaf(), until(leaf(), leaf())),
        or(or(leaf(), leaf()), always(leaf())),
        or(and(leaf(), leaf()), always(leaf())),
        or(not(or(leaf(), leaf())), eventually(leaf())),
    ] {
        let (compared, satisfying) = lassos_agree(&form, 2, 3);
        assert!(compared > 0 && satisfying > 0 && satisfying < compared);
    }
    // `p or always q` on the word whose first letter has neither: `p` fails
    // and `q` holds forever, so the formula holds, and the translation that
    // loses the second disjunct rejects it.
    let subject = infinite("holds(a0) or always holds(a1)");
    let word = [Letter::from_bits(0b10)];
    assert!(
        accepts(&subject.gba, &word, 0) != (subject.property.kind() == AutomatonKind::Liveness)
    );
}

/// The shape of QSL-923's second effect: a disjunction of a negation, an
/// `until` and a nested `or`, which the translation refused as a fault.
#[trace("TC-601")]
#[test]
fn a_disjunction_of_a_negation_an_until_and_a_nested_or_translates() {
    for form in [
        or(not(leaf()), or(until(leaf(), leaf()), or(leaf(), leaf()))),
        or(not(leaf()), or(until(leaf(), leaf()), leaf())),
        or(not(leaf()), or(until(leaf(), leaf()), not(leaf()))),
        or(or(until(leaf(), leaf()), leaf()), not(leaf())),
    ] {
        let width = form.numbered(&mut 0).text().matches("holds").count();
        let (compared, _) = lassos_agree(&form, if width > 4 { 1 } else { 2 }, 2);
        assert!(compared > 0);
    }
}

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

/// A formula of at most `leaves` leaves.
fn random_form(rng: &mut Rng, leaves: usize) -> Form {
    if leaves <= 1 {
        return if rng.below(3) == 0 {
            not(leaf())
        } else {
            leaf()
        };
    }
    let split = 1 + rng.below(leaves - 1);
    let (a, b) = (random_form(rng, split), random_form(rng, leaves - split));
    match rng.below(9) {
        0 => Form::Next(Box::new(random_form(rng, leaves))),
        1 => always(random_form(rng, leaves)),
        2 => eventually(random_form(rng, leaves)),
        3 => not(random_form(rng, leaves)),
        4 => and(a, b),
        5 | 6 => or(a, b),
        7 => until(a, b),
        _ => Form::Release(Box::new(a), Box::new(b)),
    }
}

/// The automaton of random bodies over `or`, `and`, `not`, `until`,
/// `release`, `always`, `eventually` and `next` of up to 3 atoms accepts
/// exactly the lassos the formula's semantics accepts, for every stem and
/// loop up to 3 positions (2 positions for the 3-atom bodies' stem).
#[trace("TC-601")]
#[test]
fn the_automaton_of_a_random_body_accepts_exactly_the_lassos_of_its_semantics() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut checked = 0;
    let mut mixed = 0;
    while checked < 24 {
        let leaves = 2 + rng.below(2);
        let form = random_form(&mut rng, leaves);
        let atoms = form.numbered(&mut 0).text().matches("holds").count();
        if atoms > 3 {
            continue;
        }
        let (stem, looped) = if atoms == 3 { (2, 3) } else { (3, 3) };
        let (compared, satisfying) = lassos_agree(&form, stem, looped);
        checked += 1;
        mixed += usize::from(satisfying > 0 && satisfying < compared);
    }
    assert!(
        mixed > 8,
        "{mixed} of the {checked} bodies split the lassos"
    );
}

/// `or` bodies over bounded intervals and past operators answer to the
/// evaluator on the closed traces and lassos, in the bounded and the
/// infinite-trace profile.
#[trace("TC-601")]
#[test]
fn an_or_body_over_intervals_and_past_operators_answers_to_the_evaluator() {
    for body in [
        "holds(p) or eventually[0,2] holds(q)",
        "holds(p) or always[1,2] holds(q)",
        "not holds(p) or (holds(q) until[0,2] holds(r))",
        "holds(p) or (holds(q) release[1,2] holds(r))",
        "once[0,2] holds(p) or eventually[1,2] holds(q)",
        "(holds(p) since[0,2] holds(q)) or eventually[0,1] holds(r)",
        "not holds(p) or eventually[1,2] (holds(q) or always[0,1] holds(r))",
    ] {
        bounded_loose(body, if body.contains("holds(r)") { 3 } else { 5 });
    }
    for body in [
        "holds(p) or always holds(q)",
        "not holds(p) or eventually holds(q)",
        "holds(p) or always[1,*] holds(q)",
        "always (holds(p) or eventually[0,2] holds(q))",
        "always (not holds(p) or once[0,2] holds(q))",
        "always (holds(p) or (holds(q) since[1,2] holds(r)))",
        "eventually (holds(p) or once[1,1] holds(q))",
        "holds(p) or (holds(q) until[1,*] holds(r))",
    ] {
        let subject = infinite(body);
        loose(&subject, if body.contains("holds(r)") { 3 } else { 5 });
    }
}

fn bounded_loose(body: &str, longest: usize) {
    let subject = bounded(body);
    assert_eq!(subject.property.kind(), AutomatonKind::Bounded, "{body}");
    loose(&subject, longest);
}

/// `agree`, for a body that may hold on every word compared.
fn loose(subject: &Subject, longest: usize) {
    assert!(agree_on(subject, longest, false) > 0);
}
