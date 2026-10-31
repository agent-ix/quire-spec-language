// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-383-AC-2, AC-13: a protocol's requirements set names the emitted
//! temporal clause nodes, deduplicated and sorted by canonical node identity.

use qsl_semantics::check::CheckCause;
use quire_semantic_value::checking::CheckingLimits;

use super::owner_model::verdict;
use super::protocol_graph::{flow_over, model};
use super::protocol_reference::{clause_argument, node_by_id, node_of};
use super::reachability::{lower_with, Lowered};
use super::*;

const PROFILES: &str = "profile p = \"quire.protocol.finite-global/v1\";\n\
                     profile t = \"quire.temporal.infinite-trace/v1\";\n";

#[trace("FR-383-AC-2", "FR-383-AC-13", "TC-1061", "TC-1062")]
#[test]
fn protocol_requirements_reference_deduplicated_temporal_clause_nodes() {
    let temporal = "temporal Due using t over (c: M::Order) clock \"model-steps\" \
                    on origin { always true }";
    let protocol = flow_over(
        "M::Order",
        "on origin",
        "role Buyer on M::Order;\nrequires temporal Due;\nrequires temporal Due;",
    );
    let unit = format!("{temporal}\n{protocol}");
    let lowered = lower_with(model(), PROFILES, &unit);
    assert!(
        lowered.verified,
        "requirements package should emit and read back: {}",
        lowered.text
    );

    let clause = node_of(&lowered.wire, "protocol", "protocol_clause");
    let requirements = clause_argument(clause, "requirements");
    assert_eq!(requirements["members"].as_array().unwrap().len(), 1);
    let target = node_by_id(&lowered.wire, &requirements["members"][0]["target"]);
    assert_eq!(target["node_tag"], "temporal");
    assert_eq!(target["semantic_form"], "temporal_clause");
    assert!(clause["dependencies"]
        .as_array()
        .unwrap()
        .contains(&target["node_id"]));
    let rows = lowered
        .ir
        .as_ref()
        .expect("the checked package")
        .protocol_activation_relation();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        serde_json::to_value(&rows[0].clause).unwrap(),
        clause["node_id"]
    );
    assert_eq!(
        serde_json::to_value(&rows[0].required_temporal).unwrap(),
        target["node_id"]
    );
    assert_eq!(
        rows[0].activation,
        quire_contract_model::CheckedProtocolActivation::Origin
    );
}

fn protocol_clause_index(wire: &Value) -> usize {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node["semantic_form"] == "protocol_clause")
        .expect("one protocol clause")
}

/// The reader's verdict on `valid` after `edit` to its clause node, with the
/// clause's structural node key retained, so the refusal is the reader's
/// rule on the edited content.
fn clause_edit_verdict(valid: &Lowered, edit: impl FnOnce(&mut Value)) -> String {
    let mut wire = valid.wire.clone();
    let index = protocol_clause_index(&wire);
    edit(&mut wire["semantic_graph"]["nodes"][index]);
    verdict(valid, &wire)
}

fn requirements_members(clause: &mut Value) -> &mut Vec<Value> {
    clause["body"]["arguments"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|argument| argument["name"] == "requirements")
        .unwrap()["value"]["members"]
        .as_array_mut()
        .unwrap()
}

fn sort_dependencies(clause: &mut Value) {
    clause["dependencies"]
        .as_array_mut()
        .unwrap()
        .sort_by_key(|value| qsl_foundation::node_wire::node_key_from_json(value).unwrap());
}

/// FR-383-AC-2, AC-13: each mutated clause is refused by the reader's own
/// requirements-set rule; the unmutated wire is admitted.
#[trace("FR-383-AC-2", "FR-383-AC-13", "TC-1062")]
#[test]
fn requirement_wire_mutations_refuse() {
    let declarations = "temporal Due using t over (c: M::Order) clock \"model-steps\" \
                        on origin { always true }\n\
                        temporal Later using t over (c: M::Order) clock \"model-steps\" \
                        on origin { always eventually true }";
    let protocol = flow_over(
        "M::Order",
        "on origin",
        "role Buyer on M::Order;\nrequires temporal Due;\nrequires temporal Later;",
    );
    let valid = lower_with(model(), PROFILES, &format!("{declarations}\n{protocol}"));
    assert!(valid.verified, "{}", valid.text);
    let clause_index = protocol_clause_index(&valid.wire);
    let clause = &valid.wire["semantic_graph"]["nodes"][clause_index];
    let requirements = clause_argument(clause, "requirements");
    assert_eq!(requirements["members"].as_array().unwrap().len(), 2);
    let original_target = requirements["members"][0]["target"].clone();

    assert_eq!(clause_edit_verdict(&valid, |_| {}), "admitted");

    let noncanonical = clause_edit_verdict(&valid, |clause| {
        requirements_members(clause).swap(0, 1);
    });
    let missing_dependency = clause_edit_verdict(&valid, |clause| {
        clause["dependencies"]
            .as_array_mut()
            .unwrap()
            .retain(|dependency| dependency != &original_target);
    });
    let non_temporal = valid.wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| {
            node["semantic_form"] != "temporal_clause"
                && node["node_id"] != clause["node_id"]
                && !clause["dependencies"]
                    .as_array()
                    .unwrap()
                    .contains(&node["node_id"])
        })
        .expect("a non-temporal node")["node_id"]
        .clone();
    let retarget = |replacement: Value| {
        let original = original_target.clone();
        move |clause: &mut Value| {
            requirements_members(clause)[0]["target"] = replacement.clone();
            let dependencies = clause["dependencies"].as_array_mut().unwrap();
            dependencies.retain(|dependency| dependency != &original);
            dependencies.push(replacement);
            sort_dependencies(clause);
        }
    };
    let wrong_form = clause_edit_verdict(&valid, retarget(non_temporal));
    let mut unknown = original_target.clone();
    unknown["path"] = json!(["absent"]);
    let unknown_key = clause_edit_verdict(&valid, retarget(unknown));

    let clause_path = format!("/semantic_graph/nodes/{clause_index}");
    let member_target = format!("{clause_path}/body/arguments/7/value/members/0/target");
    // A set out of canonical order, and a clause that drops the dependency
    // its body names, are the reader's graph rules on the clause node.
    assert_eq!(
        noncanonical,
        format!("InvalidSemanticGraph at {clause_path} (None)")
    );
    assert_eq!(
        missing_dependency,
        format!("InvalidSemanticGraph at {clause_path}/dependencies (None)")
    );
    // The requirements member must be a temporal clause node of the graph:
    // a node of another form is a malformed declaration at the member, and
    // a key no node carries is a missing name at the member.
    assert_eq!(
        wrong_form,
        format!("InvalidModelBinding at {member_target} (Some(MalformedDeclaration))")
    );
    assert_eq!(
        unknown_key,
        format!("MissingDeclaration at {member_target} (Some(MissingName))")
    );
}

#[trace("FR-383-AC-13", "TC-1062")]
#[test]
fn requirement_order_does_not_change_the_emitted_set() {
    let declarations = "temporal Due using t over (c: M::Order) clock \"model-steps\" \
                        on origin { always true }\n\
                        temporal Later using t over (c: M::Order) clock \"model-steps\" \
                        on origin { always eventually true }";
    let protocol = |requirements: &str| {
        flow_over(
            "M::Order",
            "on origin",
            &format!("role Buyer on M::Order;\n{requirements}"),
        )
    };
    let compile = |requirements: &str| {
        lower_with(
            model(),
            PROFILES,
            &format!("{declarations}\n{}", protocol(requirements)),
        )
    };
    let first = compile("requires temporal Due;\nrequires temporal Later;");
    let second = compile("requires temporal Later;\nrequires temporal Due;");
    assert!(first.verified, "{}", first.text);
    assert!(second.verified, "{}", second.text);
    let clause = node_of(&first.wire, "protocol", "protocol_clause");
    let requirements = clause_argument(clause, "requirements")["members"]
        .as_array()
        .unwrap();
    let keys: Vec<_> = requirements
        .iter()
        .map(|reference| {
            qsl_foundation::node_wire::node_key_from_json(&reference["target"]).unwrap()
        })
        .collect();
    assert_eq!(requirements.len(), 2);
    // The expected order comes from the wire's own temporal nodes, sorted
    // here, not from the order lowering or the source wrote them in.
    let mut temporal_keys: Vec<NodeKey> = first.wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["semantic_form"] == "temporal_clause")
        .map(|node| qsl_foundation::node_wire::node_key_from_json(&node["node_id"]).unwrap())
        .collect();
    temporal_keys.sort_unstable();
    assert_eq!(temporal_keys.len(), 2);
    assert_eq!(keys, temporal_keys);
    let second_clause = node_of(&second.wire, "protocol", "protocol_clause");
    assert_eq!(
        clause_argument(second_clause, "requirements"),
        clause_argument(clause, "requirements"),
        "the reversed source order emits the same set"
    );
    for requirement in requirements {
        let target = node_by_id(&first.wire, &requirement["target"]);
        assert!(clause["dependencies"]
            .as_array()
            .unwrap()
            .contains(&target["node_id"]));
    }
    let rows = first
        .ir
        .as_ref()
        .expect("the checked package")
        .protocol_activation_relation();
    assert_eq!(rows.len(), 2);
    assert!(rows
        .windows(2)
        .all(|pair| pair[0].required_temporal < pair[1].required_temporal));
}

/// FR-093: a function that reads a protocol clause is keyed after the
/// temporal clause required by that protocol, and the clause node carries
/// that required temporal node as a dependency.
#[trace("FR-093-AC-4", "FR-218-AC-16", "FR-383-AC-13", "TC-1062")]
#[test]
fn a_function_can_reference_a_protocol_with_a_temporal_requirement() {
    let unit = format!(
        "function refers using v(): Boolean pure {{ protocol Flow }}\n\
         temporal Due using t over (c: M::Order) clock \"model-steps\" \
         on origin {{ always true }}\n\
         {}",
        flow_over(
            "M::Order",
            "on origin",
            "role Buyer on M::Order;\nrequires temporal Due;",
        )
    );
    let lowered = lower_with(model(), PROFILES, &unit);
    assert!(lowered.verified, "{}", lowered.text);

    let clause = node_of(&lowered.wire, "protocol", "protocol_clause");
    let due = node_of(&lowered.wire, "temporal", "temporal_clause");
    assert!(clause["dependencies"]
        .as_array()
        .unwrap()
        .contains(&due["node_id"]));

    let reference = node_of(&lowered.wire, "protocol", "protocol_reference");
    assert_eq!(reference["dependencies"], json!([clause["node_id"]]));
}

/// FR-218-AC-17, FR-383-AC-13: an unrequired temporal clause contributes no
/// activation relation dependency, while adding it to the requirement set
/// changes the protocol clause identity.
#[trace("FR-218-AC-17", "FR-383-AC-13", "TC-1062")]
#[test]
fn a_protocol_requirement_changes_the_clause_key_and_empty_set_has_no_row() {
    let temporal = "temporal Due using t over (c: M::Order) clock \"model-steps\" \
                    on origin { always true }";
    let protocol = |members: &str| {
        flow_over(
            "M::Order",
            "on origin",
            &format!("role Buyer on M::Order;\n{members}"),
        )
    };
    let compile = |members: &str| {
        lower_with(
            model(),
            PROFILES,
            &format!("{temporal}\n{}", protocol(members)),
        )
    };
    let empty = compile("");
    let required = compile("requires temporal Due;");
    assert!(empty.verified, "{}", empty.text);
    assert!(required.verified, "{}", required.text);

    let empty_clause = node_of(&empty.wire, "protocol", "protocol_clause");
    let required_clause = node_of(&required.wire, "protocol", "protocol_clause");
    assert_eq!(
        clause_argument(empty_clause, "requirements")["members"],
        json!([])
    );
    assert!(empty
        .ir
        .as_ref()
        .expect("the checked package")
        .protocol_activation_relation()
        .is_empty());
    assert!(!empty_clause["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|key| { node_by_id(&empty.wire, key)["semantic_form"] == "temporal_clause" }));
    assert_ne!(empty_clause["body"], required_clause["body"]);
}

/// FR-464-AC-13: the activation relation binds a required temporal clause to
/// the clause's exact `each` trigger node.
#[trace("FR-464-AC-13", "TC-946")]
#[test]
fn a_required_clause_relation_carries_the_each_trigger() {
    let unit = format!(
        "temporal Due using t over (c: M::Order) clock \"model-steps\" \
         on origin {{ always true }}\n\
         {}",
        flow_over(
            "M::Order",
            "on each (opened: M::Order)",
            "role Buyer on M::Order;\nrequires temporal Due;",
        )
    );
    let lowered = lower_with(model(), PROFILES, &unit);
    assert!(lowered.verified, "{}", lowered.text);
    let clause = node_of(&lowered.wire, "protocol", "protocol_clause");
    let activation = clause_argument(clause, "activation");
    let trigger = activation["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "trigger")
        .map(|member| &member["value"]["target"])
        .unwrap();
    let rows = lowered
        .ir
        .as_ref()
        .expect("the checked package")
        .protocol_activation_relation();
    assert_eq!(rows.len(), 1);
    let quire_contract_model::CheckedProtocolActivation::Each {
        trigger: row_trigger,
    } = &rows[0].activation
    else {
        panic!("an each activation row: {:?}", rows[0]);
    };
    assert_eq!(serde_json::to_value(row_trigger).unwrap(), *trigger);
}

/// FR-383-AC-17: a unit that selects no protocol profile emits no protocol
/// node and no refusal for a protocol declaration, whatever it requires; the
/// temporal clause it requires is still emitted and read back verified, and
/// no `protocol_clause` (hence no requirements set) is in the wire.
#[trace("FR-383-AC-17", "TC-1063")]
#[test]
fn a_requirement_without_a_protocol_profile_emits_no_protocol_node_and_no_refusal() {
    let unit = format!(
        "temporal Due using t over (c: M::Order) clock \"model-steps\" \
         on origin {{ always true }}\n{}",
        flow_over(
            "M::Order",
            "on origin",
            "role Buyer on M::Order;\nrequires temporal Due;",
        )
    );
    let lowered = lower_with(
        model(),
        "profile t = \"quire.temporal.infinite-trace/v1\";\n",
        &unit,
    );
    assert!(lowered.verified, "{}", lowered.text);
    let nodes = lowered.wire["semantic_graph"]["nodes"].as_array().unwrap();
    assert!(nodes.iter().all(|node| node["node_tag"] != "protocol"));
    assert!(nodes
        .iter()
        .all(|node| node["semantic_form"] != "protocol_clause"));
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node["semantic_form"] == "temporal_clause")
            .count(),
        1
    );
}

/// The refusals of `clauses` checked over [`model`] with `profiles`.
fn refusals_of(profiles: &str, clauses: &str) -> Vec<qsl_semantics::check::CheckRefusal> {
    let document = model();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n{profiles}\
         model M = \"acme/orders\";\n\
         {clauses}\n"
    );
    match assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
    {
        Ok(_) => Vec::new(),
        Err(refusals) => refusals,
    }
}

/// FR-383-AC-17: a protocol that requires a temporal clause whose own
/// lowering refused adds no refusal of its own: the clause's refusal is the
/// only report of it, and the protocol is neither an internal fault nor a
/// `missing-name` at the requirement.
#[trace("FR-383-AC-17", "TC-1063")]
#[test]
fn a_protocol_requiring_a_refused_temporal_adds_no_second_refusal() {
    let unit = format!(
        "record Ack {{ ok: Boolean; }}\n\
         function refers using v(): Boolean pure {{ protocol Bad }}\n\
         protocol Bad using v over (input: M::Order) on origin {{\n\
         role R on M::Order;\n\
         run sequence Main {{ repeat Again by R visible (e.ok) while {{ not e.ok }} \
         sequence Body {{ event E by R as (e: Ack) {{ true }}; }} }}\n\
         finish End as (outcome: Boolean) {{ true }};\n}}\n\
         temporal Due using t over (c: M::Order) clock \"model-steps\" \
         on origin {{ always holds(refers()) }}\n{}",
        flow_over(
            "M::Order",
            "on origin",
            "role Buyer on M::Order;\nrequires temporal Due;"
        )
    );
    let refusals = refusals_of(PROFILES, &unit);
    let name = |refusal: &qsl_semantics::check::CheckRefusal| match &refusal.location.origin {
        quire_semantic_value::location::Origin::TypeDeclaration { name } => name.clone(),
        other => format!("{other:?}"),
    };
    assert!(
        refusals
            .iter()
            .any(|refusal| name(refusal) == "temporal 0 holds 1"
                && matches!(refusal.cause, CheckCause::UnsupportedFeature { .. })),
        "the temporal clause's own refusal is reported: {refusals:?}"
    );
    assert!(
        refusals
            .iter()
            .all(|refusal| name(refusal) != "protocol Flow"),
        "the requiring protocol adds no refusal: {refusals:?}"
    );
    assert!(
        refusals.iter().all(|refusal| !matches!(
            refusal.cause,
            CheckCause::InternalFault(_) | CheckCause::MissingName(_)
        )),
        "{refusals:?}"
    );
    assert_eq!(refusals.len(), 3, "{refusals:?}");
}
