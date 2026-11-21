// SPDX-License-Identifier: AGPL-3.0-or-later
//! Request-surface stubs for the plan outcome document, the plan search
//! limits, plan refusals and the reference planning examples (FR-1010 to
//! FR-1014), and for the TLA+ and Lean export outcome, mappers and golden
//! corpus (FR-1015 to FR-1018). Each feeds source and a `native-run/1`
//! request through the command adapter (`fsl_command::command::run`) and
//! asserts on the output document, because the engines do not exist yet.

use crate::support::config_version_spine::{spine, Case};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const MODEL_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.model.complete/v1\";\n\
     model Config = \"example/config-version\";\n";

// The domain document supplies the operation frame; the authored spine
// postcondition constrains that frame's possible post-states.
const PLAN_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.temporal.infinite-trace/v1\";\n\
     profile g = \"filament.state.graph/v1\";\n\
     model Config = \"example/config-version\";\n\
     post VersionAdvances using g on Config::ConfigVersion::attemptUpdate {\n\
       (result implies convert<Integer>(self.versionNumber) = pre(self.versionNumber) + 1)\n\
       and (not result implies self.versionNumber = pre(self.versionNumber)) }\n";

/// FR-447's `Braking` without `t`, with `SlowEnough` over it.
const BRAKING: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.temporal.infinite-trace/v1\";\n\
     hybrid Braking using p {\n\
       mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
         flow { v' = -a; a' = 0; }\n\
         transition to Stopped when { v = 0 } reset { v = 0 };\n\
       mode Stopped invariant { v = 0 }\n\
         flow { v' = 0; a' = 0; }\n\
     }\n\
     temporal SlowEnough using p over (h: Braking) clock \"model-time\" \
     on origin { always holds(h.v < 30) }\n";

pub(crate) fn source(file: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:w21:{file}"),
        "document": "W21", "formal_revision": 1})
}

/// Writes the spine files, `unit` as `unit.native` and a `native-run/1`
/// request whose `request` carries `member`, runs it through the command
/// adapter, and returns its exit code and document.
fn run(unit: &str, with_model: bool, member: (&str, Value)) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let models = if with_model {
        json!([{"format": "semantic-ir/2.0.0", "source": source("spine-model.semantic-ir.json")}])
    } else {
        json!([])
    };
    let mut request = json!({"models": models, "program": {"source": source("unit.native")}});
    request[member.0] = member.1;
    write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": request}),
    )
}

fn write_request(directory: &Path, request: &Value) -> (u8, Value) {
    if request["request"]["synthesis"]["initial"].as_str() == Some("spine-snapshot.json") {
        std::fs::write(
            directory.join("spine-model.semantic-ir.json"),
            crate::support::plan_model::document(),
        )
        .expect("the authored small planning model is written");
        let snapshot_path = directory.join("spine-snapshot.json");
        let bytes = std::fs::read(&snapshot_path).expect("the plan's spine snapshot exists");
        let mut snapshot: Value = serde_json::from_slice(&bytes).expect("a JSON snapshot");
        snapshot["anchor"] = json!({"kind": "initialization", "name": "start"});
        std::fs::write(snapshot_path, serde_json::to_vec(&snapshot).unwrap())
            .expect("the plan's initialization snapshot is written");
    }
    let path = directory.join("request.json");
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
        Ok(result) => (
            result.exit_code,
            serde_json::from_str(&result.value.to_string()).expect("a JSON document"),
        ),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

fn names(value: &Value, member: &str) {
    assert!(
        value.to_string().contains(member),
        "names {member}: {value}"
    );
}

/// A plan synthesis over the spine model with `lengths`, `goal` and an
/// optional objective, plus the two temporal clauses a policy may name.
fn plan_unit(lengths: &str, objective: &str, goal: &str) -> String {
    format!(
        "{PLAN_HEADER}type Lengths = {lengths};\n\
         temporal SomeTime using v over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ eventually holds(c.versionNumber = 3) }}\n\
         synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
         {objective} satisfies {{ {goal} }};\n"
    )
}

fn plan(unit: &str, synthesis: Value) -> (u8, Value) {
    let mut member = json!({"declaration": "Reach", "initial": "spine-snapshot.json"});
    for (key, value) in synthesis.as_object().into_iter().flatten() {
        member[key] = value.clone();
    }
    run(unit, true, ("synthesis", member))
}

// ---- QSL-1978: planner and optimizer ------------------------------------

/// FR-1010-AC-1 and FSpec FR-805-AC-1: an optimal plan writes the
/// `filament.plan-outcome/v1` document with all its members and exits 0.
#[test]
#[trace("TC-2860", "FR-1010-AC-1")]
#[trace("FSpec-TC-1955", "FSpec-FR-805-AC-1")]
#[trace("FSpec-TC-1950", "FSpec-FR-800-AC-1")]
fn an_optimal_plan_writes_the_plan_outcome_document() {
    for objective in ["", "minimize Reach.length"] {
        let (exit, value) = plan(
            &plan_unit("Int[0, 3]", objective, "Reach.length >= 1"),
            json!({}),
        );
        let outcome = &value["synthesis"];
        assert_eq!(outcome["format"], "filament.plan-outcome/v1", "{value}");
        assert_eq!(outcome["status"], "optimal", "{value}");
        for member in [
            "objective",
            "plan",
            "replay",
            "certificate",
            "certification",
            "explored",
            "limits",
        ] {
            assert!(outcome.get(member).is_some(), "{member}: {value}");
        }
        assert_eq!(outcome["replay"]["outcome"], "reproduced", "{value}");
        let steps = outcome["plan"]["steps"].as_array().expect("steps");
        assert_eq!(steps.len(), 1, "the shortest admissible plan: {value}");
        assert_eq!(steps[0]["index"], 0, "{value}");
        assert_eq!(
            outcome["objective"]["value"],
            json!({"type": "integer", "value": "1"}),
            "{value}"
        );
        assert_eq!(outcome["certificate"]["type"], "potential", "{value}");
        assert_eq!(outcome["certification"], "certified", "{value}");
        assert_eq!(
            outcome["plan"]["states"].as_array().map(Vec::len),
            Some(steps.len() + 1),
            "{value}"
        );
        assert_eq!(exit, 0, "{value}");
    }
}

/// FR-1010-AC-2 and FSpec FR-805-AC-2/AC-4, FR-806-AC-5: `infeasible`
/// writes a null plan and exits 10; an unknown member refuses, exit 20.
#[test]
#[trace("TC-2860", "FR-1010-AC-2")]
#[trace("FSpec-TC-1955", "FSpec-FR-805-AC-2")]
#[trace("FSpec-TC-1955", "FSpec-FR-805-AC-4")]
#[trace("FSpec-TC-1956", "FSpec-FR-806-AC-5")]
fn an_infeasible_plan_exits_ten_and_a_malformed_request_refuses() {
    let unit = plan_unit("Int[0, 0]", "", "Reach.length >= 1");
    let (exit, value) = plan(&unit, json!({}));
    assert_eq!(value["synthesis"]["status"], "infeasible", "{value}");
    assert!(value["synthesis"]["plan"].is_null(), "{value}");
    assert_eq!(exit, 10, "{value}");
    let (exit, value) = plan(&unit, json!({"strategy": "greedy"}));
    assert_eq!(exit, 20, "{value}");
    names(&value, "unknown-member");
    names(&value, "/synthesis/strategy");
}

/// FR-1011-AC-1, FSpec FR-801-AC-1/AC-6: the plan search stops at its own
/// named limits, distinct from the verification planner's.
#[test]
#[trace("TC-2861", "FR-1011-AC-1")]
#[trace("FSpec-TC-1951", "FSpec-FR-801-AC-1")]
#[trace("FSpec-TC-1951", "FSpec-FR-801-AC-6")]
fn the_plan_search_stops_at_its_named_limits() {
    let unit = plan_unit("Int[0, 3]", "", "Reach.length >= 3");
    for (member, setting) in [
        ("max_work", "plan_search.max_work"),
        ("max_states", "plan_search.max_states"),
    ] {
        let (exit, value) = plan(&unit, json!({"limits": {member: 1}}));
        let outcome = &value["synthesis"];
        assert_eq!(outcome["status"], "incomplete", "{value}");
        assert_eq!(outcome["cause"]["setting"], setting, "{value}");
        assert_eq!(exit, 22, "{value}");
    }
}

/// FSpec FR-801-AC-7: every plan search limit has its published default, a
/// request overrides one member only, and an invalid value refuses at its
/// pointer before any search.
#[test]
#[trace("FSpec-TC-2003", "FSpec-FR-801-AC-7")]
#[trace("TC-2861", "FR-1011-AC-3")]
fn plan_search_limits_have_published_defaults_and_refuse_invalid_values() {
    let unit = plan_unit("Int[0, 3]", "", "Reach.length >= 1");
    let defaults = json!({
        "plan_search.max_states": 10_000_000_u64,
        "plan_search.max_work": 100_000_000_u64,
        "plan_search.max_certificate_entries": 16_777_216_u64,
        "plan_search.max_check_work": 100_000_000_u64,
        "plan_search.max_state_key_bytes": 1_048_576_u64,
    });
    let (_, value) = plan(&unit, json!({}));
    assert_eq!(value["synthesis"]["limits"], defaults, "{value}");

    let (_, value) = plan(&unit, json!({"limits": {"max_states": 500}}));
    let mut overridden = defaults.clone();
    overridden["plan_search.max_states"] = json!(500);
    assert_eq!(value["synthesis"]["limits"], overridden, "{value}");

    for (limit, cause) in [
        (json!(0), "invalid-value"),
        (json!(-1), "invalid-value"),
        (json!(1.5), "invalid-value"),
        (
            serde_json::from_str::<Value>("18446744073709551616").unwrap(),
            "invalid-value",
        ),
        (json!("10"), "wrong-value-kind"),
    ] {
        let (exit, value) = plan(&unit, json!({"limits": {"max_work": limit}}));
        assert_eq!(exit, 20, "{value}");
        names(&value, cause);
        names(&value, "/synthesis/limits/max_work");
        assert!(value["synthesis"].get("status").is_none(), "{value}");
    }
    let (exit, value) = plan(&unit, json!({"limits": {"max_steps": 1}}));
    assert_eq!(exit, 20, "{value}");
    names(&value, "unknown-member");
    names(&value, "/synthesis/limits/max_steps");
}

/// FR-1011-AC-2, FSpec FR-801-AC-2: an exhausted evaluation meter names
/// its `accounting` setting.
#[test]
#[trace("TC-2861", "FR-1011-AC-2")]
#[trace("FSpec-TC-1951", "FSpec-FR-801-AC-2")]
fn an_exhausted_evaluation_meter_names_its_accounting_setting() {
    let (_, value) = plan(
        &plan_unit("Int[0, 3]", "", "Reach.length >= 3"),
        json!({"accounting": {"work_units": 1}}),
    );
    assert_eq!(value["synthesis"]["status"], "incomplete", "{value}");
    assert_eq!(
        value["synthesis"]["cause"]["setting"], "accounting.work_units",
        "{value}"
    );
}

/// FR-1012-AC-1, FSpec FR-803-AC-2: a non-safety policy is evaluated on
/// each completed plan, never refused.
#[test]
#[trace("TC-2862", "FR-1012-AC-1")]
#[trace("FSpec-TC-1953", "FSpec-FR-803-AC-2")]
fn a_non_safety_policy_is_evaluated_on_each_completed_plan() {
    let (exit, value) = plan(
        &plan_unit("Int[0, 3]", "", "Reach.length >= 1"),
        json!({"policy": ["SomeTime"]}),
    );
    assert_ne!(exit, 20, "{value}");
    let outcome = &value["synthesis"];
    if outcome["status"] == "optimal" {
        let visits = outcome["plan"]["states"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|state| state.to_string().contains("\"versionNumber\":3"));
        assert!(visits, "{value}");
    } else {
        assert_eq!(outcome["status"], "infeasible", "{value}");
    }
}

/// FR-1012-AC-2, FSpec FR-804-AC-4: equal requests give byte-identical
/// plan outcome documents.
#[test]
#[trace("TC-2862", "FR-1012-AC-2")]
#[trace("FSpec-TC-1954", "FSpec-FR-804-AC-4")]
fn equal_plan_requests_give_byte_identical_outcomes() {
    let unit = plan_unit("Int[0, 3]", "", "Reach.length >= 2");
    let (_, first) = plan(&unit, json!({}));
    let (_, second) = plan(&unit, json!({}));
    assert_eq!(first["synthesis"]["status"], "optimal", "{first}");
    assert_eq!(first.to_string(), second.to_string());
}

/// FR-1012-AC-3, FSpec FR-802-AC-1 and FR-805-AC-3: a decimal per-step
/// cost is exact, certified by `potential`, and never a JSON float.
#[test]
#[trace("TC-2862", "FR-1012-AC-3")]
#[trace("FSpec-TC-1952", "FSpec-FR-802-AC-1")]
#[trace("FSpec-TC-1955", "FSpec-FR-805-AC-3")]
fn a_decimal_step_cost_is_exact_and_certified() {
    let (_, value) = plan(
        &plan_unit(
            "Int[0, 3]",
            "minimize Reach.steps.sum(s => 0.10)",
            "Reach.length >= 3",
        ),
        json!({}),
    );
    let outcome = &value["synthesis"];
    assert_eq!(outcome["status"], "optimal", "{value}");
    assert!(!outcome["objective"]["value"].is_f64(), "{value}");
    assert_eq!(
        outcome["objective"]["value"],
        json!({"type": "decimal", "coefficient": "30", "scale": "2"}),
        "{value}"
    );
    assert_eq!(outcome["certificate"]["type"], "potential", "{value}");
}

/// FR-1013-AC-1, FSpec FR-806-AC-2: a negative plan length and a read of
/// `elapsed` over an untimed model refuse at S3.
#[test]
#[trace("TC-2863", "FR-1013-AC-1")]
#[trace("FSpec-TC-1956", "FSpec-FR-806-AC-2")]
fn plan_declarations_refuse_at_check() {
    let (exit, value) = plan(&plan_unit("Int[-1, 3]", "", "Reach.length >= 1"), json!({}));
    assert_eq!(exit, 20, "{value}");
    names(&value, "type-mismatch");
}

/// FSpec FR-806-AC-2: a read of `elapsed` over an untimed model refuses
/// `missing-name`.
#[test]
#[trace("FSpec-TC-1956", "FSpec-FR-806-AC-2")]
fn an_elapsed_read_over_an_untimed_plan_refuses_missing_name() {
    let (exit, value) = plan(
        &plan_unit("Int[0, 3]", "minimize Reach.elapsed", "Reach.length >= 1"),
        json!({}),
    );
    assert_eq!(exit, 20, "{value}");
    names(&value, "missing-name");
    names(&value, "elapsed");
}

/// FR-1014-AC-1, FSpec FR-810-AC-1 to AC-6: each reference planning
/// example plans with its stated result and exact objective.
#[test]
#[trace("TC-2864", "FR-1014-AC-1")]
#[trace("FSpec-TC-1960", "FSpec-FR-810-AC-1")]
#[trace("FSpec-TC-1961", "FSpec-FR-810-AC-2")]
#[trace("FSpec-TC-1962", "FSpec-FR-810-AC-3")]
#[trace("FSpec-TC-1963", "FSpec-FR-810-AC-4")]
#[trace("FSpec-TC-1964", "FSpec-FR-810-AC-5")]
#[trace("FSpec-TC-1965", "FSpec-FR-810-AC-6")]
#[ignore = "QSL-1978: the six reference planning examples are not published yet; its FSpec planning fixtures are not in the pinned filament-specification v0.1.1, so published_file returns none until the pin moves"]
fn the_six_reference_planning_examples_plan_as_specified() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/planning");
    let decimal =
        |coefficient: &str| json!({"type": "decimal", "coefficient": coefficient, "scale": "2"});
    for (case, objective) in [
        ("o1-rollout", json!({"type": "integer", "value": "15"})),
        ("o2-battery-dispatch", decimal("30")),
        (
            "o3-production-schedule",
            json!({"type": "rational", "numerator": "60", "denominator": "1", "unit": "min"}),
        ),
        ("o4-staffing", json!({"type": "integer", "value": "20"})),
        (
            "o5-robot-routing",
            json!({"type": "integer", "value": "10"}),
        ),
        ("o6-autoscaling", decimal("80")),
    ] {
        // FR-1014: the example holds only its envelope; the unit, model,
        // snapshot and plan are FSpec's published fixture files.
        let directory = tempfile::tempdir().unwrap();
        let prefix = format!("proposals/planning-v1/fixtures/{case}/");
        for (path, bytes) in filament_specification::published_files(&prefix) {
            let name = path.strip_prefix(&prefix).expect("a fixture file");
            std::fs::write(directory.path().join(name), bytes).unwrap();
        }
        let envelope = root.join(case).join("request.json");
        assert!(envelope.is_file(), "missing example {}", envelope.display());
        let request = directory.path().join("request.json");
        std::fs::copy(&envelope, &request).unwrap();
        let (exit, value) = match fsl_command::command::run(&request, &crate::negotiation::Fr290) {
            Ok(result) => (
                result.exit_code,
                serde_json::from_str::<Value>(&result.value.to_string()).unwrap(),
            ),
            Err(error) => (error.category().exit_code(), error.value().unwrap()),
        };
        assert_eq!(exit, 0, "{case}: {value}");
        assert_eq!(value["synthesis"]["status"], "optimal", "{case}: {value}");
        assert_eq!(
            value["synthesis"]["objective"]["value"], objective,
            "{case}: {value}"
        );
    }
}

// ---- QSL-1979: TLA+ and Lean export -------------------------------------

fn low_unit() -> String {
    format!(
        "{MODEL_HEADER}invariant Low using v on Config::ConfigVersion at current {{ self.versionNumber >= 0 }}\n"
    )
}

fn export(unit: &str, with_model: bool, member: Value) -> (u8, Value) {
    run(unit, with_model, ("export", member))
}

/// FR-756-AC-1's counter (FR-1018 "Behavior"): one `Counter` of
/// `example/counter-third` with `inc` enabled while `value < 5` and adding 1,
/// the state clause `Bounded` (`value <= 5`) and the initial state
/// `value = 0`, so its reachable states are the six values 0 to 5.
pub(crate) const COUNTER_UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     profile m = \"filament.model.complete/v1\";\n\
     model Counter = \"example/counter-third\";\n\
     pre CanInc using v on Counter::Counter::inc { self.value < 5 }\n\
     post Inc using v on Counter::Counter::inc \
     { pre(self.value) < 5 and self.value = pre(self.value) + 1 }\n\
     invariant Bounded using m on Counter::Counter at current { self.value <= 5 }\n";

/// The counter's domain package: the fixture's third counter model with its
/// `Level` widened to `Int[0, 9]`, so `value <= 5` is a clause the
/// precondition keeps and not a consequence of the type.
pub(crate) fn counter_model() -> Vec<u8> {
    use crate::support::refinement_cas::{self, Variant};
    let mut document: Value =
        serde_json::from_slice(&refinement_cas::documents(Variant::CAS)[2]).unwrap();
    let level = format!("ix://{}/Level", refinement_cas::THIRD);
    for node in document["types"].as_array_mut().expect("types") {
        if node["identity"] == level.as_str() {
            for constraint in node["constraints"].as_array_mut().expect("constraints") {
                if constraint["keyword"] == "max" {
                    constraint["operands"]["value"] = json!("9");
                }
            }
        }
    }
    serde_json::to_vec(&document).unwrap()
}

/// Writes the counter's model, unit and initial snapshot into `directory`
/// and a `native-run/1` request whose `export` member is `member` with
/// `initial` naming the snapshot, and returns the request path.
pub(crate) fn counter_request(directory: &Path, mut member: Value) -> PathBuf {
    use crate::support::refinement_cas::THIRD;
    std::fs::write(directory.join("counter.json"), counter_model()).unwrap();
    std::fs::write(directory.join("unit.native"), COUNTER_UNIT).unwrap();
    let start = counter_snapshot(THIRD, json!({"value": {"integer": "0"}}));
    std::fs::write(directory.join("counter-0.json"), start.to_string()).unwrap();
    member["initial"] = json!("counter-0.json");
    let model = json!({"format": "semantic-ir/2.0.0", "source": {"file": "counter.json",
        "authority": "agent-ix", "identity": THIRD, "document": "Counter", "formal_revision": 1}});
    let request = json!({"format": "native-run/1", "request": {"models": [model],
        "program": {"source": source("unit.native")}, "export": member}});
    let path = directory.join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

/// Runs the request at `path` under `overrides`: its exit code, its native
/// run document and, when FSL returns one, its FR-286 outcome document.
fn run_path(
    path: &Path,
    overrides: &fsl_command::command::RequestOverrides<'_>,
) -> (u8, Value, Option<fsl_replay::OutcomeDocument>) {
    match fsl_command::command::run_with_overrides(path, overrides, &crate::negotiation::Fr290) {
        Ok(result) => (
            result.exit_code,
            serde_json::from_str(&result.value.to_string()).expect("a JSON document"),
            result.outcome,
        ),
        Err(error) => (error.category().exit_code(), error.value().unwrap(), None),
    }
}

/// Exports the counter with `member` in a fresh directory.
pub(crate) fn counter_export(member: Value) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    let path = counter_request(directory.path(), member);
    let (exit, value, _) = run_path(&path, &fsl_command::command::RequestOverrides::default());
    (exit, value)
}

/// The JSON form of an FR-286 outcome document.
pub(crate) fn outcome_json(document: &fsl_replay::OutcomeDocument) -> Value {
    serde_json::from_slice(&document.to_bytes().expect("serializes")).unwrap()
}

/// Writes `BRAKING` as the unit and a `native-run/1` request whose `export`
/// member is `member` into `directory`; returns the request path. Its one
/// obligation is the hybrid claim `SlowEnough`, whose flows the target cannot
/// state, so under `minimum_fidelity` `any` it assembles a package.
pub(crate) fn braking_request(directory: &Path, member: Value) -> PathBuf {
    std::fs::write(directory.join("unit.native"), BRAKING).unwrap();
    let request = json!({"format": "native-run/1", "request": {"models": [],
        "program": {"source": source("unit.native")}, "export": member}});
    let path = directory.join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

/// An export of `SlowEnough` that assembles: `member` over the defaults.
pub(crate) fn braking_member() -> Value {
    json!({"profile": "filament.output.tla2/v1", "obligations": ["SlowEnough"],
        "minimum_fidelity": "any"})
}

/// FR-1015-AC-1, FSpec FR-811-AC-2/AC-3 and FR-817-AC-3: below the
/// default floor the export refuses with no text, no package and no file,
/// exit 21; under `any` it assembles with the unrepresented record claiming
/// no region. (The `preserved` companion record of FR-811-AC-2 needs the
/// lowering of FR-812, which stays ignored below.)
#[test]
#[trace("TC-2865", "FR-1015-AC-1")]
#[trace("FSpec-TC-1970", "FSpec-FR-811-AC-2")]
#[trace("FSpec-TC-1970", "FSpec-FR-811-AC-3")]
#[trace("FSpec-TC-1976", "FSpec-FR-817-AC-3")]
fn an_export_below_its_fidelity_floor_refuses_with_no_text() {
    let directory = tempfile::tempdir().unwrap();
    let mut member = braking_member();
    member.as_object_mut().unwrap().remove("minimum_fidelity");
    member["output"] = json!({"file": "m.tla"});
    let path = braking_request(directory.path(), member);
    let (exit, value, outcome) =
        run_path(&path, &fsl_command::command::RequestOverrides::default());
    assert_eq!(
        value["export"]["format"], "filament.export-outcome/v1",
        "{value}"
    );
    assert_eq!(value["export"]["status"], "refused", "{value}");
    assert_eq!(
        value["export"]["refusal"]["code"], "unsupported_projection",
        "{value}"
    );
    names(&value["export"], "below-fidelity-floor");
    names(&value["export"]["refusal"]["failing"], "SlowEnough");
    assert!(value["export"].get("target_text").is_none(), "{value}");
    assert!(value["export"].get("package").is_none(), "{value}");
    assert!(!directory.path().join("m.tla").exists());
    assert_eq!(exit, 21, "{value}");
    assert_eq!(
        outcome_json(&outcome.expect("an outcome"))["category"],
        "unsupported"
    );
    let (exit, value) = export(BRAKING, false, braking_member());
    assert_eq!(value["export"]["status"], "assembled", "{value}");
    let record = &value["export"]["records"][0];
    assert_eq!(record["disposition"], "unrepresented", "{value}");
    assert_eq!(record["regions"], json!([]), "{value}");
    assert_eq!(exit, 0, "{value}");
}

/// FR-1015-AC-2, FSpec FR-817-AC-4: a denied output-byte charge stops the
/// export `incomplete` naming `export.output_bytes` and its bound, exit 22,
/// whether the limit is in the member or the `--limit` override; a `limits`
/// entry that is not an FR-255 setting name refuses, exit 20; a setting given
/// in both refuses naming `limits`.
#[test]
#[trace("TC-2865", "FR-1015-AC-2")]
#[trace("FSpec-TC-1976", "FSpec-FR-817-AC-4")]
fn an_export_stops_at_its_output_byte_limit() {
    use fsl_command::command::RequestOverrides;
    let mut member = braking_member();
    member["limits"] = json!({"export.output_bytes": 1});
    let (exit, value) = export(BRAKING, false, member);
    assert_eq!(value["export"]["status"], "incomplete", "{value}");
    names(&value["export"], "export.output_bytes");
    assert_eq!(
        value["export"]["cause"],
        json!({"type": "limit-reached", "limit": "output_bytes", "value": 1,
            "setting": "export.output_bytes"}),
        "{value}"
    );
    assert!(value["export"].get("target_text").is_none(), "{value}");
    assert_eq!(exit, 22, "{value}");
    let mut member = braking_member();
    member["limits"] = json!({"output_bytes": 1});
    let (exit, value) = export(BRAKING, false, member);
    assert_eq!(exit, 20, "{value}");
    names(&value, "invalid-request");
    names(&value, "output_bytes");
    assert!(value.get("export").is_none(), "{value}");
    let directory = tempfile::tempdir().unwrap();
    let path = braking_request(directory.path(), braking_member());
    let limits = fsl_replay::spine::SpineLimits {
        export: fsl_replay::spine::ExportLimits {
            output_bytes: Some(1),
            ..Default::default()
        },
        ..Default::default()
    };
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    let (exit, value, _) = run_path(&path, &overrides);
    assert_eq!(value["export"]["status"], "incomplete", "{value}");
    names(&value["export"], "export.output_bytes");
    assert_eq!(exit, 22, "{value}");
    let mut member = braking_member();
    member["limits"] = json!({"export.output_bytes": 9});
    let doubled = tempfile::tempdir().unwrap();
    let path = braking_request(doubled.path(), member);
    let (exit, value, _) = run_path(&path, &overrides);
    assert_eq!(exit, 20, "{value}");
    names(&value, "limits");
    names(&value, "export.output_bytes");
}

/// FR-1015-AC-3, FSpec FR-817-AC-1/AC-2: FSL writes the target file named
/// by `export.output` or by the `output` request override, byte-identically,
/// never over an existing path, and refuses the two together.
#[test]
#[trace("TC-2992", "FR-1015-AC-3")]
#[trace("FSpec-TC-1976", "FSpec-FR-817-AC-1")]
#[trace("FSpec-TC-1976", "FSpec-FR-817-AC-2")]
fn an_export_writes_its_target_file_from_the_member_or_the_override() {
    use fsl_command::command::RequestOverrides;
    // The request member names the file.
    let first = tempfile::tempdir().unwrap();
    let mut with_output = braking_member();
    with_output["output"] = json!({"file": "m.tla"});
    let path = braking_request(first.path(), with_output);
    let (exit, value, _) = run_path(&path, &RequestOverrides::default());
    assert_eq!(exit, 0, "{value}");
    let text = value["export"]["target_text"]
        .as_str()
        .expect("target text");
    let written = std::fs::read(first.path().join("m.tla")).expect("the target file");
    assert_eq!(written, text.as_bytes());
    assert_eq!(
        value["export"]["output"],
        json!({"path": "m.tla", "bytes_written": text.len()}),
        "{value}"
    );
    // The same path again: refused by path before any stage, bytes unchanged.
    let error = fsl_command::command::run_with_overrides(
        &path,
        &RequestOverrides::default(),
        &crate::negotiation::Fr290,
    )
    .expect_err("an existing path refuses");
    assert_eq!(error.category().exit_code(), 20);
    let value = error.value().unwrap();
    names(&value, "invalid-request");
    names(&value, "m.tla");
    let document = outcome_json(&error.outcome_document(fsl_replay::Operation::Export));
    assert_eq!(document["last_stage"], Value::Null, "{document}");
    assert_eq!(document["result"], Value::Null, "{document}");
    assert_eq!(std::fs::read(first.path().join("m.tla")).unwrap(), written);
    // The override names the file: byte-identical.
    let second = tempfile::tempdir().unwrap();
    let path = braking_request(second.path(), braking_member());
    let target = second.path().join("m.tla");
    let overrides = RequestOverrides {
        output: Some(&target),
        ..RequestOverrides::default()
    };
    let (exit, value, _) = run_path(&path, &overrides);
    assert_eq!(exit, 0, "{value}");
    assert_eq!(std::fs::read(&target).expect("the target file"), written);
    // The member and the override together: one writer, refused, no file.
    let third = tempfile::tempdir().unwrap();
    let mut with_output = braking_member();
    with_output["output"] = json!({"file": "member.tla"});
    let path = braking_request(third.path(), with_output);
    let target = third.path().join("option.tla");
    let overrides = RequestOverrides {
        output: Some(&target),
        ..RequestOverrides::default()
    };
    let (exit, value, _) = run_path(&path, &overrides);
    assert_eq!(exit, 20, "{value}");
    names(&value, "--output");
    names(&value, "output");
    assert!(!target.exists() && !third.path().join("member.tla").exists());
}

/// FR-1015-AC-4, FSpec FR-817-AC-6: an assembled export carries FSpec
/// FR-299's package key apart from its records, with the three export
/// limits at their compile-lifecycle defaults.
#[test]
#[trace("TC-2992", "FR-1015-AC-4")]
#[trace("FSpec-TC-2020", "FSpec-FR-817-AC-6")]
fn an_assembled_export_carries_its_package_key() {
    let member = braking_member();
    let (_, first) = export(BRAKING, false, member.clone());
    let (_, second) = export(BRAKING, false, member.clone());
    let package = &first["export"]["package"];
    let mut keys: Vec<&str> = package
        .as_object()
        .expect("a package member")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["generator", "limits", "source_package", "target_profile"],
        "{first}"
    );
    assert_eq!(
        package["target_profile"], "filament.output.tla2/v1",
        "{first}"
    );
    assert_eq!(
        package["limits"],
        json!({"export.output_bytes": 16_777_216_u64, "export.nodes": 100_000,
            "export.work_units": 16_777_216_u64}),
        "{first}"
    );
    assert_eq!(package, &second["export"]["package"]);
    let mut lowered = member;
    lowered["limits"] = json!({"export.nodes": 99_999});
    let (_, third) = export(BRAKING, false, lowered);
    let mut expected = package.clone();
    expected["limits"]["export.nodes"] = json!(99_999);
    assert_eq!(third["export"]["package"], expected, "{third}");
}

/// FR-255-AC-22: the three `export.*` settings exist with the compile
/// lifecycle's defaults, `export.nodes=7` parses to that setting alone, and an
/// export stops at `export.work_units` naming the bound.
#[test]
#[trace("TC-2993", "FR-255-AC-22")]
fn the_export_settings_take_the_compile_defaults() {
    use fsl_foundation::setting::{parse_operands, Setting};
    for (name, compile) in [
        ("export.output_bytes", "emit.output_bytes"),
        ("export.nodes", "s3.nodes"),
        ("export.work_units", "s3.work_units"),
    ] {
        assert!(Setting::from_name(compile).is_some(), "{compile}");
        assert!(Setting::from_name(name).is_some(), "{name} is a setting");
    }
    let parsed = parse_operands(["export.nodes=7"]).expect("export.nodes parses");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].1, 7);
    let mut member = braking_member();
    member["limits"] = json!({"export.work_units": 1});
    let (exit, value) = export(BRAKING, false, member);
    assert_eq!(value["export"]["status"], "incomplete", "{value}");
    assert_eq!(
        value["export"]["cause"],
        json!({"type": "limit-reached", "limit": "work_units", "value": 1,
            "setting": "export.work_units"}),
        "{value}"
    );
    assert_eq!(exit, 22, "{value}");
    let (exit, value) = counter_export(json!({"profile": "filament.output.tla2/v1",
        "obligations": ["Bounded"], "limits": {"export.work_units": 1}}));
    assert_eq!(value["export"]["status"], "incomplete", "{value}");
    assert_eq!(exit, 22, "{value}");
}

/// FR-286-AC-22: the export's FR-286 document has operation `export`, the
/// standard categories and an `exported` result holding the export outcome.
#[test]
#[trace("TC-2994", "FR-286-AC-22")]
fn an_export_outcome_document_has_operation_export() {
    let directory = tempfile::tempdir().unwrap();
    let path = braking_request(directory.path(), braking_member());
    let (exit, value, outcome) =
        run_path(&path, &fsl_command::command::RequestOverrides::default());
    let document = outcome_json(&outcome.expect("an outcome document"));
    assert_eq!(document["operation"], "export", "{document}");
    assert_eq!(document["category"], "success", "{document}");
    assert_eq!(document["last_stage"], "S4", "{document}");
    assert_eq!(document["items"], json!([]), "{document}");
    assert_eq!(document["artifacts"], json!([]), "{document}");
    assert_eq!(document["result"]["kind"], "exported", "{document}");
    assert_eq!(document["result"]["export"], value["export"], "{document}");
    assert_eq!(exit, 0);
    let limited = tempfile::tempdir().unwrap();
    let mut member = braking_member();
    member["limits"] = json!({"export.output_bytes": 1});
    let path = braking_request(limited.path(), member);
    let (_, _, outcome) = run_path(&path, &fsl_command::command::RequestOverrides::default());
    let document = outcome_json(&outcome.expect("an outcome document"));
    assert_eq!(document["operation"], "export", "{document}");
    assert_eq!(document["category"], "incomplete", "{document}");
    assert_eq!(document["result"]["kind"], "exported", "{document}");
}

/// FR-865-AC-7: an export outcome renders as its header, target, bytes
/// written and output path, and no other line.
#[test]
#[trace("TC-2996", "FR-865-AC-7")]
fn an_export_outcome_renders_as_four_lines() {
    use filament_exact::Cancel;
    use fsl_inspect::{render_outcome, RenderForm, RenderSources};
    let directory = tempfile::tempdir().unwrap();
    let path = counter_request(
        directory.path(),
        json!({"profile": "filament.output.tla2/v1", "obligations": ["Bounded"],
            "output": {"file": "m.tla"}}),
    );
    let (_, value, outcome) = run_path(&path, &fsl_command::command::RequestOverrides::default());
    let document = outcome.expect("an outcome document");
    let rendered = render_outcome(
        &document,
        RenderForm::Text,
        &RenderSources::from_sources(Vec::new()),
        fsl_replay::RenderLimits::default(),
        &Cancel::new(),
    )
    .expect("the document renders");
    let text = rendered.value().as_str().to_owned();
    let length = value["export"]["target_text"]
        .as_str()
        .expect("target text")
        .len();
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        [
            "export: success (exit 0) at S4".to_owned(),
            "target: tla filament.output.tla2/v1".to_owned(),
            format!("bytes written: {length}"),
            "output: m.tla".to_owned(),
        ],
        "{text}"
    );
}

/// FR-1016-AC-1, FSpec FR-812-AC-1, FR-814-AC-3 and FR-817-AC-1: the TLA+
/// module is canonical ASCII with one `fsl:` comment per definition.
#[test]
#[trace("TC-2866", "FR-1016-AC-1")]
#[trace("FSpec-TC-1971", "FSpec-FR-812-AC-1")]
#[trace("FSpec-TC-1973", "FSpec-FR-814-AC-3")]
#[trace("FSpec-TC-1976", "FSpec-FR-817-AC-1")]
#[ignore = "QSL-1979: the TLA+ mapper is not implemented yet"]
fn the_tla_module_is_canonical_text() {
    let (exit, value) = export(
        &low_unit(),
        true,
        json!({"profile": "filament.output.tla2/v1", "obligations": ["Low"]}),
    );
    let text = value["export"]["target_text"]
        .as_str()
        .expect("target text");
    assert!(text.starts_with("---- MODULE M_"), "{text}");
    assert!(text.is_ascii(), "{text}");
    assert!(
        text.ends_with("====\n") && !text.ends_with("\n\n"),
        "{text}"
    );
    for part in [
        "VARIABLES",
        "Init ==",
        "Next ==",
        "Spec ==",
        "WF_vars(Next)",
        "\\* fsl: ",
    ] {
        assert!(text.contains(part), "{part}: {text}");
    }
    assert!(!text.lines().any(|line| line.ends_with(' ')), "{text}");
    assert!(value["export"]["name_map"].is_array(), "{value}");
    assert_eq!(exit, 0, "{value}");
}

/// FR-1017-AC-1, FSpec FR-813-AC-5 and FR-651-AC-2: the Lean file states
/// the obligation as a proposition with no theorem, proof or `sorry`.
#[test]
#[trace("TC-2867", "FR-1017-AC-1")]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-5")]
#[ignore = "QSL-1979: the Lean mapper is not implemented yet"]
fn the_lean_file_states_obligations_without_proofs() {
    let (_, value) = export(
        &low_unit(),
        true,
        json!({"profile": "filament.output.lean4/v1", "obligations": ["Low"]}),
    );
    let text = value["export"]["target_text"]
        .as_str()
        .expect("target text");
    assert!(text.is_ascii(), "{text}");
    for part in [
        "structure State",
        "def Init",
        "def Step",
        "def Ob_",
        "-- fsl: ",
    ] {
        assert!(text.contains(part), "{part}: {text}");
    }
    assert!(
        !text.contains("theorem") && !text.contains("sorry"),
        "{text}"
    );
}

/// FR-1018-AC-1, FSpec FR-818-AC-1: the counter export equals its reviewed
/// golden files byte for byte.
#[test]
#[trace("TC-2868", "FR-1018-AC-1")]
#[trace("FSpec-TC-1977", "FSpec-FR-818-AC-1")]
#[ignore = "QSL-1979: the export golden corpus is not published yet"]
fn the_counter_export_equals_its_golden_files() {
    let golden = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/export");
    for (profile, file) in [
        ("filament.output.tla2/v1", "counter.tla"),
        ("filament.output.lean4/v1", "counter.lean"),
    ] {
        let expected = std::fs::read_to_string(golden.join(file))
            .unwrap_or_else(|error| panic!("golden {file}: {error}"));
        let (_, value) = counter_export(json!({"profile": profile, "obligations": ["Bounded"]}));
        assert_eq!(value["export"]["target_text"], json!(expected), "{file}");
    }
}

/// FR-1018-AC-2, FSpec FR-818-AC-2: with TLC present its state count and
/// verdict equal FSL's exploration of the counter (six states, `Bounded`
/// holding); without it the oracle names the missing tool. The export
/// outcome itself reports no native exploration (FSpec FR-817).
#[test]
#[trace("TC-2868", "FR-1018-AC-2")]
#[trace("FSpec-TC-1977", "FSpec-FR-818-AC-2")]
#[ignore = "QSL-1979: the TLC export oracle is not implemented yet"]
fn the_tlc_oracle_agrees_or_names_the_missing_tool() {
    let (_, value) = counter_export(json!({"profile": "filament.output.tla2/v1",
        "obligations": ["Bounded"], "observer": "tlc"}));
    assert!(value["export"].get("native").is_none(), "{value}");
    let observer = &value["export"]["observer"];
    assert!(observer.is_object(), "{value}");
    if observer["outcome"] == "tool-not-found" {
        names(observer, "tlc not found");
    } else {
        assert_eq!(observer["reachable_states"], 6, "{value}");
        assert_eq!(observer["verdict"], "holds", "{value}");
    }
}

// ---- QSL-1978: hybrid plans ----------------------------------------------

const TANK: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.temporal.infinite-trace/v1\";\n\
     hybrid Tank using p {\n\
       mode Fill invariant { h <= 10 } flow { h' = 1; }\n\
         transition to Drain when { h >= 8 } reset { h = h };\n\
       mode Drain invariant { h >= 0 } flow { h' = -2; }\n\
     }\n\
     type Lengths = Int[0, 1];\n";

/// The reference hybrid provider (FR-1019) to register beside
/// `Engines::fsl()` through `run_with_providers` (FR-1021: the engine set is
/// a library parameter, never a request member). Until FR-1019's
/// `Engine::hybrid` reference descriptor exists this registers none, so the
/// provider-backed assertions below fail as they should.
fn reference_hybrid_providers() -> Vec<fsl_analyze::analyze::Engine> {
    Vec::new()
}

/// Runs `unit` with the `synthesis` member through `run_with_providers`
/// with `providers` registered beside `Engines::fsl()`.
fn run_with_providers(
    unit: &str,
    synthesis: Value,
    providers: Vec<fsl_analyze::analyze::Engine>,
) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [], "program": {"source": source("unit.native")}, "synthesis": synthesis}});
    let path = directory.path().join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    match fsl_command::command::run_with_providers(
        &path,
        &crate::negotiation::Fr290,
        providers,
        &fsl_analyze::provider::ProviderOptions::default(),
        &filament_exact::Cancel::new(),
    ) {
        Ok(result) => (
            result.exit_code,
            serde_json::from_str(&result.value.to_string()).expect("a JSON document"),
        ),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

fn rational(numerator: &str, denominator: &str) -> Value {
    json!({"type": "rational", "numerator": numerator, "denominator": denominator})
}

fn tank_unit(goal: &str) -> String {
    format!("{TANK}synthesis Reach using p grammar Tank domain Lengths satisfies {{ {goal} }};\n")
}

fn tank_member(grid: Option<Value>) -> Value {
    let mut synthesis = json!({"declaration": "Reach"});
    if let Some(grid) = grid {
        synthesis["dwell_grid"] = grid;
    }
    synthesis
}

/// FR-1021-AC-1, FSpec FR-819-AC-1 to AC-4: a hybrid model plans
/// `feasible` through the reference provider registered by the caller,
/// `infeasible` with a certified proof, `unsupported` through `run`, whose
/// engine set holds no hybrid provider, and needs a `dwell_grid`.
#[test]
#[trace("TC-2871", "FR-1021-AC-1")]
#[trace("FSpec-TC-1966", "FSpec-FR-819-AC-1")]
#[trace("FSpec-TC-1966", "FSpec-FR-819-AC-2")]
#[trace("FSpec-TC-1966", "FSpec-FR-819-AC-3")]
#[trace("FSpec-TC-1966", "FSpec-FR-819-AC-4")]
#[ignore = "QSL-1978: hybrid plan synthesis is not implemented yet"]
fn a_hybrid_model_plans_through_its_provider() {
    let grid = || Some(rational("1", "1"));
    let (_, value) = run_with_providers(
        &tank_unit("Reach.final.mode = Tank::Mode::Drain"),
        tank_member(grid()),
        reference_hybrid_providers(),
    );
    assert_eq!(value["synthesis"]["status"], "feasible", "{value}");
    assert!(value["synthesis"]["bound"].is_null(), "{value}");
    let (_, value) = run_with_providers(
        &tank_unit("Reach.final.h >= 12"),
        tank_member(grid()),
        reference_hybrid_providers(),
    );
    assert_eq!(value["synthesis"]["status"], "infeasible", "{value}");
    assert_eq!(value["synthesis"]["certification"], "certified", "{value}");
    let (exit, value) = run(
        &tank_unit("Reach.final.mode = Tank::Mode::Drain"),
        false,
        ("synthesis", tank_member(grid())),
    );
    assert_eq!(value["synthesis"]["status"], "unsupported", "{value}");
    names(&value["synthesis"], "hybrid-dynamics");
    assert_eq!(exit, 21, "{value}");
    let (exit, value) = run(
        &tank_unit("Reach.final.mode = Tank::Mode::Drain"),
        false,
        ("synthesis", tank_member(None)),
    );
    assert_eq!(exit, 20, "{value}");
    names(&value, "missing-member");
    names(&value, "/synthesis/dwell_grid");
}

/// FR-1021-AC-2, FSpec FR-805-AC-8 and AC-6: `dwell_grid` is an exact
/// rational in the run value form, `engines` is not a member, and the
/// `unsupported` outcome carries every member with its no-plan members null.
#[test]
#[trace("TC-2871", "FR-1021-AC-2")]
#[trace("FSpec-TC-1955", "FSpec-FR-805-AC-8")]
#[trace("FSpec-TC-1955", "FSpec-FR-805-AC-6")]
#[ignore = "QSL-1978: the hybrid plan request reader is not implemented yet"]
fn a_hybrid_plan_request_reads_an_exact_grid_and_no_engine_member() {
    let unit = tank_unit("Reach.final.mode = Tank::Mode::Drain");
    for (grid, cause) in [
        (json!(1), "wrong-value-kind"),
        (rational("0", "1"), "invalid-value"),
    ] {
        let (exit, value) = run(&unit, false, ("synthesis", tank_member(Some(grid))));
        assert_eq!(exit, 20, "{value}");
        names(&value, cause);
        names(&value, "/synthesis/dwell_grid");
    }
    let mut member = tank_member(Some(rational("1", "2")));
    member["engines"] = json!("fsl_with_reference_hybrid");
    let (exit, value) = run(&unit, false, ("synthesis", member));
    assert_eq!(exit, 20, "{value}");
    names(&value, "unknown-member");
    names(&value, "/synthesis/engines");
    let (_, value) = run(
        &unit,
        false,
        ("synthesis", tank_member(Some(rational("1", "2")))),
    );
    let outcome = &value["synthesis"];
    assert_eq!(outcome["status"], "unsupported", "{value}");
    for member in ["plan", "replay", "certificate", "certification", "bound"] {
        assert!(
            outcome.get(member).is_some_and(Value::is_null),
            "{member}: {value}"
        );
    }
    assert!(outcome["objective"]["value"].is_null(), "{value}");
}

// ---- QSL-1978: the counter fixture and the planner's test seams ----------

const PLANNING_FIXTURES: &str = "proposals/planning-v1/fixtures/";

/// The declaration FSpec FR-650 "The counter fixture" holds verbatim.
const COUNTER_REACH: &str =
    "synthesis Reach using p grammar M::Counter domain Upto5 satisfies { Reach.final.value = 8 };";

/// FR-755's `PlanFault` seam. Until `fsl_analyze` exposes it, this stand-in
/// carries the fault and [`run_with_plan_fault`] runs the request unfaulted,
/// so the seam assertions fail as they should.
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "read once fsl_analyze's PlanFault replaces this stand-in"
)]
enum PlanFault {
    ReportPlan(Vec<&'static str>),
    DropSuccessor { entry: usize, successor: usize },
}

/// Writes FSpec's published fixture `case` into a directory, the unit with
/// `edit` applied, and returns the directory and its `native-run/1` request
/// whose `synthesis` member is the fixture's `plan.json` merged with
/// `synthesis`.
fn fixture_request(
    case: &str,
    edit: &dyn Fn(String) -> String,
    synthesis: Value,
) -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let prefix = format!("{PLANNING_FIXTURES}{case}/");
    let mut files = 0;
    for (path, bytes) in filament_specification::published_files(&prefix) {
        let name = path.strip_prefix(&prefix).expect("a fixture file");
        std::fs::write(directory.path().join(name), bytes).unwrap();
        files += 1;
    }
    assert_eq!(files, 4, "FSpec publishes the {case} fixture's four files");
    let unit_path = directory.path().join("unit.native");
    let unit = std::fs::read_to_string(&unit_path).unwrap();
    std::fs::write(&unit_path, edit(unit)).unwrap();
    let mut member: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("plan.json")).unwrap())
            .unwrap();
    for (key, value) in synthesis.as_object().into_iter().flatten() {
        member[key] = value.clone();
    }
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source": source("model.semantic-ir.json")}],
        "program": {"source": source("unit.native")}, "synthesis": member}});
    let path = directory.path().join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    (directory, path)
}

fn run_plan_path(path: &Path) -> (u8, Value) {
    match fsl_command::command::run(path, &crate::negotiation::Fr290) {
        Ok(result) => (
            result.exit_code,
            serde_json::from_str(&result.value.to_string()).expect("a JSON document"),
        ),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

/// FR-755's `run_with_plan_fault`; see [`PlanFault`].
fn run_with_plan_fault(path: &Path, fault: &PlanFault) -> (u8, Value) {
    let _ = fault;
    run_plan_path(path)
}

fn fixture(case: &str, edit: &dyn Fn(String) -> String, synthesis: Value) -> (u8, Value) {
    let (_directory, path) = fixture_request(case, edit, synthesis);
    run_plan_path(&path)
}

fn counter_goal(goal: &str) -> impl Fn(String) -> String + '_ {
    move |unit: String| {
        unit.replace(
            COUNTER_REACH,
            &COUNTER_REACH.replace("Reach.final.value = 8", goal),
        )
    }
}

const DOUBLE_COSTS_FIVE: &str =
    "minimize sum<Cost>(s in Reach.steps : if s.operation = M::Op::double then 5 else 1) satisfies";

fn operations(outcome: &Value) -> Vec<String> {
    outcome["plan"]["steps"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|step| step["operation"].as_str().unwrap_or_default().to_owned())
        .collect()
}

/// FR-755-AC-1, FSpec FR-650-AC-1/AC-2 and FR-805-AC-7: over the counter,
/// `length` gives `inc, double, double` at `{"type": "integer", "value": "3"}` and the
/// `double`-costs-five sum gives `inc, inc, inc, double` at 8.
#[test]
#[trace("TC-2105", "FR-755-AC-1")]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-1")]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-2")]
#[trace("FSpec-TC-1955", "FSpec-FR-805-AC-7")]
#[ignore = "QSL-1978: plan search over the counter fixture is not implemented yet; its FSpec planning fixtures are not in the pinned filament-specification v0.1.1, so published_file returns none until the pin moves"]
fn the_counter_plans_by_length_and_by_step_cost() {
    let (exit, value) = fixture("counter", &|unit| unit, json!({}));
    let outcome = &value["synthesis"];
    assert_eq!(outcome["status"], "optimal", "{value}");
    assert_eq!(operations(outcome), ["inc", "double", "double"], "{value}");
    assert_eq!(
        outcome["objective"]["value"],
        json!({"type": "integer", "value": "3"}),
        "{value}"
    );
    assert_eq!(outcome["certificate"]["type"], "potential", "{value}");
    assert_eq!(outcome["certification"], "certified", "{value}");
    assert_eq!(exit, 0, "{value}");
    let (_, value) = fixture(
        "counter",
        &|unit| {
            unit.replace(
                COUNTER_REACH,
                &COUNTER_REACH.replace("satisfies", DOUBLE_COSTS_FIVE),
            )
        },
        json!({}),
    );
    let outcome = &value["synthesis"];
    assert_eq!(outcome["status"], "optimal", "{value}");
    assert_eq!(
        operations(outcome),
        ["inc", "inc", "inc", "double"],
        "{value}"
    );
    assert_eq!(
        outcome["objective"]["value"],
        json!({"type": "integer", "value": "8"}),
        "{value}"
    );
    assert_eq!(outcome["certification"], "certified", "{value}");
}

/// FR-755-AC-2, FSpec FR-650-AC-5/AC-7: a tampered `potential` certificate
/// demotes the plan to `feasible`; a tampered closure certificate leaves the
/// goal-21 run `incomplete` with `certificate-rejected`, never `infeasible`.
#[test]
#[trace("TC-2105", "FR-755-AC-2")]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-5")]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-7")]
#[ignore = "QSL-1978: the certificate tamper seam is not implemented yet; its FSpec planning fixtures are not in the pinned filament-specification v0.1.1, so published_file returns none until the pin moves"]
fn a_tampered_certificate_is_rejected_by_the_core() {
    let (_, value) = fixture(
        "counter",
        &counter_goal("Reach.final.value = 21"),
        json!({}),
    );
    assert_eq!(value["synthesis"]["status"], "infeasible", "{value}");
    assert_eq!(
        value["synthesis"]["certificate"]["outcome"], "accepted",
        "{value}"
    );

    let (_directory, path) = fixture_request("counter", &|unit| unit, json!({}));
    let (_, value) = run_with_plan_fault(
        &path,
        &PlanFault::DropSuccessor {
            entry: 0,
            successor: 1,
        },
    );
    let outcome = &value["synthesis"];
    assert_eq!(outcome["status"], "feasible", "{value}");
    assert_eq!(operations(outcome), ["inc", "double", "double"], "{value}");
    assert_eq!(outcome["certificate"]["outcome"], "rejected", "{value}");
    assert_eq!(outcome["certificate"]["rule"], "closure", "{value}");
    assert_eq!(outcome["certificate"]["entry"], 0, "{value}");
    assert_eq!(outcome["certification"], "uncertified", "{value}");

    let (_directory, path) = fixture_request(
        "counter",
        &counter_goal("Reach.final.value = 21"),
        json!({}),
    );
    let (exit, value) = run_with_plan_fault(
        &path,
        &PlanFault::DropSuccessor {
            entry: 0,
            successor: 0,
        },
    );
    let outcome = &value["synthesis"];
    assert_eq!(outcome["status"], "incomplete", "{value}");
    names(&outcome["cause"], "certificate-rejected");
    assert!(outcome["plan"].is_null(), "{value}");
    assert_eq!(exit, 22, "{value}");
}

/// FR-755-AC-4, FSpec FR-650-AC-6: a substituted plan whose fifth `double`
/// is disabled is a method failure naming step 4, with no outcome written.
#[test]
#[trace("TC-2105", "FR-755-AC-4")]
#[trace("FSpec-TC-1603", "FSpec-FR-650-AC-6")]
#[ignore = "QSL-1978: the plan substitution seam is not implemented yet; its FSpec planning fixtures are not in the pinned filament-specification v0.1.1, so published_file returns none until the pin moves"]
fn a_substituted_plan_that_fails_replay_is_a_method_failure() {
    let (_directory, path) = fixture_request("counter", &|unit| unit, json!({}));
    let (exit, value) = run_with_plan_fault(&path, &PlanFault::ReportPlan(vec!["double"; 5]));
    assert_eq!(exit, 30, "{value}");
    names(&value, "double");
    names(&value, "4");
    assert!(value["synthesis"].get("status").is_none(), "{value}");
}

/// FR-1012-AC-6, FSpec FR-801-AC-8: the canonical exploration pins the
/// counts of a stopped counter search.
#[test]
#[trace("TC-2862", "FR-1012-AC-6")]
#[trace("FSpec-TC-1951", "FSpec-FR-801-AC-8")]
#[ignore = "QSL-1978: the canonical plan exploration order is not implemented yet; its FSpec planning fixtures are not in the pinned filament-specification v0.1.1, so published_file returns none until the pin moves"]
fn the_canonical_exploration_pins_stopped_counts() {
    for (limits, states, work) in [
        (json!({"max_states": 2}), 2, 6),
        (json!({"max_work": 1}), 1, 1),
    ] {
        let first = fixture("counter", &|unit| unit, json!({"limits": limits}));
        let second = fixture("counter", &|unit| unit, json!({"limits": limits}));
        let outcome = &first.1["synthesis"];
        assert_eq!(outcome["status"], "incomplete", "{}", first.1);
        assert_eq!(outcome["explored"]["states"], states, "{}", first.1);
        assert_eq!(outcome["explored"]["work"], work, "{}", first.1);
        assert!(outcome["plan"].is_null(), "{}", first.1);
        assert_eq!(first.1.to_string(), second.1.to_string());
    }
}

/// FR-1012-AC-4, FSpec FR-808-AC-1/2/3/5: timed plans carry exact delays and
/// `elapsed` in the run value form.
#[test]
#[trace("TC-2862", "FR-1012-AC-4")]
#[trace("FSpec-TC-1958", "FSpec-FR-808-AC-1")]
#[trace("FSpec-TC-1958", "FSpec-FR-808-AC-2")]
#[trace("FSpec-TC-1958", "FSpec-FR-808-AC-3")]
#[trace("FSpec-TC-1958", "FSpec-FR-808-AC-5")]
#[ignore = "QSL-1978: timed plan search is not implemented yet; its FSpec planning fixtures are not in the pinned filament-specification v0.1.1, so published_file returns none until the pin moves"]
fn timed_plans_carry_exact_delays() {
    let (_, value) = fixture("timed-tick", &|unit| unit, json!({}));
    let outcome = &value["synthesis"];
    names(&outcome["objective"]["value"], "\"unit\":\"min\"");
    names(&outcome["objective"]["value"], "\"numerator\":\"30\"");
    for step in outcome["plan"]["steps"].as_array().expect("steps") {
        names(&step["delay"], "\"numerator\":\"0\"");
    }
    let (_, value) = fixture("timed-go", &|unit| unit, json!({}));
    let outcome = &value["synthesis"];
    assert_eq!(outcome["status"], "optimal", "{value}");
    assert_eq!(outcome["certificate"]["type"], "closure", "{value}");
    names(&outcome["plan"]["steps"][0]["delay"], "\"numerator\":\"2\"");
    let strict = |unit: String| unit.replace("x >= 2", "x > 2");
    let first = fixture("timed-go", &strict, json!({}));
    let second = fixture("timed-go", &strict, json!({}));
    let outcome = &first.1["synthesis"];
    assert_eq!(outcome["status"], "feasible", "{}", first.1);
    assert_eq!(outcome["bound"]["side"], "lower", "{}", first.1);
    assert_eq!(outcome["bound"]["attained"], false, "{}", first.1);
    names(&outcome["plan"]["steps"][0]["delay"], "\"numerator\":\"5\"");
    names(
        &outcome["plan"]["steps"][0]["delay"],
        "\"denominator\":\"2\"",
    );
    assert_eq!(first.1.to_string(), second.1.to_string());
}

/// FR-1012-AC-5, FSpec FR-809-AC-1/2: probabilistic plans compute exact
/// probabilities.
#[test]
#[trace("TC-2862", "FR-1012-AC-5")]
#[trace("FSpec-TC-1959", "FSpec-FR-809-AC-1")]
#[trace("FSpec-TC-1959", "FSpec-FR-809-AC-2")]
#[ignore = "QSL-1978: probabilistic plan search is not implemented yet; its FSpec planning fixtures are not in the pinned filament-specification v0.1.1, so published_file returns none until the pin moves"]
fn probabilistic_plans_compute_exact_probabilities() {
    let (_, value) = fixture("probabilistic-try", &|unit| unit, json!({}));
    let outcome = &value["synthesis"];
    assert_eq!(outcome["status"], "optimal", "{value}");
    assert_eq!(operations(outcome), ["retry"], "{value}");
    let (_, value) = fixture(
        "probabilistic-try",
        &|unit| unit.replace("satisfies", "maximize Reach.probability satisfies"),
        json!({}),
    );
    let outcome = &value["synthesis"];
    assert_eq!(outcome["status"], "optimal", "{value}");
    assert_eq!(operations(outcome), ["try", "try"], "{value}");
    assert_eq!(
        outcome["objective"]["value"],
        json!({"type": "rational", "numerator": "3", "denominator": "4"}),
        "{value}"
    );
}

/// FR-1013-AC-2, FSpec FR-806-AC-3/AC-4 and FR-823-AC-4: admission refuses a
/// value-type declaration carrying the plan member `initial`, an unknown
/// declaration, an unknown policy, a policy over another declaration and an
/// inadmissible initial snapshot, each at its `/synthesis` pointer.
#[test]
#[trace("TC-2863", "FR-1013-AC-2")]
#[trace("FSpec-TC-1956", "FSpec-FR-806-AC-3")]
#[trace("FSpec-TC-2038", "FSpec-FR-823-AC-4")]
#[trace("FSpec-TC-1956", "FSpec-FR-806-AC-4")]
fn plan_requests_refuse_at_admission() {
    let unit = format!(
        "{}type Digit = Int[0, 9];\n\
         synthesis Pick using v grammar Digit domain Digit satisfies {{ Pick = 7 }};\n{}",
        plan_unit("Int[0, 3]", "", "Reach.length >= 1").replacen(
            PLAN_HEADER,
            &PLAN_HEADER.replace(
                "model Config",
                "profile p = \"filament.temporal.infinite-trace/v1\";\nmodel Config"
            ),
            1
        ),
        BRAKING
            .split_once("hybrid")
            .map(|(_, rest)| format!("hybrid{rest}"))
            .unwrap(),
    );
    for (synthesis, cause, pointer) in [
        (
            json!({"declaration": "Pick"}),
            "unknown-member",
            "/synthesis/initial",
        ),
        (
            json!({"declaration": "Nope"}),
            "missing-name",
            "/synthesis/declaration",
        ),
        (
            json!({"policy": ["Nowhere"]}),
            "missing-name",
            "/synthesis/policy/0",
        ),
        (
            json!({"policy": ["SlowEnough"]}),
            "type-mismatch",
            "/synthesis/policy/0",
        ),
    ] {
        let (exit, value) = plan(&unit, synthesis);
        assert_eq!(exit, 20, "{value}");
        names(&value, cause);
        names(&value, pointer);
        assert!(value["synthesis"].get("status").is_none(), "{value}");
    }
    let directory = tempfile::tempdir().unwrap();
    // FSpec FR-806-AC-4: the healthy snapshot holds `root` and `child`, two
    // objects, which the population bound of one object refuses.
    spine::write(directory.path(), Case::Healthy).unwrap();
    std::fs::write(
        directory.path().join("unit.native"),
        plan_unit("Int[0, 3]", "", "Reach.length >= 1")
            + "bound HistoryBound on Config::config_history [1]\n",
    )
    .unwrap();
    let (exit, value) = write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": {
            "models": [{"format": "semantic-ir/2.0.0", "source": source("spine-model.semantic-ir.json")}],
            "program": {"source": source("unit.native")},
            "synthesis": {"declaration": "Reach", "initial": "spine-snapshot.json"}}}),
    );
    assert_eq!(exit, 20, "{value}");
    names(&value, "/synthesis/initial");
    assert!(value["synthesis"].get("status").is_none(), "{value}");
}

/// FR-1014-AC-2: each planning example holds only its host envelope, whose
/// `synthesis` member equals FSpec's published `plan.json`.
#[test]
#[trace("TC-2864", "FR-1014-AC-2")]
#[ignore = "QSL-1978: the planning example envelopes are not published yet; its FSpec planning fixtures are not in the pinned filament-specification v0.1.1, so published_file returns none until the pin moves"]
fn planning_examples_hold_only_their_envelope() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/planning");
    for case in [
        "o1-rollout",
        "o2-battery-dispatch",
        "o3-production-schedule",
        "o4-staffing",
        "o5-robot-routing",
        "o6-autoscaling",
    ] {
        let directory = root.join(case);
        let entries: Vec<_> = std::fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("{}: {error}", directory.display()))
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(entries, ["request.json"], "{case}");
        let request: Value =
            serde_json::from_slice(&std::fs::read(directory.join("request.json")).unwrap())
                .unwrap();
        let published =
            filament_specification::published_file(&format!("{PLANNING_FIXTURES}{case}/plan.json"))
                .unwrap_or_else(|| panic!("FSpec publishes {case}/plan.json"));
        let plan: Value = serde_json::from_slice(published).unwrap();
        assert_eq!(request["request"]["synthesis"], plan, "{case}");
    }
}

// ---- QSL-2247: refinement transfer through a native run -------------------

/// The clauses to transfer, the contracts of the third counter model and the
/// identity refinement `ThirdRefinesSpec`, which maps the one concrete
/// operation to the one abstract operation and so is step-for-step.
const TRANSFER_DECLARATIONS: &str =
    "pre ThirdCanInc using v on Third::Counter::inc { self.value < 2 }\n\
     post ThirdInc using v on Third::Counter::inc \
     { pre(self.value) < 2 and self.value = pre(self.value) + 1 }\n\
     temporal PrevSafe using inf over (c: Spec::Counter) clock \"model-steps\" \
     on origin { always (holds(c.value = 2) implies once[1,1] holds(c.value = 1)) }\n\
     temporal SoonTwo using inf over (c: Spec::Counter) clock \"model-steps\" \
     on origin { always (holds(c.value = 1) implies eventually[0,2] holds(c.value = 2)) }\n\
     temporal AtMostTwo using inf over (c: Spec::Counter) clock \"model-steps\" \
     on origin { always holds(c.value <= 2) }\n\
     temporal OverThird using inf over (c: Third::Counter) clock \"model-steps\" \
     on origin { always holds(c.value <= 2) }\n\
     refinement ThirdRefinesSpec using inf abstract Spec concrete Third {\n\
       population Spec::counters from Third::counters;\n\
       object Spec::Counter from Third::Counter { value = self.value; }\n\
       step Third::Counter::inc -> Spec::Counter::inc(self);\n\
     }\n";

/// An initialization snapshot of the one counter `c` of `package`.
fn counter_snapshot(package: &str, fields: Value) -> Value {
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": format!("{package}/start")},
        "observation": "current", "model": {"identity": package},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": format!("ix://{package}/counters"), "complete": true,
            "objects": [{"key": "c", "type": format!("ix://{package}/Counter"), "fields": fields}]}],
    })
}

/// A `refinement_transfer` member over `refinement` between `abstract` and
/// `concrete` for `clauses`, with the fixture's initial snapshots.
fn transfer_member(
    refinement: &str,
    (abstract_model, concrete): (&str, &str),
    clauses: &[&str],
) -> Value {
    let initial = |model: &str| match model {
        "Spec" => "spec-0",
        "Impl" => "cas-0",
        _ => "third-0",
    };
    json!({
        "refinement": refinement,
        "abstract": {"model": abstract_model, "initial": [initial(abstract_model)]},
        "concrete": {"model": concrete, "initial": [initial(concrete)]},
        "clauses": clauses,
        "observations": {"spec-0": "spec-0.json", "cas-0": "cas-0.json", "third-0": "third-0.json"},
    })
}

/// Writes ADR-020 §8's counter models, a unit with `CasRefinesCounter` and
/// [`TRANSFER_DECLARATIONS`], the initial snapshots and a `native-run/1`
/// request carrying `members` (with the `Spec` model's entry dropped when
/// `supply_spec` is false), runs it and returns its exit code and document.
fn transfer_run(members: &[(&str, Value)], supply_spec: bool) -> (u8, Value) {
    use crate::support::refinement_cas::{self, Variant};
    let directory = tempfile::tempdir().unwrap();
    let declarations = format!(
        "{TRANSFER_DECLARATIONS}{}",
        refinement_cas::cas_refines_counter()
    );
    let unit = refinement_cas::unit_text(
        &refinement_cas::packages(Variant::CAS),
        &declarations,
        Variant::CAS,
    );
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let identities = [
        refinement_cas::SPEC,
        refinement_cas::CAS,
        refinement_cas::THIRD,
    ];
    let mut models = Vec::new();
    for (index, document) in refinement_cas::documents(Variant::CAS).iter().enumerate() {
        let file = format!("model-{index}.json");
        std::fs::write(directory.path().join(&file), document).unwrap();
        if index > 0 || supply_spec {
            models.push(
                json!({"format": "semantic-ir/2.0.0", "source": {"file": file,
                "authority": "agent-ix", "identity": identities[index],
                "document": format!("Model{index}"), "formal_revision": 1}}),
            );
        }
    }
    let zero = json!({"type": "integer", "value": "0"});
    let snapshots = [
        (
            "spec-0.json",
            counter_snapshot(refinement_cas::SPEC, json!({"value": zero})),
        ),
        (
            "third-0.json",
            counter_snapshot(refinement_cas::THIRD, json!({"value": zero})),
        ),
        (
            "cas-0.json",
            counter_snapshot(
                refinement_cas::CAS,
                json!({"value": zero, "tmpA": zero, "busyA": {"boolean": false},
                    "tmpB": zero, "busyB": {"boolean": false}}),
            ),
        ),
    ];
    for (file, snapshot) in snapshots {
        std::fs::write(directory.path().join(file), snapshot.to_string()).unwrap();
    }
    let mut request = json!({"models": models, "program": {"source": source("unit.native")}});
    for (name, member) in members {
        request[*name] = member.clone();
    }
    write_request(
        directory.path(),
        &json!({"format": "native-run/1", "request": request}),
    )
}

/// The transfer result of `clause` in a `refinement_transfer` outcome.
fn transfer<'a>(value: &'a Value, clause: &str) -> &'a Value {
    value["refinement_transfer"]["transfers"]
        .as_array()
        .and_then(|transfers| transfers.iter().find(|entry| entry["clause"] == clause))
        .map(|entry| &entry["result"])
        .unwrap_or_else(|| panic!("a transfer of {clause}: {value}"))
}

/// FR-1030-AC-1/AC-2, FR-1020-AC-1/AC-2, FSpec FR-380-AC-9, FSpec
/// FR-820-AC-1/AC-2: `once[1,1]` and `eventually[0,2]` clauses transfer
/// `proved` through the step-for-step `ThirdRefinesSpec`, and settle
/// `transfer-not-preserving` through `CasRefinesCounter`, whose step map
/// sends concrete operations to `stutter`; a fragment clause still transfers.
#[test]
#[trace("TC-2920", "FR-1030-AC-1")]
#[trace("TC-2920", "FR-1030-AC-2")]
#[trace("TC-2870", "FR-1020-AC-1")]
#[trace("TC-2870", "FR-1020-AC-2")]
#[trace("FSpec-TC-1042", "FSpec-FR-380-AC-9")]
#[trace("FSpec-TC-2000", "FSpec-FR-820-AC-1")]
#[trace("FSpec-TC-2000", "FSpec-FR-820-AC-2")]
#[ignore = "QSL-2247: the refinement_transfer request member and step-for-step transfer are not implemented yet"]
fn previous_transfers_only_through_a_step_for_step_refinement() {
    let (exit, value) = transfer_run(
        &[(
            "refinement_transfer",
            transfer_member(
                "ThirdRefinesSpec",
                ("Spec", "Third"),
                &["PrevSafe", "SoonTwo"],
            ),
        )],
        true,
    );
    assert_eq!(exit, 0, "{value}");
    assert_eq!(
        value["refinement_transfer"]["refinement"]["step_for_step"]["holds"], true,
        "{value}"
    );
    for clause in ["PrevSafe", "SoonTwo"] {
        let result = transfer(&value, clause);
        assert_eq!(result["value"], "proved", "{value}");
        assert_eq!(result["basis"], "decisive-witness", "{value}");
        assert_eq!(
            result["transfer"]["premises"].as_array().map(Vec::len),
            Some(2),
            "{value}"
        );
    }

    let (exit, value) = transfer_run(
        &[(
            "refinement_transfer",
            transfer_member(
                "CasRefinesCounter",
                ("Spec", "Impl"),
                &["PrevSafe", "SoonTwo", "AtMostTwo"],
            ),
        )],
        true,
    );
    assert_eq!(exit, 22, "{value}");
    let step_for_step = &value["refinement_transfer"]["refinement"]["step_for_step"];
    assert_eq!(step_for_step["holds"], false, "{value}");
    assert_eq!(
        step_for_step["row"]["operation"], "Impl::Counter::beginA",
        "{value}"
    );
    assert_eq!(step_for_step["row"]["arm"], "stutter", "{value}");
    for (clause, operator) in [("PrevSafe", "once[1,1]"), ("SoonTwo", "eventually[0,2]")] {
        let result = transfer(&value, clause);
        assert_eq!(result["value"], "inconclusive", "{value}");
        assert_eq!(
            result["cause"]["type"], "transfer-not-preserving",
            "{value}"
        );
        assert_eq!(result["cause"]["reason"], "fragment", "{value}");
        assert_eq!(result["cause"]["operator"], operator, "{value}");
    }
    assert_eq!(transfer(&value, "AtMostTwo")["value"], "proved", "{value}");
}

/// FR-1031-AC-1 to AC-3: conflicting selection members, a duplicate clause,
/// a missing model, a subject that is not the refinement's and clauses
/// outside the transferable family each refuse at their stage and pointer.
#[test]
#[trace("TC-2921", "FR-1031-AC-1")]
#[trace("TC-2921", "FR-1031-AC-2")]
#[trace("TC-2921", "FR-1031-AC-3")]
#[ignore = "QSL-2247: refinement_transfer request refusals are not implemented yet"]
fn a_refinement_transfer_request_is_refused_by_stage() {
    let member = |abstract_model: &str, concrete: &str, clauses: &[&str]| {
        transfer_member("ThirdRefinesSpec", (abstract_model, concrete), clauses)
    };
    let refused = |members: &[(&str, Value)], supply_spec: bool, stage: &str, code: &str| {
        let (exit, value) = transfer_run(members, supply_spec);
        assert_eq!(exit, 20, "{value}");
        names(&value, stage);
        names(&value, code);
        value
    };

    let call = json!({"function": "f", "arguments": []});
    let value = refused(
        &[
            (
                "refinement_transfer",
                member("Spec", "Third", &["PrevSafe"]),
            ),
            ("call", call),
        ],
        true,
        "request",
        "invalid-request",
    );
    names(&value, "refinement_transfer");
    names(&value, "call");
    let value = refused(
        &[
            (
                "refinement_transfer",
                member("Spec", "Third", &["PrevSafe"]),
            ),
            ("items", json!([])),
        ],
        true,
        "request",
        "invalid-request",
    );
    names(&value, "items");
    let value = refused(
        &[(
            "refinement_transfer",
            member("Spec", "Third", &["PrevSafe", "PrevSafe"]),
        )],
        true,
        "request",
        "invalid-request",
    );
    names(&value, "/refinement_transfer/clauses/1");

    let value = refused(
        &[(
            "refinement_transfer",
            member("Ghost", "Third", &["PrevSafe"]),
        )],
        true,
        "select",
        "missing-name",
    );
    names(&value, "/refinement_transfer/abstract/model");
    refused(
        &[(
            "refinement_transfer",
            member("Spec", "Third", &["PrevSafe"]),
        )],
        false,
        "compile",
        "missing-selection",
    );
    let value = refused(
        &[("refinement_transfer", member("Spec", "Impl", &["PrevSafe"]))],
        true,
        "select",
        "invalid-value",
    );
    names(&value, "/refinement_transfer/concrete/model");
    names(&value, "Third");

    // `CanInc` is a state precondition, not an infinite-trace clause.
    let value = refused(
        &[("refinement_transfer", member("Spec", "Third", &["CanInc"]))],
        true,
        "select",
        "type-mismatch",
    );
    names(&value, "/refinement_transfer/clauses/0");
    let value = refused(
        &[(
            "refinement_transfer",
            member("Spec", "Third", &["OverThird"]),
        )],
        true,
        "select",
        "invalid-value",
    );
    names(&value, "/refinement_transfer/clauses/0");
}

// ---- QSL-1979: target results as trusted evidence -------------------------

/// FR-1022-AC-1, FSpec FR-815-AC-1: a Lean proof of an exported
/// proposition enters as `trusted` evidence, never `certified`, or settles
/// `tool-unavailable` when Lean is absent.
#[test]
#[trace("TC-2872", "FR-1022-AC-1")]
#[trace("FSpec-TC-1974", "FSpec-FR-815-AC-1")]
#[ignore = "QSL-1979: intake of TLA+ and Lean results as trusted evidence is not implemented yet"]
fn a_lean_proof_enters_as_trusted_evidence() {
    let (_, exported) = export(
        &low_unit(),
        true,
        json!({"profile": "filament.output.lean4/v1", "obligations": ["Low"]}),
    );
    let package = exported["export"]["package"].clone();
    assert!(package.is_object(), "{exported}");
    let (exit, value) = run(
        &low_unit(),
        true,
        (
            "export_evidence",
            json!({"package": package, "obligation": "Low", "tool": "lean",
                "artifact": "low_proof.lean"}),
        ),
    );
    let result = &value["export_evidence"];
    if result["value"] == "unsupported" {
        names(result, "tool-unavailable");
        assert_eq!(exit, 21, "{value}");
    } else {
        assert_eq!(result["value"], "proved", "{value}");
        assert_eq!(result["certification"], "trusted", "{value}");
    }
}

/// FR-1022-AC-3, FSpec FR-815-AC-3: evidence for an unrepresented record is
/// refused at its obligation member.
#[test]
#[trace("TC-2872", "FR-1022-AC-3")]
#[trace("FSpec-TC-1974", "FSpec-FR-815-AC-3")]
#[ignore = "QSL-1979: evidence intake refusals are not implemented yet"]
fn evidence_for_an_unrepresented_record_is_refused() {
    let (exit, value) = run(
        BRAKING,
        false,
        (
            "export_evidence",
            json!({"package": {}, "obligation": "SlowEnough", "tool": "tlc",
                "artifact": "slow.cfg"}),
        ),
    );
    assert_eq!(exit, 20, "{value}");
    names(&value, "/export_evidence/obligation");
}

/// Policy evaluation spends the request's accounting budget before a
/// trivial empty-plan goal can give a planning verdict.
#[test]
#[trace("FR-1011-AC-2", "FSpec-FR-801-AC-2")]
fn policy_accounting_exhaustion_is_an_incomplete_run() {
    let unit = plan_unit("Int[0, 0]", "", "true");
    let (unlimited_exit, unlimited) = plan(&unit, json!({"policy": ["SomeTime"]}));
    assert_eq!(unlimited_exit, 10, "{unlimited}");
    assert_eq!(
        unlimited["synthesis"]["status"], "infeasible",
        "{unlimited}"
    );
    let (exit, limited) = plan(
        &unit,
        json!({"policy": ["SomeTime"], "accounting": {"work_units": 1}}),
    );
    assert_eq!(exit, 22, "{limited}");
    assert_eq!(limited["synthesis"]["status"], "incomplete", "{limited}");
    assert_eq!(
        limited["synthesis"]["cause"]["setting"], "accounting.work_units",
        "{limited}"
    );
}
