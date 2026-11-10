// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real cyclic proofs cross a fresh qualified-core process boundary.

use super::bounded::{document, key, mutate, request_with, run};
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

fn model(transient: bool) -> String {
    let language = include_str!("../fixtures/link.native")
        .lines()
        .next()
        .unwrap();
    let (post, weights) = if transient {
        ("self.attempts = (if pre(self.attempts) = 0 then 1 else if lost = 0 then 0 else 2) and self.delivered = (pre(self.attempts) = 1 and lost = 1)", "0: 2, 1: 1, 2: 1")
    } else {
        (
            "self.attempts = (if lost = 0 then 0 else 2) and self.delivered = (lost = 1)",
            "0: 1, 1: 1, 2: 1",
        )
    };
    format!(
        "{language}\n\
        profile v = \"quire.model.complete/v1\";\n\
        profile inf = \"quire.temporal.infinite-trace/v1\";\n\
        model M = \"test/link\";\n\
        pre A using v on M::Msg::send_a {{ true }}\n\
        post A_done using v on M::Msg::send_a {{ {post} }}\n\
        pre B using v on M::Msg::send_b {{ false }}\n\
        post B_done using v on M::Msg::send_b {{ self.attempts = 0 and not self.delivered }}\n\
        pre C using v on M::Msg::send_c {{ false }}\n\
        post C_done using v on M::Msg::send_c {{ self.attempts = 0 and not self.delivered }}\n\
        random Msg::send_a(lost) ~ {{ {weights} }};\n\
        random Msg::send_b(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        random Msg::send_c(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        terminal Stop using v on M when {{ self.attempts = 2 and not self.delivered }};\n"
    )
}

fn claim(model: &str, scheduler: &str, op: &str, threshold: &str, target: &str) -> String {
    format!("{model}\nprobabilistic Likely using inf on M over (m: M::Msg) under {scheduler} on origin {{ probability {op} {threshold} [ eventually holds({target}) ] }}\n")
}

fn proof(source: &str) -> Value {
    let (request, settlement) = request_with(
        source,
        SNAPSHOT,
        ExactProbLimits::default(),
        domain().as_bytes(),
    );
    request
        .unwrap_or_else(|| panic!("cyclic proof must retain a document: {settlement:?}\n{source}"))
}

fn accepted(request: &Value, n: &str, d: &str) {
    let (code, output) = run(request);
    assert_eq!(code, 0, "{output}");
    assert_eq!(output["value"], json!({"numerator": n, "denominator": d}));
}

fn replace_source(request: &mut Value, source: &str) {
    request["source"]["content"] = json!(source);
    request["byte_provision"][0][2] = json!(source.as_bytes());
}

#[trace("TC-636", "FR-201-AC-23", "FR-201-AC-24", "FR-282-AC-18")]
#[test]
fn self_loop_and_transient_scc_accept_exact_values_and_shortest_distances() {
    for transient in [false, true] {
        for op in [">=", "<="] {
            let source = claim(
                &model(transient),
                "every scheduler",
                op,
                "rational(1,2)",
                "m.delivered",
            );
            let request = proof(&source);
            accepted(&request, "1", "2");
            let doc = document(&request);
            assert_eq!(
                doc["objective"]["profile"],
                "quire.probability-objective/cyclic-dtmc-eventual/v1"
            );
            assert_eq!(
                doc["values"].as_array().unwrap().len(),
                if transient { 4 } else { 3 }
            );
            let ranks: Vec<_> = doc["ranking"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["rank"].as_u64().unwrap())
                .collect();
            assert_eq!(
                ranks.iter().copied().max(),
                Some(if transient { 2 } else { 1 })
            );
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
                &claim(
                    &model(transient),
                    "every scheduler",
                    op,
                    if op == ">=" {
                        "rational(3,5)"
                    } else {
                        "rational(2,5)"
                    },
                    "m.delivered",
                ),
            );
            let (code, output) = run(&stronger);
            assert_eq!(code, 1, "{output}");
            assert_eq!(output["rule"], "threshold-missed");
            let wrong = mutate(&request, |doc| {
                for row in doc["ranking"].as_array_mut().unwrap() {
                    if row["rank"] == 2 {
                        row["rank"] = json!(1);
                    } else if row["rank"] == 1 && !transient {
                        row["rank"] = json!(2);
                    }
                }
            });
            let (code, output) = run(&wrong);
            assert_eq!(code, 1, "{output}");
            assert_eq!(output["rule"], "rank-broken");
        }
    }
}

#[trace("TC-636", "FR-201-AC-23", "FR-201-AC-25", "FR-282-AC-18")]
#[test]
fn target_free_recurrence_forces_zero_and_early_target_cannot_hide_scheduler_choice() {
    let closed = model(true).replace("0: 2, 1: 1, 2: 1", "0: 1");
    let source = claim(&closed, "every scheduler", "<=", "0", "m.delivered");
    let request = proof(&source);
    accepted(&request, "0", "1");
    let wrong = mutate(&request, |doc| {
        for row in doc["values"].as_array_mut().unwrap() {
            row["value"] = json!({"numerator":"1", "denominator":"2"});
        }
    });
    let (code, output) = run(&wrong);
    assert_eq!(code, 1, "{output}");
    assert_eq!(output["rule"], "inequality-fails");
    let source = claim(
        &model(false),
        "every scheduler",
        ">=",
        "rational(1,2)",
        "m.delivered",
    );
    let request = proof(&source);
    let choices = model(false).replace(
        "pre B using v on M::Msg::send_b { false }",
        "pre B using v on M::Msg::send_b { self.delivered }",
    );
    let mut changed = request;
    replace_source(
        &mut changed,
        &claim(
            &choices,
            "every scheduler",
            ">=",
            "rational(1,2)",
            "m.delivered",
        ),
    );
    let (code, output) = run(&changed);
    assert_ne!(
        code, 0,
        "an original choice beyond target must not be accepted: {output}"
    );
    assert_eq!(output["code"], "unsupported");
}
