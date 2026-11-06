// SPDX-License-Identifier: AGPL-3.0-or-later
//! The complete TC-1061 W choreography used by TC-1194's activation cases.

use super::*;
use crate::checked_v2::V2ReadRefusal;
use quire_contract_model::{CheckedPackageRefusalCause, CheckedPackageRefusalCode};

pub(in crate::emit::tests) const PROFILES_W: &str =
    "profile P = \"quire.protocol.finite-global/v1\";\n\
                        profile T = \"quire.temporal.event-position.false-extension/v1\";\n";
/// W's order, payment, shipment and refund domain, using the test domain's
/// existing semantic-IR envelope and object declaration shape.
pub(in crate::emit::tests) fn document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&model()).unwrap();
    let template = document["types"][0].clone();
    let field = |owner: &str, name: &str, ty: &str| {
        let mut field = template["fields"][0].clone();
        field["identity"] = json!(format!("ix://acme/orders/{owner}/{name}"));
        field["name"] = json!(name);
        field["typeRef"] = json!(format!("ix://quire/native/{ty}"));
        field["presence"] = json!("required");
        field
    };
    let operation = |owner: &str, name: &str, result: &str| {
        json!({
            "identity": format!("ix://acme/orders/{owner}/{name}"),
            "name": name, "params": [], "pre": [], "post": [],
            "returns": {"typeRef": format!("ix://acme/orders/{result}"),
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "nullable": false},
            "frame": {"modifies": [], "creates": [], "deletes": []},
            "origin": template["origin"],
        })
    };
    let mut types = Vec::new();
    for name in ["OrderView", "Payment", "Shipment"] {
        let mut ty = template.clone();
        ty["identity"] = json!(format!("ix://acme/orders/{name}"));
        ty["displayName"] = json!(name);
        ty["supertypes"] = json!([]);
        ty["fields"] = json!([field(name, "order", "Integer")]);
        ty["relationships"] = json!([]);
        ty["operations"] = match name {
            "Payment" => json!([
                operation(name, "charge", "ChargeResult"),
                operation(name, "refund", "RefundAttempt")
            ]),
            "Shipment" => json!([
                operation(name, "dispatch", "ShipmentResult"),
                operation(name, "poll", "ShipmentResult")
            ]),
            _ => json!([]),
        };
        if name == "OrderView" {
            let mut relationship = super::super::reachability::relationship(
                "ShipmentOwner",
                "source-to-target",
                "shipments",
                "order",
            );
            relationship["identity"] = json!("ix://acme/orders/relationship/OrderShipment");
            relationship["sourceEnd"]["type"] = json!("ix://acme/orders/OrderView");
            relationship["targetEnd"]["type"] = json!("ix://acme/orders/Shipment");
            ty["relationships"] = json!([relationship]);
        }
        types.push(ty);
    }
    let mut record_kind = document["constructs"][0].clone();
    record_kind["kind"]["name"] = json!("record");
    record_kind["construct"]["meaning"] = json!("quire.meaning.model.record-value-type/v1");
    document["constructs"]
        .as_array_mut()
        .unwrap()
        .push(record_kind);
    for name in [
        "OrderStarted",
        "ChargeRequest",
        "ChargeResult",
        "ShipmentResult",
        "RefundTrigger",
        "RefundAttempt",
        "RefundRecovery",
        "Closure",
    ] {
        let mut ty = template.clone();
        ty["identity"] = json!(format!("ix://acme/orders/{name}"));
        ty["displayName"] = json!(name);
        ty["kind"]["name"] = json!("record");
        ty["fields"] = json!([
            field(name, "order", "Integer"),
            field(name, "ok", "Boolean")
        ]);
        ty["relationships"] = json!([]);
        ty["operations"] = json!([]);
        ty["supertypes"] = json!([]);
        types.push(ty);
    }
    document["types"] = json!(types);
    document["populations"] = json!([]);
    serde_json::to_vec(&document).unwrap()
}

pub(in crate::emit::tests) const W: &str = r#"
pre CanCharge using v on M::Payment::charge { true }
post ChargePost using v on M::Payment::charge { true }
temporal OrderClosureBounded using T over (view: M::OrderView) clock "model-steps"
  on origin { always[0,3] true }
protocol OrderFlow using P over (view: M::OrderView)
on each (opened: M::OrderStarted) {
  capture order: Integer = opened.order;
  role Orders on M::OrderView;
  role Payments on M::Payment;
  role Fulfillment on M::Shipment;
  relationship ShipmentOwner = M::OrderShipment;
  channel Charges from Orders to Payments carries M::ChargeRequest
    ordering fifo by (message: M::ChargeRequest) { message.order }
    delivery at-least-once capacity 4 overflow reject;
  requires temporal OrderClosureBounded;
  compensate Refund for Main::Applied as (forward: M::ChargeResult)
    by Payments on M::Payment::refund using T clock "refund-clock" {
    capture saved: Integer = forward.order;
    activate first (trigger: M::RefundTrigger) when { trigger.ok } { }
    within [0,5];
    attempts 2 of M::RefundAttempt;
    retry (older: M::RefundAttempt, newer: M::RefundAttempt) { newer.order = older.order };
    commit Main::Finalized;
    recover (recovery: M::RefundRecovery) { recovery.order = saved };
  }
  terminal any;
  scheduling adversarial;
  run sequence Main {
    send Requested via Charges as (sent: M::ChargeRequest) { sent.order = order };
    receive Received via Charges of Requested as (got: M::ChargeRequest) { got.order = order };
    attempt Charged by Payments on M::Payment::charge contracts [CanCharge,ChargePost]
      as (charge: M::ChargeResult) { true };
    effect Applied of Charged as (paid: M::ChargeResult) { true };
    parallel Shipments {
      branch east attempt East by Fulfillment on M::Shipment::dispatch contracts []
        as (eastRes: M::ShipmentResult) { true };
      branch west attempt West by Fulfillment on M::Shipment::dispatch contracts []
        as (westRes: M::ShipmentResult) { true };
    } join all [east,west];
    choice Outcome by Fulfillment visible (eastRes.ok) {
      case shipped when { eastRes.ok } check Done using v { true };
      case failed when { not eastRes.ok } check Failed using v { true };
    }
    repeat Poll by Fulfillment visible (eastRes.ok) max 3 while { eastRes.ok }
      attempt Polled by Fulfillment on M::Shipment::poll contracts []
        as (polled: M::ShipmentResult) { true };
      exhausted check Limit using v { true };
    commit Finalized by Orders as (done: Boolean) { true };
  }
  finish Closed as (closed: M::Closure) { closed.order = order };
}
"#;

/// TC-134's checked-v2 fixture: one static charge attempt and effect live
/// inside a bounded repeat, while two refund obligations refer to that exact
/// successful effect.
pub(in crate::emit::tests) fn tc134_source() -> String {
    let source = W.replace(
        "using E clock \\\"refund-clock\\\"",
        "using T clock \\\"refund-clock\\\"",
    );
    let source = source.replace(
        "  terminal any;",
        "  compensate PartialRefund for Main::Applied as (partial: M::ChargeResult)\n\
    by Payments on M::Payment::refund using T clock \"refund-clock\" {\n\
    capture partialSaved: Integer = partial.order;\n\
    activate first (partialTrigger: M::RefundTrigger) when { partialTrigger.ok } { }\n\
    within [0,5];\n\
    attempts 1 of M::RefundAttempt;\n\
    retry (partialOlder: M::RefundAttempt, partialNewer: M::RefundAttempt) { partialNewer.order = partialOlder.order };\n\
    commit never;\n\
    recover (partialRecovery: M::RefundRecovery) { partialRecovery.order = partialSaved };\n\
  }\n  terminal any;",
    );
    let source = source.replace(
        "    attempt Charged",
        "    repeat PaymentRetries by Payments visible () max 3 while { true }\n\
      sequence ChargeCycle {\n\
      attempt Charged",
    );
    source
        .replace("    effect Applied", "      effect Applied")
        .replace(
            "    parallel Shipments {",
            "      }\n    exhausted check RetriesExhausted using v { true };\n    parallel Shipments {",
        )
        .replace(
            "for Main::Applied",
            "for Main::PaymentRetries::ChargeCycle::Applied",
        )
}

#[trace("FR-048-AC-6", "FR-048-AC-7", "TC-134")]
#[test]
fn tc134_checked_v2_retains_static_refund_provenance() {
    let lowered = lower_with(document(), PROFILES_W, &tc134_source());
    assert!(lowered.verified, "{}", lowered.text);
    assert!(
        lowered.ir.is_ok(),
        "strict checked-v2 read: {:?}",
        lowered.ir
    );

    let nodes = lowered.wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("emitted nodes");
    let templates: Vec<&Value> = nodes
        .iter()
        .filter(|node| node["semantic_form"] == "compensation")
        .collect();
    assert_eq!(templates.len(), 2, "both refund obligations remain static");
    assert_ne!(templates[0]["node_id"], templates[1]["node_id"]);

    let charged = nodes
        .iter()
        .find(|node| {
            node["semantic_form"] == "control"
                && node["declaration"]["qualified_name"]
                    .as_array()
                    .is_some_and(|name| name.last() == Some(&json!("Charged")))
        })
        .expect("one authored charge attempt");
    let applied = nodes
        .iter()
        .find(|node| {
            node["semantic_form"] == "control"
                && node["declaration"]["qualified_name"]
                    .as_array()
                    .is_some_and(|name| name.last() == Some(&json!("Applied")))
        })
        .expect("one forward effect");
    let repeat = nodes
        .iter()
        .find(|node| {
            node["semantic_form"] == "control"
                && node["declaration"]["qualified_name"]
                    .as_array()
                    .is_some_and(|name| name.last() == Some(&json!("PaymentRetries")))
        })
        .expect("bounded payment repeat");
    assert_eq!(repeat["body"]["operation"]["member"]["operator"], "repeat");
    assert_eq!(
        repeat["body"]["arguments"][2]["value"]["members"][0]["value"], "3",
        "the static repeat bound is retained"
    );
    let body_id = &repeat["body"]["arguments"][5]["value"]["target"];
    let body = nodes
        .iter()
        .find(|node| &node["node_id"] == body_id)
        .expect("the repeat body is an emitted sequence");
    assert_eq!(body["body"]["operation"]["member"]["operator"], "sequence");
    let children = body["body"]["arguments"][0]["value"]["members"]
        .as_array()
        .expect("sequence child references");
    assert_eq!(children.len(), 2);
    assert_eq!(children[0]["target"], charged["node_id"]);
    assert_eq!(children[1]["target"], applied["node_id"]);
    assert_eq!(
        nodes
            .iter()
            .filter(|node| {
                node["semantic_form"] == "control"
                    && node["declaration"]["qualified_name"]
                        .as_array()
                        .is_some_and(|name| name.last() == Some(&json!("Charged")))
            })
            .count(),
        1,
        "repeat ordinals are runtime inputs and do not mint static attempts"
    );
    assert_eq!(
        applied["body"]["arguments"][0]["value"]["target"],
        charged["node_id"]
    );
    assert_eq!(applied["body"]["operation"]["member"]["operator"], "effect");

    for template in templates {
        let args = template["body"]["arguments"].as_array().unwrap();
        let argument = |name: &str| {
            args.iter()
                .find(|argument| argument["name"] == name)
                .unwrap_or_else(|| panic!("{name} argument in {template}"))
        };
        assert_eq!(argument("forward")["value"]["target"], applied["node_id"]);
        for name in [
            "registration_captures",
            "trigger",
            "activation_captures",
            "attempts",
            "retry",
            "commit",
            "recovery",
        ] {
            assert!(argument(name)["value"].is_object(), "{name} is present");
        }
        let registration = &argument("registration_captures")["value"]["members"][0]["members"];
        let retry = &argument("retry")["value"]["members"];
        let recovery = &argument("recovery")["value"];
        let mut distinct_references = vec![
            argument("forward")["value"]["target"].clone(),
            registration[1]["value"]["target"].clone(),
            argument("trigger")["value"]["members"][1]["value"]["target"].clone(),
            argument("attempts")["value"]["members"][1]["value"]["target"].clone(),
            retry[2]["value"]["target"].clone(),
            recovery["members"][1]["value"]["target"].clone(),
        ];
        if let Some(commit) = argument("commit")["value"]["members"]
            .as_array()
            .and_then(|members| members.first())
        {
            distinct_references.push(commit["target"].clone());
        }
        let unique_references: BTreeSet<String> = distinct_references
            .iter()
            .map(serde_json::Value::to_string)
            .collect();
        assert_eq!(
            unique_references.len(),
            distinct_references.len(),
            "registration, activation, attempt type, retry, forward effect, commit and recovery references stay distinct"
        );
    }

    let relationships: Vec<_> = nodes
        .iter()
        .filter(|node| node["node_tag"] == "relation" && node["semantic_form"] == "relationship")
        .collect();
    assert_eq!(relationships.len(), 1);
    assert_eq!(
        relationships[0]["owner"]["node"],
        "ix://acme/orders/relationship/OrderShipment"
    );
    let clause = node_of(&lowered.wire, "protocol", "protocol_clause");
    assert_eq!(
        clause_argument(clause, "relationships")["members"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    for attempt_name in ["East", "West"] {
        assert!(
            nodes.iter().any(|node| {
                node["semantic_form"] == "control"
                    && node["declaration"]["qualified_name"]
                        .as_array()
                        .is_some_and(|name| name.last() == Some(&json!(attempt_name)))
            }),
            "separate static shipment subject {attempt_name}"
        );
    }
}

#[trace("FR-048-AC-6", "FR-048-AC-7", "TC-134")]
#[test]
fn tc134_attempt_bounds_and_provenance_mutations_are_checked() {
    for count in ["1", "18446744073709551615"] {
        let source = tc134_source()
            .replace(
                "attempts 2 of M::RefundAttempt;",
                &format!("attempts {count} of M::RefundAttempt;"),
            )
            .replace(
                "attempts 1 of M::RefundAttempt;",
                &format!("attempts {count} of M::RefundAttempt;"),
            );
        let lowered = lower_with(document(), PROFILES_W, &source);
        assert!(lowered.verified, "attempt bound {count}: {}", lowered.text);
        assert!(
            lowered.ir.is_ok(),
            "attempt bound {count}: {:?}",
            lowered.ir
        );
        assert_eq!(
            lowered.wire["semantic_graph"]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|node| node["semantic_form"] == "compensation")
                .count(),
            2,
            "attempt bound {count} preserves both obligations"
        );
    }

    let zero = tc134_source()
        .replace(
            "attempts 2 of M::RefundAttempt;",
            "attempts 0 of M::RefundAttempt;",
        )
        .replace(
            "attempts 1 of M::RefundAttempt;",
            "attempts 0 of M::RefundAttempt;",
        );
    let refusals = check_tc134(&zero).expect_err("zero attempt bounds refuse");
    assert!(
        refusals.iter().any(|refusal| {
            refusal.cause.code() == qsl_foundation::Code::IllTyped
                && refusal.cause.cause() == Some("type-mismatch")
        }),
        "typed zero-bound refusal: {refusals:?}"
    );

    let overflow = tc134_source()
        .replace(
            "attempts 2 of M::RefundAttempt;",
            "attempts 18446744073709551616 of M::RefundAttempt;",
        )
        .replace(
            "attempts 1 of M::RefundAttempt;",
            "attempts 18446744073709551616 of M::RefundAttempt;",
        );
    let overflow_unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n{PROFILES_W}\
         model M = \"acme/orders\";\n{overflow}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "tc134-overflow"),
        "tc134.native",
        overflow_unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("the source retains its typed parser result");
    let overflow_refusals = parsed.diagnostics();
    assert!(
        overflow_refusals.iter().any(|refusal| {
            refusal.code == qsl_cst::CompleteCode::InvalidSyntax
                && refusal.cause == qsl_cst::CompleteCause::UnexpectedToken
        }),
        "overflow retains a typed refusal: {overflow_refusals:?}"
    );

    let mutations = [
        (
            "wrong forward",
            tc134_source().replace(
                "for Main::PaymentRetries::ChargeCycle::Applied",
                "for Main::PaymentRetries::ChargeCycle::Charged",
            ),
            qsl_foundation::Code::IllTyped,
        ),
        (
            "wrong owner",
            tc134_source().replace(
                "by Payments on M::Payment::refund",
                "by UnregisteredPayments on M::Payment::refund",
            ),
            qsl_foundation::Code::MissingDeclaration,
        ),
        (
            "wrong commit anchor",
            tc134_source().replace("commit Main::Finalized;", "commit Main::RetriesExhausted;"),
            qsl_foundation::Code::MissingDeclaration,
        ),
    ];
    for (name, source, expected_code) in mutations {
        let refusals = match check_tc134(&source) {
            Ok(()) => panic!("{name} mutation unexpectedly checks"),
            Err(refusals) => refusals,
        };
        assert!(!refusals.is_empty(), "{name} typed refusal");
        assert!(
            refusals
                .iter()
                .any(|refusal| refusal.cause.code() == expected_code),
            "{name} refusal has the expected typed code {expected_code:?}: {refusals:?}"
        );
    }

    let lowered = lower_with(document(), PROFILES_W, &tc134_source());
    let mut wire = lowered.wire.clone();
    let graph = qsl_semantics::check::wire_id::WireGraph::of_package_value(&wire)
        .expect("emitted graph has structural keys");
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let template_index = nodes
        .iter()
        .position(|node| {
            node["semantic_form"] == "compensation"
                && node["declaration"]["qualified_name"][2] == "Refund"
        })
        .expect("the Refund template");
    let effect = nodes
        .iter()
        .find(|node| {
            node["semantic_form"] == "control"
                && node["declaration"]["qualified_name"]
                    .as_array()
                    .is_some_and(|name| name.last() == Some(&json!("Applied")))
        })
        .expect("the forward effect")["node_id"]
        .clone();
    let attempt = nodes
        .iter()
        .find(|node| {
            node["semantic_form"] == "control"
                && node["declaration"]["qualified_name"]
                    .as_array()
                    .is_some_and(|name| name.last() == Some(&json!("Charged")))
        })
        .expect("the payment attempt")["node_id"]
        .clone();
    let template = &mut wire["semantic_graph"]["nodes"][template_index];
    let forward_target = &mut template["body"]["arguments"][0]["value"]["target"];
    assert_eq!(*forward_target, effect, "positive control names the effect");
    *forward_target = attempt.clone();
    let dependencies = template["dependencies"].as_array_mut().unwrap();
    dependencies.retain(|dependency| dependency != &effect);
    dependencies.push(attempt);
    dependencies.sort_by_key(|dependency| {
        graph
            .key_of_value(dependency)
            .expect("structural dependency key")
    });

    let mut evidence = CheckedPackageEvidence::new();
    for feature in lowered.wire["lock"]["required_features"]
        .as_array()
        .unwrap()
    {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let domain = document();
    evidence
        .insert_domain_package_document(package_identity(&domain), domain.as_slice())
        .expect("fresh domain evidence");
    let refused = read_v2(&jcs(&wire), library(), V2ReadLimits::default(), &evidence);
    let Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) = refused else {
        panic!("a forward reference to an attempt instead of an effect must refuse");
    };
    assert_eq!(
        refusal.code,
        CheckedPackageRefusalCode::InvalidModelBinding,
        "{refusal:?}"
    );
    assert_eq!(
        refusal.cause,
        Some(CheckedPackageRefusalCause::MalformedDeclaration),
        "{refusal:?}"
    );
    assert_eq!(
        refusal.loci[0].path.as_ref().map(|path| path.as_str()),
        Some(
            format!("/semantic_graph/nodes/{template_index}/body/arguments/0/value/target")
                .as_str()
        ),
        "{refusal:?}"
    );

    let source = tc134_source();
    let commit = "    commit Finalized by Orders as (done: Boolean) { true };\n";
    let before_effect = source.replace(commit, "").replace(
        "    repeat PaymentRetries",
        &format!("{commit}    repeat PaymentRetries"),
    );
    let refusals = check_tc134(&before_effect)
        .expect_err("a commit preceding its registered forward effect refuses");
    assert!(
        refusals.iter().any(|refusal| {
            refusal.cause.code() == qsl_foundation::Code::UnsupportedConstruct
                && refusal.cause.cause() == Some("declaration-form")
        }),
        "typed commit/effect ordering refusal: {refusals:?}"
    );

    // This is separate from commit-before-effect: a recovery predicate cannot
    // treat a value produced by the protocol commit as an available prerequisite.
    let commit_supplied_recovery = tc134_source().replace(
        "recover (recovery: M::RefundRecovery) { recovery.order = saved };",
        "recover (recovery: M::RefundRecovery) { recovery.ok = done };",
    );
    let refusals = check_tc134(&commit_supplied_recovery)
        .expect_err("recovery cannot use a value produced by the protocol commit");
    assert!(
        refusals.iter().any(|refusal| {
            refusal.cause.code() == qsl_foundation::Code::MissingDeclaration
                && refusal.cause.cause() == Some("missing-name")
        }),
        "typed refusal when recovery depends on a later commit result: {refusals:?}"
    );
}

fn jcs(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).expect("wire fixture serializes")
}

fn library() -> qsl_semantics::library::LibraryName {
    qsl_semantics::library::LibraryName::new("pkg").expect("fixture library name")
}

fn check_tc134(unit: &str) -> Result<(), Vec<qsl_semantics::check::CheckRefusal>> {
    let domain = document();
    let packages =
        qsl_semantics::model::intake::package_input([domain.as_slice()]).expect("model intake");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n{PROFILES_W}\
         model M = \"acme/orders\";\n{unit}\n"
    );
    assemble_with_models(source.as_bytes(), &packages)
        .expect("unit assembles")
        .check(CheckingLimits::default())
        .map(|_| ())
}

#[trace(
    "TC-1194",
    "FR-449-AC-1",
    "FR-449-AC-2",
    "FR-449-AC-3",
    "FR-449-AC-4",
    "FR-449-AC-5",
    "FR-054-AC-1",
    "FR-054-AC-2",
    "FR-054-AC-3",
    "FR-054-AC-4",
    "FR-513-AC-11"
)]
#[test]
fn complete_w_activation_map_matches_read_back() {
    let lowered = lower_with(document(), PROFILES_W, W);
    assert!(lowered.verified, "{}", lowered.text);
    assert_activation_target_refusals(&lowered, &document());
    let rows = activation_rows(&lowered);
    assert_eq!(rows.len(), 1);
    let clause = node_of(&lowered.wire, "protocol", "protocol_clause");
    assert_eq!(
        rows[0].clause,
        structural(&lowered.wire, &clause["node_id"])
    );
    let trigger = clause_argument(clause, "activation")["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "trigger")
        .unwrap()["value"]["target"]
        .clone();
    assert_eq!(
        rows[0].activation,
        quire_contract_model::CheckedProtocolActivation::Each {
            trigger: structural(&lowered.wire, &trigger),
        }
    );
    let temporal = node_of(&lowered.wire, "temporal", "temporal_clause");
    assert_eq!(
        rows[0].required_temporal,
        structural(&lowered.wire, &temporal["node_id"])
    );
    let nodes = lowered.wire["semantic_graph"]["nodes"].as_array().unwrap();
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node["semantic_form"] == "control")
            .count(),
        15
    );
    for (form, count) in [
        ("role", 3),
        ("channel", 1),
        ("queue", 1),
        ("compensation", 1),
    ] {
        assert_eq!(
            nodes
                .iter()
                .filter(|node| node["semantic_form"] == form)
                .count(),
            count,
            "W's {form} nodes"
        );
    }
    for text in [W.to_owned(), format!("// reformatted W\n{}", W.replace(';', ";\n\n")),
        W.replace("requires temporal OrderClosureBounded;", "requires temporal OrderClosureBounded;\nrequires temporal OrderClosureBounded;"),
        format!("temporal RefundBounded using T over (view: M::OrderView) clock \"refund-clock\" on origin {{ eventually[0,1] holds(false) }}\n{W}"),
        format!("temporal Unrequired using T over (view: M::OrderView) clock \"model-steps\" on origin {{ eventually[0,1] holds(false) }}\n{W}"),
    ] {
        let variant = lower_with(document(), PROFILES_W, &text);
        assert!(variant.verified, "{}", variant.text);
        assert_eq!(activation_rows(&variant), rows);
    }
    let empty = lower_with(
        document(),
        PROFILES_W,
        &W.replace("requires temporal OrderClosureBounded;", ""),
    );
    assert!(empty.verified, "{}", empty.text);
    assert!(activation_rows(&empty).is_empty());
    let two = lower_with(document(), PROFILES_W, &format!("temporal OrderPaidBounded using T over (view: M::OrderView) clock \"model-steps\" on origin {{ eventually[0,1] holds(false) }}\n{}", W.replace("requires temporal OrderClosureBounded;", "requires temporal OrderClosureBounded;\nrequires temporal OrderPaidBounded;")));
    assert!(two.verified, "{}", two.text);
    let rows = activation_rows(&two);
    assert_eq!(rows.len(), 2);
    assert!(rows[0].required_temporal < rows[1].required_temporal);
    let mut temporal_keys: Vec<_> = two.wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["semantic_form"] == "temporal_clause")
        .map(|node| structural(&two.wire, &node["node_id"]))
        .collect();
    temporal_keys.sort_unstable();
    assert_eq!(
        rows.iter()
            .map(|row| row.required_temporal.clone())
            .collect::<Vec<_>>(),
        temporal_keys
    );
}

/// FR-383-AC-11 / TC-1062: walk the complete protocol subtree of W. Every
/// guarded/constraint-like position is a literal or a reference to an emitted
/// expression node, and the wire body contains only the stratified
/// Leaf/Group/Tuple/Member/Body forms, never an inline application.
#[trace("TC-1062", "FR-383-AC-11")]
#[test]
fn every_w_protocol_term_position_uses_the_stratified_body_grammar() {
    #[derive(Default)]
    struct Counts {
        leaves: usize,
        groups: usize,
        tuples: usize,
        members: usize,
        bodies: usize,
    }

    fn expression_member(name: &str) -> bool {
        matches!(
            name,
            "guard"
                | "condition"
                | "constraint"
                | "key"
                | "initializer"
                | "until"
                | "visible"
                | "start"
                | "end"
                | "invariant"
                | "variant"
                | "relation"
                | "predicate"
        )
    }

    fn assert_expression_roots(wire: &Value, root: &Value, include_descendants: bool) {
        let mut pending = vec![(root, include_descendants)];
        while let Some((value, all_children)) = pending.pop() {
            match value {
                Value::Object(object) if object.contains_key("term") => {
                    match object["term"].as_str().expect("term kind") {
                        "literal" => {}
                        "reference" => {
                            let target = nodes(wire)
                                .iter()
                                .find(|node| node["node_id"] == object["target"])
                                .expect("the expression target is emitted");
                            assert!(
                                matches!(target["node_tag"].as_str(), Some("expression" | "value")),
                                "a protocol term position must name an FR-093 expression/value leaf: {object:?}; target {target:?}"
                            );
                        }
                        "binding" => {
                            let name = object["name"].as_str().expect("member name");
                            pending
                                .push((&object["value"], all_children || expression_member(name)));
                        }
                        "aggregate" => pending.extend(
                            object["members"]
                                .as_array()
                                .expect("group members")
                                .iter()
                                .map(|member| (member, all_children)),
                        ),
                        "tuple" => pending.extend(
                            object["items"]
                                .as_array()
                                .expect("tuple items")
                                .iter()
                                .map(|item| (item, all_children)),
                        ),
                        other => {
                            panic!("expression position is not a stratified term: {other}: {value}")
                        }
                    }
                }
                Value::Object(object) => {
                    if let (Some(name), Some(member_value)) = (
                        object.get("name").and_then(Value::as_str),
                        object.get("value"),
                    ) {
                        pending.push((member_value, all_children || expression_member(name)));
                    } else {
                        pending.extend(object.values().map(|child| (child, all_children)));
                    }
                }
                Value::Array(items) => {
                    pending.extend(items.iter().map(|item| (item, all_children)));
                }
                _ => {}
            }
        }
    }

    fn visit(wire: &Value, value: &Value, counts: &mut Counts) {
        match value {
            Value::Object(object) if object.contains_key("term") => {
                match object["term"].as_str().expect("term kind") {
                    "literal" => counts.leaves += 1,
                    "binding" => {
                        counts.members += 1;
                        let name = object["name"].as_str().expect("binding name");
                        if expression_member(name) {
                            assert_expression_roots(wire, &object["value"], name == "visible");
                        }
                        visit(wire, &object["value"], counts);
                    }
                    "reference" => counts.leaves += 1,
                    "aggregate" => {
                        let members = object["members"].as_array().expect("group members");
                        if members.iter().all(|member| member.get("name").is_some()) {
                            counts.groups += 1;
                            for member in members {
                                visit(wire, member, counts);
                            }
                        } else {
                            counts.tuples += 1;
                            for member in members {
                                visit(wire, member, counts);
                            }
                        }
                    }
                    "tuple" => {
                        counts.tuples += 1;
                        for item in object["items"].as_array().expect("tuple items") {
                            visit(wire, item, counts);
                        }
                    }
                    other => panic!("not a stratified protocol term: {other}: {value}"),
                }
            }
            Value::Object(object) => {
                if let (Some(name), Some(member_value)) = (
                    object.get("name").and_then(Value::as_str),
                    object.get("value"),
                ) {
                    counts.members += 1;
                    if expression_member(name) {
                        assert_expression_roots(wire, member_value, name == "visible");
                    }
                    visit(wire, member_value, counts);
                    return;
                }
                for (name, child) in object {
                    if name == "body" {
                        counts.bodies += 1;
                    }
                    if expression_member(name) {
                        assert_expression_roots(wire, child, name == "visible");
                    }
                    visit(wire, child, counts);
                }
            }
            Value::Array(items) => {
                for item in items {
                    visit(wire, item, counts);
                }
            }
            _ => {}
        }
    }

    let lowered = lower_with(document(), PROFILES_W, W);
    assert!(lowered.verified, "{}", lowered.text);
    let mut counts = Counts::default();
    for node in nodes(&lowered.wire) {
        if matches!(
            node["semantic_form"].as_str(),
            Some(
                "protocol"
                    | "role"
                    | "relationship"
                    | "channel"
                    | "queue"
                    | "compensation"
                    | "terminal"
                    | "temporal_clause"
                    | "control"
            )
        ) {
            counts.bodies += 1;
            let body = &node["body"];
            if body["term"] == "application" {
                for argument in body["arguments"].as_array().expect("body arguments") {
                    visit(&lowered.wire, argument, &mut counts);
                }
            } else {
                visit(&lowered.wire, body, &mut counts);
            }
        }
    }
    assert!(counts.leaves > 0, "W has stratified Leaf terms");
    assert!(counts.groups > 0, "W has stratified Groups");
    assert!(counts.tuples + counts.groups > 0, "W has Groups or Tuples");
    assert!(counts.members > 0, "W has stratified Members");
    assert!(counts.bodies > 0, "W has protocol Bodies");
}

#[trace("TC-1194", "FR-449-AC-2", "FR-449-AC-4")]
#[test]
fn origin_w_rebinds_the_capture_without_a_trigger() {
    let lowered = lower_with(
        document(),
        PROFILES_W,
        &W.replace("on each (opened: M::OrderStarted)", "on origin")
            .replace(
                "capture order: Integer = opened.order;",
                "capture order: Integer = deref(view).order;",
            ),
    );
    assert!(lowered.verified, "{}", lowered.text);
    let rows = activation_rows(&lowered);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].activation,
        quire_contract_model::CheckedProtocolActivation::Origin
    );
    let clause = node_of(&lowered.wire, "protocol", "protocol_clause");
    let temporal = node_of(&lowered.wire, "temporal", "temporal_clause");
    assert_eq!(
        rows[0].clause,
        structural(&lowered.wire, &clause["node_id"])
    );
    assert_eq!(
        rows[0].required_temporal,
        structural(&lowered.wire, &temporal["node_id"])
    );
    assert_eq!(
        clause_argument(clause, "activation")["members"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[trace("FR-383-AC-2", "FR-383-AC-13", "TC-1061", "TC-1062")]
#[test]
fn w_relationships_are_model_nodes_sorted_and_deduplicated() {
    let mut domain: Value = serde_json::from_slice(&document()).unwrap();
    let mut second = domain["types"][0]["relationships"][0].clone();
    second["identity"] = json!("ix://acme/orders/relationship/ReturnedShipment");
    domain["types"][0]["relationships"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let document = serde_json::to_vec(&domain).unwrap();
    let declaration = "relationship ShipmentOwner = M::OrderShipment;";
    let first = lower_with(
        document.clone(),
        PROFILES_W,
        &W.replace(
            declaration,
            &format!("{declaration}\nrelationship Returns = M::ReturnedShipment;"),
        ),
    );
    let second = lower_with(
        document.clone(),
        PROFILES_W,
        &W.replace(
            declaration,
            &format!("relationship Returns = M::ReturnedShipment;\n{declaration}"),
        ),
    );
    let repeated = lower_with(document, PROFILES_W, &W.replace(declaration, &format!("{declaration}\nrelationship Returns = M::ReturnedShipment;\nrelationship SameShipment = M::OrderShipment;")));
    for lowered in [&first, &second, &repeated] {
        assert!(lowered.verified, "{}", lowered.text);
        assert_eq!(activation_rows(lowered).len(), 1);
        let clause = node_of(&lowered.wire, "protocol", "protocol_clause");
        let keys: Vec<_> = clause_argument(clause, "relationships")["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|member| qsl_foundation::node_wire::node_key_from_json(&member["target"]).unwrap())
            .collect();
        let mut expected: Vec<_> = lowered.wire["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|node| {
                node["node_tag"] == "relation" && node["semantic_form"] == "relationship"
            })
            .map(|node| qsl_foundation::node_wire::node_key_from_json(&node["node_id"]).unwrap())
            .collect();
        expected.sort_unstable();
        assert_eq!(expected.len(), 2);
        assert_eq!(keys, expected);
    }
    assert_eq!(
        first.wire, second.wire,
        "declaration order preserves the package"
    );
    // Source regions differ when an alias is repeated; the clause body and
    // activation map retain exactly the same model node identities.
    assert_eq!(
        clause_argument(
            node_of(&first.wire, "protocol", "protocol_clause"),
            "relationships"
        ),
        clause_argument(
            node_of(&repeated.wire, "protocol", "protocol_clause"),
            "relationships"
        )
    );
    assert_eq!(activation_rows(&first), activation_rows(&repeated));
}

#[trace("TC-1194", "FR-449-AC-6")]
#[test]
fn w_missing_required_temporal_refuses_before_a_package_exists() {
    let document = document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let text = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.model.complete/v1\";\n{PROFILES_W}model M = \"acme/orders\";\n{}", W.replace("requires temporal OrderClosureBounded;", "requires temporal Missing;"));
    let refusals = assemble_with_models(text.as_bytes(), &packages)
        .expect("W assembles")
        .check(CheckingLimits::default())
        .expect_err("no package for a missing requirement");
    let [refusal] = refusals.as_slice() else {
        panic!("one missing temporal: {refusals:?}");
    };
    assert_eq!(
        refusal.cause.code(),
        qsl_foundation::Code::MissingDeclaration
    );
    assert_eq!(refusal.cause.cause(), Some("missing-name"));
    let CheckCause::ProtocolAnchor(cause) = &refusal.cause else {
        panic!("protocol requirement: {refusal:?}");
    };
    let qsl_semantics::check::ProtocolAnchorCause::MissingRequirement { name, span } =
        cause.as_ref()
    else {
        panic!("missing temporal: {cause:?}");
    };
    assert_eq!(name, "Missing");
    assert_eq!(&text[span.start..span.end], "Missing");
}

/// The model profile isolates S3 relationship binding from protocol emission;
/// the complete W test separately selects the finite-global protocol profile.
#[trace("FR-056-AC-5", "FR-218-AC-14", "TC-1056")]
#[test]
fn a_canonical_relationship_binds_through_real_model_intake() {
    let document = document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
                profile P = \"quire.model.complete/v1\";\n\
                profile p = \"quire.protocol.finite-global/v1\";\n\
                model M = \"acme/orders\";\n\
                protocol OrderFlow using p over (view: M::OrderView) on origin {\n\
                role Orders on M::OrderView;\n\
                relationship ShipmentOwner = M::OrderShipment;\n\
                run sequence Main {}\n\
                finish Closed as (closed: Boolean) {true};\n}\n";
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("source assembles")
        .check(CheckingLimits::default())
        .expect("a declared relationship resolves");
    let protocol = graph.protocol("OrderFlow").expect("checked OrderFlow");
    assert_eq!(protocol.relationships.len(), 1);
    assert_eq!(
        protocol.relationships[0].key.node,
        "ix://acme/orders/relationship/OrderShipment"
    );
    let ends: Vec<_> = protocol.relationships[0]
        .ends
        .iter()
        .map(|end| {
            let quire_exact::ValueType::Reference(id) = end else {
                panic!("an object-type relationship end: {end:?}");
            };
            graph
                .object_type_key(id.clone())
                .expect("admitted end declaration")
                .node
                .as_str()
        })
        .collect();
    assert_eq!(
        ends,
        ["ix://acme/orders/OrderView", "ix://acme/orders/Shipment"]
    );
    for (target, code, cause) in [
        (
            "Missing",
            qsl_foundation::Code::MissingDeclaration,
            "missing-name",
        ),
        (
            "Payment",
            qsl_foundation::Code::InvalidModelBinding,
            "wrong-export",
        ),
    ] {
        let text = unit.replace("M::OrderShipment", &format!("M::{target}"));
        let refusals = assemble_with_models(text.as_bytes(), &packages)
            .expect("source assembles")
            .check(CheckingLimits::default())
            .expect_err("invalid relationship target refuses");
        let [refusal] = refusals.as_slice() else {
            panic!("one relationship refusal: {refusals:?}");
        };
        assert_eq!(refusal.cause.code(), code);
        assert_eq!(refusal.cause.cause(), Some(cause));
        let CheckCause::ProtocolAnchor(detail) = &refusal.cause else {
            panic!("relationship refusal: {refusal:?}");
        };
        let span = detail.span();
        assert_eq!(&text[span.start..span.end], format!("M::{target}"));
    }
}

pub(in crate::emit::tests) const TIMED_PROFILES_W: &str =
    "profile P = \"quire.protocol.finite-global/v1\";\n\
                         profile T = \"quire.temporal.event-position.false-extension/v1\";\n";
/// The v2 checked-package event-position clock the timed TC-134 fixture
/// selects for its temporal declarations.
pub(in crate::emit::tests) fn tc134_clock() -> quire_contract_model::ClockConfiguration {
    quire_contract_model::ClockConfiguration::EventPosition {
        sequence_authority: "tc134-order-events".into(),
    }
}

fn tc134_timed_graph() -> qsl_semantics::check::CheckedGraph {
    tc134_timed_graph_with(|source| source)
}

fn tc134_timed_graph_with(
    edit: impl FnOnce(String) -> String,
) -> qsl_semantics::check::CheckedGraph {
    let domain = document();
    let packages =
        qsl_semantics::model::intake::package_input([domain.as_slice()]).expect("model intake");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n{TIMED_PROFILES_W}\
         model M = \"acme/orders\";\n{}\n",
        edit(tc134_source().replace(
            "on origin { always true }",
            "on origin { always[0,3] true }"
        ))
    );
    assemble_with_models(source.as_bytes(), &packages)
        .expect("unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("timed TC-134 checks: {refusals:?}"))
}

/// One clock selection per timed temporal declaration of `graph`, the
/// `compensate` windows included (QSpec FR-322-AC-305).
pub(in crate::emit::tests) fn tc134_selections(
    graph: &qsl_semantics::check::CheckedGraph,
) -> Vec<crate::emit::TemporalClockSelection> {
    let source = graph.source().clone();
    let mut selections = Vec::new();
    for temporal in graph.temporal_clauses() {
        selections.push((
            crate::emit::tests::node_id(temporal.identity()),
            temporal.clause().span(),
        ));
    }
    for hyper in graph.hyper_clauses() {
        let Some(occurrence) = hyper.assembled_occurrence() else {
            continue;
        };
        selections.push((crate::emit::tests::node_id(occurrence.node()), hyper.span()));
    }
    for protocol in graph.protocols() {
        let Some(keys) = graph.protocol_node_keys(&protocol.name) else {
            continue;
        };
        for (index, template) in protocol.obligations.compensations.iter().enumerate() {
            if !matches!(
                template.temporal_profile,
                Some(
                    qsl_semantics::check::TemporalProfile::EventPositionFalseExtension
                        | qsl_semantics::check::TemporalProfile::FixedSampleFalseExtension
                        | qsl_semantics::check::TemporalProfile::TimestampedEventFiniteWindow
                )
            ) {
                continue;
            }
            let Some(key) = keys.compensations.get(&index) else {
                continue;
            };
            selections.push((crate::emit::tests::node_id(key), template.span));
        }
    }
    selections.sort_by(|left, right| left.0.cmp(&right.0));
    selections
        .into_iter()
        .map(|(_, span)| crate::emit::TemporalClockSelection {
            source: source.clone(),
            declaration_span: span,
            clock: tc134_clock(),
        })
        .collect()
}

/// FR-048-AC-8, FR-050-AC-13, FR-322-AC-305: a timed variant of the TC-134
/// fixture emits as strict `/2` with one typed clock binding per explicitly
/// selected temporal declaration (`OrderClosureBounded`) AND one per
/// `compensate` node, keyed by that node's own `node_id`, named by the
/// template's clock text (`refund-clock`), all in the one array in ascending
/// node order. Read-back is asserted by
/// [`tc134_strict_reader_admits_compensation_rows`].
#[trace("FR-048-AC-8", "FR-050-AC-13", "FR-322-AC-305", "TC-134")]
#[test]
fn tc134_timed_fixture_binds_every_compensation_node_and_temporal_declaration() {
    let graph = tc134_timed_graph();
    let selections = tc134_selections(&graph);
    let compensation_count: usize = graph
        .protocols()
        .iter()
        .map(|protocol| protocol.obligations.compensations.len())
        .sum();
    assert_eq!(
        compensation_count, 2,
        "the fixture writes two refund windows"
    );
    let package = CheckedPackage::link(graph);
    let emission = crate::emit::emit_checked_with_temporal_selections(&package, &selections)
        .expect("the timed package emits with its selections");
    let wire = wire(&emission);
    let bindings = wire["temporal_bindings"]
        .as_array()
        .expect("a timed subject carries temporal bindings");
    assert_eq!(
        bindings.len(),
        selections.len(),
        "exactly one per selection"
    );
    let all_nodes = nodes(&wire);
    let position = |id: &Value| {
        all_nodes
            .iter()
            .position(|node| &node["node_id"] == id)
            .expect("the binding is tagged to an emitted node")
    };
    let compensation_ids: Vec<&Value> = all_nodes
        .iter()
        .filter(|node| node["semantic_form"] == "compensation")
        .map(|node| &node["node_id"])
        .collect();
    assert_eq!(compensation_ids.len(), 2);
    for binding in bindings {
        assert_eq!(
            binding["clock"],
            json!({"kind": "event_position", "sequence_authority": "tc134-order-events"})
        );
        let name = binding["clock_name"].as_str().expect("clock name");
        let is_compensation = compensation_ids.contains(&&binding["declaration"]);
        assert_eq!(
            name,
            if is_compensation {
                "refund-clock"
            } else {
                "model-steps"
            },
            "a compensation row names the template's clock text, a clause row its clause's"
        );
    }
    let bound_compensations: Vec<&Value> = bindings
        .iter()
        .map(|binding| &binding["declaration"])
        .filter(|declaration| compensation_ids.contains(declaration))
        .collect();
    assert_eq!(
        bound_compensations.len(),
        2,
        "every compensation node has exactly one row, keyed by its own node_id"
    );
    let declarations: std::collections::BTreeSet<String> = bindings
        .iter()
        .map(|b| b["declaration"].to_string())
        .collect();
    assert_eq!(
        declarations.len(),
        bindings.len(),
        "no declaration is bound twice"
    );
    let order: Vec<usize> = bindings
        .iter()
        .map(|binding| position(&binding["declaration"]))
        .collect();
    assert!(
        order.windows(2).all(|pair| pair[0] < pair[1]),
        "clause and compensation rows share one array in ascending node order: {order:?}"
    );
    assert!(
        wire["lock"]["profile_selections"]
            .as_array()
            .expect("profile selections")
            .iter()
            .any(|selection| selection["role"] == "temporal_profile"
                && selection["definition"]["identity"]
                    == "quire.temporal.event-position.false-extension/v1"),
        "the emitted wire selects the temporal profile with role temporal_profile"
    );
}

/// FR-322-AC-305, FR-322-AC-361, FR-322-AC-364: each `compensate` node has
/// one binding row per `declaration` occurrence it carries, whose `occurrence`
/// is that occurrence's ordinal and whose name is the template clock text.
#[trace("FR-322-AC-305", "FR-322-AC-361", "FR-322-AC-364", "TC-134")]
#[test]
fn tc134_compensation_rows_are_one_per_declaration_occurrence() {
    let graph = tc134_timed_graph();
    let selections = tc134_selections(&graph);
    let emission = crate::emit::emit_checked_with_temporal_selections(
        &CheckedPackage::link(graph),
        &selections,
    )
    .expect("the timed package emits with its selections");
    let wire = wire(&emission);
    let rows = wire["temporal_bindings"].as_array().expect("bindings");
    let mut expected = 0;
    for node in nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "compensation")
    {
        let ordinals: Vec<u64> = node["occurrences"]
            .as_array()
            .expect("occurrences")
            .iter()
            .filter(|occurrence| occurrence["role"] == "declaration")
            .map(|occurrence| occurrence["ordinal"].as_u64().expect("ordinal"))
            .collect();
        assert!(!ordinals.is_empty());
        let mut written: Vec<(u64, String)> = rows
            .iter()
            .filter(|row| row["declaration"] == node["node_id"])
            .map(|row| {
                (
                    row["occurrence"].as_u64().expect("occurrence"),
                    row["clock_name"].as_str().expect("name").to_owned(),
                )
            })
            .collect();
        written.sort();
        assert_eq!(
            written,
            ordinals
                .iter()
                .map(|ordinal| (*ordinal, "refund-clock".to_owned()))
                .collect::<Vec<_>>()
        );
        expected += ordinals.len();
    }
    assert!(expected >= 2);
    assert_eq!(
        rows.iter()
            .filter(|row| row["clock_name"] == "refund-clock")
            .count(),
        expected
    );
}

/// FR-322-AC-305: the strict reader admits a `temporal_bindings` row keyed by
/// a compensation node. The emitted package reads back admitted, and its
/// rows are the written ones: one per selection, each `compensate` node's
/// own `node_id` named by the template clock text (`refund-clock`) with the
/// selected clock configuration.
#[trace("FR-322-AC-305", "TC-134")]
#[test]
fn tc134_strict_reader_admits_compensation_rows() {
    let graph = tc134_timed_graph();
    let selections = tc134_selections(&graph);
    let package = CheckedPackage::link(graph);
    let emission = crate::emit::emit_checked_with_temporal_selections(&package, &selections)
        .expect("the timed package emits with its selections");
    let wire = wire(&emission);
    let all_nodes = nodes(&wire);
    let compensation_ids: Vec<&Value> = all_nodes
        .iter()
        .filter(|node| node["semantic_form"] == "compensation")
        .map(|node| &node["node_id"])
        .collect();
    assert_eq!(compensation_ids.len(), 2);
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(package_identity(&document()[..]), &document()[..])
        .expect("a fresh evidence key");
    let quire_contract_model::CheckedPackageDispatchResult::AdmittedV2(admitted) =
        quire_contract_model::read_checked_package(
            emission.package.bytes(),
            quire_contract_model::CheckedPackageReadLimits::bounded(),
            &evidence,
        )
    else {
        panic!("the strict reader admits compensation rows");
    };
    let rows = admitted.temporal_bindings().expect("read-back bindings");
    assert_eq!(rows.len(), selections.len());
    let compensation_rows: Vec<_> = rows
        .iter()
        .filter(|row| &*row.clock_name == "refund-clock")
        .collect();
    assert_eq!(compensation_rows.len(), 2);
    for row in compensation_rows {
        let declaration = serde_json::to_value(&row.declaration).expect("node id");
        assert!(
            compensation_ids.contains(&&declaration),
            "a read-back row is keyed by a written compensation node: {declaration}"
        );
        assert_eq!(row.clock, tc134_clock());
    }
}

/// FR-048-AC-8, FR-050-AC-14, FR-322-AC-305: a missing, duplicate, surplus or
/// profile-incompatible clock selection for an explicit temporal declaration
/// refuses emission, and so does each of those for a `compensate` window
/// (CB-01 missing, CB-02 duplicate, CB-03 surplus, CB-05 kind mismatch).
#[trace(
    "FR-048-AC-8",
    "FR-050-AC-14",
    "FR-322-AC-305",
    "TC-134",
    "TC-1308",
    "FR-513-AC-9"
)]
#[test]
fn tc134_explicit_temporal_declaration_refuses_each_unfit_clock_selection() {
    use crate::emit::EmitRefusal as R;
    let graph = tc134_timed_graph();
    let selections = tc134_selections(&graph);
    let package = CheckedPackage::link(graph);
    let emit = |s: &[crate::emit::TemporalClockSelection]| {
        crate::emit::emit_checked_with_temporal_selections(&package, s)
    };

    assert!(matches!(
        emit(&[]),
        Err(R::MissingTemporalClockSelection { .. })
    ));
    let mut duplicate = selections.clone();
    duplicate.push(selections[0].clone());
    assert!(matches!(
        emit(&duplicate),
        Err(R::DuplicateTemporalClockSelection { .. })
    ));
    let mut surplus = selections.clone();
    surplus.push(crate::emit::TemporalClockSelection {
        declaration_span: qsl_foundation::Span { start: 0, end: 1 },
        ..selections[0].clone()
    });
    assert!(matches!(
        emit(&surplus),
        Err(R::SurplusTemporalClockSelection { .. })
    ));
    let mut incompatible = selections.clone();
    incompatible[0].clock = quire_contract_model::ClockConfiguration::TimestampedEvent {
        timestamp_unit: "tc134-rfc3339".into(),
    };
    assert!(matches!(
        emit(&incompatible),
        Err(R::TemporalClockProfileMismatch { .. })
    ));

    // A `compensate` window is a selected declaration of its own.
    let compensation_spans: Vec<qsl_foundation::Span> = package
        .graph()
        .protocols()
        .iter()
        .flat_map(|protocol| protocol.obligations.compensations.iter().map(|c| c.span))
        .collect();
    let compensation_at = |selections: &[crate::emit::TemporalClockSelection], span| {
        selections
            .iter()
            .position(|selection| selection.declaration_span == span)
            .expect("a compensation selection")
    };
    let first = compensation_spans[0];

    let mut missing = selections.clone();
    missing.remove(compensation_at(&selections, first));
    assert!(
        matches!(
            emit(&missing),
            Err(R::MissingTemporalClockSelection { span, .. }) if span == first
        ),
        "a compensation node without its row refuses"
    );
    let mut duplicate = selections.clone();
    duplicate.push(selections[compensation_at(&selections, first)].clone());
    assert!(
        matches!(
            emit(&duplicate),
            Err(R::DuplicateTemporalClockSelection { span, .. }) if span == first
        ),
        "two rows for one compensation node refuse"
    );
    let mut surplus = selections.clone();
    surplus.push(crate::emit::TemporalClockSelection {
        declaration_span: qsl_foundation::Span { start: 0, end: 1 },
        ..selections[0].clone()
    });
    assert!(matches!(
        emit(&surplus),
        Err(R::SurplusTemporalClockSelection { .. })
    ));
    let mut incompatible = selections.clone();
    let at = compensation_at(&selections, first);
    incompatible[at].clock = quire_contract_model::ClockConfiguration::TimestampedEvent {
        timestamp_unit: "tc134-rfc3339".into(),
    };
    assert!(
        matches!(
            emit(&incompatible),
            Err(R::TemporalClockProfileMismatch { span, .. }) if span == first
        ),
        "a compensation clock of another kind than its profile refuses"
    );
}

/// FR-508: a protocol profile is not an admissible compensation profile.
#[trace("FR-508-AC-4", "TC-1303")]
#[test]
fn tc134_compensation_rejects_a_protocol_profile_alias() {
    let domain = document();
    let packages =
        qsl_semantics::model::intake::package_input([domain.as_slice()]).expect("model intake");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n{TIMED_PROFILES_W}\
         model M = \"acme/orders\";\n{}\n",
        tc134_source().replace(
            "using T clock \"refund-clock\"",
            "using P clock \"refund-clock\"",
        )
    );
    let refusals = assemble_with_models(source.as_bytes(), &packages)
        .expect("unit assembles")
        .check(CheckingLimits::default())
        .expect_err("a protocol profile cannot select a compensation clock role");
    assert!(refusals.iter().any(|refusal| matches!(
        &refusal.cause,
        qsl_semantics::check::CheckCause::ProtocolAnchor(cause)
            if matches!(cause.as_ref(), qsl_semantics::check::ProtocolAnchorCause::WrongProfileKind { alias, required, .. } if alias == "P" && *required == "temporal profile")
    )), "{refusals:?}");
}

/// FR-322-AC-305: a template with a clock profile but no written node has
/// nothing to bind, so it needs no selection and a selection naming its
/// window is surplus. The unit carries no timed clause, so the only
/// candidates are the two `compensate` windows, and no node is written.
#[trace("FR-322-AC-305", "TC-134")]
#[test]
fn tc134_template_without_a_written_node_writes_no_row() {
    use crate::emit::EmitRefusal as R;
    let graph = tc134_timed_graph_with(|source| {
        let start = source.find("temporal OrderClosureBounded").expect("clause");
        let end = source[start..]
            .find("protocol OrderFlow")
            .expect("protocol")
            + start;
        let mut source = source;
        source.replace_range(start..end, "");
        source.replacen("  requires temporal OrderClosureBounded;\n", "", 1)
    });
    assert!(graph.temporal_clauses().is_empty());
    let selections = tc134_selections(&graph);
    assert_eq!(selections.len(), 2, "one selection per refund window");
    let none = std::collections::HashMap::new();
    let rows = crate::emit::temporal_binding_rows(&graph, &none, &[], &[])
        .expect("no written node needs no selection");
    assert!(rows.bindings.is_empty() && rows.names.is_empty());
    assert!(matches!(
        crate::emit::temporal_binding_rows(&graph, &none, &[], &selections),
        Err(R::SurplusTemporalClockSelection { span, .. }) if selections.iter().any(|s| s.declaration_span == span)
    ));
}

/// FR-048-AC-8: `/2` bytes carrying clock bindings whose `contract_version`
/// is relabelled `/1` are refused with the generic `UnsupportedVersion`
/// refusal. This shows only that the reader admits no non-`/2` version.
#[trace("FR-048-AC-8", "TC-134")]
#[test]
fn tc134_v1_contract_version_is_refused_with_clock_selections_present() {
    let graph = tc134_timed_graph();
    let selections = tc134_selections(&graph);
    let package = CheckedPackage::link(graph);
    let emission = crate::emit::emit_checked_with_temporal_selections(&package, &selections)
        .expect("the timed package emits");
    let mut forged = wire(&emission);
    assert!(forged["temporal_bindings"].is_array());
    forged["contract_version"] = json!("quire.checked-package/v1");
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(package_identity(&document()[..]), &document()[..])
        .expect("a fresh evidence key");
    let outcome = read_v2(
        &serde_json::to_vec(&forged).unwrap(),
        library(),
        V2ReadLimits::default(),
        &evidence,
    );
    assert!(
        matches!(
            &outcome,
            Read::Refused(V2ReadRefusal::UnsupportedVersion { wire, .. })
                if &*wire.actual == "quire.checked-package/v1"
        ),
        "expected /1 refusal, got {outcome:?}"
    );
}

/// An unrelated protocol `name` over W's domain: it shares no declaration
/// with W.
fn unrelated_protocol(name: &str) -> String {
    format!(
        "protocol {name} using P over (view: M::OrderView) on origin {{\n\
         role {name}Clerk on M::OrderView;\n\
         run sequence {name}Steps {{ }}\n\
         finish {name}Done as (outcome: Boolean) {{ true }};\n\
         }}\n"
    )
}

/// W between two [`unrelated_protocol`]s, one declared before it and one
/// after it.
fn w_between_unrelated_protocols() -> Lowered {
    let lowered = lower_with(
        document(),
        PROFILES_W,
        &format!(
            "{}{W}{}",
            unrelated_protocol("Before"),
            unrelated_protocol("After")
        ),
    );
    assert!(lowered.verified, "{}", lowered.text);
    lowered
}

/// The nodes W's clause reaches through `dependencies`, the clause included,
/// by structural id, and W's own protocol nodes among them, each as `view`
/// renders it. A shared node such as the literal `true` is reached but not
/// W's own: its occurrences list every construct that writes it.
fn w_closure(
    wire: &Value,
    view: impl Fn(&WireGraph, &Value) -> Value,
) -> (BTreeSet<CheckedNodeId>, BTreeMap<CheckedNodeId, Value>) {
    let graph = WireGraph::of_package_value(wire).expect("the emitted graph");
    let clause = nodes(wire)
        .iter()
        .find(|node| {
            node["semantic_form"] == "protocol_clause"
                && node["declaration"]["qualified_name"][0] == "OrderFlow"
        })
        .expect("W's clause");
    let mut reached = BTreeSet::new();
    let mut own = BTreeMap::new();
    let mut pending = vec![clause];
    while let Some(node) = pending.pop() {
        let key = graph.key_of_value(&node["node_id"]).expect("an emitted id");
        let id = qsl_semantics::check::wire_id::structural_id(&key);
        if !reached.insert(id.clone()) {
            continue;
        }
        if node["node_tag"] == "protocol" {
            own.insert(id, view(&graph, node));
        }
        for dependency in node["dependencies"].as_array().expect("dependencies") {
            pending.push(node_by_id(wire, dependency));
        }
    }
    (reached, own)
}

/// Assert that W's clause reaches the same nodes in `alone` and `around`,
/// and that `view` renders each of W's protocol nodes the same in both.
fn assert_same_w_nodes(alone: &Value, around: &Value, view: impl Fn(&WireGraph, &Value) -> Value) {
    let (reached, expected) = w_closure(alone, &view);
    let (reached_around, actual) = w_closure(around, &view);
    assert_eq!(reached, reached_around, "W reaches the same nodes");
    assert!(
        expected.len() > 20,
        "W's protocol nodes: {}",
        expected.len()
    );
    for (id, node) in &expected {
        assert_eq!(
            serde_json::to_string(node).expect("a node serializes"),
            serde_json::to_string(&actual[id]).expect("a node serializes"),
            "the bytes of {id:?}"
        );
    }
}

/// `value` with every node id resolved against `graph` to its structural
/// id, so the rendering names no graph position.
fn resolved(graph: &WireGraph, value: &Value) -> Value {
    match value {
        Value::Object(object)
            if object.len() == 3
                && ["package", "path", "ordinal"]
                    .iter()
                    .all(|name| object.contains_key(*name)) =>
        {
            let key = graph
                .key_of_value(value)
                .expect("an id of the emitted graph");
            serde_json::to_value(qsl_semantics::check::wire_id::structural_id(&key))
                .expect("a structural id serializes")
        }
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(name, member)| (name.clone(), resolved(graph, member)))
                .collect(),
        ),
        Value::Array(items) => {
            Value::Array(items.iter().map(|item| resolved(graph, item)).collect())
        }
        other => other.clone(),
    }
}

/// TC-1062 row 10, the part FR-322's graph-position operand indices allow:
/// W emitted twice is byte-identical, and W between unrelated protocols
/// reaches the same nodes, and each of W's protocol nodes has the same bytes
/// as W's own once its ids are resolved against the graph that holds it. The clause therefore does not change
/// when only unrelated declarations do.
#[trace("TC-1062")]
#[test]
fn w_nodes_do_not_follow_unrelated_protocols() {
    let alone = lower_with(document(), PROFILES_W, W);
    assert!(alone.verified, "{}", alone.text);
    let again = lower_with(document(), PROFILES_W, W);
    assert_eq!(alone.wire, again.wire, "W emitted twice");
    assert_same_w_nodes(&alone.wire, &w_between_unrelated_protocols().wire, resolved);
}

/// FR-383-AC-15's byte clause: W between unrelated protocols writes the same
/// wire bytes for each of its protocol nodes as W alone. QSpec FR-322
/// writes each operand as its index in the whole document's graph, so nodes
/// the unrelated protocols add ahead of W's operands shift those indices.
#[ignore = "gated on STD-653 (tracker QSL-1695): FR-322 operand index segments are document graph positions"]
#[test]
fn w_wire_bytes_do_not_follow_unrelated_protocols() {
    let alone = lower_with(document(), PROFILES_W, W);
    assert!(alone.verified, "{}", alone.text);
    assert_same_w_nodes(
        &alone.wire,
        &w_between_unrelated_protocols().wire,
        |_: &WireGraph, node: &Value| node.clone(),
    );
}
