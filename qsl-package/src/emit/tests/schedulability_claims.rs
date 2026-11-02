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

/// The time vocabulary every task-set unit below opens with.
const TIME: &str = "profile t = \"quire.temporal.complete/v1\";\n\
     dimension Time;\n\
     unit ms : Time = rational(1, 1);\n";
const TASK_A: &str = "task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n";
const TASK_B: &str = "task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n";
const TASK_C: &str = "task c periodic 12 ms deadline 12 ms wcet 5 ms priority 3 lo;\n";
const FP_CLAIM: &str = "schedulable CtlFp on Ctl under fixed_priority;\n";
const EDF_CLAIM: &str = "schedulable CtlEdf on Ctl under edf;\n";

/// `Ctl` over `tasks`, then `claims`.
fn ctl(tasks: &[&str], claims: &str) -> String {
    format!("{TIME}taskset Ctl {{\n{}}}\n{claims}", tasks.concat())
}

fn compile_ctl(text: &str) -> Emission {
    let (_, package) = package_from_text(text);
    emit_checked(&package).expect("the task claims emit")
}

/// The one `claim`/`schedulability` node of `wire` of `kind` under `policy`.
fn claim_with<'w>(wire: &'w Value, kind: &str, policy: &str) -> &'w Value {
    let found: Vec<&Value> = nodes(wire)
        .iter()
        .filter(|node| node["node_tag"] == "claim" && node["semantic_form"] == "schedulability")
        .filter(|node| {
            binding(&node["body"]["members"][0], "kind")["value"] == kind
                && binding(&node["body"]["members"][1], "policy")["value"] == policy
        })
        .collect();
    let [node] = found[..] else {
        panic!("one {kind}/{policy} claim node, found {}", found.len());
    };
    node
}

fn fp_id(emission: &Emission) -> Value {
    claim_with(&wire(emission), "schedulable", "fixed-priority")["node_id"].clone()
}

fn admit_ctl(emission: &Emission) -> quire_contract_model::CheckedPackageV2 {
    match CheckedPackageV2::read(
        emission.package().bytes(),
        quire_contract_model::CheckedPackageReadLimits::bounded(),
        emission.evidence(),
    ) {
        quire_contract_model::CheckedPackageV2ReadResult::Admitted(package) => *package,
        result => panic!("the emitted package is admitted: {result:?}"),
    }
}

/// FR-454-AC-2: the claim and its premises key the node and the package bytes; the
/// claim's and task set's names, the source order of the tasks and an
/// explicit zero `jitter` do not.
#[trace("TC-1202", "FR-454-AC-2")]
#[test]
fn the_node_id_follows_the_checked_premises_and_not_their_spelling() {
    let both = compile_ctl(&ctl(
        &[TASK_A, TASK_B, TASK_C],
        &format!("{FP_CLAIM}{EDF_CLAIM}"),
    ));
    let both_wire = wire(&both);
    let fp = claim_with(&both_wire, "schedulable", "fixed-priority");
    let edf = claim_with(&both_wire, "schedulable", "edf");
    assert_ne!(fp["node_id"], edf["node_id"]);

    let fp_only = compile_ctl(&ctl(&[TASK_A, TASK_B, TASK_C], FP_CLAIM));
    let edf_only = compile_ctl(&ctl(&[TASK_A, TASK_B, TASK_C], EDF_CLAIM));
    assert_ne!(fp_only.package().bytes(), edf_only.package().bytes());

    let heavier = "task c periodic 12 ms deadline 12 ms wcet 6 ms priority 3 lo;\n";
    let heavy = compile_ctl(&ctl(&[TASK_A, TASK_B, heavier], FP_CLAIM));
    assert_ne!(fp_id(&heavy), fp_id(&fp_only));
    assert_ne!(heavy.package().bytes(), fp_only.package().bytes());

    // The base order a, b, c is already ascending by name, so the reordered
    // unit is the one that differs from the emitted order.
    let reordered = compile_ctl(&ctl(&[TASK_C, TASK_A, TASK_B], FP_CLAIM));
    assert_eq!(fp_id(&reordered), fp_id(&fp_only));
    let reordered_wire = wire(&reordered);
    let names: Vec<_> = binding(
        &claim_with(&reordered_wire, "schedulable", "fixed-priority")["body"]["members"][4],
        "tasks",
    )["members"]
        .as_array()
        .expect("the task aggregates")
        .iter()
        .map(|task| binding(&task["members"][0], "name")["value"].clone())
        .collect();
    assert_eq!(names, [json!("a"), json!("b"), json!("c")]);

    let jittered = "task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 jitter 0 ms lo;\n";
    let explicit = compile_ctl(&ctl(&[jittered, TASK_B, TASK_C], FP_CLAIM));
    assert_eq!(fp_id(&explicit), fp_id(&fp_only));

    let bounded = "task a periodic 4 ms deadline 4 ms bcet 1 ms wcet 1 ms priority 1 lo;\n";
    let best_case = compile_ctl(&ctl(&[bounded, TASK_B, TASK_C], FP_CLAIM));
    assert_ne!(fp_id(&best_case), fp_id(&fp_only));
    assert_ne!(best_case.package().bytes(), fp_only.package().bytes());

    let zero_bcet = "task a periodic 4 ms deadline 4 ms bcet 0 ms wcet 1 ms priority 1 lo;\n";
    let explicit_bcet = compile_ctl(&ctl(&[zero_bcet, TASK_B, TASK_C], FP_CLAIM));
    assert_eq!(fp_id(&explicit_bcet), fp_id(&fp_only));

    let renamed = compile_ctl(
        &ctl(
            &[TASK_A, TASK_B, TASK_C],
            "schedulable Other on Ctl under fixed_priority;\n",
        )
        .replace("Ctl", "Renamed"),
    );
    assert_eq!(fp_id(&renamed), fp_id(&fp_only));
}

fn exact(read: Option<quire_contract_model::CheckedLiteralRead<'_>>) -> String {
    use quire_contract_model::CheckedLiteralValue as V;
    match read.expect("the literal is present").value {
        V::Rational {
            numerator,
            denominator,
        } => format!("{numerator}/{denominator}"),
        V::Integer(value) => value.to_string(),
        other => panic!("an exact number, not {other:?}"),
    }
}

/// A task as the IR reader returns it: name, arrival, criticality, period,
/// deadline, bcet, wcet, wcet_hi, priority, jitter and blocking.
fn read_task(task: &quire_contract_model::CheckedSchedulabilityTaskRef<'_>) -> Vec<String> {
    vec![
        task.name().to_owned(),
        format!("{:?}", task.arrival()),
        format!("{:?}", task.criticality()),
        exact(Some(task.period())),
        exact(Some(task.deadline())),
        exact(Some(task.bcet())),
        exact(Some(task.wcet())),
        task.wcet_hi().map_or("-".to_owned(), |hi| exact(Some(hi))),
        task.priority().map_or("-".to_owned(), |p| exact(Some(p))),
        exact(Some(task.jitter())),
        exact(Some(task.blocking())),
    ]
}

fn strings(row: &[&str]) -> Vec<String> {
    row.iter().map(|cell| (*cell).to_owned()).collect()
}

/// FR-454-AC-1: every premise of a task reaches the claim body in its own
/// binding: sporadic arrival, a `hi` task with `wcet_hi`, an absent priority,
/// a non-zero `bcet`, `jitter` and `blocking`, and a `deadline` that differs
/// from the `period`, so a swap of any two fails the read-back.
#[trace("TC-1202", "FR-454-AC-1")]
#[test]
fn every_task_premise_reads_back_through_its_own_binding() {
    let fixed = compile_ctl(&ctl(
        &[
            "task a periodic 4 ms deadline 3 ms bcet 1 ms wcet 1 ms priority 1 lo;\n",
            "task b sporadic 6 ms deadline 5 ms bcet 1 ms wcet 2 ms priority 2 jitter 1 ms \
             blocking 3 ms hi wcet_hi 3 ms;\n",
        ],
        "schedulable CtlFp on Ctl under fixed_priority;\n\
         response R on Ctl.b <= 5 ms under fixed_priority;\n",
    ));
    let admitted = admit_ctl(&fixed);
    let claims = admitted.schedulability_claims().collect::<Vec<_>>();
    assert_eq!(claims.len(), 2);
    for claim in &claims {
        let tasks: Vec<_> = claim.tasks().map(|task| read_task(&task)).collect();
        assert_eq!(
            tasks,
            [
                strings(&[
                    "a", "Periodic", "Lo", "4/1", "3/1", "1/1", "1/1", "-", "1", "0/1", "0/1"
                ]),
                strings(&[
                    "b", "Sporadic", "Hi", "6/1", "5/1", "1/1", "2/1", "3/1", "2", "1/1", "3/1"
                ]),
            ]
        );
    }
    let response = claims
        .iter()
        .find(|claim| claim.kind() == quire_contract_model::CheckedSchedulabilityKind::Response)
        .expect("R is admitted");
    assert_eq!(response.response_task(), Some("b"));
    assert_eq!(exact(response.response_bound()), "5/1");

    let edf = compile_ctl(&ctl(
        &[
            "task a periodic 4 ms deadline 3 ms wcet 1 ms lo;\n",
            TASK_B.replace("priority 2 ", "").as_str(),
        ],
        EDF_CLAIM,
    ));
    let admitted = admit_ctl(&edf);
    let [claim] = admitted.schedulability_claims().collect::<Vec<_>>()[..] else {
        panic!("one claim");
    };
    let tasks: Vec<_> = claim.tasks().map(|task| read_task(&task)).collect();
    assert_eq!(
        tasks,
        [
            strings(&["a", "Periodic", "Lo", "4/1", "3/1", "0/1", "1/1", "-", "-", "0/1", "0/1"]),
            strings(&["b", "Periodic", "Lo", "6/1", "6/1", "0/1", "2/1", "-", "-", "0/1", "0/1"]),
        ]
    );
}

/// A unit selecting the ConfigVersion model and declaring `Ctl` with a
/// schedulability claim beside a state-graph claim and a temporal clause.
fn mixed_unit() -> String {
    use super::state_graph_claim::{unit_after, REACHES_TWO};
    unit_after(
        "profile s = \"quire.temporal.complete/v1\";\n",
        &format!(
            "dimension Time;\n\
             unit ms : Time = rational(1, 1);\n\
             {REACHES_TWO}\n\
             temporal Liveness using t over (c: Config::ConfigVersion) clock \"model-steps\" \
             on origin {{ always eventually holds(c.versionNumber = 2) }}\n\
             taskset Ctl {{\n{}}}\n{FP_CLAIM}",
            [TASK_A, TASK_B, TASK_C].concat()
        ),
    )
}

/// FR-454-AC-3: the lookup returns `[node, "claim", ordinal]` for `CtlFp`
/// and the triple resolves in the recompiled package to the claim and its task
/// set; an unknown NodeId, a `claim`/`state_graph` NodeId, a
/// `temporal`/`temporal_clause` NodeId or another ordinal is
/// `clause-root-unresolved` from this lookup, and `CtlFp`'s triple given to
/// the temporal or the state-graph lookup is `clause-root-unresolved`.
/// Settling a request with an unresolved triple `Failed` is the production
/// settlement of QSL-585 PR 2 and is not exercised here.
#[trace("TC-1202", "FR-454-AC-3")]
#[test]
fn the_schedulability_lookup_resolves_its_triple_and_takes_only_its_own_form() {
    use super::state_graph_claim::{admit, claim_of, compile, key as node_key};
    use crate::checked_v2::{
        clause_roots, resolve_clause_root, resolve_schedulability_root, resolve_state_graph_root,
        schedulability_roots, state_graph_roots, ClauseRootUnresolved,
    };
    use quire_exact::Origin;

    let text = mixed_unit();
    let compiled = compile(&text);
    let admitted = admit(&compiled);
    let fp = claim_with(&compiled.wire, "schedulable", "fixed-priority");
    let roots = schedulability_roots(&admitted).expect("the roots read");
    let [root] = &roots[..] else {
        panic!("one schedulability root: {roots:?}");
    };
    let triple = root.occurrence().clone();
    assert_eq!(triple.node(), &node_key(&compiled.wire, fp));
    assert_eq!(triple.origin().role().as_str(), "claim");
    assert_eq!(triple.origin().ordinal(), 0);
    assert_eq!(
        root.claim().kind(),
        quire_contract_model::CheckedSchedulabilityKind::Schedulable
    );
    assert_eq!(
        root.claim().policy(),
        quire_contract_model::CheckedSchedulingPolicy::FixedPriority
    );

    let recompiled = admit(&compile(&text));
    let resolved = resolve_schedulability_root(&recompiled, &triple)
        .expect("the lookup reads")
        .expect("the triple resolves");
    assert_eq!(resolved.occurrence(), &triple);
    assert_eq!(resolved.claim().tasks().len(), 3);
    assert_eq!(
        resolved
            .claim()
            .tasks()
            .map(|task| task.name().to_owned())
            .collect::<Vec<_>>(),
        ["a", "b", "c"]
    );

    let claim_at =
        |node, ordinal| OccurrenceKey::new(node, Origin::new(Role::new("claim"), ordinal));
    let state_graph = claim_of(&compiled.wire, "possible");
    let temporal = nodes(&compiled.wire)
        .iter()
        .find(|node| node["node_tag"] == "temporal" && node["semantic_form"] == "temporal_clause")
        .expect("the temporal clause node");
    for foreign in [
        claim_at(
            NodeKey::new("pkg", vec![PathSegment::Text("missing".into())], 0).unwrap(),
            0,
        ),
        claim_at(node_key(&compiled.wire, state_graph), 0),
        claim_at(node_key(&compiled.wire, temporal), 0),
        claim_at(triple.node().clone(), 1),
        OccurrenceKey::new(
            triple.node().clone(),
            Origin::new(Role::new("declaration"), 0),
        ),
    ] {
        let looked = resolve_schedulability_root(&recompiled, &foreign).expect("the lookup reads");
        assert_eq!(
            looked.map(|found| found.occurrence().clone()),
            Err(ClauseRootUnresolved {
                root: foreign.clone()
            }),
            "{foreign}"
        );
    }

    assert_eq!(
        resolve_clause_root(&recompiled, &triple),
        Err(ClauseRootUnresolved {
            root: triple.clone()
        })
    );
    assert_eq!(
        resolve_state_graph_root(&recompiled, &triple),
        Ok(Err(ClauseRootUnresolved {
            root: triple.clone()
        }))
    );
    assert!(clause_roots(&recompiled)
        .expect("the roots read")
        .iter()
        .all(|clause| clause.occurrence() != &triple));
    assert!(state_graph_roots(&recompiled)
        .expect("the roots read")
        .iter()
        .all(|graph| graph.occurrence() != &triple));
}

/// `emission`'s wire with the node `key` edited by `edit`, read by IR.
fn refusal_after(
    emission: &Emission,
    key: &Value,
    edit: impl FnOnce(&mut Value),
) -> (quire_contract_model::CheckedPackageRefusal, String) {
    let mut envelope = wire(emission);
    let at = nodes(&envelope)
        .iter()
        .position(|node| &node["node_id"] == key)
        .expect("the edited node");
    edit(&mut envelope["semantic_graph"]["nodes"][at]);
    let refusal = match read_v2(
        &jcs(&envelope),
        library(),
        V2ReadLimits::default(),
        emission.evidence(),
    ) {
        Read::Refused(crate::V2ReadRefusal::Envelope { refusal, .. }) => *refusal,
        other => panic!("an IR refusal, not {other:?}"),
    };
    let path = refusal.loci[0]
        .path
        .as_ref()
        .map(|pointer| pointer.as_str().to_owned())
        .unwrap_or_default();
    assert!(
        path.starts_with(&format!("/semantic_graph/nodes/{at}/")),
        "{path}"
    );
    (refusal, path)
}

/// FR-454-AC-4: each defect of `CtlFp`'s node is refused by the I2 read at
/// QSpec FR-482's locus: a `rate-monotonic` policy, tasks listed `b`, `a`,
/// `c`, and a first task whose `period` is `0`.
#[trace("TC-1202", "FR-454-AC-4")]
#[test]
fn the_i2_read_refuses_each_fr482_defect_at_its_locus() {
    use quire_contract_model::{
        CheckedPackageRefusalCause as Cause, CheckedPackageRefusalCode as Code,
    };
    let emission = compile_ctl(&ctl(&[TASK_A, TASK_B, TASK_C], FP_CLAIM));
    let key = fp_id(&emission);

    let (refusal, path) = refusal_after(&emission, &key, |node| {
        node["body"]["members"][1]["value"]["value"] = json!("rate-monotonic");
    });
    assert_eq!(
        (refusal.code, refusal.cause),
        (Code::InvalidPackage, Some(Cause::InvalidValue)),
        "{refusal:?}"
    );
    assert!(path.ends_with("/body/members/1"), "{path}");

    let (refusal, path) = refusal_after(&emission, &key, |node| {
        let tasks = node["body"]["members"][4]["value"]["members"]
            .as_array_mut()
            .expect("the task aggregates");
        tasks.swap(0, 1);
    });
    assert_eq!(refusal.code, Code::InvalidSemanticGraph, "{refusal:?}");
    assert!(path.ends_with("/body/members/4"), "{path}");

    let (refusal, path) = refusal_after(&emission, &key, |node| {
        let first = &mut node["body"]["members"][4]["value"]["members"][0];
        let period = first["members"]
            .as_array_mut()
            .expect("the first task's bindings")
            .iter_mut()
            .find(|member| member["name"] == "period")
            .expect("the period binding");
        period["value"]["value"] = json!("0/1");
    });
    assert_eq!(
        (refusal.code, refusal.cause),
        (Code::InvalidTimedModel, Some(Cause::InvalidTaskSet)),
        "{refusal:?}"
    );
    assert!(path.ends_with("/body/members/4/value/members/0"), "{path}");
}

/// A checked claim whose normalized body has no node key is refused instead of
/// being dropped from the package.
#[trace("TC-1202", "FR-454-AC-1")]
#[test]
fn a_checked_claim_without_a_node_key_is_refused_not_dropped() {
    let claim = CheckedScheduleClaim {
        name: "CtlFp".to_owned(),
        task_set: "Ctl".to_owned(),
        claim: ScheduleClaim::Schedulable(qsl_semantics::check::SchedulingPolicy::FixedPriority),
        node_id: None,
        occurrence: 0,
        span: qsl_foundation::source::Span { start: 0, end: 0 },
    };
    assert_eq!(
        claims_by_node(&[claim]).err(),
        Some(EmitRefusal::SourceMapInvariant)
    );
}
