// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1064, FR-384: the protocol occurrence schema and the runtime binding
//! requirements are derived from the protocol nodes, and are the same from
//! the in-process package and from the package IR's reader admits.

use super::protocol_graph::{model, PROFILES};
use super::protocol_requirements::fixture_w::{document, PROFILES_W, W};
use super::reachability::{lower_with, Lowered};
use super::*;
use crate::{
    protocol_occurrence_schemas_from_read, ControlEntry, ControlOperator, ControlOwner,
    OccurrenceKeyError, ProtocolOccurrence, RepeatDimension, RoleLifetime, RoleReplication,
    RuntimeBindingRequirement, SubjectKind, WorkflowInstanceIdentity,
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

const PARTIAL: &str = "  compensate Partial for Main::Applied as (partial: M::ChargeResult)\n\
        by Payments on M::Payment::refund using T clock \"refund-clock\" {\n\
        capture partialSaved: Integer = partial.order;\n\
        activate first (partialTrigger: M::RefundTrigger) when { partialTrigger.ok } { }\n\
        within [0,5];\n\
        attempts 1 of M::RefundAttempt;\n\
        retry (partialOlder: M::RefundAttempt, partialNewer: M::RefundAttempt) { partialNewer.order = partialOlder.order };\n\
        commit never;\n\
        recover (partialRecovery: M::RefundRecovery) { partialRecovery.order = partialSaved };\n\
        }\n";

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
        .all(|role| role.replication == RoleReplication::Static));
    assert_eq!(schema.controls.len(), 15);
    assert_eq!(schema.compensations.len(), 1);
    assert_eq!(last(&schema.compensations[0].path), "Refund");
    assert_eq!(schema.compensations[0].maximum_attempts, int(2));
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
    for (name, operator) in [
        ("Main", ControlOperator::Sequence),
        ("Shipments", ControlOperator::Parallel),
        ("Outcome", ControlOperator::Choice),
        ("Poll", ControlOperator::Repeat),
        ("Applied", ControlOperator::Effect),
        ("Polled", ControlOperator::Attempt),
        ("Limit", ControlOperator::Check),
    ] {
        assert_eq!(control(&occurrence, name).operator, operator, "{name}");
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

/// The structural key of the selected `acme/orders` object type `name`.
fn object_type(name: &str) -> quire_exact::NodeKey {
    quire_exact::NodeKey::new(
        "acme/orders",
        ["model", "object_type", &format!("ix://acme/orders/{name}")]
            .into_iter()
            .map(|segment| quire_exact::PathSegment::Text(segment.to_owned()))
            .collect(),
        0,
    )
    .expect("a structural key")
}

/// QSL-1579: each role slot carries the selected model type node it is `on`,
/// equally in process and after the package IR's read, and that node is an
/// emitted `object_type` node of the selected package.
#[trace("FR-384-AC-9", "TC-1064")]
#[test]
fn role_slots_carry_their_selected_model_type_through_read_back() {
    let lowered = lower_with(document(), PROFILES_W, W);
    let occurrence = only(parity(&lowered));
    let models: Vec<_> = occurrence
        .schema
        .roles
        .iter()
        .map(|role| (last(&role.path), role.model.clone()))
        .collect();
    assert_eq!(
        models,
        [
            ("Fulfillment", object_type("Shipment")),
            ("Orders", object_type("OrderView")),
            ("Payments", object_type("Payment")),
        ],
        "each role slot names its authored model type"
    );
    let ir = lowered.ir.as_ref().expect("IR admits fixture W");
    let graph = crate::checked_v2::wire_graph(ir).expect("wire identities decode");
    for (_, model) in &models {
        assert!(
            ir.graph()
                .nodes
                .iter()
                .any(|node| &*node.semantic_form == "object_type"
                    && graph.key(&node.node_id).ok().as_ref() == Some(model)),
            "{model:?} is an emitted object type node"
        );
    }
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

/// W with a `repeat` inside its `Poll` repeat.
fn nested_repeats() -> String {
    let nested = W.replace(
        "      attempt Polled by Fulfillment on M::Shipment::poll contracts []\n        as (polled: M::ShipmentResult) { true };\n      exhausted check Limit using v { true };",
        "      sequence Cycle {\n        attempt Probe by Fulfillment on M::Shipment::poll contracts []\n          as (probe: M::ShipmentResult) { true };\n        repeat Inner by Fulfillment visible (eastRes.ok) max 2 while { eastRes.ok }\n          attempt Polled by Fulfillment on M::Shipment::poll contracts []\n            as (polled: M::ShipmentResult) { true };\n          exhausted check InnerLimit using v { true };\n      }\n      exhausted check Limit using v { true };",
    );
    assert_ne!(nested, W);
    nested
}

#[trace("FR-384-AC-2", "TC-1064")]
#[test]
fn nested_repeats_zero_and_missing_maximums_and_branches_outside_a_repeat() {
    // A repeat inside a repeat: two dimensions, outer first.
    let nested = nested_repeats();
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
    // A requirement is exactly a subject kind, a subject key and a scope: the
    // exhaustive pattern stops compiling if an authority, a contract or a
    // runtime value is added (FR-384-AC-6).
    for requirement in &occurrence.requirements {
        let RuntimeBindingRequirement {
            kind: _,
            subject: _,
            scope,
        } = requirement;
        assert_eq!(*scope, occurrence.schema.workflow);
    }
    // Reformatting the source gives the same schema and requirements
    // (compiling needs no runtime input).
    let reformatted = format!("// reformatted W\n{}", W.replace(';', ";\n\n"));
    assert_ne!(reformatted, W);
    assert_eq!(
        only(derive(PROFILES_W, &reformatted, document())),
        occurrence,
        "reformatted W derives the same schema and requirements"
    );
    let with_extras = only(derive(PROFILES_W, &w_with(AWAIT_AND_FENCE), document()));
    assert_eq!(
        with_extras.schema.compensations,
        occurrence.schema.compensations
    );
}

#[trace("FR-384-AC-4", "TC-1064")]
#[test]
fn same_named_roles_and_controls_of_two_protocols_are_distinct_entries() {
    let protocol = |name: &str| {
        format!(
            "protocol {name} using p over (input: M::Order) on origin {{\n\
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

    // Declaring W's roles, channel and compensations in another order gives
    // the same schema, whole: no part of an entry follows the source.
    let reordered_roles = W.replace(
        "  role Orders on M::OrderView;\n  role Payments on M::Payment;\n  role Fulfillment on M::Shipment;\n",
        "  role Fulfillment on M::Shipment;\n  role Payments on M::Payment;\n  role Orders on M::OrderView;\n",
    );
    assert_ne!(reordered_roles, W);
    assert_eq!(only(derive(PROFILES_W, &reordered_roles, document())), w());

    let channel = "  channel Charges from Orders to Payments carries M::ChargeRequest\n    ordering fifo by (message: M::ChargeRequest) { message.order }\n    delivery at-least-once capacity 4 overflow reject;\n";
    assert!(W.contains(channel));
    // The grammar fixes the order of the member kinds, so channels reorder
    // among themselves: a second channel declared before or after the first.
    let second = channel.replace(
        "Charges from Orders to Payments",
        "Replies from Payments to Orders",
    );
    let channels_after = W.replace(channel, &format!("{channel}{second}"));
    let channels_before = W.replace(channel, &format!("{second}{channel}"));
    assert_ne!(channels_after, channels_before);
    let after = only(derive(PROFILES_W, &channels_after, document()));
    assert_eq!(
        after,
        only(derive(PROFILES_W, &channels_before, document())),
        "two channels declared in either order give the same schema"
    );
    assert_eq!(after.schema, w().schema, "a channel adds no schema entry");

    let refund_first = W.replace("  terminal any;", &format!("{PARTIAL}  terminal any;"));
    let partial_first = W.replace(
        "  compensate Refund",
        &format!("{PARTIAL}  compensate Refund"),
    );
    assert_ne!(refund_first, partial_first);
    assert_eq!(
        only(derive(PROFILES_W, &refund_first, document())),
        only(derive(PROFILES_W, &partial_first, document())),
        "two templates declared in either order give the same schema"
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
        .all(|r| r.scope == occurrence.schema.workflow));

    // Event, fence and await imply none either.
    let extra = only(derive(PROFILES_W, &w_with(AWAIT_AND_FENCE), document()));
    assert_eq!(extra.requirements.len(), occurrence.requirements.len());
}

#[trace("FR-384-AC-5", "TC-1064")]
#[test]
fn a_replicated_role_is_marked_one_for_each_spawned_object() {
    let source = "protocol Flow using p over (input: M::Order) on origin {\n\
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
        RoleReplication::Replicated { population, max, lifetime }
            if population.as_str() == "people"
                && *max == int(3)
                && *lifetime == RoleLifetime::Workflow
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
    let occurrence = only(derive(
        PROFILES_W,
        &W.replace("  terminal any;", &format!("{PARTIAL}  terminal any;")),
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

fn workflow(name: &str) -> WorkflowInstanceIdentity<'_> {
    WorkflowInstanceIdentity::new(name).expect("a bounded workflow identity")
}

/// FR-384-AC-11 (FR-048-AC-1): two workflows that share one protocol, and so
/// one provider, bind distinct keys for the same control and ordinals.
#[trace("FR-384-AC-11", "FR-048-AC-1", "TC-1064", "TC-132")]
#[test]
fn two_workflows_sharing_a_provider_bind_distinct_keys_that_return_their_values() {
    let occurrence = w();
    let polled = control(&occurrence, "Polled");
    let (key, owner) = (&polled.key, &polled.owner);
    assert_eq!(owner_name(&occurrence, polled), Some("Fulfillment"));
    let first = [int(0)];
    let o1 = occurrence
        .schema
        .bind(workflow("O1"), key, owner, &first)
        .expect("O1 binds");
    let o2 = occurrence
        .schema
        .bind(workflow("O2"), key, owner, &first)
        .expect("O2 binds");
    assert_ne!(o1, o2, "only the workflow identity differs");
    assert_eq!(o1.workflow().as_str(), "O1");
    assert_eq!(o2.workflow().as_str(), "O2");
    for bound in [&o1, &o2] {
        assert_eq!(bound.protocol(), &occurrence.schema.workflow);
        assert_eq!(bound.control(), key);
        assert_eq!(bound.owner(), owner);
        assert_eq!(bound.repeat_ordinals(), first);
    }
    let third = [int(2)];
    let later = occurrence
        .schema
        .bind(workflow("O1"), key, owner, &third)
        .expect("ordinal 2 is inside [0, 3)");
    assert_ne!(o1, later, "only the ordinal differs");
    assert_eq!(later.repeat_ordinals(), third);
    // The schema itself still holds none of the supplied values.
    assert_eq!(occurrence, w());
}

/// FR-384-AC-11 (FR-048-AC-1): each dimension admits exactly its domain.
#[trace("FR-384-AC-11", "FR-048-AC-1", "TC-1064", "TC-132")]
#[test]
fn each_repeat_dimension_admits_exactly_its_domain() {
    let refused =
        |dimension: usize, value: Integer| OccurrenceKeyError::RepeatOrdinal { dimension, value };
    let bind = |occurrence: &ProtocolOccurrence, name: &str, ordinals: &[Integer]| {
        let entry = control(occurrence, name);
        occurrence
            .schema
            .bind(workflow("O1"), &entry.key, &entry.owner, ordinals)
            .map(|bound| bound.repeat_ordinals().to_vec())
    };

    // Two nested iteration dimensions, outer [0, 3) and inner [0, 2).
    let nested = only(derive(PROFILES_W, &nested_repeats(), document()));
    assert_eq!(
        bind(&nested, "Polled", &[int(2), int(1)]),
        Ok(vec![int(2), int(1)])
    );
    assert_eq!(
        bind(&nested, "Polled", &[int(3), int(1)]),
        Err(refused(0, int(3)))
    );
    assert_eq!(
        bind(&nested, "Polled", &[int(2), int(2)]),
        Err(refused(1, int(2)))
    );
    assert_eq!(
        bind(&nested, "Polled", &[int(-1), int(0)]),
        Err(refused(0, int(-1)))
    );
    // An iteration then an exhaustion dimension.
    assert_eq!(
        bind(&nested, "InnerLimit", &[int(1), int(2)]),
        Ok(vec![int(1), int(2)])
    );
    assert_eq!(
        bind(&nested, "InnerLimit", &[int(1), int(1)]),
        Err(refused(1, int(1)))
    );
    assert_eq!(
        bind(&nested, "InnerLimit", &[int(1), int(3)]),
        Err(refused(1, int(3)))
    );

    // The exhaustion arm admits exactly the maximum, 3.
    let w = w();
    assert_eq!(bind(&w, "Limit", &[int(3)]), Ok(vec![int(3)]));
    assert_eq!(bind(&w, "Limit", &[int(2)]), Err(refused(0, int(2))));
    assert_eq!(bind(&w, "Limit", &[int(4)]), Err(refused(0, int(4))));

    // `max 0` is an empty domain: not even ordinal 0 is inside it.
    let empty = only(derive(PROFILES_W, &W.replace("max 3", "max 0"), document()));
    assert_eq!(bind(&empty, "Polled", &[int(0)]), Err(refused(0, int(0))));

    // No maximum: every non-negative integer, beyond 64 bits too, and no
    // negative one.
    let unbounded = only(derive(
        PROFILES_W,
        &W.replace(" max 3", "")
            .replace("      exhausted check Limit using v { true };\n", ""),
        document(),
    ));
    let huge = Integer::from(i128::from(i64::MAX) * 4);
    assert_eq!(bind(&unbounded, "Polled", &[int(0)]), Ok(vec![int(0)]));
    assert_eq!(bind(&unbounded, "Polled", &[huge.clone()]), Ok(vec![huge]));
    assert_eq!(
        bind(&unbounded, "Polled", &[int(-1)]),
        Err(refused(0, int(-1)))
    );
}

/// FR-384-AC-13 (FR-048-AC-1): each way a key can be wrong has its own
/// typed refusal, and a refused call yields no key.
#[trace("FR-384-AC-13", "FR-048-AC-1", "TC-1064", "TC-132")]
#[test]
fn a_key_with_a_wrong_control_owner_or_ordinal_count_is_refused_by_name() {
    let occurrence = w();
    let schema = &occurrence.schema;
    let polled = control(&occurrence, "Polled");
    let requested = control(&occurrence, "Requested");
    let main = control(&occurrence, "Main");
    assert_eq!(main.owner, ControlOwner::Structural);
    let one = [int(1)];

    // UnknownNode: a key the schema holds no entry for.
    let absent = quire_exact::NodeKey::new(
        "acme/orders",
        vec![quire_exact::PathSegment::Text("Nowhere".to_owned())],
        0,
    )
    .expect("a structural key");
    assert_eq!(
        schema.bind(workflow("O1"), &absent, &ControlOwner::Structural, &[]),
        Err(OccurrenceKeyError::UnknownNode { node: absent })
    );
    // The protocol clause is no control entry either.
    assert_eq!(
        schema.bind(
            workflow("O1"),
            &schema.workflow,
            &ControlOwner::Structural,
            &[]
        ),
        Err(OccurrenceKeyError::UnknownNode {
            node: schema.workflow.clone(),
        })
    );

    // WrongRole: a swapped role, and structural and role owners exchanged.
    let payments = control(&occurrence, "Received").owner.clone();
    assert_ne!(payments, requested.owner);
    assert_eq!(
        schema.bind(workflow("O1"), &requested.key, &payments, &[]),
        Err(OccurrenceKeyError::WrongRole {
            expected: requested.owner.clone(),
            actual: payments.clone(),
        })
    );
    assert_eq!(
        schema.bind(
            workflow("O1"),
            &requested.key,
            &ControlOwner::Structural,
            &[]
        ),
        Err(OccurrenceKeyError::WrongRole {
            expected: requested.owner.clone(),
            actual: ControlOwner::Structural,
        })
    );
    assert_eq!(
        schema.bind(workflow("O1"), &main.key, &requested.owner, &[]),
        Err(OccurrenceKeyError::WrongRole {
            expected: ControlOwner::Structural,
            actual: requested.owner.clone(),
        })
    );

    // RepeatCount: missing, surplus, and an ordinal for an entry with none.
    for ordinals in [&[][..], &[int(0), int(0)][..]] {
        assert_eq!(
            schema.bind(workflow("O1"), &polled.key, &polled.owner, ordinals),
            Err(OccurrenceKeyError::RepeatCount {
                expected: 1,
                actual: ordinals.len(),
            })
        );
    }
    assert_eq!(
        schema.bind(workflow("O1"), &requested.key, &requested.owner, &one),
        Err(OccurrenceKeyError::RepeatCount {
            expected: 0,
            actual: 1,
        })
    );
    // The exact counts bind.
    assert!(schema
        .bind(workflow("O1"), &requested.key, &requested.owner, &[])
        .is_ok());
    assert!(schema
        .bind(workflow("O1"), &polled.key, &polled.owner, &one)
        .is_ok());
}

/// FR-384-AC-13: a control of another protocol is unknown to this schema,
/// though it shares its name, role name and model with one of this schema.
#[trace("FR-384-AC-13", "FR-048-AC-1", "TC-1064", "TC-132")]
#[test]
fn another_protocols_same_named_control_is_an_unknown_node() {
    let protocol = |name: &str| {
        format!(
            "protocol {name} using p over (input: M::Order) on origin {{\n\
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
    let [a, b] = occurrences.as_slice() else {
        panic!("two protocols")
    };
    let (charge_a, charge_b) = (control(a, "Charge"), control(b, "Charge"));
    assert_eq!(charge_a.owner, ControlOwner::Structural);
    assert_eq!(charge_b.owner, ControlOwner::Structural);
    assert_ne!(charge_a.key, charge_b.key);
    assert!(a
        .schema
        .bind(workflow("O1"), &charge_a.key, &charge_a.owner, &[])
        .is_ok());
    assert_eq!(
        a.schema
            .bind(workflow("O1"), &charge_b.key, &charge_b.owner, &[]),
        Err(OccurrenceKeyError::UnknownNode {
            node: charge_b.key.clone(),
        })
    );
    // The key names the protocol it was bound against.
    let key = b
        .schema
        .bind(workflow("O1"), &charge_b.key, &charge_b.owner, &[])
        .expect("B binds its own control");
    assert_eq!(key.protocol(), &b.schema.workflow);
    assert_ne!(key.protocol(), &a.schema.workflow);
}

/// FR-384-AC-14 (FR-048-AC-1): a schema exists for a protocol clause only,
/// and holds one entry for each role, control and compensation node read.
#[trace("FR-384-AC-14", "FR-048-AC-1", "TC-1064", "TC-132")]
#[test]
fn a_schema_exists_for_a_protocol_clause_only_with_one_entry_for_each_node() {
    let lowered = lower_with(document(), PROFILES_W, W);
    let occurrence = only(parity(&lowered));
    let nodes = lowered.wire["semantic_graph"]["nodes"].as_array().unwrap();
    let count = |form: &str| {
        nodes
            .iter()
            .filter(|node| node["semantic_form"] == form)
            .count()
    };
    assert!(
        count("object_type") > 0,
        "W's package holds model type nodes"
    );
    assert_eq!(count("protocol_clause"), 1);
    assert_eq!(count("role"), occurrence.schema.roles.len());
    assert_eq!(count("control"), occurrence.schema.controls.len());
    assert_eq!(count("compensation"), occurrence.schema.compensations.len());
    assert_eq!(
        (
            occurrence.schema.roles.len(),
            occurrence.schema.controls.len(),
            occurrence.schema.compensations.len()
        ),
        (3, 15, 1)
    );
    // The one schema is keyed by the protocol clause, which is none of the
    // role, control or compensation entries.
    let schema = &occurrence.schema;
    assert!(schema.roles.iter().all(|role| role.key != schema.workflow));
    assert!(schema
        .controls
        .iter()
        .all(|control| control.key != schema.workflow));
}
