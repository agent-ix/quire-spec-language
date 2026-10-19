// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338's canonical translation state for the safety items EN-1 decides:
//! the exact key octets of the deadlock-freedom item and of TP-1 clauses,
//! and the admission of imported keys against an item's context.
//!
//! The expected octets are written out here as ASCII text, independent of
//! the encoder under test. Not covered: the bounded, interval, past and
//! Büchi alternatives of FR-338's algebra, whose clauses no EN-1 phase
//! decides yet, so their vectors and admission negatives are not checked.

use ix_trace_rs::trace;
use qsl_eval::automaton::{
    ContextActivation, KeyRefusal, SafetyState, SafetyTranslation, StateContext, TranslationRefusal,
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
