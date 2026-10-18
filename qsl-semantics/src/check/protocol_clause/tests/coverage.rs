//! The unchecked-content refusal and the grammar-required content checks.

use ix_trace_rs::trace;

use super::attempt_fixture::attempt_flow;
use super::{assemble, checks, only_cause, recovery_flow, refusals, resolve_protocol};
use crate::check::refusal::ProtocolAnchorCause;

/// SR-770 FND-001: a protocol whose anchors (FR-113) and
/// attempt bindings (FR-114) all resolve, but which holds content no
/// checker reads yet (here a `compensate` template, an `effect`, an
/// `event` and a `commit`), still refuses
/// `unsupported_construct`/`not-yet-implemented` once, at the earliest
/// such construct -- the `compensate` template -- rather than compiling
/// with that content unchecked and silently missing from the emitted
/// package.
#[trace("TC-511", "FR-113")]
#[test]
fn a_resolving_protocol_with_unchecked_content_still_refuses() {
    let source = recovery_flow("Main::Committed", "");
    let (_, _, result) = resolve_protocol(&source);
    result.unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let (refusal, cause) = only_cause(&source);
    assert_eq!(refusal.cause.code().as_str(), "unsupported_construct");
    assert_eq!(refusal.cause.cause(), Some("not-yet-implemented"));
    match cause {
        ProtocolAnchorCause::Unimplemented {
            name,
            construct,
            span,
        } => {
            assert_eq!(name, "RecoveryFlow");
            assert_eq!(construct, "compensate");
            let (text, _) = assemble(&source);
            assert!(text[span.start..].starts_with("Undo"), "{span:?}");
        }
        other => panic!("an Unimplemented cause, got {other:?}"),
    }
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
        // An ill-typed body is not checked yet: refused, not accepted.
        (
            "as (tried: Boolean) { true }",
            "as (tried: Boolean) { 1 + true }",
            "unsupported_construct",
            "not-yet-implemented",
        ),
        // A node kind other than `sequence`/`attempt`/`finish`.
        (
            "contracts [] as (tried: Boolean) { true };\n",
            "contracts [] as (tried: Boolean) { true };\n\
             send Ping via R as (ping: Boolean) { true };\n",
            "unsupported_construct",
            "not-yet-implemented",
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
