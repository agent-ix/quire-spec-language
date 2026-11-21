// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-636-AC-5 to AC-7 (TC-2988): `local-projection` and `realizability`
//! items carry `temporal.profile`, `temporal.max_depth` and (realizability
//! only) `temporal.max_history` as key members with no default.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, bounded, counter_request, label, litmus_request, order_protocol, Run, Workspace,
};

fn root() -> Value {
    let universe = crate::fixture_ids::universe_id("u");
    let effective = crate::fixture_ids::effective_id("e");
    let universe = fsl_foundation::node_wire::node_key_to_text(universe.root_type())
        .expect("universe identity encodes");
    let effective = Value::Array(
        effective
            .owner_chain()
            .iter()
            .map(|original| json!({"package": original.package(), "node": original.node()}))
            .collect(),
    );
    json!({"observation": {"authority": "test", "identity": "protocol-claim"},
        "subject": {"tag": "object", "universe": universe, "type": effective.to_string(), "object_identity": "flow-1"}})
}

fn subject() -> Value {
    let binding = json!({"type": "model-binding", "selection": "test/litmus"});
    json!({"protocol": {"protocol": "Flow",
        "over": {"reference": {"population": "ix://test/litmus/cores", "key": "core"}},
        "roles": {"Buyer": binding, "Seller": binding}}})
}

pub(crate) fn realizability(mut temporal: Value) -> Value {
    temporal["subject"] = subject();
    json!({"claim": "Flow", "kind": "realizability", "assessed_root": root(),
        "premises": {"role_knowledge": [], "environment": []}, "temporal": temporal})
}

fn projection(mut temporal: Value) -> Value {
    temporal["subject"] = subject();
    json!({"claim": "Flow", "kind": "local-projection", "assessed_root": root(),
        "options": {"role": "Seller"}, "temporal": temporal})
}

pub(crate) fn run_items(name: &str, items: Vec<Value>) -> Run {
    let ws = Workspace::new(name);
    analyze(&litmus_request(
        &ws,
        &order_protocol(true),
        json!({ "items": items }),
    ))
}

fn run_items_with_raw_bounds(name: &str, items: Vec<Value>) -> Run {
    let ws = Workspace::new(name);
    let path = litmus_request(&ws, &order_protocol(true), json!({"items": items}));
    replace_number(&path, "max_depth", 123456789, "18446744073709551616");
    replace_number(&path, "max_history", 123456789, "18446744073709551616");
    analyze(&path)
}

fn replace_number(path: &std::path::Path, member: &str, placeholder: u64, replacement: &str) {
    let source = std::fs::read_to_string(path).expect("request text");
    let token = format!("\"{member}\":{placeholder}");
    assert!(source.contains(&token), "request includes {token}");
    std::fs::write(
        path,
        source.replacen(&token, &format!("\"{member}\":{replacement}"), 1),
    )
    .expect("request text is rewritten");
}

fn assert_refused(item: &Value, cause: &str, member: &str) {
    assert_eq!(item["category"], "refusal", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["code"], "invalid_runtime_input",
        "{item}"
    );
    assert_eq!(item["record"]["diagnostic"]["cause"], cause, "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["member"], member,
        "{item}"
    );
}

fn assert_invalid_role(item: &Value) {
    assert_eq!(item["category"], "refusal", "{item}");
    assert_eq!(item["record"]["backend"], "protocol-analysis", "{item}");
    assert_eq!(item["record"]["result"], "declined", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "invalid-input", "{item}");
    assert_eq!(
        item["record"]["cause"]["code"], "invalid_runtime_input",
        "{item}"
    );
}

/// FR-636-AC-5: complete items settle under both profile words.
#[trace("TC-2988", "FR-636-AC-5")]
#[test]
fn complete_protocol_items_settle_under_both_profiles() {
    for profile in ["bounded-static", "bounded"] {
        let run = run_items(
            &format!("protocol-bounds-{profile}"),
            vec![
                realizability(json!({"profile": profile, "max_depth": 16, "max_history": 16})),
                projection(json!({"profile": profile, "max_depth": 16})),
            ],
        );
        assert_eq!(run.exit, 0, "{}", run.document());
        assert_eq!(label(&run.item(0)), "proved", "{}", run.item(0));
        assert_eq!(label(&run.item(1)), "proved", "{}", run.item(1));
    }
}

/// FR-636-AC-6: an absent key member settles its item `missing-member`; the
/// other items still run.
#[trace("TC-2988", "FR-636-AC-6")]
#[test]
fn an_absent_profile_or_bound_settles_missing_member() {
    let run = run_items(
        "protocol-bounds-absent",
        vec![
            realizability(json!({"profile": "bounded-static", "max_depth": 16})),
            realizability(json!({"max_depth": 16, "max_history": 16})),
            realizability(json!({"profile": "bounded-static", "max_history": 16})),
            projection(json!({"max_depth": 16})),
            realizability(json!({"profile": "bounded-static", "max_depth": 16, "max_history": 16})),
        ],
    );
    assert_eq!(run.exit, 20, "{}", run.document());
    for (index, member) in [
        (0, "temporal.max_history"),
        (1, "temporal.profile"),
        (2, "temporal.max_depth"),
        (3, "temporal.profile"),
    ] {
        assert_refused(&run.item(index), "missing-member", member);
    }
    assert_eq!(label(&run.item(4)), "proved", "{}", run.item(4));
}

/// FR-636-AC-7: an out-of-range profile or bound settles `invalid-value`,
/// and `max_history` on a projection settles `unknown-member`.
#[trace("TC-2988", "FR-636-AC-7")]
#[test]
fn an_out_of_range_profile_or_bound_is_refused_naming_it() {
    let run = run_items(
        "protocol-bounds-range",
        vec![
            realizability(json!({"profile": "static", "max_depth": 16, "max_history": 16})),
            realizability(json!({"profile": "bounded-static", "max_depth": 16, "max_history": 0})),
            realizability(json!({"profile": "bounded-static", "max_depth": 0, "max_history": 16})),
            projection(json!({"profile": "bounded-static", "max_depth": 16, "max_history": 16})),
        ],
    );
    assert_eq!(run.exit, 20, "{}", run.document());
    assert_refused(&run.item(0), "invalid-value", "temporal.profile");
    assert_refused(&run.item(1), "invalid-value", "temporal.max_history");
    assert_refused(&run.item(2), "invalid-value", "temporal.max_depth");
    assert_refused(&run.item(3), "unknown-member", "temporal.max_history");
}

/// FR-636's upper-bound range settles only the affected items; an unresolved
/// role uses projection's invalid-role refusal instead of an unsupported route.
#[trace("TC-2988", "FR-636-AC-7")]
#[test]
fn upper_bound_and_unknown_role_refusals_are_item_local() {
    const ABOVE_SAFE_INTEGER: u64 = 9_007_199_254_740_992;

    let mut unknown_role = projection(json!({"profile": "bounded-static", "max_depth": 16}));
    unknown_role["options"]["role"] = json!("NotARole");
    let run = run_items(
        "protocol-bounds-upper-and-role",
        vec![
            realizability(json!({
                "profile": "bounded-static",
                "max_depth": 16,
                "max_history": ABOVE_SAFE_INTEGER
            })),
            realizability(json!({
                "profile": "bounded-static",
                "max_depth": ABOVE_SAFE_INTEGER,
                "max_history": 16
            })),
            realizability(json!({
                "profile": "bounded-static",
                "max_depth": 16,
                "max_history": 16
            })),
            unknown_role,
        ],
    );
    assert_eq!(run.exit, 20, "{}", run.document());
    assert_refused(&run.item(0), "invalid-value", "temporal.max_history");
    assert_refused(&run.item(1), "invalid-value", "temporal.max_depth");
    assert_eq!(label(&run.item(2)), "proved", "{}", run.item(2));
    assert_invalid_role(&run.item(3));
}

/// FR-636: an integer beyond `u64` still refuses only its protocol item.
#[trace("TC-2988", "FR-636-AC-7")]
#[test]
fn integers_above_u64_settle_per_protocol_item() {
    let run = run_items_with_raw_bounds(
        "protocol-bounds-above-u64",
        vec![
            realizability(json!({
                "profile": "bounded-static",
                "max_depth": 16,
                "max_history": 123456789
            })),
            realizability(json!({
                "profile": "bounded-static",
                "max_depth": 123456789,
                "max_history": 16
            })),
            realizability(json!({
                "profile": "bounded-static",
                "max_depth": 16,
                "max_history": 16
            })),
        ],
    );
    assert_eq!(run.exit, 20, "{}", run.document());
    assert_refused(&run.item(0), "invalid-value", "temporal.max_history");
    assert_refused(&run.item(1), "invalid-value", "temporal.max_depth");
    assert_eq!(label(&run.item(2)), "proved", "{}", run.item(2));
}

/// FR-620: non-protocol temporal bounds keep the shared safe-count rejection.
#[trace("FR-620")]
#[test]
fn oversized_nonprotocol_depth_refuses_the_request() {
    let ws = Workspace::new("nonprotocol-depth-over-safe-count");
    let path = counter_request(
        &ws,
        &bounded(),
        json!([{"claim": "Bounded", "temporal": {"max_depth": 123456789}}]),
    );
    replace_number(&path, "max_depth", 123456789, "9007199254740992");
    let run = analyze(&path);
    assert_eq!(run.exit, 20, "{}", run.document());
    assert_eq!(run.document()["category"], "refusal", "{}", run.document());
}
