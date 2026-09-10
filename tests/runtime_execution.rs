// SPDX-License-Identifier: AGPL-3.0-only
//! FR-023: real native execution and retained request provenance.

// This shared public-API fixture also serves graph and operation binaries.
#[allow(dead_code)]
#[path = "support/runtime_setup.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::package::{NativePackage, PackageLimits};
use quire_spec_language::runtime::{
    evaluate, execute, validate, EvaluationLimits, EvaluationOutcome, ExecutionLimits,
    ExecutionOutcome, ExecutionReport, ExecutionSelection, ImplicationEventKind, RuntimeInput,
    ValidationLimits, ValidationStatus, ValueId, ValueNode,
};
use quire_spec_language::syntax::ClauseKind;
use quire_spec_language::Code;

fn assert_request(
    report: &ExecutionReport<'_, '_>,
    package: &NativePackage<'_>,
    offered: &RuntimeInput,
    selection: &ExecutionSelection,
) {
    assert!(std::ptr::eq(report.package(), package));
    assert_eq!(report.selection(), selection);
    assert_eq!(report.input().snapshots.len(), offered.snapshots.len());
    assert_eq!(report.input().invocations.len(), offered.invocations.len());
    for (actual, expected) in report.input().snapshots.iter().zip(&offered.snapshots) {
        assert_eq!(actual.reference(), expected.reference());
        assert_eq!(actual.bytes(), expected.bytes());
    }
    for (actual, expected) in report.input().invocations.iter().zip(&offered.invocations) {
        assert_eq!(actual.reference(), expected.reference());
        assert_eq!(actual.bytes(), expected.bytes());
    }
}

#[test]
#[trace("TC-097", "FR-023-AC-1")]
fn aggregate_reports_retain_real_truth_and_the_complete_request() {
    let models = [setup::native_rule_model::parts().model()];
    let package = NativePackage::new(
        setup::checked(&models, "forall(item in self.items: item < self.n)"),
        PackageLimits::default(),
    )
    .unwrap();
    for (values, expected, comparisons) in [
        (vec![1, 1, 1], true, 3),
        (vec![1, 2, 1], false, 2),
        (vec![], true, 0),
    ] {
        let mut draft = setup::draft(&models[0]);
        setup::change_field(&mut draft, "n", ValueNode::Integer { value: 2 });
        let values = values
            .into_iter()
            .map(|value| {
                let id = ValueId::new(u32::try_from(draft.arena.len()).unwrap());
                draft.arena.push(ValueNode::Integer { value });
                id
            })
            .collect();
        setup::change_field(&mut draft, "items", ValueNode::Sequence { values });
        let snapshot = setup::snapshot(draft);
        let selected = setup::selection(&models[0], snapshot.reference());
        let offered = setup::input(snapshot);
        let report = execute(
            &package,
            offered.clone(),
            selected.clone(),
            ExecutionLimits::default(),
            || false,
        );
        assert_request(&report, &package, &offered, &selected);
        assert_eq!(report.truth(), Some(expected));
        assert!(report.validation_usage().work > 0);
        let ExecutionOutcome::Evaluated {
            result,
            usage,
            events,
            cost_model,
        } = report.outcome()
        else {
            panic!("valid aggregate input must reach evaluation");
        };
        assert_eq!(result, &EvaluationOutcome::Completed(expected));
        assert_eq!(usage.comparisons, comparisons);
        assert!(events.is_empty());
        assert_eq!(*cost_model, "native-ref-cost/1-draft");
    }
}

#[test]
#[trace("TC-098", "FR-023-AC-2", "FR-023-AC-3")]
fn validation_stops_retain_offered_inputs_and_actual_failure_details() {
    let models = [setup::native_rule_model::parts().model()];
    let package =
        NativePackage::new(setup::checked(&models, "true"), PackageLimits::default()).unwrap();
    for case in [
        "field",
        "foreign",
        "unavailable",
        "work",
        "cancel",
        "details",
    ] {
        let mut draft = setup::draft(&models[0]);
        if matches!(case, "field" | "details") {
            draft.populations[0].objects[0].fields.pop();
        }
        let snapshot = setup::snapshot(draft);
        let mut selected = setup::selection(&models[0], snapshot.reference());
        if case == "foreign" {
            selected.requirement =
                ir::RequirementRef::parse("example/foreign", "Foreign", 1).unwrap();
        }
        let mut offered = setup::input(snapshot);
        if case == "unavailable" {
            offered.snapshots.clear();
        }
        let mut limits = ExecutionLimits::default();
        if case == "work" {
            limits.validation.work = 0;
        }
        if case == "details" {
            limits.validation.diagnostics = 0;
        }
        let direct = validate(
            package.checked(),
            offered.clone(),
            selected.clone(),
            limits.validation,
            || case == "cancel",
        )
        .unwrap_err();
        let report = execute(&package, offered.clone(), selected.clone(), limits, || {
            case == "cancel"
        });
        assert_request(&report, &package, &offered, &selected);
        assert_eq!(report.truth(), None);
        let ExecutionOutcome::ValidationFailed(failure) = report.outcome() else {
            panic!("failed validation must not evaluate: {case}");
        };
        let expected = if matches!(case, "field" | "foreign" | "details") {
            ValidationStatus::Refused
        } else {
            ValidationStatus::Incomplete
        };
        assert_eq!(failure.status, expected, "{case}");
        assert_eq!(failure.status, direct.status);
        assert_eq!(failure.diagnostics, direct.diagnostics);
        assert_eq!(failure.terminal, direct.terminal);
        assert_eq!(failure.usage, direct.usage);
        assert_eq!(report.validation_usage(), direct.usage);
        if matches!(case, "work" | "cancel") {
            let retry = execute(
                &package,
                offered.clone(),
                selected.clone(),
                ExecutionLimits::default(),
                || false,
            );
            assert_request(&retry, &package, &offered, &selected);
            assert_eq!(retry.truth(), Some(true));
        }
    }
}

#[test]
#[trace("TC-098", "FR-023-AC-3")]
fn evaluation_stops_keep_measured_prefixes_and_forward_one_poll_across_stages() {
    let models = [setup::native_rule_model::parts().model()];
    let package = NativePackage::new(
        setup::checked(&models, "true implies true"),
        PackageLimits::default(),
    )
    .unwrap();
    let snapshot = setup::snapshot(setup::draft(&models[0]));
    let selected = setup::selection(&models[0], snapshot.reference());
    let offered = setup::input(snapshot);
    let mut validation_polls = 0;
    let context = validate(
        package.checked(),
        offered.clone(),
        selected.clone(),
        ValidationLimits::default(),
        || {
            validation_polls += 1;
            false
        },
    )
    .unwrap();
    for limits in [
        EvaluationLimits {
            expression_steps: 2,
            ..EvaluationLimits::default()
        },
        EvaluationLimits {
            events: 2,
            ..EvaluationLimits::default()
        },
    ] {
        let direct = evaluate(&context, limits, || false);
        let report = execute(
            &package,
            offered.clone(),
            selected.clone(),
            ExecutionLimits {
                evaluation: limits,
                ..ExecutionLimits::default()
            },
            || false,
        );
        assert_request(&report, &package, &offered, &selected);
        assert_eq!(report.truth(), None);
        let ExecutionOutcome::Evaluated {
            result,
            usage,
            events,
            cost_model,
        } = report.outcome()
        else {
            panic!("validation must complete before evaluation exhaustion");
        };
        assert!(
            matches!(result, EvaluationOutcome::Incomplete(d) if d.code == Code::ResourceExhausted)
        );
        assert_eq!(result, direct.outcome());
        assert_eq!(*usage, direct.usage());
        assert_eq!(events, direct.events());
        assert_eq!(*cost_model, direct.cost_model());
        assert_eq!(
            events.iter().map(|event| event.kind).collect::<Vec<_>>(),
            [
                ImplicationEventKind::AntecedentEntered,
                ImplicationEventKind::AntecedentCompleted(true),
            ]
        );
    }
    let mut combined_polls = 0;
    let cancelled = execute(
        &package,
        offered.clone(),
        selected.clone(),
        ExecutionLimits::default(),
        || {
            combined_polls += 1;
            combined_polls > validation_polls
        },
    );
    assert_request(&cancelled, &package, &offered, &selected);
    assert_eq!(combined_polls, validation_polls + 1);
    assert_eq!(cancelled.truth(), None);
    let ExecutionOutcome::Evaluated {
        result,
        usage,
        events,
        ..
    } = cancelled.outcome()
    else {
        panic!("cancellation occurs on first evaluator poll");
    };
    assert!(matches!(result, EvaluationOutcome::Incomplete(d) if d.code == Code::Cancelled));
    assert_eq!(usage.expression_steps, 0);
    assert!(events.is_empty());
    let retry = execute(
        &package,
        offered.clone(),
        selected.clone(),
        ExecutionLimits::default(),
        || false,
    );
    assert_request(&retry, &package, &offered, &selected);
    assert_eq!(retry.truth(), Some(true));
    assert_eq!(retry.validation_usage(), context.usage());
}

#[test]
#[trace("TC-097", "FR-023-AC-4")]
fn operation_reports_preserve_invocation_captures_and_frame_refusals() {
    let models = [setup::native_rule_model::parts().model()];
    let package = NativePackage::new(
        setup::checked_kind(
            &models,
            "pre(self.n) = 1 and self.n = 2 and result",
            ClauseKind::Postcondition,
        ),
        PackageLimits::default(),
    )
    .unwrap();
    for (result, bad_frame) in [(true, false), (false, false), (true, true)] {
        let before = setup::draft(&models[0]);
        let mut after = before.clone();
        setup::change_field(&mut after, "n", ValueNode::Integer { value: 2 });
        if bad_frame {
            setup::change_field(&mut after, "signed", ValueNode::Integer { value: 1 });
        }
        let (offered, selected) = setup::recorded(&models[0], before, after, |invocation| {
            invocation.arena[0] = ValueNode::Boolean { value: result };
        });
        let report = execute(
            &package,
            offered.clone(),
            selected.clone(),
            ExecutionLimits::default(),
            || false,
        );
        assert_request(&report, &package, &offered, &selected);
        assert_eq!(
            report.input().snapshots[0].draft().observation,
            ir::StateObservation::Pre
        );
        assert_eq!(
            report.input().snapshots[1].draft().observation,
            ir::StateObservation::Post
        );
        if bad_frame {
            assert_eq!(report.truth(), None);
            let ExecutionOutcome::ValidationFailed(failure) = report.outcome() else {
                panic!("a forbidden frame change must fail validation");
            };
            assert_eq!(failure.status, ValidationStatus::Refused);
            assert!(failure
                .diagnostics
                .iter()
                .any(|d| d.code == Code::FrameViolation));
        } else {
            assert_eq!(report.truth(), Some(result));
        }
    }
}
