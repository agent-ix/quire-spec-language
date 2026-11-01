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
    temporal_source_with_activation(body, profile, "on origin")
}

fn temporal_source_with_activation(body: &str, profile: &str, activation: &str) -> String {
    format!(
        r#"
language "ix:native" edition "1-draft";
profile v = "quire.model.complete/v1";
profile t = "{profile}";
model M = "acme/orders";
temporal Due using t over (view: M::Widget) clock "orders" {activation} {{
    {body}
}}
"#
    )
}

fn checked_activation(
    identity: &str,
    body: &str,
    activation: &str,
) -> qsl_replay::spine::CheckedUnit {
    try_checked_activation(identity, body, activation).expect("S1-S4 checked activation package")
}

fn try_checked_activation(
    identity: &str,
    body: &str,
    activation: &str,
) -> Result<qsl_replay::spine::CheckedUnit, Box<qsl_replay::spine::CompileRefusal>> {
    let packages =
        qsl_semantics::model::intake::package_input([tests_fixture_spine_model().as_slice()])
            .expect("checked model fixture");
    let source = temporal_source_with_activation(body, temporal::EVENT_POSITION, activation);
    crate::support::front_end::checked(
        SourceIdentity::new("agent-ix", identity, "fixture", "1"),
        "checked-temporal-activation.native",
        source.as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
}

fn checked(body: &str, profile: &str) -> qsl_replay::spine::CheckedUnit {
    checked_named("test:checked-temporal", body, profile)
}

fn checked_named(identity: &str, body: &str, profile: &str) -> qsl_replay::spine::CheckedUnit {
    let packages =
        qsl_semantics::model::intake::package_input([tests_fixture_spine_model().as_slice()])
            .expect("checked model fixture");
    crate::support::front_end::checked(
        SourceIdentity::new("agent-ix", identity, "fixture", "1"),
        "checked-temporal.native",
        temporal_source(body, profile).as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
    .expect("S1-S4 checked temporal package")
}

fn checked_timed_mapping() -> qsl_replay::spine::CheckedUnit {
    let packages = qsl_semantics::model::intake::package_input([
        crate::support::config_version_spine::spine::DOMAIN_PACKAGE.as_bytes(),
    ])
    .expect("timed ConfigVersion model fixture");
    let source = r#"
language "ix:native" edition "1-draft";
profile v = "quire.model.complete/v1";
profile tc = "quire.temporal.complete/v1";
profile t = "quire.temporal.timed/v1";
model Config = "example/config-version";
dimension Time;
unit s : Time = rational(1, 1);
unit ms : Time = rational(1, 1000) * s;
time Config dense unit ms;
temporal Due using t over (view: Config::ConfigVersion) clock "model-time" on origin {
    eventually[0 ms, 1 ms] true
}
"#;
    crate::support::front_end::checked(
        SourceIdentity::new("agent-ix", "test:checked-timed-mapping", "fixture", "1"),
        "checked-timed-mapping.native",
        source.as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
    .expect("S1-S4 admits a checked timed clause")
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
            instance_valuations: BTreeMap::new(),
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
#[trace("TC-1277", "TC-1278", "FR-043-AC-1", "FR-044-AC-1")]
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

/// FR-043-AC-1/2: the selected profile controls closed-boundary semantics and
/// is retained in the report premises, even when formula and interval bytes
/// are identical.
#[trace("TC-1277", "FR-043-AC-1", "FR-043-AC-2")]
#[test]
fn checked_profiles_keep_distinct_closed_boundary_results() {
    let mut reports = Vec::new();
    for (profile, expected) in [
        (temporal::EVENT_POSITION, Truth::False),
        (temporal::FIXED_SAMPLE, Truth::False),
        (temporal::TIMESTAMPED_WINDOW, Truth::True),
    ] {
        let checked = checked("always[0,1] holds(view.code > 0)", profile);
        let clause = &checked.package().graph().temporal_clauses()[0];
        let leaf = clause
            .clause()
            .formula()
            .nodes()
            .iter()
            .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
            .and_then(|index| u32::try_from(index).ok())
            .expect("checked hold atom");
        let mut input = origin_trace();
        input.clock.profile_identity = profile.into();
        input.watermark = 1;
        input.positions = vec![Position {
            coordinate: 0,
            order: None,
            valuations: BTreeMap::from([(leaf, true)]),
            instance_valuations: BTreeMap::new(),
        }];
        let report =
            temporal::evaluate_checked(checked.package(), 0, &input, temporal::Limits::default());
        let temporal::Obligation::Assessed(assessment) = &report.result().unwrap()[0] else {
            panic!("profile-specific formula assessed");
        };
        assert_eq!(assessment.truth, Some(expected), "{profile}");
        assert_eq!(assessment.premises.profile.identity(), profile);
        reports.push(assessment.premises.profile);
    }
    assert_ne!(reports[0], reports[1]);
    assert_ne!(reports[1], reports[2]);
    assert_ne!(reports[0], reports[2]);
}

/// FR-043-AC-13: identical checked inputs reproduce one assessment, while
/// changed profile, clock, clock-parameter and ceiling premises stay distinct.
#[trace("TC-1277", "FR-043-AC-13")]
#[test]
fn checked_repeated_results_retain_identity_inputs() {
    let checked_unit = checked("always[0,1] true", temporal::EVENT_POSITION);
    let input = origin_trace();
    let first = temporal::evaluate_checked(
        checked_unit.package(),
        0,
        &input,
        temporal::Limits::default(),
    );
    let repeated = temporal::evaluate_checked(
        checked_unit.package(),
        0,
        &input,
        temporal::Limits::default(),
    );
    assert_eq!(first.result(), repeated.result());

    let temporal::Obligation::Assessed(original) = &first.result().unwrap()[0] else {
        panic!("the stable checked result is assessed");
    };
    let mut narrower = temporal::Limits::default();
    narrower.visits -= 1;
    let changed_ceiling = temporal::evaluate_checked(checked_unit.package(), 0, &input, narrower);
    let temporal::Obligation::Assessed(changed_ceiling) = &changed_ceiling.result().unwrap()[0]
    else {
        panic!("the changed ceiling leaves the result assessable");
    };
    assert_ne!(original.premises.limits, changed_ceiling.premises.limits);

    let mut changed_clock = input.clone();
    changed_clock.clock.name = "other-orders".into();
    let changed_clock = temporal::evaluate_checked(
        checked_unit.package(),
        0,
        &changed_clock,
        temporal::Limits::default(),
    );
    assert!(matches!(
        changed_clock.result(),
        Err(temporal::Error::Refused(temporal::Refusal::Binding {
            dimension: temporal::Dimension::Clock,
            ..
        }))
    ));
    assert_ne!(first.result(), changed_clock.result());

    let mut changed_parameters = input.clone();
    changed_parameters.clock.parameters =
        BTreeMap::from([("sequence_authority".into(), "orders-v2".into())]);
    let changed_parameters = temporal::evaluate_checked(
        checked_unit.package(),
        0,
        &changed_parameters,
        temporal::Limits::default(),
    );
    let temporal::Obligation::Assessed(changed_parameters) =
        &changed_parameters.result().unwrap()[0]
    else {
        panic!("changed clock parameters remain retained premises");
    };
    assert_ne!(
        original.premises.clock_parameters,
        changed_parameters.premises.clock_parameters
    );
    assert_eq!(
        changed_parameters.premises.clock_parameters,
        BTreeMap::from([("sequence_authority".into(), "orders-v2".into())])
    );

    let other_profile = checked("always[0,1] true", temporal::TIMESTAMPED_WINDOW);
    let mut other_input = input;
    other_input.clock.profile_identity = temporal::TIMESTAMPED_WINDOW.into();
    let changed_profile = temporal::evaluate_checked(
        other_profile.package(),
        0,
        &other_input,
        temporal::Limits::default(),
    );
    let temporal::Obligation::Assessed(changed_profile) = &changed_profile.result().unwrap()[0]
    else {
        panic!("the changed selected profile remains assessable");
    };
    assert_ne!(original.premises.profile, changed_profile.premises.profile);
}

/// FR-043: the checked formula adapter preserves every unary and binary
/// bounded operator and pointwise connective when it enters the shared core.
#[trace("TC-1277", "FR-043-AC-4", "FR-043-AC-6")]
#[test]
fn checked_formula_normalization_preserves_all_operator_families() {
    for (body, expected) in [
        ("eventually[0,1] true", Truth::True),
        ("always[0,1] true", Truth::True),
        ("once[0,1] true", Truth::True),
        ("historically[0,1] true", Truth::True),
        ("true until[0,1] true", Truth::True),
        ("true release[0,1] true", Truth::True),
        ("true since[0,1] true", Truth::True),
        ("true triggered[0,1] true", Truth::True),
        ("not (false or true)", Truth::False),
        ("true and (false implies true)", Truth::True),
    ] {
        let checked = checked(body, temporal::EVENT_POSITION);
        let report = temporal::evaluate_checked(
            checked.package(),
            0,
            &origin_trace(),
            temporal::Limits::default(),
        );
        let temporal::Obligation::Assessed(assessment) = &report.result().unwrap()[0] else {
            panic!("formula assessed: {body}");
        };
        assert_eq!(assessment.truth, Some(expected), "{body}");
    }
}

/// FR-043-AC-6: pointwise negation remains inside each temporal quantifier.
#[trace("TC-1277", "FR-043-AC-6")]
#[test]
fn checked_temporal_negation_is_pointwise_under_quantifiers() {
    for (body, expected) in [
        ("always[0,1] not holds(view.code > 0)", Truth::False),
        ("eventually[0,1] not holds(view.code > 0)", Truth::True),
    ] {
        let checked = checked(body, temporal::EVENT_POSITION);
        let leaf = checked.package().graph().temporal_clauses()[0]
            .clause()
            .formula()
            .nodes()
            .iter()
            .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
            .and_then(|index| u32::try_from(index).ok())
            .expect("checked hold atom");
        let mut input = origin_trace();
        input.positions[0].valuations.insert(leaf, true);
        let report =
            temporal::evaluate_checked(checked.package(), 0, &input, temporal::Limits::default());
        let temporal::Obligation::Assessed(assessment) = &report.result().unwrap()[0] else {
            panic!("quantified pointwise negation is assessed: {body}");
        };
        assert_eq!(assessment.truth, Some(expected), "{body}");
    }
}

/// FR-043-AC-5: equal-coordinate participants require one admitted order
/// authority; insertion order is never accepted as the ordering premise.
#[trace("TC-1277", "FR-043-AC-5")]
#[test]
fn checked_order_sensitive_formula_refuses_unordered_equal_coordinates() {
    for operator in ["until", "release", "since", "triggered"] {
        let checked = checked(
            &format!("true {operator}[0,1] true"),
            temporal::EVENT_POSITION,
        );
        let mut input = origin_trace();
        input.positions = [0, 1]
            .into_iter()
            .map(|_| Position {
                coordinate: 1,
                order: None,
                valuations: BTreeMap::new(),
                instance_valuations: BTreeMap::new(),
            })
            .collect();
        let report =
            temporal::evaluate_checked(checked.package(), 0, &input, temporal::Limits::default());
        assert!(
            matches!(
                report.result(),
                Err(temporal::Error::Refused(temporal::Refusal::Order { .. }))
            ),
            "{operator} refuses equal coordinates without admitted order"
        );
        assert_eq!(report.usage().positions, 0, "{operator}");

        for positions in [[1, 2], [2, 1]] {
            input.positions = positions
                .into_iter()
                .map(|key| Position {
                    coordinate: 1,
                    order: Some(temporal::OrderKey {
                        authority: "sequence:orders".into(),
                        key,
                    }),
                    valuations: BTreeMap::new(),
                    instance_valuations: BTreeMap::new(),
                })
                .collect();
            let report = temporal::evaluate_checked(
                checked.package(),
                0,
                &input,
                temporal::Limits::default(),
            );
            assert!(report.result().is_ok(), "{operator} with admitted order");
        }
        input.positions[1].order.as_mut().unwrap().authority = "sequence:foreign".into();
        let report =
            temporal::evaluate_checked(checked.package(), 0, &input, temporal::Limits::default());
        assert!(
            matches!(
                report.result(),
                Err(temporal::Error::Refused(temporal::Refusal::Order { .. }))
            ),
            "{operator} refuses foreign order authorities"
        );
    }
}

/// FR-043-AC-4/5: until evaluates the left operand only inside its selected
/// range before a witness; release is its Boolean dual over the same offsets.
#[trace("TC-1277", "FR-043-AC-4", "FR-043-AC-5")]
#[test]
fn checked_until_and_release_share_the_inclusive_lower_bound_convention() {
    for (operator, expected) in [("until", Truth::True), ("release", Truth::False)] {
        let checked = checked(
            &format!("holds(view.code > 0) {operator}[1,2] holds(view.code = 2)"),
            temporal::EVENT_POSITION,
        );
        let clause = &checked.package().graph().temporal_clauses()[0];
        let leaves: Vec<_> = clause
            .clause()
            .formula()
            .nodes()
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                matches!(node, qsl_semantics::check::FormulaNode::Holds { .. })
                    .then(|| u32::try_from(index).ok())
                    .flatten()
            })
            .collect();
        assert_eq!(leaves.len(), 2);
        let mut input = origin_trace();
        input.positions = [(0, false, false), (1, false, true), (2, false, false)]
            .into_iter()
            .map(|(coordinate, left, right)| Position {
                coordinate,
                order: None,
                valuations: BTreeMap::from([(leaves[0], left), (leaves[1], right)]),
                instance_valuations: BTreeMap::new(),
            })
            .collect();
        let report =
            temporal::evaluate_checked(checked.package(), 0, &input, temporal::Limits::default());
        let temporal::Obligation::Assessed(assessment) = &report.result().unwrap()[0] else {
            panic!("relation assessed");
        };
        assert_eq!(assessment.truth, Some(expected), "{operator}");
    }
}

/// FR-043-AC-4: a zero-width endpoint is inclusive, strong previous is false
/// at the authoritative origin, and release/triggered agree with their duals.
#[trace("TC-1277", "FR-043-AC-4")]
#[test]
fn checked_temporal_zero_width_previous_and_duals_are_preserved() {
    let first_checked = checked(
        "eventually[1,1] holds(view.code > 0)",
        temporal::EVENT_POSITION,
    );
    let clause = &first_checked.package().graph().temporal_clauses()[0];
    let leaf = clause
        .clause()
        .formula()
        .nodes()
        .iter()
        .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
        .and_then(|index| u32::try_from(index).ok())
        .expect("checked hold atom");
    let mut input = origin_trace();
    input.positions = [(0, false), (1, true)]
        .into_iter()
        .map(|(coordinate, holds)| Position {
            coordinate,
            order: None,
            valuations: BTreeMap::from([(leaf, holds)]),
            instance_valuations: BTreeMap::new(),
        })
        .collect();
    let report = temporal::evaluate_checked(
        first_checked.package(),
        0,
        &input,
        temporal::Limits::default(),
    );
    let temporal::Obligation::Assessed(assessment) = &report.result().unwrap()[0] else {
        panic!("inclusive [1,1] endpoint is assessed");
    };
    assert_eq!(assessment.truth, Some(Truth::True));
    assert_eq!(assessment.support, [1]);

    let previous = checked("once[1,1] holds(view.code > 0)", temporal::EVENT_POSITION);
    let previous_clause = &previous.package().graph().temporal_clauses()[0];
    let previous_leaf = previous_clause
        .clause()
        .formula()
        .nodes()
        .iter()
        .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
        .and_then(|index| u32::try_from(index).ok())
        .expect("checked previous hold atom");
    let mut previous_input = origin_trace();
    previous_input.positions[0]
        .valuations
        .insert(previous_leaf, true);
    let report = temporal::evaluate_checked(
        previous.package(),
        0,
        &previous_input,
        temporal::Limits::default(),
    );
    let temporal::Obligation::Assessed(assessment) = &report.result().unwrap()[0] else {
        panic!("strong previous at the authoritative origin is assessed");
    };
    assert_eq!(assessment.truth, Some(Truth::False));

    for (operator, dual) in [
        ("release", "not ((not true) until[1,2] (not false))"),
        ("triggered", "not ((not true) since[1,2] (not false))"),
    ] {
        let direct = checked(
            &format!("true {operator}[1,2] false"),
            temporal::EVENT_POSITION,
        );
        let dual = checked(dual, temporal::EVENT_POSITION);
        let direct_report = temporal::evaluate_checked(
            direct.package(),
            0,
            &origin_trace(),
            temporal::Limits::default(),
        );
        let dual_report = temporal::evaluate_checked(
            dual.package(),
            0,
            &origin_trace(),
            temporal::Limits::default(),
        );
        let truth = |report: &temporal::Report| match &report.result().unwrap()[0] {
            temporal::Obligation::Assessed(assessment) => assessment.truth,
            other => panic!("dual formula has a result: {other:?}"),
        };
        assert_eq!(truth(&direct_report), truth(&dual_report), "{operator}");
    }
}

/// FR-045: TC-125 retains the selected checked clause and classifies from its
/// reachable operator graph and surrounding closure.
#[trace("TC-1279", "FR-045-AC-1", "FR-045-AC-6")]
#[test]
fn checked_mapping_retains_the_checked_selection() {
    let checked = checked("always[0,1] true", temporal::EVENT_POSITION);
    let classification = temporal::mapping_support_checked(checked.package(), 0, Closure::Open)
        .expect("checked temporal mapping classified");
    assert_eq!(classification.retained.name, "Due");
    assert_eq!(
        classification.retained.identity,
        checked.package().graph().temporal_clauses()[0].identity()
    );
    assert_eq!(
        classification.retained.profile.identity(),
        temporal::EVENT_POSITION
    );
    assert_eq!(
        classification.retained.activation,
        qsl_semantics::check::Activation::Origin
    );
    assert!(matches!(
        classification.support,
        temporal::Support::Supported {
            target: temporal::Target::OnlinePrefix,
            ..
        }
    ));
}

/// FR-045: selection failure remains a located refusal and cannot be mistaken
/// for a supported or unsupported mapping classification.
#[trace("TC-1279", "FR-045-AC-6")]
#[test]
fn checked_mapping_refuses_an_unselected_clause_index() {
    let checked = checked("always[0,1] true", temporal::EVENT_POSITION);
    assert!(matches!(
        temporal::mapping_support_checked(checked.package(), usize::MAX, Closure::Closed),
        Err(temporal::Error::Refused(temporal::Refusal::Reference {
            subject: temporal::Subject {
                declaration: usize::MAX,
                ..
            }
        }))
    ));
}

/// FR-043: checked atomic valuations remain distinct from constants, and the
/// report retains only positions in the exact decision support.
#[trace("TC-1277", "FR-043-AC-7")]
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

/// FR-043-AC-7/8: open scopes settle only from decisive support, incomplete
/// input cannot invoke false extension, and a finite empty window settles only
/// once its inclusive deadline is complete.
#[trace("TC-1277", "FR-043-AC-7", "FR-043-AC-8", "FR-043-AC-10")]
#[test]
fn checked_open_incomplete_and_empty_window_outcomes_stay_distinct() {
    let eventual = checked(
        "eventually[0,1] holds(view.code > 0)",
        temporal::EVENT_POSITION,
    );
    let leaf = eventual.package().graph().temporal_clauses()[0]
        .clause()
        .formula()
        .nodes()
        .iter()
        .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
        .and_then(|index| u32::try_from(index).ok())
        .unwrap();
    let mut input = origin_trace();
    input.decision_scope = Closure::Open;
    input.positions[0].valuations.insert(leaf, true);
    let witness =
        temporal::evaluate_checked(eventual.package(), 0, &input, temporal::Limits::default());
    let temporal::Obligation::Assessed(witness) = &witness.result().unwrap()[0] else {
        panic!("decisive witness assessed");
    };
    assert_eq!(witness.truth, Some(Truth::True));
    assert_eq!(witness.basis, Some(Basis::DecisiveWitness));
    assert_eq!(witness.support, [0]);

    let universal = checked("always[0,1] holds(view.code > 0)", temporal::EVENT_POSITION);
    let leaf = universal.package().graph().temporal_clauses()[0]
        .clause()
        .formula()
        .nodes()
        .iter()
        .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
        .and_then(|index| u32::try_from(index).ok())
        .unwrap();
    let mut input = origin_trace();
    input.decision_scope = Closure::Open;
    input.positions[0].valuations.insert(leaf, false);
    let counterexample =
        temporal::evaluate_checked(universal.package(), 0, &input, temporal::Limits::default());
    let temporal::Obligation::Assessed(counterexample) = &counterexample.result().unwrap()[0]
    else {
        panic!("decisive counterexample assessed");
    };
    assert_eq!(counterexample.truth, Some(Truth::False));
    assert_eq!(counterexample.basis, Some(Basis::DecisiveCounterexample));
    assert_eq!(counterexample.support, [0]);

    let mut incomplete = origin_trace();
    incomplete.positions.clear();
    incomplete.completeness = Completeness::Incomplete;
    let report = temporal::evaluate_checked(
        universal.package(),
        0,
        &incomplete,
        temporal::Limits::default(),
    );
    let temporal::Obligation::Assessed(incomplete) = &report.result().unwrap()[0] else {
        panic!("incomplete input remains assessed");
    };
    assert_eq!(incomplete.truth, Some(Truth::Pending));
    assert_eq!(incomplete.basis, Some(Basis::Unsettled));

    let mut axes_input = origin_trace();
    axes_input.decision_scope = Closure::Open;
    axes_input.surrounding_execution = Closure::Closed;
    axes_input.execution = Execution::Failed;
    axes_input.completeness = Completeness::Incomplete;
    let report = temporal::evaluate_checked(
        universal.package(),
        0,
        &axes_input,
        temporal::Limits::default(),
    );
    let temporal::Obligation::Assessed(axes) = &report.result().unwrap()[0] else {
        panic!("independent axes remain represented in the checked result");
    };
    assert_eq!(axes.premises.decision_scope, Closure::Open);
    assert_eq!(axes.premises.surrounding_execution, Closure::Closed);
    assert_eq!(axes.premises.execution, Execution::Failed);
    assert_eq!(axes.premises.completeness, Completeness::Incomplete);

    for (body, expected) in [
        ("always[1,1] true", Truth::True),
        ("eventually[1,1] true", Truth::False),
    ] {
        let finite = checked(body, temporal::TIMESTAMPED_WINDOW);
        let mut trace = origin_trace();
        trace.clock.profile_identity = temporal::TIMESTAMPED_WINDOW.into();
        trace.positions.clear();
        trace.watermark = 1;
        let report =
            temporal::evaluate_checked(finite.package(), 0, &trace, temporal::Limits::default());
        let temporal::Obligation::Assessed(empty_window) = &report.result().unwrap()[0] else {
            panic!("complete finite empty window assessed");
        };
        assert_eq!(empty_window.truth, Some(expected), "{body}");
    }
}

/// FR-043-AC-11: fixed-sample progress settles silence at the inclusive
/// deadline, includes an instant exactly there and excludes later instants;
/// event-position progress alone does not advance during silence.
#[trace("TC-1277", "FR-043-AC-11")]
#[test]
fn checked_watermark_deadline_and_silent_clock_semantics() {
    let fixed = checked(
        "eventually[0,2] holds(view.code > 0)",
        temporal::FIXED_SAMPLE,
    );
    let leaf = fixed.package().graph().temporal_clauses()[0]
        .clause()
        .formula()
        .nodes()
        .iter()
        .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
        .and_then(|index| u32::try_from(index).ok())
        .unwrap();
    let mut silent = origin_trace();
    silent.clock.profile_identity = temporal::FIXED_SAMPLE.into();
    silent.decision_scope = Closure::Open;
    silent.positions.clear();
    silent.watermark = 2;
    let report =
        temporal::evaluate_checked(fixed.package(), 0, &silent, temporal::Limits::default());
    let temporal::Obligation::Assessed(silent) = &report.result().unwrap()[0] else {
        panic!("sample watermark settles a silent bounded obligation");
    };
    assert_eq!(silent.truth, Some(Truth::False));
    assert_eq!(silent.basis, Some(Basis::DecisiveCounterexample));

    let mut at_deadline = origin_trace();
    at_deadline.clock.profile_identity = temporal::FIXED_SAMPLE.into();
    at_deadline.decision_scope = Closure::Open;
    at_deadline.watermark = 1;
    at_deadline.positions = vec![
        Position {
            coordinate: 0,
            order: None,
            valuations: BTreeMap::from([(leaf, false)]),
            instance_valuations: BTreeMap::new(),
        },
        Position {
            coordinate: 1,
            order: None,
            valuations: BTreeMap::from([(leaf, true)]),
            instance_valuations: BTreeMap::new(),
        },
        Position {
            coordinate: 2,
            order: None,
            valuations: BTreeMap::from([(leaf, false)]),
            instance_valuations: BTreeMap::new(),
        },
    ];
    let report = temporal::evaluate_checked(
        fixed.package(),
        0,
        &at_deadline,
        temporal::Limits::default(),
    );
    let temporal::Obligation::Assessed(at_deadline) = &report.result().unwrap()[0] else {
        panic!("the inclusive deadline position participates");
    };
    assert_eq!(at_deadline.truth, Some(Truth::True));
    assert_eq!(at_deadline.basis, Some(Basis::DecisiveWitness));

    let event = checked(
        "eventually[0,2] holds(view.code > 0)",
        temporal::EVENT_POSITION,
    );
    let mut event_silence = origin_trace();
    event_silence.decision_scope = Closure::Open;
    event_silence.positions.clear();
    event_silence.watermark = 2;
    let report = temporal::evaluate_checked(
        event.package(),
        0,
        &event_silence,
        temporal::Limits::default(),
    );
    let temporal::Obligation::Assessed(event_silence) = &report.result().unwrap()[0] else {
        panic!("event-position silence remains assessed");
    };
    assert_eq!(event_silence.truth, Some(Truth::Pending));
    assert_eq!(event_silence.basis, Some(Basis::Unsettled));
}

/// FR-043-AC-9: a past query at an authoritative origin uses the selected
/// boundary rule; the same visible suffix without origin authority is missing
/// history, not false padding.
#[trace("TC-1277", "FR-043-AC-9")]
#[test]
fn checked_past_operator_requires_authoritative_history() {
    let checked = checked("once[1,1] holds(view.code > 0)", temporal::EVENT_POSITION);
    let leaf = checked.package().graph().temporal_clauses()[0]
        .clause()
        .formula()
        .nodes()
        .iter()
        .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
        .and_then(|index| u32::try_from(index).ok())
        .unwrap();
    let mut input = origin_trace();
    input.positions[0].valuations.insert(leaf, true);

    input.authoritative_origin = true;
    let report =
        temporal::evaluate_checked(checked.package(), 0, &input, temporal::Limits::default());
    let temporal::Obligation::Assessed(origin) = &report.result().unwrap()[0] else {
        panic!("authoritative-origin past operator assessed");
    };
    assert_eq!(origin.truth, Some(Truth::False));

    input.authoritative_origin = false;
    let report =
        temporal::evaluate_checked(checked.package(), 0, &input, temporal::Limits::default());
    let temporal::Obligation::Assessed(cutoff) = &report.result().unwrap()[0] else {
        panic!("history-cutoff past operator remains represented");
    };
    assert_eq!(cutoff.truth, None);
    assert_eq!(cutoff.basis, Some(Basis::Unavailable));
    assert_eq!(
        cutoff
            .incomplete
            .as_ref()
            .map(|incomplete| incomplete.dimension),
        Some(temporal::Dimension::History)
    );

    let nested_unit = checked_named(
        "test:checked-temporal-interior-history",
        "eventually[3,3] once[1,2] holds(view.code > 0)",
        temporal::EVENT_POSITION,
    );
    let nested_clause = &nested_unit.package().graph().temporal_clauses()[0];
    let nested_leaf = nested_clause
        .clause()
        .formula()
        .nodes()
        .iter()
        .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
        .and_then(|index| u32::try_from(index).ok())
        .expect("nested checked hold atom");
    let mut interior_gap = origin_trace();
    interior_gap.positions = [(0, true), (1, false), (2, false), (3, false)]
        .into_iter()
        .map(|(coordinate, holds)| Position {
            coordinate,
            order: None,
            valuations: (coordinate != 1)
                .then(|| BTreeMap::from([(nested_leaf, holds)]))
                .unwrap_or_default(),
            instance_valuations: BTreeMap::new(),
        })
        .collect();
    let report = temporal::evaluate_checked(
        nested_unit.package(),
        0,
        &interior_gap,
        temporal::Limits::default(),
    );
    assert!(
        matches!(
            report.result(),
            Err(temporal::Error::Incomplete(temporal::Incomplete {
                dimension: temporal::Dimension::Valuation,
                subject: temporal::Subject {
                    node: Some(node),
                    position: Some(1),
                    ..
                },
            })) if *node == nested_leaf as usize
        ),
        "interior checked-history gap result: {:?}",
        report.result()
    );
}

/// FR-045: the checked classifier accumulates finite-window and mixed-direction
/// limitations and retains the exact profile selection.
#[trace("TC-1279", "FR-045-AC-2", "FR-045-AC-3", "FR-045-AC-4", "FR-045-AC-6")]
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
    let clause = &checked.package().graph().temporal_clauses()[0];
    assert_eq!(classification.retained.identity, clause.identity());
    assert_eq!(classification.retained.name, clause.clause().name());
    assert_eq!(
        classification.retained.activation,
        clause.clause().activation()
    );
}

/// FR-045-AC-1/2/5: every checked support-table row is selected by profile,
/// reachable operator direction and surrounding closure, with all past
/// operator forms retaining the origin-complete-history target.
#[trace("TC-1279", "FR-045-AC-1", "FR-045-AC-2", "FR-045-AC-5")]
#[test]
fn checked_mapping_exercises_each_supported_row_and_past_operator() {
    for profile in [temporal::EVENT_POSITION, temporal::FIXED_SAMPLE] {
        for body in [
            "once[0,1] true",
            "historically[0,1] true",
            "true since[0,1] true",
            "true triggered[0,1] true",
        ] {
            let checked = checked(body, profile);
            for closure in [Closure::Open, Closure::Closed] {
                let classification =
                    temporal::mapping_support_checked(checked.package(), 0, closure).unwrap();
                assert_eq!(
                    classification.support,
                    temporal::Support::Supported {
                        target: temporal::Target::OriginCompleteHistory,
                        total_sample_valuation: false,
                        premises: temporal::OUTSTANDING_PREMISES,
                    },
                    "{profile}: {body}"
                );
            }
        }
    }

    for (profile, total_sample_valuation) in [
        (temporal::EVENT_POSITION, false),
        (temporal::FIXED_SAMPLE, true),
    ] {
        for (closure, target) in [
            (Closure::Closed, temporal::Target::ClosedTrace),
            (Closure::Open, temporal::Target::OnlinePrefix),
        ] {
            for body in ["true", "always[0,1] true"] {
                let checked = checked(body, profile);
                let classification =
                    temporal::mapping_support_checked(checked.package(), 0, closure).unwrap();
                assert_eq!(
                    classification.support,
                    temporal::Support::Supported {
                        target,
                        total_sample_valuation: total_sample_valuation
                            && body.starts_with("always"),
                        premises: temporal::OUTSTANDING_PREMISES,
                    },
                    "{profile}: {body}: {closure:?}"
                );
            }
        }
    }

    let mixed = checked(
        "once[0,1] true and eventually[0,1] true",
        temporal::EVENT_POSITION,
    );
    assert_eq!(
        temporal::mapping_support_checked(mixed.package(), 0, Closure::Closed)
            .unwrap()
            .support,
        temporal::Support::Unsupported {
            dimensions: vec![temporal::Unmatched::MixedDirection],
        }
    );

    let profile = "quire.temporal.infinite-trace/v1";
    let checked = checked("eventually[0,1] true", profile);
    assert_eq!(
        temporal::mapping_support_checked(checked.package(), 0, Closure::Closed)
            .unwrap()
            .support,
        temporal::Support::Unsupported {
            dimensions: vec![temporal::Unmatched::InfiniteTrace],
        },
        "{profile}"
    );
    let timed = checked_timed_mapping();
    assert_eq!(
        temporal::mapping_support_checked(timed.package(), 0, Closure::Closed)
            .unwrap()
            .support,
        temporal::Support::Unsupported {
            dimensions: vec![temporal::Unmatched::TimedProfile],
        }
    );
}

/// FR-043-AC-10/12: progress retained for one checked node rejects a changed
/// decision boundary and preserves the earlier open assertion.
#[trace("TC-1278", "FR-043-AC-10", "FR-043-AC-12")]
#[test]
fn checked_progress_is_keyed_by_checked_clause_identity() {
    let first = checked_named(
        "test:checked-temporal-first",
        "always[0,1] true",
        temporal::EVENT_POSITION,
    );
    let second = checked_named(
        "test:checked-temporal-second",
        "always[0,1] true",
        temporal::EVENT_POSITION,
    );
    let mut ledger = temporal::Ledger::new();
    let mut earlier = origin_trace();
    earlier.watermark = 5;
    earlier.decision_scope = Closure::Open;
    assert!(temporal::evaluate_checked_with_progress(
        first.package(),
        0,
        &earlier,
        temporal::Limits::default(),
        &mut ledger,
    )
    .result()
    .is_ok());
    let mut restamped = earlier.clone();
    restamped.watermark = 6;
    restamped.decision_scope = Closure::Closed;
    let refused = temporal::evaluate_checked_with_progress(
        first.package(),
        0,
        &restamped,
        temporal::Limits::default(),
        &mut ledger,
    );
    assert!(matches!(
        refused.result(),
        Err(temporal::Error::Refused(temporal::Refusal::Progress {
            dimension: temporal::Dimension::DecisionScope,
            ..
        }))
    ));
    let mut completeness_conflict = earlier.clone();
    completeness_conflict.watermark = 6;
    completeness_conflict.completeness = Completeness::Incomplete;
    let refused = temporal::evaluate_checked_with_progress(
        first.package(),
        0,
        &completeness_conflict,
        temporal::Limits::default(),
        &mut ledger,
    );
    assert!(matches!(
        refused.result(),
        Err(temporal::Error::Refused(temporal::Refusal::Progress {
            dimension: temporal::Dimension::Completeness,
            ..
        }))
    ));
    let mut continued = earlier.clone();
    continued.watermark = 6;
    assert!(temporal::evaluate_checked_with_progress(
        first.package(),
        0,
        &continued,
        temporal::Limits::default(),
        &mut ledger,
    )
    .result()
    .is_ok());
    let mut regressed = earlier.clone();
    regressed.watermark = 4;
    let refused = temporal::evaluate_checked_with_progress(
        first.package(),
        0,
        &regressed,
        temporal::Limits::default(),
        &mut ledger,
    );
    assert!(matches!(
        refused.result(),
        Err(temporal::Error::Refused(temporal::Refusal::Progress {
            dimension: temporal::Dimension::Watermark,
            ..
        }))
    ));
    let mut unrelated = origin_trace();
    unrelated.watermark = 1;
    assert!(temporal::evaluate_checked_with_progress(
        second.package(),
        0,
        &unrelated,
        temporal::Limits::default(),
        &mut ledger,
    )
    .result()
    .is_ok());
}

/// FR-044: a missing guard outcome remains local to that trigger, false guards
/// create no instance, and duplicate semantic identities add receipt
/// provenance without re-establishing immutable captures.
#[trace("TC-1278", "FR-044-AC-2", "FR-044-AC-3", "FR-044-AC-5")]
#[test]
fn checked_activation_keeps_siblings_and_capture_identity() {
    let checked = checked_activation(
        "test:checked-temporal-activation",
        "capture saved: Integer = trigger.code; always[0,1] true",
        "on each (trigger: M::Widget) when (trigger.code > 0)",
    );
    let mut input = origin_trace();
    input.anchor = "event-1".into();
    let capture = || temporal::CaptureInput::Value {
        anchor: "event-1".into(),
        value: "7".into(),
    };
    let trigger = |identity: &str, receipt: &str, guard, payload: &str| temporal::Trigger {
        identity: identity.into(),
        receipt: receipt.into(),
        anchor: "event-1".into(),
        payload: payload.into(),
        guard,
        captures: vec![capture()],
    };
    input.triggers = vec![
        trigger("missing-guard", "receipt-0", None, "a"),
        trigger("good-trigger", "receipt-1", Some(true), "b"),
        trigger("good-trigger", "receipt-2", Some(true), "b"),
        trigger("false-guard", "receipt-3", Some(false), "c"),
    ];
    let report =
        temporal::evaluate_checked(checked.package(), 0, &input, temporal::Limits::default());
    let obligations = report.result().expect("sibling instances assessed");
    assert_eq!(obligations.len(), 2);
    assert!(matches!(
        &obligations[0],
        temporal::Obligation::Unactivated { instance, error, .. }
            if instance == "missing-guard"
                && matches!(error, temporal::Error::Incomplete(incomplete)
                    if incomplete.dimension == temporal::Dimension::Guard)
    ));
    let temporal::Obligation::Assessed(active) = &obligations[1] else {
        panic!("healthy sibling remains assessed: {obligations:?}");
    };
    assert_eq!(active.instance.as_deref(), Some("good-trigger"));
    assert_eq!(active.receipts, ["receipt-1", "receipt-2"]);
    assert_eq!(active.capture_evaluations, 1);
    assert_eq!(active.captures[0].value, "7");
}

/// FR-044-AC-4: bad capture inputs fail before temporal evaluation while a
/// healthy sibling in the same checked activation remains assessable.
#[trace("TC-1278", "FR-044-AC-4")]
#[test]
fn checked_capture_failures_are_typed_and_local_to_their_instances() {
    let checked = checked_activation(
        "test:checked-temporal-capture-failures",
        "capture saved: Integer = trigger.code; always[0,1] true",
        "on each (trigger: M::Widget)",
    );
    let mut input = origin_trace();
    input.anchor = "event-1".into();
    let trigger = |identity: &str, capture| temporal::Trigger {
        identity: identity.into(),
        receipt: format!("receipt:{identity}"),
        anchor: "event-1".into(),
        payload: identity.into(),
        guard: None,
        captures: vec![capture],
    };
    input.triggers = vec![
        trigger("missing", temporal::CaptureInput::Missing),
        trigger("null", temporal::CaptureInput::Null),
        trigger("wrong-type", temporal::CaptureInput::WrongType),
        trigger("stale", temporal::CaptureInput::Stale),
        trigger(
            "wrong-anchor",
            temporal::CaptureInput::Value {
                anchor: "old-anchor".into(),
                value: "7".into(),
            },
        ),
        trigger(
            "healthy",
            temporal::CaptureInput::Value {
                anchor: "event-1".into(),
                value: "7".into(),
            },
        ),
    ];
    let report =
        temporal::evaluate_checked(checked.package(), 0, &input, temporal::Limits::default());
    let obligations = report
        .result()
        .expect("all trigger outcomes remain visible");
    assert_eq!(obligations.len(), input.triggers.len());
    for (obligation, expected_dimension) in obligations[..5].iter().zip([
        temporal::Dimension::Capture,
        temporal::Dimension::Capture,
        temporal::Dimension::Capture,
        temporal::Dimension::Capture,
        temporal::Dimension::Anchor,
    ]) {
        let temporal::Obligation::Unactivated { error, .. } = obligation else {
            panic!("bad capture has no active temporal result: {obligation:?}");
        };
        let dimension = match error {
            temporal::Error::Incomplete(incomplete) => incomplete.dimension,
            temporal::Error::Refused(temporal::Refusal::Capture { dimension, .. }) => *dimension,
            other => panic!("capture failure has its own typed outcome: {other:?}"),
        };
        assert_eq!(dimension, expected_dimension);
    }
    assert!(matches!(
        obligations.last(),
        Some(temporal::Obligation::Assessed(assessment))
            if assessment.instance.as_deref() == Some("healthy")
                && assessment.truth == Some(Truth::True)
    ));
}

/// FR-044-AC-8: whole-execution identities create distinct instances, retain
/// one identity through replay, and reject conflicting deliveries.
#[trace("TC-1278", "FR-044-AC-8")]
#[test]
fn checked_whole_execution_origins_keep_distinct_identities() {
    let checked = checked_activation(
        "test:checked-temporal-whole-origin",
        "always[0,1] true",
        "on origin",
    );
    let mut input = origin_trace();
    input.triggers = vec![
        temporal::Trigger {
            identity: "execution-origin:one".into(),
            receipt: "delivery:one".into(),
            anchor: "origin".into(),
            payload: "payload-one".into(),
            guard: None,
            captures: Vec::new(),
        },
        temporal::Trigger {
            identity: "execution-origin:two".into(),
            receipt: "delivery:two".into(),
            anchor: "origin".into(),
            payload: "payload-two".into(),
            guard: None,
            captures: Vec::new(),
        },
        temporal::Trigger {
            identity: "execution-origin:one".into(),
            receipt: "delivery:one-replay".into(),
            anchor: "origin".into(),
            payload: "payload-one".into(),
            guard: None,
            captures: Vec::new(),
        },
    ];
    let mut ledger = temporal::Ledger::new();
    let report = temporal::evaluate_checked_with_progress(
        checked.package(),
        0,
        &input,
        temporal::Limits::default(),
        &mut ledger,
    );
    let obligations = report.result().expect("both execution origins assess");
    assert_eq!(obligations.len(), 2);
    let instances: Vec<_> = obligations
        .iter()
        .map(|obligation| match obligation {
            temporal::Obligation::Assessed(assessment) => assessment,
            other => panic!("origin instance is assessed: {other:?}"),
        })
        .collect();
    assert_eq!(
        instances[0].instance.as_deref(),
        Some("execution-origin:one")
    );
    assert_eq!(
        instances[1].instance.as_deref(),
        Some("execution-origin:two")
    );
    assert_eq!(
        instances[0].receipts,
        ["delivery:one", "delivery:one-replay"]
    );
    assert_ne!(instances[0].instance, instances[1].instance);

    let replay = temporal::evaluate_checked_with_progress(
        checked.package(),
        0,
        &input,
        temporal::Limits::default(),
        &mut ledger,
    );
    let replayed = replay
        .result()
        .expect("replaying origins does not split them");
    assert_eq!(replayed.len(), 2);
    let replayed_instances: Vec<_> = replayed
        .iter()
        .map(|obligation| match obligation {
            temporal::Obligation::Assessed(assessment) => assessment.instance.as_deref(),
            other => panic!("origin instance remains assessed: {other:?}"),
        })
        .collect();
    assert_eq!(
        replayed_instances,
        [Some("execution-origin:one"), Some("execution-origin:two")]
    );

    input.triggers[0].payload = "conflict".into();
    let conflict = temporal::evaluate_checked_with_progress(
        checked.package(),
        0,
        &input,
        temporal::Limits::default(),
        &mut ledger,
    );
    assert!(matches!(
        conflict.result(),
        Err(temporal::Error::Refused(
            temporal::Refusal::Contradiction { .. }
        ))
    ));
}

/// FR-044-AC-6: a capture initializer may read earlier captures but the S3
/// checked route refuses a forward reference before creating a package.
#[trace("TC-1278", "FR-044-AC-6")]
#[test]
fn checked_capture_initializer_cannot_read_a_later_capture() {
    let refusal = try_checked_activation(
        "test:checked-temporal-forward-capture",
        "capture initialCapture: Integer = futureCapture; capture futureCapture: Integer = trigger.code; always[0,1] true",
        "on each (trigger: M::Widget)",
    )
    .expect_err("S3 refuses a later capture name");
    let qsl_replay::spine::CompileRefusal::Check { refusals, .. } = *refusal else {
        panic!("forward capture is a checked-name refusal: {refusal:?}");
    };
    assert!(refusals.iter().any(|refusal| matches!(
        &refusal.cause,
        qsl_semantics::check::CheckCause::MissingName(name) if name == "futureCapture"
    )));
}

/// FR-044-AC-5/7: empty admitted closed scopes and guard-false scopes are
/// inactive, while open, missing-evidence and unknown-guard cases remain
/// unknown/incomplete without evaluating temporal positions.
#[trace("TC-1278", "FR-044-AC-5", "FR-044-AC-7")]
#[test]
fn checked_activation_dispositions_never_become_temporal_truth() {
    let checked = checked_activation(
        "test:checked-temporal-dispositions",
        "always[0,1] true",
        "on each (trigger: M::Widget) when (trigger.code > 0)",
    );

    let mut no_triggers = origin_trace();
    no_triggers.triggers.clear();
    let report = temporal::evaluate_checked(
        checked.package(),
        0,
        &no_triggers,
        temporal::Limits::default(),
    );
    let temporal::Obligation::Assessed(inactive) = &report.result().unwrap()[0] else {
        panic!("closed empty trigger scope returns a disposition");
    };
    assert_eq!(inactive.activation, temporal::Activation::Inactive);
    assert_eq!((inactive.truth, inactive.basis), (None, None));
    assert_eq!(report.usage().positions, 0);

    no_triggers.trigger_scope = Closure::Open;
    let report = temporal::evaluate_checked(
        checked.package(),
        0,
        &no_triggers,
        temporal::Limits::default(),
    );
    let temporal::Obligation::Assessed(unknown) = &report.result().unwrap()[0] else {
        panic!("open trigger scope returns a disposition");
    };
    assert_eq!(
        unknown.activation,
        temporal::Activation::Unknown {
            completeness: Completeness::Complete,
            execution: Execution::Completed,
        }
    );
    assert_eq!((unknown.truth, unknown.basis), (None, None));
    assert_eq!(report.usage().positions, 0);

    for evidence in [temporal::Evidence::Missing, temporal::Evidence::Refused] {
        let mut unavailable = origin_trace();
        unavailable.trigger_evidence = evidence;
        let report = temporal::evaluate_checked(
            checked.package(),
            0,
            &unavailable,
            temporal::Limits::default(),
        );
        let temporal::Obligation::Assessed(unknown) = &report.result().unwrap()[0] else {
            panic!("missing or refused trigger evidence is a disposition");
        };
        assert!(matches!(
            unknown.activation,
            temporal::Activation::Unknown { .. }
        ));
        assert_eq!((unknown.truth, unknown.basis), (None, None));
        assert_eq!(report.usage().positions, 0);
    }

    let mut false_guard = origin_trace();
    false_guard.triggers[0].guard = Some(false);
    let report = temporal::evaluate_checked(
        checked.package(),
        0,
        &false_guard,
        temporal::Limits::default(),
    );
    let temporal::Obligation::Assessed(inactive) = &report.result().unwrap()[0] else {
        panic!("all guard-false triggers return inactive");
    };
    assert_eq!(inactive.activation, temporal::Activation::Inactive);
    assert_eq!((inactive.truth, inactive.basis), (None, None));
    assert_eq!(report.usage().positions, 0);
}

/// FR-044-AC-1/3: valuations for a capture-bound proposition are keyed by the
/// semantic trigger identity, so equal formula nodes do not let one instance's
/// captured predicate result settle a sibling.
#[trace("TC-1278", "FR-044-AC-1", "FR-044-AC-3")]
#[test]
fn checked_capture_bound_valuations_are_isolated_by_instance_identity() {
    let checked = checked_activation(
        "test:checked-temporal-instance-values",
        "capture saved: Integer = trigger.code; always[0,0] holds(view.code >= saved)",
        "on each (trigger: M::Widget)",
    );
    let clause = checked
        .package()
        .graph()
        .temporal_clauses()
        .first()
        .expect("one checked temporal clause")
        .clause();
    let leaf = clause
        .formula()
        .nodes()
        .iter()
        .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
        .and_then(|index| u32::try_from(index).ok())
        .expect("one holds leaf has a wire-sized index");

    let mut input = origin_trace();
    input.anchor = "event-1".into();
    input.positions = vec![Position {
        coordinate: 0,
        order: None,
        valuations: BTreeMap::from([(leaf, true)]),
        instance_valuations: BTreeMap::from([
            ("trigger:low".into(), BTreeMap::from([(leaf, true)])),
            ("trigger:high".into(), BTreeMap::from([(leaf, false)])),
        ]),
    }];
    input.triggers = [
        ("trigger:low", "7", "receipt:low"),
        ("trigger:high", "7", "receipt:high"),
        ("trigger:missing", "7", "receipt:missing"),
    ]
    .into_iter()
    .map(|(identity, value, receipt)| temporal::Trigger {
        identity: identity.into(),
        receipt: receipt.into(),
        anchor: "event-1".into(),
        payload: value.into(),
        guard: None,
        captures: vec![temporal::CaptureInput::Value {
            anchor: "event-1".into(),
            value: value.into(),
        }],
    })
    .collect();

    let report =
        temporal::evaluate_checked(checked.package(), 0, &input, temporal::Limits::default());
    let obligations = report.result().expect("both instances assessed");
    let reported_instances: Vec<_> = obligations[..2]
        .iter()
        .map(|obligation| match obligation {
            temporal::Obligation::Assessed(assessment) => (
                assessment.instance.as_deref(),
                assessment.captures[0].value.as_str(),
                assessment.truth,
            ),
            other => panic!("expected active instance, got {other:?}"),
        })
        .collect();
    assert_eq!(
        reported_instances,
        [
            (Some("trigger:low"), "7", Some(Truth::True)),
            (Some("trigger:high"), "7", Some(Truth::False)),
        ],
        "mismatched outcomes identify both semantic instances even with equal captures"
    );
    assert!(matches!(
        &obligations[2],
        temporal::Obligation::Assessed(assessment)
            if assessment.truth.is_none()
        && assessment.basis == Some(Basis::Unavailable)
        && assessment.incomplete.as_ref().map(|incomplete| incomplete.dimension)
            == Some(temporal::Dimension::Valuation)
    ));
}

/// FR-044-AC-6/9: progress-ledger re-evaluation reuses the first immutable
/// capture, while explicit eviction makes the instance incomplete instead of
/// evaluating a later capture input.
#[trace("TC-1278", "FR-044-AC-2", "FR-044-AC-6", "FR-044-AC-9")]
#[test]
fn checked_capture_is_restored_once_and_eviction_is_incomplete() {
    let checked = checked_activation(
        "test:checked-temporal-capture-restore",
        "capture saved: Integer = trigger.code; capture copied: Integer = trigger.code; always[0,1] true",
        "on each (trigger: M::Widget)",
    );
    let mut input = origin_trace();
    input.anchor = "event-1".into();
    input.triggers = vec![temporal::Trigger {
        identity: "trigger:one".into(),
        receipt: "receipt:one".into(),
        anchor: "event-1".into(),
        payload: "initial".into(),
        guard: None,
        captures: vec![
            temporal::CaptureInput::Value {
                anchor: "event-1".into(),
                value: "7".into(),
            },
            temporal::CaptureInput::Value {
                anchor: "event-1".into(),
                value: "8".into(),
            },
        ],
    }];
    let mut ledger = temporal::Ledger::new();
    let first = temporal::evaluate_checked_with_progress(
        checked.package(),
        0,
        &input,
        temporal::Limits::default(),
        &mut ledger,
    );
    let temporal::Obligation::Assessed(first) = &first.result().unwrap()[0] else {
        panic!("first trigger activates");
    };
    assert_eq!(first.captures[0].value, "7");
    assert_eq!(first.captures[1].value, "8");
    assert_eq!(first.capture_evaluations, 1);

    input.triggers[0].receipt = "receipt:replay".into();
    input.triggers[0].captures = vec![
        temporal::CaptureInput::WrongType,
        temporal::CaptureInput::WrongType,
    ];
    let replay = temporal::evaluate_checked_with_progress(
        checked.package(),
        0,
        &input,
        temporal::Limits::default(),
        &mut ledger,
    );
    let temporal::Obligation::Assessed(replayed) = &replay.result().unwrap()[0] else {
        panic!("replay reuses the retained activation");
    };
    assert_eq!(replayed.captures[0].value, "7");
    assert_eq!(replayed.captures[1].value, "8");
    assert_eq!(replayed.capture_evaluations, 1);
    assert_eq!(replayed.receipts, ["receipt:one", "receipt:replay"]);

    input.triggers[0].payload = "conflicting".into();
    let conflict = temporal::evaluate_checked_with_progress(
        checked.package(),
        0,
        &input,
        temporal::Limits::default(),
        &mut ledger,
    );
    assert!(matches!(
        conflict.result(),
        Err(temporal::Error::Refused(
            temporal::Refusal::Contradiction { .. }
        ))
    ));

    input.triggers[0].payload = "initial".into();
    input.evicted = vec![temporal::Eviction::Capture {
        instance: "trigger:one".into(),
        capture: 0,
    }];
    let restored = temporal::evaluate_checked_with_progress(
        checked.package(),
        0,
        &input,
        temporal::Limits::default(),
        &mut ledger,
    );
    assert!(matches!(
        &restored.result().unwrap()[0],
        temporal::Obligation::Unactivated { error: temporal::Error::Incomplete(incomplete), .. }
            if incomplete.dimension == temporal::Dimension::Capture
    ));
}

/// FR-043: checked profile and clock identity are authenticated before any
/// position work, and a missing required valuation is unavailable rather than
/// a false truth.
#[trace("TC-1277", "TC-1278", "FR-043-AC-3", "FR-043-AC-9")]
#[test]
fn checked_binding_and_missing_fact_keep_distinct_outcomes() {
    let checked = checked("always[0,1] holds(view.code > 0)", temporal::EVENT_POSITION);
    for (clock, profile, expected) in [
        (
            "other",
            temporal::EVENT_POSITION,
            temporal::Dimension::Clock,
        ),
        (
            "orders",
            temporal::FIXED_SAMPLE,
            temporal::Dimension::Profile,
        ),
    ] {
        let mut input = origin_trace();
        input.clock.name = clock.into();
        input.clock.profile_identity = profile.into();
        let report =
            temporal::evaluate_checked(checked.package(), 0, &input, temporal::Limits::default());
        assert!(matches!(
            report.result(),
            Err(temporal::Error::Refused(temporal::Refusal::Binding {
                dimension,
                ..
            })) if *dimension == expected
        ));
        assert_eq!(report.usage().positions, 0);
    }

    let report = temporal::evaluate_checked(
        checked.package(),
        0,
        &origin_trace(),
        temporal::Limits::default(),
    );
    let temporal::Obligation::Assessed(assessment) = &report.result().unwrap()[0] else {
        panic!("the checked clause remains represented");
    };
    assert_eq!(assessment.truth, None);
    assert_eq!(assessment.basis, Some(Basis::Unavailable));
    assert_eq!(
        assessment.incomplete.as_ref().map(|value| value.dimension),
        Some(temporal::Dimension::Valuation)
    );
}
