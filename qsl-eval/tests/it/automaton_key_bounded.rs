// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338's canonical translation state for a bounded (TP-2) clause: the
//! exact key octets of `eventually[0,u] holds(A)` and `always[0,u] holds(A)`
//! on origin, the counted-distance rules a step reads them by, and the
//! admission of imported keys against the clause's context.
//!
//! The expected octets are written out here as ASCII text, independent of
//! the encoder under test. Not covered: the `on each`, lower-bound, past,
//! nested-interval and Büchi alternatives, which no EN-1 phase certifies
//! yet (Remaining work: QSL-878).

use ix_trace_rs::trace;
use qsl_eval::automaton::{
    bounded_shape, BoundedKind, BoundedState, BoundedTranslation, KeyRefusal, StateContext,
    TranslationRefusal,
};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use quire_exact::{Origin, Role};

use crate::temporal_trace::{fixture, EVENT, INFINITE};

fn occurrence(ordinal: u64) -> OccurrenceKey {
    OccurrenceKey::new(
        WireNodeId::from_digest([0xab; 32]),
        Origin::new(Role::new("temporal"), ordinal),
    )
}

/// The bounded clause `body` over `c: Counter` under event-position
/// false-extension at `occurrence(ordinal)`, activated as `activation`.
fn clause_in(body: &str, activation: &str, ordinal: u64) -> BoundedTranslation {
    let fixture = fixture(EVENT, "c: Counter", activation, body);
    let context = StateContext::clause(&fixture.clause, occurrence(ordinal), Vec::new());
    BoundedTranslation::clause(&fixture.clause, &context)
        .unwrap_or_else(|why| panic!("{body}: {why}"))
}

fn clause(body: &str) -> BoundedTranslation {
    clause_in(body, "on origin", 0)
}

fn text(translation: &BoundedTranslation, state: BoundedState) -> String {
    let key = translation.key(state).expect("the key encodes");
    String::from_utf8(key.as_bytes().to_vec()).expect("a key is UTF-8")
}

/// The context of `clause(_)` as FR-338 spells it.
fn context() -> String {
    format!(
        r#"[[{{"digest":"{}","domain":"quire.checked-semantic-node/v1"}},"temporal","0"],"quire.temporal.event-position.false-extension/v1","origin",[]]"#,
        "ab".repeat(32)
    )
}

fn key(body: &str) -> String {
    format!(r#"["quire.automaton-state-key/v1",{},{body}]"#, context())
}

/// FR-338's `eventually[0,1] holds(p)` vector: on origin with p false at
/// position zero the state is `["bounded",["origin","1"],[[[],[],[[[],
/// "positive","deadline","1"]],[]]]]`; p true at counted distance one gives
/// `["accept"]`, and p false at the final region's close gives `["reject"]`.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn eventually_one_on_origin_walks_to_accept_and_to_reject() {
    let translation = clause("eventually[0,1] holds(true)");
    assert_eq!(translation.kind(), BoundedKind::Eventually);
    assert_eq!(translation.horizon(), 1);
    let first = translation.start(false);
    assert_eq!(first, BoundedState::Open { remaining: 1 });
    assert_eq!(
        text(&translation, first),
        key(r#"["bounded",["origin","1"],[[[],[],[[[],"positive","deadline","1"]],[]]]]"#)
    );
    let accepted = translation.step(first, true, true);
    assert_eq!(accepted, BoundedState::Accept);
    assert_eq!(text(&translation, accepted), key(r#"["accept"]"#));
    // False at distance one leaves the region open until the next counted
    // step crosses it; the crossing reads no letter.
    let last = translation.step(first, true, false);
    assert_eq!(last, BoundedState::Open { remaining: 0 });
    assert_eq!(
        text(&translation, last),
        key(r#"["bounded",["origin","0"],[[[],[],[[[],"positive","deadline","0"]],[]]]]"#)
    );
    let rejected = translation.step(last, true, true);
    assert_eq!(rejected, BoundedState::Reject);
    assert_eq!(text(&translation, rejected), key(r#"["reject"]"#));
    assert_eq!(translation.step(translation.start(true), true, false), BoundedState::Accept);
    assert_eq!(translation.step(rejected, true, true), BoundedState::Reject);
}

/// QSpec FR-091 on a protocol subject (ADR-027 PB-3): a step that does not
/// count leaves the debt where it was and its letter is still read, so a
/// false fork before the counted step does not reject, a true letter after
/// an uncounted one accepts, and the crossing counted step rejects.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn a_step_that_does_not_count_reads_its_letter_without_reducing_the_debt() {
    let translation = clause("eventually[0,1] holds(true)");
    let origin = translation.start(false);
    let after_fork = translation.step(origin, false, false);
    assert_eq!(after_fork, origin);
    assert_eq!(translation.step(after_fork, true, true), BoundedState::Accept);
    // The counted step that follows a false counted one closes the region.
    let counted = translation.step(origin, true, false);
    assert_eq!(translation.step(counted, true, true), BoundedState::Reject);
    // At distance one a false letter followed by an uncounted true one
    // accepts: the zero-distance carry keeps the region open.
    assert_eq!(translation.step(counted, false, true), BoundedState::Accept);
    assert_eq!(translation.step(counted, false, false), counted);
}

/// `always[0,u] holds(A)`: A must hold at every position of the window, the
/// extent is the furthest required distance, and the window's closed upper
/// boundary discharges it on the counted step that crosses it.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn always_carries_its_extent_and_discharges_at_the_closed_boundary() {
    let translation = clause("always[0,1] holds(true)");
    assert_eq!(translation.kind(), BoundedKind::Always);
    assert_eq!(translation.start(false), BoundedState::Reject);
    let first = translation.start(true);
    assert_eq!(
        text(&translation, first),
        key(r#"["bounded",["origin","1"],[[[],[],[[[],"positive","extent","1"]],[]]]]"#)
    );
    let last = translation.step(first, true, true);
    assert_eq!(last, BoundedState::Open { remaining: 0 });
    assert_eq!(translation.step(first, true, false), BoundedState::Reject);
    // The region at the upper boundary still reads zero-distance letters.
    assert_eq!(translation.step(last, false, false), BoundedState::Reject);
    assert_eq!(translation.step(last, false, true), last);
    // The crossing step reads no letter and discharges.
    assert_eq!(translation.step(last, true, false), BoundedState::Accept);
}

/// QSpec FR-091 and FR-338: a terminal state's false extension supplies the
/// counted false letters through the remaining horizon and closes the last
/// region, so an open `eventually` debt rejects and an open `always`
/// obligation rejects unless its window already ended at the terminal
/// state.
#[trace("TC-525", "FR-338-AC-6")]
#[test]
fn the_terminal_false_extension_closes_the_open_region() {
    let eventually = clause("eventually[0,2] holds(true)");
    for remaining in 0..=2 {
        assert_eq!(
            eventually.terminal(BoundedState::Open { remaining }),
            BoundedState::Reject
        );
    }
    assert_eq!(eventually.terminal(BoundedState::Accept), BoundedState::Accept);
    let always = clause("always[0,2] holds(true)");
    assert_eq!(always.terminal(BoundedState::Open { remaining: 0 }), BoundedState::Accept);
    assert_eq!(always.terminal(BoundedState::Open { remaining: 1 }), BoundedState::Reject);
    assert_eq!(always.terminal(BoundedState::Open { remaining: 2 }), BoundedState::Reject);
}

/// u64::MAX bounds stay exact: the horizon and the remaining distance are
/// decimal strings of the checked u64 domain, with no saturation.
#[trace("TC-525", "FR-338-AC-7")]
#[test]
fn a_u64_max_horizon_stays_exact() {
    let translation = clause("eventually[0,18446744073709551615] holds(true)");
    let first = translation.start(false);
    assert_eq!(
        first,
        BoundedState::Open {
            remaining: u64::MAX
        }
    );
    assert!(text(&translation, first).contains(r#"["origin","18446744073709551615"]"#));
    assert_eq!(
        translation.admit(text(&translation, first).as_bytes()),
        Ok((translation.key(first).expect("encodes"), first))
    );
    assert_eq!(
        translation.step(first, true, false),
        BoundedState::Open {
            remaining: u64::MAX - 1
        }
    );
}

/// Every key a translation encodes is admitted back as the same state; a
/// key of another occurrence, profile or activation is `ForeignContext`,
/// and a `bounded` state with a mismatched clock and window, an `each`
/// clock, a window of the other kind, a remaining distance past the
/// horizon, a leading zero or an extra member is refused before it is used.
#[trace("TC-525", "FR-338-AC-5")]
#[test]
fn a_bounded_key_is_admitted_against_its_own_context_only() {
    let translation = clause("eventually[0,2] holds(true)");
    for state in [
        BoundedState::Open { remaining: 0 },
        BoundedState::Open { remaining: 2 },
        BoundedState::Accept,
        BoundedState::Reject,
    ] {
        let key = translation.key(state).expect("encodes");
        assert_eq!(translation.admit(key.as_bytes()), Ok((key.clone(), state)));
    }
    let other = clause_in("eventually[0,2] holds(true)", "on origin", 1);
    let foreign = other.key(BoundedState::Accept).expect("encodes");
    assert_eq!(
        translation.admit(foreign.as_bytes()),
        Err(KeyRefusal::ForeignContext)
    );
    let each = text(&translation, BoundedState::Accept).replace(r#""origin""#, r#""each""#);
    assert_eq!(
        translation.admit(each.as_bytes()),
        Err(KeyRefusal::ForeignContext)
    );
    let infinite = text(&translation, BoundedState::Accept).replace(
        "quire.temporal.event-position.false-extension/v1",
        INFINITE,
    );
    assert_eq!(
        translation.admit(infinite.as_bytes()),
        Err(KeyRefusal::ForeignContext)
    );

    let open = |clock: &str, window: &str| {
        key(&format!(
            r#"["bounded",["origin",{clock}],[[[],[],[[[],"positive",{window}]],[]]]]"#
        ))
    };
    let refused = [
        (open(r#""1""#, r#""deadline","2""#), KeyRefusal::Unreachable),
        (open(r#""3""#, r#""deadline","3""#), KeyRefusal::Unreachable),
        (open(r#""1""#, r#""extent","1""#), KeyRefusal::Unresolved),
        (open(r#""01""#, r#""deadline","01""#), KeyRefusal::Decimal("01".into())),
        (
            open(r#""18446744073709551616""#, r#""deadline","18446744073709551616""#),
            KeyRefusal::Decimal("18446744073709551616".into()),
        ),
        (open("1", r#""deadline","1""#), KeyRefusal::Members),
        (key(r#"["bounded",["origin","1"],[]]"#), KeyRefusal::Unreachable),
        (
            key(r#"["bounded",["origin","1"],[[[],[],[],[]]]]"#),
            KeyRefusal::Unresolved,
        ),
        (
            key(r#"["bounded",["origin","1"],[[[[[],"positive"]],[],[[[],"positive","deadline","1"]],[]]]]"#),
            KeyRefusal::Unresolved,
        ),
        (
            key(r#"["bounded",["origin","1"],[[[],[],[[[],"negative","deadline","1"]],[]]]]"#),
            KeyRefusal::Unresolved,
        ),
        (
            key(r#"["bounded",["origin","1"],[[[],[],[[["0"],"positive","deadline","1"]],[]]]]"#),
            KeyRefusal::Unresolved,
        ),
        (key(r#"["accept","extra"]"#), KeyRefusal::Members),
        (key(r#"["safety",[]]"#), KeyRefusal::WrongForm("safety".into())),
        (key(r#"["pending"]"#), KeyRefusal::UnknownTag("pending".into())),
    ];
    for (octets, why) in refused {
        assert_eq!(translation.admit(octets.as_bytes()), Err(why), "{octets}");
    }
    let spaced = text(&translation, BoundedState::Accept).replacen("[", "[ ", 1);
    assert_eq!(
        translation.admit(spaced.as_bytes()),
        Err(KeyRefusal::NotCanonical)
    );
}

/// Only `eventually[0,u]` and `always[0,u]` over one `holds(A)`, on origin
/// under a step-measured bounded profile, have a bounded translation.
#[trace("TC-525", "FR-338-AC-7")]
#[test]
fn another_shape_has_no_bounded_translation() {
    for (profile, activation, body) in [
        (EVENT, "on origin", "eventually[1,2] holds(true)"),
        (EVENT, "on origin", "eventually[0,2] (holds(true) and holds(false))"),
        (EVENT, "on origin", "once[0,2] holds(true)"),
        (EVENT, "on origin", "holds(true) until[0,2] holds(true)"),
        (EVENT, "on each (q: Counter)", "eventually[0,2] holds(true)"),
        (INFINITE, "on origin", "eventually[0,2] holds(true)"),
    ] {
        let fixture = fixture(profile, "c: Counter", activation, body);
        assert!(bounded_shape(&fixture.clause).is_none(), "{body}");
        let context = StateContext::clause(&fixture.clause, occurrence(0), Vec::new());
        assert!(
            matches!(
                BoundedTranslation::clause(&fixture.clause, &context),
                Err(TranslationRefusal::NotTranslated(_))
            ),
            "{body}"
        );
    }
}
