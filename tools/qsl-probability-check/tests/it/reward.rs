// SPDX-License-Identifier: AGPL-3.0-or-later
//! Named-reward documents are recomputed by a fresh qualified-core process.

use super::bounded::{alter_keys, document, key, mutate, request_with, run, run_with};
use ix_trace_rs::trace;
use qsl_analyze::exact_probabilistic::ExactProbLimits;
use serde_json::{json, Value};

const DOMAIN: &[u8] = include_bytes!("../fixtures/reward.semantic-ir.json");
const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");
fn header() -> String {
    // Retain the production fixture's admitted language identity.
    let language = include_str!("../fixtures/link.native")
        .lines()
        .next()
        .unwrap();
    format!("{language}\nprofile v = \"quire.model.complete/v1\";\nprofile inf = \"quire.temporal.infinite-trace/v1\";\nmodel M = \"test/link\";\n")
}

fn claim(model: &str, scheduler: &str, op: &str, threshold: &str, target: &str) -> String {
    format!("{model}\nprobabilistic Likely using inf on M over (m: M::Msg) under {scheduler} on origin {{ expected accumulate cost until holds({target}) {op} {threshold} }}\n")
}

fn two_step() -> String {
    format!("{}\n\
        pre A using v on M::Msg::send_a {{ self.attempts = 0 }}\n\
        post A_done using v on M::Msg::send_a {{ self.attempts = 1 and not self.delivered }}\n\
        pre B using v on M::Msg::send_b {{ self.attempts = 0 }}\n\
        post B_done using v on M::Msg::send_b {{ self.attempts = 2 and self.delivered }}\n\
        pre C using v on M::Msg::send_c {{ self.attempts = 1 }}\n\
        post C_done using v on M::Msg::send_c {{ self.attempts = 2 and self.delivered }}\n\
        random Msg::send_a(lost) ~ {{ true: 1, false: 1 }};\n\
        random Msg::send_b(lost) ~ {{ true: 1, false: 1 }};\n\
        random Msg::send_c(lost) ~ {{ true: 1, false: 1 }};\n\
        reward cost on Msg::send_a = 1;\n\
        reward cost on Msg::send_b = 4;\n\
        reward cost on Msg::send_c = 2;\n\
        terminal Stop using v on M when {{ self.attempts = 2 }};\n\
        workload Even on M {{ weight Msg::send_a = 1; weight Msg::send_b = 1; weight Msg::send_c = 1; }}\n", header())
}

fn draws(cost: &str) -> String {
    format!("{}\n\
        pre A using v on M::Msg::send_a {{ self.attempts = 0 }}\n\
        post A_done using v on M::Msg::send_a {{ self.attempts = 1 and self.delivered = (not lost) }}\n\
        pre B using v on M::Msg::send_b {{ false }}\n\
        post B_done using v on M::Msg::send_b {{ self.attempts = 2 and self.delivered }}\n\
        pre C using v on M::Msg::send_c {{ false }}\n\
        post C_done using v on M::Msg::send_c {{ self.attempts = 2 and self.delivered }}\n\
        random Msg::send_a(lost) ~ {{ true: 1, false: 1 }};\n\
        random Msg::send_b(lost) ~ {{ true: 1, false: 1 }};\n\
        random Msg::send_c(lost) ~ {{ true: 1, false: 1 }};\n\
        reward cost on Msg::send_a = {cost};\n\
        terminal Stop using v on M when {{ self.attempts = 1 }};\n\
        workload Even on M {{ weight Msg::send_a = 1; weight Msg::send_b = 1; weight Msg::send_c = 1; }}\n", header())
}

fn proof(source: &str) -> Value {
    let (request, settlement) = request_with(source, SNAPSHOT, ExactProbLimits::default(), DOMAIN);
    request.unwrap_or_else(|| {
        panic!("reward proof must retain its document: {settlement:?}\n{source}")
    })
}

fn replace_source(request: &mut Value, source: &str) {
    request["source"]["content"] = json!(source);
    request["byte_provision"][0][2] = json!(source.as_bytes());
}

fn accepted(request: &Value, n: &str, d: &str) {
    let (code, output) = run(request);
    assert_eq!(code, 0, "{output}");
    assert_eq!(output["value"], json!({"numerator": n, "denominator": d}));
}

#[trace("TC-636", "FR-201-AC-19", "FR-282-AC-16")]
#[test]
fn two_step_named_reward_checks_minimum_maximum_and_both_workload_bounds() {
    for (scheduler, op, threshold, n, d, beyond) in [
        ("every scheduler", ">=", "3", "3", "1", "4"),
        ("every scheduler", "<=", "4", "4", "1", "3"),
        ("Even", ">=", "rational(7,2)", "7", "2", "4"),
        ("Even", "<=", "rational(7,2)", "7", "2", "3"),
    ] {
        let source = claim(&two_step(), scheduler, op, threshold, "m.delivered");
        let request = proof(&source);
        accepted(&request, n, d);
        let doc = document(&request);
        assert_eq!(
            doc["objective"]["profile"],
            "quire.probability-objective/expected-reward/v1"
        );
        assert_eq!(doc["objective"]["reward"][1], "cost");
        assert!(doc["objective"]["unit"].is_null());
        for row in doc["values"].as_array().unwrap() {
            let full = key(&row["state"]);
            assert!(!full[1].is_null());
            assert_eq!(
                &full.as_array().unwrap()[3..],
                &[Value::Null, Value::Null, Value::Null]
            );
        }
        let mut stronger = request.clone();
        replace_source(
            &mut stronger,
            &claim(&two_step(), scheduler, op, beyond, "m.delivered"),
        );
        let (code, output) = run(&stronger);
        assert_eq!(code, 1, "{output}");
        assert_eq!(output["rule"], "threshold-missed");
        let target = doc["objective"]["targets"][0].clone();
        let target_one = mutate(&request, |doc| {
            let row = doc["values"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|row| row["state"] == target)
                .unwrap();
            row["value"]["numerator"] = json!("1");
        });
        let (code, output) = run(&target_one);
        assert_eq!(code, 1, "{output}");
        assert_eq!(output["rule"], "inequality-fails");
    }
}

#[trace("TC-636", "FR-201-AC-20", "FR-282-AC-16")]
#[test]
fn independent_draw_costs_accept_value_preserving_swap_and_reject_changed_equation() {
    let source = claim(
        &draws("if lost then 2 else 6"),
        "every scheduler",
        ">=",
        "4",
        "m.attempts = 1",
    );
    let request = proof(&source);
    accepted(&request, "4", "1");
    for (cost, code) in [
        ("if lost then 6 else 2", 0),
        ("if lost then 2 else 8", 1),
        ("1", 1),
    ] {
        let mut changed = request.clone();
        replace_source(
            &mut changed,
            &claim(&draws(cost), "every scheduler", ">=", "4", "m.attempts = 1"),
        );
        let (actual, output) = run(&changed);
        assert_eq!(actual, code, "{cost}: {output}");
        if code == 1 {
            assert_eq!(output["rule"], "inequality-fails");
        }
    }
    let initial = proof(&claim(
        &draws("99"),
        "every scheduler",
        ">=",
        "0",
        "m.attempts >= 0",
    ));
    accepted(&initial, "0", "1");
    assert_eq!(document(&initial)["values"].as_array().unwrap().len(), 1);
    let stopped = proof(&claim(
        &two_step(),
        "every scheduler",
        ">=",
        "1",
        "m.attempts >= 1",
    ));
    accepted(&stopped, "1", "1");
}

#[trace("TC-636", "FR-201-AC-21", "FR-282-AC-16")]
#[test]
fn quantity_reward_uses_checked_exact_conversion_and_retains_unit_identity() {
    let model = format!("{}\ndimension Time;\nunit ms : Time = rational(1,1);\nunit s : Time = rational(1000,1) * ms;\n", draws("if lost then 2 ms else 6 ms"));
    let request = proof(&claim(
        &model,
        "every scheduler",
        ">=",
        "rational(4,1000) s",
        "m.attempts = 1",
    ));
    accepted(&request, "4", "1");
    let doc = document(&request);
    assert_eq!(doc["objective"]["unit"][0], "declared");
    let changed = mutate(&request, |doc| doc["objective"]["unit"] = Value::Null);
    let (code, output) = run(&changed);
    assert_eq!(code, 2, "{output}");
    assert_eq!(
        output["reason"],
        "invalid_runtime_input: probability certificate objective mismatch"
    );
    let wrong_dimension =
        format!("{model}\ndimension Length;\nunit metre : Length = rational(1,1);\n");
    let mut wrong_bound = request;
    replace_source(
        &mut wrong_bound,
        &claim(
            &wrong_dimension,
            "every scheduler",
            ">=",
            "4 metre",
            "m.attempts = 1",
        ),
    );
    let (code, output) = run(&wrong_bound);
    assert_eq!(code, 2, "{output}");
    assert_eq!(output["status"], "refused");
}

#[trace("TC-636", "FR-201-AC-21", "FR-201-AC-22", "FR-282-AC-17")]
#[test]
fn reward_objective_keys_coverage_heights_and_caller_stops_are_checked() {
    let request = proof(&claim(
        &draws("if lost then 2 else 6"),
        "every scheduler",
        ">=",
        "4",
        "m.attempts = 1",
    ));
    for changed in [
        mutate(&request, |doc| {
            doc["objective"]["reward"][1] = json!("other")
        }),
        mutate(&request, |doc| {
            doc["objective"]["reward"][0] = doc["identity"]["node_id"].clone()
        }),
        mutate(&request, |doc| doc["objective"]["targets"] = json!([])),
        mutate(&request, |doc| {
            doc["objective"]["optimization"] = json!("maximum")
        }),
    ] {
        let (code, output) = run(&changed);
        assert_eq!(code, 2, "{output}");
        assert!(
            output["reason"]
                .as_str()
                .unwrap()
                .contains("objective mismatch"),
            "{output}"
        );
    }
    let altered = mutate(&request, |doc| {
        alter_keys(doc, |state| state[1] = Value::Null)
    });
    assert_ne!(run(&altered).0, 0);
    let omitted = mutate(&request, |doc| {
        doc["values"].as_array_mut().unwrap().pop();
    });
    assert_ne!(run(&omitted).0, 0);
    let rank = mutate(&request, |doc| {
        for row in doc["ranking"].as_array_mut().unwrap() {
            row["rank"] = json!(9);
        }
    });
    let (code, output) = run(&rank);
    assert_eq!(code, 1, "{output}");
    assert_eq!(output["rule"], "rank-broken");
    let missing_rank = mutate(&request, |doc| {
        let rows = doc["ranking"].as_array_mut().unwrap();
        let running = rows.iter().position(|row| row["rank"] == json!(1)).unwrap();
        rows.remove(running);
    });
    let (code, output) = run(&missing_rank);
    assert_eq!(code, 1, "{output}");
    assert_eq!(output["rule"], "rank-missing");
    let zero = proof(&claim(
        &draws("0"),
        "every scheduler",
        ">=",
        "0",
        "m.attempts = 1",
    ));
    accepted(&zero, "0", "1");
    let wrong_zero_height = mutate(&zero, |doc| {
        for row in doc["ranking"].as_array_mut().unwrap() {
            row["rank"] = json!(0);
        }
    });
    let (code, output) = run(&wrong_zero_height);
    assert_eq!(code, 1, "{output}");
    assert_eq!(output["rule"], "rank-broken");
    for setting in ["bytes", "storage_bytes", "states", "edges", "scalar_bits"] {
        let mut limited = request.clone();
        limited["limits"][setting] = json!(0);
        let (code, output) = run(&limited);
        assert_eq!(code, 2, "{setting}: {output}");
        assert_eq!(output["status"], "stopped");
    }
    let (code, output) = run_with(&request, &["--cancel"]);
    assert_eq!(code, 2, "{output}");
    assert_eq!(output["status"], "stopped");
}

#[trace("TC-636", "FR-201-AC-20", "FR-201-AC-22", "FR-282-AC-17")]
#[test]
fn nonreaching_terminals_hidden_raw_cycles_and_negative_rewards_never_accept() {
    let model = draws("if lost then 2 else 6");
    let request = proof(&claim(
        &model,
        "every scheduler",
        ">=",
        "4",
        "m.attempts = 1",
    ));
    let mut nonreaching = request.clone();
    replace_source(
        &mut nonreaching,
        &claim(&model, "every scheduler", ">=", "4", "m.delivered"),
    );
    let (code, output) = run(&nonreaching);
    assert_eq!(code, 2, "{output}");
    assert_eq!(output["status"], "unsupported");
    let cycle = model
        .replace("pre B using v on M::Msg::send_b { false }", "pre B using v on M::Msg::send_b { self.attempts = 1 }")
        .replace("post B_done using v on M::Msg::send_b { self.attempts = 2 and self.delivered }", "post B_done using v on M::Msg::send_b { self.attempts = 1 and self.delivered = pre(self.delivered) }")
        .replace("terminal Stop using v on M when { self.attempts = 1 }", "terminal Stop using v on M when { self.attempts = 2 }");
    let mut cyclic = request.clone();
    replace_source(
        &mut cyclic,
        &claim(&cycle, "every scheduler", ">=", "4", "m.attempts = 1"),
    );
    let (code, output) = run(&cyclic);
    assert_eq!(code, 2, "{output}");
    assert_eq!(output["status"], "unsupported");
    for cost in ["-1", "1 div 0"] {
        let mut invalid = request.clone();
        replace_source(
            &mut invalid,
            &claim(&draws(cost), "every scheduler", ">=", "4", "m.attempts = 1"),
        );
        let (code, output) = run(&invalid);
        assert_eq!(code, 2, "{output}");
        assert!(output.get("value").is_none());
    }
    let mut foreign_item = request;
    foreign_item["item"]["ordinal"] = json!(99);
    let (code, output) = run(&foreign_item);
    assert_eq!(code, 2, "{output}");
    assert!(output["reason"]
        .as_str()
        .unwrap()
        .contains("conflicting_identity"));
}

#[trace("TC-636", "FR-201-AC-22", "FR-282-AC-17")]
#[test]
fn cancellation_inside_qualified_reward_evaluation_stops_and_old_actions_still_work() {
    use qsl_eval::simulation::model::{
        ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
    };
    use qsl_eval::simulation::TransitionSystem;
    use qsl_replay::spine::SpineLimits;
    use qsl_semantics::model::observation::{DocumentRef, ObservationLimits};
    use quire_checked::key::DeclarationKey;
    use quire_exact::{Cancel, CancelCause};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, OnceLock};
    let source = claim(
        &draws("if lost then 2 else 6"),
        "every scheduler",
        ">=",
        "4",
        "m.attempts = 1",
    );
    let (checked, packages) = super::bounded::compile_source_with(&source, DOMAIN);
    let document = DocumentRef {
        authority: "test".to_owned(),
        identity: "s0".to_owned(),
    };
    let snapshots = [(document.clone(), SNAPSHOT.to_vec())]
        .into_iter()
        .collect();
    let limits = SpineLimits::default();
    let simulation = ModelSimulationRequest {
        package: checked.package(),
        packages: &packages,
        normalization: limits.model,
        snapshots: &snapshots,
        initial: vec![document],
        universes: vec![PopulationUniverse {
            population: DeclarationKey {
                package: "test/link".to_owned(),
                node: "ix://test/link/msgs".to_owned(),
            },
            keys: vec!["m".to_owned()],
        }],
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: 100_000,
        },
        meter: qsl_replay::AccountingLimits::default().0,
        emit: limits.package.emit(),
    };
    let system = ModelSystem::new(simulation.clone()).unwrap();
    let initial = system.initial().remove(0);
    let ordinary = system.actions(&initial).unwrap().unwrap();
    assert_eq!(ordinary.len(), 1);
    assert_eq!(ordinary[0].draws.len(), 2);
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let calls = Arc::new(AtomicU32::new(0));
        let seen = Arc::clone(&calls);
        let cancel = Cancel::observing(move || {
            if seen.fetch_add(1, Ordering::Relaxed) == 1 {
                weak.upgrade().unwrap().get().unwrap().cancel(cause);
            }
        });
        slot.set(cancel.clone()).unwrap();
        assert!(system.actions_with_cancel(&initial, &cancel).is_err());
        assert_eq!(cancel.tripped(), Some(cause));
        assert_eq!(
            calls.load(Ordering::Relaxed),
            2,
            "the callback runs inside a qualified reward meter"
        );
        assert_eq!(system.actions(&initial).unwrap().unwrap()[0].draws.len(), 2);
    }
}
