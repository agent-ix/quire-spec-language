// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338's canonical translation state for the safety items EN-1 decides:
//! the exact key octets of the deadlock-freedom item, of TP-1 clauses and of
//! the TP-4 Büchi and guarded-Büchi states, and the admission of imported
//! keys against an item's context.
//!
//! The expected octets are written out here as ASCII text, independent of
//! the encoder under test. The interval and past members of the
//! `["safety", branches]` algebra and the acceptance members of the Büchi
//! alternatives are checked against clause formulas below.
//! Not covered here: the bounded alternative (`automaton_key_bounded.rs`,
//! except its vector rows in the FR-338-AC-4 aggregate), the protocol
//! subject's `distance-history` past form and finite F/U acceptance (QSL-982
//! PR 2b), and the unguarded `on each` and open-upper TP-4 forms, which are
//! refused until the spec rulings R-5 and R-2.

use ix_trace_rs::trace;
use qsl_eval::automaton::{
    BoundedTranslation, BuchiPhase, BuchiState, BuchiStates, Configuration, ContextActivation,
    DistanceBucket, HistoryOperand, KeyRefusal, Letter, Obligation, Occurrence, PastMemory,
    PastOperator, Polarity, SafetyState, SafetyStates, SafetyTranslation, StateContext,
    SubjectKind, TranslationRefusal, Window,
};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use quire_exact::{Origin, Role};
use std::collections::{BTreeMap, BTreeSet};

use crate::temporal_trace::{fixture, EVENT, INFINITE};

/// FR-338's full-key oracle for the deadlock-freedom item's carried root.
const DEADLOCK_CARRIED: &str = r#"["quire.automaton-state-key/v1",[["deadlock-freedom"],"quire.temporal.infinite-trace/v1","infinite",[]],["safety",[[[[[],"positive"]],[],[],[]]]]]"#;

/// The deadlock-freedom context's encoding, as the oracle spells it.
const DEADLOCK_CONTEXT: &str =
    r#"[["deadlock-freedom"],"quire.temporal.infinite-trace/v1","infinite",[]]"#;

fn deadlock() -> SafetyTranslation {
    SafetyTranslation::deadlock_freedom().expect("the deadlock context encodes")
}

/// `octets` with the deadlock context and `body`.
fn deadlock_key(body: &str) -> String {
    format!(r#"["quire.automaton-state-key/v1",{DEADLOCK_CONTEXT},{body}]"#)
}

fn occurrence(ordinal: u64) -> OccurrenceKey {
    OccurrenceKey::new(
        crate::support::ids::node_key([0xab]),
        Origin::new(Role::new("temporal"), ordinal),
    )
}

/// The TP-1 clause `body` over `c: Counter` under infinite-trace at
/// `occurrence(ordinal)`, with an empty binding.
fn clause(body: &str, ordinal: u64) -> SafetyTranslation {
    let fixture = fixture(INFINITE, "c: Counter", "on origin", body);
    let context = StateContext::clause(&fixture.clause, occurrence(ordinal), Vec::new());
    SafetyTranslation::reachable_invariant(&fixture.clause, &context)
        .unwrap_or_else(|why| panic!("{body}: {why}"))
}

fn text(translation: &SafetyTranslation, state: SafetyState) -> String {
    let key = translation.key(state).expect("the key encodes");
    String::from_utf8(key.as_bytes().to_vec()).expect("a key is UTF-8")
}

/// FR-338's deadlock-freedom oracle: at the initial value the carried
/// root's complete key is the authored ASCII line; the rejecting state is
/// `["safety",[]]` under the same context. Reading a deadlocked letter
/// rejects, and rejection is absorbing.
#[trace("TC-525")]
#[test]
fn the_deadlock_item_keys_are_the_authored_octets() {
    let translation = deadlock();
    let carried = translation.read(None, true);
    assert_eq!(carried, SafetyState::Carried);
    assert_eq!(text(&translation, carried), DEADLOCK_CARRIED);
    let rejected = translation.read(Some(carried), false);
    assert_eq!(
        text(&translation, rejected),
        deadlock_key(r#"["safety",[]]"#)
    );
    assert_eq!(
        translation.read(Some(rejected), true),
        SafetyState::Rejected
    );
    assert_eq!(translation.read(None, false), SafetyState::Rejected);
}

/// FR-338's TP-1 vectors: `always holds(false)` reads to the bad prefix
/// `["safety",[]]` and `always holds(true)` carries its root G,
/// `["safety",[[[[[],"positive"]],[],[],[]]]]`. The clause root is the
/// FR-322 NodeId object with role and ordinal as strings: ordinal 0 is
/// `"0"` and u64::MAX is its exact decimal string. A clause that is not
/// `always holds(A)`, or one checked under a bounded profile, is not
/// translated.
#[trace("TC-525")]
#[test]
fn a_reachable_invariant_keys_its_clause_root_and_state_body() {
    let node = format!(
        r#"{{"digest":"{}","domain":"quire.checked-semantic-node/v1"}}"#,
        "ab".repeat(32)
    );
    let context = |ordinal: &str| {
        format!(
            r#"[[{node},"temporal","{ordinal}"],"quire.temporal.infinite-trace/v1","infinite",[]]"#
        )
    };
    let falsity = clause("always holds(false)", 0);
    let bad = falsity.read(None, false);
    assert_eq!(
        text(&falsity, bad),
        format!(
            r#"["quire.automaton-state-key/v1",{},["safety",[]]]"#,
            context("0")
        )
    );
    let truth = clause("always holds(true)", u64::MAX);
    let carried = truth.read(Some(truth.read(None, true)), true);
    assert_eq!(
        text(&truth, carried),
        format!(
            r#"["quire.automaton-state-key/v1",{},["safety",[[[[[],"positive"]],[],[],[]]]]]"#,
            context("18446744073709551615")
        )
    );

    let shape = fixture(
        INFINITE,
        "c: Counter",
        "on origin",
        "eventually holds(true)",
    );
    let context = StateContext::clause(&shape.clause, occurrence(0), Vec::new());
    assert!(matches!(
        SafetyTranslation::reachable_invariant(&shape.clause, &context),
        Err(TranslationRefusal::NotTranslated(_))
    ));
    let bounded = fixture(EVENT, "c: Counter", "on origin", "always[0,1] holds(true)");
    let context = StateContext::clause(&bounded.clause, occurrence(0), Vec::new());
    assert_eq!(context.activation, ContextActivation::Origin);
    assert!(matches!(
        SafetyTranslation::reachable_invariant(&bounded.clause, &context),
        Err(TranslationRefusal::NotTranslated(_))
    ));
}

/// The state is a function of the facts read, not of the order or path
/// that reached it: every reading order that keeps the root carried gives
/// identical octets, and the context, not a materialization index,
/// distinguishes two items' keys.
#[trace("TC-525")]
#[test]
fn equal_facts_give_equal_keys_in_any_reading_order() {
    let translation = deadlock();
    let once = translation.read(None, true);
    let thrice = translation.read(Some(translation.read(Some(once), true)), true);
    assert_eq!(
        translation.key(once).expect("encodes"),
        translation.key(thrice).expect("encodes")
    );
    let first = clause("always holds(true)", 0);
    let second = clause("always holds(true)", 1);
    assert_ne!(
        first.key(SafetyState::Carried).expect("encodes"),
        second.key(SafetyState::Carried).expect("encodes")
    );
}

/// Every key a translation encodes is admitted back as the same state; a
/// key of another item, of another occurrence, or under another profile or
/// activation is refused `ForeignContext` before it is used.
#[trace("TC-525")]
#[test]
fn a_key_of_another_context_is_not_admitted() {
    let translation = deadlock();
    for state in [SafetyState::Carried, SafetyState::Rejected] {
        let key = translation.key(state).expect("encodes");
        assert_eq!(
            translation.admit(key.as_bytes()),
            Ok((key.clone(), state)),
            "{state:?}"
        );
    }
    let clause_key = clause("always holds(true)", 0)
        .key(SafetyState::Carried)
        .expect("encodes");
    assert_eq!(
        translation.admit(clause_key.as_bytes()),
        Err(KeyRefusal::ForeignContext)
    );
    let other_occurrence = clause("always holds(true)", 1);
    assert_eq!(
        other_occurrence.admit(clause_key.as_bytes()),
        Err(KeyRefusal::ForeignContext)
    );
    let event = DEADLOCK_CARRIED.replace(
        "quire.temporal.infinite-trace/v1",
        "quire.temporal.event-position.false-extension/v1",
    );
    let origin = DEADLOCK_CARRIED.replace("\"infinite\"", "\"origin\"");
    for foreign in [event, origin] {
        assert_eq!(
            translation.admit(foreign.as_bytes()),
            Err(KeyRefusal::ForeignContext),
            "{foreign}"
        );
    }
}

/// FR-338's admission negatives that a safety key can carry, each refused
/// before membership: a non-canonical encoding, a wrong discriminator, an
/// unknown tag, another form's tag, a state index in place of the closed
/// state, missing and extra members, unsorted and duplicate sets, a
/// non-canonical decimal, a node id used as an operand path, an
/// occurrence the formula has no carry for, an interval debt, past memory
/// and acceptance membership the formula has no occurrence for, and an
/// obligation-free branch the translation never reaches.
#[trace("TC-525")]
#[test]
fn malformed_safety_keys_refuse_before_membership() {
    let translation = deadlock();
    let root = r#"[[],"positive"]"#;
    let branch = |obligations: &str| format!(r#"[[{obligations}],[],[],[]]"#);
    let safety = |branches: &str| format!(r#"["safety",[{branches}]]"#);
    let cases: Vec<(String, KeyRefusal)> = vec![
        (
            DEADLOCK_CARRIED.replace(",", ", "),
            KeyRefusal::NotCanonical,
        ),
        (format!("{DEADLOCK_CARRIED}\n"), KeyRefusal::NotCanonical),
        ("[".to_owned(), KeyRefusal::Malformed),
        (
            DEADLOCK_CARRIED.replace("automaton-state-key/v1", "automaton-state-key/v2"),
            KeyRefusal::Discriminator,
        ),
        (
            format!(r#"["quire.automaton-state-key/v1",{DEADLOCK_CONTEXT}]"#),
            KeyRefusal::Shape,
        ),
        (
            deadlock_key(r#"["tableau",[]]"#),
            KeyRefusal::UnknownTag("tableau".to_owned()),
        ),
        (
            deadlock_key(r#"["accept"]"#),
            KeyRefusal::WrongForm("accept".to_owned()),
        ),
        (
            deadlock_key(r#"["each-buchi"]"#),
            KeyRefusal::WrongForm("each-buchi".to_owned()),
        ),
        (deadlock_key(r#"["safety",0]"#), KeyRefusal::Members),
        (deadlock_key(r#"["safety"]"#), KeyRefusal::Members),
        (deadlock_key(r#"["safety",[],[]]"#), KeyRefusal::Members),
        (
            deadlock_key(&safety(&format!(r#"[[{root}],[],[]]"#))),
            KeyRefusal::Members,
        ),
        (
            deadlock_key(&safety(&format!(r#"[[{root}],[],[],[],[]]"#))),
            KeyRefusal::Members,
        ),
        (
            deadlock_key(&safety(&format!("{},{}", branch(root), branch(root)))),
            KeyRefusal::Order,
        ),
        (
            deadlock_key(&safety(&branch(&format!("{root},{root}")))),
            KeyRefusal::Order,
        ),
        (
            deadlock_key(&safety(&branch(r#"[["01"],"positive"]"#))),
            KeyRefusal::Decimal("01".to_owned()),
        ),
        (
            deadlock_key(&safety(&branch(
                r#"[[{"digest":"00","domain":"quire.checked-semantic-node/v1"}],"positive"]"#,
            ))),
            KeyRefusal::Members,
        ),
        (
            deadlock_key(&safety(&branch(r#"[["0"],"positive"]"#))),
            KeyRefusal::Unresolved,
        ),
        (
            deadlock_key(&safety(&branch(r#"[[],"negative"]"#))),
            KeyRefusal::Unresolved,
        ),
        (
            deadlock_key(&safety(&branch(r#"[[],"neutral"]"#))),
            KeyRefusal::Members,
        ),
        (
            deadlock_key(&safety(&branch(r#"[[],"positive","0","1"]"#))),
            KeyRefusal::Unresolved,
        ),
        (
            deadlock_key(&safety(&format!(r#"[[{root}],[[[],"once",true]],[],[]]"#))),
            KeyRefusal::Unresolved,
        ),
        (
            deadlock_key(&safety(&format!(r#"[[{root}],[],[],[{root}]]"#))),
            KeyRefusal::Unresolved,
        ),
        (deadlock_key(&safety(&branch(""))), KeyRefusal::Unreachable),
        (
            deadlock_key(&safety(&format!("{},{}", branch(root), branch("")))),
            KeyRefusal::Unreachable,
        ),
    ];
    for (octets, refusal) in cases {
        assert_eq!(
            translation.admit(octets.as_bytes()),
            Err(refusal),
            "{octets}"
        );
    }
}

/// The unbounded-profile clause `body` over `c: Counter`, its safety
/// states under `occurrence(0)`.
fn states(body: &str) -> SafetyStates {
    let fixture = fixture(INFINITE, "c: Counter", "on origin", body);
    let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
    SafetyStates::clause(&fixture.clause, &context, SubjectKind::Ordinary)
        .unwrap_or_else(|why| panic!("{body}: {why}"))
}

/// The context of `states(_)` as FR-338 spells it.
fn states_context() -> String {
    let node = format!(
        r#"{{"digest":"{}","domain":"quire.checked-semantic-node/v1"}}"#,
        "ab".repeat(32)
    );
    format!(r#"[[{node},"temporal","0"],"quire.temporal.infinite-trace/v1","infinite",[]]"#)
}

/// The full key `K_C(body)` of `states(_)`, as the oracle spells it.
fn states_key(body: &str) -> String {
    format!(
        r#"["quire.automaton-state-key/v1",{},{body}]"#,
        states_context()
    )
}

fn octets(states: &SafetyStates, branches: &[Configuration]) -> String {
    let key = states.key(branches).expect("the key encodes");
    String::from_utf8(key.as_bytes().to_vec()).expect("a key is UTF-8")
}

fn at(path: &[u64], polarity: Polarity) -> Occurrence {
    Occurrence::new(path.to_vec(), polarity)
}

/// The carried root `G`: `[[],"positive"]` as a current obligation.
fn root() -> Obligation {
    Obligation::Current(at(&[], Polarity::Positive))
}

const FUTURE: &str = "always (holds(c.level = 1) implies eventually[0,3] holds(c.level = 2))";
const TWO_FUTURES: &str =
    "eventually[0,3] holds(c.level = 1) and eventually[0,3] holds(c.level = 2)";
const SHIFTED: &str = "always (holds(c.level = 1) implies eventually[1,3] holds(c.level = 2))";

/// The `["safety", branches]` algebra's share of FR-338-AC-4's vectors (the
/// whole table is asserted by `every_canonical_octet_vector_is_the_authored_octets`):
/// the state bodies and full keys are the authored octets; reversing the insertion order of
/// obligations, windows, offsets and branches leaves the bytes
/// equal; distinct occurrence paths and differing live facts leave them
/// unequal; and the bytes are a function of those facts alone, equal across
/// independently built translations, with no enumeration index in them.
#[trace("TC-525")]
#[test]
fn safety_keys_are_the_authored_octets_and_a_function_of_the_facts() {
    let future = states(FUTURE);
    let deadline = |remaining: u64| Configuration {
        obligations: vec![root()],
        windows: vec![Window::Deadline(at(&[0, 1], Polarity::Positive), remaining)],
        ..Configuration::default()
    };
    assert_eq!(
        octets(&future, &[deadline(1)]),
        states_key(
            r#"["safety",[[[[[],"positive"]],[],[[["0","1"],"positive","deadline","1"]],[]]]]"#
        )
    );
    assert_eq!(
        octets(&future, &[]),
        states_key(r#"["safety",[]]"#),
        "the rejecting state"
    );

    // Offsets: `{[0,2],[1,3]}` encodes `[["0","2"],["1","3"]]` however the
    // set was materialized.
    let shifted = states(SHIFTED);
    let offsets = |pairs: Vec<(u64, u64)>| Configuration {
        windows: vec![Window::Offsets(at(&[0, 1], Polarity::Positive), pairs)],
        ..Configuration::default()
    };
    let forward = octets(&shifted, &[offsets(vec![(0, 2), (1, 3)])]);
    assert_eq!(
        forward,
        states_key(
            r#"["safety",[[[],[],[[["0","1"],"positive","offsets",[["0","2"],["1","3"]]]],[]]]]"#
        )
    );
    assert_eq!(forward, octets(&shifted, &[offsets(vec![(1, 3), (0, 2)])]));
    let reversed = forward.replace(r#"[["0","2"],["1","3"]]"#, r#"[["1","3"],["0","2"]]"#);
    assert_eq!(
        shifted.admit(reversed.as_bytes()),
        Err(KeyRefusal::Order),
        "the reversed array supplied as canonical input refuses"
    );

    // Insertion order of every set and of the branches.
    let both = states(TWO_FUTURES);
    let windows = vec![
        Window::Deadline(at(&[0], Polarity::Positive), 1),
        Window::Deadline(at(&[1], Polarity::Positive), 1),
    ];
    let mixed = states("always holds(c.level = 1) and always holds(c.level = 2)");
    let one = Configuration {
        obligations: vec![
            Obligation::Current(at(&[0], Polarity::Positive)),
            Obligation::Current(at(&[1], Polarity::Positive)),
        ],
        ..Configuration::default()
    };
    let flipped = Configuration {
        obligations: one.obligations.iter().rev().cloned().collect(),
        ..Configuration::default()
    };
    let other = Configuration {
        obligations: vec![Obligation::Current(at(&[0], Polarity::Positive))],
        ..Configuration::default()
    };
    assert_eq!(
        octets(&mixed, &[one.clone(), other.clone()]),
        octets(&mixed, &[other.clone(), flipped.clone(), one.clone()]),
        "branch and obligation order, and a duplicate branch"
    );
    let reversed_windows = Configuration {
        windows: windows.iter().rev().cloned().collect(),
        ..Configuration::default()
    };
    let forward_windows = Configuration {
        windows: windows.clone(),
        ..Configuration::default()
    };
    assert_eq!(
        octets(&both, &[forward_windows]),
        octets(&both, &[reversed_windows]),
        "window order"
    );
    let two_acceptances = buchi(TWO_SETS, "on origin");
    let members = vec![
        at(&[1, 0], Polarity::Negative),
        at(&[1, 1], Polarity::Negative),
    ];
    let accept = |entries: Vec<Occurrence>| {
        BuchiState::Elementary(Configuration {
            acceptance: entries,
            ..Configuration::default()
        })
    };
    assert_eq!(
        two_acceptances.key(&accept(members.clone())),
        two_acceptances.key(&accept(members.iter().rev().cloned().collect()))
    );
    assert!(two_acceptances.key(&accept(members.clone())).is_ok());

    // Distinct occurrence paths with equal counters, and differing live
    // facts, give unequal bytes.
    let at_zero = Configuration {
        windows: vec![Window::Deadline(at(&[0], Polarity::Positive), 1)],
        ..Configuration::default()
    };
    let at_one = Configuration {
        windows: vec![Window::Deadline(at(&[1], Polarity::Positive), 1)],
        ..Configuration::default()
    };
    let keys = [
        octets(&both, std::slice::from_ref(&at_zero)),
        octets(&both, &[at_one]),
        octets(
            &both,
            &[Configuration {
                windows: vec![Window::Deadline(at(&[0], Polarity::Positive), 2)],
                ..Configuration::default()
            }],
        ),
        octets(
            &both,
            &[Configuration {
                windows: vec![Window::Extent(at(&[0], Polarity::Negative), 1)],
                ..Configuration::default()
            }],
        ),
        octets(&both, &[Configuration::default()]),
        octets(&both, &[]),
    ];
    for (index, key) in keys.iter().enumerate() {
        for other in &keys[index + 1..] {
            assert_ne!(key, other);
        }
    }

    // The bytes are a function of the facts: an independently built
    // translation of the same clause writes them again, and they hold no
    // graph index.
    assert_eq!(
        octets(&states(TWO_FUTURES), &[at_zero]),
        keys[0],
        "independent translations"
    );
    assert_eq!(
        keys[0],
        states_key(r#"["safety",[[[],[],[[["0"],"positive","deadline","1"]],[]]]]"#),
        "the whole key is the authored members and nothing else"
    );
}

/// Each member kind a key can carry is admitted back as the facts that
/// were written, sorted, whatever order they were written in.
#[trace("TC-525")]
#[test]
fn safety_keys_admit_back_as_the_facts_they_were_written_from() {
    let future = states(FUTURE);
    let shifted = states(SHIFTED);
    let open = states("always (holds(c.level = 1) implies always[2,*] holds(c.level = 2))");
    let interval = Configuration {
        obligations: vec![Obligation::Interval {
            occurrence: at(&[0, 1], Polarity::Positive),
            lower: 0,
            upper: Some(2),
        }],
        ..Configuration::default()
    };
    let up = |states: &SafetyStates, configuration: Configuration| {
        let key = states
            .key(std::slice::from_ref(&configuration))
            .expect("encodes");
        let (admitted, back) = states.admit(key.as_bytes()).expect("admitted");
        assert_eq!(admitted, key);
        back
    };
    assert_eq!(up(&future, interval.clone()), vec![interval]);
    let windows = Configuration {
        windows: vec![Window::Deadline(at(&[0, 1], Polarity::Positive), 3)],
        ..Configuration::default()
    };
    assert_eq!(up(&future, windows.clone()), vec![windows]);
    let extent = Configuration {
        windows: vec![Window::Extent(at(&[0, 1], Polarity::Negative), 2)],
        ..Configuration::default()
    };
    assert_eq!(up(&future, extent.clone()), vec![extent]);
    let offsets = Configuration {
        windows: vec![Window::Offsets(
            at(&[0, 1], Polarity::Positive),
            vec![(0, 2), (1, 3)],
        )],
        ..Configuration::default()
    };
    assert_eq!(up(&shifted, offsets.clone()), vec![offsets]);
    let shift = Configuration {
        obligations: vec![root()],
        windows: vec![Window::Shift(at(&[0, 1], Polarity::Positive), 2)],
        ..Configuration::default()
    };
    assert_eq!(up(&open, shift.clone()), vec![shift]);
}

/// Past memory is complete and typed by the occurrence's interval: an
/// unbounded flag, a saturated age counter (`b+1` included), a `history`
/// vector and a `shift-history` vector with its tail; a branch that omits a
/// past occurrence's memory, or gives it the form of another interval
/// class, is refused.
#[trace("TC-525", "FR-338-AC-5")]
#[test]
fn past_memory_is_typed_by_the_occurrence_interval_and_complete() {
    let path = vec![0, 1];
    let memory = |states: &SafetyStates, entry: PastMemory, text: &str| {
        let configuration = Configuration {
            past: vec![entry],
            ..Configuration::default()
        };
        let key = octets(states, std::slice::from_ref(&configuration));
        assert_eq!(
            key,
            states_key(&format!(r#"["safety",[[[],[{text}],[],[]]]]"#)),
            "{text}"
        );
        assert_eq!(
            states.admit(key.as_bytes()).map(|(_, branches)| branches),
            Ok(vec![configuration]),
            "{text}"
        );
    };
    let unbounded = states("always (holds(c.level = 1) implies once holds(c.level = 2))");
    memory(
        &unbounded,
        PastMemory::Flag {
            path: path.clone(),
            operator: PastOperator::Once,
            value: true,
        },
        r#"[["0","1"],"once",true]"#,
    );
    let age = states("always (holds(c.level = 1) implies once[0,2] holds(c.level = 2))");
    memory(
        &age,
        PastMemory::Age {
            path: path.clone(),
            operator: PastOperator::Once,
            age: 3,
        },
        r#"[["0","1"],"last-true","3"]"#,
    );
    let history = states("always (holds(c.level = 1) implies once[1,2] holds(c.level = 2))");
    memory(
        &history,
        PastMemory::History {
            path: path.clone(),
            operands: vec![HistoryOperand {
                path: vec![0, 1, 0],
                offsets: vec![1, 2],
            }],
        },
        r#"[["0","1"],"history",[[["0","1","0"],["1","2"]]]]"#,
    );
    let reversed = Configuration {
        past: vec![PastMemory::History {
            path: path.clone(),
            operands: vec![HistoryOperand {
                path: vec![0, 1, 0],
                offsets: vec![2, 1],
            }],
        }],
        ..Configuration::default()
    };
    assert_eq!(
        octets(&history, &[reversed]),
        states_key(
            r#"["safety",[[[],[[["0","1"],"history",[[["0","1","0"],["1","2"]]]]],[],[]]]]"#
        ),
        "offsets are a set"
    );
    let shifted = states("always (holds(c.level = 1) implies once[2,*] holds(c.level = 2))");
    memory(
        &shifted,
        PastMemory::ShiftHistory {
            path: path.clone(),
            operands: vec![HistoryOperand {
                path: vec![0, 1, 0],
                offsets: vec![1],
            }],
            tail: false,
        },
        r#"[["0","1"],"shift-history",[[["0","1","0"],["1"]]],false]"#,
    );

    let flag = |tag: &str, value: &str| {
        states_key(&format!(
            r#"["safety",[[[],[[["0","1"],"{tag}",{value}]],[],[]]]]"#
        ))
    };
    let refused = [
        (
            &unbounded,
            states_key(r#"["safety",[[[],[],[],[]]]]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &unbounded,
            flag("historically", "true"),
            KeyRefusal::Unresolved,
        ),
        (
            &unbounded,
            flag("last-true", r#""0""#),
            KeyRefusal::Unresolved,
        ),
        (&unbounded, flag("once", "1"), KeyRefusal::Members),
        (&age, flag("last-true", r#""4""#), KeyRefusal::Unresolved),
        (&age, flag("last-true", "true"), KeyRefusal::Members),
        (&age, flag("once", "true"), KeyRefusal::Unresolved),
        (
            &age,
            flag("last-true", r#""03""#),
            KeyRefusal::Decimal("03".into()),
        ),
        (
            &history,
            flag("history", r#"[[["0","1","0"],["3"]]]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &history,
            flag("history", r#"[[["0","1","1"],["1"]]]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &history,
            flag("history", r#"[[["0","1","0"],["2","1"]]]"#),
            KeyRefusal::Order,
        ),
        (
            &history,
            flag("history", r#"[[["0","1","0"],["0"]]]"#),
            KeyRefusal::Unresolved,
        ),
        (&history, flag("history", "[]"), KeyRefusal::Members),
    ];
    for (states, key, refusal) in refused {
        assert_eq!(
            states.admit(key.as_bytes()).map(|_| ()),
            Err(refusal),
            "{key}"
        );
    }
    // The saturation value of a `u64::MAX` bound is `2^64`, no more.
    let widest = states(
        "always (holds(c.level = 1) implies once[0,18446744073709551615] holds(c.level = 2))",
    );
    for (age, admitted) in [
        ("18446744073709551616", true),
        ("18446744073709551617", false),
    ] {
        let key = flag("last-true", &format!(r#""{age}""#));
        assert_eq!(widest.admit(key.as_bytes()).is_ok(), admitted, "{age}");
    }
}

/// Window and acceptance members resolve in the formula with the matching
/// operator, polarity and bounds: a counter strategy other than the one the
/// interval shape selects, a debt past the checked upper bound, a path
/// that is no occurrence, an interval debt stored in both obligations and
/// windows, and acceptance for a greatest-fixed-point or bounded
/// occurrence are refused.
#[trace("TC-525", "FR-338-AC-5")]
#[test]
fn window_and_acceptance_members_must_resolve_in_the_formula() {
    let future = states(FUTURE);
    let shifted = states(SHIFTED);
    let open = states("always (holds(c.level = 1) implies always[2,*] holds(c.level = 2))");
    let window = |entry: &str| states_key(&format!(r#"["safety",[[[],[],[{entry}],[]]]]"#));
    let accept = |entry: &str| states_key(&format!(r#"["safety",[[[],[],[],[{entry}]]]]"#));
    let refused = [
        (
            &future,
            window(r#"[["0","1"],"positive","deadline","4"]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &future,
            window(r#"[["0","1"],"positive","extent","1"]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &future,
            window(r#"[["0","1"],"negative","deadline","1"]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &future,
            window(r#"[["0","1"],"positive","offsets",[["0","1"]]]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &future,
            window(r#"[["0","0"],"positive","deadline","1"]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &future,
            window(r#"[["0","9"],"positive","deadline","1"]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &future,
            window(r#"[["0","1"],"positive","bucket","1"]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &future,
            window(r#"[["0","1"],"positive","deadline"]"#),
            KeyRefusal::Members,
        ),
        (
            &shifted,
            window(r#"[["0","1"],"positive","deadline","1"]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &shifted,
            window(r#"[["0","1"],"positive","offsets",[["2","1"]]]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &shifted,
            window(r#"[["0","1"],"positive","offsets",[["2","3"]]]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &shifted,
            window(r#"[["0","1"],"positive","offsets",[]]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &shifted,
            window(r#"[["0","1"],"positive","offsets",[["1","3"],["0","2"]]]"#),
            KeyRefusal::Order,
        ),
        (
            &open,
            window(r#"[["0","1"],"positive","shift","0"]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &open,
            window(r#"[["0","1"],"positive","shift","3"]"#),
            KeyRefusal::Unresolved,
        ),
        (
            &future,
            accept(r#"[["0","1"],"positive"]"#),
            KeyRefusal::Unresolved,
        ),
    ];
    for (states, key, refusal) in refused {
        assert_eq!(
            states.admit(key.as_bytes()).map(|_| ()),
            Err(refusal),
            "{key}"
        );
    }
    // The same debt cannot sit in both the obligations and the windows, and
    // one signed occurrence has one window.
    let both = states_key(
        r#"["safety",[[[[["0","1"],"positive","0","2"]],[],[[["0","1"],"positive","deadline","2"]],[]]]]"#,
    );
    let twice = states_key(
        r#"["safety",[[[],[],[[["0","1"],"positive","deadline","1"],[["0","1"],"positive","deadline","2"]],[]]]]"#,
    );
    for key in [both, twice] {
        assert_eq!(
            future.admit(key.as_bytes()).map(|_| ()),
            Err(KeyRefusal::Members),
            "{key}"
        );
    }
}

/// A key of the safety algebra under another clause's context, or one of
/// another form, is refused before its members are read.
#[trace("TC-525", "FR-338-AC-5")]
#[test]
fn safety_states_refuse_foreign_contexts_and_other_forms() {
    let future = states(FUTURE);
    let key = octets(&future, &[Configuration::default()]);
    let foreign = key.replace(r#""0"],"quire.temporal"#, r#""1"],"quire.temporal"#);
    assert_ne!(foreign, key);
    assert_eq!(
        future.admit(foreign.as_bytes()).map(|_| ()),
        Err(KeyRefusal::ForeignContext)
    );
    for tag in [
        "bounded",
        "accept",
        "reject",
        "buchi",
        "guarded-buchi",
        "each-buchi",
    ] {
        let other = states_key(&format!(r#"["{tag}"]"#));
        assert_eq!(
            future.admit(other.as_bytes()).map(|_| ()),
            Err(KeyRefusal::WrongForm(tag.to_owned())),
            "{tag}"
        );
    }
    let bounded = fixture(
        EVENT,
        "c: Counter",
        "on origin",
        "eventually[0,1] holds(true)",
    );
    let context = StateContext::clause(&bounded.clause, occurrence(0), Vec::new());
    assert!(matches!(
        SafetyStates::clause(&bounded.clause, &context, SubjectKind::Ordinary),
        Err(TranslationRefusal::NotTranslated(_))
    ));
}

const OPEN: &str = "always (holds(c.level = 1) implies always[2,*] holds(c.level = 2))";
const TWO_SETS: &str =
    "eventually holds(c.level = 1) and (always holds(c.level = 2) and always holds(c.level = 3))";

/// An `[a,open]` occurrence's shift entry ends at zero, and its unbounded
/// obligation `[o,s]` takes its place: that state is admitted and written,
/// the pending shift beside it is a different state, and the finished shift
/// stored in both places is refused. `[o,s,"0","open"]` is not the spelling.
#[trace("TC-525")]
#[test]
fn a_completed_shift_is_carried_as_the_unbounded_obligation() {
    let open = states(OPEN);
    let target = at(&[0, 1], Polarity::Positive);
    let pending = Configuration {
        obligations: vec![root()],
        windows: vec![Window::Shift(target.clone(), 1)],
        ..Configuration::default()
    };
    let done = Configuration {
        obligations: vec![Obligation::Current(target.clone()), root()],
        ..Configuration::default()
    };
    let key = open.key(std::slice::from_ref(&done)).expect("encodes");
    assert_eq!(
        String::from_utf8(key.as_bytes().to_vec()).expect("UTF-8"),
        states_key(r#"["safety",[[[[["0","1"],"positive"],[[],"positive"]],[],[],[]]]]"#)
    );
    assert_eq!(
        open.admit(key.as_bytes()).map(|(_, branches)| branches),
        Ok(vec![done.clone()])
    );
    assert_ne!(open.key(&[pending]).expect("encodes"), key);
    let twice = Configuration {
        windows: vec![Window::Shift(target, 1)],
        ..done
    };
    assert_eq!(open.key(&[twice]), Err(KeyRefusal::Members));
    let spelled = states_key(r#"["safety",[[[[["0","1"],"positive","0","open"]],[],[],[]]]]"#);
    assert_eq!(
        open.admit(spelled.as_bytes()).map(|_| ()),
        Err(KeyRefusal::Unresolved)
    );
}

/// FR-338-AC-5's interval-obligation negatives: `lower>upper`, an offset
/// outside the checked interval on either end, a negative offset, a finite
/// upper on an open checked interval, and a `[o,s]` obligation where the
/// occurrence is not a future temporal carry or carries an interval.
#[trace("TC-525", "FR-338-AC-5")]
#[test]
fn interval_obligations_outside_the_checked_interval_are_refused() {
    let future = states(FUTURE);
    let shifted = states(SHIFTED);
    let open = states(OPEN);
    let obligation = |entry: &str| states_key(&format!(r#"["safety",[[[{entry}],[],[],[]]]]"#));
    let refused = [
        (
            &future,
            r#"[["0","1"],"positive","2","1"]"#,
            KeyRefusal::Unresolved,
        ),
        (
            &future,
            r#"[["0","1"],"positive","0","4"]"#,
            KeyRefusal::Unresolved,
        ),
        (
            &future,
            r#"[["0","1"],"positive","1","3"]"#,
            KeyRefusal::Unresolved,
        ),
        (
            &future,
            r#"[["0","1"],"positive","0","open"]"#,
            KeyRefusal::Unresolved,
        ),
        (
            &future,
            r#"[["0","1"],"positive","-1","2"]"#,
            KeyRefusal::Decimal("-1".into()),
        ),
        (&future, r#"[["0","1"],"positive"]"#, KeyRefusal::Unresolved),
        (&future, r#"[["0","0"],"positive"]"#, KeyRefusal::Unresolved),
        (&future, r#"[["0"],"positive"]"#, KeyRefusal::Unresolved),
        (
            &future,
            r#"[["0","0"],"positive","0","1"]"#,
            KeyRefusal::Unresolved,
        ),
        (
            &shifted,
            r#"[["0","1"],"positive","2","3"]"#,
            KeyRefusal::Unresolved,
        ),
        (
            &open,
            r#"[["0","1"],"positive","1","5"]"#,
            KeyRefusal::Unresolved,
        ),
        (
            &open,
            r#"[["0","1"],"positive","3","open"]"#,
            KeyRefusal::Unresolved,
        ),
        (
            &open,
            r#"[["0","1"],"positive","0","open"]"#,
            KeyRefusal::Unresolved,
        ),
    ];
    for (states, entry, refusal) in refused {
        let key = obligation(entry);
        assert_eq!(
            states.admit(key.as_bytes()).map(|_| ()),
            Err(refusal),
            "{entry}"
        );
    }
    // Inside the checked interval, the same forms are admitted.
    for (states, entry) in [
        (&future, r#"[["0","1"],"positive","0","3"]"#),
        (&shifted, r#"[["0","1"],"positive","1","3"]"#),
        (&open, r#"[["0","1"],"positive","1","open"]"#),
    ] {
        let key = obligation(entry);
        assert!(states.admit(key.as_bytes()).is_ok(), "{entry}");
    }
}

/// FR-338 refuses a non-normal form rather than repairing it: a branch an
/// obligation-free branch subsumes, an acceptance set that disagrees with
/// what is carried, and a set naming an occurrence that is never a
/// least-fixed point at that polarity. The writer reaches the same
/// refusals, or the normal form, and never writes a key `admit` refuses.
#[trace("TC-525", "FR-338-AC-5")]
#[test]
fn non_normal_branches_and_acceptance_are_refused() {
    let mixed = states("always holds(c.level = 1) and always holds(c.level = 2)");
    let carried = Configuration {
        obligations: vec![Obligation::Current(at(&[0], Polarity::Positive))],
        ..Configuration::default()
    };
    let free = Configuration::default();
    let subsumed = states_key(r#"["safety",[[[[["0"],"positive"]],[],[],[]],[[],[],[],[]]]]"#);
    assert_eq!(
        mixed.admit(subsumed.as_bytes()).map(|_| ()),
        Err(KeyRefusal::NonNormal)
    );
    assert_eq!(
        mixed.key(&[carried.clone(), free.clone()]),
        mixed.key(&[free]),
        "the writer drops the subsumed branch"
    );

    // Acceptance membership is a Büchi fact: a branch belongs to a
    // least-fixed-point occurrence's set exactly when it is not carried.
    let response = buchi(RESPONSE, "on origin");
    let root = at(&[], Polarity::Negative);
    let elementary = |obligations: Vec<Obligation>, acceptance: Vec<Occurrence>| {
        BuchiState::Elementary(Configuration {
            obligations,
            acceptance,
            ..Configuration::default()
        })
    };
    let carried_root = vec![Obligation::Current(root.clone())];
    assert!(response
        .key(&elementary(Vec::new(), vec![root.clone()]))
        .is_ok());
    assert_eq!(
        response.key(&elementary(Vec::new(), Vec::new())),
        Err(KeyRefusal::NonNormal),
        "not carried, yet not a member"
    );
    assert_eq!(
        response.key(&elementary(carried_root, vec![root])),
        Err(KeyRefusal::NonNormal),
        "carried, yet a member"
    );
    let entry = |entry: &str| buchi_key("infinite", &format!(r#"["buchi",[[],[],[],[{entry}]]]"#));
    for (octets, refusal) in [
        // never a least-fixed point at that polarity, or never read there
        (entry(r#"[["0","1"],"negative"]"#), KeyRefusal::Unresolved),
        (entry(r#"[["0","1"],"positive"]"#), KeyRefusal::Unresolved),
        (entry(r#"[["0"],"positive"]"#), KeyRefusal::Unresolved),
        (entry(r#"[[],"negative","x"]"#), KeyRefusal::Members),
    ] {
        assert_eq!(
            response.admit(octets.as_bytes()).map(|_| ()),
            Err(refusal),
            "{octets}"
        );
    }
    // An antecedent is read at the dual polarity (and a `not` dualizes its
    // operand): under the negated root, `eventually a` there is a
    // least-fixed-point set.
    for (formula, path) in [
        (
            "eventually ((eventually holds(c.level = 1)) implies holds(c.level = 2))",
            vec![0, 0],
        ),
        (
            "eventually (not (eventually holds(c.level = 1)))",
            vec![0, 0],
        ),
    ] {
        let dual = buchi(formula, "on origin");
        assert_eq!(
            dual.key(&elementary(Vec::new(), Vec::new())),
            Err(KeyRefusal::NonNormal),
            "{formula}"
        );
        assert!(
            dual.key(&elementary(Vec::new(), vec![at(&path, Polarity::Positive)]))
                .is_ok(),
            "{formula}"
        );
        assert_eq!(
            dual.key(&elementary(Vec::new(), vec![at(&path, Polarity::Negative)])),
            Err(KeyRefusal::Unresolved),
            "{formula}"
        );
    }
}

/// The writer retains one counter per occurrence however the debts were
/// inserted (FR-338's "deadlines 1 and 3 retain deadline 1"), the furthest
/// extent, and the union of offsets; it refuses facts the context would
/// refuse rather than writing them.
#[trace("TC-525")]
#[test]
fn the_writer_retains_one_counter_per_occurrence_and_refuses_what_admit_refuses() {
    let future = states(FUTURE);
    let target = at(&[0, 1], Polarity::Positive);
    let window = |windows: Vec<Window>| Configuration {
        windows,
        ..Configuration::default()
    };
    let retained = states_key(r#"["safety",[[[],[],[[["0","1"],"positive","deadline","1"]],[]]]]"#);
    for order in [[3, 1], [1, 3]] {
        let debts = order
            .iter()
            .map(|remaining| Window::Deadline(target.clone(), *remaining))
            .collect();
        assert_eq!(
            octets(&future, &[window(debts)]),
            retained,
            "deadlines {order:?}"
        );
    }
    let dual = at(&[0, 1], Polarity::Negative);
    assert_eq!(
        octets(
            &future,
            &[window(vec![
                Window::Extent(dual.clone(), 1),
                Window::Extent(dual, 3)
            ])]
        ),
        states_key(r#"["safety",[[[],[],[[["0","1"],"negative","extent","3"]],[]]]]"#)
    );
    let shifted = states(SHIFTED);
    assert_eq!(
        octets(
            &shifted,
            &[window(vec![
                Window::Offsets(target.clone(), vec![(1, 3)]),
                Window::Offsets(target.clone(), vec![(0, 2), (1, 3)]),
            ])]
        ),
        states_key(
            r#"["safety",[[[],[],[[["0","1"],"positive","offsets",[["0","2"],["1","3"]]]],[]]]]"#
        )
    );

    let age = states("always (holds(c.level = 1) implies once[0,2] holds(c.level = 2))");
    let too_old = Configuration {
        past: vec![PastMemory::Age {
            path: vec![0, 1],
            operator: PastOperator::Once,
            age: 99,
        }],
        ..Configuration::default()
    };
    assert_eq!(age.key(&[too_old]), Err(KeyRefusal::Unresolved));
    let open = states(OPEN);
    let shifts = window(vec![
        Window::Shift(target.clone(), 1),
        Window::Shift(target, 2),
    ]);
    assert_eq!(open.key(&[shifts]), Err(KeyRefusal::Members));
}

/// FR-338's protocol-subject rows: the subject kind selects the forms, so a
/// protocol clause is translated and its keys are not the ordinary subject's.
#[trace("TC-525")]
#[test]
fn a_protocol_subject_is_translated_with_its_own_forms() {
    let fixture = fixture(INFINITE, "c: Counter", "on origin", FUTURE);
    let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
    let protocol =
        SafetyStates::clause(&fixture.clause, &context, SubjectKind::Protocol).expect("translates");
    let ordinary =
        SafetyStates::clause(&fixture.clause, &context, SubjectKind::Ordinary).expect("translates");
    // The same past facts are `distance-history` for the protocol subject and
    // `history` for the ordinary one, and each refuses the other's key.
    let window = "always (holds(c.level = 1) implies once[1,2] holds(c.level = 2))";
    let window = self::fixture(INFINITE, "c: Counter", "on origin", window);
    let context = StateContext::clause(&window.clause, occurrence(0), Vec::new());
    let protocol_window =
        SafetyStates::clause(&window.clause, &context, SubjectKind::Protocol).expect("translates");
    let ordinary_window =
        SafetyStates::clause(&window.clause, &context, SubjectKind::Ordinary).expect("translates");
    let distance = Configuration {
        past: vec![PastMemory::DistanceHistory {
            path: vec![0, 1],
            buckets: vec![bucket(1, [false, true])],
            tail: None,
        }],
        ..Configuration::default()
    };
    let history = Configuration {
        past: vec![PastMemory::History {
            path: vec![0, 1],
            operands: vec![HistoryOperand {
                path: vec![0, 1, 0],
                offsets: vec![1],
            }],
        }],
        ..Configuration::default()
    };
    assert!(protocol_window.key(std::slice::from_ref(&distance)).is_ok());
    assert!(ordinary_window.key(std::slice::from_ref(&history)).is_ok());
    assert_eq!(
        ordinary_window.key(std::slice::from_ref(&distance)),
        Err(KeyRefusal::Unresolved)
    );
    assert_eq!(
        protocol_window.key(std::slice::from_ref(&history)),
        Err(KeyRefusal::Unresolved)
    );
    // A finite `eventually` not carried is an acceptance member only for the
    // protocol subject.
    let free = [Configuration::default()];
    let member = [Configuration {
        acceptance: vec![at(&[0, 1], Polarity::Positive)],
        ..Configuration::default()
    }];
    assert!(ordinary.key(&free).is_ok());
    assert_eq!(protocol.key(&free), Err(KeyRefusal::NonNormal));
    assert!(protocol.key(&member).is_ok());
    assert_eq!(ordinary.key(&member), Err(KeyRefusal::Unresolved));
    let live = self::fixture(INFINITE, "c: Counter", "on origin", LIVE);
    let context = StateContext::clause(&live.clause, occurrence(0), Vec::new());
    assert!(BuchiStates::clause(&live.clause, &context, SubjectKind::Protocol).is_ok());
}

/// Subsumption compares facts as sets: an obligation-free branch whose past
/// memory arrives in another order than a root-carrying branch's still
/// subsumes it, in the writer and in the reader.
#[trace("TC-525")]
#[test]
fn subsumption_ignores_the_order_the_facts_arrive_in() {
    let both = states(
        "always (holds(c.level = 1) implies (once holds(c.level = 2) and historically holds(c.level = 3)))",
    );
    let once = PastMemory::Flag {
        path: vec![0, 1, 0],
        operator: PastOperator::Once,
        value: true,
    };
    let historically = PastMemory::Flag {
        path: vec![0, 1, 1],
        operator: PastOperator::Historically,
        value: true,
    };
    let free = Configuration {
        past: vec![once.clone(), historically.clone()],
        ..Configuration::default()
    };
    let carried = Configuration {
        obligations: vec![root()],
        past: vec![historically, once],
        ..Configuration::default()
    };
    assert_eq!(
        both.key(&[free.clone(), carried.clone()]),
        both.key(&[free]),
        "the writer drops the subsumed branch"
    );
    let past = r#"[[["0","1","0"],"once",true],[["0","1","1"],"historically",true]]"#;
    let key = states_key(&format!(
        r#"["safety",[[[[[],"positive"]],{past},[],[]],[[],{past},[],[]]]]"#
    ));
    assert_eq!(
        both.admit(key.as_bytes()).map(|_| ()),
        Err(KeyRefusal::NonNormal),
        "the reader refuses the pair"
    );
}

const LIVE: &str = "eventually holds(c.level = 1)";
const RESPONSE: &str = "always (holds(c.level = 1) implies eventually holds(c.level = 2))";
const GUARD: &str = "on each (q: Counter) when (c.level = 1)";

/// The TP-4 clause `body` over `c: Counter` under infinite-trace, its Büchi
/// states under `occurrence(0)`, guarded when `activation` has a `when`.
fn buchi(body: &str, activation: &str) -> BuchiStates {
    let fixture = fixture(INFINITE, "c: Counter", activation, body);
    let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
    BuchiStates::clause(&fixture.clause, &context, SubjectKind::Ordinary)
        .unwrap_or_else(|why| panic!("{body}: {why}"))
}

/// `K_C(body)` under the context of `buchi(_, activation)`.
fn buchi_key(activation: &str, body: &str) -> String {
    states_key(body).replace(r#""infinite",[]]"#, &format!(r#""{activation}",[]]"#))
}

fn text_of(key: &qsl_eval::automaton::AutomatonStateKey) -> String {
    String::from_utf8(key.as_bytes().to_vec()).expect("a key is UTF-8")
}

/// FR-163 needs one state identity for every property-automaton kind. The
/// key follows the reached configuration, not the automaton's local
/// `StateId`, so different discovery orders write the same set of keys.
#[trace("FR-338-AC-4", "FR-338-AC-7")]
#[test]
fn property_automaton_keys_are_independent_of_state_numbering() {
    fn reached_keys(body: &str, profile: &str, reverse: bool) -> BTreeMap<Vec<u8>, usize> {
        let fixture = fixture(profile, "c: Counter", "on origin", body);
        let mut automaton =
            qsl_eval::automaton::translate(&fixture.clause, 256).expect("the clause translates");
        let initial = automaton.initial().expect("initial state fits");
        let initial_key = automaton
            .state_key(initial, 1 << 20)
            .expect("the initial state's key fits");
        let exact_bound = u64::try_from(initial_key.as_bytes().len()).expect("key length fits");
        assert_eq!(
            automaton.state_key(initial, exact_bound),
            Ok(initial_key),
            "the exact byte bound admits the key"
        );
        assert_eq!(
            automaton.state_key(initial, exact_bound - 1),
            Err(qsl_eval::automaton::AutomatonError::KeyEncodingLimit {
                limit: exact_bound - 1,
            }),
            "one byte below the key length refuses"
        );
        let mut pending = vec![initial];
        let mut reached = BTreeSet::new();
        let mut keys = BTreeMap::new();
        while let Some(state) = pending.pop() {
            if !reached.insert(state) {
                continue;
            }
            let key = automaton
                .state_key(state, 1 << 20)
                .expect("a reached state has a canonical key");
            assert!(
                keys.insert(key.as_bytes().to_vec(), state.index())
                    .is_none(),
                "distinct states of one automaton have distinct keys"
            );
            let atom_count = automaton.atoms().len() + usize::from(automaton.uncounted().is_some());
            let letters: Box<dyn Iterator<Item = u64>> = if reverse {
                Box::new((0..(1_u64 << atom_count)).rev())
            } else {
                Box::new(0..(1_u64 << atom_count))
            };
            for bits in letters {
                let mut letter = Letter::from_bits(bits);
                if let Some(uncounted) = automaton.uncounted() {
                    letter = letter.with(uncounted);
                }
                pending.extend(
                    automaton
                        .successors(state, letter)
                        .expect("successors materialize"),
                );
            }
        }
        keys
    }

    for (body, profile) in [
        ("eventually[0,2] holds(c.level = 1)", EVENT),
        ("always holds(c.level = 1)", INFINITE),
        (
            "always eventually (holds(c.level = 1) and once[1,1] holds(c.level = 2))",
            INFINITE,
        ),
    ] {
        let forward = reached_keys(body, profile, false);
        let reverse = reached_keys(body, profile, true);
        assert_eq!(
            forward.keys().collect::<Vec<_>>(),
            reverse.keys().collect::<Vec<_>>()
        );
    }
    let forward = reached_keys(
        "always eventually (holds(c.level = 1) and once[1,1] holds(c.level = 2))",
        INFINITE,
        false,
    );
    let reverse = reached_keys(
        "always eventually (holds(c.level = 1) and once[1,1] holds(c.level = 2))",
        INFINITE,
        true,
    );
    assert!(forward
        .iter()
        .any(|(key, first)| { reverse.get(key).is_some_and(|second| second != first) }));

    let bounded = reached_keys("eventually[0,2] holds(c.level = 1)", EVENT, false);
    let safety = reached_keys("always holds(c.level = 1)", INFINITE, false);
    let liveness = reached_keys(
        "always eventually (holds(c.level = 1) and once[1,1] holds(c.level = 2))",
        INFINITE,
        false,
    );
    for (left_name, left, right_name, right) in [
        ("bounded", &bounded, "safety", &safety),
        ("bounded", &bounded, "liveness", &liveness),
        ("safety", &safety, "liveness", &liveness),
    ] {
        assert!(
            left.keys().all(|key| !right.contains_key(key)),
            "{left_name} and {right_name} automaton keys are distinct across kinds/formulas"
        );
    }
}

/// FR-338 TP-4: each reached PropertyAutomaton StateId resolves to a
/// context-admitted structural key, including past history for both subject
/// key algebras.
#[trace("TC-525", "FR-338-AC-23")]
#[test]
fn reached_tp4_engine_states_resolve_to_fr338_keys() {
    let mut different_ids_for_same_key = BTreeMap::new();
    for body in [
        "always eventually (holds(p) and once[1,1] holds(q))",
        "eventually (always[0,2] eventually[0,1] holds(p))",
        "eventually (holds(p) and eventually holds(q))",
        "eventually holds(p) and eventually holds(q)",
    ] {
        let fixture = fixture(INFINITE, "c: Counter", "on origin", body);
        let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
        for subject in [SubjectKind::Ordinary, SubjectKind::Protocol] {
            let keys = BuchiStates::clause(&fixture.clause, &context, subject)
                .expect("the TP-4 key context translates");
            let mut automaton = match subject {
                SubjectKind::Ordinary => qsl_eval::automaton::translate(&fixture.clause, 256),
                SubjectKind::Protocol => {
                    qsl_eval::automaton::translate_counted(&fixture.clause, 256)
                }
            }
            .expect("the clause translates");
            let initial = automaton.initial().expect("initial state fits");
            let mut reached = BTreeSet::new();
            let mut by_key = BTreeMap::new();
            let mut pending = vec![initial];
            while let Some(from) = pending.pop() {
                if !reached.insert(from) {
                    continue;
                }
                let structural = automaton
                    .buchi_state(from, keys.max_key_bytes())
                    .expect("the run-local state resolves structurally");
                let key = keys.key(&structural).unwrap_or_else(|error| {
                    panic!("{body} ({subject:?}) state {structural:?}: {error}")
                });
                let octets = key.as_bytes().to_vec();
                assert_eq!(keys.admit(key.as_bytes()), Ok((key, structural)));
                by_key.insert(octets, from);
                for bits in 0..4 {
                    let mut letter = Letter::from_bits(bits);
                    if subject == SubjectKind::Protocol {
                        if let Some(uncounted) = automaton.uncounted() {
                            letter = letter.with(uncounted);
                        }
                    }
                    pending.extend(
                        automaton
                            .successors(from, letter)
                            .expect("successors materialize"),
                    );
                }
            }
            assert!(!reached.is_empty(), "the test must reach a TP-4 state");

            let mut reversed = match subject {
                SubjectKind::Ordinary => qsl_eval::automaton::translate(&fixture.clause, 256),
                SubjectKind::Protocol => {
                    qsl_eval::automaton::translate_counted(&fixture.clause, 256)
                }
            }
            .expect("the clause translates");
            let initial = reversed.initial().expect("initial state fits");
            let mut reached = BTreeSet::new();
            let mut reversed_by_key = BTreeMap::new();
            let mut pending = vec![initial];
            while let Some(from) = pending.pop() {
                if !reached.insert(from) {
                    continue;
                }
                let structural = reversed
                    .buchi_state(from, keys.max_key_bytes())
                    .expect("the run-local state resolves structurally");
                let key = keys
                    .key(&structural)
                    .unwrap_or_else(|error| panic!("{body} ({subject:?}): {error}"));
                reversed_by_key.insert(key.as_bytes().to_vec(), from);
                for bits in (0..4).rev() {
                    let mut letter = Letter::from_bits(bits);
                    if subject == SubjectKind::Protocol {
                        if let Some(uncounted) = reversed.uncounted() {
                            letter = letter.with(uncounted);
                        }
                    }
                    pending.extend(
                        reversed
                            .successors(from, letter)
                            .expect("successors materialize"),
                    );
                }
            }
            assert_eq!(
                by_key.keys().collect::<BTreeSet<_>>(),
                reversed_by_key.keys().collect()
            );
            let differs = by_key
                .iter()
                .any(|(key, state)| reversed_by_key.get(key).is_some_and(|other| other != state));
            different_ids_for_same_key.insert((body, subject == SubjectKind::Protocol), differs);
        }
    }
    // The past-operand body reaches one key through different `StateId`s in
    // the two discovery orders, under both key algebras.
    let past = "always eventually (holds(p) and once[1,1] holds(q))";
    for protocol in [false, true] {
        assert_eq!(
            different_ids_for_same_key.get(&(past, protocol)),
            Some(&true)
        );
    }
}

/// FR-338's TP-4 vector: for `eventually holds(p)` the elementary
/// configuration that carries the negated `G not p` is
/// `["buchi",[[[[],"negative"]],[],[],[]]]`, with no acceptance set for the
/// dual `G`. A carried least-fixed-point occurrence has no acceptance
/// membership, and one not carried has it. The key is admitted back as the
/// state it was written from.
#[trace("TC-525")]
#[test]
fn a_tp4_key_is_one_configuration_of_the_negated_formula() {
    let live = buchi(LIVE, "on origin");
    let carry = Configuration {
        obligations: vec![Obligation::Current(at(&[], Polarity::Negative))],
        ..Configuration::default()
    };
    let state = BuchiState::Elementary(carry);
    let key = live.key(&state).expect("encodes");
    assert_eq!(
        text_of(&key),
        states_key(r#"["buchi",[[[[],"negative"]],[],[],[]]]"#)
    );
    assert_eq!(live.admit(key.as_bytes()), Ok((key, state)));

    // `always (p implies eventually q)` negates to `eventually (p and G not
    // q)`: its root is a least-fixed-point occurrence at negative polarity.
    let response = buchi(RESPONSE, "on origin");
    let root = at(&[], Polarity::Negative);
    let carried = BuchiState::Elementary(Configuration {
        obligations: vec![Obligation::Current(root.clone())],
        ..Configuration::default()
    });
    let discharged = BuchiState::Elementary(Configuration {
        acceptance: vec![root.clone()],
        ..Configuration::default()
    });
    assert_eq!(
        text_of(&response.key(&carried).expect("encodes")),
        states_key(r#"["buchi",[[[[],"negative"]],[],[],[]]]"#)
    );
    assert_eq!(
        text_of(&response.key(&discharged).expect("encodes")),
        states_key(r#"["buchi",[[],[],[],[[[],"negative"]]]]"#)
    );
    let both = BuchiState::Elementary(Configuration {
        obligations: vec![Obligation::Current(root.clone())],
        acceptance: vec![root],
        ..Configuration::default()
    });
    assert_eq!(response.key(&both), Err(KeyRefusal::NonNormal));
    assert_eq!(
        response.key(&BuchiState::Elementary(Configuration::default())),
        Err(KeyRefusal::NonNormal)
    );
}

/// FR-338's guarded TP-4 vectors: before any activation the state is
/// `["guarded-buchi","waiting",[[],[],[],[]]]` (the negated `G not p` has no
/// least-fixed-point set, so the derived acceptance is empty), and after an
/// activation is selected where p is false,
/// `["guarded-buchi","active",[[[[],"negative"]],[],[],[]]]`.
#[trace("TC-525")]
#[test]
fn a_guarded_tp4_key_carries_its_phase() {
    let guarded = buchi(LIVE, GUARD);
    let waiting = BuchiState::Guarded(BuchiPhase::Waiting, Configuration::default());
    let active = BuchiState::Guarded(
        BuchiPhase::Active,
        Configuration {
            obligations: vec![Obligation::Current(at(&[], Polarity::Negative))],
            ..Configuration::default()
        },
    );
    for (state, body) in [
        (&waiting, r#"["guarded-buchi","waiting",[[],[],[],[]]]"#),
        (
            &active,
            r#"["guarded-buchi","active",[[[[],"negative"]],[],[],[]]]"#,
        ),
    ] {
        let key = guarded.key(state).expect("encodes");
        assert_eq!(text_of(&key), buchi_key("each-when", body));
        assert_eq!(guarded.admit(key.as_bytes()), Ok((key, state.clone())));
    }
    assert_ne!(
        guarded.key(&waiting).expect("encodes"),
        guarded.key(&active).expect("encodes"),
        "the waiting key stays reachable beside the active one"
    );
    // A waiting branch derives its membership: the dual `G` has none, but
    // `always (p implies eventually q)` negates to a least-fixed point root.
    let response = buchi(RESPONSE, GUARD);
    let member = Configuration {
        acceptance: vec![at(&[], Polarity::Negative)],
        ..Configuration::default()
    };
    assert!(response
        .key(&BuchiState::Guarded(BuchiPhase::Waiting, member))
        .is_ok());
    assert_eq!(
        response.key(&BuchiState::Guarded(
            BuchiPhase::Waiting,
            Configuration::default()
        )),
        Err(KeyRefusal::NonNormal)
    );
}

/// FR-338's Büchi admission negatives: a `guarded-buchi` state under a
/// context that is not `"each-when"`, a `buchi` state under an `"each-when"`
/// context, a phase other than `waiting` or `active`, a waiting branch with
/// an obligation or a window entry, extra and missing members, and a
/// clause that is not TP-4.
#[trace("TC-525", "FR-338-AC-5")]
#[test]
fn buchi_keys_refuse_the_wrong_form_phase_and_waiting_facts() {
    let plain = buchi(LIVE, "on origin");
    let guarded = buchi(LIVE, GUARD);
    let branch = r#"[[[[],"negative"]],[],[],[]]"#;
    let activation = r#"[[[[],"negative"]],[],[],[["activation"]]]"#;
    let empty = "[[],[],[],[]]";
    let in_context = |activation: &str, body: String| buchi_key(activation, &body);
    let cases = [
        // a supplied `["activation"]` member, which is derived from the phase
        (
            &plain,
            in_context("infinite", format!(r#"["buchi",{activation}]"#)),
            KeyRefusal::Members,
        ),
        (
            &guarded,
            in_context(
                "each-when",
                format!(r#"["guarded-buchi","active",{activation}]"#),
            ),
            KeyRefusal::Members,
        ),
        (
            &plain,
            in_context(
                "infinite",
                format!(r#"["guarded-buchi","active",{branch}]"#),
            ),
            KeyRefusal::WrongForm("guarded-buchi".into()),
        ),
        (
            &guarded,
            in_context("each-when", format!(r#"["buchi",{branch}]"#)),
            KeyRefusal::WrongForm("buchi".into()),
        ),
        (
            &guarded,
            in_context(
                "each-when",
                format!(r#"["guarded-buchi","selected",{branch}]"#),
            ),
            KeyRefusal::Members,
        ),
        (
            &guarded,
            in_context(
                "each-when",
                format!(r#"["guarded-buchi","waiting",{branch}]"#),
            ),
            KeyRefusal::Members,
        ),
        (
            &guarded,
            in_context(
                "each-when",
                r#"["guarded-buchi","waiting",[[],[],[[[],"positive","deadline","1"]],[]]]"#
                    .to_owned(),
            ),
            KeyRefusal::Unresolved,
        ),
        (
            &guarded,
            in_context("each-when", format!(r#"["guarded-buchi",{empty}]"#)),
            KeyRefusal::Members,
        ),
        (
            &guarded,
            in_context(
                "each-when",
                format!(r#"["guarded-buchi","active",{empty},{empty}]"#),
            ),
            KeyRefusal::Members,
        ),
        (
            &plain,
            in_context("infinite", format!(r#"["buchi",{branch},{branch}]"#)),
            KeyRefusal::Members,
        ),
        (
            &plain,
            in_context("infinite", r#"["buchi"]"#.to_owned()),
            KeyRefusal::Members,
        ),
        (
            &plain,
            in_context("infinite", format!(r#"["safety",[{branch}]]"#)),
            KeyRefusal::WrongForm("safety".into()),
        ),
        (
            &plain,
            in_context("infinite", format!(r#"["tableau",{branch}]"#)),
            KeyRefusal::UnknownTag("tableau".into()),
        ),
        (
            &plain,
            buchi_key("each-when", &format!(r#"["buchi",{branch}]"#)),
            KeyRefusal::ForeignContext,
        ),
    ];
    for (states, octets, refusal) in cases {
        assert_eq!(
            states.admit(octets.as_bytes()).map(|_| ()),
            Err(refusal),
            "{octets}"
        );
    }
    let safety = fixture(
        INFINITE,
        "c: Counter",
        "on origin",
        "always holds(c.level = 1)",
    );
    let context = StateContext::clause(&safety.clause, occurrence(0), Vec::new());
    assert!(matches!(
        BuchiStates::clause(&safety.clause, &context, SubjectKind::Ordinary),
        Err(TranslationRefusal::NotTranslated(_))
    ));
    // A closed interval under an unbounded `always` is TP-3, not TP-4.
    let bounded = fixture(INFINITE, "c: Counter", "on origin", FUTURE);
    let context = StateContext::clause(&bounded.clause, occurrence(0), Vec::new());
    assert!(matches!(
        BuchiStates::clause(&bounded.clause, &context, SubjectKind::Ordinary),
        Err(TranslationRefusal::NotTranslated(_))
    ));
    // A guarded clause has the `each-when` context, which the two-state
    // reachable-invariant translation does not read.
    let guarded_invariant = fixture(INFINITE, "c: Counter", GUARD, "always holds(c.level = 1)");
    let context = StateContext::clause(&guarded_invariant.clause, occurrence(0), Vec::new());
    assert_eq!(context.activation, ContextActivation::EachWhen);
    assert!(matches!(
        SafetyTranslation::reachable_invariant(&guarded_invariant.clause, &context),
        Err(TranslationRefusal::NotTranslated(_))
    ));
    assert_eq!(
        plain.key(&BuchiState::Guarded(
            BuchiPhase::Active,
            Configuration::default()
        )),
        Err(KeyRefusal::WrongForm("guarded-buchi".into()))
    );
}

/// FR-338-AC-4: every canonical octet vector of the "Canonical octet
/// vectors" table is asserted as exact ASCII, with the full `K_C` where the
/// table gives one: the TP-1 bad prefix and carried root, the bounded
/// `eventually[0,1]` row with its `accept` and `reject` successors, the
/// deadlines 1 and 3 that retain deadline 1, two distinct interval
/// occurrences, the general offset set and its reversal, the TP-4 `buchi`
/// carry, both `guarded-buchi` rows, both `each-buchi` rows and the deadlock item's complete key
/// with its refused profile and activation replacements. Reversing the
/// insertion order of subsets and windows leaves the bytes equal, distinct
/// occurrence paths and differing live facts leave them unequal, and no
/// enumeration index is a member: an independently built translation of
/// the same facts writes the same octets. The rest of the table is here too:
/// a false fork before the counted step leaves deadline 1, subset branches
/// reversed as insertion order write the same bytes and as canonical input
/// refuse, the clause-root ordinals "0" and u64::MAX are decimal strings in
/// the full `K_C` while a JSON-number ordinal and "18446744073709551616" are
/// refused, and the TP-4 `buchi` carry is not the positive F `safety` carry.
#[trace("TC-525", "FR-338-AC-4")]
#[test]
fn every_canonical_octet_vector_is_the_authored_octets() {
    // TP-1, deadlock: the complete key, and the refused replacements.
    let deadlock = deadlock();
    assert_eq!(text(&deadlock, SafetyState::Carried), DEADLOCK_CARRIED);
    for foreign in [
        DEADLOCK_CARRIED.replace(
            "quire.temporal.infinite-trace/v1",
            "quire.temporal.event-position.false-extension/v1",
        ),
        DEADLOCK_CARRIED.replace("\"infinite\"", "\"origin\""),
    ] {
        assert_eq!(
            deadlock.admit(foreign.as_bytes()).map(|_| ()),
            Err(KeyRefusal::ForeignContext)
        );
    }
    let bad = clause("always holds(false)", 0);
    assert!(text(&bad, bad.read(None, false)).ends_with(r#",["safety",[]]]"#));
    let carried = clause("always holds(true)", u64::MAX);
    assert!(
        text(&carried, carried.read(Some(carried.read(None, true)), true))
            .ends_with(r#"["safety",[[[[[],"positive"]],[],[],[]]]]]"#)
    );

    // Bounded `eventually[0,1] holds(p)` on origin.
    let fixture = fixture(
        EVENT,
        "c: Counter",
        "on origin",
        "eventually[0,1] holds(true)",
    );
    let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
    let bounded = BoundedTranslation::clause(&fixture.clause, &context).expect("translates");
    let bounded_text = |state| {
        String::from_utf8(bounded.key(state).expect("encodes").as_bytes().to_vec()).expect("UTF-8")
    };
    let origin = bounded.start(false);
    let event_context = |activation: &str| {
        states_context()
            .replace("infinite-trace", "event-position.false-extension")
            .replace("\"infinite\"", &format!("\"{activation}\""))
    };
    let bounded_key = |body: &str| {
        format!(
            r#"["quire.automaton-state-key/v1",{},{body}]"#,
            event_context("origin")
        )
    };
    assert_eq!(
        bounded_text(origin),
        bounded_key(r#"["bounded",["origin","1"],[[[],[],[[[],"positive","deadline","1"]],[]]]]"#)
    );
    assert_eq!(
        bounded_text(bounded.step(origin, true, true)),
        bounded_key(r#"["accept"]"#)
    );
    let last = bounded.step(origin, true, false);
    assert_eq!(
        bounded_text(bounded.step(last, true, true)),
        bounded_key(r#"["reject"]"#)
    );

    // Deadlines 1 and 3 retain deadline 1 in either insertion order; an
    // extent would change the future and is refused.
    let pending = states("eventually[0,3] holds(c.level = 1)");
    let deadline = |remaining| Window::Deadline(at(&[], Polarity::Positive), remaining);
    let debts = |windows: Vec<Window>| Configuration {
        windows,
        ..Configuration::default()
    };
    let retained = states_key(r#"["safety",[[[],[],[[[],"positive","deadline","1"]],[]]]]"#);
    for order in [
        vec![deadline(1), deadline(3)],
        vec![deadline(3), deadline(1)],
    ] {
        assert_eq!(octets(&pending, &[debts(order)]), retained);
    }
    assert_eq!(
        pending.key(&[debts(vec![Window::Extent(at(&[], Polarity::Positive), 3)])]),
        Err(KeyRefusal::Unresolved)
    );

    // Two distinct interval occurrences with equal counters keep both
    // windows; changing either counter changes the bytes.
    let two = states(TWO_FUTURES);
    let at =
        |path: &[u64], remaining| Window::Deadline(self::at(path, Polarity::Positive), remaining);
    let equal = octets(&two, &[debts(vec![at(&[0], 1), at(&[1], 1)])]);
    assert_eq!(
        equal,
        states_key(
            r#"["safety",[[[],[],[[["0"],"positive","deadline","1"],[["1"],"positive","deadline","1"]],[]]]]"#
        )
    );
    assert_eq!(
        equal,
        octets(&two, &[debts(vec![at(&[1], 1), at(&[0], 1)])]),
        "window insertion order"
    );
    for changed in [(at(&[0], 2), at(&[1], 1)), (at(&[0], 1), at(&[1], 2))] {
        assert_ne!(equal, octets(&two, &[debts(vec![changed.0, changed.1])]));
    }
    assert_ne!(
        octets(&two, &[debts(vec![at(&[0], 1)])]),
        octets(&two, &[debts(vec![at(&[1], 1)])]),
        "distinct occurrence paths"
    );

    // The general offset set `{[0,2],[1,3]}` and its reversal.
    let shifted = states(SHIFTED);
    let offsets = Window::Offsets(self::at(&[0, 1], Polarity::Positive), vec![(1, 3), (0, 2)]);
    let general = octets(&shifted, &[debts(vec![offsets])]);
    assert!(general.contains(r#"[["0","2"],["1","3"]]"#));
    let reversed = general.replace(r#"[["0","2"],["1","3"]]"#, r#"[["1","3"],["0","2"]]"#);
    assert_eq!(
        shifted.admit(reversed.as_bytes()).map(|_| ()),
        Err(KeyRefusal::Order)
    );

    // A false fork before the counted step leaves the deadline at 1: the
    // state is the origin's again, and does not reject.
    let forked = bounded.step(origin, false, false);
    assert_eq!(bounded_text(forked), bounded_text(origin));

    // Subset branches: insertion order does not change the bytes, and the
    // reversed array supplied as canonical input refuses.
    let one_each = |path: &[u64]| debts(vec![at(path, 1)]);
    let branches = octets(&two, &[one_each(&[0]), one_each(&[1])]);
    assert_eq!(branches, octets(&two, &[one_each(&[1]), one_each(&[0])]));
    let first = r#"[[],[],[[["0"],"positive","deadline","1"]],[]]"#;
    let second = r#"[[],[],[[["1"],"positive","deadline","1"]],[]]"#;
    assert_eq!(
        branches,
        states_key(&format!(r#"["safety",[{first},{second}]]"#))
    );
    assert_eq!(
        two.admit(states_key(&format!(r#"["safety",[{second},{first}]]"#)).as_bytes())
            .map(|_| ()),
        Err(KeyRefusal::Order)
    );

    // The clause-root lexical vector: ordinals are decimal strings. A
    // JSON-number ordinal and 2^64 are never the context of the clause at
    // ordinal 0 or u64::MAX.
    let node = format!(
        r#"{{"digest":"{}","domain":"quire.checked-semantic-node/v1"}}"#,
        "ab".repeat(32)
    );
    let rooted = |ordinal: &str| {
        format!(
            r#"["quire.automaton-state-key/v1",[[{node},"temporal",{ordinal}],"quire.temporal.infinite-trace/v1","infinite",[]],["safety",[[[[[],"positive"]],[],[],[]]]]]"#
        )
    };
    for (ordinal, zero) in [("0", true), ("18446744073709551615", false)] {
        let truth = clause("always holds(true)", if zero { 0 } else { u64::MAX });
        let written = text(&truth, truth.read(Some(truth.read(None, true)), true));
        assert_eq!(written, rooted(&format!(r#""{ordinal}""#)), "{ordinal}");
        assert!(truth.admit(written.as_bytes()).is_ok());
        // A bare number is the same value as no string: JCS spells the
        // largest ones in exponent form, which is not canonical input.
        let number = if zero {
            ("0", KeyRefusal::ForeignContext)
        } else {
            (ordinal, KeyRefusal::NotCanonical)
        };
        for (wrong, refusal) in [
            number,
            (r#""18446744073709551616""#, KeyRefusal::ForeignContext),
            ("18446744073709551616", KeyRefusal::NotCanonical),
        ] {
            assert_eq!(
                truth.admit(rooted(wrong).as_bytes()).map(|_| ()),
                Err(refusal),
                "{wrong}"
            );
        }
    }

    // TP-4 and guarded TP-4.
    let live = buchi(LIVE, "on origin");
    let carry = Configuration {
        obligations: vec![Obligation::Current(self::at(&[], Polarity::Negative))],
        ..Configuration::default()
    };
    assert_eq!(
        text_of(
            &live
                .key(&BuchiState::Elementary(carry.clone()))
                .expect("encodes")
        ),
        states_key(r#"["buchi",[[[[],"negative"]],[],[],[]]]"#)
    );
    // The positive F carry is a different state: the `buchi` key the engine
    // writes for the negated carry is not the `safety` carry of the root.
    let positive_carry = states_key(r#"["safety",[[[[[],"positive"]],[],[],[]]]]"#);
    let written = text_of(
        &live
            .key(&BuchiState::Elementary(carry.clone()))
            .expect("encodes"),
    );
    assert_ne!(written, positive_carry);
    assert_eq!(
        live.admit(positive_carry.as_bytes()).map(|_| ()),
        Err(KeyRefusal::WrongForm("safety".into()))
    );
    let guarded = buchi(LIVE, GUARD);
    for (state, body) in [
        (
            BuchiState::Guarded(BuchiPhase::Waiting, Configuration::default()),
            r#"["guarded-buchi","waiting",[[],[],[],[]]]"#,
        ),
        (
            BuchiState::Guarded(BuchiPhase::Active, carry),
            r#"["guarded-buchi","active",[[[[],"negative"]],[],[],[]]]"#,
        ),
    ] {
        assert_eq!(
            text_of(&guarded.key(&state).expect("encodes")),
            buchi_key("each-when", body)
        );
    }

    // Unguarded `on each` TP-4: the activation is "each", the waiting state
    // is empty and the active one carries the negated root; neither is the
    // phase-less origin `buchi` key.
    let each = buchi(LIVE, "on each (q: Counter)");
    for (state, body) in [
        (
            BuchiState::Each(BuchiPhase::Waiting, Configuration::default()),
            r#"["each-buchi","waiting",[[],[],[],[]]]"#,
        ),
        (
            BuchiState::Each(
                BuchiPhase::Active,
                Configuration {
                    obligations: vec![Obligation::Current(self::at(&[], Polarity::Negative))],
                    ..Configuration::default()
                },
            ),
            r#"["each-buchi","active",[[[[],"negative"]],[],[],[]]]"#,
        ),
    ] {
        assert_eq!(
            text_of(&each.key(&state).expect("encodes")),
            buchi_key("each", body)
        );
    }

    // No enumeration index: independent translations of the same facts
    // write the same octets.
    assert_eq!(
        octets(
            &states(TWO_FUTURES),
            &[debts(vec![at(&[0], 1), at(&[1], 1)])]
        ),
        equal
    );
}

/// A TP-4 clause is not translated when its key form is not derived: a
/// clause checked under another profile, a context whose activation is not
/// the clause's, and any clause the safety key cannot carry.
#[trace("TC-525")]
#[test]
fn tp4_clauses_without_a_derived_key_form_are_not_translated() {
    let refused = |profile: &str, activation: &str, body: &str| {
        let fixture = fixture(profile, "c: Counter", activation, body);
        let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
        BuchiStates::clause(&fixture.clause, &context, SubjectKind::Ordinary).map(|_| ())
    };
    for (activation, body) in [
        // not unbounded, so a safety clause
        ("on origin", "eventually[0,3] holds(c.level = 1)"),
        ("on each (q: Counter)", "eventually[0,3] holds(c.level = 1)"),
    ] {
        assert!(
            matches!(
                refused(INFINITE, activation, body),
                Err(TranslationRefusal::NotTranslated(_))
            ),
            "{activation}: {body}"
        );
    }
    // The same shapes with a closed upper bound are keyed, and so is an
    // open-upper eventuality in the negated closure (#1516 R-2).
    for body in [
        "eventually always[2,*] holds(c.level = 1)",
        "eventually always[0,3] holds(c.level = 1)",
        "eventually always holds(c.level = 1)",
        // an open-lower, open-upper eventuality is liveness
        "eventually[2,*] holds(c.level = 1)",
    ] {
        assert!(refused(INFINITE, "on origin", body).is_ok(), "{body}");
    }
    // A context whose activation is not the clause's is refused.
    let each = fixture(INFINITE, "c: Counter", "on each (q: Counter)", LIVE);
    let each_context = StateContext::clause(&each.clause, occurrence(0), Vec::new());
    assert_eq!(each_context.activation, ContextActivation::Each);
    let origin = fixture(INFINITE, "c: Counter", "on origin", LIVE);
    let origin_context = StateContext::clause(&origin.clause, occurrence(0), Vec::new());
    assert_eq!(origin_context.activation, ContextActivation::Infinite);
    for (clause, context) in [(&origin, &each_context), (&each, &origin_context)] {
        assert!(matches!(
            BuchiStates::clause(&clause.clause, context, SubjectKind::Ordinary),
            Err(TranslationRefusal::NotTranslated(_))
        ));
    }
    // A safety clause under `on each` keeps the "infinite" activation.
    let safety_each = fixture(INFINITE, "c: Counter", "on each (q: Counter)", FUTURE);
    assert_eq!(
        StateContext::clause(&safety_each.clause, occurrence(0), Vec::new()).activation,
        ContextActivation::Infinite
    );

    // Another profile is not the context's.
    let live = fixture(INFINITE, "c: Counter", "on origin", LIVE);
    let other = fixture(
        EVENT,
        "c: Counter",
        "on origin",
        "eventually[0,3] holds(c.level = 1)",
    );
    let context = StateContext::clause(&other.clause, occurrence(0), Vec::new());
    assert!(BuchiStates::clause(&live.clause, &context, SubjectKind::Ordinary).is_err());

    // A context of another profile is refused, though its activation fits.
    let timed = crate::timed_trace::timed("always holds(c.level = 1)");
    let timed_context = StateContext::clause(&timed.clause, occurrence(0), Vec::new());
    assert_eq!(timed_context.activation, ContextActivation::Infinite);
    assert!(BuchiStates::clause(&live.clause, &timed_context, SubjectKind::Ordinary).is_err());
    let safe = fixture(INFINITE, "c: Counter", "on origin", FUTURE);
    assert!(SafetyStates::clause(&safe.clause, &timed_context, SubjectKind::Ordinary).is_err());

    // A TP-4 clause has no safety key, and a safety clause no Büchi key.
    let context = StateContext::clause(&live.clause, occurrence(0), Vec::new());
    assert!(SafetyStates::clause(&live.clause, &context, SubjectKind::Ordinary).is_err());
    let safe = fixture(INFINITE, "c: Counter", "on origin", FUTURE);
    let context = StateContext::clause(&safe.clause, occurrence(0), Vec::new());
    assert!(BuchiStates::clause(&safe.clause, &context, SubjectKind::Ordinary).is_err());
}

/// A guarded TP-4 key in the waiting phase carries no window: the waiting
/// rule is reached with a formula that has an interval, not one that cannot
/// resolve the window at all.
#[trace("TC-525")]
#[test]
fn a_waiting_branch_cannot_carry_a_window() {
    let guarded = buchi("eventually always[0,3] holds(c.level = 1)", GUARD);
    let window = r#"[[],[],[[["0"],"negative","deadline","1"]],[]]"#;
    let waiting = |branch: &str| {
        buchi_key(
            "each-when",
            &format!(r#"["guarded-buchi","waiting",{branch}]"#),
        )
    };
    assert_eq!(
        guarded.admit(waiting(window).as_bytes()).map(|_| ()),
        Err(KeyRefusal::Members)
    );
    assert!(guarded.admit(waiting("[[],[],[],[]]").as_bytes()).is_ok());
}

/// A timed-profile clause is neither a safety nor a Büchi state: its
/// intervals measure time, not steps, and the key algebra is over steps.
#[trace("TC-525")]
#[test]
fn a_timed_clause_is_not_keyed() {
    for body in ["always holds(c.level = 1)", "eventually holds(c.level = 1)"] {
        let timed = crate::timed_trace::timed(body);
        let context = StateContext::clause(&timed.clause, occurrence(0), Vec::new());
        assert_eq!(context.activation, ContextActivation::Infinite);
        assert!(
            matches!(
                SafetyStates::clause(&timed.clause, &context, SubjectKind::Ordinary),
                Err(TranslationRefusal::NotTranslated(_))
            ),
            "{body}"
        );
        assert!(
            matches!(
                BuchiStates::clause(&timed.clause, &context, SubjectKind::Ordinary),
                Err(TranslationRefusal::NotTranslated(_))
            ),
            "{body}"
        );
    }
}

/// Over a model subject the checked property form decides: liveness is a
/// Büchi key and not a safety key, a safety clause the reverse.
#[trace("TC-525")]
#[test]
fn a_model_subject_is_keyed_by_its_checked_property_form() {
    let keyed = |body: &str| {
        let model = crate::temporal_trace::modelled(body);
        assert!(model.clause.property_form().is_some(), "{body}");
        let context = StateContext::clause(&model.clause, occurrence(0), Vec::new());
        (
            SafetyStates::clause(&model.clause, &context, SubjectKind::Ordinary).is_ok(),
            BuchiStates::clause(&model.clause, &context, SubjectKind::Ordinary).is_ok(),
        )
    };
    assert_eq!(keyed("eventually holds(c.level = 1)"), (false, true));
    assert_eq!(keyed("always holds(c.level = 1)"), (true, false));
    assert_eq!(
        keyed("always (holds(c.level = 1) implies eventually[0,3] holds(c.level = 2))"),
        (true, false)
    );
}

/// The protocol subject's `states(_)`.
fn protocol(body: &str) -> SafetyStates {
    let fixture = fixture(INFINITE, "c: Counter", "on origin", body);
    let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
    SafetyStates::clause(&fixture.clause, &context, SubjectKind::Protocol)
        .unwrap_or_else(|why| panic!("{body}: {why}"))
}

fn bucket(distance: u64, transform: [bool; 2]) -> DistanceBucket {
    DistanceBucket {
        distance,
        transform,
    }
}

/// FR-338: on a protocol subject a general finite past window and a shifted
/// open one are `[o,"distance-history",buckets,tail]`, buckets of
/// `[distance,[false_result,true_result]]` sorted by complete bytes, the
/// distances `0..b` (`0..a`), `null` tail for a finite window and a Boolean
/// for a shift. The ordinary `history`/`shift-history` are refused there,
/// `distance-history` is refused on an ordinary subject, and the flag and
/// age forms are the same on both.
#[trace("TC-525")]
#[test]
fn a_protocol_past_window_is_distance_history() {
    let entry = |states: &SafetyStates, memory: PastMemory, text: &str| {
        let configuration = Configuration {
            past: vec![memory],
            ..Configuration::default()
        };
        let key = octets(states, std::slice::from_ref(&configuration));
        assert_eq!(
            key,
            states_key(&format!(r#"["safety",[[[],[{text}],[],[]]]]"#)),
            "{text}"
        );
        assert_eq!(
            states.admit(key.as_bytes()).map(|(_, branches)| branches),
            Ok(vec![configuration]),
            "{text}"
        );
    };
    let path = vec![0, 1];
    // Distance 10 sorts before distance 2 by complete bytes.
    let finite = protocol("always (holds(c.level = 1) implies once[1,10] holds(c.level = 2))");
    entry(
        &finite,
        PastMemory::DistanceHistory {
            path: path.clone(),
            buckets: vec![bucket(10, [false, true]), bucket(2, [true, true])],
            tail: None,
        },
        r#"[["0","1"],"distance-history",[["10",[false,true]],["2",[true,true]]],null]"#,
    );
    entry(
        &finite,
        PastMemory::DistanceHistory {
            path: path.clone(),
            buckets: vec![bucket(0, [true, true])],
            tail: None,
        },
        r#"[["0","1"],"distance-history",[["0",[true,true]]],null]"#,
    );
    let shifted =
        protocol("always (holds(c.level = 1) implies historically[2,*] holds(c.level = 2))");
    entry(
        &shifted,
        PastMemory::DistanceHistory {
            path: path.clone(),
            buckets: vec![bucket(0, [false, true]), bucket(2, [false, false])],
            tail: Some(true),
        },
        r#"[["0","1"],"distance-history",[["0",[false,true]],["2",[false,false]]],true]"#,
    );
    // The same facts in another bucket order write the same bytes.
    assert_eq!(
        octets(
            &finite,
            &[Configuration {
                past: vec![PastMemory::DistanceHistory {
                    path: path.clone(),
                    buckets: vec![bucket(2, [true, true]), bucket(10, [false, true])],
                    tail: None,
                }],
                ..Configuration::default()
            }]
        ),
        states_key(
            r#"["safety",[[[],[[["0","1"],"distance-history",[["10",[false,true]],["2",[true,true]]],null]],[],[]]]]"#
        )
    );
    assert_eq!(
        octets(
            &shifted,
            &[Configuration {
                past: vec![PastMemory::DistanceHistory {
                    path: path.clone(),
                    buckets: vec![bucket(2, [false, false]), bucket(0, [false, true])],
                    tail: Some(true),
                }],
                ..Configuration::default()
            }]
        ),
        states_key(
            r#"["safety",[[[],[[["0","1"],"distance-history",[["0",[false,true]],["2",[false,false]]],true]],[],[]]]]"#
        )
    );

    let state = |text: &str| states_key(&format!(r#"["safety",[[[],[{text}],[],[]]]]"#));
    let ordinary = states("always (holds(c.level = 1) implies once[1,10] holds(c.level = 2))");
    let ordinary_shift =
        states("always (holds(c.level = 1) implies historically[2,*] holds(c.level = 2))");
    let history = r#"[["0","1"],"history",[[["0","1","0"],["1"]]]]"#;
    let shift_history = r#"[["0","1"],"shift-history",[[["0","1","0"],["1"]]],false]"#;
    let wide = r#"[["0","1"],"distance-history",[["0",[false,true]]],null]"#;
    let shift = r#"[["0","1"],"distance-history",[["0",[false,true]]],false]"#;
    let cases = [
        (&finite, history, KeyRefusal::Unresolved),
        (&shifted, shift_history, KeyRefusal::Unresolved),
        (&ordinary, wide, KeyRefusal::Unresolved),
        (&ordinary_shift, shift, KeyRefusal::Unresolved),
        // a distance beyond b, or beyond a
        (
            &finite,
            r#"[["0","1"],"distance-history",[["11",[false,true]]],null]"#,
            KeyRefusal::Unresolved,
        ),
        (
            &shifted,
            r#"[["0","1"],"distance-history",[["3",[false,true]]],true]"#,
            KeyRefusal::Unresolved,
        ),
        // a finite window has a null tail; a shift a Boolean one
        (
            &finite,
            r#"[["0","1"],"distance-history",[],false]"#,
            KeyRefusal::Unresolved,
        ),
        (
            &shifted,
            r#"[["0","1"],"distance-history",[],null]"#,
            KeyRefusal::Members,
        ),
        // a transform no composition of the operator's update gives: a
        // negation, constant false for once, constant true for historically
        (
            &finite,
            r#"[["0","1"],"distance-history",[["0",[true,false]]],null]"#,
            KeyRefusal::Unresolved,
        ),
        (
            &finite,
            r#"[["0","1"],"distance-history",[["0",[false,false]]],null]"#,
            KeyRefusal::Unresolved,
        ),
        (
            &shifted,
            r#"[["0","1"],"distance-history",[["0",[true,true]]],true]"#,
            KeyRefusal::Unresolved,
        ),
        // two buckets at one distance, unsorted, malformed transforms
        (
            &finite,
            r#"[["0","1"],"distance-history",[["0",[false,true]],["0",[true,true]]],null]"#,
            KeyRefusal::Members,
        ),
        (
            &finite,
            r#"[["0","1"],"distance-history",[["2",[true,true]],["10",[false,true]]],null]"#,
            KeyRefusal::Order,
        ),
        (
            &finite,
            r#"[["0","1"],"distance-history",[["0",[false]]],null]"#,
            KeyRefusal::Members,
        ),
        (
            &finite,
            r#"[["0","1"],"distance-history",[["0",[0,1]]],null]"#,
            KeyRefusal::Members,
        ),
        (
            &finite,
            r#"[["0","1"],"distance-history",[["01",[false,true]]],null]"#,
            KeyRefusal::Decimal("01".into()),
        ),
    ];
    for (states, text, refusal) in cases {
        assert_eq!(
            states.admit(state(text).as_bytes()).map(|_| ()),
            Err(refusal),
            "{text}"
        );
    }

    // Since and triggered compose all three non-negating transforms.
    let since = protocol(
        "always (holds(c.level = 1) implies (holds(c.level = 2) since[1,3] holds(c.level = 3)))",
    );
    let triggered = protocol(
        "always (holds(c.level = 1) implies (holds(c.level = 2) triggered[1,3] holds(c.level = 3)))",
    );
    for operator in [&since, &triggered] {
        for transform in ["[false,true]", "[true,true]", "[false,false]"] {
            let text = format!(r#"[["0","1"],"distance-history",[["0",{transform}]],null]"#);
            assert!(operator.admit(state(&text).as_bytes()).is_ok(), "{text}");
        }
        assert_eq!(
            operator
                .admit(
                    state(r#"[["0","1"],"distance-history",[["0",[true,false]]],null]"#).as_bytes()
                )
                .map(|_| ()),
            Err(KeyRefusal::Unresolved)
        );
    }

    // The flag and age forms are the subject's own on both.
    let age = protocol("always (holds(c.level = 1) implies once[0,2] holds(c.level = 2))");
    assert!(age
        .admit(state(r#"[["0","1"],"last-true","3"]"#).as_bytes())
        .is_ok());
    let flag = protocol("always (holds(c.level = 1) implies once holds(c.level = 2))");
    assert!(flag
        .admit(state(r#"[["0","1"],"once",true]"#).as_bytes())
        .is_ok());
}

/// FR-338: on a protocol subject a finite `F`/`U` (and a signed dual, and a
/// debt before its lower shift) has a least-fixed-point acceptance set: a
/// carry excludes membership, not carrying includes it. An ordinary subject
/// has none for a finite deadline, and `F[a,open]` has none on either until
/// the follow-up implements #1516's ruling.
#[trace("TC-525")]
#[test]
fn a_protocol_finite_eventuality_has_an_acceptance_set() {
    let member = at(&[0, 1], Polarity::Positive);
    let carried = |window: Window| Configuration {
        obligations: vec![root()],
        windows: vec![window],
        ..Configuration::default()
    };
    let free = Configuration {
        acceptance: vec![member.clone()],
        ..Configuration::default()
    };
    let deadline = Window::Deadline(member.clone(), 1);
    let finite = protocol(FUTURE);
    assert!(finite.key(&[carried(deadline.clone())]).is_ok());
    assert_eq!(
        octets(&finite, std::slice::from_ref(&free)),
        states_key(r#"["safety",[[[],[],[],[[["0","1"],"positive"]]]]]"#)
    );
    assert_eq!(
        finite
            .admit(octets(&finite, std::slice::from_ref(&free)).as_bytes())
            .map(|(_, branches)| branches),
        Ok(vec![free.clone()])
    );
    // Carried and a member, or neither.
    let both = Configuration {
        acceptance: vec![member.clone()],
        ..carried(deadline.clone())
    };
    assert_eq!(finite.key(&[both]), Err(KeyRefusal::NonNormal));
    assert_eq!(
        finite.key(&[Configuration::default()]),
        Err(KeyRefusal::NonNormal),
        "not carried, yet not a member"
    );
    // A debt before its lower shift is carried as an offsets window; one
    // carried as a remaining interval obligation is too.
    let shifted = protocol(SHIFTED);
    let offsets = Window::Offsets(member.clone(), vec![(0, 2), (1, 3)]);
    assert!(shifted.key(&[carried(offsets.clone())]).is_ok());
    assert_eq!(
        shifted.key(&[Configuration {
            acceptance: vec![member.clone()],
            ..carried(offsets)
        }]),
        Err(KeyRefusal::NonNormal)
    );
    let interval = Obligation::Interval {
        occurrence: member.clone(),
        lower: 0,
        upper: Some(2),
    };
    let pending = |acceptance: Vec<Occurrence>| Configuration {
        obligations: vec![root(), interval.clone()],
        acceptance,
        ..Configuration::default()
    };
    assert!(shifted.key(&[pending(Vec::new())]).is_ok());
    assert_eq!(
        shifted.key(&[pending(vec![member.clone()])]),
        Err(KeyRefusal::NonNormal)
    );

    // An ordinary subject keeps its finite deadlines out of acceptance.
    let ordinary = states(FUTURE);
    assert_eq!(ordinary.key(&[free]), Err(KeyRefusal::Unresolved));
    assert!(ordinary.key(&[carried(deadline)]).is_ok());

    // A signed dual in the negated formula of a TP-4 clause.
    let negated = "eventually always[0,3] holds(c.level = 1)";
    let dual = at(&[0], Polarity::Negative);
    let on = |subject: SubjectKind| {
        let fixture = fixture(INFINITE, "c: Counter", "on origin", negated);
        let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
        BuchiStates::clause(&fixture.clause, &context, subject).expect("translates")
    };
    let waiting = |acceptance: Vec<Occurrence>| {
        BuchiState::Elementary(Configuration {
            acceptance,
            ..Configuration::default()
        })
    };
    let protocol_states = on(SubjectKind::Protocol);
    assert!(protocol_states.key(&waiting(vec![dual.clone()])).is_ok());
    assert_eq!(
        protocol_states.key(&waiting(Vec::new())),
        Err(KeyRefusal::NonNormal)
    );
    assert_eq!(
        on(SubjectKind::Ordinary).key(&waiting(vec![dual])),
        Err(KeyRefusal::Unresolved)
    );
}

/// #1516 R-2: an ordinary `F`/`U[a,open]` is a member of its Büchi acceptance
/// set exactly when it is neither carried as a current obligation nor pending
/// as a `shift` window; the shift completes into the unbounded obligation.
#[trace("TC-525")]
#[test]
fn an_open_upper_eventuality_is_accepted_once_its_shift_completes() {
    // The negated closure is `always eventually[2,*] not p`: the open-upper
    // least fixed point is the signed occurrence `[["0"],"negative"]`.
    let negated = "eventually always[2,*] holds(c.level = 1)";
    let dual = at(&[0], Polarity::Negative);
    let carry =
        |obligations: Vec<Obligation>, windows: Vec<Window>, acceptance: Vec<Occurrence>| {
            BuchiState::Elementary(Configuration {
                obligations,
                windows,
                acceptance,
                ..Configuration::default()
            })
        };
    let root_carry = Obligation::Current(at(&[], Polarity::Negative));
    let unbounded = Obligation::Current(dual.clone());
    let shift = Window::Shift(dual.clone(), 2);
    for subject in [SubjectKind::Ordinary, SubjectKind::Protocol] {
        let fixture = fixture(INFINITE, "c: Counter", "on origin", negated);
        let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
        let states = BuchiStates::clause(&fixture.clause, &context, subject).expect("translates");
        // Pending: carried as the shift, so not a member.
        assert!(states
            .key(&carry(
                vec![root_carry.clone()],
                vec![shift.clone()],
                Vec::new()
            ))
            .is_ok());
        // Completed and carried as the unbounded obligation: not a member.
        assert!(states
            .key(&carry(
                vec![root_carry.clone(), unbounded.clone()],
                Vec::new(),
                Vec::new()
            ))
            .is_ok());
        // Completed and discharged: a member.
        let member = states.key(&carry(
            vec![root_carry.clone()],
            Vec::new(),
            vec![dual.clone()],
        ));
        assert!(member.is_ok());
        // Wrongly carried: a member while pending, a member while carried,
        // and neither carried nor a member.
        assert_eq!(
            states.key(&carry(
                vec![root_carry.clone()],
                vec![shift.clone()],
                vec![dual.clone()]
            )),
            Err(KeyRefusal::NonNormal)
        );
        assert_eq!(
            states.key(&carry(
                vec![root_carry.clone(), unbounded.clone()],
                Vec::new(),
                vec![dual.clone()]
            )),
            Err(KeyRefusal::NonNormal)
        );
        assert_eq!(
            states.key(&carry(vec![root_carry.clone()], Vec::new(), Vec::new())),
            Err(KeyRefusal::NonNormal)
        );
        // One debt stored as both the shift and the unbounded obligation.
        assert_eq!(
            states.key(&carry(
                vec![root_carry.clone(), unbounded.clone()],
                vec![shift.clone()],
                Vec::new()
            )),
            Err(KeyRefusal::Members)
        );
    }
}

/// FR-338's `each-buchi` vectors and its admission negatives
/// (FR-338-AC-5): an unguarded `on each` TP-4 clause is keyed under
/// activation `"each"` as `["each-buchi", phase, branch]`, and a waiting
/// state carrying the well-formed shift window of its `eventually[1,*]` root,
/// a `buchi` state under that context, an `each-buchi` state under an origin
/// or `each-when` context, and a TP-4 key under a timed profile are refused,
/// as is a same-shaped state under a foreign profile or activation.
#[trace("TC-525", "FR-338-AC-5")]
#[test]
fn an_unguarded_on_each_tp4_clause_is_keyed_each_buchi() {
    let each = |body: &str| buchi(body, "on each (q: Counter)");
    let states = each(LIVE);
    let branch = r#"[[[[],"negative"]],[],[],[]]"#;
    let key = |tag: &str, body: &str| buchi_key("each", &format!(r#"["{tag}",{body}]"#));
    // The two vectors.
    assert_eq!(
        text_of(
            &states
                .key(&BuchiState::Each(
                    BuchiPhase::Waiting,
                    Configuration::default()
                ))
                .expect("encodes")
        ),
        buchi_key("each", r#"["each-buchi","waiting",[[],[],[],[]]]"#)
    );
    let carry = Configuration {
        obligations: vec![Obligation::Current(at(&[], Polarity::Negative))],
        ..Configuration::default()
    };
    let active = text_of(
        &states
            .key(&BuchiState::Each(BuchiPhase::Active, carry.clone()))
            .expect("encodes"),
    );
    assert_eq!(
        active,
        buchi_key("each", &format!(r#"["each-buchi","active",{branch}]"#))
    );
    // It differs from the phase-less origin `buchi` key (under its own
    // context), and is admitted back as the state it was written from.
    let origin_key = text_of(
        &buchi(LIVE, "on origin")
            .key(&BuchiState::Elementary(carry.clone()))
            .expect("encodes"),
    );
    assert_ne!(active, origin_key);
    assert_eq!(
        states.admit(active.as_bytes()).map(|(_, state)| state),
        Ok(BuchiState::Each(BuchiPhase::Active, carry))
    );

    // The derived `["activation"]` set is never a supplied member.
    assert_eq!(
        states
            .admit(
                key(
                    "each-buchi",
                    r#""active",[[[[],"negative"]],[],[],[["activation"]]]"#
                )
                .as_bytes()
            )
            .map(|_| ()),
        Err(KeyRefusal::Members)
    );

    // The waiting state carries no window, though the shift window of the
    // `eventually[1,*]` root is well formed in the active phase.
    let shifting = each("eventually[1,*] holds(c.level = 1)");
    let window = r#"[[],[],[[[],"negative","shift","1"]],[]]"#;
    assert!(shifting
        .admit(key("each-buchi", &format!(r#""active",{window}"#)).as_bytes())
        .is_ok());
    assert_eq!(
        shifting
            .admit(key("each-buchi", &format!(r#""waiting",{window}"#)).as_bytes())
            .map(|_| ()),
        Err(KeyRefusal::Members)
    );
    let obligation = r#"[[[[],"negative"]],[],[],[]]"#;
    for (octets, refusal) in [
        // a `buchi` or `guarded-buchi` state under the unguarded context
        (
            key("buchi", obligation),
            KeyRefusal::WrongForm("buchi".into()),
        ),
        (
            key("guarded-buchi", &format!(r#""active",{obligation}"#)),
            KeyRefusal::WrongForm("guarded-buchi".into()),
        ),
        // a waiting branch with an obligation; an unknown phase
        (
            key("each-buchi", &format!(r#""waiting",{obligation}"#)),
            KeyRefusal::Members,
        ),
        (
            key("each-buchi", &format!(r#""selected",{obligation}"#)),
            KeyRefusal::Members,
        ),
        // the phase-less shape
        (key("each-buchi", obligation), KeyRefusal::Members),
    ] {
        assert_eq!(
            states.admit(octets.as_bytes()).map(|_| ()),
            Err(refusal),
            "{octets}"
        );
    }

    // An `each-buchi` state under the origin or the `each-when` context is
    // not that context's form, and a same-shaped state under a foreign
    // activation or profile is not this context's.
    let waiting = r#"["each-buchi","waiting",[[],[],[],[]]]"#;
    let origin = buchi(LIVE, "on origin");
    let guarded = buchi(LIVE, GUARD);
    assert_eq!(
        origin
            .admit(buchi_key("infinite", waiting).as_bytes())
            .map(|_| ()),
        Err(KeyRefusal::WrongForm("each-buchi".into()))
    );
    assert_eq!(
        guarded
            .admit(buchi_key("each-when", waiting).as_bytes())
            .map(|_| ()),
        Err(KeyRefusal::WrongForm("each-buchi".into()))
    );
    let written = buchi_key("each", waiting);
    assert!(states.admit(written.as_bytes()).is_ok());
    for foreign in [
        written.replace(r#""each""#, r#""origin""#),
        written.replace(r#""each""#, r#""each-when""#),
        written.replace(r#""each""#, r#""infinite""#),
        written.replace(
            "quire.temporal.infinite-trace/v1",
            "quire.temporal.timed/v1",
        ),
    ] {
        assert_eq!(
            states.admit(foreign.as_bytes()).map(|_| ()),
            Err(KeyRefusal::ForeignContext),
            "{foreign}"
        );
    }
    // A timed-profile TP-4 clause has no key at all, and no "each" context.
    let timed_each =
        crate::timed_trace::timed_on("on each (q: M::Counter)", "eventually holds(c.level = 1)");
    assert_eq!(
        StateContext::clause(&timed_each.clause, occurrence(0), Vec::new()).activation,
        ContextActivation::Infinite
    );
    let timed = crate::timed_trace::timed("eventually holds(c.level = 1)");
    let context = StateContext::clause(&timed.clause, occurrence(0), Vec::new());
    assert!(matches!(
        BuchiStates::clause(&timed.clause, &context, SubjectKind::Ordinary),
        Err(TranslationRefusal::NotTranslated(_))
    ));
}
