// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-487 (TC-978) reader order over `monitor-plan-vectors.json`
//! `MP-01` to `MP-76`: the abstract reader applied to the described plan and
//! to the plan this producer writes. The IR reader (IR-721) decides the same
//! cases on the wire; nothing here reads the wire form back.

use ix_trace_rs::trace;
use serde_json::{json, Value};

const BINDINGS: [&str; 17] = qsl_replay::PLAN_ROOT_BINDINGS;

#[derive(Debug, Eq, PartialEq)]
enum Verdict {
    Admitted,
    Refused {
        code: &'static str,
        cause: Option<&'static str>,
        locus: String,
    },
}

fn refuse(code: &'static str, cause: Option<&'static str>, member: &str) -> Verdict {
    Verdict::Refused {
        code,
        cause,
        locus: format!("members/{member}"),
    }
}

fn is_int(value: &Value) -> bool {
    value.is_i64() || value.is_u64()
}

/// A canonical rational spelling (QSpec FR-322-AC-114): `n` or `n/d` with
/// `d > 1` and no common factor. Returns its value as `(numerator,
/// denominator)`.
fn canonical(text: &str) -> Option<(i128, i128)> {
    let (num, den) = match text.split_once('/') {
        Some((n, d)) => (n.parse::<i128>().ok()?, d.parse::<i128>().ok()?),
        None => return text.parse::<i128>().ok().map(|n| (n, 1)),
    };
    fn gcd(a: i128, b: i128) -> i128 {
        if b == 0 {
            a.abs()
        } else {
            gcd(b, a % b)
        }
    }
    (den > 1 && gcd(num, den) == 1).then_some((num, den))
}

fn expected_state(buffers: usize) -> Vec<(String, &'static str)> {
    let mut fields = vec![
        ("now".to_owned(), "integer"),
        ("history_start".to_owned(), "integer"),
    ];
    fields.extend((0..buffers).map(|n| (format!("buffer_{n}"), "sequence")));
    fields.push(("activations".to_owned(), "sequence"));
    fields.push(("event_position".to_owned(), "integer"));
    fields
}

/// FR-487 "Reader order", steps 1 to 5, on a plan description.
fn read(plan: &Value) -> Verdict {
    let premises = &plan["premises"];
    let names: Vec<&str> = plan["bindings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap())
        .collect();
    // 1. Shape.
    let mut shape: Vec<usize> = Vec::new();
    if let Some(at) =
        (0..names.len().max(BINDINGS.len())).find(|i| names.get(*i) != BINDINGS.get(*i))
    {
        shape.push(at);
    }
    for (member, name, kind) in [
        (8, "counter_width_bits", "int"),
        (9, "max_reading_gap", "int"),
        (10, "max_event_rate", "text"),
        (12, "counter", "text"),
        (14, "uncertainty", "text"),
        (16, "activation_capacity", "int"),
    ] {
        let value = &premises[name];
        let ok = if kind == "int" {
            is_int(value)
        } else {
            value.is_string()
        };
        if !ok {
            shape.push(member);
        }
    }
    if plan["buffers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|b| !is_int(&b["capacity"]))
    {
        shape.push(11);
    }
    if let Some(member) = shape.into_iter().min() {
        return refuse("invalid_semantic_graph", None, &member.to_string());
    }
    // 2. References.
    let mut refs: Vec<usize> = Vec::new();
    if plan["clause_form"] != "temporal_clause" {
        refs.push(0);
    }
    for (member, key, form) in [
        (2, "inputs_form", "record"),
        (3, "scope_form", "record"),
        (4, "results_form", "sequence"),
    ] {
        if plan[key] != form {
            refs.push(member);
        }
    }
    if plan["state"]["form"] != "record" {
        refs.push(1);
    }
    for (member, entry) in [
        (5, "init"),
        (6, "on_event"),
        (7, "on_watermark"),
        (15, "on_gap"),
    ] {
        let e = &plan["entries"][entry];
        if e["form"] != "pure_function" || e["declared"] != false {
            refs.push(member);
        }
    }
    if plan["tick_unit"]["form"] != "unit" || plan["tick_unit"]["dimension"] != "Time" {
        refs.push(13);
    }
    if let Some(member) = refs.into_iter().min() {
        return refuse("invalid_semantic_graph", None, &member.to_string());
    }
    // 3. Signatures.
    let mut sigs: Vec<usize> = Vec::new();
    let cases: Vec<(&str, Option<&str>)> = plan["outcome"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c[0].as_str().unwrap(), c[1].as_str()))
        .collect();
    let want_cases = [
        ("Failed", Some("ReadingGapExceeded")),
        ("False", None),
        ("Incomplete", Some("LimitReached")),
        ("Indeterminate", None),
        ("Pending", None),
        ("True", None),
    ];
    let payloads = &plan["payload_fields"];
    if cases != want_cases
        || payloads["LimitReached"] != json!(["limit", "value", "setting"])
        || payloads["ReadingGapExceeded"] != json!(["gap", "max_reading_gap"])
    {
        sigs.push(4);
    }
    for (member, entry, params, result) in [
        (5, "init", json!([]), "State"),
        (6, "on_event", json!(["State", "integer", "Inputs"]), "Step"),
        (
            7,
            "on_watermark",
            json!(["State", "integer", "Scope"]),
            "Step",
        ),
        (15, "on_gap", json!(["State", "integer"]), "Step"),
    ] {
        let e = &plan["entries"][entry];
        if e["params"] != params || e["result"] != result {
            sigs.push(member);
        }
    }
    if let Some(member) = sigs.into_iter().min() {
        return refuse("ill_typed", Some("type-mismatch"), &member.to_string());
    }
    // 4. Premises, in body order.
    let int = |name: &str| premises[name].as_i64().unwrap();
    let invalid = |member: String| Verdict::Refused {
        code: "invalid_package",
        cause: Some("invalid-value"),
        locus: format!("members/{member}/value"),
    };
    let width = int("counter_width_bits");
    if !(1..=64).contains(&width) {
        return invalid("8".into());
    }
    let gap = premises["max_reading_gap"].as_i64().unwrap_or(-1);
    if gap < 0 || i128::from(gap) >= 1_i128 << width {
        return invalid("9".into());
    }
    match canonical(premises["max_event_rate"].as_str().unwrap()) {
        Some((n, _)) if n > 0 => {}
        _ => return invalid("10".into()),
    }
    for (k, b) in plan["buffers"].as_array().unwrap().iter().enumerate() {
        if b["capacity"].as_i64().unwrap() < 1 {
            return Verdict::Refused {
                code: "invalid_package",
                cause: Some("invalid-value"),
                locus: format!("members/11/value/members/{k}/members/1/value"),
            };
        }
    }
    if premises["counter"].as_str().unwrap().is_empty() {
        return invalid("12".into());
    }
    match canonical(premises["uncertainty"].as_str().unwrap()) {
        Some((n, _)) if n >= 0 => {}
        _ => return invalid("14".into()),
    }
    if int("activation_capacity") < 0 {
        return invalid("16".into());
    }
    // 5. Buffers and state.
    let buffers = plan["buffers"].as_array().unwrap();
    let ascending = buffers
        .windows(2)
        .all(|p| p[0]["subformula"].as_i64() < p[1]["subformula"].as_i64());
    if !ascending || buffers.iter().any(|b| b["reached"] != true) {
        return refuse("invalid_semantic_graph", None, "11");
    }
    let fields: Vec<(String, String)> = plan["state"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f[0].as_str().unwrap().to_owned(),
                f[1].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let want = expected_state(buffers.len());
    let state_ok = fields.len() == want.len()
        && fields
            .iter()
            .zip(&want)
            .all(|(a, b)| a.0 == b.0 && a.1 == b.1)
        && plan["state"]["activation_fields"] == json!(["subformula", "origin", "status"]);
    if !state_ok {
        return refuse("invalid_semantic_graph", None, "1");
    }
    Verdict::Admitted
}

/// Apply a dotted-path edit (list indexes numeric).
fn set(plan: &mut Value, path: &str, value: &Value) {
    let mut at = plan;
    let parts: Vec<&str> = path.split('.').collect();
    for (n, part) in parts.iter().enumerate() {
        let last = n + 1 == parts.len();
        let next = match part.parse::<usize>() {
            Ok(index) => at.get_mut(index).unwrap(),
            Err(_) => at.get_mut(*part).unwrap(),
        };
        if last {
            *next = value.clone();
            return;
        }
        at = next;
    }
}

fn vectors() -> Value {
    let path = std::path::Path::new(&std::env::var("QSPEC_DIR").expect("QSPEC_DIR"))
        .join("proposals/checked-package-v2/monitor-plan-vectors.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[trace("TC-978", "FR-487-AC-2")]
#[trace("TC-978", "FR-487-AC-3")]
#[trace("TC-978", "FR-487-AC-4")]
#[trace("TC-978", "FR-487-AC-8")]
#[test]
fn the_reader_order_decides_every_monitor_plan_vector() {
    let all = vectors();
    for case in all["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let mut plan = all["base"].clone();
        for edit in case["set"].as_array().unwrap() {
            set(&mut plan, edit[0].as_str().unwrap(), &edit[1]);
        }
        let got = read(&plan);
        let want = if case["expected"] == "admitted" {
            Verdict::Admitted
        } else {
            Verdict::Refused {
                code: match case["code"].as_str().unwrap() {
                    "invalid_semantic_graph" => "invalid_semantic_graph",
                    "ill_typed" => "ill_typed",
                    "invalid_package" => "invalid_package",
                    other => panic!("{id}: code {other}"),
                },
                cause: match case["cause"].as_str() {
                    None => None,
                    Some("type-mismatch") => Some("type-mismatch"),
                    Some("invalid-value") => Some("invalid-value"),
                    Some(other) => panic!("{id}: cause {other}"),
                },
                locus: case["locus"]
                    .as_str()
                    .unwrap()
                    .rsplit_once("/body/")
                    .unwrap()
                    .1
                    .to_owned(),
            }
        };
        assert_eq!(got, want, "{id}");
    }
}

/// The plan this producer writes, described as the vectors' base is,
/// is admitted by the same reader order (MP-01).
#[trace("TC-978", "FR-487-AC-1")]
#[test]
fn the_written_plan_is_admitted_by_the_reader_order() {
    let all = vectors();
    let mut plan = all["base"].clone();
    // The producer's State has one buffer and its Activation is the
    // `subformula, origin, status` record; describe them from the generator.
    plan["buffers"] = json!([{ "subformula": 1, "capacity": 5, "reached": true }]);
    plan["state"]["fields"] = json!(expected_state(1)
        .into_iter()
        .map(|(n, k)| json!([n, k]))
        .collect::<Vec<_>>());
    assert_eq!(read(&plan), Verdict::Admitted);
}

/// MP-81 and MP-82 (FR-487-AC-8): the reader does not constrain the type of
/// an activation's `status`, so the record holding the integer `position` of
/// an "on each" instance (FR-251-AC-17) is admitted, as is a text scalar.
#[trace("TC-978", "FR-487-AC-8")]
#[test]
fn the_status_type_is_not_constrained_by_the_reader() {
    let all = vectors();
    let mut seen = Vec::new();
    for case in all["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        if id != "MP-81" && id != "MP-82" {
            continue;
        }
        let mut plan = all["base"].clone();
        for edit in case["set"].as_array().unwrap() {
            set(&mut plan, edit[0].as_str().unwrap(), &edit[1]);
        }
        assert_eq!(read(&plan), Verdict::Admitted, "{id}");
        seen.push(id.to_owned());
    }
    assert_eq!(seen, ["MP-81", "MP-82"]);
    // The status this producer writes.
    let mut plan = all["base"].clone();
    plan["buffers"] = json!([{ "subformula": 1, "capacity": 5, "reached": true }]);
    plan["state"]["fields"] = json!(expected_state(1)
        .into_iter()
        .map(|(n, k)| json!([n, k]))
        .collect::<Vec<_>>());
    set(
        &mut plan,
        "state.activation_status",
        &json!("record{position:integer}"),
    );
    assert_eq!(read(&plan), Verdict::Admitted);
}
