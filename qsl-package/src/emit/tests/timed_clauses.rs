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
    let packages = qsl_semantics::model::intake::package_input([document]);
    let unit = clocked_unit(&packages, body);
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
    let digest = &reference["target"]["digest"];
    nodes(wire)
        .iter()
        .find(|node| node["node_id"]["digest"] == *digest)
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
    let nodes: Vec<quire_contract_model::CheckedSemanticNodeV2> =
        serde_json::from_value(wire["semantic_graph"]["nodes"].clone()).expect("nodes");
    let source_map: Vec<quire_contract_model::CheckedSourceMapEntry> =
        serde_json::from_value(wire["source_map"].clone()).expect("source map");
    assert_eq!(
        crate::emit::timed_order::first_out_of_order(&nodes, &source_map),
        Ok(None)
    );
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
/// operation anchor, so two guarded preconditions of one operation, `self.x
/// >= 1 ms or self.x > 7 ms` and `self.versionNumber > 0 and self.x < 5 ms`,
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
    assert_eq!(operands, ["self.versionNumber > 0", "self.versionNumber < 9"]);
    assert_eq!(part["members"].as_array().map(Vec::len), Some(3));
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

    let (member, text) = unwritten(
        &document(false),
        "pre Or using v on Config::ConfigVersion::attemptUpdate { \
         self.x >= 1 ms or self.versionNumber > 0 }\n",
    );
    assert_eq!(member, Member::UnconstrainedGuardPart);
    assert!(text.starts_with("pre Or"), "{text}");
}

/// FR-230-AC-50, QSpec FR-342-AC-11: a clock-reading clause bound to an
/// execution point refuses `not-yet-implemented` at the clause, naming
/// STD-555, and is never written with a point-qualified anchor.
#[trace("TC-685", "FR-230-AC-50", "QSpec-FR-342-AC-11")]
#[test]
fn a_point_qualified_clock_clause_refuses_at_the_clause() {
    use quire_contract_model::{AnchorName, ExecutionPoint};
    let document = document(false);
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let unit = clocked_unit(
        &packages,
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
        built,
        models,
        Vec::new(),
        Default::default(),
        &quire_exact::Cancel::new(),
        BTreeMap::from([("Late".to_owned(), point)]),
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
    let integer = qsl_semantics::check::builtin_integer_type_key().to_string();
    assert_eq!(binder["semantic_type"]["digest"], integer);
    assert_true(&wire, target(&wire, member(&reset["body"], "condition")));
    assert_eq!(
        located(&wire, reset, &unit),
        "forall(e in self.ts: e.x = 0 ms)"
    );
    assert_declared_order(&wire);

    let (member, text) = unwritten(
        &timer_document(),
        "post R using v on Config::ConfigVersion::attemptUpdate { \
         forall(e in self.ts: e.n > 1 implies e.x = 0 ms) }\n",
    );
    assert_eq!(
        member,
        qsl_semantics::check::UnwrittenTimedMember::SelectionPredicate
    );
    assert_eq!(text, "forall(e in self.ts: e.n > 1 implies e.x = 0 ms)");
}
