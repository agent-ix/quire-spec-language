// SPDX-License-Identifier: AGPL-3.0-only
//! TC-046/050/052: actual observations, lexical scope and evaluation order.

use super::*;
use quire_spec_language::checking::Observation;
use quire_spec_language::syntax::ExprKind;
use serde_json::json;

fn observed_names(checked: &CheckedPackage<'_>, name: &str, expected: ir::StateObservation) {
    let unit = checked.linked().unit();
    let ids = checked.linked().clauses()[0]
        .occurrences()
        .iter()
        .filter_map(|occurrence| occurrence.expression)
        .filter(|id| {
            let span = unit.expression(*id).unwrap().span;
            &unit.source().text()[span.start..span.end] == name
        })
        .collect::<Vec<_>>();
    assert!(
        !ids.is_empty(),
        "must inspect at least one actual {name} read"
    );
    for id in ids {
        assert_eq!(
            checked.clauses()[0].observation(id),
            Some(Observation::Snapshot(expected)),
            "{name}"
        );
    }
}

#[test]
#[trace("TC-046", "FR-016-AC-2")]
fn tc_046_availability_refusals_record_the_actual_native_phase() {
    let models = [native_rule_model::parts().model()];
    for (expression, kind, phase) in [
        ("result", ClauseKind::Invariant, Phase::Link),
        ("result", ClauseKind::Precondition, Phase::Check),
        ("pre(self.n) = self.n", ClauseKind::Invariant, Phase::Check),
        (
            "pre(self.n) = self.n",
            ClauseKind::Precondition,
            Phase::Check,
        ),
        ("step_result", ClauseKind::Invariant, Phase::Link),
        ("step_result", ClauseKind::Precondition, Phase::Link),
        ("step_result", ClauseKind::Postcondition, Phase::Link),
    ] {
        let error = request(&models, expression, kind).unwrap_err();
        assert_eq!(
            (error.phase, error.code),
            (phase, Code::WrongSnapshot),
            "{expression}: {error:?}"
        );
        assert!(!error.is_incomplete());
    }
    let checked = request(
        &models,
        "result and pre(self.n) = self.n",
        ClauseKind::Postcondition,
    )
    .unwrap();
    observed_names(&checked, "result", ir::StateObservation::Post);
    assert_eq!(
        checked.clauses()[0]
            .runtime_requirements()
            .operation
            .unwrap()
            .result
            .as_ref()
            .unwrap()
            .as_str(),
        "step_result"
    );
}

#[test]
#[trace("TC-046", "TC-052", "FR-016-AC-2", "FR-016-AC-9")]
fn tc_046_invocation_parameters_and_aliases_keep_their_captured_observation() {
    let models = [authored_model(|data| {
        data["values"].as_array_mut().unwrap().push(json!({
            "name": "request", "kind": "input", "type": {"kind": "record", "name": "NodeRef"}
        }));
        data["operations"][0]["parameters"] = json!(["request"]);
    })];
    for kind in [ClauseKind::Precondition, ClauseKind::Postcondition] {
        let checked = request(&models, "deref(request).n = self.n", kind).unwrap();
        observed_names(&checked, "request", ir::StateObservation::Pre);
        let runtime = checked.clauses()[0].runtime_requirements();
        assert_eq!(
            runtime.operation.unwrap().parameters,
            vec![native_rule_model::symbol("request")]
        );
        assert!(runtime.validate_frame);
        assert!(runtime.universes[0]
            .observations
            .contains(&ir::StateObservation::Pre));
    }
    for (expression, name, expected) in [
        (
            "let p = request in pre(p) = p",
            "p",
            ir::StateObservation::Pre,
        ),
        (
            "let p = self.peer in pre(p) = p",
            "p",
            ir::StateObservation::Post,
        ),
        (
            "let p = pre(self.peer) in pre(p) = p",
            "p",
            ir::StateObservation::Pre,
        ),
        (
            "let r = result in pre(r) = r",
            "r",
            ir::StateObservation::Post,
        ),
    ] {
        let checked = request(&models, expression, ClauseKind::Postcondition).unwrap();
        observed_names(&checked, name, expected);
        assert_eq!(
            checked.clauses()[0]
                .runtime_requirements()
                .context_observations,
            vec![ir::StateObservation::Pre, ir::StateObservation::Post]
        );
    }
    let error = request(&models, "request = self.peer", ClauseKind::Invariant).unwrap_err();
    assert_eq!(
        (error.phase, error.code),
        (Phase::Link, Code::MissingDeclaration)
    );
}

#[test]
#[trace("TC-050", "FR-016-AC-7")]
fn tc_050_stable_and_bound_optional_values_have_structured_guard_identity() {
    let models = [native_rule_model::parts().model()];
    for expression in [
        "let p = self.parent in present(self.parent) implies deref(value(p)).n < self.n",
        "let p = if self.n = 0 then self.parent else self.parent in present(p) implies deref(value(p)).n < self.n",
        "present(self.parent) implies (let p = value(self.parent) in deref(p).n < self.n)",
        "let g = (present(self.parent) and self.n = 0) or (present(self.parent) and self.n != 0) in g implies deref(value(self.parent)).n < self.n",
        "let g = present(self.parent) implies deref(value(self.parent)).n < self.n in g implies true",
        "(present(self.parent) and not present(self.parent)) implies deref(value(self.parent)).n < self.n",
    ] {
        accepted(&models, expression, ClauseKind::Invariant);
    }
    for (expression, locus) in [
        ("let p = value(self.parent) in present(self.parent) implies deref(p).n < self.n", "value(self.parent)"),
        ("present(if self.n = 0 then self.parent else self.parent) implies deref(value(if self.n = 0 then self.parent else self.parent)).n < self.n", "value(if self.n = 0 then self.parent else self.parent)"),
        ("((present(self.parent) and self.n = 0) or self.n != 0) implies deref(value(self.parent)).n < self.n", "value(self.parent)"),
    ] {
        refused(&models, expression, ClauseKind::Invariant, Code::UndefinedExpression, Some(locus));
    }
}

#[test]
#[trace("TC-050", "FR-016-AC-7")]
fn tc_050_unreachable_work_skips_only_definedness_and_active_locals_do_not_shadow() {
    let models = [native_rule_model::parts().model()];
    for expression in [
        "false and self.n + 1 = self.n",
        "true or self.n + 1 = self.n",
        "false implies self.n + 1 = self.n",
        "if false then deref(value(self.parent)).n < self.n else true",
        "if true then true else deref(value(self.parent)).n < self.n",
    ] {
        let checked = request(&models, expression, ClauseKind::Invariant).unwrap();
        assert!(checked.clauses()[0].proofs().is_empty(), "{expression}");
        assert!(checked.linked().unit().source().text().contains(expression));
    }
    for expression in [
        "false and self.n",
        "true or present(self.n)",
        "if false then self.n else true",
        "let p = self.parent in let p = self.parent in true",
        "let x = self.n in forall(x in self.items: true)",
        "forall(x in self.items: exists(x in self.items: true))",
    ] {
        refused(
            &models,
            expression,
            ClauseKind::Invariant,
            Code::IllTyped,
            None,
        );
    }
    for expression in [
        "false and unknown_name",
        "if true then true else unknown_name",
        "let p = p in true",
        "forall(p in p: true)",
        "forall(p in self.items: true) and p = self.n",
    ] {
        let error = request(&models, expression, ClauseKind::Invariant).unwrap_err();
        assert_eq!(
            (error.phase, error.code),
            (Phase::Link, Code::MissingDeclaration),
            "{expression}: {error:?}"
        );
    }
}

#[test]
#[trace("TC-050", "FR-016-AC-7")]
fn tc_050_quantifiers_check_arbitrary_elements_under_their_own_guards() {
    let models = [authored_model(|data| {
        data["records"][0]["fields"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "name": "parents", "type": {"kind": "sequence", "maximum": 3,
                    "value": {"kind": "option", "value": {"kind": "record", "name": "NodeRef"}}}
            }));
    })];
    for quantifier in ["forall", "exists"] {
        let expression = format!(
            "{quantifier}(p in self.parents: present(p) implies deref(value(p)).n < self.n)"
        );
        let checked = request(&models, &expression, ClauseKind::Invariant).unwrap();
        let root = checked.linked().unit().clauses()[0].expression;
        assert!(
            matches!(checked.linked().unit().expression(root).unwrap().kind, ExprKind::Quantifier { universal, .. } if universal == (quantifier == "forall"))
        );
        assert!(!checked.clauses()[0].proofs().is_empty());
        // The admitted domain may be empty at runtime. Checking still examines
        // an arbitrary element; no runtime sample participates in this judgment.
        refused(
            &models,
            &format!("{quantifier}(p in self.parents: deref(value(p)).n < self.n)"),
            ClauseKind::Invariant,
            Code::UndefinedExpression,
            Some("value(p)"),
        );
    }
}
