// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-753 (TC-2103) and FR-754 (TC-2104): S2 builds a `verify` plan into a
//! `VerificationPlan` form and a `synthesis` declaration, with or without its
//! objective, into a `Synthesis` form, on real complete-V1 source through
//! `fsl_cst::parse` and `build_unit`.

use filament_exact::Integer;
use fsl_forms::{
    build_unit, BinaryOperator, DeclarationForm, ExprNode, ExpressionForm, SynthesisDirectionForm,
    SynthesisForm,
};
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.value.complete/v1\";\n";

/// FSpec FR-192's `Release` plan.
const RELEASE: &str = "verify Release using p {\n\
       check safe claim Orders::NoDoubleRefund method model_check domain M::Order bound 20;\n\
       check tested claim Orders::NoDoubleRefund method model_based_test domain M::Order bound 500;\n\
       check timely claim Orders::RefundWithin method model_check domain M::Order depends [safe];\n\
     }";

const PICK: &str =
    "synthesis Pick using p grammar Digit domain Digit satisfies { Pick * Pick = 49 };";

const PICK_MIN: &str = "synthesis Pick using p grammar Digit domain Digit \
     minimize Pick satisfies { Pick >= 3 };";
const PICK_MAX: &str = "synthesis Pick using p grammar Digit domain Digit \
     maximize Pick satisfies { Pick <= 7 };";

/// The one declaration form S2 builds for `declaration`, or the S1/S2
/// refusal rendered by its `Debug` form.
fn built_form(declaration: &str) -> Result<DeclarationForm, String> {
    let text = format!("{HEADER}{declaration}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    if !parsed.is_admissible() {
        return Err(format!("S1: {:?}", parsed.diagnostics()));
    }
    let unit = build_unit(&parsed).map_err(|refusal| format!("S2: {refusal:?}"))?;
    let [form] = unit.forms() else {
        return Err(format!("not one form: {:?}", unit.forms()));
    };
    Ok(form.form().clone())
}

fn synthesis_form(declaration: &str) -> Result<SynthesisForm, String> {
    let text = format!("{HEADER}{declaration}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    if !parsed.is_admissible() {
        return Err(format!("S1: {:?}", parsed.diagnostics()));
    }
    let unit = build_unit(&parsed).map_err(|refusal| format!("S2: {refusal:?}"))?;
    let [form] = unit.forms() else {
        return Err(format!("not one form: {:?}", unit.forms()));
    };
    match form.form() {
        DeclarationForm::Synthesis(form) => Ok(form.as_ref().clone()),
        other => Err(format!("not a synthesis form: {other:?}")),
    }
}

/// FR-753-AC-1: `Release` builds one `VerificationPlan` form holding its
/// three steps, their methods and bounds, and `timely`'s `depends [safe]`.
#[trace("TC-2103", "FR-753-AC-1")]
#[trace("FSpec-TC-1601", "FSpec-FR-192-AC-8")]
#[test]
fn release_builds_a_verification_plan_form_with_three_steps() {
    let form =
        built_form(RELEASE).unwrap_or_else(|refusal| panic!("S2 builds `verify`: {refusal}"));
    let rendered = format!("{form:?}");
    for word in ["RefundWithin", "500"] {
        assert!(rendered.contains(word), "{word} in {rendered}");
    }
    let DeclarationForm::VerificationPlan(plan) = form else {
        panic!("S2 returns a verification plan form: {rendered}");
    };
    assert_eq!(plan.name.name, "Release");
    assert!(plan.name.span.start < plan.name.span.end);
    assert_eq!(plan.profile.alias, "p");
    assert!(plan.profile.span.start < plan.profile.span.end);
    assert!(plan.span.start < plan.span.end);
    assert_eq!(plan.steps.len(), 3);
    for (step, (name, method, bound)) in plan.steps.iter().zip([
        ("safe", "model_check", Some("20")),
        ("tested", "model_based_test", Some("500")),
        ("timely", "model_check", None),
    ]) {
        assert_eq!(step.name.name, name);
        assert!(step.name.span.start < step.name.span.end, "{name}");
        assert_eq!(step.method.name, method, "{name}");
        assert!(step.method.span.start < step.method.span.end, "{name}");
        assert_eq!(step.domain.name, "M::Order", "{name}");
        assert!(step.domain.span.start < step.domain.span.end, "{name}");
        assert!(step.claim.span.start < step.claim.span.end, "{name}");
        assert!(step.span.start < step.span.end, "{name}");
        assert_eq!(step.bound.as_deref(), bound, "{name}");
        assert_eq!(step.bound_span.is_some(), bound.is_some(), "{name}");
    }
    assert_eq!(plan.steps[0].claim.name, "Orders::NoDoubleRefund");
    assert_eq!(plan.steps[2].claim.name, "Orders::RefundWithin");
    assert!(plan.steps[0].depends.is_empty() && plan.steps[1].depends.is_empty());
    let [safe] = plan.steps[2].depends.as_slice() else {
        panic!("timely depends on one step: {:?}", plan.steps[2].depends);
    };
    assert_eq!(safe.name, "safe");
    assert!(safe.span.start < safe.span.end);
}

/// FR-754-AC-1: `Pick` builds one `Synthesis` form with its names and block.
#[trace("TC-2104", "FR-754-AC-1")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-6")]
#[test]
fn pick_builds_a_synthesis_form() {
    let form =
        synthesis_form(PICK).unwrap_or_else(|refusal| panic!("S2 builds `synthesis`: {refusal}"));
    assert_eq!(form.name.name, "Pick");
    assert!(form.name.span.start < form.name.span.end);
    assert_eq!(form.grammar.name, "Digit");
    assert!(form.grammar.span.start < form.grammar.span.end);
    assert_eq!(form.domain.name, "Digit");
    assert!(form.domain.span.start < form.domain.span.end);
    assert!(form.objective.is_none());
    assert!(form.satisfies_span.start < form.satisfies_span.end);
    assert!(form
        .satisfies
        .spans
        .span(form.satisfies.spans.root())
        .is_some());
    assert!(format!("{:?}", form.satisfies.expression).contains("Multiply"));
}

/// FR-754-AC-1: `minimize Pick` parses at S1 and the `Synthesis` form holds
/// the objective's direction.
#[trace("TC-2104", "FR-754-AC-1")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-7")]
#[test]
fn pick_with_minimize_builds_a_synthesis_form_holding_its_objective() {
    let form = synthesis_form(PICK_MIN)
        .unwrap_or_else(|refusal| panic!("S1 and S2 build the objective: {refusal}"));
    let objective = form.objective.expect("minimize is retained");
    assert_eq!(objective.direction, SynthesisDirectionForm::Minimize);
    assert!(format!("{:?}", objective.expression.expression).contains("Pick"));
    assert!(objective.span.start < objective.span.end);
    assert!(objective
        .expression
        .spans
        .span(objective.expression.spans.root())
        .is_some());
}

/// FR-754-AC-1: `maximize Pick` parses and its objective direction is retained.
#[trace("TC-2104", "FR-754-AC-1")]
#[test]
fn pick_with_maximize_builds_a_synthesis_form_holding_its_objective() {
    let form = synthesis_form(PICK_MAX)
        .unwrap_or_else(|refusal| panic!("S1 and S2 build the objective: {refusal}"));
    let objective = form.objective.expect("maximize is retained");
    assert_eq!(objective.direction, SynthesisDirectionForm::Maximize);
}

const STEP_SUM: &str = "synthesis Reach using p grammar M::G domain M::D \
     minimize Reach.steps.sum(s => s.index) + 3 satisfies { true };";

/// FSpec FR-802 "Classification": `Reach.steps.sum(s => c) + k` builds one
/// sum over the plan's `steps` field with no written result type, plus the
/// constant, each node spanned in source order.
#[trace("FSpec-TC-1952", "FSpec-FR-802-AC-1")]
#[test]
fn a_step_sum_objective_builds_a_sum_over_the_plan_steps() {
    let form = synthesis_form(STEP_SUM).unwrap_or_else(|refusal| panic!("S2 builds it: {refusal}"));
    let objective = form.objective.expect("the objective is retained");
    assert_eq!(objective.direction, SynthesisDirectionForm::Minimize);
    let ExpressionForm { expression, spans } = &objective.expression;
    let root = expression.root();
    let ExprNode::Binary {
        operator: BinaryOperator::Add,
        left,
        right,
    } = root.node()
    else {
        panic!("the root adds the constant: {expression:?}");
    };
    let ExprNode::Sum {
        result_type: None,
        binder,
        source,
        summand,
        ..
    } = root.at(*left).node()
    else {
        panic!("the left operand is the step sum: {expression:?}");
    };
    assert_eq!(binder, "s");
    assert!(matches!(
        root.at(*source).node(),
        ExprNode::Field { field, .. } if field == "steps"
    ));
    assert!(matches!(
        root.at(*summand).node(),
        ExprNode::Field { field, .. } if field == "index"
    ));
    assert!(
        matches!(root.at(*right).node(), ExprNode::Integer(three) if *three == Integer::from(3_i64))
    );
    let text = format!("{HEADER}{STEP_SUM}\n");
    let span = spans.span(spans.root()).expect("the root is spanned");
    assert_eq!(
        &text[span.start..span.end],
        "Reach.steps.sum(s => s.index) + 3"
    );
}

/// FSpec FR-802: `Reach.steps.sum(s => e)` reads only as the objective root:
/// any other aggregate, or the form inside another expression, is a syntax
/// refusal.
#[trace("FSpec-TC-1952", "FSpec-FR-802-AC-1")]
#[test]
fn a_step_sum_outside_the_objective_root_is_refused_at_s1() {
    for declaration in [
        "synthesis Reach using p grammar M::G domain M::D minimize Reach.steps.min(s => 1) satisfies { true };",
        "synthesis Reach using p grammar M::G domain M::D minimize 1 + Reach.steps.sum(s => 1) satisfies { true };",
        "synthesis Reach using p grammar M::G domain M::D satisfies { Reach.steps.sum(s => 1) > 0 };",
        "synthesis Reach using p grammar M::G domain M::D minimize Reach.steps.sum(s => 1) + Reach.length * 2 satisfies { true };",
    ] {
        let refusal = synthesis_form(declaration).expect_err(declaration);
        assert!(refusal.starts_with("S1:"), "{declaration}: {refusal}");
    }
}

/// Numeric families remain representable even when a planner must return
/// an explicit unsupported result for their execution.
#[trace("FSpec-FR-802-AC-3", "FSpec-FR-802-AC-5")]
#[test]
fn a_step_sum_retains_float_and_quantity_constant_forms() {
    for (term, constant) in [
        (
            "float64(bits: 0x3ff0000000000000)",
            "float64(bits: 0x3ff0000000000000)",
        ),
        ("30 M::seconds", "1 M::seconds"),
    ] {
        let form = synthesis_form(&format!("synthesis Reach using p grammar M::G domain M::D minimize Reach.steps.sum(s => {term}) + {constant} satisfies {{ true }};"))
            .expect("S2 preserves numeric constant syntax");
        let objective = form.objective.expect("the objective is retained");
        let root = objective.expression.expression.root();
        let ExprNode::Binary {
            operator: BinaryOperator::Add,
            left,
            right,
        } = root.node()
        else {
            panic!("the root adds the numeric constant");
        };
        assert!(matches!(
            root.at(*left).node(),
            ExprNode::Sum {
                result_type: None,
                ..
            }
        ));
        match term {
            "30 M::seconds" => assert!(matches!(root.at(*right).node(), ExprNode::Quantity(_))),
            _ => assert!(matches!(
                root.at(*right).node(),
                ExprNode::Float64(0x3ff0000000000000)
            )),
        }
    }
}
