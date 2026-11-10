// SPDX-License-Identifier: AGPL-3.0-or-later
//! First-hit cyclic reward documents checked without analyzer linkage.
use super::bounded::{document, key, mutate, request_with, run};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::ExactProbLimits;
use serde_json::{json, Value};

const DOMAIN: &[u8] = include_bytes!("../fixtures/reward.semantic-ir.json");
const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");
fn source(cost: &str, bound: &str, exit: bool, target: &str) -> String {
    format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.model.complete/v1\";\nprofile inf = \"quire.temporal.infinite-trace/v1\";\nmodel M = \"test/link\";\npre A using v on M::Msg::send_a {{ self.attempts = 0 }}\npost A_done using v on M::Msg::send_a {{ ((lost and self.attempts = 0) or (not lost and self.attempts = {})) and self.delivered = (not lost and {}) }}\npre B using v on M::Msg::send_b {{ self.attempts = 1 }}\npost B_done using v on M::Msg::send_b {{ self.attempts = 2 and not self.delivered }}\npre C using v on M::Msg::send_c {{ self.attempts = 2 }}\npost C_done using v on M::Msg::send_c {{ self.attempts = 1 and self.delivered }}\nrandom Msg::send_a(lost) ~ {{ true: 1, false: 1 }};\nrandom Msg::send_b(lost) ~ {{ true: 1, false: 1 }};\nrandom Msg::send_c(lost) ~ {{ true: 1, false: 1 }};\nreward cost on Msg::send_a = {cost};\nreward cost on Msg::send_b = 100;\nreward cost on Msg::send_c = 100;\nworkload Even on M {{ weight Msg::send_a = 1; weight Msg::send_b = 1; weight Msg::send_c = 1; }}\nprobabilistic Likely using inf on M over (m: M::Msg) under Even on origin {{ expected accumulate cost until holds({target}) {bound} }}\n",u32::from(exit),if exit {"true"}else{"false"})
}
fn proof(source: &str) -> Value {
    let (request, settlement) = request_with(source, SNAPSHOT, ExactProbLimits::default(), DOMAIN);
    request.unwrap_or_else(|| panic!("expected cyclic reward proof {settlement:?}\n{source}"))
}
fn replace_source(request: &mut Value, source: &str) {
    request["source"]["content"] = json!(source);
    request["byte_provision"][0][2] = json!(source.as_bytes());
}
#[trace("TC-1403", "FR-201-AC-35", "FR-201-AC-36", "FR-201-AC-37")]
#[test]
fn cyclic_reward_roundtrip_stops_before_post_target_cycles() {
    let request = proof(&source(
        "if lost then 1 else 3",
        "<= 4",
        true,
        "m.delivered",
    ));
    let d = document(&request);
    assert_eq!(
        d["objective"]["profile"],
        "quire.probability-objective/dtmc-expected-reward/v1"
    );
    assert_eq!(d["values"].as_array().unwrap().len(), 2);
    let (code, output) = run(&request);
    assert_eq!(code, 0, "{output}");
    assert_eq!(output["value"], json!({"numerator":"4","denominator":"1"}));
    let mut stricter = request.clone();
    replace_source(
        &mut stricter,
        &source("if lost then 1 else 3", "<= 3", true, "m.delivered"),
    );
    let (code, output) = run(&stricter);
    assert_eq!(code, 1, "{output}");
    assert_eq!(output["rule"], "threshold-missed");
    let mut irrelevant = request.clone();
    replace_source(
        &mut irrelevant,
        &source(
            "if lost then 1 else 3",
            "<= 4",
            true,
            "if m.attempts = 2 then (1 mod 0 = 0) else m.delivered",
        ),
    );
    let (code, output) = run(&irrelevant);
    assert_eq!(code, 0, "irrelevant post-B predicate: {output}");
}
#[trace("TC-1403", "FR-201-AC-36", "FR-201-AC-37")]
#[test]
fn closed_zero_cost_finite_forgery_rejects_at_actual_key() {
    let request = proof(&source("0", "<= 0", true, "m.delivered"));
    let initial = document(&request)["values"][0]["state"].clone();
    let mut forged = mutate(&request, |d| {
        d["objective"]["targets"] = json!([]);
        d["values"]
            .as_array_mut()
            .unwrap()
            .retain(|r| r["state"] == initial);
        d["ranking"]
            .as_array_mut()
            .unwrap()
            .retain(|r| r["state"] == initial);
        d["ranking"][0]["rank"] = json!(0);
    });
    replace_source(&mut forged, &source("0", "<= 0", false, "m.delivered"));
    let (code, output) = run(&forged);
    assert_eq!(code, 1, "{output}");
    assert_eq!(output["rule"], "rank-broken");
    assert_eq!(output["at"]["product_key"], initial);
}
#[trace("TC-1403", "FR-201-AC-37")]
#[test]
fn cyclic_reward_strict_rows_members_and_objective_cannot_alias() {
    let request = proof(&source("1", "<= 2", true, "m.delivered"));
    for shape in 0..9 {
        let bad = mutate(&request, |d| match shape {
            0 => {
                d.as_object_mut().unwrap().remove("ranking");
            }
            1 => {
                d["values"] = json!([]);
            }
            2 => {
                let row = d["values"][0].clone();
                d["values"].as_array_mut().unwrap().push(row);
            }
            3 => {
                d["values"].as_array_mut().unwrap().reverse();
            }
            4 => {
                d["ranking"][0]["rank"] = json!(0);
            }
            5 => {
                d["values"][0]["value"]["numerator"] = json!("00");
            }
            6 => {
                d["objective"]["reward"][1] = json!("foreign");
            }
            7 => {
                d["objective"]["unit"] = json!(["foreign"]);
            }
            _ => {
                let mut parts = key(&d["values"][0]["state"]);
                parts[1] = Value::Null;
                d["values"][0]["state"] =
                    json!(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&parts).unwrap()));
            }
        });
        let (code, output) = run(&bad);
        assert_ne!(code, 0, "shape {shape}: {output}");
        assert_ne!(output["status"], "accepted");
    }
    assert_eq!(run(&request).0, 0);
}
