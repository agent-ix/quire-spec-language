// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-800 to FR-802: a plan synthesis types the plan value its goal and
//! objective read (`Reach.length`, `.final`, `.initial`, `.steps`), refuses a
//! plan length that is not a non-negative integer range, and refuses a read
//! of a member the plan value does not have.

use filament_exact::ValueType;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_semantics::check::{CheckedSynthesis, PackageDeclarations};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::package_input_from_sources;
use ix_trace_rs::trace;

const SHOP: &str = "language \"ix:native\" edition \"1-draft\";
domain \"example/shop\";
object Order identity (id) {
  id: Int[0, 999];
  total: Int[0, 100];
  operation pay(): Boolean modifies self.total {
    pre CanPay { self.total > 0 };
    post Paid { self.total = pre(self.total) - 1 };
  }
}
population orders: Order;
";

/// Checks a unit declaring `Lengths` as `lengths` and one plan synthesis with
/// `objective` and `goal`; the checked plan synthesis, or the refusals.
fn check(lengths: &str, objective: &str, goal: &str) -> Result<CheckedSynthesis, String> {
    check_with_types(lengths, objective, goal, "")
}

fn check_with_types(
    lengths: &str,
    objective: &str,
    goal: &str,
    types: &str,
) -> Result<CheckedSynthesis, String> {
    let model = fsl_cst::diagnostic::read_source(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:shop-model"),
        "shop.fsl",
        SHOP.as_bytes(),
        fsl_cst::Limits::default().source_bytes,
    )
    .expect("the model source is admitted");
    let packages =
        package_input_from_sources([("example/shop", &model)]).expect("the model unit compiles");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.temporal.infinite-trace/v1\";\n\
         model M = \"example/shop\";\n\
         type Lengths = {lengths};\n\
         {types}\n\
         synthesis Reach using v grammar M::Order domain Lengths {objective} \
         satisfies {{ {goal} }};\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "plan-value"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = fsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .map_err(|refusal| format!("{refusal:?}"))?;
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        models,
        Vec::new(),
    )
    .map_err(|refusal| format!("{refusal:?}"))?;
    declarations
        .link_dispatch(ModelNormalizationLimits::default())
        .map_err(|refusal| format!("{refusal:?}"))?;
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .map_err(|refusals| format!("{refusals:?}"))?;
    Ok(graph
        .syntheses()
        .first()
        .expect("the unit declares one synthesis")
        .clone())
}

#[trace("FSpec-FR-800-AC-1", "FSpec-TC-1950")]
#[test]
fn a_plan_goal_reads_the_typed_plan_value() {
    let checked = check(
        "Int[0, 3]",
        "",
        "Reach.length >= 1 and Reach.final.total >= 0 and Reach.initial.total >= 0",
    )
    .expect("the goal reads the length and the state records");
    let plan = checked
        .plan()
        .expect("a plan synthesis carries its plan value");
    assert_eq!(
        plan.shape()
            .operations
            .iter()
            .map(|operation| operation.name.as_str())
            .collect::<Vec<_>>(),
        ["pay"]
    );
}

#[trace("FSpec-FR-802-AC-1", "FSpec-TC-1952")]
#[test]
fn a_plan_objective_is_typed_without_an_expected_domain_type() {
    let checked = check("Int[0, 3]", "minimize Reach.length", "Reach.length >= 1")
        .expect("`minimize Reach.length` checks");
    assert!(checked.objective().is_some());
    assert_eq!(checked.objective_direction(), Some("minimize"));
}

#[trace("FSpec-FR-806-AC-2", "FSpec-TC-1956")]
#[test]
fn a_plan_declaration_refuses_what_the_plan_value_does_not_have() {
    let refusals = check("Int[-1, 3]", "", "Reach.length >= 1")
        .expect_err("a negative plan length is refused");
    assert!(refusals.contains("TypeMismatch"), "{refusals}");
    let refusals = check("Int[0, 3]", "minimize Reach.elapsed", "Reach.length >= 1")
        .expect_err("an untimed plan has no `elapsed`");
    assert!(refusals.contains("MissingName(\"elapsed\")"), "{refusals}");
    let refusals =
        check("Int[0, 3]", "", "Reach.unknown = 0").expect_err("an unknown plan field is refused");
    assert!(refusals.contains("MissingName(\"unknown\")"), "{refusals}");
}

#[trace("FSpec-FR-802-AC-1", "FSpec-TC-1952")]
#[test]
fn a_step_sum_types_its_binder_as_the_step_record() {
    let checked = check(
        "Int[0, 3]",
        "minimize Reach.steps.sum(s => s.index + s.after.total)",
        "Reach.length >= 1",
    )
    .expect("`s` is the plan step record with `index` and `after`");
    let objective = checked.objective().expect("the objective is checked");
    assert_eq!(objective.root().value_type(), &ValueType::Integer);
    let refusals = check(
        "Int[0, 3]",
        "minimize Reach.steps.sum(s => s.cost)",
        "Reach.length >= 1",
    )
    .expect_err("the step record has no `cost`");
    assert!(refusals.contains("cost"), "{refusals}");
}

#[trace("FSpec-FR-802-AC-1", "FSpec-FR-802-AC-7", "FSpec-TC-1952")]
#[test]
fn a_decimal_step_sum_retains_its_complete_declared_summand_type() {
    for term in [
        "let cost: Cost = if s.index < 2 then 0.10 else 0.05 in cost",
        "convert<Cost>(0.10)",
        "convert<Cost>(0.10) + 0.05",
    ] {
        let checked = check_with_types(
            "Int[0, 3]",
            &format!("minimize Reach.steps.sum(s => {term})"),
            "Reach.length >= 1",
            "type Cost = Decimal[0,10000;2,2;exact];",
        )
        .expect("an FR-140 typed context supplies the complete decimal type");
        let objective = checked.objective().expect("the objective is checked");
        let ValueType::Decimal(total) = objective.root().value_type() else {
            panic!(
                "the total is a decimal: {:?}",
                objective.root().value_type()
            );
        };
        assert_eq!(total.lower().to_string(), "0");
        assert_eq!(total.upper().to_string(), "10000");
        assert_eq!((total.min_scale(), total.max_scale()), (2, 2));
        assert_eq!(total.rounding(), filament_exact::RoundingMode::Exact);
    }
}

#[trace("FSpec-FR-802-AC-8")]
#[test]
fn a_decimal_step_sum_without_a_typed_context_refuses_ambiguity() {
    for term in [
        "if s.index < 2 then 0.10 else 0.05",
        "if s.index < 2 then decimal(10,2) else decimal(5,2)",
        "let cost = 0.10 in cost",
    ] {
        let refusals = check(
            "Int[0, 3]",
            &format!("minimize Reach.steps.sum(s => {term})"),
            "true",
        )
        .expect_err("the objective and decimal spelling cannot supply a type");
        assert!(refusals.contains("AmbiguousLiteral"), "{term}: {refusals}");
    }
}

#[trace("FSpec-FR-802-AC-1", "FSpec-FR-194-AC-9", "FSpec-TC-1952")]
#[test]
fn a_step_sum_whose_term_is_not_numeric_is_ill_typed() {
    for term in ["true", "s.operation", "s.after"] {
        let refusals = check(
            "Int[0, 3]",
            &format!("minimize Reach.steps.sum(s => {term})"),
            "Reach.length >= 1",
        )
        .expect_err(term);
        assert!(refusals.contains("TypeMismatch"), "{term}: {refusals}");
    }
}
