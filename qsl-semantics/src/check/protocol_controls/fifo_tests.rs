//! FR-218 "Channels" (TC-1056): a channel's `fifo` ordering, its binder and
//! its key, over fixture R of the protocol obligation tests.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use quire_exact::ValueType;
use quire_semantic_value::checking::CheckingLimits;

use super::CheckedOrdering;
use crate::check::{CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations};
use crate::check::refusal::ProtocolAnchorCause;

mod fixture {
    use crate as semantics;
    include!("../../../tests/fixtures/protocol_obligations.rs");
}
use fixture::{obligation_model, HEADER};

/// A protocol whose one channel `Q` carries `carries`, ordered `ordering`.
fn with_channel(carries: &str, ordering: &str) -> String {
    format!(
        "{HEADER}protocol Queue using v over (view: M::Node) on origin {{\n\
         role Service on M::Node;\n\
         channel Q from Service to Service carries {carries} ordering {ordering} \
         delivery unknown capacity 1 overflow reject;\n\
         run sequence Main {{\n\
         attempt Tried by Service on M::Node::step contracts [] as (attempted: M::Flag) {{ true }};\n\
         }}\n\
         finish Closed as (closed: M::Receipt) {{ true }};\n\
         }}\n"
    )
}

fn check(text: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![obligation_model()],
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: the assembler refuses: {refusal:?}"))
    .check(CheckingLimits::default())
}

/// The one channel's checked ordering.
fn ordering(text: &str) -> CheckedOrdering {
    let graph = check(text).unwrap_or_else(|refusals| panic!("{text}: {refusals:?}"));
    let [channel] = graph.protocols()[0].channels.as_slice() else {
        panic!("one channel");
    };
    channel.ordering.clone()
}

/// The one refusal's cause, its catalog pair and the text at its span.
fn only(text: &str) -> (ProtocolAnchorCause, (&'static str, Option<&'static str>), &str) {
    let refusals = check(text).map(|_| ()).expect_err("the protocol refuses");
    let [refusal] = refusals.as_slice() else {
        panic!("{text}: one refusal, got {refusals:?}");
    };
    let CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
        panic!("a protocol cause, got {refusal:?}");
    };
    let span = cause.span();
    (
        (**cause).clone(),
        (refusal.cause.code().as_str(), refusal.cause.cause()),
        &text[span.start..span.end],
    )
}

/// FR-218-AC-12: `ordering fifo by (m: T) {m.ready}` on a channel carrying
/// `T` checks and the checked channel holds `Fifo` with that binder and key;
/// a binder of another type refuses `ill_typed`/`type-mismatch` at the type;
/// a key that reads model state refuses `ill_typed`/`operator-ineligible` at
/// the read; `ordering unordered` holds no key.
#[trace("TC-1056", "FR-218-AC-12")]
#[test]
fn a_fifo_key_is_checked_over_its_binder() {
    let CheckedOrdering::Fifo { binder, key } =
        ordering(&with_channel("M::Flag", "fifo by (m: M::Flag) { m.ready }"))
    else {
        panic!("a Fifo ordering");
    };
    assert_eq!(binder.name, "m");
    assert_eq!(key.root().value_type(), &ValueType::Boolean);
    assert_eq!(
        ordering(&with_channel("M::Flag", "unordered")),
        CheckedOrdering::Unordered
    );

    let text = with_channel("M::Flag", "fifo by (m: M::Tick) { m.ready }");
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("ill_typed", Some("type-mismatch")));
    let ProtocolAnchorCause::CarriesMismatch { channel, expected, actual, .. } = &cause else {
        panic!("a carries mismatch, got {cause:?}");
    };
    assert_eq!(channel, "Q");
    assert_ne!(expected, actual);
    assert_eq!(at, "M::Tick");

    let text = with_channel("M::Hop", "fifo by (m: M::Hop) { m.node.total = 1 }");
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("ill_typed", Some("operator-ineligible")));
    assert!(matches!(&cause, ProtocolAnchorCause::KeyModelState { channel, .. } if channel == "Q"));
    assert_eq!(at, "m.node.total");
}

/// FR-218-AC-20: a FIFO key of type `Boolean`, and a key of a record type,
/// check because the clause checker admits `=` over them; a key of an event
/// type, over which it admits no `=`, refuses `ill_typed`/
/// `operator-ineligible` at the key, retaining the type.
#[trace("TC-1056", "FR-218-AC-20")]
#[test]
fn a_fifo_key_admits_equality() {
    for (carries, key) in [("M::Flag", "m.ready"), ("M::Flag", "m")] {
        let ordering = ordering(&with_channel(carries, &format!("fifo by (m: {carries}) {{ {key} }}")));
        assert!(matches!(ordering, CheckedOrdering::Fifo { .. }), "{key}");
    }
    let text = with_channel("M::Pulse", "fifo by (m: M::Pulse) { m }");
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("ill_typed", Some("operator-ineligible")));
    let ProtocolAnchorCause::KeyEquality { channel, key_type, .. } = &cause else {
        panic!("a key-equality refusal, got {cause:?}");
    };
    assert_eq!(channel, "Q");
    assert!(matches!(key_type, ValueType::Composite(_)));
    assert_eq!(at, "m");
}
