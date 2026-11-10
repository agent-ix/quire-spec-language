// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite MDP reward evidence checked in an analyzer-free process.
use super::bounded::{alter_keys, document, mutate, request_with, run};
use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::ExactProbLimits;
use serde_json::{json, Value};
const DOMAIN: &[u8] = include_bytes!("../fixtures/reward.semantic-ir.json");
const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");
fn source(wait: bool, a: &str, b: &str, bound: &str, target: &str) -> String {
    let apost = if wait {
        "self.attempts = 0 and not self.delivered"
    } else {
        "self.attempts - (if lost then 0 else 1) = 0 and self.delivered = not lost"
    };
    let bpost = if wait {
        "self.attempts = 1 and self.delivered"
    } else {
        "self.attempts - (if lost then 0 else 1) = 0 and self.delivered = not lost"
    };
    format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.model.complete/v1\";\nprofile inf = \"quire.temporal.infinite-trace/v1\";\nmodel M = \"test/link\";\npre A using v on M::Msg::send_a {{ self.attempts = 0 }}\npost AD using v on M::Msg::send_a {{ {apost} }}\npre B using v on M::Msg::send_b {{ self.attempts = 0 }}\npost BD using v on M::Msg::send_b {{ {bpost} }}\npre C using v on M::Msg::send_c {{ self.attempts > 0 }}\npost CD using v on M::Msg::send_c {{ self.attempts - (if pre(self.attempts) = 1 then 2 else 1) = 0 and self.delivered }}\nrandom Msg::send_a(lost) ~ {{ true: 1, false: 1 }};\nrandom Msg::send_b(lost) ~ {{ true: 1, false: 1 }};\nrandom Msg::send_c(lost) ~ {{ true: 1, false: 1 }};\nreward cost on Msg::send_a = {a};\nreward cost on Msg::send_b = {b};\nreward cost on Msg::send_c = 100;\nprobabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin {{ expected accumulate cost until holds({target}) {bound} }}\n")
}
fn proof(source: &str) -> Value {
    let (request, settlement) = request_with(source, SNAPSHOT, ExactProbLimits::default(), DOMAIN);
    request.unwrap_or_else(|| panic!("expected finite reward proof: {settlement:?}\n{source}"))
}
fn replace_source(request: &mut Value, source: &str) {
    request["source"]["content"] = json!(source);
    request["byte_provision"][0][2] = json!(source.as_bytes());
}
fn non_target(doc: &Value) -> Value {
    doc["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            !doc["objective"]["targets"]
                .as_array()
                .unwrap()
                .contains(&row["state"])
        })
        .unwrap()["state"]
        .clone()
}
#[trace("TC-1405", "FR-201-AC-43", "FR-201-AC-45")]
#[test]
fn same_distribution_distinct_costs_check_both_extrema_and_thresholds() {
    for (bound, value, miss) in [
        (">= 2", "2", ">= rational(5,2)"),
        ("<= 4", "4", "<= rational(7,2)"),
    ] {
        let src = source(false, "1", "2", bound, "m.delivered");
        let request = proof(&src);
        let doc = document(&request);
        assert_eq!(
            doc["objective"]["profile"],
            "quire.probability-objective/cyclic-mdp-expected-reward/v1"
        );
        assert_eq!(doc["values"].as_array().unwrap().len(), 2);
        let (code, out) = run(&request);
        assert_eq!(code, 0, "{out}");
        assert_eq!(out["value"], json!({"numerator":value,"denominator":"1"}));
        let mut stricter = request.clone();
        replace_source(&mut stricter, &source(false, "1", "2", miss, "m.delivered"));
        assert_eq!(run(&stricter).1["rule"], "threshold-missed");
        let mut changed = request.clone();
        replace_source(
            &mut changed,
            &source(
                false,
                "if lost then 1 else 3",
                "if lost then 2 else 4",
                bound,
                "m.delivered",
            ),
        );
        assert_eq!(run(&changed).1["rule"], "inequality-fails");
        let mut post_b = request.clone();
        replace_source(
            &mut post_b,
            &source(
                false,
                "1",
                "2",
                bound,
                "if m.attempts = 2 then (1 mod 0 = 0) else m.delivered",
            ),
        );
        assert_eq!(run(&post_b).0, 0);
    }
}
#[trace("TC-1405", "FR-201-AC-44", "FR-201-AC-45")]
#[test]
fn zero_wait_cannot_justify_tight_minimum_or_universal_maximum_progress() {
    let request = proof(&source(true, "0", "1", ">= 1", "m.delivered"));
    assert_eq!(run(&request).0, 0);
    let initial = non_target(&document(&request));
    for (n, d) in [("0", "1"), ("1", "2")] {
        let false_min = mutate(&request, |doc| {
            for row in doc["values"].as_array_mut().unwrap() {
                if row["state"] == initial {
                    row["value"] = json!({"numerator":n,"denominator":d});
                }
            }
        });
        let (code, out) = run(&false_min);
        assert_eq!(code, 1, "{out}");
        assert_eq!(out["rule"], "rank-broken");
        assert_eq!(out["at"]["product_key"], initial);
    }
    let mut false_max = mutate(&request, |doc| {
        doc["objective"]["optimization"] = json!("maximum")
    });
    replace_source(
        &mut false_max,
        &source(true, "0", "1", "<= 1", "m.delivered"),
    );
    assert_eq!(run(&false_max).1["rule"], "rank-broken");
    let (native, settlement) = request_with(
        &source(true, "0", "1", "<= 1", "m.delivered"),
        SNAPSHOT,
        ExactProbLimits::default(),
        DOMAIN,
    );
    assert!(native.is_none(), "{settlement:?}");
}
#[trace("TC-1405", "FR-201-AC-44", "FR-201-AC-45")]
#[test]
fn proper_zero_reward_keeps_positive_rank_and_strict_rows_are_closed() {
    let request = proof(&source(false, "0", "0", "<= 0", "m.delivered"));
    assert_eq!(run(&request).0, 0);
    let initial = non_target(&document(&request));
    let zero_rank = mutate(&request, |doc| {
        for row in doc["ranking"].as_array_mut().unwrap() {
            if row["state"] == initial {
                row["rank"] = json!(0);
            }
        }
    });
    assert_eq!(run(&zero_rank).1["rule"], "rank-broken");
    for bad in [
        mutate(&request, |d| {
            d.as_object_mut().unwrap().remove("ranking");
        }),
        mutate(&request, |d| d["ranking"][0]["extra"] = json!(0)),
        mutate(&request, |d| d["values"][0]["value"]["extra"] = json!(0)),
        mutate(&request, |d| {
            d["values"].as_array_mut().unwrap().pop();
        }),
        mutate(&request, |d| {
            let row = d["ranking"][0].clone();
            d["ranking"].as_array_mut().unwrap().push(row);
        }),
        mutate(&request, |d| d["objective"]["reward"][1] = json!("foreign")),
        mutate(&request, |d| d["objective"]["unit"] = json!(["foreign"])),
        mutate(&request, |d| alter_keys(d, |key| key[1] = Value::Null)),
    ] {
        assert_ne!(run(&bad).0, 0);
    }
    assert_eq!(run(&request).0, 0);
}

#[trace("TC-1405", "FR-201-AC-44", "FR-201-AC-45")]
#[test]
fn mixed_infinite_minimum_and_false_terminal_forgery_never_become_finite_proofs() {
    let base = source(true, "1", "2", ">= 1", "m.attempts = 1")
        .replace(
            "post AD using v on M::Msg::send_a { self.attempts = 0 and not self.delivered }",
            "post AD using v on M::Msg::send_a { self.attempts = 1 and self.delivered }",
        )
        .replace(
            "post BD using v on M::Msg::send_b { self.attempts = 1 and self.delivered }",
            "post BD using v on M::Msg::send_b { self.attempts = 2 and not self.delivered }",
        )
        .replace(
            "self.attempts - (if pre(self.attempts) = 1 then 2 else 1) = 0 and self.delivered",
            "self.attempts = 1 and self.delivered",
        );
    let request = proof(&base);
    assert_eq!(document(&request)["values"].as_array().unwrap().len(), 3);
    let trap = document(&request)["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["value"]["numerator"] == "100")
        .unwrap()["state"]
        .clone();
    for terminal in [false, true] {
        let mut src = base.replace("post CD using v on M::Msg::send_c { self.attempts = 1 and self.delivered }", "post CD using v on M::Msg::send_c { self.attempts - pre(self.attempts) = 0 and self.delivered = pre(self.delivered) }")
            .replace("reward cost on Msg::send_c = 100", "reward cost on Msg::send_c = 0");
        if terminal {
            src = src.replace(
                "pre C using v on M::Msg::send_c { self.attempts > 0 }",
                "pre C using v on M::Msg::send_c { self.attempts = 1 }",
            );
            src.push_str("terminal Stop using v on M when { self.attempts = 2 };\n");
        }
        let (native, settlement) = request_with(&src, SNAPSHOT, ExactProbLimits::default(), DOMAIN);
        assert!(native.is_none(), "{settlement:?}");
        let qsl_analyze::analyze::Settlement::Disposed(disposal) = settlement else {
            panic!("{settlement:?}");
        };
        assert_eq!(
            disposal.kind,
            qsl_analyze::analyze::DispositionKind::Unsupported
        );
        assert_eq!(
            disposal.diagnostic.unwrap().code.cause(),
            "mixed-infinite-coverage"
        );
        let mut forged = mutate(&request, |d| {
            for row in d["values"].as_array_mut().unwrap() {
                if row["state"] == trap {
                    row["value"] = json!({"numerator":"0","denominator":"1"});
                }
            }
        });
        replace_source(&mut forged, &src);
        let (code, out) = run(&forged);
        assert_eq!(code, 1, "{out}");
        assert_eq!(out["rule"], "rank-broken");
        assert_eq!(out["at"]["product_key"], trap);
        let missing = mutate(&forged, |d| {
            d["values"]
                .as_array_mut()
                .unwrap()
                .retain(|r| r["state"] != trap);
        });
        assert_eq!(run(&missing).1["rule"], "malformed");
    }
}
#[trace("TC-1405", "FR-201-AC-43", "FR-201-AC-45")]
#[test]
fn checked_comparison_unit_converts_exactly_and_rejects_foreign_unit_identity() {
    let src = source(false, "1 ms", "2 ms", "<= 0.004 s", "m.delivered")
         .replace("reward cost on Msg::send_c = 100;", "reward cost on Msg::send_c = 100 ms;")
        + "\ndimension Time;\nunit ms : Time = rational(1,1);\nunit s : Time = rational(1000,1) * ms;\nunit other : Time = rational(1,1) * ms;\n";
    let request = proof(&src);
    assert_eq!(run(&request).0, 0);
    let d = document(&request);
    assert_eq!(d["objective"]["unit"][0], "declared");
    let bad = mutate(&request, |d| d["objective"]["unit"] = Value::Null);
    let (_, out) = run(&bad);
    assert_eq!(out["status"], "refused");
    let foreign_source = source(false, "1 other", "2 other", "<= 4 other", "m.delivered")
        .replace("reward cost on Msg::send_c = 100;", "reward cost on Msg::send_c = 100 other;")
        + "\ndimension Time;\nunit ms : Time = rational(1,1);\nunit s : Time = rational(1000,1) * ms;\nunit other : Time = rational(1,1) * ms;\n";
    let foreign = proof(&foreign_source);
    let foreign_unit = document(&foreign)["objective"]["unit"].clone();
    assert_ne!(foreign_unit, d["objective"]["unit"]);
    let changed = mutate(&request, |doc| {
        doc["objective"]["unit"] = foreign_unit.clone()
    });
    assert_eq!(
        run(&changed).1["reason"],
        "invalid_runtime_input: probability certificate objective mismatch"
    );
}
