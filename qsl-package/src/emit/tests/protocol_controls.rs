// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1061, TC-1062, TC-1063: the event controls of a protocol's `run` tree
//! (QSpec FR-464 "Controls" and binder levels, FR-466 control layouts;
//! FR-383-AC-7, AC-9, AC-10, AC-14, AC-20). Every shape is emitted by
//! `emit_checked` and read back by IR's strict reader; each mutation's
//! verdict is the reader's, not one the test computes.

use super::admission_corpus::config_version_unit;
use super::owner_model::targets;
use super::population_bound::document;
use super::protocol_graph::{argument, clause, member, target, PROFILES};
use super::protocol_reference::clause_argument;
use super::reachability::{lower_with, Lowered};
use super::*;
use crate::checked_v2::V2ReadRefusal;
use quire_contract_model::{CheckedPackageRefusalCause, CheckedPackageRefusalCode};

/// Read the complete envelope through QSL's I2 dispatch with its domain evidence.
fn read_protocol(wire: &Value) -> Read {
    read_protocol_over(wire, &document())
}

fn read_protocol_over(wire: &Value, document: &[u8]) -> Read {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(document), document)
        .expect("fresh domain evidence");
    read_v2(&jcs(wire), library(), V2ReadLimits::default(), &evidence)
}

fn refused_at(
    wire: &Value,
    code: CheckedPackageRefusalCode,
    cause: Option<CheckedPackageRefusalCause>,
    pointer: &str,
) -> quire_contract_model::CheckedPackageRefusal {
    assert_refused_at(read_protocol(wire), code, cause, pointer)
}

fn assert_refused_at(
    outcome: Read,
    code: CheckedPackageRefusalCode,
    cause: Option<CheckedPackageRefusalCause>,
    pointer: &str,
) -> quire_contract_model::CheckedPackageRefusal {
    let Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) = outcome else {
        panic!("expected {code:?} at {pointer}");
    };
    assert_eq!(refusal.code, code, "{refusal:?}");
    assert_eq!(refusal.cause, cause, "{refusal:?}");
    assert_eq!(
        refusal
            .loci
            .first()
            .and_then(|locus| locus.path.as_ref())
            .map(|path| path.as_str()),
        Some(pointer),
        "{refusal:?}"
    );
    *refusal
}

const MEMBERS: &str = "role Buyer on M::Order;\nrole Seller on M::Person;\n\
    channel Ch from Buyer to Seller carries Boolean ordering unordered \
    delivery at-least-once capacity 4 overflow block;";

fn flow_with(members: &str, run: &str) -> String {
    format!(
        "protocol Flow using v over (input: M::Order) on origin {{\n{members}\nrun {run}\n\
         finish End as (outcome: Boolean) {{ true }};\n}}\n"
    )
}

fn flow(run: &str) -> String {
    flow_with(MEMBERS, run)
}

/// Every event control except `attempt`, `effect` and `await`, which need the
/// ConfigVersion model, in source order.
const RUN: &str = "sequence Main {\n\
    send Placed via Ch as (placed: Boolean) { true };\n\
    receive Got via Ch of Placed as (got: Boolean) { true };\n\
    check Gate using v { true };\n\
    event Done by Buyer as (done: Boolean) { true };\n\
    commit Cm by Buyer as (cm: Boolean) { true };\n\
    fence Barrier ordering seq_cst;\n}";

/// `text` emitted over the `acme/orders` document and read back by IR.
fn emitted(text: &str) -> Lowered {
    let lowered = lower_with(document(), PROFILES, text);
    assert!(
        lowered.ir.is_ok(),
        "{:?}\n{}",
        lowered.ir.err(),
        lowered.text
    );
    assert!(lowered.verified, "{}", lowered.text);
    lowered
}

/// The node declared at `path`, which must be the one node of that
/// declaration.
fn control<'w>(wire: &'w Value, path: &[&str]) -> &'w Value {
    let found: Vec<&Value> = nodes(wire)
        .iter()
        .filter(|node| node["declaration"]["qualified_name"] == json!(path))
        .collect();
    let [node] = found[..] else {
        panic!("one node {path:?}: {found:?}");
    };
    node
}

fn run_control<'w>(wire: &'w Value, name: &str) -> &'w Value {
    control(wire, &["Flow", "run", "Main", name])
}

fn position(wire: &Value, node: &Value) -> usize {
    nodes(wire)
        .iter()
        .position(|candidate| candidate["node_id"] == node["node_id"])
        .expect("a node of the graph")
}

fn operator(node: &Value) -> &str {
    node["body"]["operation"]["member"]["operator"]
        .as_str()
        .expect("a profile operator")
}

fn names(node: &Value) -> Vec<&str> {
    node["body"]["arguments"]
        .as_array()
        .expect("arguments")
        .iter()
        .map(|argument| argument["name"].as_str().expect("a name"))
        .collect()
}

/// The `(name, level)` of a binder `parameter` node.
fn level(binder: &Value) -> (String, String) {
    assert_eq!(binder["semantic_form"], "parameter", "{binder}");
    let text = |name: &str| {
        member(&binder["body"], name)["value"]
            .as_str()
            .expect("a literal")
            .to_owned()
    };
    (text("name"), text("level"))
}

/// The binder an introducing control binds, as `(name, level)`.
fn binder_of(wire: &Value, control: &Value) -> (String, String) {
    let value = &control["body"]["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|argument| argument["name"] == "binder")
        .expect("a binder binding")["value"];
    level(target(wire, value))
}

fn pair(name: &str, level: &str) -> (String, String) {
    (name.to_owned(), level.to_owned())
}

/// Sets `node`'s dependencies to the reference targets of its body, once each
/// in ascending structural-key order (a control depends on what it names).
fn redepend(envelope: &mut Value, position: usize) {
    let node = &mut envelope["semantic_graph"]["nodes"][position];
    let mut found = Vec::new();
    targets(&node["body"], &mut found);
    found.sort_by_key(|value| qsl_foundation::node_wire::node_key_from_json(value).unwrap());
    found.dedup();
    node["dependencies"] = Value::Array(found);
}

/// `lowered` after `edit`; structural keys remain stable.
fn modified(lowered: &Lowered, edit: impl FnOnce(&mut Value)) -> Value {
    let mut envelope = lowered.wire.clone();
    edit(&mut envelope);
    envelope
}

fn children_of(envelope: &mut Value, position: usize) -> &mut Vec<Value> {
    envelope["semantic_graph"]["nodes"][position]["body"]["arguments"][0]["value"]["members"]
        .as_array_mut()
        .expect("children")
}

/// I2 retains each emitted protocol node and its occurrence records.
#[trace("FR-383-AC-19", "TC-1063")]
#[test]
fn protocol_nodes_and_occurrences_survive_the_whole_i2_read() {
    let lowered = emitted(&flow(RUN));
    let Read::Verified { admitted, .. } = read_protocol(&lowered.wire) else {
        panic!("the emitted protocol is admitted");
    };
    let expected = nodes(&lowered.wire)
        .iter()
        .filter(|node| node["node_tag"] == "protocol")
        .collect::<Vec<_>>();
    assert!(!expected.is_empty());
    for node in expected {
        let key = qsl_foundation::node_wire::node_key_from_json(&node["node_id"]).unwrap();
        let typed = admitted
            .graph()
            .nodes
            .iter()
            .find(|candidate| serde_json::to_value(&candidate.node_id).unwrap() == node["node_id"])
            .expect("emitted protocol node retained by I2");
        let retained = serde_json::to_value(typed).unwrap();
        for field in [
            "node_id",
            "node_tag",
            "semantic_form",
            "semantic_type",
            "declaration",
            "body",
            "dependencies",
            "occurrences",
        ] {
            assert_eq!(retained[field], node[field], "{key}: {field}");
        }
    }
    assert_eq!(
        serde_json::to_value(admitted.source_map()).unwrap(),
        lowered.wire["source_map"]
    );
}

/// TC-1063 row 8a: sets are strictly ascending and duplicate-free.
#[trace("FR-383-AC-20", "TC-1063")]
#[test]
fn a_misordered_or_repeated_role_is_refused_by_i2_at_its_normative_locus() {
    let lowered = emitted(&flow(RUN));
    let at = position(&lowered.wire, clause(&lowered.wire));
    for repeat in [false, true] {
        let mut wire = lowered.wire.clone();
        let roles = wire["semantic_graph"]["nodes"][at]["body"]["arguments"][3]["value"]["members"]
            .as_array_mut()
            .unwrap();
        assert_eq!(roles.len(), 2);
        if repeat {
            roles.insert(0, roles[0].clone());
        } else {
            roles.swap(0, 1);
        }
        let at = if repeat {
            position(
                &wire,
                target(
                    &wire,
                    &clause_argument(clause(&wire), "roles")["members"][0],
                ),
            )
        } else {
            at
        };
        refused_at(
            &wire,
            CheckedPackageRefusalCode::InvalidSemanticGraph,
            None,
            &format!("/semantic_graph/nodes/{at}"),
        );
    }
}

/// TC-1063 row 8b: each channel has exactly one queue.
#[trace("FR-383-AC-20", "TC-1063")]
#[test]
fn a_missing_or_second_queue_is_refused_by_i2() {
    let lowered = emitted(&flow_with(&format!("{MEMBERS}\nchannel Other from Seller to Buyer carries Boolean ordering unordered delivery unknown capacity 2 overflow block;"), RUN));
    let clause_at = position(&lowered.wire, clause(&lowered.wire));
    let queues = nodes(&lowered.wire)
        .iter()
        .filter(|node| node["semantic_form"] == "queue")
        .collect::<Vec<_>>();
    assert_eq!(queues.len(), 2);
    for second in [false, true] {
        let mut wire = lowered.wire.clone();
        if second {
            let at = position(&wire, queues[1]);
            wire["semantic_graph"]["nodes"][at]["body"]["arguments"][0]["value"] =
                queues[0]["body"]["arguments"][0]["value"].clone();
            redepend(&mut wire, at);
        } else {
            let key = &queues[1]["node_id"];
            wire["semantic_graph"]["nodes"][clause_at]["body"]["arguments"][6]["value"]["members"]
                .as_array_mut()
                .unwrap()
                .retain(|entry| entry["target"] != *key);
            wire["semantic_graph"]["nodes"]
                .as_array_mut()
                .unwrap()
                .retain(|node| node["node_id"] != *key);
            wire["source_map"]
                .as_array_mut()
                .unwrap()
                .retain(|entry| entry["node_id"] != *key);
        }
        let clause_at = position(&wire, clause(&lowered.wire));
        super::owner_model::depend_on(&mut wire, clause_at, &clause(&lowered.wire)["owner_model"]);
        let expected_channel = if second {
            let mut channels = nodes(&wire)
                .iter()
                .filter(|node| node["semantic_form"] == "channel")
                .collect::<Vec<_>>();
            channels.sort_by_key(|node| {
                qsl_foundation::node_wire::node_key_from_json(&node["node_id"]).unwrap()
            });
            channels[0]
        } else {
            target(&wire, &queues[1]["body"]["arguments"][0]["value"])
        };
        let channel_at = position(&wire, expected_channel);
        refused_at(
            &wire,
            CheckedPackageRefusalCode::InvalidSemanticGraph,
            None,
            &format!("/semantic_graph/nodes/{channel_at}"),
        );
    }
}

/// TC-1063 row 8f: a role's model is a domain object, never a record type.
#[trace("FR-383-AC-20", "TC-1063")]
#[test]
fn a_record_type_as_a_role_model_is_refused_at_the_target() {
    let lowered = emitted(&deciding(&format!("sequence Main {{ {RECEIVED} }}")));
    let record = nodes(&lowered.wire)
        .iter()
        .find(|node| node["node_tag"] == "composite_type" && node["semantic_form"] == "record")
        .expect("the channel carries the Ack record");
    let role_at = position(
        &lowered.wire,
        control(&lowered.wire, &["Flow", "role", "Buyer"]),
    );
    let mut wire = lowered.wire.clone();
    wire["semantic_graph"]["nodes"][role_at]["body"]["arguments"][0]["value"] =
        json!({"term": "reference", "target": record["node_id"]});
    redepend(&mut wire, role_at);
    refused_at(
        &wire,
        CheckedPackageRefusalCode::InvalidModelBinding,
        Some(CheckedPackageRefusalCause::MalformedDeclaration),
        &format!("/semantic_graph/nodes/{role_at}/body/arguments/0/value/target"),
    );
}

/// TC-1063 row 8h: a choice's cases follow label byte order.
#[trace("FR-383-AC-20", "TC-1063")]
#[test]
fn descending_choice_labels_are_refused_by_i2() {
    let lowered = emitted(&flow("sequence Main { choice Pick by Buyer visible () { case a when { true } sequence A { } case z when { false } sequence Z { } } }"));
    let at = position(&lowered.wire, run_control(&lowered.wire, "Pick"));
    let mut wire = lowered.wire.clone();
    wire["semantic_graph"]["nodes"][at]["body"]["arguments"][2]["value"]["members"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    refused_at(
        &wire,
        CheckedPackageRefusalCode::InvalidSemanticGraph,
        None,
        &format!("/semantic_graph/nodes/{at}"),
    );
}

/// FR-383-AC-7, FR-466 (QSpec): `send`, `receive`, `check`, `event`, `commit`
/// and `fence` emit the operator FR-466 names with its arguments in layout
/// order, and the `sequence` lists them in source order; IR's reader admits
/// the package.
#[trace("FR-383-AC-7", "TC-1061")]
#[test]
fn the_event_controls_emit_their_layouts_in_source_order() {
    let lowered = emitted(&flow(RUN));
    let wire = &lowered.wire;
    let layouts: [(&str, &str, &[&str]); 6] = [
        (
            "Placed",
            "send",
            &["channel", "binder", "related", "constraint"],
        ),
        (
            "Got",
            "receive",
            &["channel", "of", "binder", "related", "constraint"],
        ),
        ("Gate", "check", &["condition"]),
        (
            "Done",
            "event",
            &["role", "compensation", "binder", "related", "constraint"],
        ),
        ("Cm", "commit", &["role", "binder", "constraint"]),
        ("Barrier", "fence", &["ordering"]),
    ];
    let main = control(wire, &["Flow", "run", "Main"]);
    assert_eq!(operator(main), "sequence");
    let children: Vec<&Value> = argument(main, &["children"], "children")["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|child| target(wire, child))
        .collect();
    assert_eq!(children.len(), layouts.len());
    for ((name, operator_name, arguments), child) in layouts.into_iter().zip(children) {
        let node = run_control(wire, name);
        assert_eq!(node["node_id"], child["node_id"], "{name} in source order");
        assert_eq!(node["semantic_form"], "control");
        assert_eq!(operator(node), operator_name, "{name}");
        assert_eq!(names(node), arguments, "{name}");
    }

    // The receive names its send, the send and the receive their channel, and
    // the event and the commit their role.
    let placed = run_control(wire, "Placed");
    let got = run_control(wire, "Got");
    let channel = control(wire, &["Flow", "channel", "Ch"]);
    let buyer = control(wire, &["Flow", "role", "Buyer"]);
    let of = &got["body"]["arguments"][1]["value"];
    assert_eq!(target(wire, of)["node_id"], placed["node_id"]);
    for node in [placed, got] {
        assert_eq!(
            target(wire, &node["body"]["arguments"][0]["value"])["node_id"],
            channel["node_id"]
        );
    }
    for name in ["Done", "Cm"] {
        assert_eq!(
            target(
                wire,
                &run_control(wire, name)["body"]["arguments"][0]["value"]
            )["node_id"],
            buyer["node_id"]
        );
    }
    assert_eq!(
        run_control(wire, "Done")["body"]["arguments"][1]["value"],
        json!({"term": "aggregate", "members": []}),
        "an event with no compensation template"
    );
    let ordering = argument(run_control(wire, "Barrier"), &["ordering"], "ordering");
    assert_eq!(ordering["value"], "seq_cst");
}

/// FR-383-AC-10, FR-383-AC-9 (QSpec FR-464 "Binders"): a control's binder
/// level is the size of its entry set; each control is declared at the path
/// of its holder followed by its own name; `check` and `fence` introduce no
/// binder.
#[trace("FR-383-AC-9", "FR-383-AC-10", "TC-1062")]
#[test]
fn a_control_binder_is_levelled_by_its_entry_set_and_declared_by_its_holder() {
    let lowered = emitted(&flow(RUN));
    let wire = &lowered.wire;
    let levels = [
        ("Placed", pair("placed", "1")),
        ("Got", pair("got", "2")),
        ("Done", pair("done", "3")),
        ("Cm", pair("cm", "4")),
    ];
    for (name, expected) in levels {
        assert_eq!(binder_of(wire, run_control(wire, name)), expected, "{name}");
        assert_eq!(
            run_control(wire, name)["declaration"]["qualified_name"],
            json!(["Flow", "run", "Main", name])
        );
    }
    for name in ["Gate", "Barrier"] {
        assert!(!names(run_control(wire, name)).contains(&"binder"));
    }
    // The run root hangs off the clause at its own declaration.
    let run = target(wire, clause_argument(clause(wire), "run"));
    assert_eq!(
        run["declaration"]["qualified_name"],
        json!(["Flow", "run", "Main"])
    );
}

fn controls(wire: &Value) -> Vec<(Value, Value)> {
    nodes(wire)
        .iter()
        .filter(|node| node["semantic_form"] == "control")
        .map(|node| (node["declaration"].clone(), node["node_id"].clone()))
        .collect()
}

/// FR-383-AC-14: reformatting the protocol, adding a comment and a second
/// reference to it leave every control key unchanged.
#[trace("FR-383-AC-14", "TC-1062")]
#[test]
fn control_keys_do_not_move_with_layout_comments_or_a_second_reference() {
    let base = emitted(&flow(RUN));
    let spread = RUN.replace(";\n", ";\n\n   // a comment\n");
    let reformatted = emitted(&flow(&spread));
    assert_eq!(controls(&reformatted.wire), controls(&base.wire));

    let referred = format!(
        "{}function refers using v(): Boolean pure {{ protocol Flow }}\n",
        flow(RUN)
    );
    let referred = emitted(&referred);
    assert_eq!(controls(&referred.wire), controls(&base.wire));
    assert_eq!(
        node_of_clause_key(&referred.wire),
        node_of_clause_key(&base.wire),
        "the protocol clause key"
    );
}

fn node_of_clause_key(wire: &Value) -> Value {
    clause(wire)["node_id"].clone()
}

/// FR-383-AC-20, TC-1063 row 8c: the control `Gate` removed from `Main`'s
/// children but kept as a node is refused by IR's reader at the control.
#[trace("FR-383-AC-20", "TC-1063")]
#[test]
fn a_control_dropped_from_its_holders_children_is_refused_at_the_control() {
    let lowered = emitted(&flow(RUN));
    let wire = &lowered.wire;
    let main = position(wire, control(wire, &["Flow", "run", "Main"]));
    let gate = run_control(wire, "Gate");
    let at = position(wire, gate);
    let mutated = modified(&lowered, |envelope| {
        children_of(envelope, main).retain(|child| child["target"] != gate["node_id"]);
        redepend(envelope, main);
    });
    refused_at(
        &mutated,
        CheckedPackageRefusalCode::InvalidSemanticGraph,
        None,
        &format!("/semantic_graph/nodes/{at}"),
    );
}

/// FR-383-AC-20, TC-1063 row 8d: a `receive` sequenced before the `send` it
/// names is a definition cycle.
#[trace("FR-383-AC-20", "TC-1063")]
#[test]
fn a_receive_sequenced_before_its_send_is_a_definition_cycle() {
    let lowered = emitted(&flow(RUN));
    let wire = &lowered.wire;
    let main = position(wire, control(wire, &["Flow", "run", "Main"]));
    let mutated = modified(&lowered, |envelope| children_of(envelope, main).swap(0, 1));
    let at = position(wire, run_control(wire, "Got"));
    let refusal = refused_at(
        &mutated,
        CheckedPackageRefusalCode::InvalidPackage,
        Some(CheckedPackageRefusalCause::DefinitionCycle),
        &format!("/semantic_graph/nodes/{at}"),
    );
    let Some(quire_contract_model::CheckedProtocolRefusalDetail::CausalCycle { controls }) =
        refusal.protocol_detail.as_deref()
    else {
        panic!("the causal cycle retains its controls: {refusal:?}");
    };
    assert_eq!(
        serde_json::to_value(controls).unwrap(),
        json!([
            run_control(wire, "Got")["node_id"],
            run_control(wire, "Placed")["node_id"]
        ])
    );
}

/// FR-383-AC-20, TC-1063 row 8e: a binding missing from a control, and two
/// of its arguments swapped, are each `malformed_wire` at the body.
#[trace("FR-383-AC-20", "TC-1063")]
#[test]
fn a_missing_or_swapped_control_binding_is_malformed_wire() {
    let mut attempt = admitted_config(
        "protocol Flow using v over (input: Config::ConfigVersion) on origin {\n\
         role R on Config::ConfigVersion;\n\
         run attempt Update by R on Config::ConfigVersion::attemptUpdate \
         contracts [] as (tried: Boolean) { true };\n\
         finish End as (outcome: Boolean) { true };\n}\n",
    );
    let at = position(&attempt, control(&attempt, &["Flow", "run", "Update"]));
    attempt["semantic_graph"]["nodes"][at]["body"]["arguments"]
        .as_array_mut()
        .unwrap()
        .remove(3);
    redepend(&mut attempt, at);
    assert_refused_at(
        read_protocol_over(&attempt, domain_document()),
        CheckedPackageRefusalCode::MalformedWire,
        None,
        &format!("/semantic_graph/nodes/{at}/body"),
    );

    let lowered = emitted(&flow(RUN));
    let wire = &lowered.wire;
    let placed = position(wire, run_control(wire, "Placed"));
    let expected = format!("/semantic_graph/nodes/{placed}/body");
    let swapped = modified(&lowered, |envelope| {
        envelope["semantic_graph"]["nodes"][placed]["body"]["arguments"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
    });
    refused_at(
        &swapped,
        CheckedPackageRefusalCode::MalformedWire,
        None,
        &expected,
    );
}

/// FR-383-AC-20, TC-1063 row 8g: a `check` condition that reads model state
/// (the `over` input) is `ill_typed`/`operator-ineligible` at the read, the `over` binder. The
/// checked source cannot say it (S3 refuses it), so the condition of the
/// emitted `check` is repointed at a Boolean initializer that reads `input`.
#[trace("FR-383-AC-20", "TC-1063")]
#[test]
fn a_check_condition_that_reads_model_state_is_operator_ineligible() {
    let captured = format!(
        "protocol Flow using v over (input: M::Order) on origin {{\n\
         capture seen: Boolean = present(deref(input).trail);\n{MEMBERS}\n\
         run sequence Main {{ check Gate using v {{ seen }}; }}\n\
         finish End as (outcome: Boolean) {{ true }};\n}}\n"
    );
    let lowered = emitted(&captured);
    let wire = &lowered.wire;
    let capture = &clause_argument(clause(wire), "captures")["members"][0];
    let initializer = member(capture, "initializer").clone();
    assert_eq!(initializer["term"], "reference");
    let at = position(wire, run_control(wire, "Gate"));
    let mutated = modified(&lowered, |envelope| {
        envelope["semantic_graph"]["nodes"][at]["body"]["arguments"][0]["value"] = initializer;
        redepend(envelope, at);
    });
    let over = nodes(&mutated)
        .iter()
        .position(|node| node["semantic_form"] == "parameter" && level(node) == pair("input", "0"))
        .expect("the over binder");
    refused_at(
        &mutated,
        CheckedPackageRefusalCode::IllTyped,
        Some(CheckedPackageRefusalCause::OperatorIneligible),
        &format!("/semantic_graph/nodes/{over}"),
    );
}

fn domain_document() -> &'static [u8] {
    config_version_unit::DOMAIN_PACKAGE.as_bytes()
}

/// `declarations` over the ConfigVersion model (one operation,
/// `attemptUpdate`), emitted and read back by IR; returns the wire.
fn admitted_config(declarations: &str) -> Value {
    let packages = qsl_semantics::model::intake::package_input([domain_document()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile p = \"quire.protocol.finite-global/v1\";\n\
         model Config = \"example/config-version\";\n{declarations}\n"
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{unit}\n{refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(
            package_identity(domain_document()),
            domain_document().to_vec(),
        )
        .expect("a fresh evidence key");
    crate::checked_v2::read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the package: {failure:?}"));
    wire
}

/// FR-383-AC-7, FR-466 (QSpec): an `attempt` names the existing no-point
/// operation anchor and the existing state clauses it contracts, and an
/// `effect` the attempt it is the effect of; IR's reader admits the package.
#[trace("FR-383-AC-7", "TC-1061")]
#[test]
fn an_attempt_names_its_operation_anchor_and_contracts_and_an_effect_its_attempt() {
    let wire = admitted_config(
        "post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate { 1 < 2 }\n\
         protocol Flow using v over (input: Config::ConfigVersion) on origin {\n\
         role R on Config::ConfigVersion;\n\
         run sequence Main {\n\
         attempt Update by R on Config::ConfigVersion::attemptUpdate \
         contracts [VersionUnchanged] as (tried: Boolean) { true };\n\
         effect Applied of Update as (applied: Boolean) { true };\n\
         }\n\
         finish End as (outcome: Boolean) { true };\n}\n",
    );
    let update = run_control(&wire, "Update");
    assert_eq!(operator(update), "attempt");
    assert_eq!(
        names(update),
        [
            "role",
            "operation",
            "ordering",
            "contracts",
            "binder",
            "related",
            "constraint"
        ]
    );
    let operation = target(&wire, &update["body"]["arguments"][1]["value"]);
    assert_eq!(operation["semantic_form"], "operation_anchor");
    let contracts = update["body"]["arguments"][3]["value"]["members"]
        .as_array()
        .unwrap();
    let [contract] = contracts.as_slice() else {
        panic!("one contract: {contracts:?}");
    };
    assert_eq!(target(&wire, contract)["semantic_form"], "state_clause");
    assert_eq!(binder_of(&wire, update), pair("tried", "1"));

    let applied = run_control(&wire, "Applied");
    assert_eq!(operator(applied), "effect");
    assert_eq!(names(applied), ["of", "binder", "related", "constraint"]);
    assert_eq!(
        target(&wire, &applied["body"]["arguments"][0]["value"])["node_id"],
        update["node_id"]
    );
    assert_eq!(binder_of(&wire, applied), pair("applied", "2"));
}

/// FR-383-AC-7, FR-466 (QSpec): an `await` anchored at an event node emits
/// its clock text, closed interval, awaited event, `then` and `timeout`, and
/// the lock selects the unit's temporal profile, which an `await` needs for
/// its clock role; IR's reader admits the package.
#[trace("FR-383-AC-7", "TC-1061")]
#[test]
fn an_await_emits_its_anchor_clock_window_event_then_and_timeout() {
    let profiles = format!("{PROFILES}profile t = \"quire.temporal.infinite-trace/v1\";\n");
    let run = "sequence Main {\n\
        event Done by Buyer as (done: Boolean) { true };\n\
        await Wait after Done using t clock \"ticks\" within [0,1] \
        match event Seen by Seller as (seen: Boolean) { true }; \
        then sequence Then { } timeout sequence Late { }\n}";
    let lowered = lower_with(document(), &profiles, &flow(run));
    assert!(
        lowered.ir.is_ok(),
        "{:?}\n{}",
        lowered.ir.err(),
        lowered.text
    );
    assert!(lowered.verified, "{}", lowered.text);
    let wire = &lowered.wire;
    let wait = run_control(wire, "Wait");
    assert_eq!(operator(wait), "await");
    assert_eq!(
        names(wait),
        ["after", "clock", "within", "event", "then", "timeout"]
    );
    let after = &wait["body"]["arguments"][0]["value"];
    assert_eq!(member(after, "kind")["value"], "event");
    assert_eq!(
        target(wire, member(after, "target"))["node_id"],
        run_control(wire, "Done")["node_id"]
    );
    assert_eq!(wait["body"]["arguments"][1]["value"]["value"], "ticks");
    let within = &wait["body"]["arguments"][2]["value"];
    assert_eq!(member(within, "lower")["value"], "0");
    assert_eq!(member(within, "upper")["value"], "1");
    let seen = target(wire, &wait["body"]["arguments"][3]["value"]);
    assert_eq!(operator(seen), "event");
    for (index, name) in [(4, "sequence"), (5, "sequence")] {
        let branch = target(wire, &wait["body"]["arguments"][index]["value"]);
        assert_eq!(operator(branch), name);
    }
    let selected = wire["lock"]["profile_selections"].as_array().unwrap();
    assert!(
        selected.contains(&json!({
            "role": "temporal_profile",
            "definition": {
                "authority": "agent-ix",
                "identity": "quire.temporal.infinite-trace/v1"
            }
        })),
        "{selected:?}"
    );
}

/// The `Main` sequence of an `event` and an `await` after it, whose `then`
/// and `timeout` each hold an `event` with a binder, `using` the profile
/// alias `using`.
fn awaiting(using: &str) -> String {
    format!(
        "sequence Main {{\n\
         event Done by Buyer as (done: Boolean) {{ true }};\n\
         await Wait after Done using {using} clock \"ticks\" within [0,1] \
         match event Seen by Seller as (seen: Boolean) {{ true }}; \
         then sequence Then {{ event After by Buyer as (later: Boolean) {{ true }}; }} \
         timeout sequence Late {{ event Gave by Buyer as (gave: Boolean) {{ true }}; }}\n}}"
    )
}

fn lock_selections(wire: &Value) -> Vec<Value> {
    wire["lock"]["profile_selections"]
        .as_array()
        .expect("selections")
        .clone()
}

fn temporal_rows(wire: &Value) -> Vec<Value> {
    lock_selections(wire)
        .into_iter()
        .filter(|row| row["role"] == "temporal_profile")
        .collect()
}

/// FR-464 "Profiles and laws": an `await` in a protocol that emits no node
/// (no protocol profile is selected) leaves the lock, and so the package,
/// unchanged by the unit's temporal profile.
#[trace("FR-383-AC-7", "TC-1061")]
#[test]
fn an_await_without_a_protocol_profile_leaves_the_lock_unchanged() {
    let profiles = "profile t = \"quire.temporal.infinite-trace/v1\";\n";
    let with = lower_with(document(), profiles, &flow(&awaiting("t")));
    assert!(with.ir.is_ok(), "{:?}\n{}", with.ir.err(), with.text);
    assert!(
        controls(&with.wire).is_empty(),
        "no control node is emitted"
    );
    assert!(lock_selections(&with.wire).is_empty());
}

/// FR-464 "Profiles and laws", FR-322: an emitted `await` selects exactly
/// the one temporal profile its `using` alias names, though the header
/// selects two.
#[trace("FR-383-AC-7", "TC-1061")]
#[test]
fn an_emitted_await_selects_exactly_one_temporal_profile() {
    let profiles = format!(
        "{PROFILES}profile t = \"quire.temporal.infinite-trace/v1\";\n\
         profile u = \"quire.temporal.timestamped-event.finite-window/v1\";\n"
    );
    for (alias, identity) in [
        ("t", "quire.temporal.infinite-trace/v1"),
        ("u", "quire.temporal.timestamped-event.finite-window/v1"),
    ] {
        let lowered = lower_with(document(), &profiles, &flow(&awaiting(alias)));
        assert!(lowered.verified, "{}", lowered.text);
        assert_eq!(
            temporal_rows(&lowered.wire),
            [json!({
                "role": "temporal_profile",
                "definition": {"authority": "agent-ix", "identity": identity}
            })],
            "{alias}"
        );
    }
}

/// FR-466 "Binder scope" (QSpec): the binder levels along an `await` are the
/// sizes of each entry set: the input is 0, `done` 1, the matched event's
/// binder 2, a binder in `then` 3 and one in `timeout` 2.
#[trace("FR-383-AC-9", "TC-1062")]
#[test]
fn await_then_and_timeout_binders_are_levelled_by_their_entry_sets() {
    let profiles = format!("{PROFILES}profile t = \"quire.temporal.infinite-trace/v1\";\n");
    let lowered = lower_with(document(), &profiles, &flow(&awaiting("t")));
    assert!(lowered.verified, "{}", lowered.text);
    let wire = &lowered.wire;
    let named = |name: &str| -> &Value {
        let found: Vec<&Value> = nodes(wire)
            .iter()
            .filter(|node| {
                node["semantic_form"] == "control"
                    && node["declaration"]["qualified_name"]
                        .as_array()
                        .and_then(|path| path.last())
                        == Some(&json!(name))
            })
            .collect();
        let [node] = found[..] else {
            panic!("one control {name}: {found:?}");
        };
        node
    };
    for (name, expected) in [
        ("Done", pair("done", "1")),
        ("Seen", pair("seen", "2")),
        ("After", pair("later", "3")),
        ("Gave", pair("gave", "2")),
    ] {
        assert_eq!(binder_of(wire, named(name)), expected, "{name}");
    }
}

/// FR-466 (QSpec): a `fence` emits each of its orderings, not only
/// `seq_cst`.
#[trace("FR-383-AC-7", "TC-1061")]
#[test]
fn a_fence_emits_each_of_its_orderings() {
    for ordering in ["acquire", "release", "acq_rel", "seq_cst"] {
        let lowered = emitted(&flow(&RUN.replace("seq_cst", ordering)));
        let barrier = run_control(&lowered.wire, "Barrier");
        assert_eq!(operator(barrier), "fence");
        assert_eq!(
            argument(barrier, &["ordering"], "ordering")["value"],
            ordering
        );
    }
}

/// FR-466 (QSpec): an `attempt`'s `ordering` group is written only for an
/// attempt the checker admits an ordering on. MA-1 shares no place outside a
/// `parallel`, so an attempt in a `sequence` is `Local` and an ordering on it
/// is refused at S3; the `Some` branch of the group is reachable once
/// `parallel` is written (QSL-1126 c2), and is exercised there.
#[trace("FR-383-AC-7", "TC-1061")]
#[test]
fn an_attempt_ordering_outside_a_parallel_is_refused_so_the_group_is_empty() {
    let packages = qsl_semantics::model::intake::package_input([domain_document()]).unwrap();
    let unit = String::from(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile p = \"quire.protocol.finite-global/v1\";\n\
         model Config = \"example/config-version\";\n\
         protocol Flow using v over (input: Config::ConfigVersion) on origin {\n\
         role R on Config::ConfigVersion;\n\
         run attempt Update by R on Config::ConfigVersion::attemptUpdate \
         ordering seq_cst contracts [] as (tried: Boolean) { true };\n\
         finish End as (outcome: Boolean) { true };\n}\n",
    );
    let refusals = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect_err("an ordering on a local attempt is refused");
    let text = format!("{refusals:?}");
    assert!(
        text.contains("AccessOrdering") && text.contains("Local"),
        "{text}"
    );
    let unordered = admitted_config(
        "protocol Flow using v over (input: Config::ConfigVersion) on origin {\n\
         role R on Config::ConfigVersion;\n\
         run attempt Update by R on Config::ConfigVersion::attemptUpdate \
         contracts [] as (tried: Boolean) { true };\n\
         finish End as (outcome: Boolean) { true };\n}\n",
    );
    assert_eq!(
        control(&unordered, &["Flow", "run", "Update"])["body"]["arguments"][2]["value"],
        json!({"term": "aggregate", "members": []})
    );
}

/// FR-383-AC-12 (QSpec): an `attempt`'s `contracts` are its state clauses'
/// keys in digest order, whatever order the source lists them in.
#[trace("FR-383-AC-12", "TC-1061")]
#[test]
fn two_contracts_are_listed_in_digest_order_whatever_the_source_order() {
    let contracts = |listed: &str| -> Vec<Value> {
        let wire = admitted_config(&format!(
            "post Alpha using v on Config::ConfigVersion::attemptUpdate {{ 1 < 2 }}\n\
             post Beta using v on Config::ConfigVersion::attemptUpdate {{ 2 < 3 }}\n\
             protocol Flow using v over (input: Config::ConfigVersion) on origin {{\n\
             role R on Config::ConfigVersion;\n\
             run attempt Update by R on Config::ConfigVersion::attemptUpdate \
             contracts [{listed}] as (tried: Boolean) {{ true }};\n\
             finish End as (outcome: Boolean) {{ true }};\n}}\n"
        ));
        control(&wire, &["Flow", "run", "Update"])["body"]["arguments"][3]["value"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|member| member["target"].clone())
            .collect()
    };
    let forward = contracts("Alpha, Beta");
    assert_eq!(forward.len(), 2);
    let keys: Vec<NodeKey> = forward
        .iter()
        .map(|id| qsl_foundation::node_wire::node_key_from_json(id).unwrap())
        .collect();
    assert!(keys[0] < keys[1], "{keys:?}");
    assert_eq!(forward, contracts("Beta, Alpha"));
}

/// A `send` and the `receive` of it, whose `got` record binder the
/// `Seller`, the channel's `to` role, observes field by field.
const RECEIVED: &str = "send Placed via Ch as (placed: Ack) { true };\n\
    receive Got via Ch of Placed as (got: Ack) { true };\n";

/// [`flow`] whose channel carries the record `Ack`, the decision facts'
/// record.
fn deciding(run: &str) -> String {
    format!(
        "record Ack {{ ok: Boolean; }}\n{}",
        flow_with(&MEMBERS.replace("carries Boolean", "carries Ack"), run)
    )
}

const CHOICE: [&str; 3] = ["owner", "visible", "cases"];
const REPEAT: [&str; 7] = [
    "owner",
    "visible",
    "maximum",
    "proof",
    "guard",
    "body",
    "exhausted",
];

fn references(wire: &Value, set: &Value) -> Vec<Value> {
    set["members"]
        .as_array()
        .expect("a set")
        .iter()
        .map(|entry| target(wire, entry)["node_id"].clone())
        .collect()
}

/// FR-466 "Control layouts" (FR-464-AC-1, AC-2, AC-4): a `choice` writes its
/// owning role, its `visible` entries as nodes and its `cases` sorted by
/// label as UTF-8 bytes, each with its guard and its body's control; a case
/// body is declared at the `choice`'s declaration and its own name (the case
/// label is no segment); a guard that reads a listed fact names that
/// entry's node; and IR's reader admits the package.
#[trace("FR-464-AC-1", "FR-464-AC-2", "FR-464-AC-4", "TC-1021")]
#[test]
fn a_choice_emits_its_owner_facts_and_sorted_cases_and_reads_back() {
    let lowered = emitted(&deciding(&format!(
        "sequence Main {{\n{RECEIVED}\
         choice Pick by Seller visible (got.ok) {{ \
         case yes when {{ got.ok }} sequence Yes {{ check Fine using v {{ true }}; }} \
         case no when {{ not got.ok }} sequence No {{ }} }}\n}}"
    )));
    let wire = &lowered.wire;
    let pick = run_control(wire, "Pick");
    assert_eq!(operator(pick), "choice");
    assert_eq!(names(pick), CHOICE);
    let seller = control(wire, &["Flow", "role", "Seller"]);
    assert_eq!(
        target(wire, argument(pick, &CHOICE, "owner"))["node_id"],
        seller["node_id"]
    );
    let got = binder_of(wire, run_control(wire, "Got"));
    let visible = argument(pick, &CHOICE, "visible");
    let [fact] = &references(wire, visible)[..] else {
        panic!("one visible entry: {visible}");
    };
    // The fact projects `ok` of the receive's binder.
    let projection = target(wire, &visible["members"][0]);
    assert_eq!(
        projection["body"]["operation"]["identity"],
        "quire.op.record.project"
    );
    assert_eq!(
        level(target(wire, &projection["body"]["arguments"][0])),
        got
    );

    let cases = argument(pick, &CHOICE, "cases")["members"]
        .as_array()
        .expect("case records");
    let labels: Vec<&str> = cases
        .iter()
        .map(|case| member(case, "label")["value"].as_str().expect("a label"))
        .collect();
    assert_eq!(labels, ["no", "yes"]);
    let no = control(wire, &["Flow", "run", "Main", "Pick", "No"]);
    let yes = control(wire, &["Flow", "run", "Main", "Pick", "Yes"]);
    control(wire, &["Flow", "run", "Main", "Pick", "Yes", "Fine"]);
    assert_eq!(member(&cases[0], "body")["target"], no["node_id"]);
    assert_eq!(member(&cases[1], "body")["target"], yes["node_id"]);
    // `got` is the listed fact itself; `not got` reads it.
    assert_eq!(member(&cases[1], "guard")["target"], *fact);
    let negated = target(wire, member(&cases[0], "guard"));
    assert_eq!(negated["dependencies"], json!([fact]));
}

/// FR-466 "Control layouts" (FR-464-AC-1, AC-2, AC-4): a `repeat` writes its
/// owning role, its `visible` entries, the written `maximum`, its `proof`
/// Group, its guard, its body and the `exhausted` control present exactly
/// with a maximum; unwritten optionals are the empty Group and an absent
/// proof the empty Group; both packages read back.
#[trace("FR-464-AC-1", "FR-464-AC-2", "FR-464-AC-4", "TC-1021")]
#[test]
fn a_repeat_emits_its_bound_proof_guard_body_and_exhausted_and_reads_back() {
    let bounded = emitted(&deciding(&format!(
        "sequence Main {{\n{RECEIVED}\
         repeat Loop by Seller visible (got.ok) max 2 invariant {{ true }} variant {{ 1 }} \
         while {{ got.ok }} sequence Body {{ event Tick by Seller as (tick: Boolean) {{ true }}; }} \
         exhausted sequence Spent {{ }}\n}}"
    )));
    let wire = &bounded.wire;
    let repeat = run_control(wire, "Loop");
    assert_eq!(operator(repeat), "repeat");
    assert_eq!(names(repeat), REPEAT);
    let seller = control(wire, &["Flow", "role", "Seller"]);
    assert_eq!(
        target(wire, argument(repeat, &REPEAT, "owner"))["node_id"],
        seller["node_id"]
    );
    let facts = references(wire, argument(repeat, &REPEAT, "visible"));
    assert_eq!(facts.len(), 1);
    assert_eq!(argument(repeat, &REPEAT, "guard")["target"], facts[0]);
    assert_eq!(
        argument(repeat, &REPEAT, "maximum")["members"][0]["value"],
        "2"
    );
    let proof = argument(repeat, &REPEAT, "proof");
    assert_eq!(
        proof["members"]
            .as_array()
            .expect("proof bindings")
            .iter()
            .map(|binding| binding["name"].as_str().expect("a name"))
            .collect::<Vec<_>>(),
        ["invariant", "variant"]
    );
    let body = control(wire, &["Flow", "run", "Main", "Loop", "Body"]);
    let spent = control(wire, &["Flow", "run", "Main", "Loop", "Spent"]);
    control(wire, &["Flow", "run", "Main", "Loop", "Body", "Tick"]);
    assert_eq!(argument(repeat, &REPEAT, "body")["target"], body["node_id"]);
    assert_eq!(
        argument(repeat, &REPEAT, "exhausted")["members"],
        json!([{"term": "reference", "target": spent["node_id"]}])
    );

    let open = emitted(&flow(
        "sequence Main { repeat Loop by Buyer visible () while { true } \
         sequence Body { event Tick by Buyer as (tick: Boolean) { true }; } }",
    ));
    let repeat = run_control(&open.wire, "Loop");
    for empty in ["visible", "maximum", "proof", "exhausted"] {
        assert_eq!(
            argument(repeat, &REPEAT, empty)["members"],
            json!([]),
            "{empty}"
        );
    }
}

/// A `repeat` as a case's body and a `choice` as a `repeat`'s body read
/// back, each declared at its holder's declaration and its own name (the
/// case label is no segment).
#[trace("FR-464-AC-4", "TC-1021")]
#[test]
fn choices_and_repeats_nest_in_each_other_and_read_back() {
    let lowered = emitted(&flow(
        "sequence Main { choice Pick by Buyer visible () { \
         case a when { true } repeat Inner by Buyer visible () max 1 while { true } \
         sequence Step { event Tick by Buyer as (tick: Boolean) { true }; } \
         exhausted sequence Spent { } \
         case b when { false } sequence Other { } } \
         repeat Loop by Buyer visible () while { true } \
         choice Each by Buyer visible () { \
         case x when { true } sequence X { event Ex by Buyer as (ex: Boolean) { true }; } \
         case y when { false } sequence Y { event Why by Buyer as (why: Boolean) { true }; } } }",
    ));
    let wire = &lowered.wire;
    let pick = run_control(wire, "Pick");
    let inner = control(wire, &["Flow", "run", "Main", "Pick", "Inner"]);
    control(
        wire,
        &["Flow", "run", "Main", "Pick", "Inner", "Step", "Tick"],
    );
    let cases = argument(pick, &CHOICE, "cases")["members"]
        .as_array()
        .expect("case records");
    assert_eq!(member(&cases[0], "body")["target"], inner["node_id"]);
    let each = control(wire, &["Flow", "run", "Main", "Loop", "Each"]);
    control(wire, &["Flow", "run", "Main", "Loop", "Each", "X", "Ex"]);
    assert_eq!(
        argument(run_control(wire, "Loop"), &REPEAT, "body")["target"],
        each["node_id"]
    );
}

/// A post-test `repeat`, whose guard reads a binder its body declares, has
/// no FR-466 node form (the guard reads the repeat's entry only); nor has a
/// literal or bare-binder `visible` entry (FR-466 lists `expression`
/// nodes), nor a repeat whose `visible`
/// entry, invariant or variant reads a body binder; a guard that refers to
/// a protocol directly is not written yet either; QSpec #481 (STD-539)
/// settles these as non-encodings. A repeat that steps only through a
/// nested repeat has a node form (FR-464 "Nested `repeat` rule") but is
/// withheld until IR's reader decides that rule (IR-1100). Each refuses the protocol
/// `unsupported_construct`/`not-yet-implemented` naming the construct.
#[trace("FR-092-AC-18", "TC-1021")]
#[test]
fn decisions_without_a_node_form_are_not_written() {
    use super::protocol_graph::checked;
    use super::protocol_reference::not_emitted;
    use qsl_semantics::check::ProtocolConstruct;
    let cases = [
        (
            ProtocolConstruct::PostTestRepeat,
            "STD-539",
            deciding(
                "sequence Main { repeat Loop by Seller visible (got.ok) while { not got.ok } \
                 sequence Body { send Placed via Ch as (placed: Ack) { true }; \
                 receive Got via Ch of Placed as (got: Ack) { true }; } }",
            ),
        ),
        (
            ProtocolConstruct::VisibleEntryNotAnExpression,
            "STD-539",
            flow(
                "sequence Main { event E by Buyer as (e: Boolean) { true }; \
                 repeat Loop by Buyer visible (e) while { e } \
                 sequence Body { event Tick by Buyer as (tick: Boolean) { true }; } }",
            ),
        ),
        (
            ProtocolConstruct::RepeatDecisionReadsBody,
            "STD-539",
            deciding(
                "sequence Main { repeat Loop by Seller visible (got.ok) while { true } \
                 sequence Body { send Placed via Ch as (placed: Ack) { true }; \
                 receive Got via Ch of Placed as (got: Ack) { true }; } }",
            ),
        ),
        (
            ProtocolConstruct::RepeatDecisionReadsBody,
            "STD-539",
            deciding(
                "sequence Main { repeat Loop by Seller visible () invariant { got.ok } \
                 variant { 1 } while { true } sequence Body { send Placed via Ch as (placed: Ack) { true }; \
                 receive Got via Ch of Placed as (got: Ack) { true }; } }",
            ),
        ),
        (
            ProtocolConstruct::RepeatDecisionReadsBody,
            "STD-539",
            deciding(
                "sequence Main { repeat Loop by Seller visible () invariant { true } \
                 variant { if got.ok then 1 else 0 } while { true } sequence Body { send Placed via Ch as (placed: Ack) { true }; \
                 receive Got via Ch of Placed as (got: Ack) { true }; } }",
            ),
        ),
        (
            ProtocolConstruct::RepeatStepsThroughRepeat,
            "IR-1100",
            flow(
                "sequence Main { repeat Loop by Buyer visible () while { true } \
                 choice Pick by Buyer visible () { \
                 case a when { true } sequence A { event Ea by Buyer as (ea: Boolean) { true }; } \
                 case b when { false } repeat Inner by Buyer visible () while { true } \
                 sequence S { event Eb by Buyer as (eb: Boolean) { true }; } } }",
            ),
        ),
        (
            ProtocolConstruct::RepeatStepsThroughRepeat,
            "IR-1100",
            flow(
                "sequence Main { repeat Loop by Buyer visible () while { true } \
                 repeat Inner by Buyer visible () while { true } \
                 sequence Step { event Tick by Buyer as (tick: Boolean) { true }; } }",
            ),
        ),
        (
            ProtocolConstruct::VisibleEntryNotAnExpression,
            "STD-539",
            flow(
                "sequence Main { repeat Loop by Buyer visible (true) while { true } \
                 sequence Body { event Tick by Buyer as (tick: Boolean) { true }; } }",
            ),
        ),
        (
            ProtocolConstruct::ProtocolReferenceInBody,
            "QSL-1121g",
            format!(
                "{}protocol Other using v over (input: M::Order) on origin {{\n\
                 role Buyer on M::Order;\nrun sequence Main {{ }}\n\
                 finish End as (outcome: Boolean) {{ true }};\n}}\n",
                flow(
                    "sequence Main { repeat Loop by Buyer visible () while { protocol Other } \
                     sequence Body { event Tick by Buyer as (tick: Boolean) { true }; } }",
                )
            ),
        ),
    ];
    for (construct, ticket, text) in cases {
        assert_eq!(construct.tracking(), ticket);
        let refusals = checked(&text, false).expect_err(&text);
        assert_eq!(
            refusals
                .iter()
                .map(|refusal| &refusal.cause)
                .collect::<Vec<_>>(),
            [&not_emitted(construct)],
            "{text}"
        );
    }
}

/// A decision that calls a function is written once the function's node is
/// (QSL-1131): the guard names the call's node, and IR's reader admits the
/// package.
#[trace("FR-464-AC-1", "TC-1021")]
#[test]
fn a_decision_that_calls_a_function_reads_back() {
    emitted(&format!(
        "function ok using v(): Boolean pure {{ true }}\n{}",
        flow(
            "sequence Main { repeat Loop by Buyer visible () max 1 while { ok() } \
             sequence Body { event Tick by Buyer as (tick: Boolean) { true }; } \
             exhausted sequence Out { } }",
        )
    ));
}
