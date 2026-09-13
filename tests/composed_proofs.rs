// SPDX-License-Identifier: AGPL-3.0-only
//! TC-119: actual IR definedness under fixture-authored source and clause owners.
//! Discharged value obligations grant neither family admission nor execution.

#[path = "support/composed_types/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::checking::composed::proofs::work::Dimension;
use quire_spec_language::checking::composed::proofs::{
    self, CauseKind, CorrespondenceError, DeclarationProof, ProofDisposition, ProofLimits,
    ProofReport,
};
use quire_spec_language::checking::composed::{
    self, ObservationOrigin, TypeDisposition, TypeLimits, TypeReport,
};
use quire_spec_language::checking::{CheckBindings, ClauseBinding};
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::linking::composed::binding_work::Limits as BindingLimits;
use quire_spec_language::linking::composed::scopes::Anchor;
use quire_spec_language::linking::composed::DeclarationId;
use quire_spec_language::model_source::{self, ModelSourceLimits};
use quire_spec_language::native_model::{ModelLimits, NativeModel};
use quire_spec_language::{
    link_native, parse, Code, Limits, LinkLimits, Source, SourceIdentity, Span,
};

fn owner() -> ir::RequirementRef {
    ir::RequirementRef::new(
        ir::PackageId::new("test/authored-proofs").unwrap(),
        ir::RequirementId::new("ComposedProofs").unwrap(),
        ir::RequirementRevision::new(7).unwrap(),
    )
}

fn handler() -> ir::ExecutionPoint {
    ir::ExecutionPoint::Handler {
        name: ir::AnchorName::new("validate").unwrap(),
    }
}

fn mapping(source: &FormalSource, names: &[&str], point: ir::ExecutionPoint) -> CheckBindings {
    CheckBindings {
        source: source.clone(),
        clauses: names
            .iter()
            .map(|name| ClauseBinding {
                name: (*name).into(),
                requirement: owner(),
                clause: ir::ClauseId::new(name.to_lowercase()).unwrap(),
                execution_point: point.clone(),
            })
            .collect(),
    }
}

fn with_types(
    model: &NativeModel,
    sources: &[Source],
    test: impl FnOnce(&TypeReport<'_, '_>, &[FormalSource]),
) {
    let formal = setup::formal_sources(sources);
    setup::with_binding(sources, &[model], BindingLimits::default(), |binding| {
        assert!(binding.complete(), "{:?}", binding.exhaustion());
        let types = composed::admit_types(binding, &formal, TypeLimits::default());
        assert!(types.exhaustion().is_none(), "{:?}", types.exhaustion());
        test(&types, &formal);
    });
}

fn inspect(body: &str, names: &[&str], test: impl FnOnce(&TypeReport<'_, '_>, &[CheckBindings])) {
    let model = setup::model("ProofInputs");
    let sources = [setup::source("proofs", &model, body)];
    with_types(&model, &sources, |types, formal| {
        test(types, &[mapping(&formal[0], names, handler())]);
    });
}

fn id(types: &TypeReport<'_, '_>, name: &str) -> DeclarationId {
    let [id] = types.binding().namespace().lookup(name) else {
        panic!("one original declaration named {name}")
    };
    *id
}

fn discharged<'p>(report: &'p ProofReport<'_, '_, '_>, name: &str) -> &'p DeclarationProof {
    let declaration = id(report.types(), name);
    assert_eq!(
        report.types().disposition(declaration),
        Some(TypeDisposition::Typed)
    );
    let proof = report.declaration(declaration).unwrap();
    assert_eq!(
        report.disposition(declaration),
        Some(ProofDisposition::Discharged),
        "{name}: {:?}; exhaustion {:?}",
        proof.causes(),
        report.exhaustion()
    );
    assert!(proof.complete());
    assert!(proof.causes().is_empty());
    proof
}

fn unproved(
    report: &ProofReport<'_, '_, '_>,
    name: &str,
    expected: ir::DefinednessObligationKind,
    original: &str,
) {
    let declaration = id(report.types(), name);
    assert_eq!(
        report.types().disposition(declaration),
        Some(TypeDisposition::Typed)
    );
    assert_eq!(
        report.disposition(declaration),
        Some(ProofDisposition::Refused)
    );
    let proof = report.declaration(declaration).unwrap();
    assert!(
        !proof.complete(),
        "a refused declaration cannot be complete"
    );
    let cause = proof
        .causes()
        .iter()
        .find(|cause| match &cause.kind {
            CauseKind::Unproved { diagnostics } => diagnostics
                .iter()
                .any(|diagnostic| diagnostic.obligation_kind == Some(expected)),
            _ => false,
        })
        .unwrap_or_else(|| {
            panic!(
                "{name}: expected actual {expected:?} refusal, got {:?}",
                proof.causes()
            )
        });
    let unit = report
        .types()
        .binding()
        .namespace()
        .unit(proof.unit())
        .unwrap();
    let declaration_span = report
        .types()
        .binding()
        .namespace()
        .syntax(declaration)
        .unwrap()
        .span;
    let start = declaration_span.start
        + unit
            .source()
            .slice(declaration_span)
            .unwrap()
            .find(original)
            .unwrap();
    assert_eq!(
        cause.site.span,
        Span {
            start,
            end: start + original.len()
        }
    );
    assert_eq!(cause.site.declaration, declaration);
    assert_eq!(cause.site.unit, proof.unit());
    assert_eq!(
        unit.expression(cause.site.expression.unwrap())
            .unwrap()
            .span,
        cause.site.span
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-4", "FR-040-AC-8")]
fn integer_discharge_keeps_actual_ir_goals_and_authored_source_owners() {
    inspect("predicate Bounded using S (amount: M::Signed): Boolean { amount < 10 implies amount + 1 <= 10 }",
        &["Bounded"], |types, bindings| {
            let report = proofs::discharge(types, bindings, ProofLimits::default());
            let proof = discharged(&report, "Bounded");
            let declaration = id(types, "Bounded");
            assert!(std::ptr::eq(report.types(), types));
            assert_eq!(report.clause_binding(declaration), Some(&bindings[0].clauses[0]));
            assert_eq!(proof.environment().unwrap().owner(), &owner());
            assert_eq!(proof.authored_binding().unwrap().source, 0);
            assert_eq!(proof.authored_binding().unwrap().clause, 0);
            assert!(!proof.goals().is_empty());
            assert!(proof.goals().iter().any(|goal| goal.checked().obligations().iter()
                .any(|obligation| obligation.kind() == ir::DefinednessObligationKind::CheckedRange)));
            let unit = types.binding().namespace().unit(proof.unit()).unwrap();
            assert_eq!(bindings[0].source.source().digest(), unit.source().digest());
            assert_eq!(bindings[0].source.source().identity(), unit.source().identity());
            for goal in proof.goals() {
                let original = unit.expression(goal.native_expression()).unwrap();
                assert_eq!(*goal.source(), bindings[0].source.to_ir(unit.source(), original.span).unwrap());
                assert!(!goal.checked().nodes().is_empty());
                for node in goal.checked().nodes() {
                    assert_eq!(node.source().source(), bindings[0].source.identity());
                    assert!(bindings[0].source.to_native(node.source()).is_ok());
                }
            }
            assert!(!proof.values().is_empty());
            for value in proof.values() {
                let original = unit.expression(value.expression).unwrap();
                assert_eq!(bindings[0].source.to_native(&value.source).unwrap(), original.span);
                assert!(proof.environment().unwrap().values().iter().any(|input| input.name() == &value.symbol));
            }
        });
}

#[test]
#[trace("TC-119", "FR-040-AC-4", "FR-040-AC-6")]
fn preceding_integer_guards_work_through_conditionals_and_immutable_aliases() {
    for expression in [
        "amount < 10 and amount + 1 <= 10",
        "amount >= 10 or amount + 1 <= 10",
        "if amount < 10 then amount + 1 <= 10 else true",
        "let saved = amount in saved < 10 implies saved + 1 <= 10",
        "let allowed = amount < 10 in allowed implies amount + 1 <= 10",
        "false implies amount + 1 <= 10",
    ] {
        let body =
            format!("predicate Guarded using S (amount: M::Signed): Boolean {{ {expression} }}");
        inspect(&body, &["Guarded"], |types, bindings| {
            let report = proofs::discharge(types, bindings, ProofLimits::default());
            discharged(&report, "Guarded");
        });
    }
    for expression in [
        "amount + 1 <= 10",
        "amount <= 10 implies amount + 1 <= 10",
        "amount + 1 <= 10 and amount < 10",
        "(amount < 10 or alternate) implies amount + 1 <= 10",
    ] {
        let body = format!("predicate Unguarded using S (amount: M::Signed, alternate: Boolean): Boolean {{ {expression} }}");
        inspect(&body, &["Unguarded"], |types, bindings| {
            let report = proofs::discharge(types, bindings, ProofLimits::default());
            unproved(
                &report,
                "Unguarded",
                ir::DefinednessObligationKind::CheckedRange,
                "amount + 1",
            );
        });
    }
}

fn denominator_one_model() -> NativeModel {
    let seed = setup::model("WholeRational");
    let mut document: serde_json::Value =
        serde_json::from_str(seed.source().source().text()).unwrap();
    let ratio = document["scalars"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|scalar| scalar["name"] == "Q")
        .unwrap();
    ratio["maximum_denominator"] = serde_json::json!(1);
    let text = serde_json::to_string_pretty(&document).unwrap();
    let source = Source::read(
        seed.source().source().identity().clone(),
        seed.source().source().path(),
        text.as_bytes(),
        1_048_576,
    )
    .unwrap();
    let formal = FormalSource::new(source, seed.source().identity().clone());
    model_source::read(
        formal,
        model_source::FORMAT_V2,
        ModelSourceLimits::default(),
    )
    .unwrap()
    .admit(ModelLimits::default())
    .unwrap()
}

#[test]
#[trace("TC-119", "FR-040-AC-4", "FR-040-AC-8")]
fn exact_rational_literals_discharge_after_normalization_before_domain_checks() {
    for expression in [
        "rational(2,4) + rational(1,2) = expected",
        "rational(1,2) / rational(1,2) = expected",
    ] {
        let body = format!("predicate Exact using S (expected: M::Q): Boolean {{ {expression} }}");
        inspect(&body, &["Exact"], |types, bindings| {
            let report = proofs::discharge(types, bindings, ProofLimits::default());
            let proof = discharged(&report, "Exact");
            assert!(proof.goals().iter().any(|goal| goal
                .checked()
                .nodes()
                .iter()
                .any(|node| matches!(node.value_type(), ir::ValueType::Rational { .. }))));
        });
    }
    inspect("predicate Overflow using S (expected: M::Q): Boolean { rational(1,1) + rational(1,1) = expected }",
        &["Overflow"], |types, bindings| {
            let report = proofs::discharge(types, bindings, ProofLimits::default());
            unproved(&report, "Overflow", ir::DefinednessObligationKind::CheckedRange,
                "rational(1,1) + rational(1,1)");
        });
}

#[test]
#[trace("TC-119", "FR-040-AC-4", "FR-040-AC-6")]
fn rational_guarded_divisors_and_result_ranges_use_actual_declared_domains() {
    let model = denominator_one_model();
    let sources = [setup::source("rational", &model,
        "predicate Nonzero using S (divisor: M::Q): Boolean { divisor != rational(0,1) implies rational(0,1) / divisor = rational(0,1) }\n\
         predicate ZeroPossible using S (divisor: M::Q): Boolean { rational(0,1) / divisor = rational(0,1) }\n\
         predicate Bounded using S (amount: M::Q): Boolean { amount <= rational(0,1) implies amount + rational(1,1) <= rational(1,1) }\n\
         predicate Outside using S (amount: M::Q): Boolean { amount + rational(1,1) <= rational(1,1) }")];
    with_types(&model, &sources, |types, formal| {
        let bindings = [mapping(
            &formal[0],
            &["Nonzero", "ZeroPossible", "Bounded", "Outside"],
            handler(),
        )];
        let report = proofs::discharge(types, &bindings, ProofLimits::default());
        let nonzero = discharged(&report, "Nonzero");
        assert!(nonzero.goals().iter().any(|goal| goal
            .checked()
            .obligations()
            .iter()
            .any(|obligation| obligation.kind() == ir::DefinednessObligationKind::NonZeroDivisor)));
        discharged(&report, "Bounded");
        unproved(
            &report,
            "ZeroPossible",
            ir::DefinednessObligationKind::NonZeroDivisor,
            "rational(0,1) / divisor",
        );
        unproved(
            &report,
            "Outside",
            ir::DefinednessObligationKind::CheckedRange,
            "amount + rational(1,1)",
        );
    });
}

#[test]
#[trace("TC-119", "FR-040-AC-6")]
fn optional_guards_follow_the_same_receiver_and_selected_let_value() {
    for expression in [
        "present(input.optionalQ) implies value(input.optionalQ) >= rational(-1,1)",
        "if present(input.optionalQ) then value(input.optionalQ) >= rational(-1,1) else true",
        "let saved = input.optionalQ in present(saved) implies value(saved) >= rational(-1,1)",
        "let selected = if flag then input.optionalQ else other.optionalQ in present(selected) implies value(selected) >= rational(-1,1)",
    ] {
        let body = format!("predicate Present using S (input: M::Node, other: M::Node, flag: Boolean): Boolean {{ {expression} }}");
        inspect(&body, &["Present"], |types, bindings| {
            let report = proofs::discharge(types, bindings, ProofLimits::default());
            discharged(&report, "Present");
        });
    }
    for (expression, locus) in [
        ("value(input.optionalQ) >= rational(-1,1) and present(input.optionalQ)", "value(input.optionalQ)"),
        ("present(other.optionalQ) implies value(input.optionalQ) >= rational(-1,1)", "value(input.optionalQ)"),
        ("let selected = if flag then input.optionalQ else other.optionalQ in present(input.optionalQ) implies value(selected) >= rational(-1,1)", "value(selected)"),
    ] {
        let body = format!("predicate Missing using S (input: M::Node, other: M::Node, flag: Boolean): Boolean {{ {expression} }}");
        inspect(&body, &["Missing"], |types, bindings| {
            let report = proofs::discharge(types, bindings, ProofLimits::default());
            unproved(&report, "Missing", ir::DefinednessObligationKind::OptionPresence, locus);
        });
    }
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-6", "FR-040-AC-8")]
fn invocation_pre_captures_keep_their_read_origin_and_cannot_guard_post_reads() {
    let model = setup::model("Invocation");
    let sources = [setup::source("invocation", &model,
        "post Captured using S on M::Node::step { let prior = pre(self.signed) in prior < 10 implies prior + 1 <= 10 }\n\
         post WrongObservation using S on M::Node::step { pre(self.signed) < 10 implies self.signed + 1 <= 10 }\n\
         post Retagged using S on M::Node::step { let prior = self.signed in pre(prior) = self.signed }\n\
         post Composite using S on M::Node::step { pre(let prior = self.signed in prior < 10 implies prior + 1 <= 10) and delta = delta }")];
    with_types(&model, &sources, |types, formal| {
        let bindings = [mapping(
            &formal[0],
            &["Captured", "WrongObservation", "Retagged", "Composite"],
            ir::ExecutionPoint::Post {
                operation: ir::AnchorName::new("step").unwrap(),
            },
        )];
        let report = proofs::discharge(types, &bindings, ProofLimits::default());
        discharged(&report, "Captured");
        discharged(&report, "Composite");
        let typed = types.declaration(id(types, "Captured")).unwrap();
        let scope = types
            .binding()
            .scopes()
            .unwrap()
            .declaration(typed.declaration())
            .unwrap();
        let prior = scope
            .binders
            .iter()
            .find(|binder| binder.name.as_deref() == Some("prior"))
            .unwrap();
        assert_eq!(prior.anchor, Anchor::InvocationPost);
        let unit = types.binding().namespace().unit(typed.unit()).unwrap();
        let reads: Vec<_> = typed
            .nodes()
            .iter()
            .filter(|node| unit.source().slice(node.span) == Some("prior"))
            .collect();
        assert!(!reads.is_empty());
        assert!(reads
            .iter()
            .all(|node| node.origin == Some(ObservationOrigin::Anchored(Anchor::InvocationPre))));
        unproved(
            &report,
            "WrongObservation",
            ir::DefinednessObligationKind::CheckedRange,
            "self.signed + 1",
        );
        let retagged = id(types, "Retagged");
        assert_eq!(types.disposition(retagged), Some(TypeDisposition::Refused));
        assert_eq!(
            report.disposition(retagged),
            Some(ProofDisposition::Refused)
        );
        assert!(report
            .declaration(retagged)
            .unwrap()
            .causes()
            .iter()
            .any(|cause| matches!(cause.kind, CauseKind::UpstreamType)));
        for wrong in [
            ir::ExecutionPoint::Pre {
                operation: ir::AnchorName::new("step").unwrap(),
            },
            ir::ExecutionPoint::Post {
                operation: ir::AnchorName::new("other_operation").unwrap(),
            },
        ] {
            let mut offered = bindings.clone();
            offered[0].clauses[0].execution_point = wrong;
            let failed = proofs::discharge(types, &offered, ProofLimits::default());
            let captured = id(types, "Captured");
            assert_eq!(
                failed.disposition(captured),
                Some(ProofDisposition::Refused)
            );
            assert!(failed
                .declaration(captured)
                .unwrap()
                .causes()
                .iter()
                .any(|cause| matches!(
                    cause.kind,
                    CauseKind::Correspondence(CorrespondenceError::ExecutionPoint)
                )));
            discharged(&failed, "Composite");
        }
    });
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-8")]
fn correspondence_refusals_preserve_types_and_an_independent_source() {
    let model = setup::model("Correspondence");
    let sources = [
        setup::source("target", &model, "predicate Target using S (amount: M::Signed): Boolean { amount < 10 implies amount + 1 <= 10 }"),
        setup::source("independent", &model, "predicate Independent using S (): Boolean { true }"),
    ];
    with_types(&model, &sources, |types, formal| {
        let original = [
            mapping(&formal[0], &["Target"], handler()),
            mapping(&formal[1], &["Independent"], handler()),
        ];
        for (mutation, expected) in [
            (0, CorrespondenceError::MissingSource),
            (1, CorrespondenceError::DuplicateSource),
            (2, CorrespondenceError::MissingDeclaration),
            (3, CorrespondenceError::ForeignDeclaration),
            (4, CorrespondenceError::DuplicateDeclaration),
            (5, CorrespondenceError::ExecutionPoint),
            (6, CorrespondenceError::ForeignSource),
        ] {
            let mut offered = original.to_vec();
            match mutation {
                0 => {
                    offered.remove(0);
                }
                1 => offered.push(original[0].clone()),
                2 => offered[0].clauses.clear(),
                3 => offered[0].clauses[0].name = "NotAuthoredHere".into(),
                4 => offered[0].clauses.push(original[0].clauses[0].clone()),
                5 => {
                    offered[0].clauses[0].execution_point = ir::ExecutionPoint::Pre {
                        operation: ir::AnchorName::new("step").unwrap(),
                    }
                }
                6 => {
                    offered[0].source =
                        FormalSource::new(sources[0].clone(), model.source().identity().clone())
                }
                _ => unreachable!(),
            }
            let report = proofs::discharge(types, &offered, ProofLimits::default());
            let target = id(types, "Target");
            assert_eq!(types.disposition(target), Some(TypeDisposition::Typed));
            assert_eq!(
                report.disposition(target),
                Some(ProofDisposition::Refused),
                "{expected:?}"
            );
            let refused = report.declaration(target).unwrap();
            assert!(refused.causes().iter().any(|cause|
                matches!(cause.kind, CauseKind::Correspondence(actual) if actual == expected)),
                "{expected:?}: {:?}", refused.causes());
            assert!(refused.goals().is_empty());
            for cause in refused.causes() {
                assert_eq!(cause.site.declaration, target);
                assert_eq!(cause.site.unit, refused.unit());
                assert!(sources[0].slice(cause.site.span).is_some());
            }
            discharged(&report, "Independent");
        }
        let retry = proofs::discharge(types, &original, ProofLimits::default());
        discharged(&retry, "Target");
        discharged(&retry, "Independent");
    });
}

#[test]
#[trace("TC-119", "FR-040-AC-8")]
fn source_bytes_labels_revision_and_path_are_all_part_of_exact_correspondence() {
    inspect(
        "predicate Original using S (): Boolean { true }",
        &["Original"],
        |types, bindings| {
            for (changed, expected) in [
                ("bytes", CorrespondenceError::ForeignSource),
                ("identity", CorrespondenceError::MissingSource),
                ("revision", CorrespondenceError::MissingSource),
                ("path", CorrespondenceError::ForeignSource),
            ] {
                let mut offered = bindings.to_vec();
                let original = bindings[0].source.source();
                let mut identity = original.identity().clone();
                let mut text = original.text().to_owned();
                let mut path = original.path().to_owned();
                match changed {
                    "bytes" => text.push('\n'),
                    "identity" => identity.identity.push_str(":different"),
                    "revision" => identity.revision.push_str(":different"),
                    "path" => path.push_str(".different"),
                    _ => unreachable!(),
                }
                let replacement = Source::read(identity, path, text.as_bytes(), 1_048_576).unwrap();
                offered[0].source =
                    FormalSource::new(replacement, bindings[0].source.identity().clone());
                let report = proofs::discharge(types, &offered, ProofLimits::default());
                let declaration = id(types, "Original");
                assert_eq!(
                    report.disposition(declaration),
                    Some(ProofDisposition::Refused),
                    "{changed}"
                );
                assert!(
                    report
                        .declaration(declaration)
                        .unwrap()
                        .causes()
                        .iter()
                        .any(|cause| matches!(
                            cause.kind,
                            CauseKind::Correspondence(actual) if actual == expected
                        )),
                    "{changed}"
                );
                assert_eq!(
                    report.types().disposition(declaration),
                    Some(TypeDisposition::Typed)
                );
            }
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-8")]
fn authored_clause_identities_cannot_be_duplicated_across_units() {
    let model = setup::model("Owners");
    let sources = [
        setup::source(
            "left",
            &model,
            "predicate Left using S (): Boolean { true }",
        ),
        setup::source(
            "right",
            &model,
            "predicate Right using S (): Boolean { true }",
        ),
    ];
    with_types(&model, &sources, |types, formal| {
        let mut bindings = [
            mapping(&formal[0], &["Left"], handler()),
            mapping(&formal[1], &["Right"], handler()),
        ];
        bindings[1].clauses[0].clause = bindings[0].clauses[0].clause.clone();
        let report = proofs::discharge(types, &bindings, ProofLimits::default());
        for name in ["Left", "Right"] {
            let declaration = id(types, name);
            assert_eq!(
                report.disposition(declaration),
                Some(ProofDisposition::Refused)
            );
            assert!(report
                .declaration(declaration)
                .unwrap()
                .causes()
                .iter()
                .any(|cause| matches!(
                    cause.kind,
                    CauseKind::Correspondence(CorrespondenceError::DuplicateDeclaration)
                )));
        }
    });
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-3", "FR-040-AC-4", "FR-040-AC-8")]
fn callee_totality_and_call_argument_definedness_are_independent_obligations() {
    let model = setup::model("Calls");
    let sources = [setup::source("callers", &model,
        "predicate SafeCall using S (amount: M::Signed): Boolean { amount < 10 implies Total(amount + 1) }\n\
         predicate BadArgument using S (amount: M::Signed): Boolean { Total(amount + 1) }\n\
         predicate GuardedBadCallee using S (amount: M::Signed): Boolean { amount < 10 implies Partial(amount) }\n\
         predicate Independent using S (): Boolean { true }"),
        setup::source("callees", &model,
        "predicate Total using S (input: M::Signed): Boolean { input < 10 implies input + 1 <= 10 }\n\
         predicate Partial using S (input: M::Signed): Boolean { input + 1 <= 10 }")];
    with_types(&model, &sources, |types, formal| {
        let bindings = [
            mapping(
                &formal[0],
                &["SafeCall", "BadArgument", "GuardedBadCallee", "Independent"],
                handler(),
            ),
            mapping(&formal[1], &["Total", "Partial"], handler()),
        ];
        let report = proofs::discharge(types, &bindings, ProofLimits::default());
        discharged(&report, "Total");
        discharged(&report, "SafeCall");
        discharged(&report, "Independent");
        unproved(
            &report,
            "BadArgument",
            ir::DefinednessObligationKind::CheckedRange,
            "amount + 1",
        );
        unproved(
            &report,
            "Partial",
            ir::DefinednessObligationKind::CheckedRange,
            "input + 1",
        );
        let dependent = id(types, "GuardedBadCallee");
        assert_eq!(
            report.disposition(dependent),
            Some(ProofDisposition::Refused)
        );
        assert!(!report.declaration(dependent).unwrap().complete());
        assert!(report.declaration(dependent).unwrap().causes().iter().any(|cause|
            matches!(cause.kind, CauseKind::Dependency { target } if target == id(types, "Partial"))));
        for (name, unit_index) in [("SafeCall", 0), ("Total", 1)] {
            let proof = report.declaration(id(types, name)).unwrap();
            assert_eq!(proof.unit().index(), unit_index);
            for goal in proof.goals() {
                assert_eq!(goal.source().source(), formal[unit_index].identity());
                assert!(formal[unit_index].to_native(goal.source()).is_ok());
            }
        }
    });
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-5", "FR-040-AC-8")]
fn ordered_query_definedness_keeps_original_types_and_unblocks_consumers() {
    inspect("predicate Ordered using S (input: M::Node): Boolean { forall(item in input.amounts: item >= 1) }\n\
        predicate Consumer using S (input: M::Node): Boolean { Ordered(input) }\n\
        predicate Independent using S (): Boolean { true }", &["Ordered", "Consumer", "Independent"], |types, bindings| {
            let report = proofs::discharge(types, bindings, ProofLimits::default());
            let ordered = id(types, "Ordered");
            assert_eq!(types.disposition(ordered), Some(TypeDisposition::Typed));
            let proof = discharged(&report, "Ordered");
            assert_eq!(proof.declaration(), ordered);
            let typed = types.declaration(ordered).unwrap();
            assert_eq!(proof.unit(), typed.unit());
            let unit = types.binding().namespace().unit(proof.unit()).unwrap();
            let query = typed.nodes().iter().find(|node| unit.source().slice(node.span)
                == Some("forall(item in input.amounts: item >= 1)")).unwrap();
            assert!(matches!(query.ty, Some(quire_spec_language::checking::NativeType::Boolean)));
            assert_eq!(unit.expression(query.expression).unwrap().span, query.span);
            assert_eq!(report.clause_binding(ordered), Some(&bindings[0].clauses[0]));
            discharged(&report, "Consumer");
            discharged(&report, "Independent");
        });
}

#[test]
#[trace("TC-119", "FR-040-AC-3", "FR-040-AC-4", "FR-040-AC-8")]
fn excluded_numeric_meanings_keep_the_actual_type_refusal_and_original_locus() {
    // These source meanings cannot reach the proof adapter's defensive
    // ValueRepresentation paths through a constructor-admitted TypeReport.
    for (expression, original, expected) in [
        (
            "lhs_arg / rhs_arg = lhs_arg",
            "lhs_arg / rhs_arg",
            composed::CauseKind::ForbiddenOperator,
        ),
        (
            "lhs_arg div rhs_arg = lhs_arg",
            "lhs_arg div rhs_arg",
            composed::CauseKind::ForbiddenOperator,
        ),
        (
            "lhs_arg rem rhs_arg = lhs_arg",
            "lhs_arg rem rhs_arg",
            composed::CauseKind::ForbiddenOperator,
        ),
        (
            "lhs_arg mod rhs_arg = lhs_arg",
            "lhs_arg mod rhs_arg",
            composed::CauseKind::ForbiddenOperator,
        ),
        (
            "lhs_arg = 9223372036854775808",
            "9223372036854775808",
            composed::CauseKind::LiteralDomain,
        ),
        (
            "fraction = rational(9223372036854775808,9223372036854775808)",
            "rational(9223372036854775808,9223372036854775808)",
            composed::CauseKind::UnsupportedPrerequisite(
                composed::Prerequisite::RationalNormalization,
            ),
        ),
    ] {
        let body = format!(
            "predicate Excluded using S (lhs_arg: M::Signed, rhs_arg: M::Signed, fraction: M::Q): Boolean {{ {expression} }}\n\
             predicate Independent using S (): Boolean {{ true }}"
        );
        inspect(&body, &["Excluded", "Independent"], |types, bindings| {
            let declaration = id(types, "Excluded");
            assert_eq!(
                types.disposition(declaration),
                Some(TypeDisposition::Refused)
            );
            let typed = types.declaration(declaration).unwrap();
            let cause = typed
                .causes()
                .iter()
                .find(|cause| cause.kind == expected)
                .unwrap_or_else(|| panic!("{expression}: {:?}", typed.causes()));
            let unit = types.binding().namespace().unit(typed.unit()).unwrap();
            assert_eq!(unit.source().slice(cause.site.span), Some(original));
            assert_eq!(cause.site.declaration, declaration);
            assert_eq!(cause.site.unit, typed.unit());
            assert_eq!(
                unit.expression(cause.site.expression.unwrap())
                    .unwrap()
                    .span,
                cause.site.span
            );
            let report = proofs::discharge(types, bindings, ProofLimits::default());
            let proof = report.declaration(declaration).unwrap();
            assert_eq!(proof.disposition(), ProofDisposition::Refused);
            assert!(!proof.complete());
            assert!(proof.environment().is_none());
            assert!(proof.goals().is_empty());
            assert_eq!(proof.causes().len(), 1);
            assert!(matches!(proof.causes()[0].kind, CauseKind::UpstreamType));
            assert_eq!(proof.causes()[0].site.declaration, declaration);
            assert_eq!(proof.causes()[0].site.unit, typed.unit());
            assert_eq!(
                proof.causes()[0].site.span,
                types
                    .binding()
                    .namespace()
                    .syntax(declaration)
                    .unwrap()
                    .span
            );
            discharged(&report, "Independent");
        });
    }
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-4", "FR-040-AC-8")]
fn opaque_proof_witnesses_preserve_native_types_without_proving_numeric_guards() {
    inspect("predicate Opaque using G (input: M::Node, other: M::Node, candidate: M::Plain, lhs_ref: M::NodeRef, rhs_ref: M::NodeRef): Boolean {\n\
        input.label = \"ok\" and input.mode = M::Mode::Ready and input.plain.ready and candidate.ready and input = other and lhs_ref = rhs_ref\n\
        }\n\
        predicate OpaqueGuard using S (label: M::Label, amount: M::Signed): Boolean { label = \"ok\" implies amount + 1 <= 10 }",
        &["Opaque", "OpaqueGuard"], |types, bindings| {
            let report = proofs::discharge(types, bindings, ProofLimits::default());
            let proof = discharged(&report, "Opaque");
            let typed = types.declaration(id(types, "Opaque")).unwrap();
            let mut kinds = std::collections::BTreeSet::new();
            for value in proof.values() {
                let native = typed.node(value.expression).unwrap().ty.as_ref().unwrap();
                let kind = match native {
                    quire_spec_language::checking::NativeType::Scalar { role, .. }
                        if matches!(role.kind, quire_spec_language::native_model::ScalarKind::Text { .. }) => "text",
                    quire_spec_language::checking::NativeType::Enumeration { .. } => "enum",
                    quire_spec_language::checking::NativeType::Record { .. } => "record",
                    quire_spec_language::checking::NativeType::Object { .. } => "object",
                    quire_spec_language::checking::NativeType::Reference { .. } => "reference",
                    _ => continue,
                };
                kinds.insert(kind);
                let symbol = proof.environment().unwrap().values().iter()
                    .find(|input| input.name() == &value.symbol).unwrap();
                assert_eq!(symbol.value_type(), &ir::ValueType::Boolean);
                assert_eq!(bindings[0].source.to_native(&value.source).unwrap(), typed.node(value.expression).unwrap().span);
            }
            assert_eq!(kinds, ["text", "enum", "record", "object", "reference"].into_iter().collect());
            unproved(&report, "OpaqueGuard", ir::DefinednessObligationKind::CheckedRange, "amount + 1");
        });
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-8", "FR-040-AC-9")]
fn every_zero_proof_budget_refuses_before_work_and_fresh_retries_preserve_inputs() {
    let model = denominator_one_model();
    let sources = [setup::source("budgets", &model,
        "predicate Leaf using S (amount: M::Q): Boolean { amount >= rational(0,1) }\n\
         predicate Budget using S (input: M::Node): Boolean { present(input.optionalQ) implies Leaf(value(input.optionalQ) + rational(0,1)) }")];
    with_types(&model, &sources, |types, formal| {
        let bindings = [mapping(&formal[0], &["Leaf", "Budget"], handler())];
        let expected_digest = sources[0].digest();
        let declaration = id(types, "Budget");
        let original_nodes = types.declaration(declaration).unwrap().nodes().len();
        let baseline = proofs::discharge(types, &bindings, ProofLimits::default());
        discharged(&baseline, "Leaf");
        discharged(&baseline, "Budget");
        for dimension in [
            Dimension::Bytes,
            Dimension::Declarations,
            Dimension::Expressions,
            Dimension::Types,
            Dimension::Edges,
            Dimension::Values,
            Dimension::GraphNodes,
            Dimension::Facts,
            Dimension::Goals,
            Dimension::Materialized,
            Dimension::GoalNodes,
            Dimension::Normalization,
            Dimension::Records,
            Dimension::Depth,
        ] {
            let mut limits = ProofLimits::default();
            match dimension {
                Dimension::Bytes => limits.bytes = 0,
                Dimension::Declarations => limits.declarations = 0,
                Dimension::Expressions => limits.expressions = 0,
                Dimension::Types => limits.types = 0,
                Dimension::Edges => limits.edges = 0,
                Dimension::Values => limits.values = 0,
                Dimension::GraphNodes => limits.graph_nodes = 0,
                Dimension::Facts => limits.facts = 0,
                Dimension::Goals => limits.goals = 0,
                Dimension::Materialized => limits.materialized = 0,
                Dimension::GoalNodes => limits.goal_nodes = 0,
                Dimension::Normalization => limits.normalization = 0,
                Dimension::Records => limits.records = 0,
                Dimension::Depth => limits.depth = 0,
            }
            let failed = proofs::discharge(types, &bindings, limits);
            let exhaustion = failed
                .exhaustion()
                .unwrap_or_else(|| panic!("{dimension:?} must be charged"));
            assert_eq!(exhaustion.dimension, dimension);
            assert_eq!((exhaustion.limit, exhaustion.prior), (0, 0));
            assert!(exhaustion.requested > 0);
            let original = types
                .binding()
                .namespace()
                .unit(exhaustion.site.unit)
                .unwrap();
            assert!(original.source().slice(exhaustion.site.span).is_some());
            assert_eq!(
                failed.disposition(declaration),
                Some(ProofDisposition::Unfinished),
                "{dimension:?}"
            );
            let retry = proofs::discharge(types, &bindings, ProofLimits::default());
            discharged(&retry, "Budget");
            assert_eq!(
                retry.usage(),
                baseline.usage(),
                "fresh retry: {dimension:?}"
            );
            assert_eq!(
                failed.disposition(declaration),
                Some(ProofDisposition::Unfinished)
            );
            assert_eq!(bindings[0].source.source().digest(), expected_digest);
            assert_eq!(
                types.declaration(declaration).unwrap().nodes().len(),
                original_nodes
            );
        }
        let oversized = ProofLimits {
            bytes: usize::MAX,
            declarations: usize::MAX,
            expressions: usize::MAX,
            types: usize::MAX,
            edges: usize::MAX,
            values: usize::MAX,
            graph_nodes: usize::MAX,
            facts: usize::MAX,
            goals: usize::MAX,
            materialized: usize::MAX,
            goal_nodes: usize::MAX,
            normalization: usize::MAX,
            records: usize::MAX,
            depth: usize::MAX,
        };
        let clamped = proofs::discharge(types, &bindings, oversized);
        discharged(&clamped, "Budget");
        assert_eq!(clamped.limits(), ProofLimits::default());
        assert_eq!(clamped.usage(), baseline.usage());
    });
}

#[test]
#[trace("TC-119", "TC-131", "FR-040-AC-1", "FR-040-AC-10", "FR-047-AC-8")]
fn composed_discharge_does_not_upgrade_the_historical_model_boundary() {
    let model = setup::model("HistoricalBoundary");
    let sources = [setup::source("composed", &model,
        "predicate Exact using S (expected: M::Q): Boolean { rational(1,2) + rational(1,2) = expected }")];
    with_types(&model, &sources, |types, formal| {
        let bindings = [mapping(&formal[0], &["Exact"], handler())];
        let report = proofs::discharge(types, &bindings, ProofLimits::default());
        discharged(&report, "Exact");
        let text = format!("language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"{}\" version \"{}\" digest \"{}\";\ninvariant Legacy on M::Node at current {{ true }}",
            model.environment().owner().package().as_str(), model.environment().owner().revision().get(), model.digest());
        let unit = parse(
            SourceIdentity {
                identity: "test:historical-proof-boundary".into(),
                revision: "1".into(),
            },
            "historical.native",
            text.as_bytes(),
            Limits::default(),
        )
        .unwrap();
        let import = unit.imports()[0].span;
        let error =
            link_native(unit, std::slice::from_ref(&model), LinkLimits::default()).unwrap_err();
        assert_eq!(error.code, Code::UnsupportedConstruct);
        assert_eq!(
            (error.span.start.byte, error.span.end.byte),
            (import.start, import.end)
        );
        discharged(&report, "Exact");
    });
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-3", "FR-040-AC-9")]
fn shared_diamond_charges_each_declaration_and_dependency_occurrence_exactly() {
    inspect(
        "predicate Top using S (flag: Boolean): Boolean { Left(flag) and Right(flag) }\n\
        predicate Left using S (flag: Boolean): Boolean { Base(flag) }\n\
        predicate Right using S (flag: Boolean): Boolean { Base(flag) }\n\
        predicate Base using S (flag: Boolean): Boolean { flag }",
        &["Top", "Left", "Right", "Base"],
        |types, bindings| {
            // Four declarations; four authored calls, each charged once at
            // insertion and once when its target settles. No measured usage
            // supplies these exact or one-step-insufficient capacities.
            let limits = ProofLimits {
                declarations: 4,
                edges: 8,
                ..ProofLimits::default()
            };
            let exact = proofs::discharge(types, bindings, limits);
            for name in ["Top", "Left", "Right", "Base"] {
                discharged(&exact, name);
            }
            assert_eq!(exact.usage().declarations, 4);
            assert_eq!(exact.usage().edges, 8);
            let short_edges =
                proofs::discharge(types, bindings, ProofLimits { edges: 7, ..limits });
            let exhaustion = short_edges.exhaustion().unwrap();
            assert_eq!(exhaustion.dimension, Dimension::Edges);
            assert_eq!(
                (exhaustion.limit, exhaustion.prior, exhaustion.requested),
                (7, 7, 1)
            );
            assert_eq!(
                short_edges.disposition(id(types, "Top")),
                Some(ProofDisposition::Unfinished)
            );
            discharged(&short_edges, "Base");
            let short_declarations = proofs::discharge(
                types,
                bindings,
                ProofLimits {
                    declarations: 3,
                    ..limits
                },
            );
            let exhaustion = short_declarations.exhaustion().unwrap();
            assert_eq!(exhaustion.dimension, Dimension::Declarations);
            assert_eq!(
                (exhaustion.limit, exhaustion.prior, exhaustion.requested),
                (3, 3, 1)
            );
            assert_eq!(
                short_declarations.disposition(id(types, "Top")),
                Some(ProofDisposition::Unfinished)
            );
        },
    );
}
