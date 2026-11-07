// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retained criteria whose tests rode the deleted composed lane, re-owned on
//! the spine (`qsl_replay::spine` parse, select, check): FR-035-AC-2 and
//! AC-3 (source refusals), FR-040-AC-4 (exact integer bounds) and
//! FR-048-AC-2 and AC-3 (a checked protocol's channels and control nodes).

use ix_trace_rs::trace;
use qsl_forms::ProtocolNodeKind;
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::{
    check, parse, refusal_or_fault, select, CheckedUnit, CompileRefusal, DependencyInput,
    ParseRequest, SpineLimits,
};
use qsl_semantics::check::{ChannelCapacity, ChannelDelivery, ChannelOverflow, CheckedOrdering};
use quire_exact::Cancel;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";

/// `body` after the `1-draft` header, taken through parse, select and check.
fn spine_check(body: &str) -> Result<CheckedUnit, Box<CompileRefusal>> {
    let text = format!("{HEADER}{body}\n");
    let source = SourceIdentity::new("agent-ix", "test:spine-retained");
    let cancel = Cancel::new();
    let limits = SpineLimits::default();
    let packages =
        qsl_semantics::model::intake::package_input([crate::support::spine_model::DOCUMENT])
            .expect("the model fixture enters intake");
    let fault = |failure| refusal_or_fault(failure).unwrap_or_else(|f| panic!("fault: {f:?}"));
    let parsed = parse(
        &ParseRequest {
            source: &source,
            path: "retained.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(fault)?
    .into_value();
    let models = select(&parsed, &packages, limits.model, &cancel)
        .map_err(fault)?
        .into_value();
    Ok(check(
        &parsed,
        "spine-retained".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(fault)?
    .into_value())
}

/// FR-035-AC-2: nonchained comparisons, temporal syntax in a value argument,
/// malformed family bodies and trailing garbage refuse at the parser with
/// no successful unit, while a chained `and` of comparisons is admitted.
#[trace("TC-113", "FR-035-AC-2")]
#[test]
fn malformed_source_refuses_at_the_parser_and_grouped_comparisons_admit() {
    for body in [
        "function f using v(a: Integer, b: Integer, c: Integer): Boolean pure { a < b < c }",
        "function f using v(a: Integer, b: Integer, c: Integer): Boolean pure { a = b = c }",
        "function f using v(a: Integer): Integer pure { always a }",
        "predicate P using v (a: Integer): Boolean { }",
        "function f using v(a: Integer): Integer pure { a } garbage",
    ] {
        let refusal = spine_check(body).err().unwrap_or_else(|| {
            panic!("{body}: a malformed unit must not produce a successful unit")
        });
        assert!(
            matches!(*refusal, CompileRefusal::Source(_)),
            "{body}: refused at the parser: {refusal}"
        );
    }
    spine_check(
        "function f using v(a: Integer, b: Integer, c: Integer): Boolean pure { a < b and b < c }",
    )
    .expect("comparisons grouped by `and` are admitted");
}

/// FR-035-AC-3: under `1-draft`, `result` and `retry` are reserved and
/// refuse as a native binder, while a neighbouring spelling is admitted.
#[trace("TC-113", "FR-035-AC-3")]
#[test]
fn result_and_retry_refuse_as_native_binders() {
    for word in ["result", "retry"] {
        let body = format!("function f using v({word}: Integer): Integer pure {{ {word} }}");
        let refusal = spine_check(&body)
            .err()
            .unwrap_or_else(|| panic!("`{word}` must refuse as a binder"));
        assert!(
            matches!(*refusal, CompileRefusal::Source(_)),
            "{word}: {refusal}"
        );
    }
    spine_check("function f using v(results: Integer): Integer pure { results }")
        .expect("an unreserved spelling is an ordinary binder");
}

/// FR-040-AC-4: inclusive integer extrema are admitted exactly, and an
/// operation whose proved interval exceeds the declared result range refuses
/// with the exact required and proved bounds.
#[trace("TC-119", "FR-040-AC-4")]
#[test]
fn integer_bounds_are_inclusive_and_exact() {
    spine_check(
        "function f using v(x: Int[-9223372036854775808, 9223372036854775807]): \
         Int[-9223372036854775808, 9223372036854775807] pure { x }",
    )
    .expect("the i64 extrema are inclusive bounds");
    spine_check("function f using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }")
        .expect("x + 1 is proved inside [0, 10]");

    let refusal = spine_check("function f using v(x: Int[0, 9]): Int[0, 9] pure { x + 1 }")
        .err()
        .unwrap_or_else(|| panic!("x + 1 is not proved inside [0, 9]"));
    let CompileRefusal::Check { .. } = *refusal else {
        panic!("a bounds failure is a check refusal: {refusal:?}");
    };
    let shown = format!("{refusal:?}");
    assert!(
        shown.contains("lower: Some(Integer(1))") && shown.contains("upper: Some(Integer(10))"),
        "the refusal carries the exact proved interval [1, 10]: {shown}"
    );
}

/// FR-048-AC-2 and AC-3: a checked protocol retains each channel's roles,
/// payload and delivery premises, and its `run` tree keeps `sequence`,
/// `parallel` with its branches and join, `send`, `receive`, `choice` with
/// its cases and `check` as distinct control nodes in authored order.
#[trace("TC-132", "TC-133", "FR-048-AC-2", "FR-048-AC-3")]
#[test]
fn a_checked_protocol_retains_its_channels_and_control_topology() {
    let checked = spine_check(
        "profile Complete = \"quire.protocol.finite-global/v1\";\n\
         model M = \"acme/orders\";\n\
         protocol Flow using Complete over (input: M::Widget) on origin {\n\
           role R on M::Widget;\n\
           channel C from R to R carries Boolean ordering unordered delivery at-least-once \
             capacity 2 overflow block;\n\
           run sequence Main {\n\
             parallel Both {\n\
               branch s sequence S { send Tx via C as (tx: Boolean) { true }; }\n\
               branch r sequence Rc { receive Rx via C of s::S::Tx as (rx: Boolean) { true }; }\n\
             } join all [s, r];\n\
             choice Pick by R visible () {\n\
               case yes when { true } sequence Y { }\n\
               case no when { false } sequence N { }\n\
             }\n\
             check Ok using v { true };\n\
           }\n\
           finish End as (outcome: Boolean) { true };\n\
         }",
    )
    .expect("the protocol checks on the spine");
    let [protocol] = checked.package().graph().protocols() else {
        panic!("one checked protocol");
    };

    let [channel] = protocol.channels.as_slice() else {
        panic!("one checked channel: {:?}", protocol.channels);
    };
    assert_eq!(
        (
            channel.name.as_str(),
            channel.from.as_str(),
            channel.to.as_str()
        ),
        ("C", "R", "R")
    );
    assert_eq!(channel.carries, quire_exact::ValueType::Boolean);
    assert!(matches!(channel.ordering, CheckedOrdering::Unordered));
    assert_eq!(channel.delivery, ChannelDelivery::AtLeastOnce);
    assert_eq!(channel.capacity, ChannelCapacity::Literal(2));
    assert_eq!(channel.overflow, ChannelOverflow::Block);

    let tree: Vec<(ProtocolNodeKind, &str)> = protocol
        .controls
        .controls
        .iter()
        .map(|control| (control.kind, control.name.as_str()))
        .collect();
    assert_eq!(
        tree,
        [
            (ProtocolNodeKind::Sequence, "Main"),
            (ProtocolNodeKind::Parallel, "Both"),
            (ProtocolNodeKind::Branch, "s"),
            (ProtocolNodeKind::Sequence, "S"),
            (ProtocolNodeKind::Send, "Tx"),
            (ProtocolNodeKind::Branch, "r"),
            (ProtocolNodeKind::Sequence, "Rc"),
            (ProtocolNodeKind::Receive, "Rx"),
            (ProtocolNodeKind::Choice, "Pick"),
            (ProtocolNodeKind::Case, "yes"),
            (ProtocolNodeKind::Sequence, "Y"),
            (ProtocolNodeKind::Case, "no"),
            (ProtocolNodeKind::Sequence, "N"),
            (ProtocolNodeKind::Check, "Ok"),
        ]
    );
    let root = protocol.controls.root().expect("the run root");
    let children: Vec<&str> = root
        .children
        .iter()
        .map(|node| {
            protocol
                .controls
                .control(*node)
                .expect("child")
                .name
                .as_str()
        })
        .collect();
    assert_eq!(
        children,
        ["Both", "Pick", "Ok"],
        "children keep authored order"
    );
    let [parallel] = protocol.parallels.as_slice() else {
        panic!("one checked parallel join: {:?}", protocol.parallels);
    };
    assert_eq!(parallel.joined, ["s", "r"]);
}
