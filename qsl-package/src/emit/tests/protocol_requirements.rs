// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-383-AC-2, AC-13: a protocol's requirements set names the emitted
//! temporal clause nodes, deduplicated in first-requirement order.

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
}
