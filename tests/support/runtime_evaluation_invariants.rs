// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-069: private fault controls for invariants public constructors prevent violating.

use crate::runtime::{
    evaluate, validate, EvaluationLimits, EvaluationOutcome, ValidationLimits, ValueNode,
};
use crate::runtime_test_setup as setup;
use crate::syntax::ClauseKind;
use crate::{Code, Phase};
use ix_trace_rs::trace;
use setup::*;

#[test]
#[trace("TC-069", "FR-008-AC-12", "FR-008-AC-19")]
fn private_corruption_cannot_turn_failed_checked_arithmetic_into_a_boolean() {
    let models = [native_rule_model::parts().model()];
    for (expression, corrupt) in [
        (
            "self.signed + 0 = self.signed",
            ValueNode::Integer { value: 11 },
        ),
        (
            "self.signed div -1 = self.signed",
            ValueNode::Integer { value: i64::MIN },
        ),
        (
            "self.signed + 0 = self.signed",
            ValueNode::Boolean { value: true },
        ),
    ] {
        let checked = checked(&models, expression);
        let artifact = snapshot(draft(&models[0]));
        let selected = selection(&models[0], artifact.reference());
        let mut context = validate(
            &checked,
            input(artifact),
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        let original = evaluate(&context, EvaluationLimits::default(), || false);
        assert_eq!(original.outcome(), &EvaluationOutcome::Completed(true));
        let usage = original.usage();
        // This descendant of validation::api can corrupt private storage; callers cannot.
        context.input.snapshots[0].artifact.draft.arena[1] = corrupt;
        let report = evaluate(&context, EvaluationLimits::default(), || false);
        let EvaluationOutcome::Refused(diagnostic) = report.outcome() else {
            panic!("violated invariant produced {report:?}");
        };
        assert_eq!(diagnostic.phase, Phase::Evaluate);
        assert_eq!(diagnostic.code, Code::RuntimeInvariant);
        assert!(!diagnostic.is_incomplete());
        assert_eq!(
            diagnostic.source,
            *checked.linked().unit().source().identity()
        );
        assert!(std::ptr::eq(report.context(), &context));
        context.input.snapshots[0].artifact.draft.arena[1] = ValueNode::Integer { value: 0 };
        let retry = evaluate(&context, EvaluationLimits::default(), || false);
        assert_eq!(retry.outcome(), &EvaluationOutcome::Completed(true));
        assert_eq!(retry.usage(), usage);
    }
}

#[test]
#[trace("TC-069", "FR-008-AC-12")]
fn private_unavailable_capture_refuses_without_a_public_unchecked_context() {
    let models = [authored_model(|_| {})];
    let checked = checked_kind(&models, "self.n = 2", ClauseKind::Postcondition);
    let mut after = draft(&models[0]);
    change_field(&mut after, "n", ValueNode::Integer { value: 2 });
    let (offered, selected) = recorded(&models[0], draft(&models[0]), after, |_| {});
    let mut context = validate(
        &checked,
        offered,
        selected,
        ValidationLimits::default(),
        || false,
    )
    .unwrap();
    assert_eq!(
        evaluate(&context, EvaluationLimits::default(), || false).outcome(),
        &EvaluationOutcome::Completed(true)
    );
    context.input.snapshots.clear();
    let report = evaluate(&context, EvaluationLimits::default(), || false);
    assert!(
        matches!(report.outcome(), EvaluationOutcome::Refused(d) if d.code == Code::RuntimeInvariant)
    );
}
