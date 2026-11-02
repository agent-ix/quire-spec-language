// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1064, FR-384: the protocol occurrence schema and the runtime binding
//! requirements are derived from the protocol nodes, and are the same from
//! the in-process package and from the package IR's reader admits.

use super::protocol_graph::{model, PROFILES};
use super::protocol_requirements::fixture_w::{document, PROFILES_W, W};
use super::reachability::{lower_with, Lowered};
use super::*;
use crate::{
    protocol_occurrence_schemas_from_read, ControlEntry, ControlOwner, ProtocolOccurrence,
    RepeatDimension, RoleReplication, SubjectKind,
};
use quire_exact::Integer;

fn int(value: i64) -> Integer {
    Integer::from(value)
}

/// The occurrences of `text` over fixture W's domain, derived in process and
/// from IR's read; the two must be equal (FR-384-AC-3).
fn derive(profiles: &str, text: &str, document: Vec<u8>) -> Vec<ProtocolOccurrence> {
    let lowered = lower_with(document, profiles, text);
    parity(&lowered)
}

fn parity(lowered: &Lowered) -> Vec<ProtocolOccurrence> {
    assert!(lowered.verified, "{}", lowered.text);
    let in_process = lowered
        .occurrences
        .clone()
        .unwrap_or_else(|refusal| panic!("{refusal:?}\n{}", lowered.text));
    let ir = lowered
        .ir
        .as_ref()
        .unwrap_or_else(|error| panic!("{error}\n{}", lowered.text));
    let read_back = protocol_occurrence_schemas_from_read(ir)
        .unwrap_or_else(|refusal| panic!("{refusal:?}\n{}", lowered.text));
    assert_eq!(in_process, read_back, "in-process/read-back parity");
    in_process
}

fn only(mut occurrences: Vec<ProtocolOccurrence>) -> ProtocolOccurrence {
    assert_eq!(occurrences.len(), 1);
    occurrences.remove(0)
}

fn last(path: &[String]) -> &str {
    path.last().map(String::as_str).unwrap_or("")
}

fn control<'a>(occurrence: &'a ProtocolOccurrence, name: &str) -> &'a ControlEntry {
    let found: Vec<_> = occurrence
        .schema
        .controls
        .iter()
        .filter(|control| last(&control.path) == name)
        .collect();
    assert_eq!(found.len(), 1, "one control {name}");
    found[0]
}

/// The owner's role name, or `None` for a structural control.
fn owner_name<'a>(occurrence: &'a ProtocolOccurrence, entry: &ControlEntry) -> Option<&'a str> {
    match &entry.owner {
        ControlOwner::Structural => None,
        ControlOwner::Role(key) => occurrence
            .schema
            .roles
            .iter()
            .find(|role| &role.key == key)
            .map(|role| last(&role.path)),
    }
}

fn w() -> ProtocolOccurrence {
    only(derive(PROFILES_W, W, document()))
}

/// W with `extra` controls inserted before `Finalized`.
fn w_with(extra: &str) -> String {
    W.replace(
        "    commit Finalized",
        &format!("{extra}\n    commit Finalized"),
    )
}

const AWAIT_AND_FENCE: &str = "    fence Barrier ordering seq_cst;\n\
    event Done2 by Orders as (doneEv: Boolean) { true };\n\
    await Wait after Done2 using T clock \"model-steps\" within [0,1] \
    match event Seen by Payments as (seen: Boolean) { true }; \
    then sequence Then { } timeout sequence Late { }";

#[trace("FR-384-AC-1", "TC-1064")]
#[test]
fn w_schema_holds_the_slots_the_owners_and_the_template() {
    let occurrence = w();
    let schema = &occurrence.schema;
    assert_eq!(
        schema
            .roles
            .iter()
            .map(|r| last(&r.path))
            .collect::<Vec<_>>(),
        ["Fulfillment", "Orders", "Payments"],
        "role slots ordered by declaration path"
    );
    assert!(schema
        .roles
        .iter()
        .all(|role| role.replication == RoleReplication::Static && !role.region.is_empty()));
    assert_eq!(schema.controls.len(), 15);
    assert_eq!(schema.compensations.len(), 1);
    assert_eq!(last(&schema.compensations[0].path), "Refund");
    assert_eq!(schema.compensations[0].maximum_attempts, int(2));
    assert!(!schema.compensations[0].region.is_empty());
    let mut paths: Vec<_> = schema.controls.iter().map(|c| c.path.clone()).collect();
    let sorted = {
        let mut copy = paths.clone();
        copy.sort();
        copy
    };
    assert_eq!(paths, sorted, "controls ordered by declaration path");
    paths.dedup();
    assert_eq!(paths.len(), 15);
    for (owner, names) in [
        ("Orders", &["Requested", "Finalized"][..]),
        ("Payments", &["Received", "Charged", "Applied"]),
        (
            "Fulfillment",
            &["East", "West", "Outcome", "Poll", "Polled"],
        ),
    ] {
        for name in names {
            assert_eq!(
                owner_name(&occurrence, control(&occurrence, name)),
                Some(owner),
                "{name}"
            );
        }
    }
    for name in ["Main", "Shipments", "Done", "Failed", "Limit"] {
        assert_eq!(
            control(&occurrence, name).owner,
            ControlOwner::Structural,
            "{name}"
        );
    }
    let compensation = &schema.compensations[0];
    let applied = control(&occurrence, "Applied");
    assert_eq!(
        compensation.effect, applied.key,
        "the forward effect control"
    );
    assert_eq!(
        schema
            .roles
            .iter()
            .find(|role| role.key == compensation.role)
            .map(|role| last(&role.path)),
        Some("Payments")
    );
}

#[trace("FR-384-AC-1", "TC-1064")]
#[test]
fn an_await_and_a_fence_are_structural() {
    let occurrence = only(derive(PROFILES_W, &w_with(AWAIT_AND_FENCE), document()));
    assert_eq!(occurrence.schema.controls.len(), 15 + 6);
    for name in ["Barrier", "Wait", "Then", "Late"] {
        assert_eq!(
            control(&occurrence, name).owner,
            ControlOwner::Structural,
            "{name}"
        );
    }
    assert_eq!(
        owner_name(&occurrence, control(&occurrence, "Done2")),
        Some("Orders")
    );
    assert_eq!(
        owner_name(&occurrence, control(&occurrence, "Seen")),
        Some("Payments")
    );
}

#[trace("FR-384-AC-2", "TC-1064")]
#[test]
fn w_repeat_dimensions_are_an_iteration_and_an_exhaustion() {
    let occurrence = w();
    let poll = control(&occurrence, "Poll").key.clone();
    for entry in &occurrence.schema.controls {
        let expected = match last(&entry.path) {
            "Polled" => vec![RepeatDimension::Iteration {
                repeat: poll.clone(),
                upper_exclusive: Some(int(3)),
            }],
            "Limit" => vec![RepeatDimension::Exhaustion {
                repeat: poll.clone(),
                value: int(3),
            }],
            _ => Vec::new(),
        };
        assert_eq!(entry.dimensions, expected, "{:?}", entry.path);
    }
}

#[trace("FR-384-AC-2", "TC-1064")]
#[test]
fn nested_repeats_zero_and_missing_maximums_and_branches_outside_a_repeat() {
    // A repeat inside a repeat: two dimensions, outer first.
    let nested = W.replace(
        "      attempt Polled by Fulfillment on M::Shipment::poll contracts []\n        as (polled: M::ShipmentResult) { true };\n      exhausted check Limit using v { true };",
        "      sequence Cycle {\n        attempt Probe by Fulfillment on M::Shipment::poll contracts []\n          as (probe: M::ShipmentResult) { true };\n        repeat Inner by Fulfillment visible (eastRes.ok) max 2 while { eastRes.ok }\n          attempt Polled by Fulfillment on M::Shipment::poll contracts []\n            as (polled: M::ShipmentResult) { true };\n          exhausted check InnerLimit using v { true };\n      }\n      exhausted check Limit using v { true };",
    );
    assert_ne!(nested, W);
    let occurrence = only(derive(PROFILES_W, &nested, document()));
    let poll = control(&occurrence, "Poll").key.clone();
    let inner = control(&occurrence, "Inner").key.clone();
    assert_eq!(
        control(&occurrence, "Polled").dimensions,
        [
            RepeatDimension::Iteration {
                repeat: poll.clone(),
                upper_exclusive: Some(int(3))
            },
            RepeatDimension::Iteration {
                repeat: inner.clone(),
                upper_exclusive: Some(int(2))
            },
        ]
    );
    assert_eq!(
        control(&occurrence, "Probe").dimensions,
        [RepeatDimension::Iteration {
            repeat: poll.clone(),
            upper_exclusive: Some(int(3))
        }]
    );
    assert_eq!(
        control(&occurrence, "InnerLimit").dimensions,
        [
            RepeatDimension::Iteration {
                repeat: poll.clone(),
                upper_exclusive: Some(int(3))
            },
            RepeatDimension::Exhaustion {
                repeat: inner,
                value: int(2)
            },
        ]
    );
    assert_eq!(
        control(&occurrence, "Limit").dimensions,
        [RepeatDimension::Exhaustion {
            repeat: poll,
            value: int(3)
        }]
    );

    // max 0: an empty Iteration domain.
    let zero = only(derive(PROFILES_W, &W.replace("max 3", "max 0"), document()));
    assert_eq!(
        control(&zero, "Polled").dimensions,
        [RepeatDimension::Iteration {
            repeat: control(&zero, "Poll").key.clone(),
            upper_exclusive: Some(int(0)),
        }]
    );

    // No maximum: an unbounded Iteration dimension and no Exhaustion entry.
    let unbounded = only(derive(
        PROFILES_W,
        &W.replace(" max 3", "")
            .replace("      exhausted check Limit using v { true };\n", ""),
        document(),
    ));
    assert_eq!(
        control(&unbounded, "Polled").dimensions,
        [RepeatDimension::Iteration {
            repeat: control(&unbounded, "Poll").key.clone(),
            upper_exclusive: None,
        }]
    );
    assert!(unbounded
        .schema
        .controls
        .iter()
        .flat_map(|c| &c.dimensions)
        .all(|d| matches!(d, RepeatDimension::Iteration { .. })));

    // Branch, case and await-then outside any repeat carry no dimension.
    let plain = only(derive(PROFILES_W, &w_with(AWAIT_AND_FENCE), document()));
    for name in ["East", "West", "Done", "Failed", "Then", "Late", "Wait"] {
        assert!(control(&plain, name).dimensions.is_empty(), "{name}");
    }
}

#[trace("FR-384-AC-3", "FR-384-AC-6", "TC-1064")]
#[test]
fn w_holds_no_runtime_value_and_parity_holds_across_variants() {
    let occurrence = w();
    assert!(occurrence.requirements.iter().all(|r| r.authority.is_none()
        && r.contract.is_none()
        && r.scope == occurrence.schema.workflow));
    // Reformatting, a duplicated requirement and a reordered declaration
    // give the same derivation (compiling needs no runtime input).
    let reformatted = format!("// reformatted W\n{}", W.replace(';', ";\n\n"));
    assert_eq!(
        derive(PROFILES_W, &reformatted, document()).len(),
        1,
        "reformatted W derives"
    );
    for variant in [w_with(AWAIT_AND_FENCE), reformatted] {
        let derived = only(derive(PROFILES_W, &variant, document()));
        assert_eq!(derived.schema.compensations.len(), 1);
    }
}

#[trace("FR-384-AC-4", "TC-1064")]
#[test]
fn same_named_roles_and_controls_of_two_protocols_are_distinct_entries() {
    let protocol = |name: &str| {
        format!(
            "protocol {name} using v over (input: M::Order) on origin {{\n\
             role Payer on M::Order;\n\
             run sequence Main {{ check Charge using v {{ true }}; }}\n\
             finish End as (outcome: Boolean) {{ true }};\n}}\n"
        )
    };
    let occurrences = derive(
        PROFILES,
        &format!("{}{}", protocol("A"), protocol("B")),
        model(),
    );
    assert_eq!(occurrences.len(), 2);
    let mut keys = Vec::new();
    for occurrence in &occurrences {
        assert_eq!(occurrence.schema.roles.len(), 1);
        keys.push(occurrence.schema.roles[0].key.clone());
        keys.push(control(occurrence, "Charge").key.clone());
    }
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), 4, "two Payer roles and two Charge controls");

    // Declaring W's roles and channel in another order gives the same schema.
    let reordered = W.replace(
        "  role Orders on M::OrderView;\n  role Payments on M::Payment;\n  role Fulfillment on M::Shipment;\n",
        "  role Fulfillment on M::Shipment;\n  role Payments on M::Payment;\n  role Orders on M::OrderView;\n",
    );
    assert_ne!(reordered, W);
    // A declaration region is a source position, so it is the one part that
    // follows the source text; everything else is equal.
    let strip = |mut occurrences: Vec<ProtocolOccurrence>| {
        for occurrence in &mut occurrences {
            occurrence
                .schema
                .roles
                .iter_mut()
                .for_each(|r| r.region.clear());
            occurrence
                .schema
                .controls
                .iter_mut()
                .for_each(|c| c.region.clear());
            occurrence
                .schema
                .compensations
                .iter_mut()
                .for_each(|c| c.region.clear());
        }
        occurrences
    };
    assert_eq!(
        strip(derive(PROFILES_W, &reordered, document())),
        strip(vec![w()]),
        "schema is a function of the nodes, not the declaration order"
    );
}

fn names(occurrence: &ProtocolOccurrence) -> Vec<(String, SubjectKind)> {
    let mut by_key = std::collections::BTreeMap::new();
    for role in &occurrence.schema.roles {
        by_key.insert(role.key.clone(), last(&role.path).to_owned());
    }
    for entry in &occurrence.schema.controls {
        by_key.insert(entry.key.clone(), last(&entry.path).to_owned());
    }
    for entry in &occurrence.schema.compensations {
        by_key.insert(entry.key.clone(), last(&entry.path).to_owned());
    }
    by_key.insert(occurrence.schema.workflow.clone(), "<clause>".to_owned());
    let mut rows: Vec<_> = occurrence
        .requirements
        .iter()
        .map(|r| {
            (
                by_key
                    .get(&r.subject)
                    .cloned()
                    .unwrap_or_else(|| "<finish>".to_owned()),
                r.kind,
            )
        })
        .collect();
    rows.sort();
    rows
}

#[trace("FR-384-AC-5", "FR-384-AC-6", "TC-1064")]
#[test]
fn requirements_are_exactly_the_subject_kinds_of_the_operator_table() {
    use SubjectKind::*;
    let occurrence = w();
    let role = |per| RoleInstance {
        per_spawned_object: per,
    };
    let mut expected = vec![
        ("<clause>", WorkflowInstance),
        ("Orders", role(false)),
        ("Payments", role(false)),
        ("Fulfillment", role(false)),
        ("Requested", Message),
        ("Requested", Send),
        ("Received", Receive),
        ("Received", Delivery),
        ("Charged", OperationAttempt),
        ("East", OperationAttempt),
        ("West", OperationAttempt),
        ("Polled", OperationAttempt),
        ("Applied", BusinessEffect),
        ("Finalized", Commit),
        ("Refund", Compensation),
        ("Refund", CompensationAttempt),
        ("Refund", CompensationEffect),
        ("<finish>", WorkflowClosure),
    ]
    .into_iter()
    .map(|(name, kind)| (name.to_owned(), kind))
    .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(names(&occurrence), expected);
    // A sequence, choice, parallel, repeat, check, channel, queue and
    // terminal imply none: no requirement names them.
    let structural: Vec<_> = ["Main", "Outcome", "Shipments", "Poll", "Done", "Limit"]
        .iter()
        .map(|name| control(&occurrence, name).key.clone())
        .collect();
    assert!(occurrence
        .requirements
        .iter()
        .all(|r| !structural.contains(&r.subject)));
    // Each requirement names its subject key and the clause as scope.
    assert!(occurrence
        .requirements
        .iter()
        .all(|r| r.scope == occurrence.schema.workflow
            && r.authority.is_none()
            && r.contract.is_none()));

    // Event, fence and await imply none either.
    let extra = only(derive(PROFILES_W, &w_with(AWAIT_AND_FENCE), document()));
    assert_eq!(extra.requirements.len(), occurrence.requirements.len());
}

#[trace("FR-384-AC-5", "TC-1064")]
#[test]
fn a_replicated_role_is_marked_one_for_each_spawned_object() {
    let source = "protocol Flow using v over (input: M::Order) on origin {\n\
        role Static on M::Person;\n\
        role Agent each M::Person from people max 3 lifetime workflow;\n\
        run sequence Main { }\n\
        finish End as (outcome: Boolean) { true };\n}\n";
    let occurrence = only(derive(PROFILES, source, model()));
    let agent = occurrence
        .schema
        .roles
        .iter()
        .find(|role| last(&role.path) == "Agent")
        .unwrap();
    assert!(matches!(
        &agent.replication,
        RoleReplication::Replicated { max, lifetime, .. } if *max == int(3) && lifetime == "workflow"
    ));
    for role in &occurrence.schema.roles {
        let replicated = last(&role.path) == "Agent";
        assert!(
            occurrence.requirements.iter().any(|r| r.subject == role.key
                && r.kind
                    == SubjectKind::RoleInstance {
                        per_spawned_object: replicated
                    }),
            "{:?}",
            role.path
        );
    }
}

#[trace("FR-384-AC-7", "TC-1064")]
#[test]
fn no_node_carries_the_schema_and_a_role_member_is_refused() {
    let lowered = lower_with(document(), PROFILES_W, W);
    let nodes = lowered.wire["semantic_graph"]["nodes"].as_array().unwrap();
    let text = serde_json::to_string(nodes).unwrap();
    for forbidden in [
        "requirement_kind",
        "repeat_dimensions",
        "ordinal_domain",
        "workflow_instance",
        "\"internal\"",
        "\"external\"",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
    let role = nodes
        .iter()
        .position(|node| node["semantic_form"] == "role")
        .unwrap();
    let document = document();
    let refusal_of = |wire: &Value| {
        let mut evidence = CheckedPackageEvidence::new();
        for feature in wire["lock"]["required_features"].as_array().unwrap() {
            evidence.support_feature(feature.as_str().unwrap());
        }
        evidence
            .insert_domain_package_document(package_identity(&document), &*document)
            .unwrap();
        let outcome = quire_contract_model::read_checked_package(
            &serde_json::to_vec(wire).unwrap(),
            quire_contract_model::CheckedPackageReadLimits::bounded(),
            &evidence,
        );
        let quire_contract_model::CheckedPackageDispatchResult::Refused(refusal) = outcome else {
            panic!("the reader must refuse");
        };
        refusal.code
    };
    for member in ["internal", "external"] {
        // A member bound into the role's body is a malformed layout.
        let mut wire = lowered.wire.clone();
        wire["semantic_graph"]["nodes"][role]["body"]["arguments"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"term": "binding", "name": member,
                "value": {"term": "aggregate", "members": []}}));
        assert_eq!(
            refusal_of(&wire),
            quire_contract_model::CheckedPackageRefusalCode::MalformedWire,
            "{member} bound into the body"
        );
        // A node-level member is an unknown member of the closed node shape.
        let mut wire = lowered.wire.clone();
        wire["semantic_graph"]["nodes"][role][member] = serde_json::json!([]);
        assert_eq!(
            refusal_of(&wire),
            quire_contract_model::CheckedPackageRefusalCode::UnknownMember,
            "{member} on the node"
        );
    }
}

#[trace("FR-384-AC-8", "TC-1064")]
#[test]
fn a_full_and_a_partial_template_give_two_entries_and_two_requirement_sets() {
    let partial = "  compensate Partial for Main::Applied as (partial: M::ChargeResult)\n\
        by Payments on M::Payment::refund using T clock \"refund-clock\" {\n\
        capture partialSaved: Integer = partial.order;\n\
        activate first (partialTrigger: M::RefundTrigger) when { partialTrigger.ok } { }\n\
        within [0,5];\n\
        attempts 1 of M::RefundAttempt;\n\
        retry (partialOlder: M::RefundAttempt, partialNewer: M::RefundAttempt) { partialNewer.order = partialOlder.order };\n\
        commit never;\n\
        recover (partialRecovery: M::RefundRecovery) { partialRecovery.order = partialSaved };\n\
        }\n  terminal any;";
    let occurrence = only(derive(
        PROFILES_W,
        &W.replace("  terminal any;", partial),
        document(),
    ));
    let schema = &occurrence.schema;
    assert_eq!(
        schema
            .compensations
            .iter()
            .map(|c| last(&c.path))
            .collect::<Vec<_>>(),
        ["Partial", "Refund"]
    );
    assert_eq!(schema.compensations[0].maximum_attempts, int(1));
    assert_eq!(schema.compensations[1].maximum_attempts, int(2));
    for entry in &schema.compensations {
        let kinds: Vec<_> = occurrence
            .requirements
            .iter()
            .filter(|r| r.subject == entry.key)
            .map(|r| r.kind)
            .collect();
        assert_eq!(
            kinds,
            [
                SubjectKind::Compensation,
                SubjectKind::CompensationAttempt,
                SubjectKind::CompensationEffect
            ],
            "{:?}",
            entry.path
        );
    }
}
