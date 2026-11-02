// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-322 / FR-482 schedulability claim body and carrier emission.

use super::*;

fn binding<'a>(term: &'a Value, name: &str) -> &'a Value {
    assert_eq!(term["term"], "binding");
    assert_eq!(term["name"], name);
    &term["value"]
}

fn literal<'a>(term: &'a Value, kind: &str, value: &str) -> &'a Value {
    assert_eq!(term["term"], "literal");
    assert_eq!(term["value_kind"], kind);
    assert_eq!(term["value"], value);
    term
}

fn assert_text(term: &Value, expected: &str, text_type: &Value) {
    let value = literal(term, "text", expected);
    assert_eq!(value["type"], *text_type);
}

fn assert_quantity(term: &Value, expected: &str, time_unit: &Value) {
    let value = literal(term, "quantity", expected);
    assert_eq!(value["type"], *time_unit);
}

/// Semantic type ids that the claim body literals reference.
struct Types<'a> {
    text: &'a Value,
    integer: &'a Value,
    time_unit: &'a Value,
}

fn assert_task_body(
    task: &Value,
    name: &str,
    period: &str,
    wcet: &str,
    priority: &str,
    types: &Types<'_>,
) {
    let text_type = types.text;
    let integer_type = types.integer;
    let time_unit = types.time_unit;
    assert_eq!(task["term"], "aggregate");
    let members = task["members"].as_array().expect("task aggregate members");
    assert_eq!(members.len(), 11);
    assert_text(binding(&members[0], "name"), name, text_type);
    assert_text(binding(&members[1], "arrival"), "periodic", text_type);
    assert_quantity(binding(&members[2], "period"), period, time_unit);
    assert_quantity(binding(&members[3], "deadline"), period, time_unit);
    assert_quantity(binding(&members[4], "bcet"), "0/1", time_unit);
    assert_quantity(binding(&members[5], "wcet"), wcet, time_unit);
    assert_eq!(binding(&members[6], "wcet_hi")["term"], "aggregate");
    assert_eq!(binding(&members[6], "wcet_hi")["members"], json!([]));
    assert_text(binding(&members[7], "criticality"), "lo", text_type);
    let priority = literal(binding(&members[8], "priority"), "integer", priority);
    assert_eq!(priority["type"], *integer_type);
    assert_quantity(binding(&members[9], "jitter"), "0/1", time_unit);
    assert_quantity(binding(&members[10], "blocking"), "0/1", time_unit);
}

fn type_id_for_term<'a>(task: &'a Value, name: &str) -> &'a Value {
    let members = task["members"].as_array().expect("task aggregate members");
    let term = members
        .iter()
        .find(|term| term["name"] == name)
        .expect("the named task binding exists");
    &term["value"]["type"]
}

fn claim_node<'a>(wire: &'a Value, declaration: &Value) -> &'a Value {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .iter()
        .find(|node| node["node_id"] == *declaration)
        .expect("the carrier points at an emitted claim node")
}

#[trace("TC-1202", "FR-454-AC-1")]
#[test]
fn emits_and_admits_the_normative_three_task_claim_bodies() {
    let (_, package) = package_from_text(
        "profile t = \"quire.temporal.complete/v1\";\n\
         dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         taskset Ctl {\n\
         task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n\
         task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n\
         task c periodic 12 ms deadline 12 ms wcet 5 ms priority 3 lo;\n\
         }\n\
         schedulable CtlFp on Ctl under fixed_priority;\n\
         schedulable CtlEdf on Ctl under edf;\n\
         response R on Ctl.c <= 12 ms under fixed_priority;\n",
    );
    let emission = emit_checked(&package).expect("the task claims emit");
    let wire = wire(&emission);
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes");
    let claim_nodes = nodes
        .iter()
        .filter(|node| node["node_tag"] == "claim" && node["semantic_form"] == "schedulability")
        .collect::<Vec<_>>();
    assert_eq!(claim_nodes.len(), 3);

    let bytes = emission.package().bytes();
    let admitted = match CheckedPackageV2::read(
        bytes,
        quire_contract_model::CheckedPackageReadLimits::bounded(),
        emission.evidence(),
    ) {
        quire_contract_model::CheckedPackageV2ReadResult::Admitted(package) => package,
        result => panic!("the emitted task claims are admitted: {result:?}"),
    };
    let claims = admitted.schedulability_claims().collect::<Vec<_>>();
    assert_eq!(claims.len(), 3);
    let rows = admitted
        .schedulability_claim_rows()
        .expect("the carrier rows are retained");
    assert_eq!(rows.len(), 3);
    let boolean_node = nodes
        .iter()
        .find(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "boolean")
        .expect("the claim nodes use the Boolean semantic type");
    for (row, claim) in rows.iter().zip(&claims) {
        assert_eq!(&row.declaration, claim.declaration());
        assert_eq!(row.occurrence, claim.occurrence().ordinal);
        let node = claim_node(&wire, &serde_json::to_value(claim.declaration()).unwrap());
        assert_eq!(node["node_tag"], "claim");
        assert_eq!(node["semantic_form"], "schedulability");
        assert!(node["declaration"].is_null());
        assert!(node["owner"].is_null());
        assert_eq!(node["dependencies"], json!([]));
        assert_eq!(
            node["occurrences"],
            json!([{"role":"claim","ordinal":row.occurrence}])
        );
        assert_eq!(node["semantic_type"], boolean_node["node_id"]);
    }

    let text_node = nodes
        .iter()
        .find(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "text")
        .expect("the claim text literals use the text type");
    let integer_node = nodes
        .iter()
        .find(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "integer")
        .expect("the task priorities use the integer type");

    let fp = claims
        .iter()
        .find(|claim| {
            claim.kind() == quire_contract_model::CheckedSchedulabilityKind::Schedulable
                && claim.policy() == quire_contract_model::CheckedSchedulingPolicy::FixedPriority
        })
        .expect("CtlFp is admitted");
    let edf = claims
        .iter()
        .find(|claim| {
            claim.kind() == quire_contract_model::CheckedSchedulabilityKind::Schedulable
                && claim.policy() == quire_contract_model::CheckedSchedulingPolicy::Edf
        })
        .expect("CtlEdf is admitted");
    let response = claims
        .iter()
        .find(|claim| claim.kind() == quire_contract_model::CheckedSchedulabilityKind::Response)
        .expect("R is admitted");
    assert_ne!(fp.declaration(), edf.declaration());
    assert_eq!(fp.occurrence().ordinal, 0);
    assert_eq!(edf.occurrence().ordinal, 0);
    assert_eq!(response.occurrence().ordinal, 0);
    assert_eq!(fp.tasks().len(), 3);
    assert_eq!(edf.tasks().len(), 3);
    assert_eq!(response.tasks().len(), 3);
    assert_eq!(response.response_task(), Some("c"));
    assert_eq!(
        response.policy(),
        quire_contract_model::CheckedSchedulingPolicy::FixedPriority
    );
    assert_eq!(
        response.response_bound().expect("response bound").value,
        quire_contract_model::CheckedLiteralValue::Rational {
            numerator: 12.into(),
            denominator: 1.into(),
        }
    );

    for claim in [&fp, &edf] {
        let declaration = serde_json::to_value(claim.declaration()).unwrap();
        let node = claim_node(&wire, &declaration);
        assert_eq!(node["semantic_type"], boolean_node["node_id"]);
        let body = node["body"]
            .as_object()
            .expect("claim body is an aggregate");
        assert_eq!(body["term"], "aggregate");
        let members = body["members"].as_array().expect("claim body members");
        assert_eq!(members.len(), 5);
        assert_text(
            binding(&members[0], "kind"),
            "schedulable",
            &text_node["node_id"],
        );
        assert_text(
            binding(&members[1], "policy"),
            if claim.policy() == quire_contract_model::CheckedSchedulingPolicy::Edf {
                "edf"
            } else {
                "fixed-priority"
            },
            &text_node["node_id"],
        );
        assert_eq!(binding(&members[2], "task")["members"], json!([]));
        assert_eq!(binding(&members[3], "bound")["members"], json!([]));
        let task_terms = binding(&members[4], "tasks")["members"]
            .as_array()
            .expect("three canonical task rows");
        assert_eq!(task_terms.len(), 3);
        let time_unit = type_id_for_term(&task_terms[0], "period").clone();
        let unit_node = nodes
            .iter()
            .find(|node| node["node_id"] == time_unit)
            .expect("all time quantities use an emitted unit node");
        assert_eq!(unit_node["semantic_form"], "unit");
        assert_task_body(
            &task_terms[0],
            "a",
            "4/1",
            "1/1",
            "1",
            &Types {
                text: &text_node["node_id"],
                integer: &integer_node["node_id"],
                time_unit: &time_unit,
            },
        );
        assert_task_body(
            &task_terms[1],
            "b",
            "6/1",
            "2/1",
            "2",
            &Types {
                text: &text_node["node_id"],
                integer: &integer_node["node_id"],
                time_unit: &time_unit,
            },
        );
        assert_task_body(
            &task_terms[2],
            "c",
            "12/1",
            "5/1",
            "3",
            &Types {
                text: &text_node["node_id"],
                integer: &integer_node["node_id"],
                time_unit: &time_unit,
            },
        );
        assert_eq!(type_id_for_term(&task_terms[1], "period"), &time_unit);
        assert_eq!(type_id_for_term(&task_terms[2], "period"), &time_unit);
    }

    let response_node = claim_node(
        &wire,
        &serde_json::to_value(response.declaration()).unwrap(),
    );
    let response_body = response_node["body"]["members"]
        .as_array()
        .expect("response claim body members");
    assert_eq!(response_body.len(), 5);
    assert_text(
        binding(&response_body[0], "kind"),
        "response",
        &text_node["node_id"],
    );
    assert_text(
        binding(&response_body[1], "policy"),
        "fixed-priority",
        &text_node["node_id"],
    );
    assert_text(
        binding(&response_body[2], "task"),
        "c",
        &text_node["node_id"],
    );
    let response_tasks = binding(&response_body[4], "tasks")["members"]
        .as_array()
        .expect("response claim carries the same three task rows");
    assert_eq!(response_tasks.len(), 3);
    let time_unit = type_id_for_term(&response_tasks[0], "period").clone();
    assert_quantity(binding(&response_body[3], "bound"), "12/1", &time_unit);
    assert_eq!(
        binding(&response_body[3], "bound")["type"],
        time_unit,
        "the response bound uses the same exact time unit"
    );
    assert_task_body(
        &response_tasks[0],
        "a",
        "4/1",
        "1/1",
        "1",
        &Types {
            text: &text_node["node_id"],
            integer: &integer_node["node_id"],
            time_unit: &time_unit,
        },
    );
    assert_task_body(
        &response_tasks[1],
        "b",
        "6/1",
        "2/1",
        "2",
        &Types {
            text: &text_node["node_id"],
            integer: &integer_node["node_id"],
            time_unit: &time_unit,
        },
    );
    assert_task_body(
        &response_tasks[2],
        "c",
        "12/1",
        "5/1",
        "3",
        &Types {
            text: &text_node["node_id"],
            integer: &integer_node["node_id"],
            time_unit: &time_unit,
        },
    );
}

#[trace("TC-1202", "FR-454-AC-2")]
#[test]
fn duplicate_claim_bodies_share_one_node_with_distinct_carrier_occurrences() {
    let (_, package) = package_from_text(
        "profile t = \"quire.temporal.complete/v1\";\n\
         dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         taskset Ctl {\n\
         task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n\
         task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n\
         task c periodic 12 ms deadline 12 ms wcet 5 ms priority 3 lo;\n\
         }\n\
         schedulable CtlFp on Ctl under fixed_priority;\n\
         schedulable CtlFp2 on Ctl under fixed_priority;\n",
    );
    let emission = emit_checked(&package).expect("the duplicate claims emit");
    let admitted = match CheckedPackageV2::read(
        emission.package().bytes(),
        quire_contract_model::CheckedPackageReadLimits::bounded(),
        emission.evidence(),
    ) {
        quire_contract_model::CheckedPackageV2ReadResult::Admitted(package) => package,
        result => panic!("the duplicate claim body is admitted: {result:?}"),
    };
    let claims = admitted.schedulability_claims().collect::<Vec<_>>();
    assert_eq!(claims.len(), 2);
    assert_eq!(claims[0].declaration(), claims[1].declaration());
    assert_eq!(claims[0].occurrence().ordinal, 0);
    assert_eq!(claims[1].occurrence().ordinal, 1);
    let rows = admitted
        .schedulability_claim_rows()
        .expect("duplicate claim carrier rows are retained");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].declaration, rows[1].declaration);
    assert_eq!(
        rows.iter().map(|row| row.occurrence).collect::<Vec<_>>(),
        [0, 1]
    );
}
