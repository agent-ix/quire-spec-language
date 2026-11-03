// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251 "Plan root" and FR-487: the public handoff from an authored unit to
//! its tick-monitor plan-root package, read back through Contract IR's strict
//! reader.

use ix_trace_rs::trace;
use qsl_analyze::monitor_root::{emit_plan_root, PlanRoot, PlanRootRefusal, PlanRootRequest};
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::SpineLimits;
use qsl_replay::{MonitorTarget, TimingObligationSettings};
use qsl_semantics::check::{CheckCause, TemporalCause};
use qsl_semantics::library::LibraryName;
use quire_contract_model::{
    read_checked_package, CheckedNodeId, CheckedPackageDispatchResult, CheckedPackageEvidence,
    CheckedPackageReadLimits, CheckedPackageV2,
};
use quire_exact::{Cancel, Integer, Rational};

use crate::monitor_entries::{microsecond_target, ROOT_SHAPES};
use crate::monitor_plan::{ratio, unit_source};
use crate::units::Domain;

/// Read limits roomy enough for the program's checked closure.
pub(crate) const LIMITS: CheckedPackageReadLimits = CheckedPackageReadLimits {
    bytes: 1 << 26,
    nodes: 1_000_000,
    edges: 10_000_000,
    occurrences: 10_000_000,
    diagnostics: 10_000,
    work: 1_000_000_000,
};

pub(crate) fn target() -> MonitorTarget {
    microsecond_target(ratio(0, 1), 2, 12, 3600)
}

pub(crate) fn root_of(
    unit: &str,
    clause: &str,
    target: &MonitorTarget,
) -> Result<PlanRoot, PlanRootRefusal> {
    let domain = Domain::config_version();
    let packages =
        qsl_semantics::model::intake::package_input([domain.document.as_slice()]).unwrap();
    let source = SourceIdentity::new("agent-ix", "monitor-root", "git", "1");
    let name = LibraryName::new("fixture").unwrap();
    emit_plan_root(
        &PlanRootRequest {
            source: &source,
            path: "monitor.native",
            bytes: unit.as_bytes(),
            packages: &packages,
            package_name: &name,
            limits: SpineLimits::default(),
            clause,
            target,
            settings: &TimingObligationSettings::default(),
        },
        &Cancel::new(),
    )
}

pub(crate) fn authored(shape: &str) -> String {
    unit_source(shape, "", true, "on origin")
}

/// The package read back through Contract IR's strict reader, with the domain
/// package document its model selects supplied.
pub(crate) fn read_back(bytes: &[u8], evidence: &CheckedPackageEvidence) -> Box<CheckedPackageV2> {
    let domain = Domain::config_version();
    let mut evidence = evidence.clone();
    // A document the evidence already holds is the same document.
    let _ = evidence.insert_domain_package_document(domain.package, domain.document.clone());
    match read_checked_package(bytes, LIMITS, &evidence) {
        CheckedPackageDispatchResult::AdmittedV2(package) => package,
        other => panic!("the strict reader did not admit the plan root: {other:?}"),
    }
}

fn admitted(root: &PlanRoot) -> Box<CheckedPackageV2> {
    read_back(root.emission.package().bytes(), root.emission.evidence())
}

fn json(root: &PlanRoot) -> serde_json::Value {
    serde_json::from_slice(root.emission.package().bytes()).unwrap()
}

/// The first of each kind of the plan root's keys: the root and its four
/// entries, as the strict reader derives them.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Keys {
    pub(crate) root: CheckedNodeId,
    pub(crate) entries: [CheckedNodeId; 4],
}

pub(crate) fn keys(package: &CheckedPackageV2) -> Keys {
    assert_eq!(package.monitor_plans().len(), 1);
    let plan = package.monitor_plans().next().unwrap();
    Keys {
        root: plan.structural_key().clone(),
        entries: [
            plan.init().structural_key().clone(),
            plan.on_event().structural_key().clone(),
            plan.on_watermark().structural_key().clone(),
            plan.on_gap().structural_key().clone(),
        ],
    }
}

/// FR-251 "Plan root" / FR-251-AC-9 / TC-1081 step 6: the unit is taken to
/// one plan-root node whose premises are the target's, with the four entries
/// as declaration-less function nodes, and the strict reader admits it.
#[trace("TC-1081", "FR-251-AC-9")]
#[test]
fn the_authored_unit_is_taken_to_a_plan_root_the_strict_reader_admits() {
    let root = root_of(&authored(ROOT_SHAPES[0]), "C", &target()).expect("the plan root");
    let package = admitted(&root);
    assert_eq!(package.monitor_plans().len(), 1);
    let view = package.monitor_plans().next().unwrap();
    assert_eq!(view.counter(), "counter");
    assert_eq!(view.counter_width_bits(), 12);
    assert_eq!(view.max_reading_gap(), 3600);
    assert_eq!(view.activation_capacity(), "8");
    assert_eq!(view.max_event_rate().numerator.as_ref(), "1");
    assert_eq!(view.max_event_rate().denominator.as_ref(), "500");
    assert_eq!(view.uncertainty().numerator.as_ref(), "0");
    let buffers: Vec<String> = view
        .buffers()
        .map(|buffer| buffer.capacity().to_owned())
        .collect();
    let expected: Vec<String> = root
        .plan
        .buffers()
        .iter()
        .map(|(_, capacity)| capacity.to_string())
        .collect();
    assert_eq!(buffers, expected);

    let wire = json(&root);
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let roots: Vec<&serde_json::Value> = nodes
        .iter()
        .filter(|n| n["node_tag"] == "temporal" && n["semantic_form"] == "monitor_plan")
        .collect();
    assert_eq!(roots.len(), 1);
    let members = roots[0]["body"]["members"].as_array().unwrap();
    let names: Vec<&str> = members
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, qsl_replay::PLAN_ROOT_BINDINGS.to_vec());
    for name in ["init", "on_event", "on_watermark", "on_gap"] {
        let member = members.iter().find(|m| m["name"] == name).unwrap();
        let entry = nodes
            .iter()
            .find(|n| n["node_id"] == member["value"]["target"])
            .unwrap();
        assert_eq!(entry["node_tag"], "function");
        assert!(entry.get("declaration").is_none() && entry.get("owner").is_none());
    }
    // FR-487 field spellings.
    let fields = |name: &str| -> Vec<String> {
        let record = nodes
            .iter()
            .find(|n| n["declaration"]["qualified_name"] == serde_json::json!([name]))
            .unwrap();
        record["body"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["name"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(fields("Instance"), ["subject", "subformula", "origin"]);
    assert_eq!(fields("Activation"), ["subformula", "origin", "status"]);
    assert_eq!(fields("LimitReached"), ["limit", "value", "setting"]);
    assert_eq!(fields("ActStatus")[0], "position");
    assert_eq!(
        fields("State"),
        [
            "now",
            "history_start",
            "buffer_0",
            "activations",
            "event_position"
        ]
    );
}

/// The root binds the plan the producer derived: the same plan the unit's
/// clause and the target derive again, its obligations alongside.
#[trace("TC-1081", "FR-251-AC-9")]
#[test]
fn the_returned_plan_is_the_one_the_root_carries() {
    let unit = authored(ROOT_SHAPES[0]);
    let root = root_of(&unit, "C", &target()).unwrap();
    let again = root_of(&unit, "C", &target()).unwrap();
    assert_eq!(root.plan, again.plan);
    assert_eq!(root.plan.id(), again.plan.id());
    assert_eq!(root.plan.root_premises().counter, "counter");
    assert_eq!(root.obligations.len(), 2);
}

/// Emitting the same unit twice gives byte-identical packages, and the
/// strict reader derives the same keys from both.
#[trace("TC-1081", "FR-251-AC-9")]
#[test]
fn emitting_twice_gives_identical_bytes_and_keys() {
    let unit = authored(ROOT_SHAPES[0]);
    let first = root_of(&unit, "C", &target()).unwrap();
    let second = root_of(&unit, "C", &target()).unwrap();
    assert_eq!(
        first.emission.package().bytes(),
        second.emission.package().bytes()
    );
    assert_eq!(keys(&admitted(&first)), keys(&admitted(&second)));
}

/// The clause of the root, named by the operator that opens its formula.
fn clause_operator(root: &PlanRoot) -> String {
    let wire = json(root);
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let plan = nodes
        .iter()
        .find(|n| n["semantic_form"] == "monitor_plan")
        .unwrap();
    let clause_id = &plan["body"]["members"][0]["value"]["target"];
    let clause = nodes.iter().find(|n| &n["node_id"] == clause_id).unwrap();
    assert_eq!(clause["semantic_form"], "temporal_clause");
    let formula = clause["body"]["arguments"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    formula["target"]["path"][3].as_str().unwrap().to_owned()
}

/// FND: a package with two temporal clauses binds the one the request
/// names, by its identity, whichever comes first in the graph.
#[test]
fn a_package_with_two_clauses_binds_the_selected_one() {
    let second = "temporal D using t over (c: M::ConfigVersion) clock \"model-time\" on origin { eventually[0 ms, 3 ms] holds(c.versionNumber = 3) }";
    let unit = unit_source(ROOT_SHAPES[0], second, true, "on origin");
    let first = root_of(&unit, "C", &target()).expect("C");
    let other = root_of(&unit, "D", &target()).expect("D");
    assert_eq!(clause_operator(&first), "quire.op.temporal.always");
    assert_eq!(clause_operator(&other), "quire.op.temporal.eventually");
    let both = [&first, &other].map(admitted);
    assert_ne!(keys(&both[0]).root, keys(&both[1]).root);
}

#[test]
fn a_clause_the_unit_does_not_have_is_refused() {
    let refusal = root_of(&authored(ROOT_SHAPES[0]), "Missing", &target()).unwrap_err();
    assert!(
        matches!(&refusal, PlanRootRefusal::UnknownClause { clause } if clause == "Missing"),
        "{refusal:?}"
    );
}

/// A unit that declares one of the generated names is refused rather than
/// bound to the wrong declaration: the entries are checked in the unit, so
/// the checker sees the clash.
#[test]
fn a_user_declaration_of_a_generated_name_is_refused() {
    for name in ["State", "Result", "Inputs", "Scope", "monitorInit"] {
        let extra = if name == "monitorInit" {
            format!("function {name} using v(): Integer pure {{ 1 }}")
        } else {
            format!("record {name} {{ x: Integer; }}")
        };
        let unit = unit_source(ROOT_SHAPES[0], &extra, true, "on origin");
        let refusal = root_of(&unit, "C", &target()).unwrap_err();
        assert!(
            matches!(refusal, PlanRootRefusal::FrontEnd(_)),
            "{name}: {refusal:?}"
        );
    }
}

/// FR-251 "Fault boundary": a clause with no complete program derives no
/// plan, so no package is produced.
#[test]
fn a_clause_without_a_complete_program_derives_no_plan() {
    use qsl_replay::MonitorPlanRefusal as Refusal;
    for shape in [
        "once holds(c.versionNumber = 3)",
        "historically holds(c.versionNumber = 3)",
        "holds(c.versionNumber = 2) since holds(c.versionNumber = 3)",
    ] {
        let refusal = root_of(&authored(shape), "C", &target()).unwrap_err();
        assert!(
            matches!(
                refusal,
                PlanRootRefusal::Plan(Refusal::UnboundedPast { .. })
            ),
            "{shape}: {refusal:?}"
        );
    }
    for shape in [
        "always eventually holds(c.versionNumber = 3)",
        "eventually always holds(c.versionNumber = 3)",
    ] {
        let refusal = root_of(&authored(shape), "C", &target()).unwrap_err();
        assert!(
            matches!(
                refusal,
                PlanRootRefusal::Plan(Refusal::UnboundedOverUnbounded { .. })
            ),
            "{shape}: {refusal:?}"
        );
    }
}

/// Ticks beyond the program's integers refuse with the operator named.
#[test]
fn ticks_beyond_the_program_integers_derive_no_plan() {
    use qsl_replay::MonitorPlanRefusal as Refusal;
    let huge = format!(
        "eventually[0 ms, 1{} ms] holds(c.versionNumber = 3)",
        "0".repeat(40)
    );
    let tiny_rate = MonitorTarget {
        max_event_rate: Some(
            Rational::new(Integer::from(1_i64), Integer::from(10_i128.pow(30))).expect("nonzero"),
        ),
        ..target()
    };
    let refusal = root_of(&authored(&huge), "C", &tiny_rate).unwrap_err();
    assert!(
        matches!(refusal, PlanRootRefusal::Plan(Refusal::TickRange { .. })),
        "{refusal:?}"
    );
}

/// The front end refuses an aggregate atom in a clause over a model
/// subject, and under the timed profile no other subject is admitted, so no
/// timed clause reaches the entries with one.
#[test]
fn a_timed_clause_cannot_hold_an_aggregate_atom() {
    let refusal = root_of(
        &authored("eventually[0 ms, 3 ms] count over past[0 ms, 3 ms] >= 1"),
        "C",
        &target(),
    )
    .unwrap_err();
    let PlanRootRefusal::FrontEnd(front) = refusal else {
        panic!("a front-end refusal");
    };
    let qsl_replay::ReplayRefusal::Recompile(compile) = *front else {
        panic!("a recompile refusal");
    };
    let qsl_replay::spine::CompileRefusal::Check { refusals, .. } = *compile else {
        panic!("a check refusal");
    };
    let CheckCause::Temporal(cause) = &refusals[0].cause else {
        panic!("a temporal refusal: {:?}", refusals[0].cause);
    };
    assert!(
        matches!(**cause, TemporalCause::AggregateModelSubject { .. }),
        "{cause:?}"
    );
}

/// A function that shares a generated type's name leaves the name declared
/// twice; no plan root is written (the strict reader refuses the base, or the
/// writer refuses the ambiguous name).
#[test]
fn a_function_named_like_a_generated_type_writes_no_plan_root() {
    let unit = unit_source(
        ROOT_SHAPES[0],
        "function State using v(): Integer pure { 1 }",
        true,
        "on origin",
    );
    let refusal = root_of(&unit, "C", &target()).unwrap_err();
    assert!(matches!(refusal, PlanRootRefusal::Write(_)), "{refusal:?}");
}
