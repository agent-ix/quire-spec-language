// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared TC-1070 E/L source and semantic-IR model fixture.
use serde_json::{json, Value as Json};
#[path = "tc235_k2.rs"]
#[allow(
    dead_code,
    clippy::duplicate_mod,
    reason = "shared semantic-IR fixture constructors also used by other integration fixtures"
)]
pub(crate) mod model;

pub(crate) const SOURCE: &str = r#"
language "ix:native" edition "1-draft";
profile P = "quire.protocol.finite-global/v1";
profile T = "quire.temporal.infinite-trace/v1";
model M = "test/orders";
protocol Refunds using P over (view: M::Node)
    on each (opened: M::Tick) when (opened.ready) {
  capture order: M::Total = view.total;
  role Service on M::Node;
  compensate Refund for Main::Applied as (forward: M::Receipt)
      by Service on M::Node::step using T clock "refund-clock" {
    capture target: M::Total = forward.total;
    activate first (trigger: M::Flag)
        when { not trigger.ready and target >= 0 } {
      capture activated: Boolean = trigger.ready;
    }
    within [0,30]; attempts 3 of M::Attempt;
    retry (earlier: M::Attempt, later: M::Attempt) {
      earlier.tally < 3 and later.tally = earlier.tally + 1
      and target >= 0 and not activated
    };
    commit Main::Committed;
    recover (recovered: M::Receipt) { recovered.total = target and not activated };
  }
  compensate Partial for Main::Applied as (partialForward: M::Receipt)
      by Service on M::Node::step using T clock "partial-clock" {
    capture partialTarget: M::Total = partialForward.total;
    activate first (partialTrigger: M::Flag)
        when { not partialTrigger.ready and partialTarget >= 0 } {
      capture partialActivated: Boolean = partialTrigger.ready;
    }
    within [0,30]; attempts 3 of M::Attempt;
    retry (partialEarlier: M::Attempt, partialLater: M::Attempt) {
      partialEarlier.tally < 3 and partialLater.tally = partialEarlier.tally + 1
      and partialTarget >= 0 and not partialActivated
    };
    commit never;
    recover (partialRecovered: M::Receipt) {
      partialRecovered.total = partialTarget and not partialActivated
    };
  }
  run sequence Main {
    attempt Tried by Service on M::Node::step contracts []
        as (attempted: M::Flag) { attempted.ready };
    effect Applied of Main::Tried as (applied: M::Receipt) { order >= 0 };
    commit Committed by Service as (committed: M::Flag) { committed.ready };
  }
  finish Closed as (closed: M::Receipt) { true };
}
protocol Pipeline using P over (view: M::Node) on origin {
  role Seller on M::Node;
  role Buyer on M::Node;
  relationship Owns = M::owns;
  channel Pay from Seller to Buyer carries M::Flag
      ordering unordered delivery at-least-once capacity 2 overflow block;
  run sequence Main {
    send Sent via Pay as (sent: M::Flag) { true };
    receive Got via Pay of Main::Sent as (got: M::Flag)
        related by Owns (view, view) { true };
    choice Decide by Buyer visible (got.ready) {
      case yes when { got.ready } check A using P { true };
      case no when { not got.ready } check B using P { true };
    }
  }
  finish Closed as (closed: M::Flag) { true };
}
"#;

pub(crate) fn document() -> Json {
    use qsl_semantics::model::intake::meaning;
    let bounded = |name: &str, upper: &str| {
        let mut ty = model::declaration(name, "bounded", json!({"scalar": "integer"}));
        ty["constraints"] = Json::Array([("min", "0"), ("max", upper)].into_iter().map(|(keyword, value)| json!({
            "identity": model::id(&format!("{name}/constraints/{keyword}")),
            "keyword": keyword, "operands": {"value": value},
            "appliesTo": "ix://quire/native/Integer", "diagnosticCode": format!("bound.{keyword}"),
            "origin": model::origin(name)
        })).collect());
        ty
    };
    let record = |name: &str, fields: &[(&str, &str)]| {
        model::declaration(
            name,
            "record",
            json!({
                "fields": fields.iter().map(|(field, ty)| model::field(name, field, ty)).collect::<Vec<_>>()
            }),
        )
    };
    let mut node = model::declaration(
        "Node",
        "thing",
        json!({
            "identityFields": [model::id("Node/total")],
            "fields": [model::field("Node", "total", &model::id("Total"))],
            "operations": [model::operation("Node", "step")],
            "relationships": [{
                "identity": model::id("relationship/owns"), "category": "structural", "composite": false,
            "direction": "source-to-target", "origin": {"source": {"sourceIdentity": model::id("spec"), "path": "model.qspec", "startLine": 1, "startColumn": 1}},
                "sourceEnd": {"type": model::id("Node"), "role": "owner", "multiplicity": {"lower": 0, "upper": 1, "ordered": false, "unique": true}},
                "targetEnd": {"type": model::id("Node"), "role": "owned", "multiplicity": {"lower": 0, "upper": 1, "ordered": false, "unique": true}}
            }]
        }),
    );
    node["operations"][0]["returns"] = json!({"typeRef": model::id("Flag"), "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true}, "nullable": false});
    let mut bounded_kind = model::construct("bounded", meaning::VALUE_TYPE);
    bounded_kind["construct"]["shape"] = json!("record");
    bounded_kind["construct"]["members"] = json!({});
    let mut doc = model::domain_document(
        vec![
            bounded("Total", "1000"),
            bounded("Tally", "3"),
            bounded("Count", "9"),
            node,
            record("Receipt", &[("total", &model::id("Total"))]),
            record("Attempt", &[("tally", &model::id("Tally"))]),
            record("Flag", &[("ready", "ix://quire/native/Boolean")]),
            record(
                "Tick",
                &[
                    ("ready", "ix://quire/native/Boolean"),
                    ("count", &model::id("Count")),
                ],
            ),
        ],
        vec![
            bounded_kind,
            model::construct("thing", meaning::OBJECT_TYPE),
            model::construct("record", meaning::RECORD_VALUE_TYPE),
            model::construct("population", meaning::POPULATION),
        ],
    );
    doc["populations"] = json!([{"identity": model::id("nodes"), "displayName": "nodes", "kind": {"module": model::BUSINESS, "name": "population"}, "members": [model::id("Node")], "extent": "closed", "origin": model::origin("nodes")}]);
    doc
}

/// TC-1070's E-P reference-provenance model extension. Kept separate from
/// `document` so the older E/L tests continue to exercise their original
/// fixture shape.
pub(crate) fn document_ep() -> Json {
    let mut doc = document();
    let optional = |mut field: Json| {
        field["presence"] = json!("optional");
        field
    };
    let other = model::declaration(
        "Other",
        "thing",
        json!({
            "identityFields": [model::id("Other/marker")],
            "fields": [model::field("Other", "marker", "ix://quire/native/Boolean"),
                       optional(model::field("Other", "next", &model::id("Other")))],
            "relationships": [{
                "identity": model::id("relationship/otherLinks"), "category": "structural", "composite": false,
                "direction": "source-to-target", "origin": {"source": {"sourceIdentity": model::id("spec"), "path": "model.qspec", "startLine": 1, "startColumn": 1}},
                "sourceEnd": {"type": model::id("Other"), "role": "source", "multiplicity": {"lower": 0, "upper": 1, "ordered": false, "unique": true}},
                "targetEnd": {"type": model::id("Other"), "role": "target", "multiplicity": {"lower": 0, "upper": 1, "ordered": false, "unique": true}}
            }]
        }),
    );
    let elsewhere = model::declaration(
        "Elsewhere",
        "thing",
        json!({"identityFields": [model::id("Elsewhere/marker")], "fields": [model::field("Elsewhere", "marker", "ix://quire/native/Boolean")]}),
    );
    let mut sequence = model::field("PeerBundle", "sequence", &model::id("Other"));
    sequence["multiplicity"] = json!({"lower": 1, "upper": 2, "ordered": true, "unique": false});
    let peer_bundle = model::declaration(
        "PeerBundle",
        "record",
        json!({"fields": [model::field("PeerBundle", "nested", &model::id("Other")),
                    optional(model::field("PeerBundle", "optional", &model::id("Other"))), sequence] }),
    );
    let mut receipt = doc["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|ty| ty["displayName"] == "Receipt")
        .unwrap()
        .clone();
    receipt["fields"].as_array_mut().unwrap().extend([
        model::field("Receipt", "peer", &model::id("Other")),
        model::field("Receipt", "bundle", &model::id("PeerBundle")),
    ]);
    let types = doc["types"].as_array_mut().unwrap();
    let receipt_index = types
        .iter()
        .position(|ty| ty["displayName"] == "Receipt")
        .unwrap();
    types[receipt_index] = receipt;
    types.extend([other, elsewhere, peer_bundle]);
    doc["populations"].as_array_mut().unwrap().extend([
        json!({"identity": model::id("otherNodes"), "displayName": "otherNodes", "kind": {"module": model::BUSINESS, "name": "population"}, "members": [model::id("Other")], "extent": "closed", "origin": model::origin("otherNodes")}),
        json!({"identity": model::id("otherSpare"), "displayName": "otherSpare", "kind": {"module": model::BUSINESS, "name": "population"}, "members": [model::id("Other")], "extent": "closed", "origin": model::origin("otherSpare")}),
        json!({"identity": model::id("elsewhereNodes"), "displayName": "elsewhereNodes", "kind": {"module": model::BUSINESS, "name": "population"}, "members": [model::id("Elsewhere")], "extent": "closed", "origin": model::origin("elsewhereNodes")}),
    ]);
    doc
}
