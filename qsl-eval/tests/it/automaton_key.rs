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
    HistoryOperand, KeyRefusal, Obligation, Occurrence, PastMemory, PastOperator, Polarity,
    SafetyState, SafetyStates, SafetyTranslation, StateContext, SubjectKind, TranslationRefusal,
    Window,
};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use quire_exact::{Origin, Role};

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
        WireNodeId::from_digest([0xab; 32]),
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
    for tag in ["bounded", "accept", "reject", "buchi", "guarded-buchi"] {
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

/// A protocol subject's past and acceptance forms differ from the ordinary
/// ones (`distance-history`, finite `F`/`U` acceptance), and this algebra
/// does not carry them: it refuses to translate such a subject rather than
/// admit the ordinary forms as if they were its own.
#[trace("TC-525")]
#[test]
fn a_protocol_subject_has_no_safety_states_yet() {
    let fixture = fixture(INFINITE, "c: Counter", "on origin", FUTURE);
    let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
    assert!(matches!(
        SafetyStates::clause(&fixture.clause, &context, SubjectKind::Protocol),
        Err(TranslationRefusal::NotTranslated(_))
    ));
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
    let empty = "[[],[],[],[]]";
    let in_context = |activation: &str, body: String| buchi_key(activation, &body);
    let cases = [
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
    let live = fixture(INFINITE, "c: Counter", "on origin", LIVE);
    let context = StateContext::clause(&live.clause, occurrence(0), Vec::new());
    assert!(matches!(
        BuchiStates::clause(&live.clause, &context, SubjectKind::Protocol),
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
/// carry, both `guarded-buchi` rows and the deadlock item's complete key
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
    // The positive F carry is a different state: it is not a `buchi` body.
    let positive_carry = states_key(r#"["safety",[[[[[],"positive"]],[],[],[]]]]"#);
    assert_ne!(
        positive_carry,
        states_key(r#"["buchi",[[[[],"negative"]],[],[],[]]]"#)
    );
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

/// A TP-4 clause is not translated when its key form is not derived: an
/// unguarded `on each` (ruling R-5 pending), an open-upper least fixed point
/// in the negated closure (R-2 pending), a clause checked under another
/// profile, and any clause the safety key cannot carry.
#[trace("TC-525")]
#[test]
fn tp4_clauses_without_a_derived_key_form_are_not_translated() {
    let refused = |profile: &str, activation: &str, body: &str| {
        let fixture = fixture(profile, "c: Counter", activation, body);
        let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
        BuchiStates::clause(&fixture.clause, &context, SubjectKind::Ordinary).map(|_| ())
    };
    for (activation, body) in [
        ("on each (q: Counter)", "eventually[2,*] holds(c.level = 1)"),
        ("on origin", "eventually always[2,*] holds(c.level = 1)"),
        // not unbounded, so a safety clause
        ("on origin", "eventually[0,3] holds(c.level = 1)"),
    ] {
        assert!(
            matches!(
                refused(INFINITE, activation, body),
                Err(TranslationRefusal::NotTranslated(_))
            ),
            "{activation}: {body}"
        );
    }
    // The same shapes with a closed upper bound are keyed.
    for body in [
        "eventually always[0,3] holds(c.level = 1)",
        "eventually always holds(c.level = 1)",
        // an open-lower, open-upper eventuality is liveness
        "eventually[2,*] holds(c.level = 1)",
    ] {
        assert!(refused(INFINITE, "on origin", body).is_ok(), "{body}");
    }
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
