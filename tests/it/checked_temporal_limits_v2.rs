// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1415: NFR-008 temporal limit successors on strict checked-package v2.

use super::*;
use ix_trace_rs::trace;

fn checked_v2_case(
    body: &str,
    test: impl FnOnce(&qsl_replay::spine::CheckedUnit, &v2::AdmittedPackage, u32),
) {
    checked_v2_case_with_source_limits(body, quire_spec_language::Limits::default(), test);
}

fn checked_v2_case_with_source_limits(
    body: &str,
    source_limits: quire_spec_language::Limits,
    test: impl FnOnce(&qsl_replay::spine::CheckedUnit, &v2::AdmittedPackage, u32),
) {
    with_v2_inputs_and_source_limits(
        checked_fr050_inputs_with_event_body(body),
        source_limits,
        |inputs, proofs, _, temporal, emitted| {
            let mut admission_limits = Limits::default();
            admission_limits.entries = admission_limits
                .entries
                .max(source_limits.tokens.saturating_mul(20));
            admission_limits.depth = admission_limits.depth.max(source_limits.tokens);
            admission_limits.content_bytes = admission_limits
                .content_bytes
                .max(source_limits.source_bytes.saturating_mul(20));
            admission_limits.payload_bytes = admission_limits
                .payload_bytes
                .max(source_limits.source_bytes.saturating_mul(20));
            admission_limits.output_bytes = admission_limits
                .output_bytes
                .max(source_limits.source_bytes.saturating_mul(20));
            admission_limits.byte_work = admission_limits
                .byte_work
                .max(source_limits.source_bytes.saturating_mul(200));
            let admitted = inputs
                .read_v2_bytes(
                    proofs,
                    emitted.bytes(),
                    &temporal.expected,
                    admission_limits,
                )
                .into_result()
                .expect("strict checked-package v2 admission");
            let identity = SourceIdentity::new(
                inputs.sources[0].identity().authority.clone(),
                inputs.sources[0].identity().identity.clone(),
                inputs.sources[0].identity().revision_namespace.clone(),
                inputs.sources[0].identity().revision.clone(),
            );
            let checked = checked_fr050_unit_with_identity_and_limits(
                inputs,
                0,
                identity,
                qsl_replay::spine::SpineLimits {
                    source: qsl_cst::Limits {
                        source_bytes: source_limits.source_bytes,
                        tokens: source_limits.tokens,
                        nodes: source_limits.nodes,
                        work_units: qsl_cst::Limits::default().work_units,
                    },
                    ..qsl_replay::spine::SpineLimits::default()
                },
            );
            let atom = checked.package().graph().temporal_clauses()[0]
                .clause()
                .formula()
                .nodes()
                .iter()
                .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
                .and_then(|index| u32::try_from(index).ok())
                .expect("checked holds atom");
            test(&checked, &admitted, atom);
        },
    );
}

fn event_trace(atom: u32) -> temporal::Trace {
    let mut trace = trace_input(
        "event-clock",
        temporal::EVENT_POSITION,
        &[("sequence_authority", "orders")],
    );
    trace.positions.push(temporal::Position {
        coordinate: 0,
        order: None,
        valuations: BTreeMap::from([(atom, true)]),
        instance_valuations: BTreeMap::new(),
    });
    trace.watermark = 2;
    trace
}

fn activated_trace(atom: u32) -> temporal::Trace {
    let mut trace = event_trace(atom);
    trace.anchor = "event-1".into();
    trace.triggers = vec![temporal::Trigger {
        identity: "trigger:one".into(),
        receipt: "receipt:one".into(),
        anchor: "event-1".into(),
        payload: "payload:one".into(),
        guard: None,
        captures: vec![temporal::CaptureInput::Value {
            anchor: "event-1".into(),
            value: "17".into(),
        }],
    }];
    trace
}

fn limit_for(
    mut limits: temporal::Limits,
    dimension: temporal::LimitDimension,
    value: usize,
) -> temporal::Limits {
    match dimension {
        temporal::LimitDimension::Positions => limits.positions = value,
        temporal::LimitDimension::Valuations => limits.valuations = value,
        temporal::LimitDimension::Instances => limits.instances = value,
        temporal::LimitDimension::Captures => limits.captures = value,
        temporal::LimitDimension::Retention => limits.retention = value,
        temporal::LimitDimension::Visits => limits.visits = value,
        temporal::LimitDimension::Horizon => limits.horizon = value,
        temporal::LimitDimension::Depth => limits.depth = value,
    }
    limits
}

fn usage_for(usage: temporal::Usage, dimension: temporal::LimitDimension) -> usize {
    match dimension {
        temporal::LimitDimension::Positions => usage.positions,
        temporal::LimitDimension::Valuations => usage.valuations,
        temporal::LimitDimension::Instances => usage.instances,
        temporal::LimitDimension::Captures => usage.captures,
        temporal::LimitDimension::Retention => usage.retention,
        temporal::LimitDimension::Visits => usage.visits,
        temporal::LimitDimension::Horizon => usage.horizon,
        temporal::LimitDimension::Depth => usage.depth,
    }
}

fn checked_report(
    checked: &qsl_replay::spine::CheckedUnit,
    admitted: &v2::AdmittedPackage,
    trace: &temporal::Trace,
    limits: temporal::Limits,
) -> temporal::Report {
    temporal::evaluate_checked_v2(checked.package(), admitted, 0, trace, limits)
}

fn result_without_limits(
    report: &temporal::Report,
) -> Result<Vec<temporal::Obligation>, temporal::Error> {
    match report.result() {
        Ok(obligations) => {
            let mut obligations = obligations.to_vec();
            for obligation in &mut obligations {
                if let temporal::Obligation::Assessed(assessment) = obligation {
                    assessment.premises.limits = temporal::Limits::default();
                }
            }
            Ok(obligations)
        }
        Err(error) => Err(error.clone()),
    }
}

/// AC-1: checked composition refuses before visiting a position on the strict
/// v2 admitted route. The authored sum `1 + i64::MAX` is independently outside
/// the signed domain.
#[trace("TC-124", "NFR-008-AC-1")]
#[test]
fn checked_v2_temporal_composition_refuses_i64_overflow_before_work() {
    let body = format!(
        "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin {{ always[1,1] eventually[{},{}] holds(view.code > 0) }}",
        i64::MAX,
        i64::MAX
    );
    assert!(1i64.checked_add(i64::MAX).is_none());
    checked_v2_case(&body, |checked, admitted, atom| {
        let report = temporal::evaluate_checked_v2(
            checked.package(),
            admitted,
            0,
            &event_trace(atom),
            temporal::Limits::default(),
        );
        let Err(temporal::Error::Exhausted(exhaustion)) = report.result() else {
            panic!("overflow must be a typed stop: {:?}", report.result());
        };
        assert_eq!(exhaustion.dimension, temporal::LimitDimension::Horizon);
        assert!(report.usage().positions <= 1);
        assert_eq!(report.usage().valuations, 0);
        assert_eq!(report.authenticated().unwrap().clock, "event-clock");
    });
}

/// AC-1 also checks past-history composition from a nonzero negative offset;
/// its authored result is independently below `i64::MIN`.
#[trace("TC-124", "NFR-008-AC-1")]
#[test]
fn checked_v2_past_history_composition_refuses_i64_underflow_before_atoms() {
    let body = format!(
        "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin {{ once[2,2] once[{},{}] holds(view.code > 0) }}",
        i64::MAX,
        i64::MAX
    );
    assert!((-2i64).checked_sub(i64::MAX).is_none());
    checked_v2_case(&body, |checked, admitted, atom| {
        let report = checked_report(
            checked,
            admitted,
            &event_trace(atom),
            temporal::Limits::default(),
        );
        let Err(temporal::Error::Exhausted(exhaustion)) = report.result() else {
            panic!("past underflow must stop: {:?}", report.result());
        };
        assert_eq!(exhaustion.dimension, temporal::LimitDimension::Horizon);
        assert_eq!(report.usage().valuations, 0);
        assert!(report.authenticated().is_some());
    });
}

/// AC-2: every published charge dimension has a typed stop on checked-v2 and
/// suppresses truth for the affected obligation.
#[trace("TC-124", "NFR-008-AC-2")]
#[test]
fn checked_v2_limits_stop_all_seven_dimensions_without_truth() {
    let body = "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { always[0,1] holds(view.code > 0) }";
    checked_v2_case(body, |checked, admitted, atom| {
        let trace = event_trace(atom);
        let controls = [
            (temporal::LimitDimension::Positions, 0),
            (temporal::LimitDimension::Valuations, 0),
            (temporal::LimitDimension::Instances, 0),
            (temporal::LimitDimension::Retention, 0),
            (temporal::LimitDimension::Visits, 0),
            (temporal::LimitDimension::Horizon, 0),
        ];
        for (dimension, ceiling) in controls {
            let report = checked_report(
                checked,
                admitted,
                &trace,
                limit_for(temporal::Limits::default(), dimension, ceiling),
            );
            let Err(temporal::Error::Exhausted(exhaustion)) = report.result() else {
                panic!(
                    "{dimension:?} must stop with no Boolean: {:?}",
                    report.result()
                );
            };
            assert_eq!(exhaustion.dimension, dimension);
            assert_eq!(exhaustion.subject.declaration, 0);
            assert!(report.authenticated().is_some());
        }

        let capture_body = "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on each (trigger: M::Widget) { capture saved: M::Widget = trigger; eventually[0,0] holds(view.code > 0) }";
        checked_v2_case(capture_body, |checked, admitted, atom| {
            let report = checked_report(
                checked,
                admitted,
                &activated_trace(atom),
                temporal::Limits {
                    captures: 0,
                    ..temporal::Limits::default()
                },
            );
            let Err(temporal::Error::Exhausted(exhaustion)) = report.result() else {
                panic!("capture exhaustion is typed: {:?}", report.result());
            };
            assert_eq!(exhaustion.dimension, temporal::LimitDimension::Captures);
        });
    });
}

/// AC-2: a sibling settled before a later instance exhausts remains available
/// with its exact truth, basis and decisive-position support.
#[trace("TC-124", "NFR-008-AC-2")]
#[test]
fn checked_v2_exhaustion_preserves_settled_sibling_assessment() {
    let body = "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on each (trigger: M::Widget) { always[0,0] holds(view.code > 0) }";
    checked_v2_case(body, |checked, admitted, atom| {
        let mut one = activated_trace(atom);
        one.triggers[0].captures.clear();
        let first = checked_report(checked, admitted, &one, temporal::Limits::default());
        let first_visits = first.usage().visits;
        assert!(first_visits > 0, "the first obligation is evaluated");

        let mut two = one;
        two.triggers.push(temporal::Trigger {
            identity: "trigger:two".into(),
            receipt: "receipt:two".into(),
            anchor: "event-1".into(),
            payload: "payload:two".into(),
            guard: None,
            captures: Vec::new(),
        });
        let stopped = checked_report(
            checked,
            admitted,
            &two,
            temporal::Limits {
                visits: first_visits + 1,
                ..temporal::Limits::default()
            },
        );
        let Err(temporal::Error::Exhausted(exhaustion)) = stopped.result() else {
            panic!("the second obligation exceeds the shared visit ceiling");
        };
        assert_eq!(exhaustion.dimension, temporal::LimitDimension::Visits);
        assert_eq!(exhaustion.subject.instance, 1);
        assert_eq!(stopped.usage().visits, first_visits + 1);
        assert_eq!(stopped.authenticated().unwrap().clock, "event-clock");

        let [temporal::Obligation::Assessed(sibling)] = stopped.settled_siblings() else {
            panic!("the first settled sibling remains inspectable");
        };
        assert_eq!(sibling.truth, Some(temporal::Truth::True));
        assert_eq!(sibling.basis, Some(temporal::Basis::ClosedScope));
        assert_eq!(sibling.support, [0]);
    });
}

/// AC-3: explicit eviction on the checked-v2 path names the lost required
/// valuation and leaves the affected result without truth.
#[trace("TC-124", "NFR-008-AC-3")]
#[test]
fn checked_v2_eviction_is_an_incomplete_result_not_a_narrower_window() {
    let body = "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { always[0,1] holds(view.code > 0) }";
    checked_v2_case(body, |checked, admitted, atom| {
        let mut trace = event_trace(atom);
        trace.evicted.push(temporal::Eviction::Valuation {
            node: atom,
            coordinate: 0,
        });
        let report = checked_report(
            checked,
            admitted,
            &trace,
            temporal::Limits {
                retention: 1,
                ..temporal::Limits::default()
            },
        );
        let temporal::Obligation::Assessed(assessment) =
            &report.result().expect("eviction is an obligation result")[0]
        else {
            panic!("the checked obligation remains inspectable");
        };
        assert_eq!(assessment.truth, None);
        let incomplete = assessment
            .incomplete
            .as_ref()
            .expect("typed incomplete result");
        assert_eq!(incomplete.dimension, temporal::Dimension::Valuation);
        assert_eq!(incomplete.subject.declaration, 0);
        assert_eq!(incomplete.subject.position, Some(0));
        assert_eq!(report.usage().retention, 1);
    });
}

/// AC-3: retention tracks valuation records the obligation actually requires,
/// not unrelated records in the supplied eviction list.
#[trace("TC-124", "NFR-008-AC-3")]
#[test]
fn checked_v2_irrelevant_evictions_do_not_consume_retention() {
    let body = "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { eventually[0,0] holds(view.code > 0) }";
    checked_v2_case(body, |checked, admitted, atom| {
        let mut trace = event_trace(atom);
        trace
            .evicted
            .extend((1..=128).map(|coordinate| temporal::Eviction::Valuation {
                node: atom.saturating_add(100),
                coordinate,
            }));
        let report = checked_report(
            checked,
            admitted,
            &trace,
            temporal::Limits {
                retention: 1,
                ..temporal::Limits::default()
            },
        );
        let temporal::Obligation::Assessed(assessment) = &report.result().unwrap()[0] else {
            panic!("irrelevant eviction instructions do not stop the obligation");
        };
        assert_eq!(assessment.truth, Some(temporal::Truth::True));
        assert_eq!(report.usage().retention, 1);
    });
}

/// AC-2: scanning eviction records for valuation lookups consumes bounded
/// evaluator work and stops before an unaffordable scan entry.
#[trace("TC-124", "NFR-008-AC-2")]
#[test]
fn checked_v2_eviction_scan_work_is_bounded_for_formula_lookups() {
    let body = "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { eventually[0,0] holds(view.code > 0) }";
    checked_v2_case(body, |checked, admitted, atom| {
        let mut trace = event_trace(atom);
        trace
            .evicted
            .extend((1..=128).map(|coordinate| temporal::Eviction::Valuation {
                node: atom.saturating_add(100),
                coordinate,
            }));
        let report = checked_report(
            checked,
            admitted,
            &trace,
            temporal::Limits {
                visits: 2,
                ..temporal::Limits::default()
            },
        );
        let Err(temporal::Error::Exhausted(exhaustion)) = report.result() else {
            panic!("eviction lookup scanning is bounded: {:?}", report.result());
        };
        assert_eq!(exhaustion.dimension, temporal::LimitDimension::Visits);
        assert_eq!(exhaustion.used, 2);
        assert_eq!(report.usage().visits, 2);
    });
}

/// AC-2: capture activation uses the same bounded accounting while checking
/// explicit eviction records for a restored capture.
#[trace("TC-124", "NFR-008-AC-2")]
#[test]
fn checked_v2_eviction_scan_work_is_bounded_for_capture_lookups() {
    let body = "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on each (trigger: M::Widget) { capture saved: M::Widget = trigger; always[0,0] holds(view.code > 0) }";
    checked_v2_case(body, |checked, admitted, atom| {
        let mut trace = activated_trace(atom);
        trace
            .evicted
            .extend((1..=128).map(|coordinate| temporal::Eviction::Valuation {
                node: atom.saturating_add(100),
                coordinate,
            }));
        let report = checked_report(
            checked,
            admitted,
            &trace,
            temporal::Limits {
                visits: 2,
                ..temporal::Limits::default()
            },
        );
        let Err(temporal::Error::Exhausted(exhaustion)) = report.result() else {
            panic!(
                "capture eviction lookup scanning is bounded: {:?}",
                report.result()
            );
        };
        assert_eq!(exhaustion.dimension, temporal::LimitDimension::Visits);
        assert_eq!(exhaustion.used, 2);
        assert_eq!(report.usage().visits, 2);
    });
}

/// AC-5: a visit refusal during retained-instance eviction scanning leaves the
/// ledger unchanged, so retrying the same delivery appends its receipt once.
#[trace("TC-124", "NFR-008-AC-5")]
#[test]
fn checked_v2_retained_capture_scan_refusal_retries_without_duplicate_receipt() {
    let body = "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on each (trigger: M::Widget) { capture saved: M::Widget = trigger; always[0,0] holds(view.code > 0) }";
    checked_v2_case(body, |checked, admitted, atom| {
        let mut ledger = temporal::Ledger::new();
        let mut input = activated_trace(atom);
        let seeded = temporal::evaluate_checked_with_progress_v2(
            checked.package(),
            admitted,
            0,
            &input,
            temporal::Limits::default(),
            &mut ledger,
        );
        assert!(seeded.result().is_ok());

        input.triggers[0].receipt = "receipt:retry".into();
        input.triggers[0].captures = vec![temporal::CaptureInput::WrongType];
        input
            .evicted
            .extend((1..=8).map(|coordinate| temporal::Eviction::Valuation {
                node: atom.saturating_add(100),
                coordinate,
            }));
        let stopped = temporal::evaluate_checked_with_progress_v2(
            checked.package(),
            admitted,
            0,
            &input,
            temporal::Limits {
                visits: 2,
                ..temporal::Limits::default()
            },
            &mut ledger,
        );
        let Err(temporal::Error::Exhausted(exhaustion)) = stopped.result() else {
            panic!("the retained-instance scan stops at the visit limit");
        };
        assert_eq!(exhaustion.dimension, temporal::LimitDimension::Visits);
        assert_eq!(stopped.usage().visits, 2);

        let retried = temporal::evaluate_checked_with_progress_v2(
            checked.package(),
            admitted,
            0,
            &input,
            temporal::Limits {
                visits: 32,
                ..temporal::Limits::default()
            },
            &mut ledger,
        );
        let temporal::Obligation::Assessed(assessment) = &retried.result().unwrap()[0] else {
            panic!("the unchanged retained state permits a successful retry");
        };
        assert_eq!(assessment.receipts, ["receipt:one", "receipt:retry"]);
        assert_eq!(assessment.captures[0].value, "17");
    });
}

/// AC-5: exact ceilings permit the full result, one-short ceilings refuse the
/// first unpaid charge, and retries reproduce the result from unchanged input.
#[trace("TC-124", "NFR-008-AC-5")]
#[test]
fn checked_v2_retries_are_charge_exact_and_leave_inputs_unchanged() {
    for (body, dimension) in [
        (
            "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { always[0,1] holds(view.code > 0) }",
            temporal::LimitDimension::Positions,
        ),
        (
            "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { eventually[0,0] holds(view.code > 0) }",
            temporal::LimitDimension::Valuations,
        ),
        (
            "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { eventually[0,0] holds(view.code > 0) }",
            temporal::LimitDimension::Instances,
        ),
        (
            "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { always[0,1] holds(view.code > 0) }",
            temporal::LimitDimension::Visits,
        ),
        (
            "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { always[0,1] holds(view.code > 0) }",
            temporal::LimitDimension::Horizon,
        ),
    ] {
        checked_v2_case(body, |checked, admitted, atom| {
            let trace = event_trace(atom);
            exercise_exact_retry(checked, admitted, &trace, dimension);
        });
    }

    let capture_body = "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on each (trigger: M::Widget) { capture saved: M::Widget = trigger; eventually[0,0] holds(view.code > 0) }";
    checked_v2_case(capture_body, |checked, admitted, atom| {
        exercise_exact_retry(
            checked,
            admitted,
            &activated_trace(atom),
            temporal::LimitDimension::Captures,
        );
    });

    let retention_body = "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { always[0,1] holds(view.code > 0) }";
    checked_v2_case(retention_body, |checked, admitted, atom| {
        let trace = event_trace(atom);
        exercise_exact_retry(
            checked,
            admitted,
            &trace,
            temporal::LimitDimension::Retention,
        );
    });
}

fn exercise_exact_retry(
    checked: &qsl_replay::spine::CheckedUnit,
    admitted: &v2::AdmittedPackage,
    input: &temporal::Trace,
    dimension: temporal::LimitDimension,
) {
    let before = input.clone();
    let full = checked_report(checked, admitted, input, temporal::Limits::default());
    let required = usage_for(full.usage(), dimension);
    assert!(required > 0, "{dimension:?} control charges its dimension");

    let exact = checked_report(
        checked,
        admitted,
        input,
        limit_for(temporal::Limits::default(), dimension, required),
    );
    assert_eq!(
        result_without_limits(&exact),
        result_without_limits(&full),
        "exact {dimension:?} ceiling"
    );
    assert_eq!(usage_for(exact.usage(), dimension), required);

    let zero = checked_report(
        checked,
        admitted,
        input,
        limit_for(temporal::Limits::default(), dimension, 0),
    );
    let Err(temporal::Error::Exhausted(zero_stop)) = zero.result() else {
        panic!("zero {dimension:?} ceiling refuses: {:?}", zero.result());
    };
    assert_eq!(zero_stop.dimension, dimension);
    assert_eq!(usage_for(zero.usage(), dimension), 0);

    let insufficient = checked_report(
        checked,
        admitted,
        input,
        limit_for(
            temporal::Limits::default(),
            dimension,
            required.saturating_sub(1),
        ),
    );
    let Err(temporal::Error::Exhausted(exhaustion)) = insufficient.result() else {
        panic!(
            "one-short {dimension:?} refuses: {:?}",
            insufficient.result()
        );
    };
    assert_eq!(exhaustion.dimension, dimension);
    assert_eq!(
        exhaustion.used,
        usage_for(insufficient.usage(), dimension),
        "{dimension:?} reports only successful charges"
    );
    assert!(
        usage_for(insufficient.usage(), dimension) < required,
        "the refused {dimension:?} charge was not performed"
    );

    let raised = checked_report(
        checked,
        admitted,
        input,
        limit_for(temporal::Limits::default(), dimension, usize::MAX),
    );
    assert_eq!(result_without_limits(&raised), result_without_limits(&full));
    assert_eq!(usage_for(raised.usage(), dimension), required);

    let retry = checked_report(checked, admitted, input, temporal::Limits::default());
    assert_eq!(result_without_limits(&retry), result_without_limits(&full));
    assert_eq!(retry.usage(), full.usage());
    assert_eq!(*input, before, "retry inputs remain unmutated");
}

/// AC-4: authenticated checked-v2 assessments retain effective limits in
/// identity; unchanged limits reproduce the result and changed limits create
/// a distinct assessment.
#[trace("TC-124", "NFR-008-AC-4")]
#[test]
fn checked_v2_effective_limits_participate_in_result_identity() {
    let body = "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { eventually[0,0] holds(view.code > 0) }";
    checked_v2_case(body, |checked, admitted, atom| {
        let trace = event_trace(atom);
        let first = checked_report(checked, admitted, &trace, temporal::Limits::default());
        let repeated = checked_report(checked, admitted, &trace, temporal::Limits::default());
        assert_eq!(first.result(), repeated.result());

        let changed = checked_report(
            checked,
            admitted,
            &trace,
            temporal::Limits {
                visits: temporal::Limits::default().visits - 1,
                ..temporal::Limits::default()
            },
        );
        let temporal::Obligation::Assessed(original) = &first.result().unwrap()[0] else {
            panic!("the baseline has an assessment");
        };
        let temporal::Obligation::Assessed(changed) = &changed.result().unwrap()[0] else {
            panic!("the changed ceiling remains sufficient");
        };
        assert_ne!(original.premises.limits, changed.premises.limits);

        let above = checked_report(
            checked,
            admitted,
            &trace,
            temporal::Limits {
                visits: usize::MAX,
                ..temporal::Limits::default()
            },
        );
        assert_eq!(above.limits().visits, usize::MAX);
        assert_ne!(first.result(), above.result());
        let zero = checked_report(
            checked,
            admitted,
            &trace,
            temporal::Limits {
                visits: 0,
                ..temporal::Limits::default()
            },
        );
        assert_eq!(zero.limits().visits, 0);
    });
}

/// AC-4: admitted restoration state participates in checked-v2 result identity.
/// Repeating the same restored state reproduces the result; restoring a
/// different retained capture under the same checked binding produces a
/// distinct assessment.
#[trace("TC-124", "NFR-008-AC-4")]
#[test]
fn checked_v2_restored_capture_state_participates_in_result_identity() {
    let body = "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on each (trigger: M::Widget) { capture saved: M::Widget = trigger; always[0,0] holds(view.code > 0) }";
    checked_v2_case(body, |checked, admitted, atom| {
        let trigger = |capture: temporal::CaptureInput, receipt: &str| temporal::Trigger {
            identity: "trigger:restore".into(),
            receipt: receipt.into(),
            anchor: "event-1".into(),
            payload: "stable-payload".into(),
            guard: None,
            captures: vec![capture],
        };
        let trace = |capture: temporal::CaptureInput, receipt: &str| {
            let mut trace = event_trace(atom);
            trace.anchor = "event-1".into();
            trace.triggers = vec![trigger(capture, receipt)];
            trace
        };
        let seed = |value: &str| {
            trace(
                temporal::CaptureInput::Value {
                    anchor: "event-1".into(),
                    value: value.into(),
                },
                "receipt:seed",
            )
        };
        let replay = trace(temporal::CaptureInput::WrongType, "receipt:replay");

        let mut seven_state = temporal::Ledger::new();
        temporal::evaluate_checked_with_progress_v2(
            checked.package(),
            admitted,
            0,
            &seed("17"),
            temporal::Limits::default(),
            &mut seven_state,
        );
        let mut seven_replay_a = seven_state.clone();
        let mut seven_replay_b = seven_state.clone();
        let restored_seven = temporal::evaluate_checked_with_progress_v2(
            checked.package(),
            admitted,
            0,
            &replay,
            temporal::Limits::default(),
            &mut seven_replay_a,
        );
        let repeated_seven = temporal::evaluate_checked_with_progress_v2(
            checked.package(),
            admitted,
            0,
            &replay,
            temporal::Limits::default(),
            &mut seven_replay_b,
        );
        assert_eq!(restored_seven.result(), repeated_seven.result());

        let mut nine_state = temporal::Ledger::new();
        temporal::evaluate_checked_with_progress_v2(
            checked.package(),
            admitted,
            0,
            &seed("18"),
            temporal::Limits::default(),
            &mut nine_state,
        );
        let restored_nine = temporal::evaluate_checked_with_progress_v2(
            checked.package(),
            admitted,
            0,
            &replay,
            temporal::Limits::default(),
            &mut nine_state,
        );

        let temporal::Obligation::Assessed(seven) = &restored_seven.result().unwrap()[0] else {
            panic!("the seven-valued capture is restored");
        };
        let temporal::Obligation::Assessed(nine) = &restored_nine.result().unwrap()[0] else {
            panic!("the nine-valued capture is restored");
        };
        assert_eq!(seven.captures[0].value, "17");
        assert_eq!(nine.captures[0].value, "18");
        assert_ne!(restored_seven.result(), restored_nine.result());
    });
}

/// AC-4: a caller-raised ceiling admits work whose visit count exceeds the
/// published default and retains that effective ceiling in result identity.
#[trace("TC-124", "NFR-008-AC-4")]
#[test]
fn checked_v2_raised_visit_limit_admits_work_above_the_default() {
    let body = "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { always[0,500000] (holds(view.code > 0) and true) }";
    checked_v2_case(body, |checked, admitted, atom| {
        let trace = event_trace(atom);
        let default = checked_report(checked, admitted, &trace, temporal::Limits::default());
        let Err(temporal::Error::Exhausted(exhaustion)) = default.result() else {
            panic!(
                "the published visit ceiling is below this work: {:?}",
                default.result()
            );
        };
        assert_eq!(exhaustion.dimension, temporal::LimitDimension::Visits);
        assert_eq!(default.usage().visits, temporal::Limits::default().visits);

        let raised = checked_report(
            checked,
            admitted,
            &trace,
            temporal::Limits {
                visits: 3_000_000,
                ..temporal::Limits::default()
            },
        );
        let temporal::Obligation::Assessed(assessment) = &raised.result().unwrap()[0] else {
            panic!("the raised limit admits the complete evaluation");
        };
        assert_eq!(assessment.truth, Some(temporal::Truth::False));
        assert!(raised.usage().visits > temporal::Limits::default().visits);
        assert_eq!(raised.limits().visits, 3_000_000);
    });
}

/// AC-4: strict checked-v2 evaluation has no formula-depth ceiling and walks
/// long pointwise nesting on the heap, even with the specified small stack.
#[trace("TC-124", "NFR-008-AC-4")]
#[test]
fn checked_v2_walks_one_hundred_thousand_nested_not_nodes_on_a_small_stack() {
    let body = format!(
        "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin {{ {}holds(view.code > 0) }}",
        "not ".repeat(100_000)
    );
    checked_v2_case_with_source_limits(
        &body,
        quire_spec_language::Limits {
            tokens: 300_000,
            nodes: 150_000,
            ..quire_spec_language::Limits::default()
        },
        |checked, admitted, atom| {
            let trace = event_trace(atom);
            std::thread::scope(|scope| {
                let worker = std::thread::Builder::new()
                    .stack_size(512 * 1024)
                    .spawn_scoped(scope, || {
                        temporal::evaluate_checked_v2(
                            checked.package(),
                            admitted,
                            0,
                            &trace,
                            temporal::Limits {
                                visits: 300_000,
                                ..temporal::Limits::default()
                            },
                        )
                    })
                    .expect("spawn the 512 KiB evaluator thread");
                let report = worker.join().expect("heap-backed formula traversal");
                let temporal::Obligation::Assessed(assessment) =
                    &report.result().expect("the full formula is evaluated")[0]
                else {
                    panic!("the formula has a settled assessment");
                };
                assert_eq!(assessment.truth, Some(temporal::Truth::True));
                assert_eq!(report.usage().visits, 100_002);
                assert_eq!(report.limits().visits, 300_000);
            });
        },
    );
}

/// AC-4: the explicit evaluator frames also cover branching and repeated
/// non-singleton temporal quantification at deep formula nesting.
#[trace("TC-124", "NFR-008-AC-4")]
#[test]
fn checked_v2_walks_deep_branching_quantification_on_a_small_stack() {
    let body = format!(
        "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin {{ always[0,1] {}(holds(view.code > 0) and true) }}",
        "not ".repeat(100_000)
    );
    checked_v2_case_with_source_limits(
        &body,
        quire_spec_language::Limits {
            tokens: 300_000,
            nodes: 150_000,
            ..quire_spec_language::Limits::default()
        },
        |checked, admitted, atom| {
            let mut trace = event_trace(atom);
            trace.positions.push(temporal::Position {
                coordinate: 1,
                order: None,
                valuations: BTreeMap::from([(atom, true)]),
                instance_valuations: BTreeMap::new(),
            });
            std::thread::scope(|scope| {
                let worker = std::thread::Builder::new()
                    .stack_size(512 * 1024)
                    .spawn_scoped(scope, || {
                        temporal::evaluate_checked_v2(
                            checked.package(),
                            admitted,
                            0,
                            &trace,
                            temporal::Limits {
                                visits: 250_000,
                                ..temporal::Limits::default()
                            },
                        )
                    })
                    .expect("spawn the 512 KiB evaluator thread");
                let report = worker.join().expect("heap-backed formula traversal");
                let temporal::Obligation::Assessed(assessment) =
                    &report.result().expect("the full formula is evaluated")[0]
                else {
                    panic!("the formula has a settled assessment");
                };
                assert_eq!(assessment.truth, Some(temporal::Truth::True));
                assert_eq!(report.usage().visits, 200_008);
                assert_eq!(report.limits().visits, 250_000);
            });
        },
    );
}
