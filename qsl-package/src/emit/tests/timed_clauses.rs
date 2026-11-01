// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230-AC-50, FR-230-AC-54, QSpec FR-322 "Clause nodes"
//! (FR-322-AC-261, FR-322-AC-263, FR-322-AC-265), FR-342-AC-11: a dense
//! model's guards, resets, time invariants and `Clock` parameters reach S4
//! as `clock_guard`, `clock_reset`, `time_invariant` and `clock_constraint`
//! nodes, each located at its source and listed in declared order, and the
//! members S4 does not write yet refuse `not-yet-implemented` at the member.

use super::clock_places::{clocked_document_with, clocked_unit, member, vector_document};
use super::*;

const CLOCK: &str = "ix://quire/native/Clock";

/// ConfigVersion with the clock `x`, `attemptUpdate`'s frame naming `x`
/// too, and, when `parameter` holds, `attemptUpdate(c: Clock)`.
fn document(parameter: bool) -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&clocked_document_with(false)).expect("JSON");
    let operation = &mut document["types"][1]["operations"][0];
    operation["frame"]["modifies"]
        .as_array_mut()
        .expect("modifies")
        .push(json!("ix://example/config-version/ConfigVersion/x"));
    if parameter {
        let identity = "ix://example/config-version/ConfigVersion/attemptUpdate/c";
        operation["params"] = json!([{
            "identity": identity, "name": "c", "typeRef": CLOCK, "presence": "required",
            "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false},
            "origin": {"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
                "inputIdentities": [identity]}},
        }]);
    }
    serde_json::to_vec(&document).expect("JSON")
}

/// The checked unit over `document` with `body`, and the unit's text.
fn checked(
    document: &[u8],
    body: &str,
) -> (
    String,
    Result<qsl_semantics::check::CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>>,
) {
    let packages = qsl_semantics::model::intake::package_input([document]).unwrap();
    let unit = clocked_unit(body);
    let checked = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default());
    (unit, checked)
}

/// The emitted wire of the unit over `document` with `body`, and the
/// unit's text.
fn emitted(document: &[u8], body: &str) -> (String, Value) {
    let (unit, checked) = checked(document, body);
    let graph = checked.unwrap_or_else(|refusals| panic!("{body}: the unit checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    (unit, wire(&emission))
}

/// The node of `wire` that `reference` names.
fn target<'w>(wire: &'w Value, reference: &Value) -> &'w Value {
    let key = &reference["target"];
    nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == key)
        .unwrap_or_else(|| panic!("{reference} names a node"))
}

/// Every node of `wire` of the timed form `form`.
fn timed<'w>(wire: &'w Value, form: &str) -> Vec<&'w Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["node_tag"] == "timed_model" && node["semantic_form"] == form)
        .collect()
}

/// The text a node's first source-map region covers in `unit`.
fn located<'u>(wire: &Value, node: &Value, unit: &'u str) -> &'u str {
    let entry = wire["source_map"]
        .as_array()
        .expect("source map")
        .iter()
        .find(|entry| entry["node_id"] == node["node_id"])
        .unwrap_or_else(|| panic!("{} is located", node["semantic_form"]));
    let region = &entry["regions"][0];
    let at = |name: &str| region[name].as_u64().expect("a position") as usize;
    &unit[at("start")..at("end")]
}

/// A literal member's value, as text.
fn literal(node: &Value, name: &str) -> String {
    match &member(node, name)["value"] {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// The names of a node's body members, in order.
fn names(body: &Value) -> Vec<&str> {
    body["members"]
        .as_array()
        .expect("an aggregate")
        .iter()
        .map(|member| member["name"].as_str().unwrap_or("-"))
        .collect()
}

/// The `true` predicate: an `expression`/`reference` node over the
/// `value`/`literal` node `true`.
fn assert_true(wire: &Value, node: &Value) {
    assert_eq!(node["node_tag"], "expression");
    assert_eq!(node["semantic_form"], "reference");
    let literal = target(wire, &node["body"]);
    assert_eq!(
        (
            &literal["node_tag"],
            &literal["semantic_form"],
            &literal["body"]["value"]
        ),
        (&json!("value"), &json!("literal"), &json!(true))
    );
}

/// `self`: a level-0 `parameter` node.
fn assert_self(wire: &Value, reference: &Value) {
    let node = target(wire, reference);
    assert_eq!(node["semantic_form"], "parameter");
    assert_eq!(literal(&node["body"], "name"), "self");
    assert_eq!(literal(&node["body"], "level"), "0");
}

/// The emitted package's timed aggregates are in QSpec FR-322's declared
/// order.
fn assert_declared_order(wire: &Value) {
    let (nodes, source_map) = admitted_graph(wire);
    assert_eq!(
        crate::emit::timed_order::first_out_of_order(&nodes, &source_map),
        Ok(None)
    );
}

/// The typed nodes and source map the last emission on this thread built.
fn admitted_graph(
    _wire: &Value,
) -> (
    Vec<quire_contract_model::CheckedSemanticNodeV2>,
    Vec<quire_contract_model::CheckedSourceMapEntry>,
) {
    crate::emit::TIMED_GRAPH
        .with(|held| held.borrow().clone())
        .expect("the emission built a timed graph")
}

/// FR-230-AC-54, QSpec FR-322-AC-261, FR-322-AC-263 and
/// FR-342-AC-11: over `x: Clock`, the precondition `self.x >= 1 ms and
/// self.versionNumber > 0 or self.x < 5 ms` is one `clock_guard` of
/// `attemptUpdate`'s no-point `operation_anchor`, with two parts: the first
/// the expression node of `self.versionNumber > 0` and the constraint `ge
/// 1/1`, the second `true` and `lt 5/1`. The postcondition `self.x = 0 ms`
/// is one `clock_reset` of kind `place`, owner `self`, no slot or binder and
/// the condition `true`. `time invariant Bounded when {
/// self.versionNumber >= 0 } { self.x <= 9 ms }` is one `time_invariant`
/// with the bound `le 9/1`, and `urgent Always when { true }` names the `true` expression. A
/// place is exactly `model`, `path`, `resolved_path`. The root lists every
/// clause and every constraint once, in declared order, and each clause
/// node is located at its own source.
#[trace("TC-685", "FR-230-AC-50", "FR-230-AC-54")]
#[trace(
    "QSpec-FR-322-AC-261",
    "QSpec-FR-322-AC-263",
    "QSpec-FR-322-AC-265",
    "QSpec-FR-342-AC-11"
)]
#[test]
fn guards_resets_and_invariants_emit_clause_nodes() {
    let (unit, wire) = emitted(
        &document(false),
        "pre Late using v on Config::ConfigVersion::attemptUpdate { \
         self.x >= 1 ms and self.versionNumber > 0 or self.x < 5 ms }\n\
         post Reset using v on Config::ConfigVersion::attemptUpdate { self.x = 0 ms }\n\
         time invariant Bounded when { self.versionNumber >= 0 } { self.x <= 9 ms }\n\
         urgent Always when { true };\n",
    );
    let [place] = timed(&wire, "clock_place")[..] else {
        panic!("one place");
    };
    assert_eq!(names(&place["body"]), ["model", "path", "resolved_path"]);

    let [guard] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    assert_eq!(names(&guard["body"]), ["operation", "parts"]);
    assert!(located(&wire, guard, &unit).starts_with("pre Late"));
    let anchor = target(&wire, member(&guard["body"], "operation"));
    assert_eq!(anchor["semantic_form"], "operation_anchor");
    assert_eq!(literal(&anchor["body"], "operation"), "attemptUpdate");
    assert_eq!(names(&anchor["body"]), ["context", "operation", "frame"]);
    let parts = member(&guard["body"], "parts")["members"]
        .as_array()
        .expect("parts");
    let [first, second] = parts.as_slice() else {
        panic!("two parts: {parts:?}");
    };
    let part = |part: &Value| -> (Value, Vec<Value>) {
        let members = part["members"].as_array().expect("a part");
        assert_eq!(members[0]["name"], "predicate");
        (
            target(&wire, &members[0]["value"]).clone(),
            members[1..]
                .iter()
                .map(|reference| target(&wire, reference).clone())
                .collect(),
        )
    };
    let (data, constraints) = part(first);
    assert_eq!(data["node_tag"], "expression");
    assert_eq!(data["body"]["term"], "application");
    assert_eq!(located(&wire, &data, &unit), "self.versionNumber > 0");
    let [constraint] = constraints.as_slice() else {
        panic!("one constraint");
    };
    assert_eq!(
        names(&constraint["body"]),
        ["kind", "place", "owner", "slots", "comparison", "bound"]
    );
    assert_eq!(literal(&constraint["body"], "kind"), "place");
    assert_eq!(literal(&constraint["body"], "comparison"), "ge");
    assert_eq!(literal(&constraint["body"], "bound"), "1/1");
    assert_eq!(
        member(&constraint["body"], "place")["target"],
        place["node_id"]
    );
    assert_self(&wire, member(&constraint["body"], "owner"));
    assert_eq!(member(&constraint["body"], "slots")["members"], json!([]));
    assert_eq!(located(&wire, constraint, &unit), "self.x >= 1 ms");
    let (data, constraints) = part(second);
    assert_true(&wire, &data);
    let [constraint] = constraints.as_slice() else {
        panic!("one constraint");
    };
    assert_eq!(literal(&constraint["body"], "comparison"), "lt");
    assert_eq!(literal(&constraint["body"], "bound"), "5/1");

    let [reset] = timed(&wire, "clock_reset")[..] else {
        panic!("one reset");
    };
    assert_eq!(
        names(&reset["body"]),
        [
            "kind",
            "operation",
            "place",
            "owner",
            "slots",
            "binders",
            "condition",
            "value"
        ]
    );
    assert_eq!(
        member(&reset["body"], "operation"),
        member(&guard["body"], "operation"),
        "a guard and a reset name one anchor"
    );
    assert_self(&wire, member(&reset["body"], "owner"));
    assert_eq!(member(&reset["body"], "binders")["members"], json!([]));
    assert_true(&wire, target(&wire, member(&reset["body"], "condition")));
    assert_eq!(literal(&reset["body"], "value"), "0/1");
    assert_eq!(located(&wire, reset, &unit), "self.x = 0 ms");

    let [invariant] = timed(&wire, "time_invariant")[..] else {
        panic!("one invariant");
    };
    assert_eq!(names(&invariant["body"]), ["predicate", "bounds"]);
    let predicate = target(&wire, member(&invariant["body"], "predicate"));
    assert_eq!(located(&wire, predicate, &unit), "self.versionNumber >= 0");
    let [bound] = member(&invariant["body"], "bounds")["members"]
        .as_array()
        .expect("bounds")
        .as_slice()
    else {
        panic!("one bound");
    };
    let bound = target(&wire, bound);
    assert_eq!(literal(&bound["body"], "comparison"), "le");
    assert_eq!(literal(&bound["body"], "bound"), "9/1");
    assert!(located(&wire, invariant, &unit).starts_with("time invariant Bounded"));

    let [urgency] = timed(&wire, "urgency")[..] else {
        panic!("one urgency");
    };
    assert_true(&wire, target(&wire, member(&urgency["body"], "predicate")));

    let [root] = timed(&wire, "timed_model")[..] else {
        panic!("one root");
    };
    let listed = |name: &str| {
        member(&root["body"], name)["members"]
            .as_array()
            .unwrap()
            .len()
    };
    assert_eq!(
        ["resets", "guards", "invariants", "urgencies", "constraints"].map(listed),
        [1, 1, 1, 1, 3]
    );
    assert_eq!(timed(&wire, "clock_constraint").len(), 3);
    assert_declared_order(&wire);
}

/// QSpec FR-322 "Clause nodes": the comparison `eq` is a guard's, and a
/// constraint written twice is one node with an occurrence at each source,
/// listed once by the root.
#[trace("TC-685", "FR-230-AC-54", "QSpec-FR-322-AC-261")]
#[test]
fn an_equal_constraint_is_one_node_located_at_each_source() {
    let (unit, wire) = emitted(
        &document(false),
        "pre Exact using v on Config::ConfigVersion::attemptUpdate { \
         self.x = 2 ms or self.versionNumber > 0 and self.x = 2 ms }\n",
    );
    let [constraint] = timed(&wire, "clock_constraint")[..] else {
        panic!("one constraint");
    };
    assert_eq!(literal(&constraint["body"], "comparison"), "eq");
    let regions: Vec<&str> = wire["source_map"]
        .as_array()
        .expect("source map")
        .iter()
        .filter(|entry| entry["node_id"] == constraint["node_id"])
        .map(|entry| {
            let region = &entry["regions"][0];
            &unit[region["start"].as_u64().unwrap() as usize
                ..region["end"].as_u64().unwrap() as usize]
        })
        .collect();
    assert_eq!(regions, ["self.x = 2 ms", "self.x = 2 ms"]);
    assert_eq!(constraint["occurrences"].as_array().unwrap().len(), 2);
    assert_declared_order(&wire);
}

/// FR-230 "Operation parameters", QSpec FR-322 "Clause nodes" and
/// FR-342-AC-11: over `attemptUpdate(c: Clock)`, `c >= 1 ms` is a
/// `clock_constraint` of kind `parameter` naming the operation's anchor and
/// `c`, and `c = 0 ms` a `clock_reset` of kind `parameter` with no
/// `condition`: the `Clock` parameter is an inline name, no node.
#[trace("TC-685", "FR-230-AC-54", "QSpec-FR-322-AC-261", "QSpec-FR-322-AC-262")]
#[test]
fn a_clock_parameter_is_an_inline_name_of_its_operation() {
    let (_, wire) = emitted(
        &document(true),
        "pre Late using v on Config::ConfigVersion::attemptUpdate { c >= 1 ms }\n\
         post Reset using v on Config::ConfigVersion::attemptUpdate { c = 0 ms }\n",
    );
    let [constraint] = timed(&wire, "clock_constraint")[..] else {
        panic!("one constraint");
    };
    assert_eq!(
        names(&constraint["body"]),
        ["kind", "operation", "parameter", "comparison", "bound"]
    );
    assert_eq!(literal(&constraint["body"], "kind"), "parameter");
    assert_eq!(literal(&constraint["body"], "parameter"), "c");
    let anchor = target(&wire, member(&constraint["body"], "operation"));
    assert_eq!(anchor["semantic_form"], "operation_anchor");
    let [reset] = timed(&wire, "clock_reset")[..] else {
        panic!("one reset");
    };
    assert_eq!(
        names(&reset["body"]),
        ["kind", "operation", "parameter", "value"]
    );
    assert_eq!(literal(&reset["body"], "parameter"), "c");
    assert_eq!(
        member(&reset["body"], "operation"),
        member(&constraint["body"], "operation")
    );
    let [guard] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    assert_eq!(
        member(&guard["body"], "operation"),
        member(&constraint["body"], "operation")
    );
    assert_declared_order(&wire);
}

/// QSpec FR-322 "Clause nodes": a model has one `clock_guard` for each
/// operation anchor, so two guarded preconditions of one operation,
/// `self.x >= 1 ms or self.x > 7 ms` and `self.versionNumber > 0 and
/// self.x < 5 ms`,
/// are the one guard of their conjunction: each part of the one a part of
/// each part of the other, its predicate the `and` of theirs (the data
/// predicate alone where the other's is `true`) and its constraints both
/// lists, in declared order.
#[trace("TC-685", "FR-230-AC-50", "FR-230-AC-54", "QSpec-FR-322-AC-261")]
#[test]
fn two_guarded_preconditions_are_one_guard_of_their_conjunction() {
    let (unit, wire) = emitted(
        &document(false),
        "pre A using v on Config::ConfigVersion::attemptUpdate { \
         self.x >= 1 ms or self.x > 7 ms }\n\
         pre B using v on Config::ConfigVersion::attemptUpdate { \
         self.versionNumber > 0 and self.x < 5 ms }\n",
    );
    let [guard] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    assert!(located(&wire, guard, &unit).starts_with("pre A"));
    let parts = member(&guard["body"], "parts")["members"]
        .as_array()
        .expect("parts");
    let read: Vec<(String, Vec<String>)> = parts
        .iter()
        .map(|part| {
            let members = part["members"].as_array().expect("a part");
            let predicate = target(&wire, &members[0]["value"]);
            (
                located(&wire, predicate, &unit).to_owned(),
                members[1..]
                    .iter()
                    .map(|reference| located(&wire, target(&wire, reference), &unit).to_owned())
                    .collect(),
            )
        })
        .collect();
    let data = "self.versionNumber > 0".to_owned();
    assert_eq!(
        read,
        [
            (
                data.clone(),
                vec!["self.x >= 1 ms".to_owned(), "self.x < 5 ms".to_owned()]
            ),
            (
                data,
                vec!["self.x > 7 ms".to_owned(), "self.x < 5 ms".to_owned()]
            ),
        ]
    );
    let [root] = timed(&wire, "timed_model")[..] else {
        panic!("one root");
    };
    assert_eq!(
        member(&root["body"], "guards")["members"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(timed(&wire, "clock_constraint").len(), 3);
    assert_declared_order(&wire);

    let (unit, wire) = emitted(
        &document(false),
        "pre A using v on Config::ConfigVersion::attemptUpdate { \
         self.versionNumber > 0 and self.x >= 1 ms }\n\
         pre B using v on Config::ConfigVersion::attemptUpdate { \
         self.versionNumber < 9 and self.x < 5 ms }\n",
    );
    let [guard] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    let [part] = member(&guard["body"], "parts")["members"]
        .as_array()
        .expect("parts")
        .as_slice()
    else {
        panic!("one part");
    };
    let predicate = target(&wire, &part["members"][0]["value"]);
    assert_eq!(
        predicate["body"]["operation"]["identity"],
        "quire.op.boolean.and"
    );
    let operands: Vec<&str> = predicate["body"]["arguments"]
        .as_array()
        .expect("arguments")
        .iter()
        .map(|argument| located(&wire, target(&wire, argument), &unit))
        .collect();
    assert_eq!(
        operands,
        ["self.versionNumber > 0", "self.versionNumber < 9"]
    );
    assert_eq!(part["members"].as_array().map(Vec::len), Some(3));
    assert_declared_order(&wire);
}

/// [`document`] with the required field `peer: ConfigVersion`, a
/// reference to another object of the type.
fn peer_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&document(false)).expect("JSON");
    let config = &mut document["types"][1];
    let owner = config["identity"].as_str().expect("an identity").to_owned();
    let identity = format!("{owner}/peer");
    let mut peer = config["fields"][0].clone();
    peer["identity"] = json!(identity);
    peer["name"] = json!("peer");
    peer["typeRef"] = json!(owner);
    peer["presence"] = json!("required");
    peer["multiplicity"] = json!({"lower": 1, "upper": 1, "ordered": false, "unique": true});
    peer["origin"]["generated"]["generatorIdentity"] = json!(identity);
    peer["origin"]["generated"]["inputIdentities"] = json!([identity]);
    config["fields"].as_array_mut().expect("fields").push(peer);
    serde_json::to_vec(&document).expect("JSON")
}

/// FR-230-AC-10, QSpec FR-322 "Clause nodes" and "Presence": a constraint
/// on another object's clock, `peer_clock(self) <= 2 ms` with `peer_clock`
/// reading `o.peer.x`, is a `clock_constraint`
/// of kind `place` on the template of `x` whose owner term is the
/// expression `self.peer`: the `record.project` of `peer` over the
/// `quire.op.model.deref` of `self`, as S3 lowers an attribute read.
#[trace("TC-685", "FR-230-AC-10", "FR-230-AC-54", "QSpec-FR-322-AC-261")]
#[test]
fn a_constraint_on_another_objects_clock_names_its_owner_term() {
    let (unit, wire) = emitted(
        &peer_document(),
        "function peer_clock using v(o: Config::ConfigVersion): Clock pure { o.peer.x }\n\
         pre Near using v on Config::ConfigVersion::attemptUpdate { peer_clock(self) <= 2 ms }\n",
    );
    let [place] = timed(&wire, "clock_place")[..] else {
        panic!("one place");
    };
    let [constraint] = timed(&wire, "clock_constraint")[..] else {
        panic!("one constraint");
    };
    assert_eq!(literal(&constraint["body"], "kind"), "place");
    assert_eq!(
        member(&constraint["body"], "place")["target"],
        place["node_id"]
    );
    assert_eq!(
        located(&wire, constraint, &unit),
        "peer_clock(self) <= 2 ms"
    );
    let owner = target(&wire, member(&constraint["body"], "owner"));
    assert_eq!(owner["node_tag"], "expression");
    assert_eq!(
        owner["body"]["operation"]["identity"],
        "quire.op.record.project"
    );
    assert_eq!(owner["body"]["operation"]["member"]["name"], "peer");
    let [dereferenced] = owner["body"]["arguments"]
        .as_array()
        .expect("arguments")
        .as_slice()
    else {
        panic!("one argument");
    };
    let dereferenced = target(&wire, dereferenced);
    assert_eq!(
        dereferenced["body"]["operation"]["identity"],
        "quire.op.model.deref"
    );
    assert_self(&wire, &dereferenced["body"]["arguments"][0]);
    assert_declared_order(&wire);
}

/// [`peer_document`] with `v: Option<Clock>`.
fn presence_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&peer_document()).expect("JSON");
    let config = &mut document["types"][1];
    let owner = config["identity"].as_str().expect("an identity").to_owned();
    let fields = config["fields"].as_array_mut().expect("fields");
    let mut v = fields
        .iter()
        .find(|field| field["name"] == "x")
        .expect("x")
        .clone();
    let identity = format!("{owner}/v");
    v["identity"] = json!(identity);
    v["name"] = json!("v");
    v["presence"] = json!("optional");
    v["origin"]["generated"]["generatorIdentity"] = json!(identity);
    v["origin"]["generated"]["inputIdentities"] = json!([identity]);
    fields.push(v);
    serde_json::to_vec(&document).expect("JSON")
}

/// `present(p)` of the place `name`: a `presence_read` application of
/// `quire.op.option.present`, typed Boolean, over the place's template and
/// the owner term; returns the owner term's node.
fn assert_presence<'w>(wire: &'w Value, read: &Value, place: &Value) -> &'w Value {
    assert_eq!(
        (&read["node_tag"], &read["semantic_form"]),
        (&json!("expression"), &json!("presence_read"))
    );
    assert_eq!(
        read["body"]["operation"]["identity"],
        "quire.op.option.present"
    );
    assert_eq!(read["body"]["operator"], "present");
    assert_eq!(
        target(wire, &json!({"target": read["body"]["result_type"]}))["semantic_form"],
        "boolean"
    );
    let [template, owner, ..] = read["body"]["arguments"]
        .as_array()
        .expect("arguments")
        .as_slice()
    else {
        panic!("a place and an owner term: {read}");
    };
    assert_eq!(template["target"], place["node_id"]);
    target(wire, owner)
}

/// The `clock_place` of `wire` whose path is the one field `name`.
fn place_of<'w>(wire: &'w Value, name: &str) -> &'w Value {
    timed(wire, "clock_place")
        .into_iter()
        .find(|place| {
            member(&place["body"], "path")["members"][0]["members"][1]["value"]["value"] == name
        })
        .unwrap_or_else(|| panic!("the place {name}"))
}

/// QSpec FR-322 "Presence" (FR-322-AC-155), FR-230 "Presence is derived":
/// `present(self.v)` over `v: Option<Clock>` is a `presence_read` of
/// `quire.op.option.present` over `v`'s `clock_place` template and the
/// owner `self`, in a guard part beside the part's constraint on `v`, as a
/// time invariant's whole predicate and conjoined with an `urgent when`'s
/// data predicate; `present(self.peer.v)` takes the owner term `self.peer`.
/// Each place is S3's template, keyed as IR keys it, and no place or model
/// node carries a presence member.
#[trace("TC-685", "FR-230-AC-50", "FR-230-AC-53", "FR-230-AC-54")]
#[trace("QSpec-FR-322-AC-155", "QSpec-FR-322-AC-261")]
#[test]
fn a_presence_reads_the_place_template_at_its_owner() {
    let (unit, wire) = emitted(
        &presence_document(),
        "pre Armed using v on Config::ConfigVersion::attemptUpdate { \
         present(self.v) and value(self.v) >= 1 ms }\n\
         time invariant Held when { present(self.v) } { value(self.v) <= 9 ms }\n\
         urgent Due when { present(self.v) and self.versionNumber > 0 };\n\
         urgent Peer when { present(self.peer.v) };\n",
    );
    let places = timed(&wire, "clock_place");
    assert_eq!(places.len(), 2);
    let graph = qsl_semantics::check::wire_id::WireGraph::of_package_value(&wire).unwrap();
    for place in &places {
        // The held timed graph is the emitter's, in structural ids.
        let id = crate::emit::checked_node_id_of(&graph.key_of_value(&place["node_id"]).unwrap());
        let node = admitted_graph(&wire)
            .0
            .into_iter()
            .find(|node| node.node_id == id)
            .expect("a node");
        assert_eq!(
            crate::emit::timed_model_node_id(&StructuralIds::default(), &node).expect("a key"),
            node.node_id,
            "S3's place key is the key IR derives"
        );
        assert_eq!(names(&place["body"]), ["model", "path", "resolved_path"]);
    }
    let v = place_of(&wire, "v");

    let [guard] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    let part = &member(&guard["body"], "parts")["members"][0]["members"];
    let read = target(&wire, &part[0]["value"]);
    assert_self(
        &wire,
        &json!({"target": assert_presence(&wire, read, v)["node_id"]}),
    );
    let constraint = target(&wire, &part[1]);
    assert_eq!(member(&constraint["body"], "place")["target"], v["node_id"]);
    assert_eq!(literal(&constraint["body"], "comparison"), "ge");

    let [invariant] = timed(&wire, "time_invariant")[..] else {
        panic!("one invariant");
    };
    assert_eq!(
        member(&invariant["body"], "predicate")["target"],
        read["node_id"],
        "one presence node"
    );

    let urgencies = timed(&wire, "urgency");
    let predicate = |name: &str| {
        let urgency = urgencies
            .iter()
            .find(|urgency| located(&wire, urgency, &unit).starts_with(name))
            .unwrap_or_else(|| panic!("urgency {name}"));
        target(&wire, member(&urgency["body"], "predicate"))
    };
    let due = predicate("urgent Due");
    assert_eq!(due["body"]["operation"]["identity"], "quire.op.boolean.and");
    assert_eq!(due["body"]["arguments"][0]["target"], read["node_id"]);
    assert_eq!(
        located(&wire, target(&wire, &due["body"]["arguments"][1]), &unit),
        "self.versionNumber > 0"
    );
    let owner = assert_presence(&wire, predicate("urgent Peer"), v);
    assert_eq!(
        owner["body"]["operation"]["identity"],
        "quire.op.record.project"
    );
    assert_eq!(owner["body"]["operation"]["member"]["name"], "peer");

    for node in nodes(&wire) {
        if node["node_tag"] == "timed_model" || node["semantic_form"] == "model" {
            assert!(
                !node.to_string().contains("\"presence\""),
                "no node carries a presence member: {node}"
            );
        }
    }
    assert_declared_order(&wire);
}

/// The member a refused model names, and the text of its span in `unit`.
fn unwritten(document: &[u8], body: &str) -> (qsl_semantics::check::UnwrittenTimedMember, String) {
    let (unit, checked) = checked(document, body);
    let refusals = checked.expect_err("the member is not written yet");
    let found = refusals.iter().find_map(|refusal| match &refusal.cause {
        qsl_semantics::check::CheckCause::Temporal(cause) => match &**cause {
            qsl_semantics::check::TemporalCause::TimedModelNotEmitted { member, span, .. } => {
                Some((*member, unit[span.start..span.end].to_owned()))
            }
            _ => None,
        },
        _ => None,
    });
    found.unwrap_or_else(|| panic!("{body}: {refusals:?}"))
}

/// FR-230-AC-50, QSpec FR-322-AC-263 (`QN-01`): the guard `forall(e in
/// self.ts: e.x < 5 ms)` quantified over a clock collection refuses `unsupported_construct`/`not-yet-implemented`
/// at the quantifier, naming STD-554, and is never written unquantified.
/// A guard disjunct that constrains no clock refuses at the precondition.
#[trace("TC-685", "FR-230-AC-50", "QSpec-FR-322-AC-263")]
#[test]
fn an_unwritten_guard_refuses_at_its_member() {
    use qsl_semantics::check::UnwrittenTimedMember as Member;
    let quantifier = "forall(e in self.ts: e.x < 5 ms)";
    let (member, text) = unwritten(
        &vector_document(),
        &format!("pre All using v on Config::ConfigVersion::attemptUpdate {{ {quantifier} }}\n"),
    );
    assert_eq!(
        (member, text.as_str()),
        (Member::QuantifiedClause, quantifier)
    );
    assert!(member.description().contains("STD-554"));
    let cause = qsl_semantics::check::TemporalCause::TimedModelNotEmitted {
        model: "Config".to_owned(),
        member,
        span: qsl_foundation::Span { start: 0, end: 1 },
    };
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::UnsupportedConstruct, "not-yet-implemented")
    );
}

/// FR-230-AC-50, QSpec FR-342-AC-11: a clock-reading clause bound to an
/// execution point refuses `not-yet-implemented` at the clause, naming
/// STD-555, and is never written with a point-qualified anchor.
#[trace("TC-685", "FR-230-AC-50", "QSpec-FR-342-AC-11")]
#[test]
fn a_point_qualified_clock_clause_refuses_at_the_clause() {
    use quire_contract_model::{AnchorName, ExecutionPoint};
    let document = document(false);
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = clocked_unit(
        "pre Late using v on Config::ConfigVersion::attemptUpdate { self.x >= 1 ms }\n",
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine-model", "fixture", "fixture:1"),
        "program.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    let raw = parsed.source().reference().clone();
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("intake admits the package");
    let point = ExecutionPoint::Pre {
        operation: AnchorName::new("attemptUpdate").unwrap().into(),
    };
    let refusals = PackageDeclarations::assemble_with_execution_points_and_cancel(
        raw,
        "pkg".to_owned(),
        built,
        models,
        Vec::new(),
        Default::default(),
        qsl_semantics::check::AssemblyRun {
            cancel: &quire_exact::Cancel::new(),
            execution_points: BTreeMap::from([("Late".to_owned(), point)]),
        },
    )
    .expect("the unit assembles")
    .check(CheckingLimits::default())
    .expect_err("a point-qualified clock clause is not written yet");
    let found = refusals.iter().find_map(|refusal| match &refusal.cause {
        qsl_semantics::check::CheckCause::Temporal(cause) => match &**cause {
            qsl_semantics::check::TemporalCause::TimedModelNotEmitted { member, span, .. } => {
                Some((*member, &unit[span.start..span.end]))
            }
            _ => None,
        },
        _ => None,
    });
    let Some((member, text)) = found else {
        panic!("{refusals:?}");
    };
    assert_eq!(
        member,
        qsl_semantics::check::UnwrittenTimedMember::PointQualifiedClause
    );
    assert!(member.description().contains("STD-555"));
    assert!(text.starts_with("pre Late"), "{text}");
}

/// [`vector_document`] whose `attemptUpdate` frame names `ts`.
fn timer_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&vector_document()).expect("JSON");
    document["types"][1]["operations"][0]["frame"]["modifies"]
        .as_array_mut()
        .expect("modifies")
        .push(json!("ix://example/config-version/ConfigVersion/ts"));
    serde_json::to_vec(&document).expect("JSON")
}

/// FR-230 "Resets by selection", QSpec FR-322 "Clause nodes"
/// (FR-322-AC-261, FR-322-AC-264): over `ts: Sequence<Timer>[0, 3]`,
/// `forall(e in self.ts: e.x = 0 ms)` is one `clock_reset` of the place
/// `ts[slot].x` whose one slot term and one binder are the `parameter` node
/// `e` of the selector domain `Integer`, under the condition `true`. A
/// selection that reads the element's data refuses `not-yet-implemented`
/// at the reset.
#[trace("TC-685", "FR-230-AC-54", "QSpec-FR-322-AC-261", "QSpec-FR-322-AC-264")]
#[test]
fn a_reset_by_selection_binds_its_slot() {
    let (unit, wire) = emitted(
        &timer_document(),
        "post R using v on Config::ConfigVersion::attemptUpdate { \
         forall(e in self.ts: e.x = 0 ms) }\n",
    );
    let [reset] = timed(&wire, "clock_reset")[..] else {
        panic!("one reset");
    };
    let place = target(&wire, member(&reset["body"], "place"));
    let steps: Vec<String> = member(&place["body"], "path")["members"]
        .as_array()
        .expect("a path")
        .iter()
        .map(|step| literal(step, "kind"))
        .collect();
    assert_eq!(steps, ["field", "slot", "field"]);
    let slots = &member(&reset["body"], "slots")["members"];
    let binders = &member(&reset["body"], "binders")["members"];
    assert_eq!(slots, binders, "the binder is the slot term");
    let [binder] = binders.as_array().expect("binders").as_slice() else {
        panic!("one binder");
    };
    let binder = target(&wire, binder);
    assert_eq!(binder["semantic_form"], "parameter");
    assert_eq!(literal(&binder["body"], "name"), "e");
    let integer = qsl_semantics::check::builtin_integer_type_key("pkg").unwrap();
    assert_eq!(
        qsl_foundation::node_wire::node_key_from_json(&binder["semantic_type"]).unwrap(),
        integer
    );
    assert_true(&wire, target(&wire, member(&reset["body"], "condition")));
    assert_eq!(
        located(&wire, reset, &unit),
        "forall(e in self.ts: e.x = 0 ms)"
    );
    assert_declared_order(&wire);
}

/// The excluded form a refused model names, and the text of its span in
/// `unit`.
fn excluded(document: &[u8], body: &str) -> (qsl_semantics::check::ExcludedClauseForm, String) {
    let (unit, checked) = checked(document, body);
    let refusals = checked.expect_err("the form is excluded");
    let found = refusals.iter().find_map(|refusal| match &refusal.cause {
        qsl_semantics::check::CheckCause::Temporal(cause) => match &**cause {
            qsl_semantics::check::TemporalCause::ExcludedClauseForm { form, span, .. } => {
                assert_eq!(
                    (cause.code(), cause.tag()),
                    (Code::UnsupportedConstruct, "expression-form")
                );
                Some((*form, unit[span.start..span.end].to_owned()))
            }
            _ => None,
        },
        _ => None,
    });
    found.unwrap_or_else(|| panic!("{body}: {refusals:?}"))
}

/// QSpec FR-322-AC-267 `CW-06`: an indexed element write `self.ts[0] =
/// Config::Timer { x: 0 ms, n: 1 }` has no package form. S2 represents no
/// indexed element read, so the unit is refused `unsupported_construct` at
/// `self.ts[0]` before S3, and no package is emitted.
#[trace("TC-685", "QSpec-FR-322-AC-267")]
#[test]
fn an_indexed_element_write_is_refused() {
    let unit = clocked_unit(
        "post P using v on Config::ConfigVersion::attemptUpdate { \
         self.ts[0] = Config::Timer { x: 0 ms, n: 1 } }\n",
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine-model", "fixture", "fixture:1"),
        "program.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    let refusal = qsl_forms::build_unit(&parsed).expect_err("S2 has no indexed element read");
    assert_eq!(
        refusal.catalog_code(),
        Code::UnsupportedConstruct,
        "{refusal:?}"
    );
    let span = refusal.span.expect("a span");
    assert_eq!(&unit[span.start..span.end], "self.ts[0]");
}

/// [`dropping_document`] with `q: Sequence<Option<Clock>>[0, 2]`, which
/// `attemptUpdate`'s frame names too.
fn slot_presence_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&dropping_document()).expect("JSON");
    let config = &mut document["types"][1];
    let owner = config["identity"].as_str().expect("an identity").to_owned();
    let fields = config["fields"].as_array_mut().expect("fields");
    let mut q = fields
        .iter()
        .find(|field| field["name"] == "v")
        .expect("v")
        .clone();
    let identity = format!("{owner}/q");
    q["identity"] = json!(identity);
    q["name"] = json!("q");
    q["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
    q["origin"]["generated"]["generatorIdentity"] = json!(identity);
    q["origin"]["generated"]["inputIdentities"] = json!([identity]);
    fields.push(q);
    config["operations"][0]["frame"]["modifies"]
        .as_array_mut()
        .expect("modifies")
        .push(json!(identity));
    serde_json::to_vec(&document).expect("JSON")
}

/// QSpec FR-322-AC-267 `SD-02`: `forall(e in self.q: present(e) implies e
/// = 0 ms)` over `q: Sequence<Option<Clock>>[0, 2]` is one `clock_reset`
/// of `q`'s element place whose one binder `e` is its slot term and whose
/// condition is the `presence_read` of that place at `self` and `e`.
#[trace("TC-685", "FR-230-AC-54", "QSpec-FR-322-AC-267", "QSpec-FR-322-AC-155")]
#[test]
fn a_reset_selected_by_presence_conditions_on_its_presence_read() {
    let (unit, wire) = emitted(
        &slot_presence_document(),
        "post R using v on Config::ConfigVersion::attemptUpdate { \
         forall(e in self.q: present(e) implies e = 0 ms) }\n",
    );
    let [reset] = timed(&wire, "clock_reset")[..] else {
        panic!("one reset");
    };
    let place = target(&wire, member(&reset["body"], "place"));
    assert_eq!(place, place_of(&wire, "q"));
    let binders = &member(&reset["body"], "binders")["members"];
    assert_eq!(binders, &member(&reset["body"], "slots")["members"]);
    let [binder] = binders.as_array().expect("binders").as_slice() else {
        panic!("one binder");
    };
    let condition = target(&wire, member(&reset["body"], "condition"));
    assert_self(
        &wire,
        &json!({"target": assert_presence(&wire, condition, place)["node_id"]}),
    );
    assert_eq!(condition["body"]["arguments"][2], *binder);
    assert_eq!(
        located(&wire, reset, &unit),
        "forall(e in self.q: present(e) implies e = 0 ms)"
    );
    assert_declared_order(&wire);
}

/// QSpec FR-322-AC-267 `CW-05`: `post present(self.v)` is a postcondition
/// predicate, a `presence_read` of `v`'s place at `self`, and not a write:
/// the package carries no clause node for it.
#[trace("TC-685", "FR-230-AC-50", "QSpec-FR-322-AC-267")]
#[test]
fn a_postcondition_presence_requirement_is_a_presence_read_with_no_clause_node() {
    let (_, wire) = emitted(
        &dropping_document(),
        "post P using v on Config::ConfigVersion::attemptUpdate { present(self.v) }\n",
    );
    for form in [
        "clock_reset",
        "clock_guard",
        "clock_constraint",
        "time_invariant",
    ] {
        assert!(timed(&wire, form).is_empty(), "no {form}");
    }
    let v = place_of(&wire, "v");
    let (_, without) = emitted(&dropping_document(), "");
    // Two packages: their reads compare structurally (FR-322).
    let reads = |wire: &Value| -> Vec<(Value, Value)> {
        let graph = qsl_semantics::check::wire_id::WireGraph::of_package_value(wire).unwrap();
        nodes(wire)
            .iter()
            .filter(|node| node["semantic_form"] == "presence_read")
            .map(|node| (graph.structural_value(node).unwrap(), node.clone()))
            .collect()
    };
    let before: Vec<Value> = reads(&without).into_iter().map(|(key, _)| key).collect();
    let added: Vec<Value> = reads(&wire)
        .into_iter()
        .filter(|(key, _)| !before.contains(key))
        .map(|(_, read)| read)
        .collect();
    let [read] = &added[..] else {
        panic!("one presence read: {added:?}");
    };
    assert_self(
        &wire,
        &json!({"target": assert_presence(&wire, read, v)["node_id"]}),
    );
    assert_declared_order(&wire);
}

/// A presence requirement spelled as a comparison with `absent`, `self.v
/// != absent`, has no `presence_read` written yet: the model is refused
/// `not-yet-implemented` at the comparison rather than emitted without the
/// requirement.
#[trace("TC-685", "FR-230-AC-50")]
#[test]
fn a_presence_requirement_compared_with_absent_is_not_written_yet() {
    let (member, text) = unwritten(
        &dropping_document(),
        "post P using v on Config::ConfigVersion::attemptUpdate { self.v != absent }\n",
    );
    assert_eq!(
        member,
        qsl_semantics::check::UnwrittenTimedMember::ComparedPresence
    );
    assert_eq!(text, "self.v != absent");
}

/// [`presence_document`] whose `attemptUpdate` frame names `v`.
fn dropping_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&presence_document()).expect("JSON");
    document["types"][1]["operations"][0]["frame"]["modifies"]
        .as_array_mut()
        .expect("modifies")
        .push(json!("ix://example/config-version/ConfigVersion/v"));
    serde_json::to_vec(&document).expect("JSON")
}

/// QSpec FR-322-AC-267 (STD-560, `excluded_forms`): the forms the checked
/// package excludes refuse `unsupported_construct`/`expression-form` at the
/// form, naming the construct, and no package is emitted: a guard disjunct
/// that reads no clock at that disjunct (`GJ-01`), a clock collection
/// written by a literal or a `filter` at the write (`CW-01`, `CW-02`), a
/// drop `self.v = absent` at the write (`CW-03`) and a reset by selection
/// that reads the element's data at the condition (`SD-01`). The controls
/// build: `self.x >= 0 ms or self.versionNumber > 0` is the one part of
/// `x >= 0 ms`, which every clock meets (`GJ-03`), and a constant write of
/// `v` is its reset (`CW-04`).
#[trace("TC-685", "FR-230-AC-50", "QSpec-FR-322-AC-267")]
#[test]
fn excluded_clause_forms_refuse_at_the_form() {
    use qsl_semantics::check::ExcludedClauseForm as Form;
    let attempt = |body: &str| {
        format!("post P using v on Config::ConfigVersion::attemptUpdate {{ {body} }}\n")
    };
    for (document, body, form, text) in [
        (
            document(false),
            "pre Or using v on Config::ConfigVersion::attemptUpdate { \
             self.x >= 1 ms or self.versionNumber > 0 }\n"
                .to_owned(),
            Form::DataOnlyDisjunct,
            "self.versionNumber > 0",
        ),
        (
            timer_document(),
            attempt("self.ts = sequence[Config::Timer { x: 0 ms, n: 1 }]"),
            Form::CollectionWrite,
            "self.ts = sequence[Config::Timer { x: 0 ms, n: 1 }]",
        ),
        (
            timer_document(),
            attempt("self.ts = filter(e in pre(self.ts): e.n > 1)"),
            Form::CollectionWrite,
            "self.ts = filter(e in pre(self.ts): e.n > 1)",
        ),
        (
            dropping_document(),
            attempt("self.v = absent"),
            Form::PresenceDrop,
            "self.v = absent",
        ),
        (
            timer_document(),
            attempt("forall(e in self.ts: e.n > 1 implies e.x = 0 ms)"),
            Form::ElementDataSelection,
            "e.n > 1",
        ),
    ] {
        let (found, located) = excluded(&document, &body);
        assert_eq!((found, located.as_str()), (form, text), "{body}");
        assert!(!found.description().is_empty());
    }
    let (_, wire) = emitted(
        &document(false),
        "pre Zero using v on Config::ConfigVersion::attemptUpdate { \
         self.x >= 0 ms or self.versionNumber > 0 }\n",
    );
    let [guard] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    let parts = member(&guard["body"], "parts")["members"]
        .as_array()
        .expect("parts");
    assert_eq!(parts.len(), 1, "the disjunct x >= 0 ms alone");
    let (_, wire) = emitted(&dropping_document(), &attempt("self.v = 0 ms"));
    assert_eq!(timed(&wire, "clock_reset").len(), 1);
}
