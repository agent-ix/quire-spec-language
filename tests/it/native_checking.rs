// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-006/016: real native reference/operation judgments under qualified models.

#[path = "../native_checking_cases/bindings.rs"]
mod bindings;
#[path = "../native_checking_cases/flow.rs"]
mod flow;
#[path = "../native_checking_cases/limits.rs"]
mod limits;
use crate::support::native_rule_model;
#[path = "../native_checking_cases/types.rs"]
mod types;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::checking::{
    check, CheckBindings, CheckLimits, CheckedPackage, ClauseBinding, NativeType,
};
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::linking::LinkedPackage;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::syntax::ClauseKind;
use quire_spec_language::{
    link_native, parse, Code, Diagnostic, Limits, LinkLimits, Phase, SourceIdentity,
};

fn authored_owner() -> ir::RequirementRef {
    ir::RequirementRef::new(
        ir::PackageId::new("example/authored-rules").unwrap(),
        ir::RequirementId::new("ParentRule").unwrap(),
        ir::RequirementRevision::new(7).unwrap(),
    )
}

fn request<'a>(
    models: &'a [NativeModel],
    expression: &str,
    kind: ClauseKind,
) -> Result<CheckedPackage<'a>, Box<Diagnostic>> {
    let (linked, bindings) = prepared(models, expression, kind)?;
    check(linked, bindings, CheckLimits::default())
}

fn prepared<'a>(
    models: &'a [NativeModel],
    expression: &str,
    kind: ClauseKind,
) -> Result<(LinkedPackage<'a>, CheckBindings), Box<Diagnostic>> {
    let (clause, execution_point) = match kind {
        ClauseKind::Invariant => (
            format!("invariant Rule on M::Node at current {{ {expression} }}"),
            ir::ExecutionPoint::Handler {
                name: ir::AnchorName::new("validate").unwrap(),
            },
        ),
        ClauseKind::Precondition => (
            format!("pre Rule on M::Node::step {{ {expression} }}"),
            ir::ExecutionPoint::Pre {
                operation: ir::AnchorName::new("step").unwrap(),
            },
        ),
        ClauseKind::Postcondition => (
            format!("post Rule on M::Node::step {{ {expression} }}"),
            ir::ExecutionPoint::Post {
                operation: ir::AnchorName::new("step").unwrap(),
            },
        ),
    };
    let (linked, source) = program(models, &clause)?;
    Ok((
        linked,
        CheckBindings {
            source,
            clauses: vec![ClauseBinding {
                name: "Rule".into(),
                requirement: authored_owner(),
                clause: ir::ClauseId::new("parent_order").unwrap(),
                execution_point,
            }],
        },
    ))
}

fn program<'a>(
    models: &'a [NativeModel],
    clauses: &str,
) -> Result<(LinkedPackage<'a>, FormalSource), Box<Diagnostic>> {
    let text = format!("language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"example/rule-tests\" version \"1\" digest \"{}\";\n{clauses}\n", models[0].digest());
    let unit = parse(
        SourceIdentity {
            identity: "test:native-checking".into(),
            revision: "draft:9".into(),
        },
        "native-checking.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("native test syntax must parse before its judgment");
    let source = FormalSource::new(
        unit.source().clone(),
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new("AuthoredNativeRules").unwrap(),
            ir::SourceRevision::new(9).unwrap(),
        ),
    );
    let linked = link_native(unit, models, LinkLimits::default())?;
    Ok((linked, source))
}

fn authored_model(change: impl FnOnce(&mut serde_json::Value)) -> NativeModel {
    let mut data = serde_json::from_str(native_rule_model::FIXTURE).unwrap();
    change(&mut data);
    let text = serde_json::to_string_pretty(&data).unwrap();
    native_rule_model::from_text(&text, "checker-model.json", "draft:1")
        .expect("source-derived IR setup precedes checker judgment")
        .model()
}

fn accepted(models: &[NativeModel], expression: &str, kind: ClauseKind) {
    let checked =
        request(models, expression, kind).unwrap_or_else(|error| panic!("{expression}: {error:?}"));
    assert_eq!(checked.clauses().len(), 1);
    let root = checked.linked().unit().clauses()[0].expression;
    assert_eq!(
        checked.clauses()[0].expression_type(root),
        Some(&NativeType::Boolean)
    );
    assert_eq!(checked.clauses()[0].binding().requirement, authored_owner());
    assert_eq!(
        checked.clauses()[0].proof_environment().owner(),
        &authored_owner()
    );
    assert!(checked.linked().unit().source().text().contains(expression));
}

fn refused(
    models: &[NativeModel],
    expression: &str,
    kind: ClauseKind,
    code: Code,
    locus: Option<&str>,
) {
    let error = request(models, expression, kind).unwrap_err();
    assert_eq!(error.code, code, "{expression}: {error:?}");
    assert_eq!(error.phase, Phase::Check, "{expression}");
    assert!(!error.is_incomplete());
    if let Some(locus) = locus {
        let clause_text = format!("language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"example/rule-tests\" version \"1\" digest \"{}\";\n", models[0].digest());
        // Locate against the authored test expression, independently of the AST.
        let expression_start = clause_text.len()
            + match kind {
                ClauseKind::Invariant => "invariant Rule on M::Node at current { ".len(),
                ClauseKind::Precondition => "pre Rule on M::Node::step { ".len(),
                ClauseKind::Postcondition => "post Rule on M::Node::step { ".len(),
            };
        assert_eq!(
            &expression
                [error.span.start.byte - expression_start..error.span.end.byte - expression_start],
            locus
        );
    }
    if code == Code::UndefinedExpression {
        let upstream = error.upstream.as_ref().expect("actual IR proof refusal");
        assert_eq!(upstream.code, ir::DiagnosticCode::PotentiallyUndefined);
        assert!(upstream.span.is_some());
    }
}

#[test]
#[trace("TC-025", "FR-006-AC-1", "FR-016-AC-1")]
fn tc_025_actual_reference_unwraps_need_a_preceding_presence_guard() {
    let models = [native_rule_model::parts().model()];
    for expression in [
        "deref(value(self.parent)).n < self.n",
        "deref(value(self.parent)).n < self.n and present(self.parent)",
        "(present(self.parent) or true) implies deref(value(self.parent)).n < self.n",
    ] {
        refused(
            &models,
            expression,
            ClauseKind::Invariant,
            Code::UndefinedExpression,
            Some("value(self.parent)"),
        );
    }
    for expression in [
        "present(self.parent) implies deref(value(self.parent)).n < self.n",
        "if present(self.parent) then deref(value(self.parent)).n < self.n else true",
        "not present(self.parent) or deref(value(self.parent)).n < self.n",
    ] {
        accepted(&models, expression, ClauseKind::Invariant);
    }
}

#[test]
#[trace("TC-026", "FR-006-AC-2", "FR-016-AC-2")]
fn tc_026_actual_pre_post_references_keep_their_captured_observation() {
    let models = [native_rule_model::parts().model()];
    for (expression, locus) in [
        (
            "present(pre(self.parent)) implies deref(value(self.parent)).n < self.n",
            "value(self.parent)",
        ),
        (
            "let p = self.parent in present(pre(self.parent)) implies deref(value(p)).n < self.n",
            "value(p)",
        ),
    ] {
        refused(
            &models,
            expression,
            ClauseKind::Postcondition,
            Code::UndefinedExpression,
            Some(locus),
        );
    }
    for expression in [
        "present(pre(self.parent)) implies deref(value(pre(self.parent))).n < pre(self.n)",
        "let p = self.parent in present(p) implies deref(value(pre(p))).n < self.n",
    ] {
        accepted(&models, expression, ClauseKind::Postcondition);
    }
}

#[test]
#[trace("TC-027", "FR-006-AC-3", "FR-016-AC-3")]
fn tc_027_guarded_version_addition_is_discharged_by_ir() {
    let models = [native_rule_model::parts().model()];
    let checked = request(
        &models,
        "self.n < 1000 implies self.n + 1 <= 1000",
        ClauseKind::Invariant,
    )
    .unwrap();
    let proofs = checked.clauses()[0].proofs();
    assert!(proofs.iter().any(|goal| goal
        .checked()
        .obligations()
        .iter()
        .any(|obligation| obligation.kind() == ir::DefinednessObligationKind::CheckedRange)));
    for goal in proofs {
        let native = checked
            .linked()
            .unit()
            .expression(goal.native_expression())
            .unwrap();
        assert_eq!(
            checked
                .bindings()
                .source
                .to_ir(checked.linked().unit().source(), native.span)
                .unwrap(),
            *goal.source()
        );
    }
    for expression in [
        "self.n + 1 <= 1000",
        "self.n <= 1000 implies self.n + 1 <= 1000",
        "self.n + 1 <= 1000 and self.n < 1000",
    ] {
        refused(
            &models,
            expression,
            ClauseKind::Invariant,
            Code::UndefinedExpression,
            Some("self.n + 1"),
        );
    }
}

#[test]
#[trace("TC-028", "FR-006-AC-4", "FR-016-AC-4")]
fn tc_028_literal_and_size_inference_require_an_explicit_nominal_context() {
    let models = [native_rule_model::parts().model()];
    for expression in ["1 + 2 = 3", "size(self.items) = 2"] {
        refused(
            &models,
            expression,
            ClauseKind::Invariant,
            Code::IllTyped,
            None,
        );
    }
    accepted(
        &models,
        "size(self.items) = self.count",
        ClauseKind::Invariant,
    );
    accepted(&models, "let x = 2 in x = self.n", ClauseKind::Invariant);
}

#[test]
#[trace("TC-029", "FR-006-AC-5", "FR-016-AC-5")]
fn tc_029_root_typing_includes_the_real_boolean_operation_result() {
    let models = [native_rule_model::parts().model()];
    refused(
        &models,
        "self.n",
        ClauseKind::Invariant,
        Code::IllTyped,
        Some("self.n"),
    );
    accepted(&models, "self.n = self.n", ClauseKind::Invariant);
    accepted(&models, "result", ClauseKind::Postcondition);
}

#[test]
#[trace("TC-047", "FR-016-AC-3")]
fn tc_047_signed_division_and_remainder_keep_ir_definedness() {
    let models = [native_rule_model::parts().model()];
    for expression in [
        "self.signed = -7 div 3",
        "self.signed = -7 rem 3",
        "self.den != 0 implies self.signed div self.den >= self.signed",
    ] {
        accepted(&models, expression, ClauseKind::Invariant);
    }
    for expression in [
        "self.signed div self.den = self.signed",
        "self.wide div -1 = self.wide",
    ] {
        refused(
            &models,
            expression,
            ClauseKind::Invariant,
            Code::UndefinedExpression,
            None,
        );
    }
}

#[test]
#[trace("TC-048", "FR-016-AC-4", "FR-016-AC-5")]
fn tc_048_nominal_constraints_cover_branches_units_and_operator_eligibility() {
    let models = [native_rule_model::parts().model()];
    for expression in [
        "self.distance + self.duration = self.distance",
        "self.n = self.distance",
        "self.distance div self.distance = self.distance",
        "self.distance * self.distance = self.distance",
        "self.distance rem self.distance = self.distance",
        "if true then self.n else true",
        "self.parent = self.parent",
        "self.items = self.items",
        "true < false",
        "present(self.n)",
        "value(self.n) = self.n",
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
        "(if true then 1 else self.n) = self.n",
        "self = deref(self.peer)",
        "let x = 1 in self.n < 1000 implies self.n + x <= 1000",
    ] {
        accepted(&models, expression, ClauseKind::Invariant);
    }
}

#[test]
#[trace("TC-052", "FR-016-AC-9")]
fn tc_052_checked_clauses_retain_universe_and_operation_input_requirements() {
    let models = [native_rule_model::parts().model()];
    let checked = request(
        &models,
        "present(self.parent) implies reaches(deref(value(self.parent)), self, parent)",
        ClauseKind::Invariant,
    )
    .unwrap();
    let runtime = checked.clauses()[0].runtime_requirements();
    assert_eq!(
        runtime.context.identity.key,
        quire_spec_language::linking::DeclarationKey::Type(native_rule_model::symbol("Node"))
    );
    assert_eq!(
        runtime.context_observations,
        vec![ir::StateObservation::Current]
    );
    assert_eq!(runtime.universes.len(), 1);
    assert_eq!(runtime.universes[0].object.universe.as_str(), "nodes");
    assert_eq!(
        runtime.universes[0].observations,
        vec![ir::StateObservation::Current]
    );
    assert!(std::ptr::eq(runtime.model, &models[0]));
    assert!(!runtime.validate_frame);
    for expression in ["true", "pre(self) = self"] {
        let checked = request(&models, expression, ClauseKind::Postcondition).unwrap();
        let runtime = checked.clauses()[0].runtime_requirements();
        assert_eq!(
            runtime.context_observations,
            vec![ir::StateObservation::Pre, ir::StateObservation::Post]
        );
        assert!(runtime.validate_frame);
        assert_eq!(runtime.operation.unwrap().anchor.as_str(), "step");
        assert_eq!(
            runtime.operation.unwrap().frame.fields,
            vec![(
                native_rule_model::symbol("Node"),
                native_rule_model::symbol("n")
            )]
        );
        assert_eq!(
            runtime.universes[0].observations,
            vec![ir::StateObservation::Pre, ir::StateObservation::Post]
        );
    }
    refused(
        &models,
        "reaches(pre(self), self, parent)",
        ClauseKind::Postcondition,
        Code::IllTyped,
        None,
    );
}

#[derive(Clone)]
enum Formula {
    Present,
    Zero,
    True,
    False,
    Not(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Implies(Box<Self>, Box<Self>),
}

impl Formula {
    fn evaluate(&self, present: bool, zero: bool) -> bool {
        match self {
            Self::Present => present,
            Self::Zero => zero,
            Self::True => true,
            Self::False => false,
            Self::Not(value) => !value.evaluate(present, zero),
            Self::And(a, b) => a.evaluate(present, zero) && b.evaluate(present, zero),
            Self::Or(a, b) => a.evaluate(present, zero) || b.evaluate(present, zero),
            Self::Implies(a, b) => !a.evaluate(present, zero) || b.evaluate(present, zero),
        }
    }
    fn source(&self) -> String {
        match self {
            Self::Present => "present(self.parent)".into(),
            Self::Zero => "self.n = 0".into(),
            Self::True => "true".into(),
            Self::False => "false".into(),
            Self::Not(value) => format!("not ({})", value.source()),
            Self::And(a, b) => format!("({}) and ({})", a.source(), b.source()),
            Self::Or(a, b) => format!("({}) or ({})", a.source(), b.source()),
            Self::Implies(a, b) => format!("({}) implies ({})", a.source(), b.source()),
        }
    }
}

#[test]
#[trace("TC-053", "FR-016-AC-1", "FR-016-AC-7")]
fn tc_053_native_presence_facts_are_sound_against_independent_boolean_assignments() {
    let models = [native_rule_model::parts().model()];
    let atoms = [
        Formula::Present,
        Formula::Zero,
        Formula::True,
        Formula::False,
    ];
    let mut base = atoms.to_vec();
    base.extend(
        atoms
            .iter()
            .cloned()
            .map(|value| Formula::Not(Box::new(value))),
    );
    let mut formulas = base.clone();
    for a in &base {
        for b in &base {
            formulas.extend([
                Formula::And(Box::new(a.clone()), Box::new(b.clone())),
                Formula::Or(Box::new(a.clone()), Box::new(b.clone())),
                Formula::Implies(Box::new(a.clone()), Box::new(b.clone())),
            ]);
        }
    }
    let join = Formula::Or(
        Box::new(Formula::And(
            Box::new(Formula::Present),
            Box::new(Formula::Zero),
        )),
        Box::new(Formula::And(
            Box::new(Formula::Present),
            Box::new(Formula::Not(Box::new(Formula::Zero))),
        )),
    );
    let contradiction = Formula::And(
        Box::new(Formula::Present),
        Box::new(Formula::Not(Box::new(Formula::Present))),
    );
    formulas.extend([join.clone(), contradiction.clone()]);
    let mut assignments = 0;
    let mut accepted_count = 0;
    for formula in &formulas {
        let expression = format!(
            "({}) implies deref(value(self.parent)).n < self.n",
            formula.source()
        );
        let result = request(&models, &expression, ClauseKind::Invariant);
        let safe = [false, true]
            .into_iter()
            .flat_map(|p| [false, true].into_iter().map(move |z| (p, z)))
            .fold(true, |safe, (p, z)| {
                assignments += 1;
                safe & (!formula.evaluate(p, z) || p)
            });
        match result {
            Ok(_) => {
                accepted_count += 1;
                assert!(safe, "unsafe guard accepted: {expression}");
            }
            Err(error) => {
                assert_eq!(
                    error.code,
                    Code::UndefinedExpression,
                    "{expression}: {error:?}"
                );
                assert_eq!(error.phase, Phase::Check);
                assert_eq!(
                    error.upstream.as_ref().unwrap().code,
                    ir::DiagnosticCode::PotentiallyUndefined
                );
            }
        }
    }
    assert_eq!((formulas.len(), assignments), (202, 808));
    assert!(accepted_count > 0);
    for formula in [Formula::Present, Formula::False, join, contradiction] {
        accepted(
            &models,
            &format!(
                "({}) implies deref(value(self.parent)).n < self.n",
                formula.source()
            ),
            ClauseKind::Invariant,
        );
    }
    println!("independent guard qualification: {} formulas, {assignments} assignments, {accepted_count} accepted", formulas.len());
}
