// SPDX-License-Identifier: AGPL-3.0-or-later
//! The complete TC-1061 W choreography used by TC-1194's activation cases.

use super::*;
use crate::checked_v2::V2ReadRefusal;
use quire_contract_model::{CheckedPackageRefusalCause, CheckedPackageRefusalCode};

const PROFILES_W: &str = "profile P = \"quire.protocol.finite-global/v1\";\n\
                         profile T = \"quire.temporal.infinite-trace/v1\";\n";

/// W's order, payment, shipment and refund domain, using the test domain's
/// existing semantic-IR envelope and object declaration shape.
fn document() -> Vec<u8> {
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

const W: &str = r#"
pre CanCharge using v on M::Payment::charge { true }
post ChargePost using v on M::Payment::charge { true }
temporal OrderClosureBounded using T over (view: M::OrderView) clock "model-steps"
  on origin { always true }
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
fn tc134_source() -> String {
    let source = W.to_owned();
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
        qsl_foundation::SourceIdentity::new("agent-ix", "tc134-overflow", "fixture", "fixture:1"),
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
    "FR-449-AC-5"
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
        format!("temporal RefundBounded using T over (view: M::OrderView) clock \"refund-clock\" on origin {{ eventually false }}\n{W}"),
        format!("temporal Unrequired using T over (view: M::OrderView) clock \"model-steps\" on origin {{ eventually false }}\n{W}"),
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
    let two = lower_with(document(), PROFILES_W, &format!("temporal OrderPaidBounded using T over (view: M::OrderView) clock \"model-steps\" on origin {{ eventually false }}\n{}", W.replace("requires temporal OrderClosureBounded;", "requires temporal OrderClosureBounded;\nrequires temporal OrderPaidBounded;")));
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
                model M = \"acme/orders\";\n\
                protocol OrderFlow using P over (view: M::OrderView) on origin {\n\
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
