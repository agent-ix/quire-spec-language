// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-630 to FR-633 and FR-635 (TC-1590 to TC-1593, TC-1595): one accepting
//! and one refuted `fsl analyze` item for the refinement, hyperproperty,
//! timed zone and schedulability families, and an item settled on an
//! external provider, each from source plus a `native-analyze/1` request
//! through the driver binary (`$FSL_BIN`).

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, counter_request, counter_request_with, label, request_members, request_over, Workspace,
};

/// The two items' results, in request order.
fn results(run: &crate::analyze_cli_support::Run, count: usize) -> Vec<String> {
    (0..count)
        .map(|index| label(&run.item(index)).to_owned())
        .collect()
}

/// A refinement of the Counter by an abstract Counter selected as `Spec`,
/// with the extra rows `rows` and the abstract value given by `mapped`.
fn refinement(name: &str, mapped: &str, rows: &str) -> String {
    refinement_stepping(name, mapped, "Spec::Counter::inc(self)", rows)
}

/// [`refinement`] with the concrete `inc` mapped to the abstract `step`.
fn refinement_stepping(name: &str, mapped: &str, step: &str, rows: &str) -> String {
    format!(
        "refinement {name} using t0 abstract Spec concrete M {{\n\
         population Spec::counters from M::counters;\n\
         object Spec::Counter from M::Counter {{ value = {mapped}; }}\n\
         step M::Counter::inc -> {step};\n{rows}}}\n"
    )
}

/// The abstract Counter package: the Counter domain package under its own
/// identity, which the unit selects as `Spec`.
const SPEC: &str = "test/counter-spec";

/// FSpec FR-368's `abstract_subject` starting at the [`SPEC`] snapshot
/// `identity`: `spec-start` holds counter `a` at 0, `spec-start-one` at 1.
fn abstract_start(identity: &str) -> Value {
    json!({"identity": SPEC, "initial_states": [{"authority": "test", "identity": identity}]})
}

/// The Counter request with the abstract `Spec` model, a copy of the Counter
/// domain package under [`SPEC`], selected in the unit header. The concrete
/// subject is counter `a` at 0; each item's `abstract_subject` (FSpec
/// FR-368) names the abstract snapshot, `spec-start` unless the item names
/// one.
fn refinement_request(ws: &Workspace, body: &str, items: Value) -> std::path::PathBuf {
    // The abstract `inc` carries the concrete one's contract.
    let contract = crate::counter::CONTRACT
        .replace("M::", "Spec::")
        .replace("CanInc", "SpecCanInc")
        .replace("post Inc", "post SpecInc");
    let unit = crate::counter::unit_text(&format!("{}{contract}{body}", crate::counter::CONTRACT))
        .replacen(
            "model M = \"test/counter\";\n",
            &format!("model M = \"test/counter\";\nmodel Spec = \"{SPEC}\";\n"),
            1,
        );
    let spec = |bytes: Vec<u8>| -> Vec<u8> {
        String::from_utf8(bytes)
            .expect("the fixture is UTF-8")
            .replace("test/counter", SPEC)
            .into_bytes()
    };
    let snapshot = crate::analyze_cli_support::counter_snapshot(&[("a", 0)]);
    let spec_snapshot = |value: i64, identity: &str| -> Vec<u8> {
        String::from_utf8(spec(crate::analyze_cli_support::counter_snapshot(&[(
            "a", value,
        )])))
        .expect("the fixture is UTF-8")
        .replace(
            "\"identity\":\"start\"",
            &format!("\"identity\":\"{identity}\""),
        )
        .into_bytes()
    };
    ws.write("program.fsl", unit.as_bytes());
    ws.write("model.json", &crate::counter::document(false));
    ws.write("spec.json", &spec(crate::counter::document(false)));
    ws.write("start.json", &snapshot);
    ws.write("spec-start.json", &spec_snapshot(0, "spec-start"));
    ws.write("spec-start-one.json", &spec_snapshot(1, "spec-start-one"));
    let mut items = items;
    for item in items.as_array_mut().expect("items") {
        // An item whose `abstract_subject` is JSON null names none, which
        // FR-630 defaults to the mapping's image of the concrete initial states.
        match item["temporal"].get("abstract_subject") {
            None => item["temporal"]["abstract_subject"] = abstract_start("spec-start"),
            Some(Value::Null) => {
                item["temporal"]
                    .as_object_mut()
                    .expect("temporal")
                    .remove("abstract_subject");
            }
            Some(_) => {}
        }
    }
    let mut request = request_members(Some(items));
    request["models"].as_array_mut().expect("models").push(
        json!({"format": "semantic-ir/2.0.0", "source": {"file": "spec.json",
            "authority": "test", "identity": "counter-spec-model", "document": "Counter",
            "formal_revision": 1}}),
    );
    request["documents"] = json!([{"file": "start.json"}, {"file": "spec-start.json"},
        {"file": "spec-start-one.json"}]);
    let start = json!([{"authority": "test", "identity": "start"}]);
    request["subject"] = json!({"initial_states": start, "universes": start, "proof_bounds": []});
    let document = json!({"format": "native-analyze/1", "request": request});
    ws.write("analyze.json", document.to_string().as_bytes())
}

/// FR-630-AC-1: a correct mapping proves and a wrong one refutes with a
/// replayed counterexample carrying per-step records. The wrong mapping
/// sends the empty counter (`value = 0`) to a non-empty one (`value = 1`);
/// its abstract subject starts at that image, as FR-630's default does, so
/// the first concrete `inc` maps to an abstract `inc` the abstract contract
/// rejects: `abstract-step-disabled` (RS-4, `AbstractStepRejected`).
#[trace("TC-1590", "FR-630-AC-1")]
#[test]
fn a_refinement_proves_and_a_wrong_mapping_refutes() {
    let body = refinement("Same", "self.value", "")
        + &refinement("Off", "if self.value = 0 then 1 else self.value", "");
    let ws = Workspace::new("family-refinement");
    let run = analyze(&refinement_request(
        &ws,
        &body,
        json!([{"claim": "Same", "kind": "refinement", "temporal": {"item_kind": "refinement"}},
            {"claim": "Off", "kind": "refinement", "temporal": {"item_kind": "refinement",
                "abstract_subject": abstract_start("spec-start-one")}}]),
    ));
    assert_eq!(results(&run, 2), ["proved", "refuted"]);
    assert_eq!(run.item(0)["record"]["certification"], "certified");
    let refuted = run.item(1);
    assert_eq!(refuted["replay"]["reproduced"], true);
    assert!(refuted["counterexample"]["steps"].is_array(), "{refuted}");
    assert_eq!(
        refuted["counterexample"]["refinement_failure"]["kind"], "AbstractStepRejected",
        "{refuted}"
    );
}

/// FR-630 "Request item", FR-144: with no `abstract_subject` the abstract
/// initial states are the states the mapping gives the concrete initial
/// states, so the wrong mapping of the test above refutes the same way
/// without naming the abstract snapshot (`value = 1`), which the concrete
/// subject's snapshot (`value = 0`) is not.
#[trace("TC-1590", "FR-630")]
#[test]
fn an_absent_abstract_subject_starts_at_the_mapped_concrete_initial_states() {
    let body = refinement("Same", "self.value", "")
        + &refinement("Off", "if self.value = 0 then 1 else self.value", "");
    let ws = Workspace::new("family-refinement-mapped-initial");
    let run = analyze(&refinement_request(
        &ws,
        &body,
        json!([{"claim": "Same", "kind": "refinement", "temporal": {"item_kind": "refinement", "abstract_subject": null}},
            {"claim": "Off", "kind": "refinement", "temporal": {"item_kind": "refinement",
                "abstract_subject": null}}]),
    ));
    assert_eq!(results(&run, 2), ["proved", "refuted"]);
    assert_eq!(run.item(0)["record"]["certification"], "certified");
    let refuted = run.item(1);
    assert_eq!(refuted["replay"]["reproduced"], true, "{refuted}");
    assert_eq!(
        refuted["counterexample"]["refinement_failure"]["kind"], "AbstractStepRejected",
        "{refuted}"
    );
}

/// FR-630 "Outputs": the refuted item's counterexample carries the abstract
/// image of each position, from the initial state to the first failing
/// edge, and the refinement's FR-146 step records; it is refuted at that
/// first edge, the concrete `inc` the wrong row maps to a stutter.
#[trace("TC-1590", "FR-630-AC-1")]
#[test]
fn a_refuted_refinement_names_its_first_failing_edge_and_abstract_images() {
    let ws = Workspace::new("family-refinement-images");
    let run = analyze(&refinement_request(
        &ws,
        &refinement_stepping("Off", "self.value", "stutter", ""),
        json!([{"claim": "Off", "kind": "refinement", "temporal": {"item_kind": "refinement"}}]),
    ));
    let refuted = run.item(0);
    assert_eq!(label(&refuted), "refuted", "{refuted}");
    let counterexample = &refuted["counterexample"];
    assert_eq!(
        counterexample["prefix"].as_array().map(Vec::len),
        Some(1),
        "the first edge fails: {counterexample}"
    );
    let steps = counterexample["steps"].as_array().expect("steps");
    assert_eq!(steps.len(), 2, "positions 0 and 1: {counterexample}");
    for (position, step) in steps.iter().enumerate() {
        assert_eq!(step["position"], position, "{step}");
        assert_eq!(step["abstract_state"]["type"], "simulation-state", "{step}");
    }
    assert!(
        counterexample["step_simulations"].is_array(),
        "{counterexample}"
    );
}

/// FR-630 "Budgets" (FR-625): a refinement item whose product reaches
/// `max_states` 1 settles `incomplete` naming the limit, never `proved`.
#[trace("TC-1590", "FR-630")]
#[test]
fn a_refinement_budget_of_one_state_is_incomplete() {
    let ws = Workspace::new("family-refinement-budget");
    let run = analyze(&refinement_request(
        &ws,
        &refinement("Same", "self.value", ""),
        json!([{"claim": "Same", "kind": "refinement",
            "temporal": {"item_kind": "refinement", "limits": {"max_states": 1}}}]),
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "incomplete", "{item}");
    assert!(item["record"].to_string().contains("max_states"), "{item}");
}

/// FR-630, FR-143: a strong abstract fairness constraint gives the
/// liveness half the typed result the library gives it, through the wire. The
/// only concrete step maps to the strongly fair abstract `inc`, so no
/// abstract-unfair loop exists and the claim is decided `proved`; the refuted
/// reading is covered by the refinement liveness tests.
#[trace("TC-1590", "FR-630")]
#[test]
fn a_strong_abstract_fairness_refinement_settles_the_library_result() {
    let ws = Workspace::new("family-refinement-strong");
    let run = analyze(&refinement_request(
        &ws,
        &refinement(
            "Strong",
            "self.value",
            "ensure fair strong each Spec::Counter::inc;\n",
        ),
        json!([{"claim": "Strong", "kind": "refinement",
            "temporal": {"item_kind": "refinement"}}]),
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "proved", "{item}");
}

/// FR-630, FSpec FR-368: an `abstract_subject` whose `identity` is not the
/// refinement's abstract model package settles `invalid-request`.
#[trace("TC-1590", "FR-630")]
#[test]
fn an_abstract_subject_of_another_package_is_an_invalid_request() {
    let ws = Workspace::new("family-refinement-identity");
    let path = refinement_request(
        &ws,
        &refinement("Same", "self.value", ""),
        json!([{"claim": "Same", "kind": "refinement", "temporal": {"item_kind": "refinement"}}]),
    );
    let mut request: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("the request reads"))
            .expect("the request is JSON");
    request["request"]["items"][0]["temporal"]["abstract_subject"]["identity"] =
        json!("test/counter");
    let path = ws.write("analyze.json", request.to_string().as_bytes());
    let run = analyze(&path);
    let item = run.item(0);
    assert_eq!(item["category"], "refusal", "{item}");
    assert_eq!(item["record"]["settled_by"], "disposition", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["member"], "temporal.abstract_subject.identity",
        "{item}"
    );
}

/// FR-630-AC-2: `refinement_internals` reaches compilation.
#[trace("TC-1590", "FR-630-AC-2")]
#[test]
#[ignore = "QSL-2245: refinement_internals is not an analyze request member yet"]
fn refinement_internals_are_a_request_member() {
    let ws = Workspace::new("family-refinement-internals");
    let mut request = request_members(Some(json!([
        {"claim": "Same", "kind": "refinement", "temporal": {"item_kind": "refinement"}}
    ])));
    request["refinement_internals"] = json!({"Same": []});
    let run = analyze(&counter_request_with(
        &ws,
        &format!(
            "model Spec = \"test/counter\";\n{}",
            refinement("Same", "self.value", "")
        ),
        request,
    ));
    assert_eq!(run.exit, 0, "{}", String::from_utf8_lossy(&run.stdout));
    assert_eq!(label(&run.item(0)), "proved");
}

/// FR-630-AC-3, FR-138-AC-9: decimal and enum history rows check and the
/// refinement item over each proves.
#[trace("TC-1590", "FR-630-AC-3")]
#[trace("TC-1590", "FR-138-AC-9")]
#[test]
#[ignore = "QSL-2245: decimal and enum history rows are refused not-yet-implemented"]
fn decimal_and_enum_history_rows_check_and_prove() {
    for (name, rows) in [
        (
            "DecimalHistory",
            "history M::Counter last : Decimal[0, 100; 0, 2; nearest-even] = decimal(0, 0) \
             on M::Counter::inc {{ decimal(1, 0) }};\n",
        ),
        (
            "EnumHistory",
            "history M::Counter last : Phase = Phase::idle on M::Counter::inc {{ Phase::busy }};\n",
        ),
    ] {
        let rows = rows.replace("{{", "{").replace("}}", "}");
        let body =
            "enum Phase { idle, busy }\n".to_owned() + &refinement(name, "self.value", &rows);
        let ws = Workspace::new(&format!("family-history-{name}"));
        let run = analyze(&refinement_request(
            &ws,
            &body,
            json!([{"claim": name, "kind": "refinement", "temporal": {"item_kind": "refinement"}}]),
        ));
        assert_eq!(
            run.document()["last_stage"],
            Value::Null,
            "{name}: no compile refusal"
        );
        assert_eq!(label(&run.item(0)), "proved", "{name}");
    }
}

/// FR-630-AC-4, FR-135-AC-6: a nested-reference row checks and proves.
#[trace("TC-1590", "FR-630-AC-4")]
#[trace("TC-1590", "FR-135-AC-6")]
#[test]
#[ignore = "QSL-2245: nested-reference rows are refused not-yet-implemented"]
fn a_nested_reference_row_checks_and_proves() {
    let body = "record Slots { held: Option<Reference<M::Counter>>; }\n".to_owned()
        + &refinement(
            "Nested",
            "self.value",
            "history M::Counter slot : Slots = Slots { held: none } on M::Counter::inc \
             { Slots { held: some(self) } };\n",
        );
    let ws = Workspace::new("family-nested-reference");
    let run = analyze(&refinement_request(
        &ws,
        &body,
        json!([{"claim": "Nested", "kind": "refinement", "temporal": {"item_kind": "refinement"}}]),
    ));
    assert_eq!(label(&run.item(0)), "proved");
}

/// FR-631-AC-1: non-interference proves for the secure vault and refutes
/// for the leaky one with a two-trace counterexample.
#[trace("TC-1591", "FR-631-AC-1")]
#[test]
fn non_interference_proves_and_refutes() {
    use crate::hyper_world::{
        EQUAL_INPUTS, STARTS, Vault, non_interference, request_files_over_starts,
    };
    for (vault, expected) in [(Vault::Secure, "proved"), (Vault::Leaky, "refuted")] {
        let (unit, model, snapshots) =
            request_files_over_starts(vault, false, &STARTS, &non_interference(EQUAL_INPUTS));
        let ws = Workspace::new(&format!("family-hyper-{expected}"));
        let run = analyze(&request_over(
            &ws,
            &unit,
            &model,
            &snapshots,
            json!({"items": [{"claim": "NonInterference"}]}),
        ));
        let item = run.item(0);
        assert_eq!(label(&item), expected);
        if expected == "refuted" {
            assert_eq!(
                item["counterexample"]["traces"].as_array().map(Vec::len),
                Some(2)
            );
            assert_eq!(item["replay"]["reproduced"], true);
        }
    }
}

/// FR-631-AC-4: a hyper clause with no object parameter is analyzable.
#[trace("TC-1591", "FR-631-AC-4")]
#[test]
#[ignore = "QSL-2128: hyperproperty items are not reachable through fsl analyze yet"]
fn an_object_free_hyper_clause_is_analyzed() {
    use crate::hyper_world::{request_files, Vault};
    let declaration = "hyper Whole using inf over behaviours () \
                       forall trace a of V forall trace b of V { always holds(true) }";
    let (unit, model, snapshot) = request_files(Vault::Secure, false, (1, 0), declaration);
    let ws = Workspace::new("family-hyper-object-free");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &[snapshot],
        json!({"items": [{"claim": "Whole"}]}),
    ));
    assert!(matches!(label(&run.item(0)), "proved" | "refuted"));
}

/// The timed `Rpc` subject of FR-239 with timeout bound `t` and `claim`.
pub(crate) fn rpc_unit(t: u32, claim: &str) -> String {
    rpc_unit_with(t, ">= 1", ">=", claim)
}

/// [`rpc_unit`] with `reply` guarded by `self.x {reply} ms` and `timeout` by
/// `self.x {timeout} {t} ms`: ADR-026 section 11's strict-guard variant is
/// `("> 3", ">")`.
pub(crate) fn rpc_unit_with(t: u32, reply: &str, timeout: &str, claim: &str) -> String {
    use crate::timed_world::{unit_text, DENSE, PACKAGE};
    let body = format!(
        "{DENSE}\
         pre SendIdle using v on M::Call::send {{ self.phase = 0 }}\n\
         post Send using v on M::Call::send {{ \
             self.phase = 1 and self.inflight and not self.late and self.x = 0 ms }}\n\
         pre Reply using v on M::Call::reply {{ self.inflight and self.x {reply} ms }}\n\
         post Replied using v on M::Call::reply {{ not self.inflight and \
             self.late = (pre(self.phase) = 3) and \
             (pre(self.phase) = 1 implies self.phase = 2) and \
             (pre(self.phase) != 1 implies self.phase = pre(self.phase)) }}\n\
         pre Timeout using v on M::Call::timeout {{ self.phase = 1 and self.x {timeout} {t} ms }}\n\
         post TimedOut using v on M::Call::timeout {{ self.phase = 3 }}\n\
         pre Reset using v on M::Call::reset {{ \
             not self.inflight and (self.phase = 2 or self.phase = 3) }}\n\
         post ResetIdle using v on M::Call::reset {{ self.phase = 0 }}\n\
         time invariant ServerDeadline when {{ self.inflight }} {{ self.x <= 3 ms }}\n\
         time invariant ClientDeadline when {{ self.phase = 1 }} {{ self.x <= {t} ms }}\n\
         {claim}"
    );
    unit_text(PACKAGE, crate::zone_search::TIMED, &body)
}

pub(crate) fn rpc_snapshot() -> Vec<u8> {
    let package = crate::timed_world::PACKAGE;
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "s0"},
        "observation": "current", "model": {"identity": package},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": format!("ix://{package}/calls"), "complete": true,
            "objects": [{"key": "c", "type": format!("ix://{package}/Call"),
                "fields": {"phase": {"integer": "0"}, "inflight": {"boolean": false},
                    "late": {"boolean": false}, "retried": {"boolean": false}}}]}],
    })
    .to_string()
    .into_bytes()
}

/// FR-632-AC-1: the no-late-reply claim proves with a zone certificate at
/// timeout 4 and refutes with exact delays at timeout 3.
#[trace("TC-1592", "FR-632-AC-1")]
#[test]
fn zone_items_prove_with_certificates_and_refute_with_exact_delays() {
    let claim = "temporal NoLateReply using tt over (c: M::Call) clock \"model-time\" on origin \
                 { always holds(not c.late) }\n";
    for (t, expected) in [(4, "proved"), (3, "refuted")] {
        let ws = Workspace::new(&format!("family-zone-{t}"));
        let run = analyze(&request_over(
            &ws,
            &rpc_unit(t, claim),
            &crate::zone_search::rpc_document(),
            &[rpc_snapshot()],
            json!({"items": [{"claim": "NoLateReply"}]}),
        ));
        let item = run.item(0);
        assert_eq!(label(&item), expected, "T = {t}");
        if expected == "proved" {
            assert_eq!(item["record"]["basis"]["type"], "zone-certified");
            assert_eq!(item["record"]["certification"], "certified");
        } else {
            assert_eq!(item["replay"]["reproduced"], true);
        }
    }
}

/// ADR-026 §12's `Ctl` with third WCET `c` and the claims after it.
fn ctl_body(c: u32, claims: &str) -> String {
    format!(
        "dimension Time;\nunit ms : Time = rational(1, 1);\n\
         taskset Ctl {{\n\
         task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n\
         task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n\
         task c periodic 12 ms deadline 12 ms wcet {c} ms priority 3 lo;\n\
         }}\n{claims}"
    )
}

/// FR-633-AC-1, AC-2: `Ctl` proves on EN-7 and a response bound one below
/// the fixpoint refutes with the miss evidence.
#[trace("TC-1593", "FR-633-AC-1")]
#[trace("TC-1593", "FR-633-AC-2")]
#[test]
fn schedulability_proves_on_en7_and_a_tight_response_refutes() {
    let claims = "schedulable CtlFp on Ctl under fixed_priority;\n\
                  response Twelve on Ctl.c <= 12 ms under fixed_priority;\n\
                  response Eleven on Ctl.c <= 11 ms under fixed_priority;\n";
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"filament.temporal.complete/v1\";\n{}",
        ctl_body(5, claims)
    );
    let ws = Workspace::new("family-schedulability");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &crate::counter::document(false),
        &[],
        json!({"models": [], "items": [{"claim": "CtlFp"}, {"claim": "Twelve"},
            {"claim": "Eleven"}]}),
    ));
    assert_eq!(results(&run, 3), ["proved", "proved", "refuted"]);
    let proved = run.item(0);
    assert_eq!(proved["record"]["basis"]["type"], "closed-form");
    assert_eq!(proved["record"]["basis"]["analysis"], "fixed-priority");
    assert_eq!(proved["record"]["backend"], "en-7");
    assert!(proved.get("axes").is_none());
    let miss = &run.item(2)["counterexample"]["counterexample"];
    assert_eq!(miss["type"], "schedulability-miss", "{miss}");
    assert_eq!(miss["miss"]["task"], "c", "{miss}");
    assert_eq!(miss["miss"]["response"]["numerator"], "12", "{miss}");
    assert_eq!(miss["miss"]["bound"]["numerator"], "11", "{miss}");
}

/// FR-633, FR-247: EDF and AMC each settle their holding and low-mode or
/// utilization miss cases through the native-analyze request wire.
#[trace("TC-1593", "FR-633-AC-3", "FR-247-AC-2", "FR-247-AC-3")]
#[test]
fn edf_and_amc_schedulability_items_settle_through_the_wire() {
    fn schedule_source(task_set: &str, claims: &str) -> String {
        format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile t = \"filament.temporal.complete/v1\";\n\
             dimension Time;\nunit ms : Time = rational(1, 1);\n\
             {task_set}\n{claims}"
        )
    }

    fn run_schedule(name: &str, source: &str, claim: &str) -> crate::analyze_cli_support::Run {
        let ws = Workspace::new(name);
        let path = request_over(
            &ws,
            source,
            &crate::counter::document(false),
            &[],
            json!({"models": [], "items": [{"claim": claim}]}),
        );
        analyze(&path)
    }

    let edf_task_set = |third| {
        format!(
            "taskset Ctl {{\n\
             task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n\
             task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n\
             task c periodic 12 ms deadline 12 ms wcet {third} ms priority 3 lo;\
             }}"
        )
    };
    let edf_claim = "schedulable CtlEdf on Ctl under edf;";
    let edf_holds = run_schedule(
        "schedulability-edf-holds",
        &schedule_source(&edf_task_set(5), edf_claim),
        "CtlEdf",
    );
    let edf_item = edf_holds.item(0);
    assert_eq!(label(&edf_item), "proved", "{}", edf_holds.document());
    assert_eq!(edf_item["record"]["basis"]["analysis"], "edf");
    assert_eq!(edf_item["record"]["certification"], "certified");

    let edf_refuted = run_schedule(
        "schedulability-edf-refuted",
        &schedule_source(&edf_task_set(6), edf_claim),
        "CtlEdf",
    );
    let edf_miss = &edf_refuted.item(0)["counterexample"]["counterexample"];
    assert_eq!(label(&edf_refuted.item(0)), "refuted", "{edf_miss}");
    assert_eq!(edf_miss["type"], "schedulability-miss");
    assert_eq!(edf_miss["miss"]["utilization"]["numerator"], "13");
    assert_eq!(edf_miss["miss"]["utilization"]["denominator"], "12");

    let amc_task_set = |low, high| {
        format!(
            "taskset Mc {{\n\
             task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n\
             task b periodic 6 ms deadline 6 ms wcet {low} ms priority 2 hi wcet_hi {high} ms;\
             }}"
        )
    };
    let amc_claim = "schedulable McAmc on Mc under amc;";
    let amc_holds = run_schedule(
        "schedulability-amc-holds",
        &schedule_source(&amc_task_set(1, 3), amc_claim),
        "McAmc",
    );
    let amc_item = amc_holds.item(0);
    assert_eq!(label(&amc_item), "proved", "{}", amc_holds.document());
    assert_eq!(amc_item["record"]["basis"]["analysis"], "amc-rtb");
    assert_eq!(amc_item["record"]["certification"], "certified");

    let amc_refuted = run_schedule(
        "schedulability-amc-refuted",
        &schedule_source(&amc_task_set(5, 5), amc_claim),
        "McAmc",
    );
    let amc_miss = &amc_refuted.item(0)["counterexample"]["counterexample"];
    assert_eq!(label(&amc_refuted.item(0)), "refuted", "{amc_miss}");
    assert_eq!(amc_miss["type"], "schedulability-miss");
    assert_eq!(amc_miss["miss"]["task"], "b");
    assert_eq!(amc_miss["miss"]["mode"], "lo");
}

/// FR-633 budgets: EDF's default QPA demand-point cap settles
/// incomplete through the native-analyze request path.
#[trace("TC-1593", "FR-633")]
#[test]
fn edf_qpa_default_demand_point_cap_settles_incomplete_through_analyze() {
    let period = 1_048_580_u64;
    let long_period = period * period + 1;
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"filament.temporal.complete/v1\";\n\
         dimension Time;\nunit ms : Time = rational(1, 1);\n\
         taskset Many {{\n\
         task a periodic {period} ms deadline {period} ms wcet {} ms priority 1 lo;\n\
         task b periodic {long_period} ms deadline {long_period} ms wcet {period} ms priority 2 lo;\n\
         }}\n\
         schedulable ManyEdf on Many under edf;\n",
        period - 1
    );
    let ws = Workspace::new("family-edf-demand-point-limit");
    let request = request_over(
        &ws,
        &unit,
        &crate::counter::document(false),
        &[],
        json!({"models": [], "items": [{"claim": "ManyEdf"}]}),
    );
    let run = analyze(&request);
    let item = run.item(0);
    assert_eq!(run.exit, 22, "{}", run.document());
    assert_eq!(label(&item), "incomplete", "{item}");
    let cause = &item["record"]["cause"];
    assert_eq!(cause["kind"], "limit-reached", "{item}");
    assert_eq!(cause["limit"], "max_demand_points", "{item}");
    assert_eq!(cause["value"], 1_048_576, "{item}");
    assert_eq!(cause["setting"], "schedule.max_demand_points", "{item}");
}

/// FR-635-AC-1, AC-2: an item naming a process provider handed to
/// `analyze_with_providers` settles on it, the registry lists it, and another
/// item is unchanged by it. The provider stands where the driver passes a
/// started plugin session (FR-635 "One rule for every provider-routing
/// entry"); starting the plugin process is the driver's transport.
#[trace("TC-1595", "FR-635-AC-1")]
#[trace("TC-1595", "FR-635-AC-2")]
#[test]
fn an_item_naming_a_registered_provider_settles_on_it() {
    use fsl_analyze::provider::ProviderVerdict;
    let provider = PROCESS_PROVIDER.to_owned();
    let body = "function Even using v(n: Integer): Boolean pure { n * 2 - n = n }\n".to_owned()
        + &crate::analyze_cli_support::bounded();
    let ws = Workspace::new("family-provider");
    let request = counter_request(
        &ws,
        &body,
        json!([{"claim": "Even", "kind": "value-validity", "backend": provider},
            {"claim": "Bounded"}]),
    );
    let run = crate::analyze_cli_support::analyze_with_providers(
        &request,
        vec![process_provider(PROCESS_PROVIDER, ProviderVerdict::Proved)],
    );
    let item = run.item(0);
    assert_eq!(label(&item), "proved");
    assert_eq!(item["record"]["backend"], provider.as_str());
    // FR-635: the test provider is a process plugin, so its proof is trusted.
    assert_eq!(item["record"]["certification"], "trusted");
    let registry: Vec<Value> = run.document()["registry"]
        .as_array()
        .expect("registry")
        .iter()
        .map(|entry| entry["backend"].clone())
        .collect();
    assert!(registry.contains(&json!(provider)));
    assert!(registry.contains(&json!("en-1")) && registry.contains(&json!("en-7")));
    // FR-635 "Outputs": each entry carries its backend's FSpec FR-331 tool.
    assert!(run.document()["registry"]
        .as_array()
        .expect("registry")
        .iter()
        .all(|entry| entry["tool"]["name"]
            .as_str()
            .is_some_and(|name| !name.is_empty())));
    assert_eq!(label(&run.item(1)), "proved");
}

/// A process provider double: a `Process`-origin descriptor advertising
/// (`value-validity`, `unbounded`) over `Integer`, whose handle always
/// answers `verdict`. It stands where the driver passes a started plugin
/// session; the real plugin process is the driver's test (QSL-2283).
#[derive(Debug)]
pub(crate) struct AnsweringProvider(pub(crate) fsl_analyze::provider::ProviderVerdict);

impl fsl_analyze::provider::ExternalProvider for AnsweringProvider {
    fn settle(
        &self,
        request: &fsl_analyze::provider::ProviderRequest<'_>,
        _cancel: &filament_exact::Cancel,
    ) -> Vec<fsl_analyze::provider::ProviderVerdict> {
        assert_eq!(request.backend().as_str(), PROCESS_PROVIDER);
        for (item, target) in request.items().iter().zip(request.targets()) {
            assert_eq!(item.occurrence(), target.claim());
        }
        vec![self.0.clone(); request.items().len()]
    }
}

pub(crate) const PROCESS_PROVIDER: &str = "fsl.test-process-provider";

pub(crate) fn process_descriptor(backend: &str) -> fsl_route::BackendDescriptor {
    use fsl_semantics::check::Capability;
    fsl_route::descriptor_from_manifest(
        fsl_route::Candidate::new(fsl_foundation::BackendId::new(backend)),
        fsl_route::ProviderOrigin::Process,
        vec![(
            Some(Capability::ValueValidity.to_wire()),
            Some(fsl_route::Mode::Unbounded.to_wire()),
        )],
        fsl_route::ManifestMembers {
            domains: Some(&[fsl_foundation::bound::DomainKind::Integer.to_wire()]),
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    )
    .expect("the descriptor advertises the external value arm")
}

pub(crate) fn process_provider(
    backend: &str,
    verdict: fsl_analyze::provider::ProviderVerdict,
) -> fsl_analyze::analyze::Engine {
    fsl_analyze::analyze::Engine::external(
        process_descriptor(backend),
        fsl_analyze::analyze::BackendTool::new("test-provider", "1").expect("a tool"),
        Box::new(AnsweringProvider(verdict)),
    )
    .expect("the descriptor is unbounded")
}

/// FR-635 "One rule for every provider-routing entry": a process provider
/// handed to `analyze_with_providers` registers beside `Engines::fsl()`, an
/// item naming it settles on it with certification `trusted`, the registry
/// lists it, and an item that does not negotiate to it is unchanged.
#[trace("TC-1595", "FR-635-AC-2")]
#[test]
fn analyze_with_providers_registers_a_process_provider() {
    use fsl_analyze::provider::ProviderVerdict;
    let body = "function Even using v(n: Integer): Boolean pure { n * 2 - n = n }\n".to_owned()
        + &crate::analyze_cli_support::bounded();
    let ws = Workspace::new("family-process-provider");
    let request = counter_request(
        &ws,
        &body,
        json!([{"claim": "Even", "kind": "value-validity", "backend": PROCESS_PROVIDER},
            {"claim": "Bounded"}]),
    );
    let run = crate::analyze_cli_support::analyze_with_providers(
        &request,
        vec![process_provider(PROCESS_PROVIDER, ProviderVerdict::Proved)],
    );
    let item = run.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["record"]["certification"], "trusted", "{item}");
    // FR-635 "Outputs": the sorted `{backend, tool}` of every registered
    // backend, the provider's with the tool it was registered with.
    let entries = run.document()["registry"]
        .as_array()
        .expect("registry")
        .clone();
    let registry: Vec<Value> = entries
        .iter()
        .map(|entry| entry["backend"].clone())
        .collect();
    assert!(registry.contains(&json!(PROCESS_PROVIDER)), "{registry:?}");
    assert!(registry.contains(&json!("en-1")), "{registry:?}");
    let mut sorted: Vec<&str> = registry.iter().filter_map(Value::as_str).collect();
    sorted.sort_unstable();
    let listed: Vec<&str> = registry.iter().filter_map(Value::as_str).collect();
    assert_eq!(listed, sorted, "the registry is sorted by backend");
    let provider = entries
        .iter()
        .find(|entry| entry["backend"] == PROCESS_PROVIDER)
        .expect("the provider is listed");
    assert_eq!(
        provider["tool"],
        json!({"name": "test-provider", "version": "1"}),
        "{provider}"
    );
    let en1 = entries
        .iter()
        .find(|entry| entry["backend"] == "en-1")
        .expect("en-1 is listed");
    assert_eq!(en1["tool"]["name"], "fsl-analyze", "{en1}");
    let without = analyze(&request);
    assert_eq!(run.item(1), without.item(1));
}

/// A provider that records the FSpec FR-331 `request` member `analyze`
/// built for it, as the typed request serializes it, with the negotiated
/// manifest entry's backend, and answers `proved`.
#[derive(Debug, Default)]
struct RecordingProvider(std::sync::Arc<std::sync::Mutex<Vec<(Value, String)>>>);

impl fsl_analyze::provider::ExternalProvider for RecordingProvider {
    fn settle(
        &self,
        request: &fsl_analyze::provider::ProviderRequest<'_>,
        _cancel: &filament_exact::Cancel,
    ) -> Vec<fsl_analyze::provider::ProviderVerdict> {
        // The package member is the v2 document's own bytes.
        let document: Value =
            serde_json::from_slice(request.package().bytes()).expect("the package is JSON");
        assert!(document.is_object());
        // The manifest is the registry snapshot, and the negotiated entry is
        // one of its descriptors.
        let snapshot: Vec<&str> = request
            .manifest()
            .iter()
            .map(|descriptor| descriptor.id().as_str())
            .collect();
        assert!(
            snapshot.contains(&request.backend().as_str()),
            "{snapshot:?}"
        );
        assert!(snapshot.contains(&"en-1"), "{snapshot:?}");
        let wire = serde_json::to_value(request).expect("the request serializes");
        self.0
            .lock()
            .expect("the record")
            .push((wire, request.descriptor().id().to_string()));
        vec![fsl_analyze::provider::ProviderVerdict::Proved; request.items().len()]
    }
}

/// FSpec FR-331's `Request` definition with the checked-package schema it refers to.
fn fr331_request_schema() -> jsonschema::JSONSchema {
    let schema = crate::published_fspec::json("proposals/backend-provider-v1/schema.json");
    let package = crate::published_fspec::json("proposals/checked-package-v2/schema.json");
    let request = json!({
        "$schema": schema["$schema"],
        "$defs": schema["$defs"],
        "$ref": "#/$defs/Request",
    });
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .with_document("urn:filament:checked-package:v2".to_owned(), package)
        .compile(&request)
        .expect("the schema compiles")
}

/// FR-635 "How a process provider runs an item", FSpec FR-331: for the
/// `Boolean` value clause `Even`, which has no requirement record, the
/// request `analyze` builds carries the package's emitted bytes, the
/// caller's `deadline_ms`, and one item at the analyze request index whose
/// claim is the clause occurrence, whose extent is unbounded over the
/// parameter's integer domain, and whose backend is the negotiated
/// provider, its one candidate; the request validates against FSpec's
/// `Request` schema, and the negotiated manifest entry is the provider's.
#[trace("TC-1595", "FR-635-AC-1")]
#[test]
fn analyze_builds_the_fr331_request_for_a_boolean_value_clause() {
    let body = "function Even using v(n: Integer): Boolean pure { n * 2 - n = n }\n".to_owned()
        + &crate::analyze_cli_support::bounded();
    let ws = Workspace::new("family-provider-request");
    let request = counter_request(
        &ws,
        &body,
        json!([{"claim": "Bounded"},
            {"claim": "Even", "kind": "value-validity", "backend": PROCESS_PROVIDER}]),
    );
    let recording = RecordingProvider::default();
    let sent = std::sync::Arc::clone(&recording.0);
    let limits = fsl_analyze::provider::ProviderLimits {
        deadline_ms: 250,
        ..fsl_analyze::provider::ProviderLimits::default()
    };
    let engine = fsl_analyze::analyze::Engine::external(
        process_descriptor(PROCESS_PROVIDER),
        fsl_analyze::analyze::BackendTool::new("test-provider", "1").expect("a tool"),
        Box::new(recording),
    )
    .expect("the descriptor is unbounded");
    let options = fsl_analyze::provider::ProviderOptions {
        provider_limits: limits,
    };
    let run =
        crate::analyze_cli_support::analyze_with_providers_under(&request, vec![engine], &options);
    assert_eq!(label(&run.item(1)), "proved", "{}", run.document());
    let sent = sent.lock().expect("the record").clone();
    let [(sent, manifest_entry)] = sent.as_slice() else {
        panic!("one request: {sent:?}");
    };
    assert_eq!(manifest_entry, PROCESS_PROVIDER);
    let schema = fr331_request_schema();
    if let Err(errors) = schema.validate(sent) {
        let errors: Vec<_> = errors.map(|error| error.to_string()).collect();
        panic!("the request violates FR-331: {errors:?}\n{sent}");
    }
    assert_eq!(sent["deadline_ms"], 250);
    let items = sent["items"].as_array().expect("items");
    assert_eq!(items.len(), 1, "{sent}");
    let item = &items[0];
    // The item's position in this one-item request, not its analyze index
    // (1), which the verdict is joined back to.
    assert_eq!(item["request_index"], 0);
    assert_eq!(item["kind"], "value-validity");
    assert_eq!(item["backend"], PROCESS_PROVIDER);
    assert_eq!(
        item["candidates"],
        json!({"type": "set", "backends": [PROCESS_PROVIDER]})
    );
    assert_eq!(item["extent"]["classification"], "unbounded", "{item}");
    let domains = item["extent"]["domains"].as_array().expect("domains");
    assert_eq!(domains.len(), 1, "{item}");
    assert_eq!(domains[0]["kind"], "integer", "{item}");
    // The claim is the clause occurrence: `Even`'s body at its declaration.
    assert!(
        matches!(
            item["claim"]["occurrence"]["role"].as_str(),
            Some("claim" | "declaration")
        ),
        "{item}"
    );
    // A request whose item has lost its domain is not an FR-331 request,
    // so the validation above is not vacuous.
    let mut defective = sent.clone();
    defective["items"][0]["extent"]["domains"] = json!([]);
    assert!(!schema.is_valid(&defective));
}

/// A process provider repeating a linked `BackendId` refuses the request
/// `invalid-request` naming `providers` before any item runs.
#[test]
fn analyze_with_providers_refuses_a_repeated_backend() {
    use fsl_analyze::provider::ProviderVerdict;
    let ws = Workspace::new("family-provider-repeat");
    let request = counter_request(
        &ws,
        &crate::analyze_cli_support::bounded(),
        json!([{"claim": "Bounded"}]),
    );
    let run = crate::analyze_cli_support::analyze_with_providers(
        &request,
        vec![process_provider("en-1", ProviderVerdict::Proved)],
    );
    assert_eq!(run.exit, 20);
    let document = run.document();
    assert_eq!(document["items"], json!([]), "{document}");
    assert!(document.to_string().contains("providers"), "{document}");
}

/// The `Rpc` request at timeout `t` with `claim` and the request members
/// `extra`.
pub(crate) fn rpc_request(ws: &Workspace, t: u32, claim: &str, extra: Value) -> std::path::PathBuf {
    request_over(
        ws,
        &rpc_unit(t, claim),
        &crate::zone_search::rpc_document(),
        &[rpc_snapshot()],
        extra,
    )
}

/// The `Ctl` request (third WCET 5) with `claims` and `items`.
pub(crate) fn ctl_request(ws: &Workspace, claims: &str, items: Value) -> std::path::PathBuf {
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"filament.temporal.complete/v1\";\n{}",
        ctl_body(5, claims)
    );
    request_over(
        ws,
        &unit,
        &crate::counter::document(false),
        &[],
        json!({"models": [], "items": items}),
    )
}

/// FR-640-AC-12 (TC-2243), analyze and replay half: an event-position hyper
/// clause whose two traces range over two models carries one clock row per
/// trace, so it compiles, analyzes to a verdict and, when refuted, replays
/// under the two rows' configurations; it is never `not-yet-implemented`.
#[trace("TC-2243", "FR-640-AC-12")]
#[test]
fn a_hyper_clause_over_two_models_analyzes_and_replays_per_trace() {
    let declaration = "hyper Saves using e over behaviours (v: V::Vault, w: N::Vault) \
                       forall trace a of V forall trace b of N \
                       { always[0,3] holds(v.l @ a = w.l @ b) }";
    let (unit, model, other, snapshot) =
        crate::hyper_world::two_model_request_files((1, 0), declaration);
    let unit = unit.replace(
        "profile p =",
        "profile e = \"filament.temporal.event-position.false-extension/v1\";\nprofile p =",
    );
    let ws = Workspace::new("family-hyper-two-models");
    ws.write("other.json", &other);
    let mut models = request_members(None)["models"].clone();
    models.as_array_mut().expect("models").push(
        json!({"format": "semantic-ir/2.0.0", "source": {"file": "other.json",
            "authority": "test", "identity": "vault-other-model", "document": "VaultOther",
            "formal_revision": 1}}),
    );
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &[snapshot],
        json!({"models": models, "items": [{"claim": "Saves"}]}),
    ));
    let item = run.item(0);
    assert_ne!(
        item["record"]["diagnostic"]["cause"], "not-yet-implemented",
        "{item}"
    );
    let verdict = label(&item);
    // FR-173-AC-6's HP-2 `l` body: the two vaults' `l` differ, so it is refuted.
    assert_eq!(verdict, "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// FR-640-AC-12 (TC-2243): the hyper model checker refutes a two-model
/// event-position hyper clause over the two-alias Vault world, and its
/// counterexample, one trace per trace variable, replays through
/// `fsl_replay` and settles `refuted`.
#[trace("TC-2243", "FR-640-AC-12")]
#[test]
fn a_two_model_hyper_counterexample_replays_through_the_engine() {
    use filament_exact::Cancel;
    use fsl_eval::hyper::{CheckedHyperLetters, CheckedStepMatch, ParameterBinding};
    use fsl_eval::model_subject::ModelInstance;
    use fsl_replay::{
        replay_hyper_trace_tuple, settle_hyper_check, HyperOutcome, HyperReplay, ModelCheckLimits,
        TerminalValue,
    };

    let declaration = "hyper Saves using ep over behaviours (v: V::Vault, w: N::Vault) \
         forall trace s of V forall trace n of N \
         { always[0,3] holds(v.l @ s <= w.l @ n) }";
    let world = crate::hyper_world::World::with_second_vaults(
        crate::hyper_world::Vault::Secure,
        &[("saving", (0, 1))],
        &[("normal", (0, 0))],
        declaration,
    );
    let limits = ModelCheckLimits::default();
    let outcome = world
        .run_selecting(fsl_route::reductions::Reductions::default())
        .expect("the two-model hyper item runs");
    let HyperOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("Saves is refuted by the initial model values: {outcome:?}");
    };

    // One system per trace variable's own model.
    let seams = world.seams();
    let systems = seams.per_trace();
    let letters = CheckedHyperLetters::new(&world.package, &world.clause, systems.clone())
        .expect("the clause letters admit the subject");
    let matcher = CheckedStepMatch::new(&world.package, &world.clause);
    let instance = [
        ParameterBinding {
            parameter: "v".to_owned(),
            instance: ModelInstance {
                population: crate::hyper_world::key("vaults"),
                key: "saving".to_owned(),
            },
        },
        ParameterBinding {
            parameter: "w".to_owned(),
            instance: ModelInstance {
                population: crate::hyper_world::other_key("vaults"),
                key: "normal".to_owned(),
            },
        },
    ];
    let cancel = Cancel::new();
    let replay = HyperReplay {
        clause: &world.clause,
        systems,
        instance: &instance,
        letters: &letters,
        matcher: &matcher,
        meter: crate::counter::UNLIMITED,
        max_witness_set: limits.max_witness_set,
        max_states: u64::try_from(limits.limits.max_states).unwrap_or(u64::MAX),
        cancel: &cancel,
    };
    assert!(matches!(
        replay_hyper_trace_tuple(&replay, counterexample),
        Ok(fsl_replay::ModelTraceSettlement::Reproduced { .. })
    ));
    let record = settle_hyper_check(&outcome, &limits, &[], None, &cancel, |counterexample| {
        replay_hyper_trace_tuple(&replay, counterexample)
    })
    .expect("the replayed counterexample settles");
    assert_eq!(record.value, TerminalValue::Refuted);
}

/// [`refinement_request`] with every item's `abstract_subject` removed, so
/// the item takes FR-630's default abstract initial states, over the concrete
/// snapshots of counter `a` at each of `values`.
fn refinement_request_without_abstract(
    ws: &Workspace,
    body: &str,
    items: Value,
    values: &[i64],
) -> std::path::PathBuf {
    let path = refinement_request(ws, body, items);
    let mut document: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("json");
    let request = &mut document["request"];
    for item in request["items"].as_array_mut().expect("items") {
        item["temporal"]
            .as_object_mut()
            .expect("temporal")
            .remove("abstract_subject");
    }
    let mut starts = Vec::new();
    for value in values {
        let identity = format!("start-{value}");
        let snapshot = String::from_utf8(crate::analyze_cli_support::counter_snapshot(&[(
            "a", *value,
        )]))
        .expect("the fixture is UTF-8")
        .replace(
            "\"identity\":\"start\"",
            &format!("\"identity\":\"{identity}\""),
        );
        ws.write(&format!("{identity}.json"), snapshot.as_bytes());
        request["documents"]
            .as_array_mut()
            .expect("documents")
            .push(json!({"file": format!("{identity}.json")}));
        starts.push(json!({"authority": "test", "identity": identity}));
    }
    request["subject"]["initial_states"] = json!(starts);
    request["subject"]["universes"] = json!(starts);
    ws.write("analyze.json", document.to_string().as_bytes())
}

/// The refinement item for `claim` with no `abstract_subject`, under
/// `max_candidates` when given.
fn default_initial_item(claim: &str, max_candidates: Option<u64>) -> Value {
    let mut item = json!({"claim": claim, "kind": "refinement",
        "temporal": {"item_kind": "refinement"}});
    if let Some(cap) = max_candidates {
        item["temporal"]["limits"] = json!({"max_candidates": cap});
    }
    item
}

/// FR-630-AC-5 (completions and cap): with no `abstract_subject`, a field the
/// mapping does not fix ranges over its type (`Int[0, 3]`, four completions)
/// and the enumeration is charged to one `max_candidates` total across every
/// concrete initial state.
#[trace("TC-2964", "FR-630-AC-5")]
#[test]
fn default_abstract_initial_states_are_every_admissible_completion() {
    let body = "refinement Hidden using t0 abstract Spec concrete M {\n\
        population Spec::counters from M::counters;\n\
        object Spec::Counter from M::Counter { }\n\
        step M::Counter::inc -> stutter;\n}\n";

    let ws = Workspace::new("family-refinement-default-initial");
    let run = analyze(&refinement_request_without_abstract(
        &ws,
        body,
        json!([
            default_initial_item("Hidden", None),
            default_initial_item("Hidden", Some(3)),
        ]),
        &[0],
    ));
    assert_eq!(results(&run, 2), ["proved", "incomplete"]);
    let stopped = run.item(1);
    assert_eq!(stopped["category"], "incomplete", "{stopped}");
    assert_eq!(
        stopped["record"]["cause"]["kind"], "limit-reached",
        "{stopped}"
    );
    assert_eq!(
        stopped["record"]["cause"]["limit"], "max_candidates",
        "{stopped}"
    );
    assert_eq!(
        stopped["record"]["cause"]["setting"], "expansion.max_candidates",
        "{stopped}"
    );

    let ws = Workspace::new("family-refinement-default-initial-two");
    let run = analyze(&refinement_request_without_abstract(
        &ws,
        body,
        json!([
            default_initial_item("Hidden", Some(4)),
            default_initial_item("Hidden", Some(8)),
        ]),
        &[0, 1],
    ));
    assert_eq!(results(&run, 2), ["incomplete", "proved"]);
    assert_eq!(run.item(0)["record"]["cause"]["limit"], "max_candidates");
}

/// FR-630-AC-5 (empty completion set): an image outside the abstract unit
/// bound is `InitialNotAbstract` at position 0, never a refusal.
///
/// The refinement row `value = self.value + 3` is refused `ill_typed` by the
/// row rule that an expression conforms to the type it defines and is never
/// range-checked into it (ADR-016 SC-2), so the item cannot be built; the spec
/// gives no mapping that type-checks and has an image outside `Int[0, 3]`.
#[trace("TC-2964", "FR-630-AC-5")]
#[test]
#[ignore = "QSL-2407: spec gap - FR-630-AC-5 Shifted row is ill_typed under ADR-016 SC-2"]
fn an_image_outside_the_unit_bound_is_initial_not_abstract() {
    let body = refinement_stepping("Shifted", "self.value + 3", "stutter", "");
    let ws = Workspace::new("family-refinement-default-initial-empty");
    let run = analyze(&refinement_request_without_abstract(
        &ws,
        &body,
        json!([default_initial_item("Shifted", None)]),
        &[1],
    ));
    let refuted = run.item(0);
    assert_eq!(label(&refuted), "refuted", "{refuted}");
    assert_eq!(
        refuted["counterexample"]["refinement_failure"],
        json!({"kind": "InitialNotAbstract", "initial": "0"}),
        "{refuted}"
    );
    assert_eq!(refuted["replay"]["reproduced"], true, "{refuted}");
}

/// FR-630-AC-5 (TC-2964): the refinement row `value = self.value + 3` types
/// `Int[3, 6]`, which does not conform to the abstract field type `Int[0, 3]`
/// (ADR-016 SC-2), so S3 refuses it `ill_typed`/`type-mismatch` and no item
/// is analyzed.
#[trace("TC-2964", "FR-630-AC-5")]
#[test]
#[ignore = "QSL-2407: the Shifted row refused at S3 type-mismatch as FR-630-AC-5 decides not yet asserted by the suite"]
fn a_mapping_that_does_not_conform_to_the_abstract_field_is_refused() {
    let body = refinement_stepping("Shifted", "self.value + 3", "stutter", "");
    let ws = Workspace::new("family-refinement-shifted-refused");
    let run = analyze(&refinement_request_without_abstract(
        &ws,
        &body,
        json!([default_initial_item("Shifted", None)]),
        &[1],
    ));
    let document = run.document();
    let diagnostic = &document["diagnostics"][0];
    assert_eq!(diagnostic["code"], "ill_typed", "{document}");
    assert_eq!(diagnostic["cause"], "type-mismatch", "{document}");
    assert_eq!(document["items"], json!([]), "{document}");
}

/// Add the field `elapsed: Quantity<ms>` to the Counter document at `path`,
/// under the unit bound `max 3` when `bounded`.
fn add_elapsed(ws: &Workspace, path: &str, package: &str, bounded: bool) {
    let file = ws.path(path);
    let mut document: Value =
        serde_json::from_slice(&std::fs::read(&file).expect("the model document")).expect("JSON");
    let counter = format!("ix://{package}/Counter");
    let elapsed = format!("{counter}/elapsed");
    let origin = json!({"generated": {"generatorIdentity": elapsed, "generatorVersion": "1.0.0",
        "inputIdentities": [elapsed]}});
    for node in document["types"].as_array_mut().expect("types") {
        if node["identity"] != json!(counter) {
            continue;
        }
        node["fields"].as_array_mut().expect("fields").push(json!({
            "identity": elapsed, "name": "elapsed", "typeRef": "ix://filament/unit/ms",
            "presence": "required", "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": origin,
        }));
        if bounded {
            node["constraints"]
                .as_array_mut()
                .expect("constraints")
                .push(json!({
                    "identity": format!("{counter}/constraints/elapsedMax"), "keyword": "max",
                    "operands": {"value": "3"}, "appliesTo": "ix://filament/unit/ms",
                    "diagnosticCode": "bound.elapsed", "origin": origin,
                }));
        }
    }
    std::fs::write(&file, document.to_string()).expect("write");
}

/// Set `elapsed` to `ms` milliseconds in each Counter object of the snapshot
/// at `path`.
fn set_elapsed(ws: &Workspace, path: &str, ms: i64) {
    let file = ws.path(path);
    let mut snapshot: Value =
        serde_json::from_slice(&std::fs::read(&file).expect("the snapshot")).expect("JSON");
    for population in snapshot["populations"].as_array_mut().expect("populations") {
        for object in population["objects"].as_array_mut().expect("objects") {
            object["fields"]["elapsed"] = json!({"rational": {"numerator": ms.to_string(),
                "denominator": "1"}, "unit": "ms"});
        }
    }
    std::fs::write(&file, snapshot.to_string()).expect("write");
}

/// FR-630-AC-6 (TC-2971): a well-typed mapping whose image violates an
/// abstract unit bound has no admissible completion: `Late` over `elapsed`
/// 2 ms maps to 4 ms above the bound `max 3` and is `InitialNotAbstract` at
/// position 0, a replayed one-state counterexample; over `elapsed` 1 ms (3 ms,
/// the bound itself) it is proved.
#[trace("TC-2971", "FR-630-AC-6")]
#[test]
#[ignore = "QSL-2407 completed; current blocker has no assigned owner: the analyze item for an image outside an abstract unit bound carries no terminal record (null) instead of settling InitialNotAbstract"]
fn an_image_outside_an_abstract_unit_bound_is_initial_not_abstract() {
    let body = "dimension Time;\nunit s : Time = rational(1, 1);\nunit ms : Time = rational(1, 1000) * s;\n\
         refinement Late using t0 abstract Spec concrete M {\n\
         population Spec::counters from M::counters;\n\
         object Spec::Counter from M::Counter { value = self.value; elapsed = self.elapsed + 2 ms; }\n\
         step M::Counter::inc -> stutter;\n}\n";
    for (elapsed, expected) in [(2, "refuted"), (1, "proved")] {
        let ws = Workspace::new(&format!("family-refinement-unit-bound-{elapsed}"));
        let request = refinement_request_without_abstract(
            &ws,
            body,
            json!([default_initial_item("Late", None)]),
            &[0],
        );
        add_elapsed(&ws, "model.json", "test/counter", false);
        add_elapsed(&ws, "spec.json", SPEC, true);
        for snapshot in [
            "start.json",
            "start-0.json",
            "spec-start.json",
            "spec-start-one.json",
        ] {
            if ws.path(snapshot).exists() {
                set_elapsed(&ws, snapshot, elapsed);
            }
        }
        let run = analyze(&request);
        let item = run.item(0);
        assert_eq!(label(&item), expected, "{}", run.document());
        if expected == "refuted" {
            assert_eq!(
                item["counterexample"]["refinement_failure"],
                json!({"kind": "InitialNotAbstract", "initial": "0"}),
                "{item}"
            );
            assert_eq!(item["replay"]["reproduced"], true, "{item}");
        }
    }
}
