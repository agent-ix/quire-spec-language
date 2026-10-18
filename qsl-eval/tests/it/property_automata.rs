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

use qsl_eval::automaton::{
    translate, Atom, AutomatonError, AutomatonKind, Gba, Letter, Monitor, PropertyAutomaton,
};
use qsl_eval::temporal::{
    evaluate, AtomReading, TemporalLetters, TemporalOutcome, TemporalPosition, TemporalTrace,
};
use qsl_semantics::check::{FormulaNodeId, SCALAR_LIMITS_UNLIMITED};
use quire_exact::{Incomplete, Meter};

use crate::temporal_trace::{fixture, Fixture, EVENT, INFINITE};

const LIMIT: u64 = 1 << 16;
const WHEN: &str = "on each (q: Counter) when (true)";
const EACH: &str = "on each (q: Counter)";
const ORIGIN: &str = "on origin";

/// A clause, its automaton and the letter it reads.
struct Subject {
    body: String,
    fixture: Fixture,
    property: PropertyAutomaton,
}

fn subject_with(profile: &str, activation: &str, body: &str) -> Subject {
    let fixture = fixture(profile, "c: Counter", activation, body);
    let property = translate(&fixture.clause, LIMIT).unwrap_or_else(|why| panic!("{body}: {why}"));
    Subject {
        body: body.to_owned(),
        fixture,
        property,
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
    ) -> Result<AtomReading, Incomplete> {
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
    ) -> Result<AtomReading, Incomplete> {
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
                let accepted = accepts(property.automaton(), &extended, word.len());
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
                    let accepted = accepts(property.automaton(), &word, prefix);
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
    assert_eq!(
        verdicts.len(),
        2,
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
        assert!(translation_of(body).is_ok());
    }
    for body in ["always holds(p)", "always (holds(p) implies once holds(q))"] {
        let automaton = infinite(body).property;
        assert!(automaton.automaton().acceptance_sets().is_empty());
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
    assert!(subject.property.automaton().state_count() < 16);
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
    assert!(!accepts(subject.property.automaton(), &word, 0));
    let without = [Letter::from_bits(0b01), Letter::from_bits(0b00)];
    let lasso = TemporalTrace::Lasso {
        prefix: 0,
        loop_len: 2,
    };
    assert!(!evaluator_holds(&subject, &without, lasso));
    assert!(accepts(subject.property.automaton(), &without, 0));
}

/// SM-6: the bad-prefix monitor of a safety automaton rejects exactly when
/// the prefix read is a bad prefix: no continuation satisfies the clause. A
/// prefix it does not reject has a continuation that does.
#[test]
fn a_safety_monitor_rejects_exactly_the_bad_prefixes() {
    for body in [
        "always (holds(p) implies holds(q))",
        "always (holds(p) implies eventually[0,2] holds(q))",
        "always (holds(p) implies once[0,1] holds(q))",
        "holds(p) release holds(q)",
    ] {
        let subject = infinite(body);
        assert_eq!(subject.property.kind(), AutomatonKind::Safety);
        let mut monitor = Monitor::bad_prefix(subject.property.automaton(), LIMIT)
            .expect("a safety automaton has a monitor");
        let continuations = words(width(&subject), 2);
        for prefix in words(width(&subject), 4) {
            let mut state = monitor.initial().expect("within the limit");
            for letter in &prefix {
                state = monitor.step(state, *letter).expect("within the limit");
            }
            let rejects = monitor.is_rejecting(state).expect("a monitor state");
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
/// the same word always reaches the same state, and its state count stays
/// within the counter bound.
#[test]
fn a_bounded_monitor_is_deterministic_and_finite() {
    let subject = bounded("eventually[0,3] holds(p)");
    let mut monitor =
        Monitor::bad_prefix(subject.property.automaton(), LIMIT).expect("no acceptance set");
    let mut state = monitor.initial().expect("within the limit");
    for _ in 0..8 {
        state = monitor
            .step(state, Letter::default())
            .expect("within the limit");
    }
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
        assert_eq!(
            subject.property.automaton().state_count() as u64,
            bound + 2,
            "{body}"
        );
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
            let automaton = infinite(&body).property;
            assert_eq!(automaton.kind(), AutomatonKind::Safety, "{body}");
            assert_eq!(automaton.automaton().state_count() as u64, states, "{body}");
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

/// IV-3: a state limit stops the translation of an interval whose counter
/// would pass it (IV-6), and an unbounded past operator needs no counter.
#[test]
fn the_automaton_state_limit_stops_a_long_interval() {
    let fixture = fixture(
        INFINITE,
        "c: Counter",
        ORIGIN,
        "always (holds(p) implies eventually[0,1000000] holds(q))",
    );
    assert_eq!(
        translate(&fixture.clause, 100).unwrap_err(),
        AutomatonError::StateLimit { limit: 100 }
    );
    let past = infinite("always (holds(p) implies once holds(q))");
    assert_eq!(past.property.automaton().state_count(), 2);
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
