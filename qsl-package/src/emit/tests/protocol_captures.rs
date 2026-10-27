// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1021: the captures and the `each` guard of a protocol clause (QSpec
//! FR-464 "Clause body", "Binders", "Level"; FR-383 "Outputs"). Every
//! emitted package is read back through IR's v2 reader, which checks the
//! clause layout, the `captures` records and the dependency set.

use super::protocol_graph::{checked, clause, model, occurrences, reads, target, PROFILES};
use super::protocol_reference::{clause_argument, node_by_id};
use super::reachability::{lower_with, Lowered};
use super::*;
use qsl_semantics::check::{CheckCause, ObligationCause, ObligationStage, ProtocolAnchorCause};

/// A record the trigger and the captures read.
const TICK: &str = "record Tick { ready: Boolean; n: Integer; }\n";

/// A protocol `Flow` over `over_type` with `activation`, `captures`, one
/// static role, an empty `run` and a `finish` whose constraint is `finish`.
fn flow(over_type: &str, activation: &str, captures: &str, finish: &str) -> String {
    format!(
        "{TICK}protocol Flow using v over (input: {over_type}) {activation} {{\n\
         {captures}\n\
         role Buyer on M::Order;\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ {finish} }};\n\
         }}\n"
    )
}

/// `text` emitted over [`model`] and read back by IR; the reader must admit
/// it.
fn admitted(text: &str) -> Lowered {
    let lowered = lower_with(model(), PROFILES, text);
    assert!(lowered.ir.is_ok(), "{:?}\n{text}", lowered.ir.err());
    assert!(lowered.verified, "{text}");
    lowered
}

/// The `parameter` node `reference` names, as its `(name, level)`.
fn parameter(wire: &Value, reference: &Value) -> (String, String) {
    let node = target(wire, reference);
    assert_eq!(node["semantic_form"], "parameter", "{node}");
    let members = &node["body"]["members"];
    (
        members[0]["value"]["value"].as_str().unwrap().to_owned(),
        members[1]["value"]["value"].as_str().unwrap().to_owned(),
    )
}

/// The `(binder, initializer)` terms of each capture record of the clause.
fn captures(wire: &Value) -> Vec<(&Value, &Value)> {
    let captures = clause_argument(clause(wire), "captures");
    assert_eq!(captures["term"], "aggregate", "{captures}");
    captures["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| {
            assert_eq!(record["term"], "aggregate", "{record}");
            let names: Vec<&str> = record["members"]
                .as_array()
                .unwrap()
                .iter()
                .map(|member| member["name"].as_str().unwrap())
                .collect();
            assert_eq!(names, ["binder", "initializer"]);
            (
                &record["members"][0]["value"],
                &record["members"][1]["value"],
            )
        })
        .collect()
}

/// The `(name, level)` of every capture binder of the clause, in order.
fn binders(wire: &Value) -> Vec<(String, String)> {
    captures(wire)
        .into_iter()
        .map(|(binder, _)| parameter(wire, binder))
        .collect()
}

fn pair(name: &str, level: &str) -> (String, String) {
    (name.to_owned(), level.to_owned())
}

/// The level of the finish binder.
fn finish_level(wire: &Value) -> String {
    let finish = target(wire, clause_argument(clause(wire), "finish"));
    parameter(wire, &finish["body"]["arguments"][0]["value"]).1
}

/// The clause key.
fn key(lowered: &Lowered) -> String {
    clause(&lowered.wire)["node_id"]["digest"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// FR-464 "Binders", "Level", FR-383: under `on origin` the captures are
/// parameters at the levels after `over` (1, 2, ...), the `captures`
/// argument lists their `binder`/`initializer` records in source order, an
/// initializer is read over `over` and the captures before it alone, and the
/// finish binder follows the last capture.
#[test]
fn origin_captures_emit_binders_in_slot_order_and_count_in_the_finish_level() {
    let lowered = admitted(&flow(
        "M::Order",
        "on origin",
        "capture a: Option<Reference<M::Order>> = deref(input).trail;\ncapture b: Boolean = present(a);",
        "true",
    ));
    let wire = &lowered.wire;
    assert_eq!(binders(wire), [pair("a", "1"), pair("b", "2")]);
    assert_eq!(finish_level(wire), "3");
    let over = clause_argument(clause(wire), "over");
    let [(a, init_a), (b, init_b)] = captures(wire)[..] else {
        panic!("two captures");
    };
    let (a, b) = (target(wire, a), target(wire, b));
    assert_eq!(
        node_by_id(wire, &a["semantic_type"])["semantic_form"],
        "option"
    );
    // `a` reads `over` alone; `b` reads `a` alone.
    let (init_a, init_b) = (target(wire, init_a), target(wire, init_b));
    assert!(reads(wire, init_a, &over["target"]));
    assert!(!reads(wire, init_a, &b["node_id"]));
    assert!(reads(wire, init_b, &a["node_id"]));
    assert!(!reads(wire, init_b, &over["target"]));
    // The clause depends on every binder and initializer.
    let dependencies = clause(wire)["dependencies"].as_array().unwrap();
    for node in [a, b, init_a, init_b] {
        assert!(dependencies.contains(&node["node_id"]));
    }
    // The binder is placed at the capture's name and its initializer at the
    // expression written.
    // `a` is placed at its name and at the read in `b`'s initializer.
    assert_eq!(
        occurrences(&lowered, a),
        [("expression", "a"), ("expression", "a")]
    );
    assert_eq!(
        occurrences(&lowered, init_a),
        [("expression", "deref(input).trail")]
    );
    assert_eq!(
        occurrences(&lowered, init_b),
        [("expression", "present(a)")]
    );
}

/// FR-464 "Level": under `on each` the first capture is level 2 (the count
/// of `over` and the trigger), a capture initializer reads `over`, the
/// trigger and the earlier captures, and the finish binder, a replicated
/// role's binder and a FIFO message binder follow the captures.
#[test]
fn each_captures_start_after_the_trigger_and_the_readable_binders_follow_them() {
    let text = format!(
        "{TICK}protocol Flow using v over (input: M::Order) on each (t: Tick) {{\n\
         capture a: Option<Reference<M::Order>> = deref(input).trail;\n\
         capture b: Boolean = t.ready;\n\
         role Buyer on M::Order;\n\
         role Agent each M::Person from people max 3 lifetime until(Agent = Agent);\n\
         channel Pay from Buyer to Agent carries Boolean ordering fifo by (m: Boolean) {{ m }} \
         delivery unknown capacity 2 overflow block;\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ b }};\n\
         }}\n"
    );
    let lowered = admitted(&text);
    let wire = &lowered.wire;
    assert_eq!(binders(wire), [pair("a", "2"), pair("b", "3")]);
    let clause = clause(wire);
    let activation = clause_argument(clause, "activation");
    assert_eq!(
        parameter(wire, member(activation, "trigger")),
        pair("t", "1")
    );
    assert_eq!(
        parameter(wire, clause_argument(clause, "over")),
        pair("input", "0")
    );
    assert_eq!(finish_level(wire), "4");

    // `a` reads `over` and the trigger; `b` reads the trigger alone.
    let [(_, init_a), (b, init_b)] = captures(wire)[..] else {
        panic!("two captures");
    };
    let trigger = &member(activation, "trigger")["target"];
    let over = &clause_argument(clause, "over")["target"];
    let (init_a, init_b) = (target(wire, init_a), target(wire, init_b));
    assert!(reads(wire, init_a, over) && !reads(wire, init_a, trigger));
    assert!(reads(wire, init_b, trigger) && !reads(wire, init_b, over));

    // The finish constraint reads the capture `b`.
    let finish = target(wire, clause_argument(clause, "finish"));
    assert!(reads(wire, finish, &b["target"]));

    // The role binder and the message binder are the level after the
    // captures: 4.
    let roles = clause_argument(clause, "roles")["members"]
        .as_array()
        .unwrap();
    let agent = roles
        .iter()
        .map(|role| target(wire, role))
        .find(|role| role["declaration"]["qualified_name"][2] == "Agent")
        .unwrap();
    let replication = &agent["body"]["arguments"][1]["value"];
    assert_eq!(
        parameter(wire, member(replication, "binder")),
        pair("Agent", "4")
    );
    let channels = clause_argument(clause, "channels")["members"]
        .as_array()
        .unwrap();
    let [pay] = channels.as_slice() else {
        panic!("one channel");
    };
    let ordering = &target(wire, pay)["body"]["arguments"][3]["value"];
    assert_eq!(parameter(wire, member(ordering, "binder")), pair("m", "4"));
}

fn member<'g>(group: &'g Value, name: &str) -> &'g Value {
    super::protocol_graph::member(group, name)
}

/// The captures in source order, with no sorting: independent captures
/// written in either order come back in that order and key two clauses.
#[test]
fn the_captures_follow_the_source_order() {
    let lowered = |first: &str, second: &str| {
        admitted(&flow(
            "M::Order",
            "on origin",
            &format!("capture {first}\ncapture {second}"),
            "true",
        ))
    };
    let (x, y) = ("x: Integer = 1;", "y: Integer = 2;");
    let forward = lowered(x, y);
    let backward = lowered(y, x);
    assert_eq!(binders(&forward.wire), [pair("x", "1"), pair("y", "2")]);
    assert_eq!(binders(&backward.wire), [pair("y", "1"), pair("x", "2")]);
    assert_ne!(key(&forward), key(&backward));
}

/// The activation Group is `kind`, `trigger` and, for a guard, `guard`: a
/// guard is a Boolean term read over the trigger alone, an absent guard and
/// a written `true` are different clauses, and a literal guard is a Boolean
/// literal.
#[test]
fn an_each_guard_emits_a_boolean_term_over_the_trigger() {
    let lowered = |guard: &str| {
        admitted(&flow(
            "M::Order",
            &format!("on each (t: Tick){guard}"),
            "",
            "true",
        ))
    };
    let absent = lowered("");
    let read = lowered(" when (t.ready)");
    let negated = lowered(" when (not t.ready)");
    let literal = lowered(" when (true)");

    let names = |lowered: &Lowered| -> Vec<String> {
        clause_argument(clause(&lowered.wire), "activation")["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|member| member["name"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(names(&absent), ["kind", "trigger"].map(str::to_owned));
    for guarded in [&read, &negated, &literal] {
        assert_eq!(
            names(guarded),
            ["kind", "trigger", "guard"].map(str::to_owned)
        );
    }

    let wire = &read.wire;
    let activation = clause_argument(clause(wire), "activation");
    let guard = target(wire, member(activation, "guard"));
    let trigger = &member(activation, "trigger")["target"];
    let over = &clause_argument(clause(wire), "over")["target"];
    assert_eq!(
        node_by_id(wire, &guard["semantic_type"])["semantic_form"],
        "boolean"
    );
    assert!(reads(wire, guard, trigger));
    assert!(!reads(wire, guard, over));
    assert_eq!(occurrences(&read, guard), [("expression", "t.ready")]);
    assert!(clause(wire)["dependencies"]
        .as_array()
        .unwrap()
        .contains(&guard["node_id"]));

    let activation = clause_argument(clause(&literal.wire), "activation");
    let guard = target(&literal.wire, member(activation, "guard"));
    assert_eq!(guard["body"]["term"], "literal");
    assert_eq!(guard["body"]["value"], true);

    let keys = [&absent, &read, &negated, &literal].map(key);
    let distinct: BTreeSet<&String> = keys.iter().collect();
    assert_eq!(distinct.len(), keys.len(), "each guard keys its own clause");
}

/// Captures and a guard together, and two protocols that capture alike: the
/// reader admits both, the binders being one node.
#[test]
fn a_guard_with_captures_and_two_protocols_with_the_same_captures_are_admitted() {
    let one = flow(
        "M::Order",
        "on each (t: Tick) when (t.ready)",
        "capture a: Integer = t.n;",
        "true",
    );
    let lowered = admitted(&one);
    assert_eq!(binders(&lowered.wire), [pair("a", "2")]);
    let other = one.replace("Flow", "Other");
    let both = format!("{one}{}", other.replacen(TICK, "", 1));
    let lowered = admitted(&both);
    // The equal binder is one `parameter` node, placed at both captures
    // (and at both reads of `t.n`'s initializer).
    let binders: Vec<&Value> = nodes(&lowered.wire)
        .iter()
        .filter(|node| node["semantic_form"] == "parameter")
        .filter(|node| node["body"]["members"][0]["value"]["value"] == "a")
        .collect();
    let [binder] = binders[..] else {
        panic!("one parameter node `a`: {binders:?}");
    };
    let names = occurrences(&lowered, binder);
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(names
        .iter()
        .all(|(role, text)| *role == "expression" && *text == "a"));
    let clauses = nodes(&lowered.wire)
        .iter()
        .filter(|node| node["semantic_form"] == "protocol_clause")
        .count();
    assert_eq!(clauses, 2);
}

/// The `acme/orders` document whose `Order` also carries the scalar fields
/// `uid: UUID`, `at: Timestamp` and the optional
/// `flag: Boolean`: the families QSL has no literal for are read through
/// `over`.
fn typed_orders_document() -> Vec<u8> {
    let mut document: Value =
        serde_json::from_slice(&super::reachability::reachability_document()).unwrap();
    let trail = document["types"][0]["fields"][0].clone();
    let field = |name: &str, scalar: &str, presence: &str| {
        let mut field = trail.clone();
        field["identity"] = json!(format!("ix://acme/orders/Order/{name}"));
        field["name"] = json!(name);
        field["typeRef"] = json!(format!("ix://quire/native/{scalar}"));
        field["presence"] = json!(presence);
        field
    };
    let fields = document["types"][0]["fields"].as_array_mut().unwrap();
    fields.push(field("uid", "UUID", "required"));
    fields.push(field("at", "Timestamp", "required"));
    fields.push(field("flag", "Boolean", "optional"));
    serde_json::to_vec(&document).unwrap()
}

/// FR-465 "Role" / FR-464 `parameter`: a capture's declared type is the
/// binder's type; every family the reader admits as a parameter's type is
/// emitted and read back. The clause's `over` is the owner-model reference
/// (QSL-1147), so each type is initialized by a literal where QSL has one,
/// and by a field of `over` otherwise. `Decimal[...]` `Text[...]` and `Float64`
/// have neither and are asserted refused in the test below.
#[test]
fn a_capture_of_every_declared_type_is_admitted() {
    let declarations = format!("{}{TICK}", super::protocol_graph::CARRIED);
    let cases = [
        ("Integer", "1"),
        ("Boolean", "true"),
        ("String", "\"text\""),
        ("UUID", "deref(input).uid"),
        ("Timestamp", "deref(input).at"),
        ("Option<Boolean>", "deref(input).flag"),
        ("Sequence<Boolean>", "sequence[true]"),
        ("Set<Boolean>", "set[true]"),
        ("Bag<Boolean>", "bag[true]"),
        ("OrderedSet<Boolean>", "orderedSet[true]"),
        ("Rational[0, 10; 1, 1]", "rational(1, 1)"),
        ("Point", "Point { x: 1 }"),
        ("Color", "Color::RED"),
        ("Tick", "Tick { ready: true, n: 1 }"),
        ("M::Order", "input"),
        ("Quantity<m>", "1 m"),
        ("Int[0, 5]", "1"),
        ("Sequence<Boolean>[0, 3]", "sequence[true]"),
        ("Option<Reference<M::Order>>", "deref(input).trail"),
    ];
    for (declared, initial) in cases {
        let text = capture_of(&declarations, declared, initial);
        let lowered = lower_with(typed_orders_document(), PROFILES, &text);
        assert!(lowered.ir.is_ok(), "{declared}: {:?}", lowered.ir.err());
        assert!(lowered.verified, "{declared}");
        assert_eq!(binders(&lowered.wire), [pair("c", "1")], "{declared}");
    }
}

/// A protocol over `M::Order` with one capture `c: declared = initial`.
fn capture_of(declarations: &str, declared: &str, initial: &str) -> String {
    format!(
        "{declarations}protocol Flow using v over (input: M::Order) on origin {{\n\
         capture c: {declared} = {initial};\n\
         role Buyer on M::Order;\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    )
}

/// The capture types with no initializer QSL can write, named rather than
/// dropped: `Decimal[...]` (S2 builds no `decimal(...)` value), `Text[...]` (a
/// string literal is a `String`, refused at S3 as a type mismatch) and
/// `Float64` (a float literal needs a floating-point profile the fixture
/// does not select).
#[test]
fn the_capture_types_without_an_initializer_refuse_where_written() {
    let declarations = format!("{}{TICK}", super::protocol_graph::CARRIED);
    let decimal = capture_of(
        &declarations,
        "Decimal[0, 10; 0, 2; nearest-even]",
        "decimal(100, 2)",
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine-model", "fixture", "fixture:1"),
        "program.native",
        format!("language \"ix:native\" edition \"1-draft\";\n{decimal}").as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert!(
        qsl_forms::build_unit(&parsed).is_err(),
        "decimal(...) has no S2 value"
    );
    let text = capture_of(&declarations, "Text[0, 5; nfc]", "\"abc\"");
    assert!(
        checked(&text, false).is_err(),
        "a string literal is no Text"
    );
}

/// FR-381 / FR-464 "Slots that bar model state": a guard that is not Boolean
/// refuses at S3 as a non-Boolean root, at the guard, and a guard that reads
/// `over` refuses as a model-state read; neither reaches emission.
#[test]
fn a_non_boolean_guard_and_a_guard_reading_over_refuse_at_the_check() {
    let text = |guard: &str| {
        flow(
            "M::Order",
            &format!("on each (t: Tick) when ({guard})"),
            "",
            "true",
        )
    };
    let refusal = |guard: &str| -> ObligationCause {
        let text = text(guard);
        let mut causes: Vec<CheckCause> = checked(&text, false)
            .expect_err(guard)
            .into_iter()
            .map(|refusal| refusal.cause)
            .collect();
        let last = causes.pop();
        let (Some(CheckCause::ProtocolObligation(cause)), None) = (last, causes.pop()) else {
            panic!("one obligation refusal for {guard}");
        };
        *cause
    };
    let ObligationCause::Clause { stage, cause, .. } = refusal("t.n") else {
        panic!("a clause refusal");
    };
    assert_eq!(stage, ObligationStage::ProtocolActivation);
    assert_eq!(*cause, CheckCause::NonBooleanRoot);
    assert!(matches!(
        refusal("present(deref(input).trail)"),
        ObligationCause::ModelStateRead {
            stage: ObligationStage::ProtocolActivation,
            ..
        }
    ));
}

/// FR-464 "Level": a FIFO channel's message binder is introduced at the
/// binders readable at the clause, so a binder named like `over`, the
/// `each` trigger or a protocol capture refuses
/// `ambiguous_declaration`/`ambiguous-name`, naming both binders, while a
/// distinct name, the same name on another channel and a name that only a
/// later control binder holds are admitted.
#[test]
fn a_fifo_binder_named_like_a_readable_binder_refuses() {
    let text = |activation: &str, captures: &str, binder: &str| {
        flow_with_channel(
            activation,
            captures,
            &format!("fifo by ({binder}: Boolean) {{ {binder} }}"),
        )
    };
    let causes = |text: &str| -> Vec<CheckCause> {
        checked(text, false)
            .expect_err(text)
            .into_iter()
            .map(|refusal| refusal.cause)
            .collect()
    };
    let each = "on each (t: Tick)";
    let captured = "capture k: Integer = 1;";
    for (activation, captures, binder) in [
        ("on origin", "", "input"),
        (each, "", "t"),
        ("on origin", captured, "k"),
        (each, captured, "input"),
        (each, captured, "t"),
        (each, captured, "k"),
    ] {
        let source = text(activation, captures, binder);
        let causes = causes(&source);
        let [CheckCause::ProtocolAnchor(cause)] = causes.as_slice() else {
            panic!("one anchor refusal for {binder}: {causes:?}\n{source}");
        };
        let ProtocolAnchorCause::Ambiguous { name, loci, .. } = cause.as_ref() else {
            panic!("an ambiguous refusal for {binder}: {cause:?}");
        };
        assert_eq!(name, binder);
        let [earlier, later] = loci[..] else {
            panic!("two loci: {loci:?}");
        };
        assert!(earlier.start < later.start);
        assert_eq!(causes[0].code().as_str(), "ambiguous_declaration");
        assert_eq!(causes[0].cause(), Some("ambiguous-name"));
    }
    // A distinct name is admitted, and so is one that only a record binder
    // of the `run` tree or a template could hold.
    for (activation, captures, binder) in [
        ("on origin", "", "m"),
        (each, captured, "m"),
        (each, captured, "outcome"),
    ] {
        admitted(&text(activation, captures, binder));
    }
}

/// A protocol `Flow` over [`TICK`] with `activation`, `captures`, two roles
/// and the one channel `ordering`.
fn flow_with_channel(activation: &str, captures: &str, ordering: &str) -> String {
    format!(
        "{TICK}protocol Flow using v over (input: M::Order) {activation} {{\n\
         {captures}\n\
         role Buyer on M::Order;\n\
         role Seller on M::Person;\n\
         channel Pay from Buyer to Seller carries Boolean ordering {ordering} \
         delivery unknown capacity 2 overflow block;\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    )
}

/// FR-464: a capture initializer may read model state, through `over`; the
/// reader admits it and the initializer depends on `over` alone.
#[test]
fn a_capture_reading_model_state_through_over_is_admitted() {
    let lowered = admitted(
        "protocol Flow using v over (order: M::Order) on origin {\n\
         capture trail: Option<Reference<M::Order>> = deref(order).trail;\n\
         role Buyer on M::Order;\n\
         run sequence Main { }\n\
         finish End as (outcome: Boolean) { true };\n\
         }\n",
    );
    let wire = &lowered.wire;
    let [(_, initializer)] = captures(wire)[..] else {
        panic!("one capture");
    };
    let over = &clause_argument(clause(wire), "over")["target"];
    assert!(reads(wire, target(wire, initializer), over));
}
