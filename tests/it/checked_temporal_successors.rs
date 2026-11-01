// SPDX-License-Identifier: AGPL-3.0-or-later
//! Successor coverage for FR-043/FR-044/FR-045 on checked-package admission.

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::DependencyInput;
use qsl_semantics::check::PackageDeclarations;
use quire_semantic_value::checking::CheckingLimits;
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
    .expect("S1-S4 checked activation package")
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
    let packages =
        qsl_semantics::model::intake::package_input([tests_fixture_spine_model().as_slice()])
            .expect("checked model fixture");
    let source = r#"
language "ix:native" edition "1-draft";
profile v = "quire.model.complete/v1";
profile tc = "quire.temporal.complete/v1";
profile t = "quire.temporal.timed/v1";
model M = "acme/orders";
dimension Time;
unit s : Time = rational(1, 1);
unit ms : Time = rational(1, 1000) * s;
time M dense unit ms;
temporal Due using t over (view: M::Widget) clock "model-time" on origin {
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

/// Build the S3 checked graph and link it directly at S4. This keeps the
/// checked-package evaluator boundary observable for representations that the
/// current serializer does not emit.
fn checked_graph_temporal(
    formula: &str,
) -> Result<qsl_package::CheckedPackage, Vec<qsl_semantics::check::CheckRefusal>> {
    let source = format!(
        r#"
language "ix:native" edition "1-draft";
profile v = "quire.value.complete/v1";
profile t = "quire.temporal.event-position.false-extension/v1";
record Counter {{ level: Int[0, 9]; }}
temporal Due using t over (c: Counter) clock "model-steps" on origin {{ {formula} }}
"#
    );
    let identity = SourceIdentity::new("agent-ix", "test:checked-unsupported", "fixture", "1");
    let parsed = qsl_cst::parse(
        identity,
        "checked-unsupported.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 parses checked candidate");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds checked candidate");
    let graph = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "checked-unsupported".to_owned(),
        unit,
        qsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("S2 assembly retains candidate")
    .check(CheckingLimits::default())?;
    Ok(qsl_package::CheckedPackage::link(graph))
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

/// FR-043: the checked formula adapter preserves every unary and binary
/// bounded operator and pointwise connective when it enters the shared core.
#[trace("TC-122", "FR-043-AC-6")]
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

/// FR-043-AC-5: equal-coordinate participants require one admitted order
/// authority; insertion order is never accepted as the ordering premise.
#[trace("TC-122", "FR-043-AC-5")]
#[test]
fn checked_order_sensitive_formula_refuses_unordered_equal_coordinates() {
    let checked = checked("true until[0,1] true", temporal::EVENT_POSITION);
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
    assert!(matches!(
        report.result(),
        Err(temporal::Error::Refused(temporal::Refusal::Order { .. }))
    ));
    assert_eq!(report.usage().positions, 0);

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
        let report =
            temporal::evaluate_checked(checked.package(), 0, &input, temporal::Limits::default());
        let temporal::Obligation::Assessed(assessment) = &report.result().unwrap()[0] else {
            panic!("admitted order evaluates");
        };
        assert_eq!(assessment.truth, Some(Truth::True));
    }
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
#[trace("TC-125", "FR-045-AC-6")]
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

/// FR-045-AC-1/2/5: every checked support-table row is selected by profile,
/// reachable operator direction and surrounding closure, with all past
/// operator forms retaining the origin-complete-history target.
#[trace("TC-125", "FR-045-AC-1", "FR-045-AC-2", "FR-045-AC-5")]
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

    for (profile, dimension) in [(
        "quire.temporal.infinite-trace/v1",
        temporal::Unmatched::InfiniteTrace,
    )] {
        let checked = checked("eventually[0,1] true", profile);
        assert_eq!(
            temporal::mapping_support_checked(checked.package(), 0, Closure::Closed)
                .unwrap()
                .support,
            temporal::Support::Unsupported {
                dimensions: vec![dimension],
            },
            "{profile}"
        );
    }
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

/// FR-043-AC-10: progress retained for one checked node cannot contradict an
/// unrelated checked package at the same temporal-clause index.
#[trace("TC-123", "FR-043-AC-10")]
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
    assert!(temporal::evaluate_checked_with_progress(
        first.package(),
        0,
        &earlier,
        temporal::Limits::default(),
        &mut ledger,
    )
    .result()
    .is_ok());
    let mut regressed = earlier;
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
#[trace("TC-123", "FR-044-AC-3", "FR-044-AC-5", "FR-044-AC-8")]
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

/// FR-044-AC-5/7: empty admitted closed scopes and guard-false scopes are
/// inactive, while open, missing-evidence and unknown-guard cases remain
/// unknown/incomplete without evaluating temporal positions.
#[trace("TC-123", "FR-044-AC-5", "FR-044-AC-7")]
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
#[trace("TC-123", "FR-044-AC-1", "FR-044-AC-3")]
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
        valuations: BTreeMap::new(),
        instance_valuations: BTreeMap::from([
            ("trigger:low".into(), BTreeMap::from([(leaf, true)])),
            ("trigger:high".into(), BTreeMap::from([(leaf, false)])),
        ]),
    }];
    input.triggers = [
        ("trigger:low", "1", "receipt:low"),
        ("trigger:high", "9", "receipt:high"),
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
    let truths: Vec<_> = obligations
        .iter()
        .map(|obligation| match obligation {
            temporal::Obligation::Assessed(assessment) => assessment.truth,
            other => panic!("expected active instance, got {other:?}"),
        })
        .collect();
    assert_eq!(truths, [Some(Truth::True), Some(Truth::False)]);
}

/// FR-044-AC-6/9: progress-ledger re-evaluation reuses the first immutable
/// capture, while explicit eviction makes the instance incomplete instead of
/// evaluating a later capture input.
#[trace("TC-123", "FR-044-AC-6", "FR-044-AC-9")]
#[test]
fn checked_capture_is_restored_once_and_eviction_is_incomplete() {
    let checked = checked_activation(
        "test:checked-temporal-capture-restore",
        "capture saved: Integer = trigger.code; always[0,1] true",
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
        captures: vec![temporal::CaptureInput::Value {
            anchor: "event-1".into(),
            value: "7".into(),
        }],
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
    assert_eq!(first.capture_evaluations, 1);

    input.triggers[0].receipt = "receipt:replay".into();
    input.triggers[0].captures = vec![temporal::CaptureInput::WrongType];
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
#[trace("TC-122", "TC-123", "FR-043-AC-3", "FR-043-AC-9")]
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

/// FR-046-AC-5 / TC-127: S3 recognizes the checked aggregate candidate and
/// returns its typed unsupported proof representation, before S4 can produce
/// a Boolean result or partial runtime artifact.
#[trace("TC-127", "FR-046-AC-5")]
#[test]
fn checked_aggregate_candidate_returns_typed_unsupported_without_partial_result() {
    let refusals = match checked_graph_temporal("always[0,2] count over past[0,3] >= 1") {
        Ok(_) => panic!("S3 must not admit an unsupported aggregate representation"),
        Err(refusals) => refusals,
    };
    assert!(matches!(
        refusals.as_slice(),
        [qsl_semantics::check::CheckRefusal {
            cause: qsl_semantics::check::CheckCause::Temporal(cause),
            ..
        }] if matches!(
            cause.as_ref(),
            qsl_semantics::check::TemporalCause::UnsupportedProofRepresentation { name, .. }
                if name == "Due"
        )
    ));
}
