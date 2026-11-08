// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retained criteria whose tests rode the deleted composed lane, re-owned on
//! the spine (`qsl_replay::spine` parse, select, check): FR-035-AC-2 and
//! AC-3 (source refusals), FR-040-AC-4 (exact integer bounds) and
//! FR-048-AC-2 and AC-3 (a checked protocol's channels and control nodes).

use ix_trace_rs::trace;
use qsl_cst::{CompleteCause, CompleteCode};
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
/// refuse as a native binder with `invalid_syntax`/`unexpected-token`, a
/// neighbouring spelling is an ordinary binder, and as a qualified model
/// member (`M::T::result`, `M::T::retry`) both parse.
#[trace("TC-113", "FR-035-AC-3")]
#[test]
fn result_and_retry_refuse_as_binders_and_parse_as_qualified_members() {
    for word in ["result", "retry"] {
        let body = format!("function f using v({word}: Integer): Integer pure {{ {word} }}");
        let refusal = spine_check(&body)
            .err()
            .unwrap_or_else(|| panic!("`{word}` must refuse as a binder"));
        let CompileRefusal::Source(diagnostic) = *refusal else {
            panic!("{word}: refused at the parser");
        };
        assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax, "{word}");
        assert_eq!(diagnostic.cause, CompleteCause::UnexpectedToken, "{word}");

        let member = format!(
            "{HEADER}model M = \"acme/orders\";\npre P using v on M::Widget::{word} {{ true }}\n"
        );
        let source = SourceIdentity::new("agent-ix", "test:spine-retained-member");
        let parsed = parse(
            &ParseRequest {
                source: &source,
                path: "member.native",
                bytes: member.as_bytes(),
            },
            SpineLimits::default().source,
            &Cancel::new(),
        );
        assert!(
            parsed.is_ok(),
            "`M::Widget::{word}` parses in its qualified position"
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

/// The model fixture's `Widget` with one extra field `name` typed as a
/// reference to `Widget`, of presence `presence` and multiplicity
/// `[0, upper]` (`ordered` and `unique` as given).
fn widget_with_edge(
    name: &str,
    presence: &str,
    bounds: (u64, u64),
    (ordered, unique): (bool, bool),
) -> qsl_semantics::model::intake::PackageInput {
    let mut document: serde_json::Value =
        serde_json::from_slice(crate::support::spine_model::DOCUMENT).expect("fixture JSON");
    let identity = format!("ix://acme/orders/Widget/{name}");
    document["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == "ix://acme/orders/Widget")
        .expect("Widget")["fields"]
        .as_array_mut()
        .expect("fields")
        .push(serde_json::json!({
            "identity": identity,
            "name": name,
            "typeRef": "ix://acme/orders/Widget",
            "presence": presence,
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {
                "lower": bounds.0, "upper": bounds.1, "ordered": ordered, "unique": unique
            },
            "origin": {"generated": {
                "generatorIdentity": identity,
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }},
        }));
    let bytes = serde_json::to_vec(&document).expect("serializes");
    qsl_semantics::model::intake::package_input([bytes.as_slice()]).expect("the package enters")
}

/// `unit` over `packages` through parse, select and check.
fn spine_check_over(
    unit: &str,
    packages: &qsl_semantics::model::intake::PackageInput,
) -> Result<CheckedUnit, Box<CompileRefusal>> {
    let source = SourceIdentity::new("agent-ix", "test:spine-edges");
    let cancel = Cancel::new();
    let limits = SpineLimits::default();
    let fault = |failure| refusal_or_fault(failure).unwrap_or_else(|f| panic!("fault: {f:?}"));
    let parsed = parse(
        &ParseRequest {
            source: &source,
            path: "edges.native",
            bytes: unit.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(fault)?
    .into_value();
    let models = select(&parsed, packages, limits.model, &cancel)
        .map_err(fault)?
        .into_value();
    Ok(check(
        &parsed,
        "spine-edges".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(fault)?
    .into_value())
}

/// FR-047-AC-6 (static half): `reaches(a, b, field)` admits only a
/// `Reference<T>` field, optional or required, and an ordered
/// `Sequence<Reference<T>>` field; a set, a bag, an optional sequence and a
/// scalar field refuse `ill_typed`/`operator-ineligible` at the `reaches`.
/// The evaluation half (absent optional slot, empty sequence) is
/// `absent_optional_and_empty_sequence_edges_have_no_outgoing_edge`.
#[trace("TC-130", "FR-047-AC-6")]
#[test]
fn reaches_admits_only_reference_option_and_sequence_edges() {
    let unit = |edge: &str| {
        format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.model.complete/v1\";\n\
             model M = \"acme/orders\";\n\
             invariant I using v on M::Widget at current {{ reaches(self, self, {edge}) }}\n"
        )
    };
    let admitted = [
        ("optional", "optional", (1, 1), (false, true)),
        ("single", "required", (1, 1), (false, false)),
        ("kids", "required", (0, 3), (true, false)),
    ];
    for (name, presence, bounds, shape) in admitted {
        let packages = widget_with_edge(name, presence, bounds, shape);
        spine_check_over(&unit(name), &packages)
            .unwrap_or_else(|refusal| panic!("`{name}` is an admitted edge: {refusal}"));
    }
    let refused = [
        ("tags", "required", (0, 3), (false, true)),
        ("pile", "required", (0, 3), (false, false)),
        ("maybe_kids", "optional", (0, 3), (true, false)),
    ];
    for (name, presence, bounds, shape) in refused {
        let packages = widget_with_edge(name, presence, bounds, shape);
        let text = unit(name);
        let refusal = spine_check_over(&text, &packages)
            .err()
            .unwrap_or_else(|| panic!("`{name}` is not an admitted edge"));
        let CompileRefusal::Check { region, .. } = refusal.as_ref() else {
            panic!("`{name}` refuses at the check: {refusal:?}");
        };
        let shown = format!("{refusal:?}");
        assert!(shown.contains("OperatorIneligible"), "{name}: {shown}");
        let region = region.as_ref().expect("a located refusal");
        let start = usize::try_from(region.start()).unwrap();
        let end = usize::try_from(region.end()).unwrap();
        assert!(
            text.get(start..end)
                .is_some_and(|span| span.contains("reaches")),
            "{name}: refusal located at the reaches"
        );
    }
    let scalar = widget_with_edge("unused", "optional", (0, 1), (false, false));
    let refusal = spine_check_over(&unit("code"), &scalar)
        .err()
        .unwrap_or_else(|| panic!("an Integer field is not an edge"));
    assert!(format!("{refusal:?}").contains("OperatorIneligible"));
}

/// The model fixture with an operation `touch` on `Widget`, so a protocol
/// can author an `attempt` and its `effect`.
fn widget_with_touch() -> qsl_semantics::model::intake::PackageInput {
    let mut document: serde_json::Value =
        serde_json::from_slice(crate::support::spine_model::DOCUMENT).expect("fixture JSON");
    let identity = "ix://acme/orders/Widget/touch";
    let generated = serde_json::json!({"generated": {
        "generatorIdentity": identity,
        "generatorVersion": "1.0.0",
        "inputIdentities": [identity],
    }});
    document["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == "ix://acme/orders/Widget")
        .expect("Widget")["operations"] = serde_json::json!([{
        "identity": identity,
        "name": "touch",
        "params": [],
        "pre": [],
        "post": [],
        "origin": generated,
        "frame": {"modifies": [], "creates": [], "deletes": []},
    }]);
    let bytes = serde_json::to_vec(&document).expect("serializes");
    qsl_semantics::model::intake::package_input([bytes.as_slice()]).expect("the package enters")
}

/// FR-048-AC-2 and AC-3: a checked protocol retains each channel's roles,
/// payload and premises; keeps `send`, `receive`, `attempt`, `effect`,
/// `event`, `commit`, `await` (with its `then` and `timeout`), `repeat` (with
/// its body and exhaustion), `choice`, `parallel` and `check` as distinct
/// control nodes with their authored parent and child edges; and gives the
/// send, receive, attempt and effect distinct node identities.
#[trace("TC-132", "TC-133", "FR-048-AC-2", "FR-048-AC-3")]
#[test]
fn a_checked_protocol_retains_its_channels_and_control_topology() {
    let unit = format!(
        "{HEADER}profile Complete = \"quire.protocol.finite-global/v1\";\n\
         profile e = \"quire.temporal.timed/v1\";\n\
         model M = \"acme/orders\";\n\
         protocol Flow using Complete over (input: M::Widget) on origin {{\n\
           role R on M::Widget;\n\
           channel C from R to R carries Boolean ordering unordered delivery at-least-once \
             capacity 2 overflow block;\n\
           run sequence Main {{\n\
             attempt Start by R on M::Widget::touch contracts [] as (started: Boolean) {{ true }};\n\
             effect Applied of Start as (applied: Boolean) {{ true }};\n\
             parallel Both {{\n\
               branch s sequence S {{ send Tx via C as (tx: Boolean) {{ true }}; }}\n\
               branch r sequence Rc {{ receive Rx via C of s::S::Tx as (rx: Boolean) {{ true }}; }}\n\
             }} join all [s, r];\n\
             await Wait after Applied using e clock \"wall\" within [2,5] \
               match event Seen by R as (seen: Boolean) {{ true }}; \
               then sequence Then {{ }} timeout sequence Late {{ }}\n\
             repeat Again by R visible () max 2 while {{ true }} \
               sequence Body {{ event Tick by R as (tick: Boolean) {{ true }}; }} \
               exhausted sequence Spent {{ }}\n\
             choice Pick by R visible () {{\n\
               case yes when {{ true }} sequence Y {{ }}\n\
               case no when {{ false }} sequence N {{ }}\n\
             }}\n\
             check Ok using v {{ true }};\n\
             commit Done by R as (done: Boolean) {{ true }};\n\
           }}\n\
           finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    );
    let checked = spine_check_over(&unit, &widget_with_touch())
        .unwrap_or_else(|refusal| panic!("the protocol checks on the spine: {refusal}"));
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
            (ProtocolNodeKind::Attempt, "Start"),
            (ProtocolNodeKind::Effect, "Applied"),
            (ProtocolNodeKind::Parallel, "Both"),
            (ProtocolNodeKind::Branch, "s"),
            (ProtocolNodeKind::Sequence, "S"),
            (ProtocolNodeKind::Send, "Tx"),
            (ProtocolNodeKind::Branch, "r"),
            (ProtocolNodeKind::Sequence, "Rc"),
            (ProtocolNodeKind::Receive, "Rx"),
            (ProtocolNodeKind::Await, "Wait"),
            (ProtocolNodeKind::Event, "Seen"),
            (ProtocolNodeKind::Sequence, "Then"),
            (ProtocolNodeKind::Sequence, "Late"),
            (ProtocolNodeKind::Repeat, "Again"),
            (ProtocolNodeKind::Sequence, "Body"),
            (ProtocolNodeKind::Event, "Tick"),
            (ProtocolNodeKind::Sequence, "Spent"),
            (ProtocolNodeKind::Choice, "Pick"),
            (ProtocolNodeKind::Case, "yes"),
            (ProtocolNodeKind::Sequence, "Y"),
            (ProtocolNodeKind::Case, "no"),
            (ProtocolNodeKind::Sequence, "N"),
            (ProtocolNodeKind::Check, "Ok"),
            (ProtocolNodeKind::Commit, "Done"),
        ]
    );

    let children = |name: &str| -> Vec<&str> {
        let control = protocol
            .controls
            .controls
            .iter()
            .find(|control| control.name == name)
            .unwrap_or_else(|| panic!("control {name}"));
        control
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
            .collect()
    };
    assert_eq!(
        children("Main"),
        ["Start", "Applied", "Both", "Wait", "Again", "Pick", "Ok", "Done"],
        "children keep authored order"
    );
    assert_eq!(children("Both"), ["s", "r"]);
    assert_eq!(children("Wait"), ["Seen", "Then", "Late"]);
    assert_eq!(children("Again"), ["Body", "Spent"]);
    assert_eq!(children("Pick"), ["yes", "no"]);

    let [parallel] = protocol.parallels.as_slice() else {
        panic!("one checked parallel join: {:?}", protocol.parallels);
    };
    assert_eq!(parallel.joined, ["s", "r"]);

    let [await_] = protocol.awaits.as_slice() else {
        panic!("one checked await: {:?}", protocol.awaits);
    };
    assert_eq!(await_.clock, "wall");

    // The send, receive, attempt and effect keep four separate identities.
    let identity = |name: &str| {
        protocol
            .controls
            .controls
            .iter()
            .find(|control| control.name == name)
            .map(|control| control.node)
            .unwrap_or_else(|| panic!("control {name}"))
    };
    let identities = [
        identity("Tx"),
        identity("Rx"),
        identity("Start"),
        identity("Applied"),
    ];
    for (index, first) in identities.iter().enumerate() {
        for second in &identities[index + 1..] {
            assert_ne!(first, second, "node identities are distinct");
        }
    }
}
