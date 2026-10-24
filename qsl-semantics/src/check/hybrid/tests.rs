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
    ArithmeticOp, CaptureIndex, CheckCause, CheckRefusal, CheckedFunctionRef, CheckedGraph,
    CheckedHybridClaim, CheckedHybridModel, CompareOp, HybridActivation, HybridCause, HybridClock,
    HybridFormula, HybridHorizon, HybridInterval, HybridTerm, ModeIndex, PackageDeclarations,
    ResetConstraint, TemporalProfile, TransitionIndex, VariableClass, VariableIndex,
};
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.value.complete/v1\";\n\
     profile p = \"quire.temporal.infinite-trace/v1\";\n\
     profile t = \"quire.temporal.timed/v1\";\n\
     profile e = \"quire.temporal.event-position.false-extension/v1\";\n\
     dimension Time;\n\
     dimension Mass;\n\
     unit s : Time = rational(1, 1);\n\
     unit ms : Time = rational(1, 1000) * s;\n\
     unit kg : Mass = rational(1, 1);\n";

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
    .unwrap_or_else(|refusal| panic!("{text}: the unit assembles: {refusal:?}"));
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

/// The one hybrid cause of a unit that must not check.
fn sole_cause(declarations: &str) -> HybridCause {
    let Err(refusals) = check(declarations) else {
        panic!("`{declarations}` must not check");
    };
    let [(refusal, _)] = refusals.as_slice() else {
        panic!("one refusal for `{declarations}`: {refusals:?}");
    };
    let CheckCause::Hybrid(cause) = &refusal.cause else {
        panic!("a hybrid cause: {refusal:?}");
    };
    (**cause).clone()
}

/// The source text `span` of `declarations` names.
fn spanned(declarations: &str, span: qsl_foundation::Span) -> String {
    format!("{HEADER}{declarations}")[span.start..span.end].to_owned()
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

/// FR-447-AC-2, AC-8: the duplicate refusals name both declarations, the
/// first as well as the second, and the missing-equation refusal names the
/// mode and the variable.
#[trace("TC-1187", "FR-447-AC-2")]
#[test]
fn duplicate_and_missing_refusals_name_both_declarations() {
    let duplicate_mode = use_case().replace(
        "mode Stopped invariant",
        "mode Moving invariant { true } flow { v' = 0; a' = 0; t' = 1; }\n mode Stopped invariant",
    );
    let HybridCause::DuplicateMode { name, first, span } = sole_cause(&duplicate_mode) else {
        panic!("a duplicate mode");
    };
    assert_eq!(name, "Moving");
    assert_ne!(first, span);
    assert_eq!(spanned(&duplicate_mode, first), "Moving");
    assert_eq!(spanned(&duplicate_mode, span), "Moving");
    assert!(first.start < span.start);

    let equation = braking(
        INVARIANT,
        "v' = -a; v' = 0; a' = 0; t' = 1;",
        TRANSITION,
        STOPPED_FLOW,
    );
    let HybridCause::DuplicateEquation {
        variable,
        first,
        span,
    } = sole_cause(&equation)
    else {
        panic!("a duplicate equation");
    };
    assert_eq!(variable, "v");
    assert_eq!(spanned(&equation, first), "v");
    assert_eq!(spanned(&equation, span), "v");
    assert!(first.start < span.start);

    let transition = "transition to Stopped when { v = 0 } reset { v = 0 and v = 1 };";
    let reset = braking(INVARIANT, FLOW, transition, STOPPED_FLOW);
    let HybridCause::DuplicateReset {
        variable,
        first,
        span,
    } = sole_cause(&reset)
    else {
        panic!("a duplicate reset");
    };
    assert_eq!(variable, "v");
    assert_eq!(spanned(&reset, first), "v = 0");
    assert_eq!(spanned(&reset, span), "v = 1");

    let missing = braking(INVARIANT, FLOW, TRANSITION, "v' = 0; a' = 0;");
    let HybridCause::MissingEquation {
        mode,
        variable,
        span,
    } = sole_cause(&missing)
    else {
        panic!("a missing equation");
    };
    assert_eq!((mode.as_str(), variable.as_str()), ("Stopped", "t"));
    assert_eq!(spanned(&missing, span), "Stopped");
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

// ---------------------------------------------------------------- claims

/// FR-447's `SlowEnough` over `Braking`, with `body` as its formula.
fn slow_enough(body: &str) -> String {
    format!(
        "{}temporal SlowEnough using p over (h: Braking) clock \"model-time\" on origin {{ {body} }}\n",
        use_case()
    )
}

fn claim_of(declarations: &str) -> CheckedHybridClaim {
    let graph = check(declarations).unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let claims = graph.hybrid_claims();
    let [claim] = claims else {
        panic!("one claim: {claims:?}");
    };
    claim.clone()
}

fn claim_refusal(declarations: &str) -> (Code, &'static str, String) {
    sole_refusal(declarations)
}

fn rational(numerator: i64, denominator: i64) -> quire_exact::Rational {
    quire_exact::Rational::new(numerator.into(), denominator.into()).expect("a denominator")
}

fn closed(lower: i64, upper: i64) -> HybridInterval {
    HybridInterval {
        lower: rational(lower, 1),
        lower_open: false,
        upper: Some(rational(upper, 1)),
        upper_open: false,
    }
}

fn holds_below_30() -> HybridFormula {
    HybridFormula::Holds(HybridTerm::Compare(
        CompareOp::Lt,
        Box::new(variable(0)),
        Box::new(constant(30)),
    ))
}

/// FR-447-AC-3: `SlowEnough` checks to a `CheckedHybridClaim` over `Braking`
/// with clock `ModelTime`, activation `Origin`, no capture and no fairness,
/// the stated formula and horizon `Seconds(10)`; the nested, `until` and
/// unbounded bodies give horizons 15, 4, `None` and `None`; `clock "wall"`
/// and `h.w` refuse.
#[trace("TC-1187", "FR-447-AC-3")]
#[test]
fn slow_enough_checks_to_a_claim_with_its_horizon() {
    let claim = claim_of(&slow_enough("always [0, 10] holds(h.v < 30)"));
    assert_eq!(claim.clock, HybridClock::ModelTime);
    assert_eq!(claim.activation, HybridActivation::Origin);
    assert!(claim.captures.is_empty() && claim.fairness.is_empty());
    assert_eq!(
        claim.formula,
        HybridFormula::Always(Some(closed(0, 10)), Box::new(holds_below_30()))
    );
    assert_eq!(claim.horizon, Some(HybridHorizon::Seconds(rational(10, 1))));
    assert_eq!(claim.profile, TemporalProfile::InfiniteTrace);

    let horizon = |body: &str| claim_of(&slow_enough(body)).horizon;
    assert_eq!(
        horizon("always [0, 10] eventually [0, 5] holds(h.v < 30)"),
        Some(HybridHorizon::Seconds(rational(15, 1)))
    );
    assert_eq!(
        horizon("holds(h.v < 30) until [0, 4] holds(h.v = 0)"),
        Some(HybridHorizon::Seconds(rational(4, 1)))
    );
    assert_eq!(horizon("always holds(h.v < 30)"), None);
    assert_eq!(horizon("always [0, *] holds(h.v < 30)"), None);

    let wall = slow_enough("always [0, 10] holds(h.v < 30)").replace("model-time", "wall");
    assert_eq!(
        claim_refusal(&wall),
        (
            Code::UnsupportedConstruct,
            "declaration-form",
            "clock \"wall\"".to_owned()
        )
    );
    assert_eq!(
        claim_refusal(&slow_enough("always [0, 10] holds(h.w < 30)")),
        (Code::MissingDeclaration, "missing-name", "h.w".to_owned())
    );
}

/// FR-447: a hyper expression in a hybrid claim is refused at S3 at the
/// expression, after S2 has preserved its syntax.
#[trace("TC-1220", "FR-447-AC-3")]
#[test]
fn hybrid_claim_hyper_expression_refuses_at_s3() {
    assert_eq!(
        claim_refusal(&slow_enough("holds(h.v @ a > 0)")),
        (
            Code::UnsupportedConstruct,
            "expression-form",
            "h.v @ a > 0".to_owned()
        )
    );
}

const SLOWS: &str = "temporal Slows using t over (h: Braking2) clock \"model-time\"\n\
       on each (q: Braking2) when (q.v > 10) {\n\
     fair Moving::Stopped;\n\
     capture v0: Rational[0, 20; 1, 1] = q.v;\n\
       eventually [0 s, 10500 ms] holds(h.v < v0 and h.v <= 10)\n\
     }\n";

fn with_braking2(claims: &str) -> String {
    format!("{}{claims}", braking2())
}

/// FR-447-AC-9: `Slows` checks to a claim with the `Each` activation and its
/// guard, one capture over the activation read, the weak whole fairness over
/// `Moving`'s one transition, a body reading the capture, and horizon `None`;
/// the same clause with no `clock` member and with `clock "model-time"` check
/// equal; `fair strong each` resolves; and each mutation refuses.
#[trace("TC-1220", "FR-447-AC-9")]
#[test]
fn slows_checks_activation_captures_and_fairness() {
    let claim = claim_of(&with_braking2(SLOWS));
    assert_eq!(claim.clock, HybridClock::ModelTime);
    assert_eq!(claim.profile, TemporalProfile::Timed);
    let HybridActivation::Each { parameter, guard } = &claim.activation else {
        panic!("an each activation: {:?}", claim.activation);
    };
    assert_eq!(parameter.as_str(), "q");
    assert_eq!(
        guard,
        &Some(HybridTerm::Compare(
            CompareOp::Gt,
            Box::new(HybridTerm::ActivationVariable(VariableIndex::new(0))),
            Box::new(constant(10))
        ))
    );
    let [capture] = claim.captures.as_slice() else {
        panic!("one capture: {:?}", claim.captures);
    };
    assert_eq!(capture.name.as_str(), "v0");
    assert_eq!(
        capture.value,
        HybridTerm::ActivationVariable(VariableIndex::new(0))
    );
    assert_eq!(claim.fairness.len(), 1);
    assert_eq!(claim.fairness[0].kind, FairnessKind::Weak);
    assert_eq!(claim.fairness[0].granularity, FairnessGranularity::Whole);
    assert_eq!(claim.fairness[0].named_target, Some(ModeIndex::new(1)));
    assert_eq!(
        claim.fairness[0].transitions,
        [(ModeIndex::new(0), TransitionIndex::new(0))]
    );
    let HybridFormula::Eventually(Some(interval), body) = &claim.formula else {
        panic!("eventually: {:?}", claim.formula);
    };
    assert_eq!(interval.upper, Some(rational(21, 2)));
    let HybridFormula::Holds(HybridTerm::Logic(_, left, _)) = &**body else {
        panic!("a holds: {body:?}");
    };
    assert_eq!(
        **left,
        HybridTerm::Compare(
            CompareOp::Lt,
            Box::new(variable(0)),
            Box::new(HybridTerm::Capture(CaptureIndex::new(0)))
        )
    );
    assert_eq!(claim.horizon, None);

    let without_clock = SLOWS.replace(" clock \"model-time\"", "");
    assert_eq!(claim_of(&with_braking2(&without_clock)), claim);

    let strong = SLOWS.replace("fair Moving::Stopped;", "fair strong each Moving;");
    let strong = claim_of(&with_braking2(&strong));
    assert_eq!(strong.fairness[0].kind, FairnessKind::Strong);
    assert_eq!(strong.fairness[0].granularity, FairnessGranularity::Each);
    assert_eq!(strong.fairness[0].named_target, None);
    assert_eq!(
        strong.fairness[0].transitions,
        [(ModeIndex::new(0), TransitionIndex::new(0))]
    );

    let refused = |text: String, code, tag, named: &str| {
        assert_eq!(
            claim_refusal(&with_braking2(&text)),
            (code, tag, named.to_owned()),
            "{text}"
        );
    };
    refused(
        SLOWS.replace("fair Moving::Stopped;", "fair Moving::Halted;"),
        Code::MissingDeclaration,
        "missing-name",
        "Moving::Halted",
    );
    refused(
        SLOWS.replace("on each (q: Braking2)", "on each (q: Braking)"),
        Code::IllTyped,
        "type-mismatch",
        "Braking",
    );
    refused(
        SLOWS.replace(
            "capture v0: Rational[0, 20; 1, 1] = q.v;",
            "capture v0: Rational[0, 20; 1, 1] = q.v;\ncapture v0: Boolean = true;",
        ),
        Code::ConflictingDeclaration,
        "duplicate-declaration",
        "v0",
    );
    refused(
        SLOWS.replace("Rational[0, 20; 1, 1] = q.v", "Text[0, 3; nfc] = q.v"),
        Code::IllTyped,
        "type-mismatch",
        "Text[0, 3; nfc]",
    );
    refused(
        SLOWS
            .replace("using t", "using e")
            .replace("[0 s, 10500 ms]", "[0, 10]"),
        Code::UnsupportedConstruct,
        "expression-form",
        "fair Moving::Stopped;",
    );
}

/// FR-447-AC-10: past operators lower with their intervals and the reach
/// rule gives their horizons; `model-steps` counts positions.
#[trace("TC-1220", "FR-447-AC-10")]
#[test]
fn past_operators_lower_and_their_horizons_follow_the_reach_rule() {
    let claim = claim_of(&slow_enough(
        "always [0, 10] (holds(h.v = 0) implies once [0, 2] holds(h.v <= 1))",
    ));
    let HybridFormula::Always(_, body) = &claim.formula else {
        panic!("always: {:?}", claim.formula);
    };
    let HybridFormula::Logic(crate::check::LogicOp::Implies, _, once) = &**body else {
        panic!("implies: {body:?}");
    };
    assert!(matches!(&**once, HybridFormula::Once(Some(interval), _) if *interval == closed(0, 2)));
    assert_eq!(claim.horizon, Some(HybridHorizon::Seconds(rational(10, 1))));

    let horizon = |body: &str| claim_of(&slow_enough(body)).horizon;
    assert_eq!(
        horizon("always [0, 10] once [2, 3] holds(h.v < 30)"),
        Some(HybridHorizon::Seconds(rational(10, 1)))
    );
    assert_eq!(
        horizon("holds(h.v > 0) since [0, 3] eventually [0, 4] holds(h.v = 20)"),
        Some(HybridHorizon::Seconds(rational(4, 1)))
    );
    assert_eq!(
        horizon("historically holds(h.v <= 20)"),
        Some(HybridHorizon::Seconds(rational(0, 1)))
    );
    assert_eq!(
        horizon("holds(h.v = 0) triggered holds(h.v <= 20)"),
        Some(HybridHorizon::Seconds(rational(0, 1)))
    );
    let steps =
        slow_enough("eventually [0, 1] holds(h.v = 0)").replace("model-time", "model-steps");
    assert_eq!(claim_of(&steps).horizon, Some(HybridHorizon::Positions(1)));
}

fn timed_claim(body: &str) -> String {
    slow_enough(body).replace("using p", "using t")
}

/// FR-447-AC-11: timed intervals check to exact rational seconds, either end
/// open or closed, `[q, *]`; the integer profile admits only whole closed
/// bounds; bad dimensions, empty intervals and `model-steps` refuse.
#[trace("TC-1220", "FR-447-AC-11")]
#[test]
fn timed_intervals_check_to_exact_rational_seconds_by_profile() {
    let interval = |text: &str| -> HybridInterval {
        let claim = claim_of(&timed_claim(text));
        match claim.formula {
            HybridFormula::Eventually(Some(interval), _)
            | HybridFormula::Always(Some(interval), _) => interval,
            other => panic!("an interval: {other:?}"),
        }
    };
    assert_eq!(
        interval("eventually [0 s, 1500 ms] holds(h.v < 19)"),
        HybridInterval {
            lower: rational(0, 1),
            lower_open: false,
            upper: Some(rational(3, 2)),
            upper_open: false
        }
    );
    assert_eq!(
        interval("always (0.5 s, 2 s] holds(h.v <= 20)"),
        HybridInterval {
            lower: rational(1, 2),
            lower_open: true,
            upper: Some(rational(2, 1)),
            upper_open: false
        }
    );
    assert_eq!(
        interval("eventually [0, 2] holds(h.v < 19)"),
        HybridInterval {
            lower: rational(0, 1),
            lower_open: false,
            upper: Some(rational(2, 1)),
            upper_open: false
        }
    );
    let unbounded = claim_of(&timed_claim(
        "always [rational(1, 4) s, *] holds(h.v <= 20)",
    ));
    let HybridFormula::Always(Some(open), _) = &unbounded.formula else {
        panic!("always: {:?}", unbounded.formula);
    };
    assert_eq!(open.lower, rational(1, 4));
    assert_eq!(open.upper, None);
    assert_eq!(unbounded.horizon, None);

    let infinite = |body: &str| slow_enough(body);
    for body in [
        "eventually [0 s, 1500 ms] holds(h.v < 19)",
        "always (0.5 s, 2 s] holds(h.v <= 20)",
    ] {
        assert_eq!(
            claim_refusal(&infinite(body)).1,
            "expression-form",
            "{body}"
        );
    }
    let whole = claim_of(&infinite("eventually [0 s, 2 s] holds(h.v < 19)"));
    assert_eq!(whole.horizon, Some(HybridHorizon::Seconds(rational(2, 1))));
    assert_eq!(
        claim_of(&infinite("always [2, *] holds(h.v < 19)")).horizon,
        None
    );
    assert_eq!(
        claim_refusal(&timed_claim("eventually [2 kg, 3 kg] holds(h.v < 19)")),
        (Code::IllTyped, "type-mismatch", "kg".to_owned())
    );
    assert_eq!(
        claim_refusal(&timed_claim("eventually (1 s, 1 s] holds(h.v < 19)")),
        (
            Code::InvalidTimedModel,
            "invalid-interval",
            "(1 s, 1 s]".to_owned()
        )
    );
    let steps =
        slow_enough("eventually [0 s, 2 s] holds(h.v < 19)").replace("model-time", "model-steps");
    assert_eq!(
        claim_refusal(&steps),
        (Code::IllTyped, "type-mismatch", "[0 s, 2 s]".to_owned())
    );
}

/// FR-447 "Intervals": an operator with no interval is admitted only where
/// the profile admits an unbounded operator.
#[test]
fn a_missing_interval_is_refused_under_a_bounded_profile() {
    let bounded = slow_enough("always holds(h.v < 30)").replace("using p", "using e");
    assert_eq!(
        claim_refusal(&bounded),
        (
            Code::UnsupportedConstruct,
            "expression-form",
            "always".to_owned()
        )
    );
    let timed = claim_of(&timed_claim("always holds(h.v < 30)"));
    assert_eq!(timed.horizon, None);
}

/// FR-447-AC-7: a call of a pure function lowers to `Call`, a real argument
/// to an `Integer` parameter refuses at the call, and a body outside the
/// hybrid expression forms refuses at the call.
#[trace("TC-1220", "FR-447-AC-7")]
#[test]
fn calls_of_pure_functions_lower_and_foreign_bodies_refuse() {
    let drag = "function drag using v(s: Rational[-100, 100; 1, 1]): \
                Rational[-100, 100; 1, 1] pure { -s }\n";
    let with_flow = |flow: &str| {
        format!(
            "{drag}{}",
            braking(INVARIANT, flow, TRANSITION, STOPPED_FLOW)
        )
    };
    let model = model(&with_flow("v' = drag(v); a' = 0; t' = 1;"));
    assert_eq!(
        model.modes[0].rates[0],
        HybridTerm::Call(CheckedFunctionRef::new(0), vec![variable(0)])
    );
    let integer = "function integral using v(n: Integer): Integer pure { n }\n";
    let flow = braking(
        INVARIANT,
        "v' = integral(v); a' = 0; t' = 1;",
        TRANSITION,
        STOPPED_FLOW,
    );
    assert_eq!(
        sole_refusal(&format!("{integer}{flow}")),
        (Code::IllTyped, "type-mismatch", "integral(v)".to_owned())
    );
    let foreign = "function bad using v(s: Rational[0, 10; 1, 1]): Rational[0, 10; 1, 1] \
                   pure { if \"x\" = \"x\" then s else s }\n";
    let flow = braking(
        INVARIANT,
        "v' = bad(v); a' = 0; t' = 1;",
        TRANSITION,
        STOPPED_FLOW,
    );
    assert_eq!(
        sole_refusal(&format!("{foreign}{flow}")),
        (Code::IllTyped, "type-mismatch", "bad(v)".to_owned())
    );
}

/// FR-250-AC-1, FR-447 "The provider request": each checked hybrid claim
/// has one `temporal-satisfaction` requirement record carrying the
/// hybrid-dynamics requirement S3's variable classification gives it,
/// `bounded` for a claim with a horizon and unbounded for one without, and
/// a temporal claim over no hybrid model has none.
#[trace("TC-705", "FR-250-AC-1")]
#[test]
fn each_hybrid_claim_has_a_hybrid_dynamics_requirement_record() {
    use crate::check::Capability;
    use crate::family::ClaimExtent;

    let records = |declarations: &str| {
        let graph = check(declarations).unwrap_or_else(|refusals| panic!("{refusals:?}"));
        graph
            .requirements()
            .values()
            .map(|record| record.requirements().clone())
            .collect::<Vec<_>>()
    };

    let bounded = records(&slow_enough("always [0, 10] holds(h.v < 30)"));
    let [bounded] = bounded.as_slice() else {
        panic!("one record: {bounded:?}");
    };
    assert_eq!(bounded.kind(), Capability::TemporalSatisfaction);
    assert!(bounded.hybrid_dynamics());
    assert!(matches!(bounded.extent(), ClaimExtent::Bounded));

    let unbounded = records(&slow_enough("always holds(h.v < 30)"));
    let [unbounded] = unbounded.as_slice() else {
        panic!("one record: {unbounded:?}");
    };
    assert_eq!(unbounded.kind(), Capability::TemporalSatisfaction);
    assert!(unbounded.hybrid_dynamics());
    assert!(matches!(unbounded.extent(), ClaimExtent::Unbounded(_)));

    // The model's classification: `v` and `a` hybrid, `t` a clock.
    let classes: Vec<_> = model(&use_case())
        .variables
        .iter()
        .map(|variable| variable.class)
        .collect();
    assert_eq!(
        classes,
        [
            VariableClass::Hybrid,
            VariableClass::Hybrid,
            VariableClass::Clock
        ]
    );

    assert!(records(&use_case()).is_empty());
}
