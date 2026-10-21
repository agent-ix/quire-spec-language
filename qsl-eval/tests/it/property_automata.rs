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
    translate, translate_counted, Atom, AutomatonError, AutomatonKind, Gba, Letter, Monitor,
    PropertyAutomaton, StateId,
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

/// A bounded `on origin` clause translated for a subject whose steps count
/// by kind (ADR-027 PB-3).
fn counted(body: &str) -> Subject {
    let fixture = fixture(EVENT, "c: Counter", ORIGIN, body);
    let mut property =
        translate_counted(&fixture.clause, LIMIT).unwrap_or_else(|why| panic!("{body}: {why}"));
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
    /// Whether the step that closes a lasso's loop counts: the step into
    /// the loop's entry, which its letter says.
    loop_edge_counts: bool,
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

    fn counts_step(&self, position: TemporalPosition) -> bool {
        self.property
            .uncounted()
            .is_none_or(|uncounted| !self.letter(position).holds(uncounted))
    }

    fn counts_loop_edge(&self) -> bool {
        self.loop_edge_counts
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
    let loop_edge_counts = match (trace, subject.property.uncounted()) {
        (TemporalTrace::Lasso { prefix, .. }, Some(uncounted)) => usize::try_from(prefix)
            .ok()
            .and_then(|entry| word.get(entry))
            .is_none_or(|letter| !letter.holds(uncounted)),
        _ => true,
    };
    let mut letters = WordLetters {
        property: &subject.property,
        word,
        loop_edge_counts,
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

/// The number of letter atoms the evaluator supplies: the clause's atoms,
/// its `when` and the atom that marks a step that does not count.
fn width(subject: &Subject) -> usize {
    subject.property.atoms().len()
        + usize::from(subject.property.when().is_some())
        + usize::from(subject.property.uncounted().is_some())
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
        // No step leads into position 0, which the evaluator reads as one
        // counted step from before the trace.
        if let (Some(uncounted), Some(first)) = (property.uncounted(), word.first()) {
            if first.holds(uncounted) {
                continue;
            }
        }
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
                    // A bad-prefix or finitely-accepting automaton cannot see
                    // that a loop with no counted step never meets a future
                    // interval eventuality: EN-1 decides those loops by the
                    // non-progress check, and the evaluator reads them
                    // directly. A body without one (past operators, `always`,
                    // `release`) is decided by the automaton on those loops
                    // too.
                    let owes =
                        subject.body.contains("eventually") || subject.body.contains("until");
                    if let Some(uncounted) = property.uncounted().filter(|_| owes) {
                        // The loop's steps are those into its positions after
                        // the entry, and its closing step is the entry's own.
                        let progress = word
                            .get(prefix..)
                            .into_iter()
                            .flatten()
                            .any(|letter| !letter.holds(uncounted));
                        if !progress {
                            continue;
                        }
                    }
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

/// ADR-027 PB-3 (FR-213-AC-3): for a subject whose steps count by kind, a
/// bounded clause's finite monitor answers to the closed trace evaluation
/// over a letter that also says which steps do not count, whatever the
/// interval shape. A clause without an interval operator reads no such atom.
#[trace("TC-658")]
#[test]
fn a_counted_clause_translates_to_a_monitor_that_answers_to_counted_distance() {
    for body in [
        "eventually[0,2] holds(p)",
        "eventually[1,2] holds(p)",
        "always[0,2] holds(p)",
        "always[1,2] holds(p)",
        "eventually[0,1] (holds(p) and eventually[0,1] holds(q))",
        "always[0,2] (holds(p) implies eventually[1,2] holds(q))",
        "holds(p) until[0,2] holds(q)",
        "holds(p) until[1,3] holds(q)",
        "holds(p) release[0,2] holds(q)",
        "holds(p) release[1,2] holds(q)",
        "eventually[1,2] always[0,1] holds(p)",
    ] {
        let subject = counted(body);
        assert_eq!(subject.property.kind(), AutomatonKind::Bounded, "{body}");
        assert!(subject.property.uncounted().is_some(), "{body}");
        assert!(agree(&subject, if body.contains("holds(q)") { 4 } else { 5 }) > 0);
    }
    let plain = counted("holds(p) and not holds(q)");
    assert_eq!(plain.property.uncounted(), None);
}

/// ADR-027 PB-3 (FR-338, FR-092): a past operator with an interval over a
/// subject whose steps count by kind measures its window backwards in counted
/// steps, as the evaluator does: the age of a lower-bound-0 operator and the
/// shift of a lower bound move only on a counted step, several positions
/// share a distance, and an operand read at an uncounted step is still read.
/// The automaton answers to the closed-trace evaluation over every word of
/// up to four positions, `once`, `historically`, `since` and `triggered`
/// included.
#[trace("TC-658")]
#[test]
fn a_past_interval_over_counted_steps_answers_to_counted_distance() {
    for body in [
        "always[0,3] (holds(p) implies once[0,1] holds(q))",
        "always[0,3] (holds(p) implies once[1,2] holds(q))",
        "always[0,3] (holds(p) implies once[2,3] holds(q))",
        "always[0,3] (holds(p) implies historically[0,1] holds(q))",
        "always[0,3] (holds(p) implies historically[1,2] holds(q))",
        "always[0,3] (holds(p) implies (holds(p) since[0,1] holds(q)))",
        "always[0,3] (holds(p) implies (holds(q) since[1,2] holds(p)))",
        "always[0,3] (holds(p) implies (holds(q) triggered[0,1] holds(p)))",
        "always[0,3] (holds(p) implies (holds(p) triggered[1,2] holds(q)))",
        "eventually[1,3] (holds(p) and once[1,1] holds(q))",
    ] {
        let subject = counted(body);
        assert_eq!(subject.property.kind(), AutomatonKind::Bounded, "{body}");
        assert!(subject.property.uncounted().is_some(), "{body}");
        assert!(agree(&subject, 4) > 0, "{body}");
    }
}

/// ADR-027 PB-3 (FR-338): an infinite-trace clause with an interval over a
/// subject whose steps count by kind answers to the lasso evaluation in
/// counted steps, over every lasso of up to four positions and every split
/// into prefix and loop. A loop with no counted step keeps every distance
/// constant, so an interval that needs a counted step after the loop is
/// entered is not met on it; the loop's closing step counts as its entry's
/// letter says.
#[trace("TC-658")]
#[test]
fn a_counted_lasso_answers_to_counted_distance() {
    for (body, kind) in [
        ("eventually[1,2] holds(p)", AutomatonKind::Safety),
        (
            "always (holds(p) implies eventually[0,1] holds(q))",
            AutomatonKind::Safety,
        ),
        (
            "always (holds(p) implies eventually[1,2] holds(q))",
            AutomatonKind::Safety,
        ),
        (
            "always (holds(p) implies always[1,1] holds(q))",
            AutomatonKind::Safety,
        ),
        (
            "always (holds(p) implies (holds(q) until[0,2] holds(p)))",
            AutomatonKind::Safety,
        ),
        (
            "always (holds(p) implies once[1,2] holds(q))",
            AutomatonKind::Safety,
        ),
        (
            "always (holds(p) implies eventually[2,3] holds(q))",
            AutomatonKind::Safety,
        ),
        (
            "always (holds(p) implies once[2,3] holds(q))",
            AutomatonKind::Safety,
        ),
        (
            "always (holds(p) implies historically[0,1] holds(q))",
            AutomatonKind::Safety,
        ),
        ("always eventually[0,1] holds(p)", AutomatonKind::Safety),
        ("eventually[0,1] always holds(p)", AutomatonKind::Safety),
        (
            "always (holds(p) implies eventually[1,*] holds(q))",
            AutomatonKind::Liveness,
        ),
        ("eventually always[0,1] holds(p)", AutomatonKind::Liveness),
    ] {
        let fixture = fixture(INFINITE, "c: Counter", ORIGIN, body);
        let mut property =
            translate_counted(&fixture.clause, LIMIT).unwrap_or_else(|why| panic!("{body}: {why}"));
        let gba = property
            .explore()
            .unwrap_or_else(|why| panic!("{body}: {why}"));
        let subject = Subject {
            body: body.to_owned(),
            fixture,
            property,
            gba,
        };
        assert_eq!(subject.property.kind(), kind, "{body}");
        assert!(subject.property.uncounted().is_some(), "{body}");
        assert!(agree(&subject, 4) > 0, "{body}");
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
