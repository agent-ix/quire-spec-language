// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cyclic MDP certificates checked in a fresh analyzer-free process.
use super::bounded::{alter_keys, document, mutate, request_with, run};
use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::ExactProbLimits;
use serde_json::{json, Value};

const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");
fn domain() -> String {
    include_str!("../fixtures/reward.semantic-ir.json").replace(
        "\"typeRef\":\"ix://quire/native/Boolean\"}],\"post\"",
        "\"typeRef\":\"ix://test/link/Attempts\"}],\"post\"",
    )
}
fn source(a: &str, b: &str, op: &str, threshold: &str, target: &str) -> String {
    let post = "self.attempts - (if lost = 0 then 0 else if lost = 1 then 1 else 2) = 0 and self.delivered = (lost = 1)";
    let apost = if a == "wait" {
        "self.attempts = 0 and not self.delivered"
    } else {
        post
    };
    let bpost = if b == "attempt" {
        "self.attempts - (if lost = 0 then 1 else 2) = 0 and self.delivered = (lost = 0)"
    } else {
        post
    };
    let a = if a == "wait" { "0: 1, 1: 1, 2: 1" } else { a };
    let b = if b == "attempt" {
        "0: 8, 1: 1, 2: 1"
    } else {
        b
    };
    format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.model.complete/v1\";\nprofile inf = \"quire.temporal.infinite-trace/v1\";\nmodel M = \"test/link\";\npre A using v on M::Msg::send_a {{ self.attempts = 0 }}\npost AD using v on M::Msg::send_a {{ {apost} }}\npre B using v on M::Msg::send_b {{ self.attempts = 0 }}\npost BD using v on M::Msg::send_b {{ {bpost} }}\npre C using v on M::Msg::send_c {{ false }}\npost CD using v on M::Msg::send_c {{ self.attempts = 0 and not self.delivered }}\nrandom Msg::send_a(lost) ~ {{ {a} }};\nrandom Msg::send_b(lost) ~ {{ {b} }};\nrandom Msg::send_c(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\nterminal Stop using v on M when {{ self.attempts = 2 and not self.delivered }};\nprobabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin {{ probability {op} {threshold} [ eventually holds({target}) ] }}\n")
}
fn proof(source: &str) -> Value {
    let (request, settlement) = request_with(
        source,
        SNAPSHOT,
        ExactProbLimits::default(),
        domain().as_bytes(),
    );
    request.unwrap_or_else(|| panic!("expected MDP proof: {settlement:?}\n{source}"))
}
fn replace_source(request: &mut Value, source: &str) {
    request["source"]["content"] = json!(source);
    request["byte_provision"][0][2] = json!(source.as_bytes());
}
fn initial(row: &Value) -> bool {
    row["value"]["denominator"] != "1"
}
#[trace("TC-1404", "FR-201-AC-39", "FR-201-AC-40", "FR-201-AC-41")]
#[test]
fn separate_retry_actions_roundtrip_both_extrema_and_interior_threshold_misses() {
    for (op, n, d, threshold, miss) in [
        (">=", "2", "5", "rational(2,5)", "rational(1,2)"),
        ("<=", "4", "5", "rational(4,5)", "rational(3,4)"),
    ] {
        let src = source(
            "0: 5, 1: 2, 2: 3",
            "0: 5, 1: 4, 2: 1",
            op,
            threshold,
            "m.delivered",
        );
        let request = proof(&src);
        let doc = document(&request);
        assert_eq!(
            doc["objective"]["profile"],
            "quire.probability-objective/cyclic-mdp-eventual/v1"
        );
        let (code, out) = run(&request);
        assert_eq!(code, 0, "{out}");
        assert_eq!(out["value"], json!({"numerator":n,"denominator":d}));
        let mut changed = request.clone();
        replace_source(
            &mut changed,
            &source(
                "0: 5, 1: 2, 2: 3",
                "0: 5, 1: 4, 2: 1",
                op,
                miss,
                "m.delivered",
            ),
        );
        let (code, out) = run(&changed);
        assert_eq!(code, 1, "{out}");
        assert_eq!(out["rule"], "threshold-missed");
        let opposite = mutate(&request, |d| {
            d["objective"]["optimization"] = json!(if op == ">=" { "maximum" } else { "minimum" })
        });
        assert_ne!(run(&opposite).0, 0);
        let bad = mutate(&request, |d| {
            let row = d["values"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| initial(r))
                .unwrap();
            row["value"] = json!({"numerator":"1","denominator":"2"});
        });
        assert_eq!(run(&bad).1["rule"], "inequality-fails");
    }
}
#[trace("TC-1404", "FR-201-AC-40", "FR-201-AC-41")]
#[test]
fn bellman_fixed_points_require_universal_min_and_tight_max_progress() {
    // A is pure wait. For MIN, all positive fixed points falsely pass Bellman.
    // For MAX, .9 has only a nonprogressing tight action: wait.
    for (op, b, truth, wrong) in [
        (">=", "attempt", "0", ("1", "2")),
        ("<=", "attempt", "4", ("9", "10")),
    ] {
        // Start from the valid MAX .8 evidence over the same source graph.
        let base = proof(&source("wait", b, "<=", "rational(9,10)", "m.delivered"));
        let initial_key = document(&base)["values"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| initial(r))
            .map(|r| r["state"].clone());
        let initial_key = initial_key.unwrap_or_else(|| {
            let doc = document(&base);
            doc["ranking"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["rank"] != 0)
                .unwrap()["state"]
                .clone()
        });
        let mut forged = mutate(&base, |d| {
            d["objective"]["optimization"] = json!(if op == ">=" { "minimum" } else { "maximum" });
            for r in d["values"].as_array_mut().unwrap() {
                if r["state"] == initial_key {
                    r["value"] = json!({"numerator":wrong.0,"denominator":wrong.1});
                }
            }
        });
        replace_source(
            &mut forged,
            &source(
                "wait",
                b,
                op,
                if op == ">=" {
                    "rational(1,4)"
                } else {
                    "rational(19,20)"
                },
                "m.delivered",
            ),
        );
        let (code, out) = run(&forged);
        assert_eq!(code, 1, "{truth}: {out}");
        assert_eq!(out["rule"], "rank-broken", "{out}");
    }
}

#[trace("TC-1404", "FR-201-AC-39", "FR-201-AC-41")]
#[test]
fn strict_rows_boundaries_and_quantitative_source_scope_are_closed() {
    let src = source(
        "0: 5, 1: 2, 2: 3",
        "0: 5, 1: 4, 2: 1",
        ">=",
        "rational(1,3)",
        "m.delivered",
    );
    let request = proof(&src);
    for malformed in [
        mutate(&request, |d| {
            d.as_object_mut().unwrap().remove("ranking");
        }),
        mutate(&request, |d| {
            d["ranking"][0]["extra"] = json!(0);
        }),
        mutate(&request, |d| {
            d["values"][0]["value"]["extra"] = json!(0);
        }),
        mutate(&request, |d| {
            let row = d["ranking"][0].clone();
            d["ranking"].as_array_mut().unwrap().push(row);
        }),
        mutate(&request, |d| {
            d["values"][0]["value"] = json!({"numerator":"-1","denominator":"1"});
        }),
    ] {
        assert_ne!(run(&malformed).0, 0);
    }
    let foreign_binding = mutate(&request, |d| alter_keys(d, |full| full[1] = Value::Null));
    assert_ne!(run(&foreign_binding).0, 0);
    let boundary = mutate(&request, |d| {
        let target = d["objective"]["targets"][0].clone();
        for row in d["ranking"].as_array_mut().unwrap() {
            if row["state"] == target {
                row["rank"] = json!(1);
            }
        }
    });
    assert_eq!(run(&boundary).1["rule"], "rank-broken");
    for threshold in ["0", "1"] {
        let mut outside = request.clone();
        replace_source(
            &mut outside,
            &source(
                "0: 5, 1: 2, 2: 3",
                "0: 5, 1: 4, 2: 1",
                ">=",
                threshold,
                "m.delivered",
            ),
        );
        assert_ne!(run(&outside).0, 0);
    }
}

#[trace("TC-1404", "FR-201-AC-39", "FR-201-AC-40", "FR-201-AC-41")]
#[test]
fn cyclic_pair_accepts_equal_positive_ranks_with_only_zero_lower_successor() {
    for (op, threshold, s, t) in [
        (">=", "rational(2,5)", ("2", "5"), ("4", "5")),
        ("<=", "rational(2,3)", ("2", "3"), ("8", "9")),
    ] {
        let src = source("0: 2, 1: 1, 2: 1","0: 6, 1: 1, 2: 1",op,threshold,"m.delivered")
            .replace("self.attempts - (if lost = 0 then 0 else if lost = 1 then 1 else 2) = 0 and self.delivered = (lost = 1)",
                "self.attempts - (if lost = 0 then 1 else 2) = 0 and not self.delivered")
            .replace("pre C using v on M::Msg::send_c { false }","pre C using v on M::Msg::send_c { self.attempts = 1 }")
            .replace("post CD using v on M::Msg::send_c { self.attempts = 0 and not self.delivered }",
                "post CD using v on M::Msg::send_c { self.attempts = 0 and self.delivered = (lost = 0) }")
            .replace("random Msg::send_c(lost) ~ { 0: 1, 1: 1, 2: 1 }","random Msg::send_c(lost) ~ { 0: 4, 1: 1, 2: 1 }");
        let request = proof(&src);
        let doc = document(&request);
        let offered: Vec<_> = doc["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["value"].clone())
            .collect();
        assert!(offered.contains(&json!({"numerator":s.0,"denominator":s.1})));
        assert!(offered.contains(&json!({"numerator":t.0,"denominator":t.1})));
        let offered = mutate(&request, |d| {
            let positive: Vec<_> = d["values"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["value"]["denominator"] != "1")
                .map(|r| r["state"].clone())
                .collect();
            for row in d["ranking"].as_array_mut().unwrap() {
                if positive.contains(&row["state"]) {
                    row["rank"] = json!(1);
                }
            }
        });
        let (code, out) = run(&offered);
        assert_eq!(code, 0, "{out}");
    }
}
