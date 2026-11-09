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
    let threshold = match threshold {
        "1" => "0.999",
        "0" => "0.001",
        other => other,
    };
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
            let text = source(event, scheduler, operator, threshold);
            let request = proof(&text, SNAPSHOT);
            let (code, output) = run(&request);
            assert_eq!(code, 0, "{event} {scheduler}: {output}");
            assert_eq!(output["value"], json!({"numerator": n,"denominator": d}));
            let mut stronger = request.clone();
            let beyond = match (scheduler, operator) {
                ("every scheduler", ">=") => "rational(1,3)",
                ("every scheduler", "<=") => "rational(2,3)",
                (_, ">=") => "rational(3,5)",
                _ => "rational(2,5)",
            };
            replace_source(&mut stronger, &text.replace(threshold, beyond));
            let (code, rejected) = run(&stronger);
            assert_eq!(code, 1, "{rejected}");
            assert_eq!(rejected["rule"], "threshold-missed");
            let doc = document(&request);
            assert_eq!(doc["values"].as_array().unwrap().len(), 3);
            assert_eq!(doc["objective"]["targets"].as_array().unwrap().len(), 1);
            assert_eq!(doc["objective"]["failures"].as_array().unwrap().len(), 1);
            let bytes = request["certificate"].as_str().unwrap().as_bytes();
            let limits = qsl_replay::ProbabilityDocumentLimits {
                bytes: 1_000_000,
                storage_bytes: 8_000_000,
                states: 100,
                edges: 1000,
                scalar_bits: 128,
                accounting: qsl_replay::AccountingLimits::default().0,
            };
            let cancel = quire_exact::Cancel::new();
            let decoded = qsl_replay::read_probability_certificate(bytes, limits, &cancel).unwrap();
            assert_eq!(
                qsl_replay::write_probability_certificate(&decoded, limits, &cancel).unwrap(),
                bytes
            );
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
        mutate(&request, |doc| doc["objective"]["failures"] = json!([])),
        mutate(&request, |doc| {
            doc["values"].as_array_mut().unwrap().pop();
        }),
        mutate(&request, |doc| {
            alter_keys(doc, |key| key[3] = json!(["foreign-monitor"]))
        }),
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

#[trace("TC-636", "FR-201-AC-18", "FR-282-AC-15")]
#[test]
fn original_cycles_remain_unsupported_even_behind_initial_success_or_failure() {
    for event in [
        "holds(false) until holds(m.attempts = 0)",
        "always holds(false)",
    ] {
        let text = source(
            event,
            "every scheduler",
            if event.starts_with("always") {
                "<="
            } else {
                ">="
            },
            if event.starts_with("always") {
                "0"
            } else {
                "1"
            },
        );
        let mut request = proof(&text, SNAPSHOT);
        let cyclic = text
            .replace("self.attempts < 2", "true")
            .replace(
                "self.attempts - pre(self.attempts) = 1",
                "self.attempts = pre(self.attempts)",
            )
            .replace(
                "terminal Stop using v on M when { self.delivered or self.attempts = 2 };",
                "",
            );
        replace_source(&mut request, &cyclic);
        let (code, output) = run(&request);
        assert_ne!(code, 0);
        assert_eq!(output["status"], "unsupported", "{output}");
    }
}

#[trace("TC-636", "FR-201-AC-18", "FR-282-AC-15")]
#[test]
fn caller_limits_and_cancellation_never_return_truth_or_evidence() {
    let original = proof(
        &source(
            "holds(false) until holds(m.attempts = 0)",
            "every scheduler",
            ">=",
            "1",
        ),
        SNAPSHOT,
    );
    for name in [
        "bytes",
        "storage_bytes",
        "states",
        "edges",
        "scalar_bits",
        "expansion",
    ] {
        let mut limited = original.clone();
        limited["limits"][name] = json!(0);
        let (code, output) = run(&limited);
        assert_ne!(code, 0, "{name}: {output}");
        assert!(output.get("value").is_none(), "{name}: {output}");
        assert!(output.get("certificate").is_none());
    }
    let mut original_graph = original.clone();
    original_graph["limits"]["states"] = json!(1);
    assert_ne!(
        run(&original_graph).0,
        0,
        "initial absorption cannot bypass original graph limits"
    );
    let mut arithmetic = original.clone();
    arithmetic["limits"]["stages"]["accounting.work_units"] = json!(0);
    assert_ne!(run(&arithmetic).0, 0);
    let (code, output) = super::bounded::run_with(&original, &["--cancel"]);
    assert_ne!(code, 0);
    assert!(output.get("value").is_none());
}

#[trace("TC-636", "FR-201-AC-15", "FR-201-AC-16")]
#[test]
fn initial_terminal_predicates_use_infinite_stutter() {
    let mut snapshot: Value = serde_json::from_slice(SNAPSHOT).unwrap();
    snapshot["populations"][0]["objects"][0]["fields"]["attempts"]["integer"] = json!("2");
    let snapshot = serde_json::to_vec(&snapshot).unwrap();
    for (event, operator, threshold, expected) in [
        ("always holds(not m.delivered)", ">=", "1", "1"),
        (
            "holds(not m.delivered) until holds(m.delivered)",
            "<=",
            "0",
            "0",
        ),
    ] {
        let request = proof(
            &source(event, "every scheduler", operator, threshold),
            &snapshot,
        );
        let (code, output) = run(&request);
        assert_eq!(code, 0, "{output}");
        assert_eq!(
            output["value"],
            json!({"numerator":expected,"denominator":"1"})
        );
        assert_eq!(document(&request)["values"].as_array().unwrap().len(), 1);
    }
}
