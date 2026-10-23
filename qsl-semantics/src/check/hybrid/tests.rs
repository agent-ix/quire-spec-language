// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-447 (TC-1187, TC-1220): the S3 check of a `hybrid` declaration over
//! real source: S1 (`qsl_cst::parse`), S2 (`qsl_forms::build_unit`), the
//! assembler, then `PackageDeclarations::check`.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::SourceIdentity;
use quire_exact::Rational;
use quire_semantic_value::checking::CheckingLimits;

use crate::check::{
    ArithmeticOp, CheckCause, CheckRefusal, CheckedGraph, CheckedHybridModel, CompareOp,
    HybridTerm, ModeIndex, PackageDeclarations, ResetConstraint, VariableClass, VariableIndex,
};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"quire.temporal.infinite-trace/v1\";\n";

/// FR-447's `Braking`, with the `{}`-marked holes mutations replace.
fn braking(
    moving_invariant: &str,
    moving_flow: &str,
    transition: &str,
    stopped_flow: &str,
) -> String {
    format!(
        "hybrid Braking using p {{\n\
           mode Moving invariant {{ {moving_invariant} }}\n\
             flow {{ {moving_flow} }}\n\
             {transition}\n\
           mode Stopped invariant {{ v = 0 }}\n\
             flow {{ {stopped_flow} }}\n\
         }}\n"
    )
}

const INVARIANT: &str = "v >= 0 and v <= 20 and a >= 1 and a <= 2";
const FLOW: &str = "v' = -a; a' = 0; t' = 1;";
const TRANSITION: &str = "transition to Stopped when { v = 0 } reset { v = 0 };";
const STOPPED_FLOW: &str = "v' = 0; a' = 0; t' = 1;";

fn use_case() -> String {
    braking(INVARIANT, FLOW, TRANSITION, STOPPED_FLOW)
}

type Refusals = Vec<(CheckRefusal, Option<String>)>;

fn assemble(declarations: &str) -> (String, PackageDeclarations) {
    let text = format!("{HEADER}{declarations}");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let package = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        crate::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: the unit assembles: {refusal:?}"))
    .with_unwritten_hybrids();
    (text, package)
}

/// The checked graph, or every refusal with the source text it names.
fn check(declarations: &str) -> Result<CheckedGraph, Refusals> {
    let (text, package) = assemble(declarations);
    let regions = package.regions();
    package
        .check(CheckingLimits::default())
        .map_err(|refusals| {
            refusals
                .into_iter()
                .map(|refusal| {
                    let named = regions.refusal_region(&refusal).map(|region| {
                        text[region.start() as usize..region.end() as usize].to_owned()
                    });
                    (refusal, named)
                })
                .collect()
        })
}

fn model(declarations: &str) -> CheckedHybridModel {
    let graph = check(declarations).unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let [model] = graph.hybrid_models() else {
        panic!("one model: {:?}", graph.hybrid_models());
    };
    model.clone()
}

/// The one refusal of a unit that must not check: its code, cause tag and the
/// source text it names.
fn sole_refusal(declarations: &str) -> (Code, &'static str, String) {
    let Err(refusals) = check(declarations) else {
        panic!("`{declarations}` must not check");
    };
    let [(refusal, named)] = refusals.as_slice() else {
        panic!("one refusal for `{declarations}`: {refusals:?}");
    };
    let CheckCause::Hybrid(cause) = &refusal.cause else {
        panic!("a hybrid cause: {refusal:?}");
    };
    (cause.code(), cause.tag(), named.clone().expect("a region"))
}

fn constant(value: i64) -> HybridTerm {
    HybridTerm::Constant(Rational::from_integer(value.into()))
}

fn variable(index: usize) -> HybridTerm {
    HybridTerm::Variable(VariableIndex::new(index))
}

/// FR-447-AC-1 (S3 half): `Braking` checks to a `CheckedHybridModel` whose
/// variables are `v`, `a` and `t` in that order, classified `Hybrid`,
/// `Hybrid` and `Clock`, whose `Moving` rates are `Negate(Variable(1))`,
/// `Constant(0)` and `Constant(1)`, whose `initial` is the first mode with the
/// condition `true`, and whose transition's reset is `v = 0`.
#[trace("TC-1187", "FR-447-AC-1")]
#[test]
fn braking_checks_to_a_classified_model_with_its_rates_initial_mode_and_reset() {
    let model = model(&use_case());
    assert_eq!(model.name.as_str(), "Braking");
    let variables: Vec<_> = model
        .variables
        .iter()
        .map(|v| (v.name.as_str(), v.class))
        .collect();
    assert_eq!(
        variables,
        [
            ("v", VariableClass::Hybrid),
            ("a", VariableClass::Hybrid),
            ("t", VariableClass::Clock)
        ]
    );
    let modes: Vec<_> = model.modes.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(modes, ["Moving", "Stopped"]);
    assert_eq!(
        model.modes[0].rates,
        [
            HybridTerm::Negate(Box::new(variable(1))),
            constant(0),
            constant(1)
        ]
    );
    assert_eq!(model.initial.len(), 1);
    assert_eq!(model.initial[0].mode, ModeIndex::new(0));
    assert_eq!(model.initial[0].condition, HybridTerm::Boolean(true));
    let [transition] = model.modes[0].transitions.as_slice() else {
        panic!("one transition");
    };
    assert_eq!(transition.target, ModeIndex::new(1));
    assert_eq!(
        transition.resets,
        [ResetConstraint {
            variable: VariableIndex::new(0),
            relation: CompareOp::Eq,
            value: constant(0),
        }]
    );
}

/// FR-447-AC-2: each mutation of `Braking` refuses at its span with the cause
/// given.
#[trace("TC-1187", "FR-447-AC-2")]
#[test]
fn each_mutation_of_braking_refuses_at_its_span_with_its_cause() {
    let duplicate_mode = use_case().replace(
        "mode Stopped invariant",
        "mode Moving invariant { true } flow { v' = 0; a' = 0; t' = 1; }\n mode Stopped invariant",
    );
    let cases: Vec<(String, Code, &str, &str)> = vec![
        (
            duplicate_mode,
            Code::ConflictingDeclaration,
            "duplicate-declaration",
            "Moving",
        ),
        (
            braking(
                INVARIANT,
                FLOW,
                "transition to Halted when { v = 0 } reset { v = 0 };",
                STOPPED_FLOW,
            ),
            Code::MissingDeclaration,
            "missing-name",
            "Halted",
        ),
        (
            braking(INVARIANT, FLOW, TRANSITION, "v' = 0; a' = 0;"),
            Code::InvalidModelBinding,
            "malformed-declaration",
            "Stopped",
        ),
        (
            braking(
                INVARIANT,
                "v' = -a; v' = 0; a' = 0; t' = 1;",
                TRANSITION,
                STOPPED_FLOW,
            ),
            Code::ConflictingDeclaration,
            "duplicate-declaration",
            "v",
        ),
        (
            braking(
                INVARIANT,
                "P::v' = 0; v' = -a; a' = 0; t' = 1;",
                TRANSITION,
                STOPPED_FLOW,
            ),
            Code::InvalidModelBinding,
            "malformed-declaration",
            "P::v",
        ),
        (
            braking("v", FLOW, TRANSITION, STOPPED_FLOW),
            Code::IllTyped,
            "non-boolean-root",
            "v",
        ),
        (
            braking(
                INVARIANT,
                FLOW,
                "transition to Stopped when { v = 0 } reset { v + a = 1 };",
                STOPPED_FLOW,
            ),
            Code::InvalidModelBinding,
            "malformed-declaration",
            "v + a = 1",
        ),
        (
            braking(
                INVARIANT,
                FLOW,
                "transition to Stopped when { v = 0 } reset { v = 0 and v = 1 };",
                STOPPED_FLOW,
            ),
            Code::ConflictingDeclaration,
            "duplicate-declaration",
            "v = 1",
        ),
        (
            braking(
                INVARIANT,
                FLOW,
                "transition to Stopped when { w = 0 } reset { v = 0 };",
                STOPPED_FLOW,
            ),
            Code::MissingDeclaration,
            "missing-name",
            "w",
        ),
        (
            braking(
                INVARIANT,
                FLOW,
                "transition to Stopped when { v + 1 } reset { v = 0 };",
                STOPPED_FLOW,
            ),
            Code::IllTyped,
            "non-boolean-root",
            "v + 1",
        ),
        (
            braking(
                INVARIANT,
                "v' = true; a' = 0; t' = 1;",
                TRANSITION,
                STOPPED_FLOW,
            ),
            Code::IllTyped,
            "type-mismatch",
            "true",
        ),
        (
            braking(
                INVARIANT,
                "v' = f(v); a' = 0; t' = 1;",
                TRANSITION,
                STOPPED_FLOW,
            ),
            Code::MissingDeclaration,
            "missing-name",
            "f(v)",
        ),
        (
            braking(
                INVARIANT,
                "v' = v div 2; a' = 0; t' = 1;",
                TRANSITION,
                STOPPED_FLOW,
            ),
            Code::IllTyped,
            "type-mismatch",
            "v div 2",
        ),
        (
            "hybrid Timer using p { mode A invariant { true } flow { t' = 1; } \
             mode B invariant { true } flow { t' = 1; } }\n"
                .to_owned(),
            Code::InvalidTimedModel,
            "rate-one-hybrid",
            "Timer",
        ),
    ];
    for (source, code, tag, named) in cases {
        assert_eq!(
            sole_refusal(&source),
            (code, tag, named.to_owned()),
            "{source}"
        );
    }
}

/// FR-447 "The clock line": a rate written `2 - 1` or `rational(2, 2)` is not
/// the literal `1`, so its variable is hybrid dynamics.
#[test]
fn a_rate_that_is_not_the_literal_one_is_hybrid_dynamics() {
    for rate in ["2 - 1", "rational(2, 2)"] {
        let source = format!(
            "hybrid H using p {{ mode A invariant {{ true }} flow {{ x' = {rate}; y' = 1; }} }}\n"
        );
        let model = model(&source);
        let classes: Vec<_> = model.variables.iter().map(|v| v.class).collect();
        assert_eq!(
            classes,
            [VariableClass::Hybrid, VariableClass::Clock],
            "{rate}"
        );
    }
}

fn braking2() -> String {
    "hybrid Braking2 using p {\n\
       initial {\n\
         mode Moving when { v = 20 and a >= 1 and a <= 2 and t = 0 };\n\
         mode Stopped when { v = 0 and t = 0 };\n\
       }\n\
       mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
         flow { v' = -a; a' = 0; t' = 1; }\n\
         transition to Stopped when { v = 0 } reset { v = 0 and a >= 1 };\n\
       mode Stopped invariant { v = 0 }\n\
         flow { v' = 0; a' = 0; t' = 1; }\n\
     }\n"
    .to_owned()
}

/// FR-447-AC-6: `Braking2` checks to two initial entries in source order, so
/// `Moving` and `Stopped` are both initial modes; writing the default entry
/// checks to the same model as writing none; a missing mode and a non-Boolean
/// condition refuse at their spans; and `initial` stays an identifier as a
/// variable name.
#[trace("TC-1220", "FR-447-AC-6")]
#[test]
fn initial_entries_check_in_source_order_and_the_default_entry_is_the_unwritten_one() {
    let model = model(&braking2());
    let entries: Vec<_> = model.initial.iter().map(|e| e.mode).collect();
    assert_eq!(entries, [ModeIndex::new(0), ModeIndex::new(1)]);
    assert_ne!(model.initial[0].condition, HybridTerm::Boolean(true));

    let written = use_case().replace(
        "mode Moving invariant",
        "initial { mode Moving when { true }; }\n mode Moving invariant",
    );
    assert_eq!(model_of(&written), model_of(&use_case()));

    let halted = use_case().replace(
        "mode Moving invariant",
        "initial { mode Halted when { true }; }\n mode Moving invariant",
    );
    assert_eq!(
        sole_refusal(&halted),
        (
            Code::MissingDeclaration,
            "missing-name",
            "Halted".to_owned()
        )
    );
    let non_boolean = use_case().replace(
        "mode Moving invariant",
        "initial { mode Moving when { v }; }\n mode Moving invariant",
    );
    assert_eq!(
        sole_refusal(&non_boolean),
        (Code::IllTyped, "non-boolean-root", "v".to_owned())
    );

    let identifier = "hybrid H using p { mode M invariant { initial >= 0 } \
                      flow { initial' = 1; y' = 2; } }\n";
    assert_eq!(model_of(identifier).variables[0].name.as_str(), "initial");
}

fn model_of(declarations: &str) -> CheckedHybridModel {
    model(declarations)
}

/// FR-447 "Invariants, guards, resets and expression forms": the admitted
/// forms lower to the terms the rules give, folding no constant.
#[test]
fn expression_forms_lower_to_their_terms_without_folding() {
    let rate = |flow: &str| -> HybridTerm {
        let source = braking(INVARIANT, flow, TRANSITION, STOPPED_FLOW);
        model(&source).modes[0].rates[0].clone()
    };
    assert_eq!(
        rate("v' = -a * v; a' = 0; t' = 1;"),
        HybridTerm::Arithmetic(
            ArithmeticOp::Mul,
            Box::new(HybridTerm::Negate(Box::new(variable(1)))),
            Box::new(variable(0))
        )
    );
    assert_eq!(
        rate("v' = let k = a * v in -k; a' = 0; t' = 1;"),
        HybridTerm::Negate(Box::new(HybridTerm::Arithmetic(
            ArithmeticOp::Mul,
            Box::new(variable(1)),
            Box::new(variable(0))
        )))
    );
    assert!(matches!(
        rate("v' = if v > 1 then 0 else -1; a' = 0; t' = 1;"),
        HybridTerm::Conditional(..)
    ));
    let guard = |guard: &str| -> HybridTerm {
        let transition = format!("transition to Stopped when {{ {guard} }} reset {{ v = 0 }};");
        model(&braking(INVARIANT, FLOW, &transition, STOPPED_FLOW)).modes[0].transitions[0]
            .guard
            .clone()
    };
    assert_eq!(
        guard("v * v >= 2"),
        HybridTerm::Compare(
            CompareOp::Ge,
            Box::new(HybridTerm::Arithmetic(
                ArithmeticOp::Mul,
                Box::new(variable(0)),
                Box::new(variable(0))
            )),
            Box::new(constant(2))
        )
    );
    assert_eq!(
        guard("v != 3"),
        HybridTerm::Compare(CompareOp::Ne, Box::new(variable(0)), Box::new(constant(3)))
    );
}

/// FR-447-AC-8: resets check to their conjuncts in declared order, `pre(v)`
/// and a bare `v` both read the pre-jump value, `let` substitutes, and a
/// duplicate or non-conjunctive reset refuses.
#[trace("TC-1220", "FR-447-AC-8")]
#[test]
fn resets_check_to_conjuncts_over_the_pre_jump_state() {
    let resets = |reset: &str| -> Vec<ResetConstraint> {
        let transition = format!("transition to Stopped when {{ v = 0 }} reset {{ {reset} }};");
        model(&braking(INVARIANT, FLOW, &transition, STOPPED_FLOW)).modes[0].transitions[0]
            .resets
            .clone()
    };
    assert_eq!(
        resets("v = pre(v) / 2 and a >= 1 and a <= 2"),
        [
            ResetConstraint {
                variable: VariableIndex::new(0),
                relation: CompareOp::Eq,
                value: HybridTerm::Arithmetic(
                    ArithmeticOp::Div,
                    Box::new(variable(0)),
                    Box::new(constant(2))
                ),
            },
            ResetConstraint {
                variable: VariableIndex::new(1),
                relation: CompareOp::Ge,
                value: constant(1),
            },
            ResetConstraint {
                variable: VariableIndex::new(1),
                relation: CompareOp::Le,
                value: constant(2),
            },
        ]
    );
    assert_eq!(
        resets("v = v + 1"),
        [ResetConstraint {
            variable: VariableIndex::new(0),
            relation: CompareOp::Eq,
            value: HybridTerm::Arithmetic(
                ArithmeticOp::Add,
                Box::new(variable(0)),
                Box::new(constant(1))
            ),
        }]
    );
    assert_eq!(
        resets("let k = 2 in v = k"),
        [ResetConstraint {
            variable: VariableIndex::new(0),
            relation: CompareOp::Eq,
            value: constant(2),
        }]
    );
    assert!(resets("true").is_empty());

    let refusal = |reset: &str| {
        let transition = format!("transition to Stopped when {{ v = 0 }} reset {{ {reset} }};");
        sole_refusal(&braking(INVARIANT, FLOW, &transition, STOPPED_FLOW))
    };
    assert_eq!(
        refusal("v = 0 and v >= 0"),
        (
            Code::ConflictingDeclaration,
            "duplicate-declaration",
            "v >= 0".to_owned()
        )
    );
    assert_eq!(
        refusal("v = 0 or v = 1"),
        (
            Code::InvalidModelBinding,
            "malformed-declaration",
            "v = 0 or v = 1".to_owned()
        )
    );
}

/// `pre(v)` outside a reset refuses `invalid_model_binding`/
/// `malformed-declaration` at its span.
#[test]
fn pre_outside_a_reset_refuses() {
    let transition = "transition to Stopped when { pre(v) = 0 } reset { v = 0 };";
    assert_eq!(
        sole_refusal(&braking(INVARIANT, FLOW, transition, STOPPED_FLOW)),
        (
            Code::InvalidModelBinding,
            "malformed-declaration",
            "pre(v)".to_owned()
        )
    );
}
