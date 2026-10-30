// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-447 (TC-1187, TC-1220), QSpec FR-481: a checked hybrid model and its
//! claims emit as `hybrid` nodes and `temporal_clause` nodes, and QSL's I2
//! read admits them.

use std::collections::BTreeSet;

use qsl_semantics::check::{CheckedHybridClaim, CheckedHybridModel, HybridHorizon};
use quire_contract_model::{
    CheckedPackageRefusal, CheckedPackageRefusalCause, CheckedPackageRefusalCode,
};

use super::*;

use crate::checked_v2::{read_checked_package_v2, read_hybrids, HybridRead, V2ReadRefusal};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.value.complete/v1\";\n\
     profile p = \"quire.temporal.infinite-trace/v1\";\n\
     profile t = \"quire.temporal.timed/v1\";\n\
     profile e = \"quire.temporal.event-position.false-extension/v1\";\n\
     dimension Time;\n\
     unit s : Time = rational(1, 1);\n\
     unit ms : Time = rational(1, 1000) * s;\n";

/// FR-447's `Braking`, with `flow` the flow of `Moving`.
fn braking(flow: &str) -> String {
    format!(
        "hybrid Braking using p {{\n\
           mode Moving invariant {{ v >= 0 and v <= 20 and a >= 1 and a <= 2 }}\n\
             flow {{ {flow} }}\n\
             transition to Stopped when {{ v = 0 }} reset {{ v = 0 }};\n\
           mode Stopped invariant {{ v = 0 }}\n\
             flow {{ v' = 0; a' = 0; t' = 1; }}\n\
         }}\n"
    )
}

const FLOW: &str = "v' = -a; a' = 0; t' = 1;";

/// `SlowEnough` under the profile alias `profile`, with `body`.
fn slow_enough(profile: &str, body: &str) -> String {
    format!(
        "temporal SlowEnough using {profile} over (h: Braking) clock \"model-time\" \
         on origin {{ {body} }}\n"
    )
}

const BODY: &str = "always [0, 10] holds(h.v < 30)";

/// FR-447's `Braking2`, `Slows` and `Recent`.
const BRAKING2: &str = r#"hybrid Braking2 using p {
  initial {
    mode Moving when { v = 20 and a >= 1 and a <= 2 and t = 0 };
    mode Stopped when { v = 0 and t = 0 };
  }
    mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }
    flow { v' = -a; a' = 0; t' = 1; }
    transition to Stopped when { v = 0 } reset { v = 0 and a >= 1 };
  mode Stopped invariant { v = 0 }
    flow { v' = 0; a' = 0; t' = 1; }
}
temporal Slows using t over (h: Braking2) clock "model-time"
    on each (q: Braking2) when (q.v > 10) {
  fair Moving::Stopped;
  capture v0: Rational[0, 20; 1, 1] = q.v;
    eventually [0 s, 10500 ms] holds(h.v < v0 and h.v <= 10)
}
temporal Recent using p over (h: Braking2) clock "model-steps" on origin {
  once [0, 2] holds(h.v = 20)
}
"#;

/// The checked hybrid models and claims of a compiled unit.
struct Checked {
    models: Vec<CheckedHybridModel>,
    claims: Vec<CheckedHybridClaim>,
}

/// `declarations` through S1, S2, the assembler, check, link and the v2
/// emitter; nothing may be omitted.
fn compile(declarations: &str) -> (Emission, Value) {
    let (emission, wire, _) = compile_checked(declarations);
    (emission, wire)
}

/// [`compile`], with the checked models and claims the emission was made from.
fn compile_checked(declarations: &str) -> (Emission, Value, Checked) {
    let unit = format!("{HEADER}{declarations}");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "example", "fixture", "fixture:1"),
        "program.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert_eq!(parsed.diagnostics(), []);
    let raw = parsed.source().reference().clone();
    let form = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let graph = PackageDeclarations::assemble(
        raw,
        "pkg".to_owned(),
        form,
        qsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let checked = Checked {
        models: graph.hybrid_models().to_vec(),
        claims: graph.hybrid_claims().to_vec(),
    };
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    (emission, wire, checked)
}

/// `emission` after QSL's full I2 read, which runs IR's FR-481 reader.
fn admitted(emission: &Emission) {
    let Read::Verified { .. } = read_back(emission) else {
        panic!("the I2 read admits the package: {:?}", read_back(emission));
    };
}

fn idtext(id: &Value) -> String {
    id.to_string()
}

fn node_with(wire: &Value, digest: impl AsRef<str>) -> &Value {
    nodes(wire)
        .iter()
        .find(|node| idtext(&node["node_id"]) == digest.as_ref())
        .unwrap_or_else(|| panic!("a node {}", digest.as_ref()))
}

fn target(term: &Value) -> String {
    assert_eq!(term["term"], "reference", "{term}");
    idtext(&term["target"])
}

/// The nodes of `wire` with the form `form`, in graph order.
fn of_form<'w>(wire: &'w Value, form: &str) -> Vec<&'w Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["semantic_form"] == form)
        .collect()
}

/// The node `declared` names: `["Braking"]` or `["Braking", "Moving"]`.
fn declared<'w>(wire: &'w Value, form: &str, name: &[&str]) -> &'w Value {
    let matching: Vec<_> = of_form(wire, form)
        .into_iter()
        .filter(|node| node["declaration"]["qualified_name"] == json!(name))
        .collect();
    let [node] = matching[..] else {
        panic!("one {form} node {name:?}: {matching:?}");
    };
    node
}

/// The body binding `name` of the aggregate `body`.
fn binding<'w>(body: &'w Value, name: &str) -> &'w Value {
    body["members"]
        .as_array()
        .expect("an aggregate")
        .iter()
        .find(|member| member["name"] == name)
        .map(|member| &member["value"])
        .unwrap_or_else(|| panic!("a binding {name} in {body}"))
}

/// The node an aggregate `entries` reference at `index`.
fn entry<'w>(wire: &'w Value, entries: &Value, index: usize) -> &'w Value {
    node_with(wire, target(&entries["members"][index]))
}

/// `(name, level)` of the `value`/`parameter` node `node`.
fn parameter(node: &Value) -> (String, String) {
    assert_eq!(node["semantic_form"], "parameter", "{node}");
    (
        binding(&node["body"], "name")["value"]
            .as_str()
            .unwrap()
            .to_owned(),
        binding(&node["body"], "level")["value"]
            .as_str()
            .unwrap()
            .to_owned(),
    )
}

/// The operation identity of the expression node `node`.
fn operation(node: &Value) -> &str {
    node["body"]["operation"]["identity"].as_str().unwrap()
}

/// The digest set of every `reference` target in `body`.
fn targets(body: &Value, found: &mut BTreeSet<String>) {
    match body {
        Value::Object(map) => {
            if map.get("term") == Some(&json!("reference")) {
                found.insert(idtext(&map["target"]));
            } else {
                map.values().for_each(|value| targets(value, found));
            }
        }
        Value::Array(items) => items.iter().for_each(|value| targets(value, found)),
        _ => {}
    }
}

/// The one `temporal_clause` node whose `over` parameter is named `over`.
fn clause<'w>(wire: &'w Value, over: &str) -> &'w Value {
    let matching: Vec<_> = of_form(wire, "temporal_clause")
        .into_iter()
        .filter(|node| {
            let over_node = node_with(wire, target(&node["body"]["arguments"][0]));
            parameter(over_node).0 == over
        })
        .collect();
    let [node] = matching[..] else {
        panic!("one clause over {over}");
    };
    node
}

fn formula_root<'w>(wire: &'w Value, clause: &Value) -> &'w Value {
    node_with(wire, target(&clause["body"]["arguments"][5]))
}

fn operand<'w>(wire: &'w Value, formula: &Value, index: usize) -> &'w Value {
    node_with(wire, target(&formula["body"]["arguments"][index]))
}

fn law(clause: &Value) -> &str {
    clause["body"]["operation"]["laws"][0]["definition"]["identity"]
        .as_str()
        .unwrap()
}

/// FR-447-AC-5 (FR-481-AC-1): `Braking` and `SlowEnough` emit the
/// model, mode and transition nodes and the claim node FR-481 states, and
/// QSL's I2 read admits the package.
#[trace("TC-1187", "FR-447-AC-5")]
#[test]
fn braking_and_slow_enough_emit_as_hybrid_nodes_the_i2_read_admits() {
    let (emission, wire) = compile(&format!("{}{}", braking(FLOW), slow_enough("p", BODY)));
    admitted(&emission);

    let model = declared(&wire, "hybrid_model", &["Braking"]);
    assert_eq!(model["node_tag"], "hybrid");
    assert_eq!(model["owner"]["kind"], "source");
    assert_eq!(model["occurrences"].as_array().unwrap().len(), 1);
    assert_eq!(model["occurrences"][0]["role"], "declaration");
    // Every `hybrid` node is its own `semantic_type`, and its dependencies
    // are exactly its body's targets.
    for node in nodes(&wire)
        .iter()
        .filter(|node| node["node_tag"] == "hybrid")
    {
        assert_eq!(node["semantic_type"], node["node_id"], "{node}");
        let mut expected = BTreeSet::new();
        targets(&node["body"], &mut expected);
        let dependencies: BTreeSet<String> = node["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .map(idtext)
            .collect();
        assert_eq!(dependencies, expected, "{node}");
    }

    // `v`, `a` and `t` at levels 0 to 2, classed `hybrid`, `hybrid`, `clock`,
    // typed by the `rational` scalar node.
    let variables = binding(&model["body"], "variables");
    let rational = nodes(&wire)
        .iter()
        .find(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "rational")
        .expect("the rational scalar node");
    let mut variable_nodes = Vec::new();
    for (position, (name, class)) in [("v", "hybrid"), ("a", "hybrid"), ("t", "clock")]
        .into_iter()
        .enumerate()
    {
        let member = &variables["members"][position];
        let node = node_with(&wire, target(binding(member, "parameter")));
        assert_eq!(parameter(node), (name.to_owned(), position.to_string()));
        assert_eq!(node["semantic_type"], rational["node_id"]);
        assert_eq!(binding(member, "class")["value"], class);
        variable_nodes.push(node);
    }

    // The modes, in declared order.
    let modes = binding(&model["body"], "modes");
    let moving = entry(&wire, modes, 0);
    let stopped = entry(&wire, modes, 1);
    assert_eq!(
        moving["declaration"]["qualified_name"],
        json!(["Braking", "Moving"])
    );
    assert_eq!(
        stopped["declaration"]["qualified_name"],
        json!(["Braking", "Stopped"])
    );
    assert_eq!(moving["owner"], model["owner"]);

    // `Moving`'s rates are the nodes of `-a`, `0` and `1`.
    let rates = binding(&moving["body"], "rates");
    let negated = entry(&wire, rates, 0);
    assert_eq!(operation(negated), "quire.op.rational.negate");
    assert_eq!(
        target(&negated["body"]["arguments"][0]),
        idtext(&variable_nodes[1]["node_id"])
    );
    assert_eq!(entry(&wire, rates, 1)["body"]["value"], "0");
    assert_eq!(entry(&wire, rates, 2)["body"]["value"], "1");

    // Its one transition: target 1, the guard `v = 0` and the reset (0, 0),
    // with no `declaration` and an `expression` occurrence.
    let transitions = binding(&moving["body"], "transitions");
    let transition = entry(&wire, transitions, 0);
    assert_eq!(transition["semantic_form"], "hybrid_transition");
    assert!(transition.get("declaration").is_none() && transition.get("owner").is_none());
    assert_eq!(transition["occurrences"][0]["role"], "expression");
    assert_eq!(binding(&transition["body"], "target")["value"], "1");
    let guard = node_with(&wire, target(binding(&transition["body"], "guard")));
    assert_eq!(operation(guard), "quire.op.rational.eq");
    let resets = binding(&transition["body"], "resets");
    assert_eq!(resets["members"].as_array().unwrap().len(), 1);
    let reset = &resets["members"][0];
    assert_eq!(binding(reset, "variable")["value"], "0");
    assert_eq!(
        node_with(&wire, target(binding(reset, "value")))["body"]["value"],
        "0"
    );
    assert!(reset["members"]
        .as_array()
        .unwrap()
        .iter()
        .all(|member| member["name"] != "relation"));
    // `Stopped` has no transition.
    assert_eq!(
        binding(&stopped["body"], "transitions")["members"],
        json!([])
    );

    // One clause over a parameter typed by the model node, clock
    // `model-time`, origin activation, no capture, no fairness.
    let claim = clause(&wire, "h");
    let over = node_with(&wire, target(&claim["body"]["arguments"][0]));
    assert_eq!(over["semantic_type"], model["node_id"]);
    assert_eq!(parameter(over).1, "0");
    assert_eq!(claim["body"]["arguments"][1]["value"], "model-time");
    assert_eq!(law(claim), "quire.temporal.infinite-trace/v1");
    assert_eq!(
        claim["body"]["arguments"][2]["members"][0]["value"]["value"],
        "origin"
    );
    assert_eq!(claim["body"]["arguments"][3]["members"], json!([]));
    assert_eq!(claim["body"]["arguments"][4]["members"], json!([]));
    let always = formula_root(&wire, claim);
    assert_eq!(operation(always), "quire.op.temporal.always");
    assert_eq!(
        always["body"]["operation"]["member"],
        json!({"kind": "temporal_interval", "interval": {"lower": "0", "upper": "10"}})
    );
    let holds = operand(&wire, always, 0);
    assert_eq!(operation(holds), "quire.op.temporal.holds");
    let comparison = operand(&wire, holds, 0);
    assert_eq!(operation(comparison), "quire.op.rational.lt");
    assert_eq!(
        target(&comparison["body"]["arguments"][0]),
        idtext(&variable_nodes[0]["node_id"])
    );
}

/// FR-447-AC-12 (FR-481-AC-8, AC-9): `Braking2`, `Slows` and `Recent` emit
/// the initial entries, the resets, the `each` activation with its guard and
/// capture, the fairness member and both interval forms; the QSpec reader
/// admits the package, and `Braking`'s model node has no `initial` binding.
#[trace("TC-1220", "FR-447-AC-12")]
#[test]
fn braking2_slows_and_recent_emit_initial_entries_resets_activation_and_fairness() {
    let (emission, wire) = compile(&format!("{}{BRAKING2}", braking(FLOW)));
    admitted(&emission);

    // `Braking` writes no `initial`, so its model carries no binding for it.
    let default = declared(&wire, "hybrid_model", &["Braking"]);
    assert_eq!(default["body"]["members"].as_array().unwrap().len(), 2);

    let model = declared(&wire, "hybrid_model", &["Braking2"]);
    let initial = binding(&model["body"], "initial");
    assert_eq!(initial["members"].as_array().unwrap().len(), 2);
    for (position, mode) in ["0", "1"].into_iter().enumerate() {
        let item = &initial["members"][position];
        assert_eq!(binding(item, "mode")["value"], mode);
        let condition = node_with(&wire, target(binding(item, "condition")));
        assert_eq!(operation(condition), "quire.op.boolean.and");
    }

    // The resets `(0, 0)` and `(1, 1, ge)` of `Moving`'s transition.
    let moving = declared(&wire, "hybrid_mode", &["Braking2", "Moving"]);
    let transition = entry(&wire, binding(&moving["body"], "transitions"), 0);
    let resets = binding(&transition["body"], "resets");
    assert_eq!(resets["members"].as_array().unwrap().len(), 2);
    assert_eq!(binding(&resets["members"][0], "variable")["value"], "0");
    assert!(resets["members"][0]["members"]
        .as_array()
        .unwrap()
        .iter()
        .all(|member| member["name"] != "relation"));
    assert_eq!(binding(&resets["members"][1], "variable")["value"], "1");
    assert_eq!(binding(&resets["members"][1], "relation")["value"], "ge");

    // `Slows`: the `each` activation, guard, capture, fairness and interval.
    let (slows, recent) = {
        let both: Vec<_> = of_form(&wire, "temporal_clause");
        assert_eq!(both.len(), 2);
        let timed = both
            .iter()
            .copied()
            .find(|node| law(node) == "quire.temporal.timed/v1")
            .expect("Slows");
        let steps = both
            .iter()
            .copied()
            .find(|node| law(node) == "quire.temporal.infinite-trace/v1")
            .expect("Recent");
        (timed, steps)
    };
    let activation = &slows["body"]["arguments"][2];
    assert_eq!(binding(activation, "kind")["value"], "each");
    let trigger = node_with(&wire, target(binding(activation, "trigger")));
    assert_eq!(parameter(trigger), ("q".to_owned(), "1".to_owned()));
    assert_eq!(trigger["semantic_type"], model["node_id"]);
    let guard = node_with(&wire, target(binding(activation, "guard")));
    assert_eq!(operation(guard), "quire.op.rational.gt");
    let read = node_with(&wire, target(&guard["body"]["arguments"][0]));
    assert_eq!(parameter(read), ("v".to_owned(), "3".to_owned()));
    // The capture `v0` holds the activation read of `v`.
    let capture = &slows["body"]["arguments"][3];
    assert_eq!(
        target(binding(capture, "v0")),
        idtext(&read["node_id"])
    );
    // Fairness: weak whole over `[Braking2, Moving]` and `Stopped`.
    let fairness = &slows["body"]["arguments"][4];
    let fair = node_with(&wire, target(&fairness["members"][0]));
    let member = &fair["body"]["operation"]["member"];
    assert_eq!(member["fairness_kind"], "weak");
    assert_eq!(member["granularity"], "whole");
    assert_eq!(member["declaration"], moving["node_id"]);
    assert_eq!(member["name"], "Stopped");
    // The timed interval `[0, 21/2]` over `holds`, whose operand reads the
    // capture at level 6 typed by the `rational_range` node.
    let eventually = formula_root(&wire, slows);
    assert_eq!(operation(eventually), "quire.op.temporal.eventually");
    assert_eq!(
        eventually["body"]["operation"]["member"],
        json!({"kind": "temporal_interval", "interval": {
            "lower": {"numerator": "0", "denominator": "1"}, "lower_end": "closed",
            "upper": {"numerator": "21", "denominator": "2"}, "upper_end": "closed"}})
    );
    let holds = operand(&wire, eventually, 0);
    let conjunction = operand(&wire, holds, 0);
    assert_eq!(operation(conjunction), "quire.op.boolean.and");
    let below = node_with(&wire, target(&conjunction["body"]["arguments"][0]));
    let v0 = node_with(&wire, target(&below["body"]["arguments"][1]));
    assert_eq!(parameter(v0), ("v0".to_owned(), "6".to_owned()));
    let range = node_with(&wire, idtext(&v0["semantic_type"]));
    assert_eq!(range["semantic_form"], "rational_range");

    // `Recent`: clock `model-steps`, an integer `once` interval `[0, 2]`.
    assert_eq!(recent["body"]["arguments"][1]["value"], "model-steps");
    let once = formula_root(&wire, recent);
    assert_eq!(operation(once), "quire.op.temporal.once");
    assert_eq!(
        once["body"]["operation"]["member"],
        json!({"kind": "temporal_interval", "interval": {"lower": "0", "upper": "2"}})
    );
}

/// FR-447-AC-12 (QSpec FR-481-AC-8, AC-9): every restored hybrid node is
/// equal to the QSpec vector, not merely the members named in the example.
#[trace("TC-1220", "FR-447-AC-12", "QSpec-FR-481-AC-8", "QSpec-FR-481-AC-9")]
#[test]
fn braking2_restored_nodes_match_qspec_vectors() {
    let Some(qspec) = std::env::var_os("QSPEC_DIR") else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let qspec = std::path::PathBuf::from(qspec);
    let path = qspec.join("proposals/checked-package-v2/hybrid-node-vectors.json");
    let fixture: Value = serde_json::from_slice(&std::fs::read(path).expect("QSpec vectors"))
        .expect("QSpec vector JSON");
    let expected = fixture["restored_nodes"]
        .as_array()
        .expect("QSpec restored nodes");
    let (emission, wire) = compile(BRAKING2);
    admitted(&emission);
    let expected_ids: BTreeSet<String> = expected
        .iter()
        .map(|node| {
            idtext(&node["node_id"])
        })
        .collect();
    let base_ids: BTreeSet<String> = fixture["nodes"]
        .as_array()
        .expect("QSpec base nodes")
        .iter()
        .map(|node| {
            idtext(&node["node_id"])
        })
        .collect();
    let model = declared(&wire, "hybrid_model", &["Braking2"]);
    let mut pending: Vec<String> = std::iter::once(model)
        .chain(of_form(&wire, "temporal_clause"))
        .map(|node| {
            idtext(&node["node_id"])
        })
        .collect();
    let mut reached = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !reached.insert(id.clone()) {
            continue;
        }
        let node = node_with(&wire, &id);
        pending.extend(
            node["dependencies"]
                .as_array()
                .expect("node dependencies")
                .iter()
                .map(|dependency| {
                    idtext(dependency)
                }),
        );
        pending.push(
            idtext(&node["semantic_type"]),
        );
    }
    let actual_ids: BTreeSet<String> = reached.difference(&base_ids).cloned().collect();
    assert_eq!(actual_ids, expected_ids, "complete restored node set");
    for node in expected {
        let id = idtext(&node["node_id"]);
        let mut actual = node_with(&wire, &id).clone();
        let mut expected = node.clone();
        // A package's source map may attach more occurrences to a shared
        // semantic node; it does not change the node's semantic content.
        actual.as_object_mut().unwrap().remove("occurrences");
        expected.as_object_mut().unwrap().remove("occurrences");
        assert_eq!(actual, expected, "QSpec restored node {id}");
    }
}

/// QSpec FR-481: `fair Moving` and `fair Moving::Stopped` name different
/// declarations even when all of Moving's transitions target Stopped.
#[trace("TC-1187", "FR-447-AC-5", "QSpec-FR-481-AC-9")]
#[test]
fn fairness_member_preserves_the_source_mode_or_target_form() {
    for (source, declaration, name) in [
        ("fair Moving;", "model", "Moving"),
        ("fair Moving::Stopped;", "mode", "Stopped"),
    ] {
        let claim = format!(
            "temporal MovingFair using p over (h: Braking) clock \"model-time\" \
             on origin {{ {source} always [0, 10] true }}"
        );
        let (emission, wire, checked) = compile_checked(&format!("{}{}", braking(FLOW), claim));
        admitted(&emission);
        assert_read_back_equals(&read_back_hybrids(&emission, &wire), &checked);
        let model = declared(&wire, "hybrid_model", &["Braking"]);
        let mode = declared(&wire, "hybrid_mode", &["Braking", "Moving"]);
        let claim = clause(&wire, "h");
        let fair = node_with(&wire, target(&claim["body"]["arguments"][4]["members"][0]));
        let member = &fair["body"]["operation"]["member"];
        let expected = if declaration == "model" { model } else { mode };
        assert_eq!(member["declaration"], expected["node_id"], "{source}");
        assert_eq!(member["name"], name, "{source}");
    }
}

/// FR-447-AC-14: `always [0, 10] true` emits `quire.op.temporal.true` under
/// the `always` node and no `holds` node; `SlowEnough` emits the
/// `temporal_profile` law its `using` alias selects.
#[trace("TC-1187", "FR-447-AC-14")]
#[test]
fn a_constant_formula_emits_true_and_false_nodes_and_the_alias_law() {
    for (body, constant) in [
        ("always [0, 10] true", "quire.op.temporal.true"),
        ("always [0, 10] false", "quire.op.temporal.false"),
    ] {
        let (emission, wire) = compile(&format!("{}{}", braking(FLOW), slow_enough("p", body)));
        admitted(&emission);
        let claim = clause(&wire, "h");
        let always = formula_root(&wire, claim);
        assert_eq!(operation(operand(&wire, always, 0)), constant);
        assert!(of_form(&wire, "formula")
            .iter()
            .all(|node| operation(node) != "quire.op.temporal.holds"));
    }
    for (alias, identity) in [
        ("p", "quire.temporal.infinite-trace/v1"),
        ("t", "quire.temporal.timed/v1"),
    ] {
        let (_, wire) = compile(&format!("{}{}", braking(FLOW), slow_enough(alias, BODY)));
        assert_eq!(law(clause(&wire, "h")), identity);
    }
}

/// FR-447-AC-15 (FR-481-AC-11, AC-12): the claim under `timed/v1` emits the
/// timed interval `[0, 10]` with both ends closed in exact rational seconds,
/// and `[0, *]` its `upper` `null` with `upper_end` `open`; under
/// `infinite-trace` the same claim emits the integer `[0, 10]`, and `[2, *]`
/// the integer `{lower: "2", upper: null}`. The reader admits each.
#[trace("TC-1187", "FR-447-AC-15")]
#[test]
fn interval_forms_follow_the_claims_profile() {
    let interval = |profile: &str, body: &str| {
        let (emission, wire) = compile(&format!("{}{}", braking(FLOW), slow_enough(profile, body)));
        admitted(&emission);
        let always = formula_root(&wire, clause(&wire, "h")).clone();
        always["body"]["operation"]["member"].clone()
    };
    let rational = |numerator: &str| json!({"numerator": numerator, "denominator": "1"});
    assert_eq!(
        interval("t", BODY),
        json!({"kind": "temporal_interval", "interval": {
            "lower": rational("0"), "lower_end": "closed",
            "upper": rational("10"), "upper_end": "closed"}})
    );
    assert_eq!(
        interval("t", "always [0, *] holds(h.v < 30)"),
        json!({"kind": "temporal_interval", "interval": {
            "lower": rational("0"), "lower_end": "closed",
            "upper": null, "upper_end": "open"}})
    );
    assert_eq!(
        interval("t", "always holds(h.v < 30)"),
        json!({"kind": "temporal_interval", "interval": null})
    );
    assert_eq!(
        interval("p", BODY),
        json!({"kind": "temporal_interval", "interval": {"lower": "0", "upper": "10"}})
    );
    assert_eq!(
        interval("p", "always [2, *] holds(h.v < 30)"),
        json!({"kind": "temporal_interval", "interval": {"lower": "2", "upper": null}})
    );
}

// ------------------------------------------------------------- read-back

/// QSL's I2 read-back of the hybrid models and claims of `emission`.
fn read_back_hybrids(emission: &Emission, wire: &Value) -> HybridRead {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let read = read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the package: {failure:?}"));
    read_hybrids(&read.into_value().admitted, 100_000).expect("the package reads back")
}

/// Each read model equals the checked model of its name, and each read claim
/// equals a checked claim once its `model` index is the read model's.
fn assert_read_back_equals(read: &HybridRead, checked: &Checked) {
    assert_eq!(read.models.len(), checked.models.len());
    let mut remap = Vec::new();
    for model in &checked.models {
        let position = read
            .models
            .iter()
            .position(|read| read.model.name == model.name)
            .unwrap_or_else(|| panic!("a read model {}", model.name.as_str()));
        assert_eq!(&read.models[position].model, model);
        remap.push(position);
    }
    assert_eq!(read.claims.len(), checked.claims.len());
    for claim in &checked.claims {
        let expected = CheckedHybridClaim {
            model: qsl_semantics::check::HybridModelIndex::new(remap[claim.model.get()]),
            ..claim.clone()
        };
        assert!(
            read.claims.iter().any(|read| read.claim == expected),
            "no read claim equals {expected:?} in {:?}",
            read.claims
        );
    }
}

/// FR-447-AC-13: QSL's I2 read-back of AC-5's package returns `Braking`'s
/// `CheckedHybridModel` and `SlowEnough`'s `CheckedHybridClaim` with horizon
/// `Seconds(10)`, each equal to the checked one.
#[trace("TC-1187", "FR-447-AC-13")]
#[test]
fn braking_and_slow_enough_read_back_equal_to_the_checked_ones() {
    let (emission, wire, checked) =
        compile_checked(&format!("{}{}", braking(FLOW), slow_enough("p", BODY)));
    let read = read_back_hybrids(&emission, &wire);
    assert_read_back_equals(&read, &checked);
    let [claim] = read.claims.as_slice() else {
        panic!("one claim");
    };
    assert_eq!(
        claim.claim.horizon,
        Some(HybridHorizon::Seconds(quire_exact::Rational::from_integer(
            10_i64.into()
        )))
    );
}

/// FR-447-AC-16: QSL's I2 read-back of AC-12's package returns `Braking2`
/// with its two initial entries and its `Eq` and `Ge` reset constraints, and
/// `Slows` and `Recent` with their clocks, the `Each` activation and its
/// guard, the capture `v0` and its type, the fairness constraint, the `Once`
/// operator and the intervals, each equal to the checked one.
#[trace("TC-1220", "FR-447-AC-16")]
#[test]
fn braking2_slows_and_recent_read_back_equal_to_the_checked_ones() {
    let (emission, wire, checked) = compile_checked(&format!("{}{BRAKING2}", braking(FLOW)));
    let read = read_back_hybrids(&emission, &wire);
    assert_read_back_equals(&read, &checked);
    // The read-back is the declaration's content, not a shape the test
    // restates: spot-check the parts AC-16 names.
    let braking2 = read
        .models
        .iter()
        .find(|read| read.model.name.as_str() == "Braking2")
        .expect("Braking2")
        .model
        .clone();
    assert_eq!(braking2.initial.len(), 2);
    assert_eq!(
        braking2.modes[0].transitions[0]
            .resets
            .iter()
            .map(|reset| reset.relation)
            .collect::<Vec<_>>(),
        [
            qsl_semantics::check::CompareOp::Eq,
            qsl_semantics::check::CompareOp::Ge
        ]
    );
    let slows = read
        .claims
        .iter()
        .find(|read| !read.claim.captures.is_empty())
        .expect("Slows")
        .claim
        .clone();
    assert_eq!(slows.captures[0].name.as_str(), "v0");
    assert_eq!(slows.fairness.len(), 1);
    assert_eq!(slows.horizon, None);
}

/// A copy of `wire` after `edit` of the node `digest`, rekeyed and with the
/// package identity recomputed.
fn edited(wire: &Value, digest: &str, edit: impl FnOnce(&mut Value)) -> Value {
    let mut envelope = wire.clone();
    let node = envelope["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .find(|node| idtext(&node["node_id"]) == digest)
        .expect("the edited node");
    edit(node);
    envelope
}

/// QSL's I2 read of `envelope`, expected to refuse in IR's reader.
fn refused(emission: &Emission, wire: &Value, envelope: &Value) -> (CheckedPackageRefusal, String) {
    let read = read_v2(
        &jcs(envelope),
        library(),
        V2ReadLimits::default(),
        &{
            let mut evidence = CheckedPackageEvidence::new();
            for feature in wire["lock"]["required_features"].as_array().unwrap() {
                evidence.support_feature(feature.as_str().unwrap());
            }
            evidence
        },
    );
    let _ = emission;
    match read {
        Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) => {
            let path = refusal
                .loci[0]
                .path
                .as_ref()
                .map(|pointer| pointer.as_str().to_owned())
                .unwrap_or_default();
            (*refusal, path)
        }
        other => panic!("an IR refusal, not {other:?}"),
    }
}

/// FR-447-AC-13, AC-16 (QSpec FR-481 "Reader order"): with `dependencies`,
/// keys and `package_id` rebuilt after each edit, QSL's I2 read refuses a
/// mode with a fourth rate, every variable classed `clock`, a claim clock
/// `wall`, a model body with `modes` before `variables`, a written default
/// `initial` and a fairness member naming `Halted`, each at its FR-481 locus.
#[trace("TC-1187", "FR-447-AC-13")]
#[trace("TC-1220", "FR-447-AC-16")]
#[test]
fn the_i2_read_refuses_each_fr_481_defect_at_its_locus() {
    let (emission, wire) = compile(&format!(
        "{}{BRAKING2}{}",
        braking(FLOW),
        slow_enough("p", BODY)
    ));
    let digest = |form: &str, name: &[&str]| -> String {
        idtext(&declared(&wire, form, name)["node_id"])
    };
    let refuse = |envelope: &Value| refused(&emission, &wire, envelope);

    // A fourth rate in `Braking.Moving`: `invalid_model_binding` at `rates`.
    let moving = digest("hybrid_mode", &["Braking", "Moving"]);
    let (refusal, path) = refuse(&edited(&wire, &moving, |node| {
        let rates = node["body"]["members"][1]["value"]["members"]
            .as_array_mut()
            .unwrap();
        let first = rates[0].clone();
        rates.push(first);
    }));
    assert_eq!(refusal.code, CheckedPackageRefusalCode::InvalidModelBinding);
    assert_eq!(
        refusal.cause,
        Some(CheckedPackageRefusalCause::MalformedDeclaration)
    );
    assert!(path.ends_with("/body/members/1/value"), "{path}");

    // Every variable classed `clock`: `invalid_package`/`invalid-value` at
    // `v`'s class literal, `v`'s rate `-a` not being the literal `1`.
    let model = digest("hybrid_model", &["Braking"]);
    let (refusal, path) = refuse(&edited(&wire, &model, |node| {
        for variable in node["body"]["members"][0]["value"]["members"]
            .as_array_mut()
            .unwrap()
        {
            variable["members"][1]["value"]["value"] = json!("clock");
        }
    }));
    assert_eq!(refusal.code, CheckedPackageRefusalCode::InvalidPackage);
    assert_eq!(
        refusal.cause,
        Some(CheckedPackageRefusalCause::InvalidValue)
    );
    assert!(
        path.ends_with("/body/members/0/value/members/0/members/1/value/value"),
        "{path}"
    );

    // A clock `wall`: `invalid_package`/`invalid-value` at the argument.
    let claim = of_form(&wire, "temporal_clause")
        .into_iter()
        .find(|node| {
            law(node) == "quire.temporal.infinite-trace/v1"
                && node["body"]["arguments"][1]["value"] == "model-time"
        })
        .expect("SlowEnough");
    let (refusal, path) = refuse(&edited(
        &wire,
        &idtext(&claim["node_id"]),
        |node| node["body"]["arguments"][1]["value"] = json!("wall"),
    ));
    assert_eq!(refusal.code, CheckedPackageRefusalCode::InvalidPackage);
    assert_eq!(
        refusal.cause,
        Some(CheckedPackageRefusalCause::InvalidValue)
    );
    assert!(path.ends_with("/body/arguments/1"), "{path}");

    // `modes` before `variables`: `malformed_wire`, before the hybrid step.
    let mut swapped = wire.clone();
    let position = nodes(&swapped)
        .iter()
        .position(|node| idtext(&node["node_id"]) == model)
        .unwrap();
    swapped["semantic_graph"]["nodes"][position]["body"]["members"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    let (refusal, _) = refuse(&swapped);
    assert_eq!(refusal.code, CheckedPackageRefusalCode::MalformedWire);

    // A written default `initial` (mode 0, condition `true`): refused
    // `invalid_semantic_graph` at `initial`.
    let boolean = nodes(&wire)
        .iter()
        .find(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "boolean")
        .expect("the boolean scalar node")["node_id"]
        .clone();
    let integer = nodes(&wire)
        .iter()
        .find(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "integer")
        .expect("the integer scalar node")["node_id"]
        .clone();
    let placeholder = qsl_foundation::node_wire::node_key_to_json(
        &quire_exact::NodeKey::new(
            boolean["package"].as_str().unwrap().to_owned(),
            vec![quire_exact::PathSegment::Text("placeholder".to_owned())],
            0,
        )
        .unwrap(),
    )
    .unwrap();
    let mut with_literal = wire.clone();
    with_literal["semantic_graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "node_id": placeholder,
            "schema_version": "quire.checked-semantic-graph/v2",
            "node_tag": "value",
            "semantic_form": "literal",
            "semantic_type": boolean,
            "dependencies": [boolean],
            "occurrences": [{"role": "expression", "ordinal": 0}],
            "body": {"term": "literal", "type": boolean, "value": true, "value_kind": "boolean"},
        }));
    let (refusal, path) = refuse(&edited(&with_literal, &model, |node| {
        node["body"]["members"].as_array_mut().unwrap().push(json!({
            "term": "binding",
            "name": "initial",
            "value": {"term": "aggregate", "members": [{"term": "aggregate", "members": [
                {"term": "binding", "name": "mode", "value": {
                    "term": "literal", "type": integer, "value": "0", "value_kind": "integer"}},
                {"term": "binding", "name": "condition", "value": {
                    "term": "reference",
                    "target": placeholder}},
            ]}]},
        }));
        node["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(placeholder.clone());
    }));
    assert_eq!(
        refusal.code,
        CheckedPackageRefusalCode::InvalidSemanticGraph
    );
    assert!(path.ends_with("/body/members/2/value"), "{path}");

    // A fairness member naming `Halted`: `missing_declaration`/`missing-name`.
    let fair = of_form(&wire, "fairness");
    let [fair] = fair[..] else {
        panic!("one fairness node");
    };
    let (refusal, _) = refuse(&edited(
        &wire,
        &idtext(&fair["node_id"]),
        |node| node["body"]["operation"]["member"]["name"] = json!("Halted"),
    ));
    assert_eq!(refusal.code, CheckedPackageRefusalCode::MissingDeclaration);
    assert_eq!(refusal.cause, Some(CheckedPackageRefusalCause::MissingName));
}
