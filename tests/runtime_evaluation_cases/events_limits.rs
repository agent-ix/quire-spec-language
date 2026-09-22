// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-073–076: original event lineage, exact stops, immutable retries and refusals.

use super::*;
use qsl_foundation::Phase;
use quire_spec_language::syntax::{ClauseKind, ExprKind};

#[test]
#[trace("TC-073", "FR-008-AC-7", "FR-008-AC-8", "FR-008-AC-9")]
#[trace("FR-008-AC-10", "FR-008-AC-17")]
fn original_grouped_operand_spans_and_atomic_event_capacity() {
    let models = [native_rule_model::parts().model()];
    for crlf in [false, true] {
        let checked = request_source(
            &models,
            "((true)) implies (false)",
            ClauseKind::Invariant,
            |text| {
                let text = text.replace("{ ((true))", "{ // é界\n ((true))");
                if crlf {
                    text.replace('\n', "\r\n")
                } else {
                    text
                }
            },
        )
        .unwrap();
        let unit = checked.linked().unit();
        let implication = unit.clauses()[0].expression;
        let ExprKind::Binary { left, right, .. } = unit.expression(implication).unwrap().kind
        else {
            panic!("implication root")
        };
        let context = current(&checked, &models[0], draft(&models[0]));
        let completed = evaluate(&context, EvaluationLimits::default(), || false);
        assert_eq!(completed.outcome(), &EvaluationOutcome::Completed(false));
        assert_eq!(completed.usage().expression_steps, 3);
        for (event, operand, spelling) in [
            (completed.events()[0], left, "((true))"),
            (completed.events()[1], left, "((true))"),
            (completed.events()[2], right, "(false)"),
        ] {
            assert_eq!(event.implication, implication);
            assert_eq!(event.operand, operand);
            assert_eq!(
                &unit.source().text()[event.span.start..event.span.end],
                spelling
            );
            let expected_start = unit.source().text().find(spelling).unwrap();
            assert_eq!(event.span.start, expected_start);
            let formal = checked
                .bindings()
                .source
                .to_ir(unit.source(), event.span)
                .unwrap();
            let restored = checked.bindings().source.to_native(&formal).unwrap();
            assert_eq!(restored, event.span);
        }
        for (capacity, steps, events) in [(0, 1, 0), (1, 2, 1), (2, 2, 2), (3, 3, 3)] {
            let report = evaluate(
                &context,
                EvaluationLimits {
                    events: capacity,
                    ..EvaluationLimits::default()
                },
                || false,
            );
            assert_eq!(report.usage().expression_steps, steps);
            assert_eq!(report.events(), &completed.events()[..events]);
            if capacity == 3 {
                assert_eq!(report.outcome(), &EvaluationOutcome::Completed(false));
            } else {
                assert!(
                    matches!(report.outcome(), EvaluationOutcome::Incomplete(d) if d.code == Code::ResourceExhausted)
                );
            }
        }
        let no_depth = evaluate(
            &context,
            EvaluationLimits {
                depth: 1,
                ..EvaluationLimits::default()
            },
            || false,
        );
        assert_eq!(no_depth.usage().expression_steps, 1);
        assert!(no_depth.events().is_empty());
        assert!(
            matches!(no_depth.outcome(), EvaluationOutcome::Incomplete(d) if d.code == Code::ResourceExhausted)
        );
    }
}

#[test]
#[trace("TC-073", "FR-008-AC-17")]
fn nested_implications_report_actual_entry_and_completion_order() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "(true implies false) implies (false implies true)");
    let context = current(&checked, &models[0], draft(&models[0]));
    let report = evaluate(&context, EvaluationLimits::default(), || false);
    assert_eq!(report.outcome(), &EvaluationOutcome::Completed(true));
    assert_eq!(
        report.events().iter().map(|e| e.kind).collect::<Vec<_>>(),
        [
            ImplicationEventKind::AntecedentEntered,
            ImplicationEventKind::AntecedentEntered,
            ImplicationEventKind::AntecedentCompleted(true),
            ImplicationEventKind::ConsequentEntered,
            ImplicationEventKind::AntecedentCompleted(false),
        ]
    );
    assert_eq!(
        report.events()[0].implication,
        report.events()[4].implication
    );
    assert_eq!(
        report.events()[1].implication,
        report.events()[3].implication
    );
    assert_ne!(
        report.events()[0].implication,
        report.events()[1].implication
    );
}

#[test]
#[trace("TC-075", "FR-008-AC-5", "FR-008-AC-19")]
fn every_actual_poll_can_cancel_or_panic_without_changing_retry_state() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(
        &models,
        "let x = true in x implies reaches(self, self, parent)",
    );
    let mut data = draft(&models[0]);
    change_field(
        &mut data,
        "parent",
        ValueNode::Present {
            value: ValueId::new(3),
        },
    );
    let context = current(&checked, &models[0], data);
    let original = context.input().snapshots[0].bytes().to_vec();
    let mut polls = 0;
    let complete = evaluate(&context, EvaluationLimits::default(), || {
        polls += 1;
        false
    });
    assert_eq!(complete.outcome(), &EvaluationOutcome::Completed(true));
    for boundary in 1..=polls {
        let mut actual = 0;
        let stopped = evaluate(&context, EvaluationLimits::default(), || {
            actual += 1;
            actual == boundary
        });
        assert!(
            matches!(stopped.outcome(), EvaluationOutcome::Incomplete(d) if d.code == Code::Cancelled)
        );
        assert_eq!(actual, boundary);
        assert_eq!(
            stopped.events(),
            &complete.events()[..stopped.events().len()]
        );
        assert!(stopped.usage().expression_steps <= complete.usage().expression_steps);
        let mut actual = 0;
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            evaluate(&context, EvaluationLimits::default(), || {
                actual += 1;
                assert_ne!(actual, boundary, "caller poll panic");
                false
            })
        }));
        assert!(panic.is_err());
        let retry = evaluate(&context, EvaluationLimits::default(), || false);
        assert_eq!(retry.outcome(), complete.outcome());
        assert_eq!(retry.usage(), complete.usage());
        assert_eq!(retry.events(), complete.events());
    }
    let smaller = evaluate(
        &context,
        EvaluationLimits {
            expression_steps: complete.usage().expression_steps - 1,
            ..EvaluationLimits::default()
        },
        || false,
    );
    assert!(matches!(
        smaller.outcome(),
        EvaluationOutcome::Incomplete(_)
    ));
    assert_eq!(context.input().snapshots[0].bytes(), original);
}

#[test]
#[trace("TC-075", "FR-008-AC-18")]
fn active_non_group_depth_is_independent_of_other_unused_limits() {
    let models = [native_rule_model::parts().model()];
    for (expression, required) in [("true", 1), ("(((true)))", 1), ("not not true", 3)] {
        let checked = checked(&models, expression);
        let context = current(&checked, &models[0], draft(&models[0]));
        let limits = EvaluationLimits {
            depth: required,
            graph_steps: 0,
            comparisons: 0,
            text_steps: 0,
            events: 0,
            ..EvaluationLimits::default()
        };
        let result = evaluate(&context, limits, || false);
        assert_eq!(result.outcome(), &EvaluationOutcome::Completed(true));
        assert_eq!(result.usage().expression_depth, required);
        let stopped = evaluate(
            &context,
            EvaluationLimits {
                depth: required - 1,
                ..limits
            },
            || false,
        );
        assert!(
            matches!(stopped.outcome(), EvaluationOutcome::Incomplete(d) if d.code == Code::ResourceExhausted)
        );
    }
}

#[test]
#[trace("TC-076", "FR-008-AC-20")]
fn unsupported_collect_and_ineligible_equality_refuse_at_the_frontend() {
    let models = [native_rule_model::parts().model()];
    let collect = request(&models, "collect(self.items)", ClauseKind::Invariant).unwrap_err();
    assert_eq!(collect.code, Code::UnsupportedConstruct);
    assert_eq!(collect.phase, Phase::Profile);
    for expression in ["self.items = self.items", "self.parent = self.parent"] {
        let refused = request(&models, expression, ClauseKind::Invariant).unwrap_err();
        assert_eq!(refused.code, Code::IllTyped);
        assert_eq!(refused.phase, Phase::Check);
    }
    let checked = checked(
        &models,
        "forall(item in self.items: item = self.n implies true)",
    );
    let mut data = draft(&models[0]);
    change_field(
        &mut data,
        "items",
        ValueNode::Sequence {
            values: vec![ValueId::new(0); 3],
        },
    );
    let context = current(&checked, &models[0], data);
    let report = evaluate(&context, EvaluationLimits::default(), || false);
    assert_eq!(report.outcome(), &EvaluationOutcome::Completed(true));
    assert_eq!(report.events().len(), 9);
}

#[test]
#[trace("TC-073", "FR-008-AC-10")]
fn multibyte_operand_end_has_independent_byte_line_and_scalar_coordinates() {
    let models = [super::scalars::text_model()];
    for crlf in [false, true] {
        let checked = request_source(
            &models,
            "label = \"é界\" implies true",
            ClauseKind::Invariant,
            |text| {
                if crlf {
                    text.replace('\n', "\r\n")
                } else {
                    text
                }
            },
        )
        .unwrap();
        let context = current(
            &checked,
            &models[0],
            super::scalars::text_draft(&models[0], "é界", "é界"),
        );
        let report = evaluate(&context, EvaluationLimits::default(), || false);
        assert_eq!(report.outcome(), &EvaluationOutcome::Completed(true));
        let source = checked.linked().unit().source();
        let event = report.events()[0];
        let start = source.text().find("label = \"é界\"").unwrap();
        assert_eq!(event.span.start, start);
        assert_eq!(event.span.end, start + "label = \"é界\"".len());
        let formal = checked.bindings().source.to_ir(source, event.span).unwrap();
        for (offset, position) in [
            (event.span.start, formal.start()),
            (event.span.end, formal.end()),
        ] {
            let prefix = &source.text()[..offset];
            let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
            let last_line = prefix.rsplit('\n').next().unwrap();
            let column = last_line.chars().count() + 1;
            assert_eq!(usize::try_from(position.byte_offset()).unwrap(), offset);
            assert_eq!(usize::try_from(position.line()).unwrap(), line);
            assert_eq!(usize::try_from(position.column()).unwrap(), column);
        }
        assert_eq!(formal.end().column() - formal.start().column(), 12);
        assert_eq!(event.span.end - event.span.start, 15);
    }
}

#[test]
#[trace("TC-071", "TC-074", "FR-008-AC-14", "FR-008-AC-16")]
fn skipped_boolean_operands_and_conditional_branches_consume_no_work_or_events() {
    let models = [native_rule_model::parts().model()];
    for (expression, steps, truth) in [
        ("false and (true implies true)", 2, false),
        ("true or (true implies true)", 2, true),
        ("false or true", 3, true),
        ("true and false", 3, false),
        ("if false then (true implies true) else false", 3, false),
        ("if true then true else (true implies false)", 3, true),
    ] {
        let checked = checked(&models, expression);
        let context = current(&checked, &models[0], draft(&models[0]));
        let report = evaluate(
            &context,
            EvaluationLimits {
                expression_steps: steps,
                events: 0,
                ..EvaluationLimits::default()
            },
            || false,
        );
        assert_eq!(
            report.outcome(),
            &EvaluationOutcome::Completed(truth),
            "{expression}"
        );
        assert_eq!(report.usage().expression_steps, steps);
        assert!(report.events().is_empty());
    }
}
