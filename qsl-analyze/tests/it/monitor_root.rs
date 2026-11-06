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
    let source = SourceIdentity::new("agent-ix", "monitor-root");
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

/// The unit with `bump` and `twice` helpers called by the clause's atom
/// (`bump` adding `step`) and an `isGate` helper called by the activation
/// guard (comparing with `gate`). The entries call the generated helpers
/// that call these.
fn helper_unit(step: i64, gate: i64) -> String {
    let helpers = format!(
        "function bump using v(x: Int[0, 1000]): Integer pure {{ x + {step} }}\n\
         function twice using v(x: Int[0, 1000]): Integer pure {{ bump(x) + bump(x) }}\n\
         function isGate using v(x: Int[0, 1000]): Boolean pure {{ x = {gate} }}\n"
    );
    unit_source(
        "always holds(twice(c.versionNumber) = 5)",
        &helpers,
        true,
        "on each (q: M::ConfigVersion) when (isGate(q.versionNumber))",
    )
}

/// The node ids a wire value names through `target` references.
fn targets(value: &serde_json::Value, out: &mut Vec<serde_json::Value>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(target) = map.get("target") {
                out.push(target.clone());
            }
            map.values().for_each(|child| targets(child, out));
        }
        serde_json::Value::Array(items) => items.iter().for_each(|child| targets(child, out)),
        _ => {}
    }
}

/// The call closure the root's key must end with, computed from the wire
/// independently of the writer: the graph position of the body root of every
/// declared function the four entries and the clause reach, ascending by the
/// functions' node ids, one per function.
fn expected_closure(wire: &serde_json::Value) -> Vec<u64> {
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let by_id = |id: &serde_json::Value| nodes.iter().find(|n| &n["node_id"] == id).unwrap();
    let plan = nodes
        .iter()
        .find(|n| n["semantic_form"] == "monitor_plan")
        .unwrap();
    let members = plan["body"]["members"].as_array().unwrap();
    let member =
        |name: &str| members.iter().find(|m| m["name"] == name).unwrap()["value"]["target"].clone();
    let mut pending: Vec<serde_json::Value> =
        ["clause", "init", "on_event", "on_watermark", "on_gap"]
            .iter()
            .map(|name| member(name))
            .collect();
    let mut seen: Vec<serde_json::Value> = Vec::new();
    let mut called: Vec<(CheckedNodeId, serde_json::Value)> = Vec::new();
    while let Some(id) = pending.pop() {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id.clone());
        let node = by_id(&id);
        if node["node_tag"] == "function" && node.get("declaration").is_some() {
            let body = node["body"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["name"] == "body")
                .unwrap()["value"]["target"]
                .clone();
            called.push((serde_json::from_value(id.clone()).unwrap(), body));
        }
        targets(&node["body"], &mut pending);
    }
    called.sort_by(|a, b| a.0.cmp(&b.0));
    called
        .iter()
        .map(|(_, body)| nodes.iter().position(|n| &n["node_id"] == body).unwrap() as u64)
        .collect()
}

/// The positions the root's key path ends with, after the activation
/// capacity text.
fn closure_tail(wire: &serde_json::Value) -> Vec<u64> {
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let plan = nodes
        .iter()
        .find(|n| n["semantic_form"] == "monitor_plan")
        .unwrap();
    let path = plan["node_id"]["path"].as_array().unwrap();
    let capacity = path
        .iter()
        .position(|segment| segment == "8")
        .expect("the activation capacity text ends the premises");
    path[capacity + 1..]
        .iter()
        .map(|segment| segment["node"].as_u64().expect("an index segment"))
        .collect()
}

/// QSpec FR-487-AC-13 and FR-251-AC-38: two plans that differ only in the
/// body of a declared helper have different `PlanId`s and different root
/// keys, whether the helper is called by an atom (directly or through
/// another helper) or by the activation guard, while the atoms' and the
/// guard's own checked bodies are equal; the entries' keys and the helpers'
/// own declaration-path ids are unchanged, the helpers stay declared, an
/// equal helper body gives an equal `PlanId` and root key, and the key's
/// tail is exactly the body roots of the called helpers, once each,
/// ascending by helper node id.
#[trace("TC-1081", "FR-487-AC-13", "FR-251-AC-38")]
#[test]
fn the_root_key_covers_the_bodies_of_called_helpers() {
    let one = root_of(&helper_unit(1, 2), "C", &target()).expect("base");
    let again = root_of(&helper_unit(1, 2), "C", &target()).expect("base again");
    let atom = root_of(&helper_unit(2, 2), "C", &target()).expect("atom helper body");
    let guard = root_of(&helper_unit(1, 3), "C", &target()).expect("guard helper body");
    assert_eq!(
        one.plan.id(),
        again.plan.id(),
        "an equal helper body gives an equal PlanId"
    );
    for (what, other) in [("atom", &atom), ("guard", &guard)] {
        assert_eq!(
            other.plan.program().atoms(),
            one.plan.program().atoms(),
            "the {what} case keeps every atom body"
        );
        assert_eq!(
            other.plan.program().guard(),
            one.plan.program().guard(),
            "the {what} case keeps the guard body"
        );
        assert_ne!(
            other.plan.id(),
            one.plan.id(),
            "a swapped {what} helper body changes the PlanId"
        );
    }
    let [a, a2, b, c] = [&one, &again, &atom, &guard].map(admitted);
    let (ka, ka2, kb, kc) = (keys(&a), keys(&a2), keys(&b), keys(&c));
    assert_eq!(ka, ka2, "an equal helper body gives an equal root key");
    assert_ne!(
        ka.root, kb.root,
        "a swapped atom helper body changes the root key"
    );
    assert_ne!(
        ka.root, kc.root,
        "a swapped guard helper body changes the root key"
    );
    assert_eq!(
        ka.entries, kb.entries,
        "the entries' own keys are unchanged"
    );
    assert_eq!(
        ka.entries, kc.entries,
        "the entries' own keys are unchanged"
    );

    let wire = [&one, &atom, &guard].map(json);
    let declared = |wire: &serde_json::Value, name: &str| -> serde_json::Value {
        let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
        let found: Vec<&serde_json::Value> = nodes
            .iter()
            .filter(|n| n["declaration"]["qualified_name"] == serde_json::json!([name]))
            .collect();
        assert_eq!(found.len(), 1, "{name} stays one declared node");
        assert_eq!(found[0]["node_tag"], "function");
        found[0]["node_id"].clone()
    };
    for name in ["bump", "twice", "isGate"] {
        for other in &wire[1..] {
            assert_eq!(declared(&wire[0], name), declared(other, name), "{name}");
        }
    }
    for wire in &wire {
        let expected = expected_closure(wire);
        let tail = closure_tail(wire);
        assert_eq!(
            tail, expected,
            "the call closure, ascending, one per helper"
        );
        let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
        let root_of_helper = |name: &str| {
            let helper = nodes
                .iter()
                .find(|n| n["declaration"]["qualified_name"] == serde_json::json!([name]))
                .unwrap();
            let body = helper["body"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["name"] == "body")
                .unwrap();
            nodes
                .iter()
                .position(|n| n["node_id"] == body["value"]["target"])
                .unwrap() as u64
        };
        for name in ["bump", "twice", "isGate"] {
            assert!(
                tail.contains(&root_of_helper(name)),
                "{name} is in the closure"
            );
        }
        // An entry calls generated helpers that are declared functions; they
        // are in the closure too, so the closure is longer than the three
        // authored helpers.
        assert!(tail.len() > 3, "{}", tail.len());
    }
    // The order is pinned: a reversed or duplicated tail differs.
    let tail = closure_tail(&wire[0]);
    let mut reversed = tail.clone();
    reversed.reverse();
    assert_ne!(
        tail, reversed,
        "the tail is not palindromic, so order is observable"
    );
    let mut doubled = tail.clone();
    doubled.push(tail[0]);
    assert_ne!(
        doubled,
        expected_closure(&wire[0]),
        "a duplicated entry is not the closure"
    );
}

/// FR-251-AC-42: the plan root's emission retains every witness row of the
/// acquisition it was written from, so a consumer never rebuilds them: the
/// selected model declaration and field rows equal the unit's own
/// acquisition, every retained row names a node of the emitted package, and
/// IR's join accepts the exact rows against the package read back.
#[trace("TC-1081", "FR-251-AC-42")]
#[test]
fn the_plan_root_retains_the_acquisitions_witness_rows() {
    let unit = authored(ROOT_SHAPES[0]);
    let root = root_of(&unit, "C", &target()).expect("the plan root");
    let emission = &root.emission;

    let domain = Domain::config_version();
    let packages =
        qsl_semantics::model::intake::package_input([domain.document.as_slice()]).unwrap();
    let source = SourceIdentity::new("agent-ix", "monitor-root");
    let cancel = Cancel::new();
    let parsed = qsl_replay::spine::parse(
        &qsl_replay::spine::ParseRequest {
            source: &source,
            path: "monitor.native",
            bytes: unit.as_bytes(),
        },
        SpineLimits::default().source,
        &cancel,
    )
    .map_err(|_| "parses")
    .expect("the stage succeeds")
    .into_value();
    let selected =
        qsl_replay::spine::select(&parsed, &packages, SpineLimits::default().model, &cancel)
            .map_err(|_| "selects")
            .expect("the stage succeeds")
            .into_value();
    let acquisition = qsl_replay::spine::check(
        &parsed,
        "fixture".to_owned(),
        &selected,
        &qsl_replay::spine::DependencyInput::default(),
        SpineLimits::default(),
        &cancel,
    )
    .map_err(|_| "checks")
    .expect("the stage succeeds")
    .into_value();
    let graph = acquisition.package().graph();
    assert!(!graph.selected_exact_declaration_rows().is_empty());
    assert_eq!(
        emission.selected_exact_declaration_rows(),
        graph.selected_exact_declaration_rows()
    );
    assert_eq!(
        emission.selected_exact_field_rows(),
        graph.selected_exact_field_rows()
    );
    assert!(!emission.selected_source_unit_rows().is_empty());
    assert!(!emission.equality_producer_identity_rows().is_empty());
    assert!(!emission.equality_fact_witness_rows().is_empty());

    let package = admitted(&root);
    let wire = json(&root);
    let ids: Vec<&serde_json::Value> = wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| &node["node_id"])
        .collect();
    let known = |id: &CheckedNodeId| ids.contains(&&serde_json::to_value(id).unwrap());
    for row in emission.selected_source_unit_rows() {
        assert!(known(&row.structural_node), "{row:?}");
    }
    for row in emission.equality_producer_identity_rows() {
        assert!(known(row.checked_node()), "{row:?}");
        assert!(known(&row.source_map_entry().node_id), "{row:?}");
    }
    for row in emission.equality_fact_witness_rows() {
        assert!(known(row.checked_application()), "{row:?}");
        assert!(known(row.structural_node()), "{row:?}");
    }
    quire_contract_model::SelectedExactIdentities::from_stage_rows(
        &package,
        emission.selected_exact_declaration_rows().iter().cloned(),
        emission.selected_exact_field_rows().iter().cloned(),
    )
    .expect("IR joins the retained exact rows to the plan-root package");
}

fn ir_source_record_rows(root: &PlanRoot) -> Vec<quire_contract_model::SelectedSourceRecordRow> {
    root.emission
        .selected_source_record_rows()
        .iter()
        .map(|row| quire_contract_model::SelectedSourceRecordRow {
            structural_node: row.structural_node.clone(),
            source_authority: row.source_authority.clone(),
            source_identity: row.source_identity.clone(),
            qualified_declaration: row.qualified_declaration.clone(),
            record_key: row.record_key.clone(),
            members: row
                .members
                .iter()
                .map(
                    |member| quire_contract_model::SelectedSourceRecordMemberRow {
                        original_member_identity: member.original_member_identity.clone(),
                        admitted_name: member.admitted_name.clone(),
                        presence: member.presence,
                        type_node: member.type_node.clone(),
                        value_type: member.value_type.clone(),
                    },
                )
                .collect(),
        })
        .collect()
}

/// FR-251-AC-44 / TC-1081 step 28: the retained source record rows name each
/// member by the binding the plan-root package holds after the writer's field
/// renames (`origin_tick` as `origin`, `amount` as `value`), keep the declared
/// field name, and the strict reader's source-record join accepts them; a row
/// that keeps the base name is refused.
#[trace("TC-1081", "FR-251-AC-44")]
#[test]
fn retained_source_record_rows_name_the_final_bindings_and_the_ir_join_accepts_them() {
    use quire_contract_model::{ExactTypeProjectionErrorCode, SelectedExactIdentities};

    let root = root_of(&authored(ROOT_SHAPES[0]), "C", &target()).expect("the plan root");
    let package = admitted(&root);
    let rows = ir_source_record_rows(&root);
    let named = |record: &str| -> Vec<(String, String)> {
        let row = rows
            .iter()
            .find(|row| row.qualified_declaration.last().map(AsRef::as_ref) == Some(record))
            .unwrap_or_else(|| panic!("a row for {record}"));
        row.members
            .iter()
            .map(|m| {
                (
                    m.original_member_identity.to_string(),
                    m.admitted_name.to_string(),
                )
            })
            .collect()
    };
    let pairs = |names: &[(&str, &str)]| -> Vec<(String, String)> {
        names
            .iter()
            .map(|(a, b)| ((*a).into(), (*b).into()))
            .collect()
    };
    assert_eq!(
        named("Instance"),
        pairs(&[
            ("subject", "subject"),
            ("subformula", "subformula"),
            ("origin_tick", "origin")
        ])
    );
    assert_eq!(
        named("Activation"),
        pairs(&[
            ("subformula", "subformula"),
            ("origin_tick", "origin"),
            ("status", "status")
        ])
    );
    assert_eq!(
        named("LimitReached"),
        pairs(&[
            ("limit", "limit"),
            ("amount", "value"),
            ("setting", "setting")
        ])
    );

    let joined = SelectedExactIdentities::empty()
        .with_source_record_rows(&package, rows.clone(), 10_000_000)
        .expect("IR joins the retained source record rows");
    assert_eq!(joined.source_record_rows().count(), rows.len());
    for row in &rows {
        let projected = package
            .project_exact_type(&row.structural_node, &joined, 10_000_000)
            .expect("the record projects");
        assert_eq!(projected.source_record_view(), Some(row));
    }

    // The base name is refused: each renamed member, one at a time.
    for (record, member) in [("Activation", 1), ("Instance", 2), ("LimitReached", 1)] {
        let mut stale = rows.clone();
        let row = stale
            .iter_mut()
            .find(|row| row.qualified_declaration.last().map(AsRef::as_ref) == Some(record))
            .unwrap();
        let original = row.members[member].original_member_identity.clone();
        assert_ne!(original, row.members[member].admitted_name);
        row.members[member].admitted_name = original;
        let error = SelectedExactIdentities::empty()
            .with_source_record_rows(&package, stale, 10_000_000)
            .expect_err("a base name is not the final binding name");
        assert_eq!(
            error.code,
            ExactTypeProjectionErrorCode::MismatchedIdentityWitness
        );
    }
}

/// FR-251-AC-43 / TC-1081 step 27: the plan's subject (clause node and claim
/// occurrence) selects, through the returned emission's temporal binding
/// rows, the clause declaration and claim occurrence the plan-root package
/// holds, as the strict reader admits it: the declaration the plan root's
/// `clause` member names and the declaration of that occurrence's name row.
#[trace("TC-1081", "FR-251-AC-43")]
#[test]
fn the_subject_occurrence_selects_the_emitted_clause_claim_after_the_writer_rekey() {
    let second = "temporal D using t over (c: M::ConfigVersion) clock \"model-time\" on origin { eventually[0 ms, 3 ms] holds(c.versionNumber = 3) }";
    let unit = unit_source(ROOT_SHAPES[0], second, true, "on origin");
    let mut selected = Vec::new();
    for clause in ["C", "D"] {
        let root = root_of(&unit, clause, &target()).expect(clause);
        let subject = root.plan.subject();
        let ordinal = subject.occurrence().origin().ordinal();
        let declaration = root
            .emission
            .temporal_binding_rows()
            .declaration(subject.clause_node(), ordinal)
            .expect("the subject selects an emitted declaration")
            .clone();
        let package = admitted(&root);
        assert_eq!(package.monitor_plans().len(), 1);
        let plan = package.monitor_plans().next().unwrap();
        assert_eq!(plan.clause(), &declaration, "the root binds the selection");
        let names = package.temporal_clause_names().expect("name rows");
        let row = names
            .iter()
            .find(|row| row.declaration == declaration && row.occurrence == ordinal)
            .expect("the selected occurrence has a name row");
        assert_eq!(row.name.as_ref(), subject.clause_name());
        assert_eq!(row.name.as_ref(), clause);
        assert!(
            root.emission
                .temporal_binding_rows()
                .declaration(subject.clause_node(), ordinal + 1)
                .is_none(),
            "an occurrence the package does not hold selects nothing"
        );
        selected.push(declaration);
    }
    assert_ne!(
        selected[0], selected[1],
        "each subject selects its own clause"
    );
}

/// FR-092-AC-22 / TC-1021 step 12: the tuple rows a plan-root emission
/// retains follow the writer's rekey, and the strict reader's source-tuple
/// join accepts them; a row that permutes its positions is refused.
#[trace("TC-1021", "FR-092-AC-22")]
#[test]
fn retained_source_tuple_rows_follow_the_plan_root_and_the_ir_join_accepts_them() {
    use quire_contract_model::{ExactTypeProjectionErrorCode, SelectedExactIdentities};

    let extra = "tuple Pair(Integer, Boolean);\n\
        tuple Triple(Pair, Option<Integer>, Boolean);\n\
        function hold using v(t: Triple): Boolean pure { true }";
    let unit = unit_source(ROOT_SHAPES[0], extra, true, "on origin");
    let root = root_of(&unit, "C", &target()).expect("the plan root");
    let package = admitted(&root);
    let rows = root.emission.selected_source_tuple_rows().to_vec();
    let declared = |name: &str| {
        rows.iter()
            .any(|row| row.qualified_declaration.last().map(AsRef::as_ref) == Some(name))
    };
    assert!(declared("Pair") && declared("Triple"), "{rows:?}");
    let joined = SelectedExactIdentities::empty()
        .with_source_tuple_rows(&package, rows.clone(), 10_000_000)
        .expect("IR joins the retained source tuple rows");
    for row in &rows {
        let projected = package
            .project_exact_type(&row.structural_node, &joined, 10_000_000)
            .expect("the tuple projects");
        assert_eq!(projected.source_tuple_view(), Some(row));
    }
    let triple = rows
        .iter()
        .find(|row| row.qualified_declaration.last().map(AsRef::as_ref) == Some("Triple"))
        .expect("Triple's row");
    let mut permuted = triple.clone();
    permuted.positions.swap(0, 2);
    let error = SelectedExactIdentities::empty()
        .with_source_tuple_rows(&package, [permuted], 10_000_000)
        .expect_err("permuted positions are refused");
    assert_eq!(
        error.code,
        ExactTypeProjectionErrorCode::MismatchedIdentityWitness
    );
}
