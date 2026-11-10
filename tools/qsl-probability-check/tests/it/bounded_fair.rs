// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fair bounded proof bytes checked without analyzer linkage in a fresh process.
use super::bounded::{alter_keys, document, mutate, proof, request_with, run};
use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::ExactProbLimits;
use serde_json::{json, Value};

const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");
const DOMAIN: &[u8] = include_bytes!("../fixtures/link.semantic-ir.json");

fn limits() -> qsl_replay::ProbabilityDocumentLimits {
    qsl_replay::ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 16_000_000,
        states: 128,
        edges: 1024,
        scalar_bits: 128,
        accounting: qsl_replay::AccountingLimits::default().0,
    }
}

fn source(
    fairness: &str,
    operator: &str,
    threshold: &str,
    horizon: u64,
    predicate: &str,
) -> String {
    let model = include_str!("../fixtures/link.native").replace(
        "quire.temporal.infinite-trace/v1",
        "quire.temporal.event-position.false-extension/v1",
    );
    format!("{model}\nprobabilistic Likely using inf on M over (m: M::Msg) under every scheduler {fairness} on origin {{ probability {operator} {threshold} [ eventually[0,{horizon}] holds({predicate}) ] }}\n")
}
fn replace_source(request: &mut Value, source: &str) {
    request["source"]["content"] = json!(source);
    request["byte_provision"][0][2] = json!(source.as_bytes());
}

#[trace("TC-1406", "FR-201-AC-47", "FR-201-AC-49")]
#[test]
fn weak_strong_whole_each_preserve_actual_minimum_and_both_bounded_extrema() {
    for fairness in [
        "fair weak Msg::send_a",
        "fair strong Msg::send_a",
        "fair weak each Msg::send_a",
        "fair strong each Msg::send_a",
    ] {
        let src = source(fairness, ">=", "rational(19,20)", 2, "m.delivered");
        let request = proof(&src, SNAPSHOT);
        let doc = document(&request);
        assert_eq!(
            doc["objective"]["profile"],
            "quire.probability-objective/bounded-fair-eventual/v1"
        );
        assert_eq!(doc["objective"]["fairness"].as_array().unwrap().len(), 1);
        let (code, out) = run(&request);
        assert_eq!(code, 0, "{fairness}: {out}");
        assert_eq!(out["value"], json!({"numerator":"24","denominator":"25"}));
        let mut beyond = request.clone();
        replace_source(
            &mut beyond,
            &src.replace("rational(19,20)", "rational(97,100)"),
        );
        assert_eq!(run(&beyond).1["rule"], "threshold-missed");
    }
    let src = source(
        "fair strong each Msg::send_a",
        "<=",
        "rational(9,10)",
        1,
        "m.delivered",
    )
    .replace("self.attempts < 2", "self.attempts < 1")
    .replace("true: 1, false: 9", "true: 2, false: 3");
    let request = proof(&src, SNAPSHOT);
    assert_eq!(
        run(&request).1["value"],
        json!({"numerator":"4","denominator":"5"})
    );
    let mut miss = request.clone();
    replace_source(&mut miss, &src.replace("rational(9,10)", "rational(3,4)"));
    assert_eq!(run(&miss).1["rule"], "threshold-missed");
}

#[trace("TC-1406", "FR-201-AC-48")]
#[test]
fn checked_set_normalizes_authored_order_and_duplicates_but_wire_set_is_strict() {
    let both = proof(
        &source(
            "fair weak Msg::send_a fair strong Msg::send_a",
            ">=",
            "0.95",
            2,
            "m.delivered",
        ),
        SNAPSHOT,
    );
    let set = document(&both)["objective"]["fairness"].clone();
    assert_eq!(set.as_array().unwrap().len(), 2);
    assert_ne!(set[0]["fairness_kind"], set[1]["fairness_kind"]);
    assert_eq!(run(&both).0, 0);
    let a = source(
        "fair weak Msg::send_a fair strong each Msg::send_b fair weak Msg::send_a",
        ">=",
        "0.95",
        2,
        "m.delivered",
    );
    let b = source(
        "fair strong each Msg::send_b fair weak Msg::send_a",
        ">=",
        "0.95",
        2,
        "m.delivered",
    );
    let request = proof(&a, SNAPSHOT);
    assert_eq!(
        document(&request)["objective"]["fairness"],
        document(&proof(&b, SNAPSHOT))["objective"]["fairness"]
    );
    assert_eq!(run(&request).0, 0);
    for change in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] {
        let malformed = matches!(change, 0..=4);
        let changed = mutate(&request, |doc| {
            let set = doc["objective"]["fairness"].as_array_mut().unwrap();
            match change {
                0 => set.reverse(),
                1 => set.insert(0, set[0].clone()),
                2 => {
                    set[0]["extra"] = json!(true);
                }
                3 => {
                    set[0]["target"]["kind"] = json!("protocol");
                }
                4 => {
                    set[0]["target"].as_object_mut().unwrap().remove("name");
                }
                5 => {
                    set.pop();
                }
                6 => {
                    set[0]["fairness_kind"] = json!("weak");
                }
                7 => {
                    set[0]["granularity"] = json!("whole");
                }
                8 => {
                    set[0]["target"]["name"] = json!("absent");
                }
                9 => {
                    set[0]["target"]["declaration"]["package"] = json!("foreign");
                }
                10 => {
                    set[0]["target"]["declaration"]["ordinal"] = json!(1);
                }
                11 => {
                    set[0]["target"]["declaration"]["path"][2] = json!("foreign-owner");
                }
                12 => {
                    set.clear();
                }
                _ => unreachable!(),
            }
        });
        let (code, out) = run(&changed);
        assert_ne!(code, 0, "mutation {change}: {out}");
        if malformed {
            assert_eq!(out["rule"], "malformed", "mutation {change}: {out}");
        }
    }
}

#[trace("TC-1406", "FR-201-AC-48", "FR-201-AC-49")]
#[test]
fn inherited_operation_uses_declaring_model_identity_and_rejects_consumer_identity() {
    let mut domain: Value = serde_json::from_slice(DOMAIN).unwrap();
    let types = domain["types"].as_array_mut().unwrap();
    let msg = types
        .iter_mut()
        .find(|t| t["identity"] == "ix://test/link/Msg")
        .unwrap();
    let mut base: Value =
        serde_json::from_str(&msg.to_string().replace("test/link/Msg/", "test/link/Base/"))
            .unwrap();
    base["identity"] = json!("ix://test/link/Base");
    base["displayName"] = json!("Base");
    msg["supertypes"] = json!(["ix://test/link/Base"]);
    msg["fields"] = json!([]);
    msg["operations"] = json!([]);
    types.push(base);
    let src = source("fair weak each Msg::send_a", ">=", "0.95", 2, "m.delivered");
    let domain = serde_json::to_vec(&domain).unwrap();
    let (request, settlement) = request_with(&src, SNAPSHOT, ExactProbLimits::default(), &domain);
    let request =
        request.unwrap_or_else(|| panic!("inherited source must produce evidence: {settlement:?}"));
    assert_eq!(run(&request).0, 0);
    let target = document(&request)["objective"]["fairness"][0]["target"].clone();
    assert_eq!(
        target["declaration"]["path"],
        json!(["model", "object_type", "ix://test/link/Base"])
    );
    let consumer = mutate(&request, |doc| {
        doc["objective"]["fairness"][0]["target"]["declaration"]["path"][2] =
            json!("ix://test/link/Msg")
    });
    assert_ne!(run(&consumer).0, 0);
}

#[trace("TC-1406", "FR-201-AC-47", "FR-201-AC-49")]
#[test]
fn zero_horizon_and_legacy_bytes_keep_their_actual_context_and_closed_rows() {
    for (predicate, op, expected) in [("not m.delivered", ">=", "1"), ("m.delivered", "<=", "0")] {
        let request = proof(
            &source("fair weak Msg::send_a", op, "0.5", 0, predicate),
            SNAPSHOT,
        );
        assert_eq!(
            run(&request).1["value"],
            json!({"numerator":expected,"denominator":"1"})
        );
        assert!(document(&request)["ranking"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["rank"] == 0));
    }
    let ordinary = proof(&source("", ">=", "0.95", 2, "m.delivered"), SNAPSHOT);
    let zero_source = source("fair weak Msg::send_a", "<=", "0.5", 0, "m.delivered");
    let mut unresolved = proof(&zero_source, SNAPSHOT);
    let unresolved_source = zero_source.replace(
        "self.attempts - pre(self.attempts) = 1 and self.delivered = (not lost)",
        "self.attempts < 2",
    );
    replace_source(&mut unresolved, &unresolved_source);
    let (code, out) = run(&unresolved);
    assert_ne!(
        code, 0,
        "horizon zero must not waive the admitted model's unique-post-state restriction: {out}"
    );
    let query_stops = proof(
        &source(
            "fair weak Msg::send_a",
            "<=",
            "0.5",
            0,
            "if m.attempts > 0 then (1 mod 0 = 0) else m.delivered",
        ),
        SNAPSHOT,
    );
    assert_eq!(
        run(&query_stops).0,
        0,
        "model admission must not evaluate P after its zero horizon"
    );
    let bytes = ordinary["certificate"].as_str().unwrap().as_bytes();
    let decoded =
        qsl_replay::read_probability_proof_document(bytes, limits(), &quire_exact::Cancel::new())
            .unwrap();
    assert!(!decoded.is_bounded_fair());
    assert_eq!(
        qsl_replay::write_probability_proof_document(
            &decoded,
            limits(),
            &quire_exact::Cancel::new()
        )
        .unwrap(),
        bytes
    );
    let request = proof(
        &source("fair weak Msg::send_a", ">=", "0.95", 2, "m.delivered"),
        SNAPSHOT,
    );
    let wrong_context = mutate(&request, |doc| {
        alter_keys(doc, |key| key[3][1][1][1] = json!("foreign-context"))
    });
    assert_ne!(run(&wrong_context).0, 0);
    for group in ["values", "ranking"] {
        let missing = mutate(&request, |doc| {
            doc[group].as_array_mut().unwrap().pop();
        });
        assert_ne!(run(&missing).0, 0);
    }
    let false_rank = mutate(&request, |doc| doc["ranking"][0]["rank"] = json!(99));
    assert_eq!(run(&false_rank).1["rule"], "rank-broken");
}
