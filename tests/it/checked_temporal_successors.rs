// SPDX-License-Identifier: AGPL-3.0-or-later
//! Successor coverage for FR-043/FR-044/FR-045 on checked-package admission.

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::DependencyInput;
use quire_spec_language::temporal::{
    self, Basis, Closure, Completeness, Execution, Position, Truth,
};
use std::collections::BTreeMap;

fn temporal_source(body: &str, profile: &str) -> String {
    format!(
        r#"
language "ix:native" edition "1-draft";
profile v = "quire.model.complete/v1";
profile t = "{profile}";
model M = "acme/orders";
temporal Due using t over (view: M::Widget) clock "orders" on origin {{
    {body}
}}
"#
    )
}

fn checked(body: &str, profile: &str) -> qsl_replay::spine::CheckedUnit {
    let packages =
        qsl_semantics::model::intake::package_input([tests_fixture_spine_model().as_slice()])
            .expect("checked model fixture");
    crate::support::front_end::checked(
        SourceIdentity::new("agent-ix", "test:checked-temporal", "fixture", "1"),
        "checked-temporal.native",
        temporal_source(body, profile).as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
    .expect("S1-S4 checked temporal package")
}

fn tests_fixture_spine_model() -> Vec<u8> {
    include_bytes!("../fixtures/spine-model.semantic-ir.json").to_vec()
}

fn origin_trace() -> temporal::Trace {
    temporal::Trace {
        clock: temporal::ClockBinding {
            name: "orders".into(),
            profile_identity: temporal::EVENT_POSITION.into(),
            parameters: BTreeMap::new(),
        },
        positions: vec![Position {
            coordinate: 0,
            order: None,
            valuations: BTreeMap::new(),
        }],
        anchor: "origin".into(),
        triggers: vec![temporal::Trigger {
            identity: "execution-origin:one".into(),
            receipt: "delivery:one".into(),
            anchor: "origin".into(),
            payload: "origin".into(),
            guard: None,
            captures: Vec::new(),
        }],
        trigger_evidence: temporal::Evidence::Admitted,
        trigger_scope: Closure::Closed,
        decision_scope: Closure::Closed,
        surrounding_execution: Closure::Closed,
        execution: Execution::Completed,
        completeness: Completeness::Complete,
        authoritative_origin: true,
        watermark: 1,
        evicted: Vec::new(),
    }
}

/// FR-043/044: TC-122's evaluator runs from the checked clause formula, with
/// the package's activation and clock authority and no emitted protocol bytes.
#[trace("TC-122", "TC-123", "FR-043-AC-1", "FR-044-AC-1")]
#[test]
fn checked_package_formula_uses_the_shared_bounded_evaluator() {
    let checked = checked("always[0,1] true", temporal::EVENT_POSITION);
    let report = temporal::evaluate_checked(
        checked.package(),
        0,
        &origin_trace(),
        temporal::Limits::default(),
    );
    let assessed = match &report.result().expect("checked formula assessed")[0] {
        temporal::Obligation::Assessed(assessment) => assessment,
        other => panic!("expected active checked-package obligation, got {other:?}"),
    };
    assert_eq!(assessed.truth, Some(Truth::True));
    assert_eq!(assessed.basis, Some(Basis::ClosedScope));
}

/// FR-045: TC-125 retains the selected checked clause and classifies from its
/// reachable operator graph and surrounding closure.
#[trace("TC-125", "FR-045-AC-1")]
#[test]
fn checked_mapping_retains_the_checked_selection() {
    let checked = checked("always[0,1] true", temporal::EVENT_POSITION);
    let classification = temporal::mapping_support_checked(checked.package(), 0, Closure::Open)
        .expect("checked temporal mapping classified");
    assert_eq!(classification.retained.name, "Due");
    assert_eq!(
        classification.retained.profile.identity(),
        temporal::EVENT_POSITION
    );
    assert!(matches!(
        classification.support,
        temporal::Support::Supported {
            target: temporal::Target::OnlinePrefix,
            ..
        }
    ));
}

/// FR-043: checked atomic valuations remain distinct from constants, and the
/// report retains only positions in the exact decision support.
#[trace("TC-122", "TC-123", "FR-043-AC-4", "FR-043-AC-5")]
#[test]
fn checked_atom_has_exact_decision_support_and_false_extension() {
    let checked = checked("always[0,1] holds(view.code > 0)", temporal::EVENT_POSITION);
    let clause = &checked.package().graph().temporal_clauses()[0];
    let atom = clause
        .clause()
        .formula()
        .nodes()
        .iter()
        .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
        .expect("checked holds atom") as u32;
    let mut input = origin_trace();
    input.positions[0].valuations.insert(atom, true);
    let report =
        temporal::evaluate_checked(checked.package(), 0, &input, temporal::Limits::default());
    let assessed = match &report.result().expect("checked atom assessed")[0] {
        temporal::Obligation::Assessed(assessment) => assessment,
        other => panic!("expected assessed checked clause, got {other:?}"),
    };
    assert_eq!(assessed.truth, Some(Truth::False));
    assert_eq!(assessed.support, vec![0]);
}

/// FR-045: the checked classifier accumulates finite-window and mixed-direction
/// limitations and retains the exact profile selection.
#[trace("TC-125", "FR-045-AC-2", "FR-045-AC-3")]
#[test]
fn checked_mapping_accumulates_finite_window_and_direction_limits() {
    let checked = checked(
        "once[0,1] true and eventually[0,1] true",
        "quire.temporal.timestamped-event.finite-window/v1",
    );
    let classification = temporal::mapping_support_checked(checked.package(), 0, Closure::Closed)
        .expect("checked temporal mapping classified");
    assert_eq!(
        classification.support,
        temporal::Support::Unsupported {
            dimensions: vec![
                temporal::Unmatched::FiniteWindow,
                temporal::Unmatched::MixedDirection,
            ],
        }
    );
    assert_eq!(
        classification.retained.profile.identity(),
        temporal::TIMESTAMPED_WINDOW
    );
}
