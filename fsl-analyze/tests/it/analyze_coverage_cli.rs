// SPDX-License-Identifier: AGPL-3.0-or-later
//! Further `fsl analyze` stubs for FR-621, FR-624, FR-625 and FR-627 to
//! FR-634 (TC-1581 to TC-1594), driven through the driver binary
//! (`$FSL_BIN`) from source plus a `native-analyze/1` request.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, below_three, bounded, clause, counter_request, counter_request_with, label,
    litmus_request, request_members, Workspace,
};

/// FR-621-AC-4: every category and record result is a registered kind.
#[trace("TC-1581", "FR-621-AC-4")]
#[test]
fn every_written_kind_is_registered() {
    let categories = [
        "success",
        "violation",
        "refusal",
        "unsupported",
        "incomplete",
        "pending",
        "indeterminate",
        "inconclusive",
        "internal-failure",
    ];
    let ws = Workspace::new("cov-kinds");
    let body = format!("{}{}", bounded(), below_three());
    let run = analyze(&counter_request(
        &ws,
        &body,
        json!([{"claim": "Bounded"}, {"claim": "BelowThree"},
            {"claim": "Bounded", "temporal": {"limits": {"max_states": 2}}}]),
    ));
    let document = run.document();
    assert_eq!(
        document["items"].as_array().map(Vec::len),
        Some(3),
        "{document}"
    );
    assert!(categories.contains(&document["category"].as_str().unwrap_or("")));
    for item in document["items"].as_array().expect("items") {
        assert!(
            categories.contains(&item["category"].as_str().unwrap_or("")),
            "{item}"
        );
    }
}

/// FR-621-AC-5: the machine-mode document is the same bytes on two runs of
/// one request (the library and driver write one serializer's output).
#[trace("TC-1581", "FR-621-AC-5")]
#[test]
fn the_machine_document_is_deterministic_bytes() {
    let ws = Workspace::new("cov-bytes");
    let path = counter_request(&ws, &below_three(), json!([{"claim": "BelowThree"}]));
    let first = analyze(&path);
    let second = analyze(&path);
    assert!(!first.stdout.is_empty());
    assert_eq!(first.document()["operation"], "analyze");
    assert_eq!(first.document()["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(first.stdout, second.stdout);
}

/// FR-621-AC-7: a counterexample value is spelled in FSpec FR-181's typed
/// form.
#[trace("TC-1581", "FR-621-AC-7")]
#[test]
fn counterexample_values_use_the_fr181_typed_form() {
    let ws = Workspace::new("cov-values");
    let run = analyze(&counter_request(
        &ws,
        &below_three(),
        json!([{"claim": "BelowThree"}]),
    ));
    let text = run.item(0)["counterexample"].to_string();
    assert!(text.contains(r#"{"type":"integer","value":"3"}"#), "{text}");
    assert!(!text.contains(r#"{"integer":"3"}"#), "{text}");
}

/// FR-624-AC-3: a binding refusal refuses the whole request.
#[trace("TC-1584", "FR-624-AC-3")]
#[test]
fn a_binding_refusal_refuses_the_request() {
    let ws = Workspace::new("cov-binding");
    let path = counter_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    let snapshot = json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "start"},
        "observation": "current", "model": {"identity": "test/counter"},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": "ix://test/counter/nowhere", "complete": true,
            "objects": []}],
    });
    ws.write("start.json", snapshot.to_string().as_bytes());
    let run = analyze(&path);
    assert_eq!(run.exit, 20);
    let code = run.document()["diagnostics"][0]["code"].clone();
    assert!(code.is_string(), "{code}");
    assert_ne!(
        code, "unknown_wire",
        "the request is read, then its binding refuses"
    );
    assert_ne!(code, "invalid-request", "the request is well formed");
    assert_eq!(run.document()["items"], json!([]));
}

/// FR-625-AC-5: an unbounded parameter with no proof bound is unsupported.
#[trace("TC-1585", "FR-625-AC-5")]
#[test]
fn an_unbounded_domain_is_unsupported() {
    let ws = Workspace::new("cov-unbounded");
    let path = counter_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    ws.write("model.json", &crate::counter::document(true));
    let run = analyze(&path);
    let item = run.item(0);
    assert_eq!(label(&item), "unsupported");
    assert_eq!(item["record"]["diagnostic"]["cause"], "unbounded-extent");
}

/// FR-627-AC-2, AC-3: weak each refutes where strong each proves, and the
/// refuted lasso replays with its fairness checked.
#[trace("TC-1587", "FR-627-AC-2")]
#[trace("TC-1587", "FR-627-AC-3")]
#[test]
fn weak_and_strong_fairness_separate() {
    let weak = clause(
        "Weak",
        "fair weak each M::Counter::inc;",
        "eventually holds(c.value = 3)",
    );
    let strong = clause(
        "Strong",
        "fair strong each M::Counter::inc;",
        "eventually holds(c.value = 3)",
    );
    let unfair = clause("Unfair", "", "eventually holds(c.value = 3)");
    let ws = Workspace::new("cov-fairness");
    // FR-627-AC-1's staying Counter: without fairness the `setTo(0)`
    // self-loop at 0 is a behaviour that never reaches 3.
    let run = analyze(&crate::analyze_cli_support::staying_counter_request(
        &ws,
        &format!("{weak}{strong}{unfair}"),
        json!([{"claim": "Weak"}, {"claim": "Strong"}, {"claim": "Unfair"}]),
    ));
    assert_eq!(label(&run.item(0)), "proved");
    assert_eq!(label(&run.item(1)), "proved");
    let refuted = run.item(2);
    assert_eq!(label(&refuted), "refuted");
    assert_eq!(refuted["replay"]["reproduced"], true);
    // FSpec FR-364's `TemporalCounterexample` names the lasso's cycle `loop`.
    assert!(refuted["counterexample"]["loop"].is_array(), "{refuted}");
}

/// FR-628-AC-2, AC-3: always-possible refutes, and seeded witnesses repeat.
#[trace("TC-1588", "FR-628-AC-2")]
#[trace("TC-1588", "FR-628-AC-3")]
#[test]
fn always_possible_refutes_and_witnesses_repeat() {
    let body = "always possible BackToZero using v over (c: M::Counter) { c.value = 0 }\n\
                possible ReachThree using v over (c: M::Counter) { c.value = 3 }\n";
    let items = json!([{"claim": "BackToZero"},
        {"claim": "ReachThree", "temporal": {"witness_samples": 3, "seed": 7}}]);
    let ws = Workspace::new("cov-state-graph");
    let path = counter_request(&ws, body, items);
    let first = analyze(&path);
    assert_eq!(label(&first.item(0)), "refuted");
    // FR-166 keeps one witness per instance and initial state; the record
    // names the walks and seed it ran under.
    let record = first.item(1)["record"].to_string();
    assert!(
        record.contains("\"witness_samples\":\"3\"") || record.contains("\"witness_samples\":3"),
        "{record}"
    );
    assert!(
        record.contains("\"seed\":\"7\"") || record.contains("\"seed\":7"),
        "{record}"
    );
    // Exactly one sampled witness (one instance, one initial state), whose
    // FSpec FR-393 source names seed 7 and a trace index below 3.
    fn sampled_sources<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
        match value {
            Value::Object(members) => {
                if members.get("type").is_some_and(|kind| kind == "sampled") {
                    out.push(value);
                }
                members
                    .values()
                    .for_each(|member| sampled_sources(member, out));
            }
            Value::Array(items) => items.iter().for_each(|item| sampled_sources(item, out)),
            _ => {}
        }
    }
    let witnessed = first.item(1);
    let mut sources = Vec::new();
    // The witness is spelled once in the FR-393 evidence; the record's basis and
    // the evidence paths cite the same source again.
    sampled_sources(&witnessed["record"]["evidence"]["sampled"], &mut sources);
    let [source] = sources.as_slice() else {
        panic!("exactly one sampled witness: {witnessed}");
    };
    assert_eq!(source["seed"], "7", "{source}");
    assert!(
        source["trace_index"]
            .as_str()
            .and_then(|index| index.parse::<u64>().ok())
            .is_some_and(|index| index < 3),
        "{source}"
    );
    let second = analyze(&path);
    assert_eq!(
        first.item(1)["record"]["evidence"],
        second.item(1)["record"]["evidence"]
    );
}

/// FR-629-AC-2, AC-3, AC-5 through FR-1033: release/acquire message passing,
/// a race, and a selection naming no weak parallel.
#[trace("TC-1589", "FR-629-AC-2")]
#[trace("TC-1589", "FR-629-AC-3")]
#[trace("TC-1589", "FR-629-AC-5")]
#[test]
fn release_acquire_races_and_selections() {
    let binding = json!({"type": "model-binding", "selection": "test/litmus"});
    let subject = json!({"protocol": {"protocol": "SB",
        "over": {"reference": {"population": "ix://test/litmus/cores", "key": "core"}},
        "roles": {"w": binding, "c": binding}}});
    let body = crate::litmus::sb_unit(&crate::litmus::sb_run("memory ra", "release", "acquire"));
    let ws = Workspace::new("cov-ra");
    let run = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": subject, "items": [{"claim": "NoBothZero"},
            {"claim": "SB::Both", "temporal": {"item_kind": "race-freedom"}}]}),
    ));
    assert!(matches!(label(&run.item(0)), "proved" | "refuted"));
    assert!(matches!(label(&run.item(1)), "proved" | "refuted"));
    let ws = Workspace::new("cov-ra-selection");
    let run = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": subject, "memory": [{"parallel": "Nowhere", "model": "tso"}],
            "items": [{"claim": "NoBothZero"}]}),
    ));
    assert_eq!(run.exit, 20);
    assert_eq!(
        run.document()["diagnostics"][0]["code"],
        "invalid_runtime_input"
    );
}

/// FR-631-AC-2, AC-3: copy-swap reduction and the witness-set budget.
#[trace("TC-1591", "FR-631-AC-2")]
#[trace("TC-1591", "FR-631-AC-3")]
#[test]
fn copy_swap_and_witness_budget() {
    use crate::hyper_world::{
        non_interference, request_files_over_starts, Vault, EQUAL_INPUTS, STARTS,
    };
    let declaration = format!("{}\nhyper Opaque using inf over behaviours (v: V::Vault) forall trace a of V exists trace b of V {{ always holds(v.l @ a = v.l @ b) }}", non_interference(EQUAL_INPUTS));
    let (unit, model, snapshots) =
        request_files_over_starts(Vault::Secure, false, &STARTS, &declaration);
    let ws = Workspace::new("cov-hyper");
    let run = analyze(&crate::analyze_cli_support::request_over(
        &ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [
            {"claim": "NonInterference"},
            {"claim": "Opaque", "temporal": {"limits": {"max_witness_set": 1}}},
        ]}),
    ));
    assert_eq!(run.item(0)["record"]["basis"]["type"], "reduced");
    assert_eq!(run.item(0)["record"]["certification"], "certified");
    assert_eq!(label(&run.item(1)), "incomplete");
    assert_eq!(run.item(1)["record"]["cause"]["limit"], "max_witness_set");
}

/// FR-632-AC-2, AC-3: digital clocks agree with zones, and a clock budget
/// of 0 stops the run.
#[trace("TC-1592", "FR-632-AC-2")]
#[trace("TC-1592", "FR-632-AC-3")]
#[test]
fn digital_clocks_agree_and_the_clock_budget_stops() {
    let claim = "temporal Bound using tt over (c: M::Call) clock \"model-time\" on origin \
                 { always holds(not c.late) }\n";
    let ws = Workspace::new("cov-zone");
    let run = analyze(&crate::analyze_family_cli::rpc_request(
        &ws,
        4,
        claim,
        json!({"items": [{"claim": "Bound"},
            {"claim": "Bound", "options": {"method": "digital-clocks"}}]}),
    ));
    assert_eq!(label(&run.item(0)), label(&run.item(1)));
    assert_eq!(run.item(1)["record"]["backend"], "en-1");
    // FR-632-AC-3: two calls hold two live clocks, so `max_clocks` 1 stops
    // the run; `max_clocks` 0 is not a positive count and refuses the request.
    let mut snapshot: Value =
        serde_json::from_slice(&crate::analyze_family_cli::rpc_snapshot()).expect("json");
    let mut second = snapshot["populations"][0]["objects"][0].clone();
    second["key"] = json!("d");
    snapshot["populations"][0]["objects"]
        .as_array_mut()
        .expect("objects")
        .push(second);
    let ws = Workspace::new("cov-zone-two-calls");
    let run = analyze(&crate::analyze_cli_support::request_over(
        &ws,
        &crate::analyze_family_cli::rpc_unit(4, claim),
        &crate::zone_search::rpc_document(),
        &[snapshot.to_string().into_bytes()],
        json!({"items": [{"claim": "Bound", "temporal": {"limits": {"max_clocks": 1}}}]}),
    ));
    assert_eq!(label(&run.item(0)), "incomplete");
    assert_eq!(run.item(0)["record"]["cause"]["limit"], "max_clocks");
    let ws = Workspace::new("cov-zone-zero-clocks");
    let run = analyze(&crate::analyze_family_cli::rpc_request(
        &ws,
        4,
        claim,
        json!({"items": [{"claim": "Bound", "temporal": {"limits": {"max_clocks": 0}}}]}),
    ));
    assert_eq!(run.exit, 20);
    assert_eq!(run.document()["diagnostics"][0]["code"], "invalid-request");
    assert!(run
        .document()
        .to_string()
        .contains("temporal.limits.max_clocks"));
}

/// FR-633-AC-3: EDF proves `Ctl` in closed form.
#[trace("TC-1593", "FR-633-AC-3")]
#[test]
fn edf_proves_ctl() {
    let ws = Workspace::new("cov-edf");
    let run = analyze(&crate::analyze_family_cli::ctl_request(
        &ws,
        "schedulable CtlEdf on Ctl under edf;\n",
        json!([{"claim": "CtlEdf"}]),
    ));
    assert_eq!(label(&run.item(0)), "proved");
    assert_eq!(run.item(0)["record"]["basis"]["analysis"], "edf");
}

/// FR-634-AC-2, AC-3: a state budget leaves a frontier, and seeded sampling
/// repeats.
#[trace("TC-1594", "FR-634-AC-2")]
#[trace("TC-1594", "FR-634-AC-3")]
#[test]
fn simulation_budget_and_seeded_sampling() {
    let ws = Workspace::new("cov-simulation");
    let request = request_members(Some(json!([
        {"kind": "simulation", "simulation": {"mode": "enumerate", "max_states": 2}},
        {"kind": "simulation", "simulation": {"mode": "sample", "seed": 5, "samples": 3}},
    ])));
    let path = counter_request_with(&ws, "", request);
    let first = analyze(&path);
    let stopped = first.item(0);
    assert_eq!(stopped["category"], "incomplete");
    // FSpec FR-181-AC-32: the stopped bound names its FR-461 setting and value.
    assert_eq!(stopped["cause"]["limit"], "max_states");
    assert_eq!(stopped["cause"]["setting"], "explore.states");
    assert_eq!(stopped["cause"]["value"], 2);
    assert!(stopped["frontier"]
        .as_array()
        .is_some_and(|frontier| !frontier.is_empty()));
    let second = analyze(&path);
    let sampled = |run: &crate::analyze_cli_support::Run| -> Value { run.item(1) };
    assert_eq!(sampled(&first), sampled(&second));
}
