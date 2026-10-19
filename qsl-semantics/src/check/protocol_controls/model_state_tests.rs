//! FR-218 (TC-663, TC-1056, TC-1065) over fixture R of the protocol
//! obligation tests, whose object, record, reference and event types these
//! criteria read: a channel's `fifo` ordering, its binder and its key, and
//! the model-state rule of guards, `visible` entries and keys.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use quire_exact::ValueType;
use quire_semantic_value::checking::CheckingLimits;

use super::CheckedOrdering;
use crate::check::refusal::ProtocolAnchorCause;
use crate::check::{CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations};

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
fn only(
    text: &str,
) -> (
    ProtocolAnchorCause,
    (&'static str, Option<&'static str>),
    &str,
) {
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
    let ProtocolAnchorCause::CarriesMismatch {
        channel,
        expected,
        actual,
        ..
    } = &cause
    else {
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
        let ordering = ordering(&with_channel(
            carries,
            &format!("fifo by (m: {carries}) {{ {key} }}"),
        ));
        assert!(matches!(ordering, CheckedOrdering::Fifo { .. }), "{key}");
    }
    let text = with_channel("M::Pulse", "fifo by (m: M::Pulse) { m }");
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("ill_typed", Some("operator-ineligible")));
    let ProtocolAnchorCause::KeyEquality {
        channel, key_type, ..
    } = &cause
    else {
        panic!("a key-equality refusal, got {cause:?}");
    };
    assert_eq!(channel, "Q");
    assert!(matches!(**key_type, ValueType::Composite(_)));
    assert_eq!(at, "m");
}

/// A protocol `Decide` whose run tree is `run`, over `view: M::Node`.
fn decide(members: &str, run: &str) -> String {
    format!(
        "{HEADER}protocol Decide using v over (view: M::Node) on origin {{\n\
         role Service on M::Node;\n\
         {members}\n\
         run {run}\n\
         finish Closed as (closed: M::Receipt) {{ true }};\n\
         }}\n"
    )
}

/// An attempt `Tried` by `Service` with binder `attempted: binder_type`,
/// then a two-case `choice Pick` by `Service`.
fn attempted(binder_type: &str, visible: &str, yes: &str, no: &str) -> String {
    format!(
        "sequence Main {{ attempt Tried by Service on M::Node::step contracts [] \
         as (attempted: {binder_type}) {{ true }}; \
         choice Pick by Service visible ({visible}) {{ \
         case yes when {{ {yes} }} sequence Yes {{ }} \
         case no when {{ {no} }} sequence No {{ }} }} }}"
    )
}

/// FR-218-AC-2: a `choice` guard that reads the `over` input's field `k.v`
/// refuses `ill_typed`/`operator-ineligible` at the read; a two-case
/// `choice` whose guards are `m.n > 0` and `m.n > 1` refuses
/// `undefined_expression`/`unproved-exhaustiveness`; a join that names a
/// label that is no branch refuses `missing_declaration`/`missing-name` at
/// the label.
#[trace("TC-663", "FR-218-AC-2")]
#[test]
fn a_guard_reads_no_model_state_and_partitions_its_facts() {
    let text = decide(
        "",
        &attempted("M::Receipt", "", "view.n > 0", "not (view.n > 0)"),
    );
    let refusals = check(&text)
        .map(|_| ())
        .expect_err("the guard reads model state");
    let reads: Vec<&str> = refusals
        .iter()
        .map(|refusal| {
            let CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
                panic!("a protocol cause, got {refusal:?}");
            };
            assert!(
                matches!(cause.as_ref(), ProtocolAnchorCause::SlotModelState { node, .. } if node == "Pick"),
                "{cause:?}"
            );
            assert_eq!(refusal.cause.code().as_str(), "ill_typed");
            assert_eq!(refusal.cause.cause(), Some("operator-ineligible"));
            let span = cause.span();
            &text[span.start..span.end]
        })
        .collect();
    assert_eq!(reads, ["view.n", "view.n"]);

    let text = decide(
        "",
        &attempted(
            "M::Receipt",
            "attempted.n",
            "attempted.n > 0",
            "attempted.n > 1",
        ),
    );
    let (cause, pair, at) = only(&text);
    assert_eq!(
        pair,
        ("undefined_expression", Some("unproved-exhaustiveness"))
    );
    assert!(matches!(cause, ProtocolAnchorCause::Exhaustiveness { .. }));
    assert_eq!(at, "Pick");

    let text = decide(
        "",
        "parallel Fill { branch left sequence L { } branch right sequence R { } } \
         join all [left, middle];",
    );
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("missing_declaration", Some("missing-name")));
    assert!(matches!(cause, ProtocolAnchorCause::Missing { .. }));
    assert_eq!(at, "middle");
}

/// FR-218-AC-26: a `choice` guard and a `visible` entry that read a field of
/// an object-typed attempt binder, and a FIFO key that reads a field through
/// a reference-typed field of its record binder, each refuse
/// `ill_typed`/`operator-ineligible` at the read, naming the guard, the
/// `visible` entry or the key; the same reads over a record binder's own
/// fields check. A channel that carries an object type refuses once, at the
/// type, and gives no refusal at its `send`, `receive` or FIFO binders.
#[trace("TC-1056", "FR-218-AC-26")]
#[test]
fn an_object_or_reference_read_is_model_state() {
    let slots = |text: &str| -> Vec<(crate::check::SlotKind, String)> {
        let refusals = check(text)
            .map(|_| ())
            .expect_err("the reads are model state");
        refusals
            .iter()
            .map(|refusal| {
                let CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
                    panic!("a protocol cause, got {refusal:?}");
                };
                let ProtocolAnchorCause::SlotModelState { node, slot, span } = cause.as_ref()
                else {
                    panic!("a model-state refusal, got {cause:?}");
                };
                assert_eq!(node, "Pick");
                assert_eq!(refusal.cause.cause(), Some("operator-ineligible"));
                (*slot, text[span.start..span.end].to_owned())
            })
            .collect()
    };
    let guard = decide(
        "",
        &attempted("M::Node", "", "attempted.n > 0", "not (attempted.n > 0)"),
    );
    assert_eq!(
        slots(&guard),
        [
            (crate::check::SlotKind::Guard, "attempted.n".to_owned()),
            (crate::check::SlotKind::Guard, "attempted.n".to_owned()),
        ]
    );
    let visible = decide(
        "",
        &attempted("M::Node", "attempted.n > 0", "true", "false"),
    );
    assert_eq!(
        slots(&visible),
        [(crate::check::SlotKind::Visible, "attempted.n".to_owned())]
    );
    let text = with_channel("M::Hop", "fifo by (m: M::Hop) { m.node.n }");
    let (cause, pair, at) = only(&text);
    assert_eq!(pair, ("ill_typed", Some("operator-ineligible")));
    assert!(matches!(cause, ProtocolAnchorCause::KeyModelState { .. }));
    assert_eq!(at, "m.node.n");

    // The same reads over a record binder's own fields check.
    check(&decide(
        "",
        &attempted(
            "M::Receipt",
            "attempted.n > 0",
            "attempted.n > 0",
            "not (attempted.n > 0)",
        ),
    ))
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    check(&with_channel("M::Hop", "fifo by (m: M::Hop) { m.ready }"))
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));

    // A channel of an object type refuses once, at the type.
    let text = decide(
        "channel Q from Service to Service carries M::Node ordering fifo by (m: M::Node) \
         { true } delivery unknown capacity 1 overflow reject;",
        "sequence Main { send Tx via Q as (tx: M::Node) { true }; \
         receive Rx via Q of Tx as (rx: M::Node) { true }; }",
    );
    let (cause, pair, at) = only(&text);
    assert_eq!(
        pair,
        ("invalid_model_binding", Some("malformed-declaration"))
    );
    assert!(matches!(
        cause,
        ProtocolAnchorCause::ChannelCarriesObject { .. }
    ));
    assert_eq!(at, "M::Node");
}
