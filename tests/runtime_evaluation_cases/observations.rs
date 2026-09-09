// SPDX-License-Identifier: AGPL-3.0-only
//! TC-061/071: actual operation execution and immutable pre/post captures.

use super::*;
use quire_spec_language::syntax::ClauseKind;
use serde_json::json;

#[test]
#[trace("TC-061", "TC-071", "FR-008-AC-6", "FR-008-AC-14")]
fn precondition_and_postcondition_select_their_actual_self_observation() {
    let models = [native_rule_model::parts().model()];
    for (kind, expression, truth) in [
        (ClauseKind::Precondition, "self.n = 1", true),
        (ClauseKind::Postcondition, "self.n = 1", false),
        (
            ClauseKind::Postcondition,
            "pre(self.n) = 1 and self.n = 2",
            true,
        ),
        (
            ClauseKind::Postcondition,
            "let n = self.n in pre(n) = 2",
            true,
        ),
        (ClauseKind::Postcondition, "pre(pre(self.n)) = 1", true),
        (
            ClauseKind::Postcondition,
            "let n = if result then pre(self.n) else self.n in pre(n) = 1",
            true,
        ),
    ] {
        let checked = checked_kind(&models, expression, kind);
        let mut after = draft(&models[0]);
        change_field(&mut after, "n", ValueNode::Integer { value: 2 });
        let (offered, selected) = recorded(&models[0], draft(&models[0]), after, |_| {});
        let context = validate(
            &checked,
            offered,
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        assert_eq!(
            evaluate(&context, EvaluationLimits::default(), || false).outcome(),
            &EvaluationOutcome::Completed(truth),
            "{expression}"
        );
    }
}

#[test]
#[trace("TC-071", "FR-008-AC-6", "FR-008-AC-14", "FR-008-AC-15")]
fn parameter_reference_keeps_pre_capture_after_its_target_is_deleted() {
    let models = [authored_model(|data| {
        data["operations"][0]["frame"]["deleted"] = json!(["Node"]);
        data["values"].as_array_mut().unwrap().push(
            json!({"name":"captured", "kind":"input", "type":{"kind":"record", "name":"NodeRef"}}),
        );
        data["operations"][0]["parameters"] = json!(["captured"]);
    })];
    for expression in [
        "deref(captured).n = 1",
        "deref(pre(captured)).n = 1",
        "let p = captured in pre(deref(p).n) = 1",
    ] {
        let checked = checked_kind(&models, expression, ClauseKind::Postcondition);
        let mut before = draft(&models[0]);
        let mut deleted = before.populations[0].objects[0].clone();
        deleted.key = "deleted".into();
        before.populations[0].objects.push(deleted);
        let mut after = draft(&models[0]);
        change_field(&mut after, "n", ValueNode::Integer { value: 2 });
        let (offered, selected) = recorded(&models[0], before, after, |invocation| {
            invocation.deleted.push(object(&models[0], "deleted"));
            invocation.arena.push(ValueNode::Reference {
                identity: object(&models[0], "deleted"),
            });
            invocation.parameters.push(ValueBinding {
                declaration: qualified(&models[0], "captured"),
                value: ValueId::new(1),
            });
        });
        let context = validate(
            &checked,
            offered,
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        assert_eq!(
            evaluate(&context, EvaluationLimits::default(), || false).outcome(),
            &EvaluationOutcome::Completed(true),
            "{expression}"
        );
    }
}

#[test]
#[trace("TC-071", "FR-008-AC-14", "FR-008-AC-15")]
fn optional_alias_and_conditional_preserve_capture_while_direct_pre_changes_reads() {
    let models = [authored_model(|data| {
        data["operations"][0]["frame"]["fields"] = json!([["Node", "n"], ["Node", "parent"]]);
    })];
    for (expression, truth) in [
        ("let p = self.parent in present(pre(p))", true),
        ("present(pre(self.parent))", false),
        (
            "let p = if result then self.parent else pre(self.parent) in present(pre(p))",
            true,
        ),
        ("pre(self.peer) = self.peer", true),
        ("pre(self) = self", true),
        (
            "let p = pre(self.peer) in deref(p).n = 1 and deref(self.peer).n = 2",
            true,
        ),
    ] {
        let checked = checked_kind(&models, expression, ClauseKind::Postcondition);
        let mut after = draft(&models[0]);
        change_field(&mut after, "n", ValueNode::Integer { value: 2 });
        change_field(
            &mut after,
            "parent",
            ValueNode::Present {
                value: ValueId::new(3),
            },
        );
        let (offered, selected) = recorded(&models[0], draft(&models[0]), after, |_| {});
        let context = validate(
            &checked,
            offered,
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        assert_eq!(
            evaluate(&context, EvaluationLimits::default(), || false).outcome(),
            &EvaluationOutcome::Completed(truth),
            "{expression}"
        );
    }
}

#[test]
#[trace("TC-071", "FR-008-AC-6", "FR-008-AC-14")]
fn result_reference_keeps_post_capture_inside_pre() {
    let models = [authored_model(|data| {
        data["values"][2]["type"] = json!({"kind":"record", "name":"NodeRef"});
    })];
    for expression in [
        "deref(result).n = 2",
        "let r = result in pre(deref(r).n) = 2",
    ] {
        let checked = checked_kind(&models, expression, ClauseKind::Postcondition);
        let mut after = draft(&models[0]);
        change_field(&mut after, "n", ValueNode::Integer { value: 2 });
        let (offered, selected) = recorded(&models[0], draft(&models[0]), after, |invocation| {
            invocation.arena[0] = ValueNode::Reference {
                identity: object(&models[0], "self"),
            };
        });
        let context = validate(
            &checked,
            offered,
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        assert_eq!(
            evaluate(&context, EvaluationLimits::default(), || false).outcome(),
            &EvaluationOutcome::Completed(true),
            "{expression}"
        );
    }
}
