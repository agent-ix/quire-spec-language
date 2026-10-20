//! The unchecked-content refusal and the grammar-required content checks.

use ix_trace_rs::trace;

use super::attempt_fixture::attempt_flow;
use super::{assemble, checks, only_cause, recovery_flow, refusals};
use crate::check::refusal::ProtocolAnchorCause;

/// SR-770 FND-001, now that every node kind and body is checked: a
/// protocol whose anchors (FR-113) and attempt bindings (FR-114) all resolve
/// and that holds an `effect`, an `event` and a `commit` checks, and a body
/// that reads its node's own binder checks too, while the same protocol
/// with a `related by` clause naming no relationship refuses
/// `missing_declaration`/`missing-name` at the name (FR-218
/// "Relationships") rather than compiling with the clause unchecked.
#[trace("TC-511", "FR-113")]
#[test]
fn a_resolving_protocol_with_unchecked_content_still_refuses() {
    let source = recovery_flow("Main::Committed", "");
    checks(&source);
    checks(&source.replacen(
        "as (applied: Boolean) { true }",
        "as (applied: Boolean) { applied }",
        1,
    ));
    let source = source.replacen(
        "as (applied: Boolean) { true }",
        "as (applied: Boolean) related by Pays (applied, applied) { true }",
        1,
    );
    let (refusal, cause) = only_cause(&source);
    assert_eq!(refusal.cause.code().as_str(), "missing_declaration");
    assert_eq!(refusal.cause.cause(), Some("missing-name"));
    let ProtocolAnchorCause::MissingRelated { name, span, .. } = cause else {
        panic!("a missing relationship, got {cause:?}");
    };
    assert_eq!(name, "Pays");
    let (text, _) = assemble(&source);
    assert_eq!(&text[span.start..span.end], "Pays");
}

/// A protocol made only of parts the checker covers in full
/// (a role, a `sequence`, an `attempt` and a `finish`, each body a bare
/// Boolean literal) checks through the whole pipeline, and its checked
/// attempt names the resolved operation's own anchor and frame nodes.
#[trace("TC-513", "FR-114")]
#[test]
fn a_fully_covered_attempt_protocol_checks() {
    let graph = checks(&attempt_flow("Flow", ""));
    let [protocol] = graph.protocols() else {
        panic!("one protocol: {:?}", graph.protocols());
    };
    assert_eq!(protocol.name, "Flow");
    let [attempt] = protocol.attempts.as_slice() else {
        panic!("one attempt: {:?}", protocol.attempts);
    };
    assert_ne!(attempt.anchor, attempt.frame);
}

/// One `content` refusal per unchecked or ill-formed part of an
/// otherwise fully covered protocol (SR-770 FND-001), each checked
/// against its own catalog code, tag and cause.
#[trace("TC-513", "FR-114")]
#[test]
fn each_unchecked_part_of_an_attempt_protocol_refuses() {
    let base = attempt_flow("Flow", "");
    let cases: [(&str, &str, &str, &str); 7] = [
        // An ill-typed body is refused by the clause checker.
        (
            "as (tried: Boolean) { true }",
            "as (tried: Boolean) { 1 + true }",
            "ill_typed",
            "type-mismatch",
        ),
        // A `send` via a name that is no channel (FR-218 "Channels").
        (
            "contracts [] as (tried: Boolean) { true };\n",
            "contracts [] as (tried: Boolean) { true };\n\
             send Ping via R as (ping: Boolean) { true };\n",
            "missing_declaration",
            "missing-name",
        ),
        (
            "role R on M::Actor;",
            "role R on Nope::Actor;",
            "missing_declaration",
            "missing-name",
        ),
        (
            "attempt Tried by R",
            "attempt Tried by Q",
            "missing_declaration",
            "missing-name",
        ),
        (
            "(tried: Boolean)",
            "(tried: Undeclared)",
            "missing_declaration",
            "missing-name",
        ),
        (
            "using v over",
            "using M over",
            "missing_declaration",
            "missing-name",
        ),
        (
            "role R on M::Actor;",
            "role R on M::Actor;\nrole R on M::Actor;",
            "ambiguous_declaration",
            "ambiguous-name",
        ),
    ];
    for (from, to, code, tag) in cases {
        let source = base.replacen(from, to, 1);
        assert_ne!(source, base, "{from}");
        let (_, found) = refusals(&source);
        assert!(!found.is_empty(), "{source}");
        for refusal in &found {
            assert_eq!(refusal.cause.code().as_str(), code, "{source}: {found:?}");
            assert_eq!(refusal.cause.cause(), Some(tag), "{source}: {found:?}");
        }
    }
}

/// QSpec FR-464 "Profiles and laws": a `compensate` template reads a clock
/// role, so under no temporal profile it refuses `missing_declaration`/
/// `missing-selection` at the template's name, naming it; with one selected
/// the same protocol checks.
#[trace("QSpec-FR-464")]
#[test]
fn a_compensate_template_needs_a_temporal_profile() {
    let source = recovery_flow("Main::Committed", "");
    checks(&source);
    let header = super::HEADER.replace("profile t = \"quire.temporal.infinite-trace/v1\";\n", "");
    assert_ne!(
        header,
        super::HEADER,
        "the fixture header selects a temporal profile"
    );
    let (text, assembled) = super::assemble_with(&header, &source);
    let found = assembled
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect_err("a template under no temporal profile refuses");
    let [refusal] = found.as_slice() else {
        panic!("one refusal: {found:?}");
    };
    assert_eq!(refusal.cause.code().as_str(), "missing_declaration");
    assert_eq!(refusal.cause.cause(), Some("missing-selection"));
    let crate::check::CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
        panic!("a protocol cause: {refusal:?}");
    };
    let ProtocolAnchorCause::MissingTemporalProfile {
        construct,
        node,
        span,
    } = cause.as_ref()
    else {
        panic!("a missing temporal profile, got {cause:?}");
    };
    assert_eq!((*construct, node.as_str()), ("compensate", "Undo"));
    assert_eq!(&text[span.start..span.end], "Undo");
}
