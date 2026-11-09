// SPDX-License-Identifier: AGPL-3.0-or-later
//! Direct infinite-trace proof documents cross a fresh core-only process.
use super::bounded::{alter_keys, document, key, mutate, proof, request, run};
use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::ExactProbLimits;
use serde_json::{json, Value};

const SOURCE: &str = include_str!("../fixtures/link.native");
const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");

fn source(event: &str, scheduler: &str, operator: &str, threshold: &str) -> String {
    let model = SOURCE
        .replace("true: 1, false: 9", "true: 3, false: 1")
        .replace("true: 1, false: 4", "true: 1, false: 3");
    format!("{model}probabilistic Likely using inf on M over (m: M::Msg) under {scheduler} on origin {{ probability {operator} {threshold} [ {event} ] }}\n")
}

fn replace_source(request: &mut Value, source: &str) {
    request["source"]["content"] = json!(source);
    request["byte_provision"][0][2] = json!(source.as_bytes());
}

#[trace("TC-636", "FR-201-AC-15", "FR-201-AC-16", "FR-282-AC-14")]
#[test]
fn until_and_always_check_exact_direct_minimum_maximum_and_workload_values() {
    for event in [
        "holds(m.attempts = 0) until holds(m.delivered)",
        "always holds(m.attempts = 0 or m.delivered or m.attempts = 2)",
    ] {
        for (scheduler, operator, threshold, n, d) in [
            ("every scheduler", ">=", "0.25", "1", "4"),
            ("every scheduler", "<=", "0.75", "3", "4"),
            ("Even", ">=", "0.5", "1", "2"),
            ("Even", "<=", "0.5", "1", "2"),
        ] {
            let request = proof(&source(event, scheduler, operator, threshold), SNAPSHOT);
            let (code, output) = run(&request);
            assert_eq!(code, 0, "{event} {scheduler}: {output}");
            assert_eq!(output["value"], json!({"numerator": n,"denominator": d}));
            let doc = document(&request);
            assert_eq!(
                doc["objective"]["profile"],
                "quire.probability-objective/state-predicate/v1"
            );
            for row in doc["values"].as_array().unwrap() {
                let full = key(&row["state"]);
                assert!(!full[1].is_null(), "actual S3 binding is retained");
                assert_eq!(
                    &full.as_array().unwrap()[3..],
                    &[Value::Null, Value::Null, Value::Null]
                );
            }
        }
    }
}

#[trace("TC-636", "FR-201-AC-15", "FR-201-AC-16", "FR-282-AC-15")]
#[test]
fn until_precedence_and_infinite_terminal_stutter_stop_at_real_boundaries() {
    for (event, operator, threshold, expected, targets, failures, rows) in [
        (
            "holds(false) until holds(m.attempts = 0)",
            ">=",
            "1",
            "1",
            1,
            0,
            1,
        ),
        (
            "holds(false) until holds(m.delivered)",
            "<=",
            "0",
            "0",
            0,
            1,
            1,
        ),
        ("holds(true) until holds(false)", "<=", "0", "0", 0, 0, 5),
        ("always holds(true)", ">=", "1", "1", 3, 0, 5),
        ("always holds(m.attempts > 0)", "<=", "0", "0", 0, 1, 1),
        (
            "always holds(m.attempts = 0 or m.attempts = 2)",
            "<=",
            "0",
            "0",
            0,
            2,
            3,
        ),
    ] {
        let request = proof(
            &source(event, "every scheduler", operator, threshold),
            SNAPSHOT,
        );
        let (code, output) = run(&request);
        assert_eq!(code, 0, "{event}: {output}");
        assert_eq!(
            output["value"],
            json!({"numerator":expected,"denominator":"1"})
        );
        let doc = document(&request);
        assert_eq!(
            doc["objective"]["targets"].as_array().unwrap().len(),
            targets,
            "{event}"
        );
        assert_eq!(
            doc["objective"]["failures"].as_array().unwrap().len(),
            failures,
            "{event}"
        );
        assert_eq!(doc["values"].as_array().unwrap().len(), rows, "{event}");
    }
}

#[trace("TC-636", "FR-201-AC-17", "FR-282-AC-15")]
#[test]
fn altered_objective_binding_keys_values_ranks_and_threshold_do_not_prove() {
    let text = source(
        "holds(m.attempts = 0) until holds(m.delivered)",
        "every scheduler",
        ">=",
        "0.25",
    );
    let request = proof(&text, SNAPSHOT);
    for changed in [
        mutate(&request, |doc| doc["objective"]["event"] = json!("always")),
        mutate(&request, |doc| doc["objective"]["targets"] = json!([])),
        mutate(&request, |doc| {
            doc["objective"]["failures"] = doc["objective"]["targets"].clone()
        }),
        mutate(&request, |doc| {
            doc["objective"]["optimization"] = json!("maximum")
        }),
        mutate(&request, |doc| {
            doc["objective"]["profile"] = json!("quire.probability-objective/reachability/v1")
        }),
        mutate(&request, |doc| {
            doc["objective"]["fairness"] = json!(["foreign"])
        }),
        mutate(&request, |doc| alter_keys(doc, |key| key[1] = Value::Null)),
        mutate(&request, |doc| {
            alter_keys(doc, |key| key[2] = json!(["foreign"]))
        }),
        mutate(&request, |doc| {
            doc["values"][0]["value"] = json!({"numerator":"1","denominator":"1"})
        }),
        mutate(&request, |doc| {
            for row in doc["ranking"].as_array_mut().unwrap() {
                row["rank"] = json!(0);
            }
        }),
    ] {
        let (code, output) = run(&changed);
        assert_ne!(code, 0, "altered document accepted: {output}");
    }
    let mut stronger = request.clone();
    replace_source(&mut stronger, &text.replace("0.25", "0.333333333333333333"));
    assert_ne!(run(&stronger).0, 0);
    let mut other = request;
    replace_source(&mut other, &text.replace("m.attempts = 0", "false"));
    assert_ne!(run(&other).0, 0);
}

#[trace("TC-636", "FR-201-AC-17", "FR-282-AC-14")]
#[test]
fn failed_state_predicate_bounds_have_no_old_eventual_refutation() {
    for event in [
        "holds(m.attempts = 0) until holds(m.delivered)",
        "always holds(m.attempts = 0 or m.delivered or m.attempts = 2)",
    ] {
        let (document, settlement) = request(
            &source(event, "every scheduler", ">=", "1"),
            SNAPSHOT,
            ExactProbLimits::default(),
        );
        assert!(document.is_none());
        assert!(
            format!("{settlement:?}").contains("Unsupported"),
            "{settlement:?}"
        );
    }
}
