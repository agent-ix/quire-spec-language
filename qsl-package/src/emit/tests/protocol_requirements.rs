// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-383-AC-2, AC-13: a protocol's requirements set names the emitted
//! temporal clause nodes, deduplicated and sorted by canonical node identity.

use super::protocol_graph::{flow_over, model};
use super::protocol_reference::{clause_argument, node_by_id, node_of};
use super::reachability::lower_with;
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
        rows[0].clause.digest.as_ref(),
        clause["node_id"]["digest"].as_str().unwrap()
    );
    assert_eq!(
        rows[0].required_temporal.digest.as_ref(),
        target["node_id"]["digest"].as_str().unwrap()
    );
    assert_eq!(
        rows[0].activation,
        quire_contract_model::CheckedProtocolActivation::Origin
    );
}

#[trace("FR-383-AC-13", "TC-1062")]
#[test]
fn requirement_order_does_not_change_the_protocol_package_identity() {
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
    assert_eq!(first.wire["package_id"], second.wire["package_id"]);

    let clause = node_of(&first.wire, "protocol", "protocol_clause");
    let requirements = clause_argument(clause, "requirements")["members"]
        .as_array()
        .unwrap();
    let digests: Vec<_> = requirements
        .iter()
        .map(|reference| reference["target"]["digest"].as_str().unwrap())
        .collect();
    assert_eq!(requirements.len(), 2);
    assert!(digests.windows(2).all(|pair| pair[0] < pair[1]));
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
        .all(|pair| pair[0].required_temporal.digest < pair[1].required_temporal.digest));
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
    assert_ne!(empty_clause["node_id"], required_clause["node_id"]);
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
    assert_eq!(
        row_trigger.digest.as_ref(),
        trigger["digest"].as_str().unwrap()
    );
}
