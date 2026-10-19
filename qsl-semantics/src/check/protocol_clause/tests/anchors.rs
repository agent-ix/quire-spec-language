//! FR-113 anchor resolution: nested scopes, missing and ambiguous names,
//! wrong-kind and channel-mismatch refusals, and refusal ordering.

use ix_trace_rs::trace;
use qsl_forms::{AnchorSite, ProtocolNodeKind};

use super::{
    anchors_at, declaration_id, handoff_protocol, protocol_causes, recovery_flow, refusals,
    resolve_protocol,
};
use crate::check::refusal::ProtocolAnchorCause;
use crate::check::CheckRefusal;

/// FR-113-AC-1 (TC-511): every anchor of the `RecoveryFlow` fixture
/// resolves to the exact node FR-113-AC-1 names, by identity.
#[trace("TC-511", "FR-113-AC-1")]
#[test]
fn every_anchor_of_the_recovery_flow_fixture_resolves_to_its_named_node() {
    let (_, protocol, result) = resolve_protocol(&recovery_flow("Main::Committed", ""));
    let (checked, _) = result.unwrap_or_else(|refusals| panic!("{refusals:?}"));
    assert_eq!(
        checked.resolved_anchors.len(),
        protocol.scoped_anchors.len()
    );
    // Source order: compensate-for, compensate-commit, effect-of,
    // event-for (FR-112-AC-1).
    assert_eq!(
        checked.resolved_anchors[0],
        declaration_id(&protocol, "Applied", ProtocolNodeKind::Effect, &["Main"])
    );
    assert_eq!(
        checked.resolved_anchors[1],
        declaration_id(&protocol, "Committed", ProtocolNodeKind::Commit, &["Main"])
    );
    assert_eq!(
        checked.resolved_anchors[2],
        declaration_id(&protocol, "Tried", ProtocolNodeKind::Attempt, &["Main"])
    );
    assert_eq!(
        checked.resolved_anchors[3],
        declaration_id(&protocol, "Undo", ProtocolNodeKind::CompensateTemplate, &[])
    );
}

/// FR-113-AC-2 (TC-511): a nested `sequence Inner` inside `Main`, after
/// the outer `effect Applied`, declares its own `Applied`. An `await`
/// inside `Inner` naming `Applied` resolves to the *inner* node; one
/// naming `Main::Applied` resolves to the outer node from inside
/// `Inner`; and one in `Main` (outside `Inner`) naming `Applied`
/// resolves to the outer node -- asserted against the two nodes'
/// distinct identities, not just "no refusal" (a mutation to
/// outermost-first resolution passes every other test in this module
/// but fails this one).
#[trace("TC-511", "FR-113-AC-2")]
#[test]
fn a_nested_scope_resolves_its_own_name_over_an_outer_one_of_the_same_spelling() {
    // Every record binder here is uniquely named (`applied_inner`,
    // `seen_inner`/`seen_qualified`/`seen_outer`), distinct from the
    // outer fixture's own `applied`: FR-113's binder no-shadowing rule
    // is now protocol-wide (SR-765 FND-001), and this test's
    // own assertions are about anchor *resolution* through nested and
    // sibling named-control scopes, not about binder shadowing --
    // reusing a binder name here would make the fixture itself refuse.
    let inner = "sequence Inner {\n\
         effect Applied of Tried as (applied_inner: Boolean) { true };\n\
         await WaitInner after Applied using v clock \"ticks\" within [0,1] \
         match event Seen by R for Undo as (seen_inner: Boolean) { true }; \
         then sequence Then { } timeout sequence Timeout { }\n\
         }\n";
    // The two awaits in `Main` follow the outer `Applied` they wait after: an
    // await anchored at a later control closes a causal cycle (FR-218).
    let outer = "await WaitQualified after Main::Applied using v clock \"ticks\" within [0,1] \
         match event SeenQ by R for Undo as (seen_qualified: Boolean) { true }; \
         then sequence ThenQ { } timeout sequence TimeoutQ { }\n\
         await WaitOuter after Applied using v clock \"ticks\" within [0,1] \
         match event SeenO by R for Undo as (seen_outer: Boolean) { true }; \
         then sequence ThenO { } timeout sequence TimeoutO { }\n";
    let applied = "effect Applied of Tried as (applied: Boolean) { true };\n";
    let source =
        recovery_flow("Main::Committed", inner).replacen(applied, &format!("{applied}{outer}"), 1);
    let (_, protocol, result) = resolve_protocol(&source);
    let (checked, _) = result.unwrap_or_else(|refusals| panic!("{refusals:?}"));

    let outer_applied = declaration_id(&protocol, "Applied", ProtocolNodeKind::Effect, &["Main"]);
    let inner_applied = declaration_id(
        &protocol,
        "Applied",
        ProtocolNodeKind::Effect,
        &["Main", "Inner"],
    );
    assert_ne!(outer_applied, inner_applied);

    // Built in this order: `WaitInner` (inside `Inner`), `WaitQualified`
    // and `WaitOuter` (both directly in `Main`).
    let await_afters = anchors_at(&protocol, AnchorSite::AwaitAfter);
    assert_eq!(await_afters.len(), 3, "{await_afters:?}");
    let [wait_inner, wait_qualified, wait_outer] = await_afters[..] else {
        unreachable!()
    };
    assert_eq!(
        checked.resolved_anchors[wait_inner], inner_applied,
        "an unqualified reference inside Inner must resolve to Inner's own Applied"
    );
    assert_eq!(
        checked.resolved_anchors[wait_qualified], outer_applied,
        "Main::Applied must reach the outer node from inside Inner"
    );
    assert_eq!(
        checked.resolved_anchors[wait_outer], outer_applied,
        "an unqualified reference in Main (outside Inner) must resolve to the outer node"
    );
}

/// FR-113-AC-3 (TC-511): a missing anchor or member refuses
/// `missing_declaration`/`missing-name`, naming the failing segment,
/// the whole anchor's segments and the anchor's own (empty, at the top
/// level) lexical scope.
#[trace("TC-511", "FR-113-AC-3")]
#[test]
fn a_missing_anchor_or_member_refuses_naming_the_failing_segment() {
    let cases = [
        ("Main::Missing", vec!["Main", "Missing"], "Missing"),
        ("Other::Applied", vec!["Other", "Applied"], "Other"),
    ];
    for (target, segments, segment) in cases {
        let source = recovery_flow("Main::Committed", "").replacen("Main::Applied", target, 1);
        let (_, refusals) = refusals(&source);
        assert_eq!(refusals.len(), 1, "{target}: {refusals:?}");
        let [cause] = protocol_causes(&refusals)[..] else {
            panic!("one cause");
        };
        match cause {
            ProtocolAnchorCause::Missing {
                segment: found,
                segments: found_segments,
                scope,
                ..
            } => {
                assert_eq!(found, segment, "{target}");
                assert_eq!(found_segments, &segments, "{target}");
                assert!(scope.is_empty(), "{target}: {scope:?}");
            }
            other => panic!("{target}: a Missing cause, got {other:?}"),
        }
    }

    let (_, refusals) = refusals(&recovery_flow("Main::Committed", "").replacen(
        "effect Applied of Tried",
        "effect Applied of Absent",
        1,
    ));
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    let [cause] = protocol_causes(&refusals)[..] else {
        panic!("one cause");
    };
    match cause {
        ProtocolAnchorCause::Missing {
            segment,
            segments,
            scope,
            ..
        } => {
            assert_eq!(segment, "Absent");
            assert_eq!(segments, &["Absent".to_owned()]);
            assert_eq!(scope, &["Main".to_owned()]);
        }
        other => panic!("a Missing cause, got {other:?}"),
    }
}

/// The two `effect Applied` declarations' own name spans, in the order
/// they appear in `text` (not the earlier `compensate ... for
/// Main::Applied` reference).
fn duplicate_applied_spans(text: &str) -> [usize; 2] {
    let mut effect_applied = text.match_indices("effect Applied");
    let first = effect_applied.next().unwrap().0 + "effect ".len();
    let second = effect_applied.next().unwrap().0 + "effect ".len();
    [first, second]
}

/// Every `Ambiguous` cause of `refusals` names `Applied` and lists its
/// two loci in exactly `expected` order (FR-113-AC-4: "naming both in
/// source order").
fn assert_ambiguous_loci_order(refusals: &[CheckRefusal], expected: [usize; 2]) {
    for cause in protocol_causes(refusals) {
        match cause {
            ProtocolAnchorCause::Ambiguous { name, loci, .. } => {
                assert_eq!(name, "Applied");
                let starts: Vec<usize> = loci.iter().map(|span| span.start).collect();
                assert_eq!(starts, expected, "{cause:?}");
            }
            other => panic!("an Ambiguous cause, got {other:?}"),
        }
    }
}

/// FR-113-AC-4 (TC-512): two event nodes named `Applied` in `Main`
/// refuse `ambiguous_declaration`/`ambiguous-name` at both
/// declarations, and `Main::Applied` refuses at the anchor, naming
/// both in source order. Swapping the two declarations swaps nothing
/// but their order in the refusal.
#[trace("TC-512", "FR-113-AC-4")]
#[test]
fn two_declarations_of_one_name_refuse_at_both_and_at_a_qualified_anchor() {
    let extra = "effect Applied of Tried as (again: Boolean) { true };\n";
    let source = recovery_flow("Main::Committed", extra);
    let (text, found) = refusals(&source);
    // Two duplicate-declaration refusals (one per `Applied`) plus one
    // at the `compensate ... for Main::Applied` anchor, which decides
    // on the ambiguous name.
    assert_eq!(found.len(), 3, "{found:?}");
    for refusal in &found {
        assert_eq!(refusal.cause.code().as_str(), "ambiguous_declaration");
    }
    // One refusal at each `effect Applied` declaration's own name, and
    // one at the `compensate ... for Main::Applied` anchor's whole
    // reference -- in source position order, the anchor first, since it
    // is written before `run sequence Main`.
    let anchor_span = text.find("Main::Applied").unwrap();
    let [first_decl, second_decl] = duplicate_applied_spans(&text);
    let mut expected = [anchor_span, first_decl, second_decl];
    expected.sort_unstable();
    let starts: Vec<usize> = protocol_causes(&found)
        .iter()
        .map(|cause| cause.span().start)
        .collect();
    assert_eq!(starts, expected);
    assert_ambiguous_loci_order(&found, [first_decl, second_decl]);

    // Swap the two declarations' physical order (the `extra` copy,
    // marked `again`, now comes after the fixture's own `effect
    // Applied`, marked `applied`, instead of before it): the same
    // three refusals occur, and the loci list simply swaps to match
    // the new source order.
    assert!(
        text.find("again").unwrap() < text.find("applied").unwrap(),
        "fixture sanity: `again` (extra) must be declared before `applied` (outer)"
    );
    let without_extra = recovery_flow("Main::Committed", "");
    let outer_line = "effect Applied of Tried as (applied: Boolean) { true };\n";
    assert!(without_extra.contains(outer_line));
    let swapped = without_extra.replacen(outer_line, &format!("{outer_line}{extra}"), 1);
    assert_ne!(source, swapped);
    let (swapped_text, swapped_refusals) = refusals(&swapped);
    assert_eq!(swapped_refusals.len(), 3, "{swapped_refusals:?}");
    assert!(
        swapped_text.find("applied").unwrap() < swapped_text.find("again").unwrap(),
        "swap sanity: `applied` (outer) must now be declared before `again` (extra)"
    );
    let [swapped_first, swapped_second] = duplicate_applied_spans(&swapped_text);
    assert_ambiguous_loci_order(&swapped_refusals, [swapped_first, swapped_second]);
}

/// A regression fixture distinct from FR-113-AC-6's own (a duplicate
/// declaration, not a shadowing binder, paired with a missing anchor):
/// a protocol with a duplicate declaration (physically inside `run`,
/// so later in source) and an unrelated missing anchor (physically
/// before `run`) reports both, ordered strictly by source position --
/// not by the order the checker happens to build them in, which is the
/// opposite: the up-front duplicate-declaration pass runs before the
/// per-anchor pass, so without the sort this test's own refusals would
/// come back in exactly the wrong order. Checking it twice gives the
/// same refusals in the same order.
/// [`a_missing_anchor_and_a_later_shadowing_binder_report_in_source_order`]
/// carries AC-6's own trace tag, over its own fixture (a missing anchor
/// paired with a shadowing binder).
#[trace("TC-512")]
#[test]
fn a_missing_anchor_and_a_later_duplicate_declaration_report_in_source_order() {
    let extra = "effect Applied of Tried as (again: Boolean) { true };\n";
    let source = recovery_flow("Main::Committed", extra).replacen(
        "for Main::Applied",
        "for Main::Missing",
        1,
    );
    let (text, refusals_a) = refusals(&source);
    let (_, refusals_b) = refusals(&source);
    assert_eq!(refusals_a, refusals_b, "checking twice must agree");
    assert_eq!(refusals_a.len(), 3, "{refusals_a:?}");

    let missing_span = text.find("Main::Missing").unwrap();
    let [first_decl, second_decl] = duplicate_applied_spans(&text);
    // The missing-anchor refusal is textually first (before `run`), so
    // it must sort first even though the checker builds the two
    // duplicate-declaration refusals before it.
    let mut expected = [missing_span, first_decl, second_decl];
    expected.sort_unstable();
    assert_eq!(
        expected[0], missing_span,
        "fixture sanity: missing must be earliest"
    );
    let starts: Vec<usize> = protocol_causes(&refusals_a)
        .iter()
        .map(|cause| cause.span().start)
        .collect();
    assert_eq!(
        starts, expected,
        "refusals are not in source position order"
    );
    assert!(
        matches!(
            protocol_causes(&refusals_a)[0],
            ProtocolAnchorCause::Missing { .. }
        ),
        "the earliest refusal must be the Missing one, {:?}",
        refusals_a[0]
    );
}

/// FR-113-AC-7 (TC-512): a wrong-kind target refuses
/// `ill_typed`/`type-mismatch` naming the site, the admitted kinds and
/// the actual kind, for each of the six reference positions; a
/// mismatched channel refuses the same way, naming both channels, and
/// a matching channel resolves.
#[trace("TC-512", "FR-113-AC-7")]
#[test]
fn a_resolved_anchor_of_the_wrong_kind_refuses_naming_the_actual_kind() {
    // effect-of must admit only `attempt`; naming `Recovered` (an
    // `event`, FR-113's own example) instead of `Tried` (the
    // `attempt`) is wrong-kind.
    let wrong_effect_of = recovery_flow("Main::Committed", "").replacen(
        "effect Applied of Tried",
        "effect Applied of Recovered",
        1,
    );
    let (_, found) = refusals(&wrong_effect_of);
    assert_eq!(found.len(), 1, "{found:?}");
    match protocol_causes(&found)[0] {
        ProtocolAnchorCause::WrongKind {
            site,
            actual,
            admitted,
            ..
        } => {
            assert_eq!(*site, "effect-of");
            assert_eq!(*actual, "event");
            assert_eq!(admitted.as_slice(), ["attempt"]);
        }
        other => panic!("a WrongKind cause, got {other:?}"),
    }

    // compensate-for must admit only `effect`; naming `Tried` (an
    // `attempt`) is wrong-kind.
    let wrong_compensate_for =
        recovery_flow("Main::Committed", "").replacen("Main::Applied", "Main::Tried", 1);
    let (_, found) = refusals(&wrong_compensate_for);
    assert_eq!(found.len(), 1, "{found:?}");
    match protocol_causes(&found)[0] {
        ProtocolAnchorCause::WrongKind {
            site,
            actual,
            admitted,
            ..
        } => {
            assert_eq!(*site, "compensate-for");
            assert_eq!(*actual, "attempt");
            assert_eq!(admitted.as_slice(), ["effect"]);
        }
        other => panic!("a WrongKind cause, got {other:?}"),
    }

    // compensate-commit must admit only `commit`; naming `Tried` (an
    // `attempt`) is wrong-kind.
    let wrong_compensate_commit = recovery_flow("Main::Tried", "");
    let (_, found) = refusals(&wrong_compensate_commit);
    assert_eq!(found.len(), 1, "{found:?}");
    match protocol_causes(&found)[0] {
        ProtocolAnchorCause::WrongKind {
            site,
            actual,
            admitted,
            ..
        } => {
            assert_eq!(*site, "compensate-commit");
            assert_eq!(*actual, "attempt");
            assert_eq!(admitted.as_slice(), ["commit"]);
        }
        other => panic!("a WrongKind cause, got {other:?}"),
    }

    // event-for must admit only a `compensate` template; naming
    // `Tried` (an `attempt`) is wrong-kind.
    let wrong_event_for = recovery_flow("Main::Committed", "").replacen("for Undo", "for Tried", 1);
    let (_, found) = refusals(&wrong_event_for);
    assert_eq!(found.len(), 1, "{found:?}");
    match protocol_causes(&found)[0] {
        ProtocolAnchorCause::WrongKind {
            site,
            actual,
            admitted,
            ..
        } => {
            assert_eq!(*site, "event-for");
            assert_eq!(*actual, "attempt");
            assert_eq!(admitted.as_slice(), ["compensate"]);
        }
        other => panic!("a WrongKind cause, got {other:?}"),
    }

    // await-after admits every event-like kind and a compensate
    // template, but never a structural control: naming `Main` (a
    // `sequence`) is wrong-kind.
    let awaiting_a_sequence = "await Wait after Main using v clock \"ticks\" within [0,1] \
         match event Seen by R for Undo as (seen: Boolean) { true }; \
         then sequence Then { } timeout sequence Timeout { }\n";
    let (_, found) = refusals(&recovery_flow("Main::Committed", awaiting_a_sequence));
    assert_eq!(found.len(), 1, "{found:?}");
    match protocol_causes(&found)[0] {
        ProtocolAnchorCause::WrongKind {
            site,
            actual,
            admitted,
            ..
        } => {
            assert_eq!(*site, "await-after");
            assert_eq!(*actual, "sequence");
            assert!(admitted.contains(&"send"));
            assert!(!admitted.contains(&"sequence"));
        }
        other => panic!("a WrongKind cause, got {other:?}"),
    }

    // receive-of must admit only `send`; naming `Tried` (an `attempt`)
    // is wrong-kind.
    let wrong_receive_of = "receive Got via C of Tried as (got: Boolean) { true };\n";
    let (_, found) = refusals(&recovery_flow("Main::Committed", wrong_receive_of));
    assert_eq!(found.len(), 1, "{found:?}");
    match protocol_causes(&found)[0] {
        ProtocolAnchorCause::WrongKind {
            site,
            actual,
            admitted,
            ..
        } => {
            assert_eq!(*site, "receive-of");
            assert_eq!(*actual, "attempt");
            assert_eq!(admitted.as_slice(), ["send"]);
        }
        other => panic!("a WrongKind cause, got {other:?}"),
    }

    // A channel mismatch between a matching-kind `receive` and `send`
    // refuses `type-mismatch` naming both channels; the same `receive`
    // over the matching channel resolves.
    let (_, _, result) = resolve_protocol(&handoff_protocol("C"));
    result.unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let (_, found) = refusals(&handoff_protocol("D"));
    assert_eq!(found.len(), 1, "{found:?}");
    match protocol_causes(&found)[0] {
        ProtocolAnchorCause::ChannelMismatch {
            receive_channel,
            send_channel,
            ..
        } => {
            assert_eq!(receive_channel, "D");
            assert_eq!(send_channel, "C");
        }
        other => panic!("a ChannelMismatch cause, got {other:?}"),
    }
}

/// SR-753/SR-754-style mutation check, over a path
/// `a_resolved_anchor_of_the_wrong_kind_...` does not itself exercise:
/// flipping the channel comparison to always agree (as an unresolved
/// check would) must turn this test red, and reverting must turn it
/// green again.
#[trace("TC-512", "FR-113-AC-7")]
#[test]
fn a_channel_mismatch_refusal_is_not_vacuous_over_an_otherwise_valid_protocol() {
    let matching = handoff_protocol("C");
    let (_, _, result) = resolve_protocol(&matching);
    result.unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let mutated = handoff_protocol("D");
    assert_ne!(matching, mutated);
    let (_, refusals) = refusals(&mutated);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(matches!(
        protocol_causes(&refusals)[0],
        ProtocolAnchorCause::ChannelMismatch { .. }
    ));
}
