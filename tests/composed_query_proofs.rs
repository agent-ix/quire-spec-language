// SPDX-License-Identifier: AGPL-3.0-only
//! TC-119: actual query definedness over declared domains and authored sources.
//! Discharge proves neither query truth nor execution/order/runtime conformance.

#[path = "support/composed_types/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::checking::composed::proofs::{
    CauseKind, DeclarationProof, ProofDisposition, ProofLimits, ProofReport,
};
use quire_spec_language::checking::composed::{
    self, proofs, CauseKind as TypeCause, TypeDisposition, TypeLimits, TypeReport,
};
use quire_spec_language::checking::{CheckBindings, ClauseBinding, NativeType};
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::linking::composed::{
    binding_work::Limits as BindingLimits,
    scopes::{Anchor, BinderKind},
    DeclarationId,
};
use quire_spec_language::model_source::{self, ModelSourceLimits};
use quire_spec_language::native_model::{ModelLimits, NativeModel};
use quire_spec_language::{Limits, Source, SourceIdentity};
use serde_json::{json, Value};

fn owner() -> ir::RequirementRef {
    ir::RequirementRef::new(
        ir::PackageId::new("test/query-proofs").unwrap(),
        ir::RequirementId::new("QueryDefinedness").unwrap(),
        ir::RequirementRevision::new(7).unwrap(),
    )
}

fn query_model(maximum: u32) -> NativeModel {
    query_model_with_denominator(maximum, 1)
}

fn query_model_with_denominator(maximum: u32, denominator: u32) -> NativeModel {
    let base = if maximum == 5 {
        setup::model("QueryModel")
    } else {
        setup::model_with_maximum("QueryModel", maximum)
    };
    let mut document: Value = serde_json::from_str(base.source().source().text()).unwrap();
    for scalar in document["scalars"].as_array_mut().unwrap() {
        if scalar["name"] == "Q" {
            scalar["maximum_denominator"] = json!(denominator);
        }
    }
    document["scalars"].as_array_mut().unwrap().extend([
        json!({"name":"LargeTotal","kind":"integer","minimum":0,"maximum":200000,"unit":"U"}),
        json!({"name":"AlmostTotal","kind":"integer","minimum":0,"maximum":199999,"unit":"U"}),
        json!({"name":"SignedTotal","kind":"integer","minimum":-100000,"maximum":100000}),
        json!({"name":"AlmostSignedTotal","kind":"integer","minimum":-99999,"maximum":100000}),
        json!({"name":"TinyTotal","kind":"integer","minimum":0,"maximum":10,"unit":"U"}),
        json!({"name":"LargeCount","kind":"integer","minimum":0,"maximum":10000}),
        json!({"name":"SmallCount","kind":"integer","minimum":0,"maximum":4}),
        json!({"name":"WrongTotal","kind":"integer","minimum":0,"maximum":100,"unit":"V"}),
        json!({"name":"NonzeroTotal","kind":"integer","minimum":1,"maximum":100,"unit":"U"}),
        json!({"name":"RationalTotal","kind":"rational","numerator_minimum":0,"numerator_maximum":100,"maximum_denominator":1,"unit":"U"}),
        json!({"name":"IntegralTotal","kind":"rational","numerator_minimum":-5,"numerator_maximum":5,"maximum_denominator":1}),
        json!({"name":"FractionTotal","kind":"rational","numerator_minimum":-5,"numerator_maximum":5,"maximum_denominator":2}),
    ]);
    document["records"][0]["fields"].as_array_mut().unwrap().extend([
        json!({"name":"largeTotal","type":{"kind":"scalar","name":"LargeTotal"}}),
        json!({"name":"almostTotal","type":{"kind":"scalar","name":"AlmostTotal"}}),
        json!({"name":"signedTotal","type":{"kind":"scalar","name":"SignedTotal"}}),
        json!({"name":"almostSignedTotal","type":{"kind":"scalar","name":"AlmostSignedTotal"}}),
        json!({"name":"tinyTotal","type":{"kind":"scalar","name":"TinyTotal"}}),
        json!({"name":"largeCount","type":{"kind":"scalar","name":"LargeCount"}}),
        json!({"name":"smallCount","type":{"kind":"scalar","name":"SmallCount"}}),
        json!({"name":"wrongTotal","type":{"kind":"scalar","name":"WrongTotal"}}),
        json!({"name":"nonzeroTotal","type":{"kind":"scalar","name":"NonzeroTotal"}}),
        json!({"name":"rationalTotal","type":{"kind":"scalar","name":"RationalTotal"}}),
        json!({"name":"integralTotal","type":{"kind":"scalar","name":"IntegralTotal"}}),
        json!({"name":"fractionTotal","type":{"kind":"scalar","name":"FractionTotal"}}),
        json!({"name":"signeds","type":{"kind":"sequence","maximum":maximum,"value":{"kind":"scalar","name":"Signed"}}}),
        json!({"name":"fractions","type":{"kind":"sequence","maximum":maximum,"value":{"kind":"scalar","name":"Q"}}}),
        json!({"name":"optionals","type":{"kind":"sequence","maximum":maximum,"value":{"kind":"option","value":{"kind":"scalar","name":"Q"}}}}),
        json!({"name":"nested","type":{"kind":"sequence","maximum":2,"value":{"kind":"sequence","maximum":3,"value":{"kind":"scalar","name":"Amount"}}}}),
    ]);
    let bytes = serde_json::to_vec_pretty(&document).unwrap();
    let source = Source::read(
        SourceIdentity {
            identity: "model:query-domains".into(),
            revision: "authored".into(),
        },
        "query-domains.json",
        &bytes,
        Limits::default().source_bytes,
    )
    .unwrap();
    let formal = FormalSource::new(
        source,
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new("QueryDomains").unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        ),
    );
    model_source::read(
        formal,
        model_source::FORMAT_V2,
        ModelSourceLimits::default(),
    )
    .unwrap_or_else(|error| panic!("query model frontend: {error}"))
    .admit(ModelLimits::default())
    .unwrap_or_else(|error| panic!("query model admission: {error}"))
}

fn inspect(
    model: &NativeModel,
    body: &str,
    names: &[&str],
    point: ir::ExecutionPoint,
    test: impl FnOnce(&ProofReport<'_, '_, '_>),
) {
    let sources = [setup::source("queries", model, body)];
    let formal = setup::formal_sources(&sources);
    let mappings = [CheckBindings {
        source: formal[0].clone(),
        clauses: names
            .iter()
            .map(|name| ClauseBinding {
                name: (*name).into(),
                requirement: owner(),
                clause: ir::ClauseId::new(name.to_lowercase()).unwrap(),
                execution_point: point.clone(),
            })
            .collect(),
    }];
    setup::with_binding(&sources, &[model], BindingLimits::default(), |binding| {
        assert!(binding.complete(), "{:?}", binding.exhaustion());
        let typed = composed::admit_types(binding, &formal, TypeLimits::default());
        assert!(typed.exhaustion().is_none(), "{:?}", typed.exhaustion());
        let report = proofs::discharge(&typed, &mappings, ProofLimits::default());
        test(&report);
    });
}

fn handler() -> ir::ExecutionPoint {
    ir::ExecutionPoint::Handler {
        name: ir::AnchorName::new("validate").unwrap(),
    }
}

fn id(types: &TypeReport<'_, '_>, name: &str) -> DeclarationId {
    let [id] = types.binding().namespace().lookup(name) else {
        panic!("one original {name}")
    };
    *id
}

#[track_caller]
fn discharged<'a>(report: &'a ProofReport<'_, '_, '_>, name: &str) -> &'a DeclarationProof {
    let at = id(report.types(), name);
    let typed = report.types().declaration(at).unwrap();
    assert_eq!(
        report.types().disposition(at),
        Some(TypeDisposition::Typed),
        "{name}: {:?}",
        typed.causes()
    );
    let proof = report.declaration(at).unwrap();
    assert_eq!(
        proof.disposition(),
        ProofDisposition::Discharged,
        "{name}: {:?}; exhaustion {:?}",
        proof.causes(),
        report.exhaustion()
    );
    assert!(proof.complete());
    assert!(proof.causes().is_empty());
    assert_eq!(proof.environment().unwrap().owner(), &owner());
    proof
}

#[track_caller]
fn unproved(
    report: &ProofReport<'_, '_, '_>,
    name: &str,
    obligation: ir::DefinednessObligationKind,
    original: &str,
) {
    let at = id(report.types(), name);
    assert_eq!(report.types().disposition(at), Some(TypeDisposition::Typed));
    let proof = report.declaration(at).unwrap();
    assert_eq!(proof.disposition(), ProofDisposition::Refused);
    let cause = proof.causes().iter().find(|cause| matches!(&cause.kind, CauseKind::Unproved { diagnostics } if diagnostics.iter().any(|diagnostic| diagnostic.obligation_kind == Some(obligation)))).unwrap_or_else(|| panic!("{name}: expected {obligation:?}: {:?}", proof.causes()));
    let unit = report
        .types()
        .binding()
        .namespace()
        .unit(proof.unit())
        .unwrap();
    assert_eq!(cause.site.declaration, at);
    assert_eq!(cause.site.unit, proof.unit());
    assert_eq!(unit.source().slice(cause.site.span), Some(original));
    assert_eq!(
        unit.expression(cause.site.expression.unwrap())
            .unwrap()
            .span,
        cause.site.span
    );
    assert!(!proof.complete());
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-5", "FR-040-AC-8")]
fn eight_query_forms_discharge_real_domain_obligations_without_proving_query_truth() {
    let model = query_model(5);
    inspect(
        &model,
        "predicate Queries using S (input: M::Node): Boolean {
        size<M::Tally>(input.amounts) >= 0 and contains(input.amounts,2)
        and forall(allItem in input.amounts: allItem >= 1)
        and exists(someItem in input.amounts: someItem <= 20)
        and contains(filter(kept in input.amounts: kept = 2),2)
        and contains(map(projected in input.amounts: projected >= 1),true)
        and count<M::Tally>(counted in input.amounts: counted = 2) >= 0
        and sum<M::Total>(summand in input.amounts: summand) >= 0
    }",
        &["Queries"],
        handler(),
        |report| {
            let proof = discharged(report, "Queries");
            let typed = report.types().declaration(proof.declaration()).unwrap();
            let unit = report
                .types()
                .binding()
                .namespace()
                .unit(proof.unit())
                .unwrap();
            for (source, boolean_element) in [
                ("filter(kept in input.amounts: kept = 2)", false),
                ("map(projected in input.amounts: projected >= 1)", true),
            ] {
                let node = typed
                    .nodes()
                    .iter()
                    .find(|node| unit.source().slice(node.span) == Some(source))
                    .unwrap();
                let NativeType::Sequence { element, maximum } = node.ty.as_ref().unwrap() else {
                    panic!("original query sequence shape")
                };
                assert_eq!(*maximum, 5);
                if boolean_element {
                    assert!(matches!(**element, NativeType::Boolean));
                } else {
                    let NativeType::Scalar { role, .. } = element.as_ref() else {
                        panic!("Amount projection")
                    };
                    assert_eq!(role.name.as_str(), "Amount");
                }
            }
            assert!(!proof.goals().is_empty(), "count/sum use actual IR checks");
            assert!(proof.goals().iter().any(|goal| goal
                .checked()
                .obligations()
                .iter()
                .any(|o| o.kind() == ir::DefinednessObligationKind::CheckedRange)));
            for goal in proof.goals() {
                let native = unit.expression(goal.native_expression()).unwrap();
                assert_eq!(
                    report.bindings()[0]
                        .source
                        .to_native(goal.source())
                        .unwrap(),
                    native.span
                );
                assert!(goal
                    .checked()
                    .obligations()
                    .iter()
                    .all(|obligation| obligation.kind()
                        != ir::DefinednessObligationKind::OptionPresence));
            }
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-5", "FR-040-AC-6")]
fn nested_query_binders_and_captured_sequence_anchors_remain_original() {
    let model = query_model(5);
    inspect(
        &model,
        "post Nested using S on M::Node::step {
        let previous = pre(self.amounts) in
        forall(oldItem in previous: oldItem >= 1)
        and forall(group in self.nested: forall(member in group: member >= 1))
        and forall(currentItem in self.amounts: currentItem >= 1)
    }",
        &["Nested"],
        ir::ExecutionPoint::Post {
            operation: model.roles().operations[0].anchor.clone(),
        },
        |report| {
            let proof = discharged(report, "Nested");
            let binding = report.types().binding();
            let scope = binding
                .scopes()
                .unwrap()
                .declaration(proof.declaration())
                .unwrap();
            let typed = report.types().declaration(proof.declaration()).unwrap();
            let unit = binding.namespace().unit(proof.unit()).unwrap();
            let mut query_ids = Vec::new();
            for name in ["oldItem", "group", "member", "currentItem"] {
                let (original, binder) = scope
                    .binders
                    .iter()
                    .enumerate()
                    .find(|(_, binder)| binder.name.as_deref() == Some(name))
                    .unwrap();
                assert_eq!(binder.kind, BinderKind::Query);
                let retained = typed
                    .binders()
                    .iter()
                    .find(|binder| binder.binder == original)
                    .unwrap();
                assert_eq!(retained.anchor, binder.anchor);
                assert_eq!(unit.source().slice(binder.span), Some(name));
                let occurrence = scope
                    .values
                    .iter()
                    .find(|value| value.target.index() == original)
                    .unwrap();
                assert_eq!(occurrence.unit, proof.unit());
                assert_eq!(unit.source().slice(occurrence.span), Some(name));
                let node = typed.node(occurrence.expression).unwrap();
                assert_eq!(node.binder, Some(occurrence.target));
                assert_eq!(node.anchor, Some(binder.anchor));
                query_ids.push(original);
            }
            query_ids.sort_unstable();
            query_ids.dedup();
            assert_eq!(query_ids.len(), 4);
            let previous = scope
                .binders
                .iter()
                .find(|binder| binder.name.as_deref() == Some("previous"))
                .unwrap();
            assert_eq!(previous.anchor, Anchor::InvocationPost);
            let captured = typed
                .nodes()
                .iter()
                .find(|node| unit.source().slice(node.span) == Some("pre(self.amounts)"))
                .unwrap();
            assert_eq!(
                captured.origin,
                Some(composed::ObservationOrigin::Anchored(Anchor::InvocationPre))
            );
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-4", "FR-040-AC-5", "FR-040-AC-6")]
fn query_body_safety_covers_the_whole_element_domain() {
    let model = query_model(5);
    inspect(&model, "predicate Guarded using S (input: M::Node): Boolean {
        forall(item in input.signeds: item < 10 implies item + 1 <= 10)
        and contains(map(projected in input.signeds: if projected < 10 then projected + 1 else projected),0)
        and forall(kept in filter(candidate in input.signeds: candidate < 10): kept + 1 <= 10)
    }
    predicate ConvenientExists using S (input: M::Node): Boolean {
        exists(item in input.signeds: item + 1 <= 10)
    }
    predicate PartialFilter using S (input: M::Node): Boolean {
        contains(filter(item in input.signeds: item + 1 <= 10),0)
    }", &["Guarded","ConvenientExists","PartialFilter"], handler(), |report| {
        discharged(report, "Guarded");
        for name in ["ConvenientExists", "PartialFilter"] {
            unproved(report, name, ir::DefinednessObligationKind::CheckedRange, "item + 1");
        }
    });
}

#[test]
#[trace("TC-119", "FR-040-AC-4", "FR-040-AC-5", "FR-040-AC-8")]
fn sum_proves_every_prefix_from_declared_capacity_not_a_convenient_final_result() {
    for maximum in [5, 6] {
        let model = query_model(maximum);
        inspect(
            &model,
            "predicate Total using S (input: M::Node): Boolean {
            sum<M::Total>(item in input.amounts: item) >= 0
        }",
            &["Total"],
            handler(),
            |report| {
                if maximum == 5 {
                    let proof = discharged(report, "Total");
                    assert!(proof.goals().iter().any(|goal| goal
                        .checked()
                        .obligations()
                        .iter()
                        .any(|obligation| obligation.kind()
                            == ir::DefinednessObligationKind::CheckedRange)));
                } else {
                    unproved(
                        report,
                        "Total",
                        ir::DefinednessObligationKind::CheckedRange,
                        "sum<M::Total>(item in input.amounts: item)",
                    );
                }
            },
        );
    }
    let model = query_model(3);
    inspect(&model, "predicate Intermediate using S (input: M::Node): Boolean {
        let accumulated = sum<M::Signed>(item in input.signeds: item) in accumulated - accumulated = 0
    }", &["Intermediate"], handler(), |report| {
        // The admitted [10,10,-10] sequence has a final sum of 10, but its
        // second prefix is 20. Even a subsequent x-x result cannot repair it.
        unproved(report, "Intermediate", ir::DefinednessObligationKind::CheckedRange, "sum<M::Signed>(item in input.signeds: item)");
    });
}

#[test]
#[trace("TC-119", "FR-040-AC-3", "FR-040-AC-5", "FR-040-AC-8")]
fn maximal_integer_sum_discharges_real_prefix_bounds_within_default_limits() {
    let model = query_model(10_000);
    inspect(&model, "predicate WideSum using S (input: M::Node): Boolean {
        sum<M::LargeTotal>(item in input.amounts: item) >= 0
    }
    predicate SignedWide using S (input: M::Node): Boolean {
        sum<M::SignedTotal>(item in input.signeds: item) >= 0
    }
    predicate LateUpper using S (input: M::Node): Boolean {
        sum<M::AlmostTotal>(item in input.amounts: item) >= 0
    }
    predicate LateLower using S (input: M::Node): Boolean {
        sum<M::AlmostSignedTotal>(item in input.signeds: item) >= 0
    }
    predicate TinyFirst using S (input: M::Node): Boolean {
        sum<M::TinyTotal>(item in input.amounts: item) >= 0
    }
    predicate Prefix using S (input: M::Node): Boolean {
        let accumulated = sum<M::Signed>(item in input.signeds: item) in accumulated - accumulated = 0
    }", &["WideSum", "SignedWide", "LateUpper", "LateLower", "TinyFirst", "Prefix"], handler(), |report| {
        let proof = discharged(report, "WideSum");
        assert!(report.exhaustion().is_none());
        assert_eq!(report.limits(), ProofLimits::default());
        let typed = report.types().declaration(proof.declaration()).unwrap();
        let unit = report.types().binding().namespace().unit(proof.unit()).unwrap();
        let sum = typed.nodes().iter().find(|node| unit.source().slice(node.span) == Some("sum<M::LargeTotal>(item in input.amounts: item)")).unwrap();
        let domain = typed.nodes().iter().find(|node| unit.source().slice(node.span) == Some("input.amounts")).unwrap();
        let NativeType::Sequence { element, maximum } = domain.ty.as_ref().unwrap() else { panic!("real admitted collection domain") };
        assert_eq!(*maximum, 10_000);
        for (ty, name, bounds) in [
            (element.as_ref(), "Amount", (1, 20)),
            (sum.ty.as_ref().unwrap(), "LargeTotal", (0, 200_000)),
        ] {
            let NativeType::Scalar { model: owner, role, representation: ir::ValueType::Integer { value } } = ty else { panic!("actual nominal integer representation") };
            assert!(std::ptr::eq(*owner, &model));
            assert_eq!(role.name.as_str(), name);
            assert_eq!((value.minimum(), value.maximum()), bounds);
            assert!(matches!(&role.kind, quire_spec_language::native_model::ScalarKind::Integer { unit: quire_spec_language::native_model::Unit::Named(unit) } if unit.as_str() == "U"));
        }
        // The declared envelope is independently 10,000 * [1,20], with zero
        // included for an empty collection. The real IR must discharge it.
        let sum_goals: Vec<_> = proof.goals().iter().filter(|goal| goal.native_expression() == sum.expression).collect();
        assert!(!sum_goals.is_empty());
        assert!(sum_goals.iter().any(|goal| goal.checked().obligations().iter().any(|obligation| obligation.kind() == ir::DefinednessObligationKind::CheckedRange)));
        for goal in sum_goals {
            assert_eq!(report.bindings()[0].source.to_native(goal.source()).unwrap(), sum.span);
        }
        let signed = discharged(report, "SignedWide");
        assert!(signed.goals().iter().any(|goal| goal.checked().obligations().iter().any(|obligation| obligation.kind() == ir::DefinednessObligationKind::CheckedRange)));
        // 9,999 upper endpoints total 199,980; the 10,000th reaches 200,000.
        // The negative endpoint reaches -100,000 at that same final position.
        unproved(report, "LateUpper", ir::DefinednessObligationKind::CheckedRange, "sum<M::AlmostTotal>(item in input.amounts: item)");
        unproved(report, "LateLower", ir::DefinednessObligationKind::CheckedRange, "sum<M::AlmostSignedTotal>(item in input.signeds: item)");
        let tiny = id(report.types(), "TinyFirst");
        assert_eq!(report.types().disposition(tiny), Some(TypeDisposition::Refused));
        let cause = report.types().declaration(tiny).unwrap().causes().iter().find(|cause| cause.kind == TypeCause::InvalidAggregateDomain).unwrap();
        assert_eq!(cause.site.declaration, tiny);
        assert_eq!(cause.site.unit, typed.unit());
        assert_eq!(unit.source().slice(cause.site.span), Some("sum<M::TinyTotal>(item in input.amounts: item)"));
        assert_eq!(unit.expression(cause.site.expression.unwrap()).unwrap().span, cause.site.span);
        assert_eq!(report.disposition(tiny), Some(ProofDisposition::Refused));
        assert!(report.declaration(tiny).unwrap().causes().iter().any(|cause| matches!(cause.kind, CauseKind::UpstreamType)));
        // [10,10,-10,-10] is permitted by this maximum and has final total zero,
        // but its second prefix is 20. Cancellation cannot repair the fold.
        unproved(report, "Prefix", ir::DefinednessObligationKind::CheckedRange, "sum<M::Signed>(item in input.signeds: item)");
    });
}

#[test]
#[trace("TC-119", "FR-040-AC-3", "FR-040-AC-5")]
fn aggregate_result_domains_keep_exact_units_representation_zero_and_length() {
    let model = query_model(5);
    for expression in [
        "count<M::SmallCount>(item in input.amounts: item >= 1) >= 0",
        "size<M::SmallCount>(input.amounts) >= 0",
        "sum<M::WrongTotal>(item in input.amounts: item) >= 0",
        "sum<M::NonzeroTotal>(item in input.amounts: item) >= 1",
        "sum<M::RationalTotal>(item in input.amounts: item) >= rational(0,1)",
    ] {
        let body =
            format!("predicate WrongDomain using S (input: M::Node): Boolean {{ {expression} }}");
        inspect(&model, &body, &["WrongDomain"], handler(), |report| {
            let at = id(report.types(), "WrongDomain");
            assert_eq!(
                report.types().disposition(at),
                Some(TypeDisposition::Refused)
            );
            let typed = report.types().declaration(at).unwrap();
            let cause = typed
                .causes()
                .iter()
                .find(|cause| cause.kind == TypeCause::InvalidAggregateDomain)
                .unwrap_or_else(|| panic!("{:?}", typed.causes()));
            assert_eq!(cause.site.declaration, at);
            assert_eq!(cause.site.unit, typed.unit());
            let unit = report
                .types()
                .binding()
                .namespace()
                .unit(typed.unit())
                .unwrap();
            assert_eq!(
                unit.expression(cause.site.expression.unwrap())
                    .unwrap()
                    .span,
                cause.site.span
            );
            assert_eq!(report.disposition(at), Some(ProofDisposition::Refused));
            assert!(report
                .declaration(at)
                .unwrap()
                .causes()
                .iter()
                .any(|cause| matches!(cause.kind, CauseKind::UpstreamType)));
        });
    }
}

#[test]
#[trace("TC-119", "FR-040-AC-5", "FR-040-AC-8")]
fn empty_results_and_maximal_declared_domains_keep_their_actual_admission_boundaries() {
    let zero = setup::try_model_with_maximum("ZeroQueryMaximum", 0).unwrap_err();
    let model_source::ModelSourceCause::Formal(errors) = &zero.cause else {
        panic!("original IR zero-maximum refusal")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, ir::DiagnosticCode::UnboundedCollection);

    let model = query_model(5);
    inspect(
        &model,
        "predicate Empty using S (input: M::Node): Boolean {
        let absent = filter(item in input.amounts: false) in
        size<M::Tally>(absent) = 0 and not contains(absent,1)
        and forall(universal in absent: false) and not exists(existential in absent: true)
        and count<M::Tally>(counted in absent: true) = 0
        and sum<M::Total>(summand in absent: summand) = 0
    }",
        &["Empty"],
        handler(),
        |report| {
            // Only definedness is asserted; this phase is not an empty-query evaluator.
            discharged(report, "Empty");
        },
    );
    // Native model admission now owns this ceiling, before a type/proof report exists.
    let excess = setup::try_model_with_maximum("ExcessQueryMaximum", 10_001).unwrap_err();
    assert_eq!(
        excess.code(),
        quire_spec_language::Code::UnsupportedConstruct
    );
    assert!(!excess.is_incomplete());
    let model_source::ModelSourceCause::Admission(cause) = &excess.cause else {
        panic!("native admission must own the sequence refusal: {excess}")
    };
    assert_eq!(cause.code, quire_spec_language::Code::UnsupportedConstruct);
    assert_eq!(cause.phase, quire_spec_language::Phase::Link);
    assert_eq!(cause.source.identity, "model:ExcessQueryMaximum");
    assert_eq!(cause.source.revision, "authored");
    assert_eq!(cause.path, "ExcessQueryMaximum.json");
    assert_eq!(&cause.source, excess.source().source().identity());
    assert_eq!(
        cause.span,
        excess
            .source()
            .source()
            .locate(quire_spec_language::Span { start: 0, end: 0 })
            .unwrap()
    );

    let model = query_model(10_000);
    inspect(
        &model,
        "predicate Bounded using S (input: M::Node): Boolean {
            size<M::LargeCount>(input.amounts) >= 0
            and count<M::LargeCount>(counted in input.amounts: counted >= 1) >= 0
            and forall(item in input.amounts: item >= 1)
        }",
        &["Bounded"],
        handler(),
        |report| {
            discharged(report, "Bounded");
            assert!(report.exhaustion().is_none());
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-4", "FR-040-AC-5", "FR-040-AC-8")]
fn rational_sum_separates_supported_prefixes_from_missing_domain_transfer() {
    let integral = query_model(5);
    inspect(
        &integral,
        "predicate Integral using S (input: M::Node): Boolean {
        sum<M::IntegralTotal>(part in input.fractions: part) >= rational(-5,1)
    }",
        &["Integral"],
        handler(),
        |report| {
            let proof = discharged(report, "Integral");
            assert!(proof
                .goals()
                .iter()
                .any(|goal| goal
                    .checked()
                    .obligations()
                    .iter()
                    .any(|obligation| obligation.kind()
                        == ir::DefinednessObligationKind::CheckedRange)));
        },
    );
    let fractional = query_model_with_denominator(5, 2);
    inspect(
        &fractional,
        "predicate Fractional using S (input: M::Node): Boolean {
        sum<M::FractionTotal>(part in input.fractions: part) >= rational(-5,1)
    }",
        &["Fractional"],
        handler(),
        |report| {
            let at = id(report.types(), "Fractional");
            assert_eq!(report.types().disposition(at), Some(TypeDisposition::Typed));
            let proof = report.declaration(at).unwrap();
            assert_eq!(proof.disposition(), ProofDisposition::Refused);
            let cause = proof
                .causes()
                .iter()
                .find(|cause| {
                    matches!(
                        cause.kind,
                        CauseKind::Unsupported(proofs::Unsupported::SumDomainTransfer)
                    )
                })
                .unwrap_or_else(|| panic!("{:?}", proof.causes()));
            assert_eq!(cause.site.declaration, at);
            assert_eq!(cause.site.unit, proof.unit());
            let unit = report
                .types()
                .binding()
                .namespace()
                .unit(proof.unit())
                .unwrap();
            assert_eq!(
                unit.source().slice(cause.site.span),
                Some("sum<M::FractionTotal>(part in input.fractions: part)")
            );
            assert_eq!(
                unit.expression(cause.site.expression.unwrap())
                    .unwrap()
                    .span,
                cause.site.span
            );
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-5", "FR-040-AC-9")]
fn query_body_proof_exhaustion_keeps_original_types_and_a_fresh_retry() {
    let model = query_model(5);
    inspect(
        &model,
        "predicate Budget using S (input: M::Node): Boolean {
        forall(item in input.signeds: item < 10 implies item + 1 <= 10)
    }",
        &["Budget"],
        handler(),
        |original| {
            let complete = discharged(original, "Budget");
            let goal_count = complete.goals().len();
            assert!(goal_count > 0);
            let limited = proofs::discharge(
                original.types(),
                original.bindings(),
                ProofLimits {
                    goals: 0,
                    ..ProofLimits::default()
                },
            );
            let exhaustion = limited
                .exhaustion()
                .expect("first actual query-body proof is unaffordable");
            assert_eq!(exhaustion.dimension, proofs::work::Dimension::Goals);
            assert_eq!((exhaustion.prior, exhaustion.limit), (0, 0));
            let at = id(original.types(), "Budget");
            assert_eq!(
                limited.types().disposition(at),
                Some(TypeDisposition::Typed)
            );
            assert_eq!(limited.disposition(at), Some(ProofDisposition::Unfinished));
            assert_eq!(exhaustion.site.declaration, at);
            assert!(!limited.declaration(at).unwrap().complete());
            let retry = proofs::discharge(
                original.types(),
                original.bindings(),
                ProofLimits::default(),
            );
            assert_eq!(discharged(&retry, "Budget").goals().len(), goal_count);
            assert_eq!(discharged(original, "Budget").goals().len(), goal_count);
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-4", "FR-040-AC-5", "FR-040-AC-6")]
fn optional_and_division_query_guards_belong_to_the_same_element() {
    let model = query_model(5);
    inspect(
        &model,
        "predicate Presence using S (input: M::Node): Boolean {
        forall(part in input.optionals: present(part) implies value(part) = rational(0,1))
    }
    predicate Reversed using S (input: M::Node): Boolean {
        exists(part in input.optionals: value(part) = rational(0,1) and present(part))
    }
    predicate Division using S (input: M::Node): Boolean {
        forall(part in input.fractions: part != rational(0,1) implies part / part = rational(1,1))
    }
    predicate Unguarded using S (input: M::Node): Boolean {
        exists(part in input.fractions: part / part = rational(1,1))
    }
    predicate IndependentElements using S (input: M::Node): Boolean {
        let kept = filter(item in input.fractions: true) in
        forall(lhs in kept: forall(rhs in kept:
            lhs != rational(0,1) implies rational(0,1) / rhs = rational(0,1)))
    }",
        &[
            "Presence",
            "Reversed",
            "Division",
            "Unguarded",
            "IndependentElements",
        ],
        handler(),
        |report| {
            discharged(report, "Presence");
            discharged(report, "Division");
            unproved(
                report,
                "Reversed",
                ir::DefinednessObligationKind::OptionPresence,
                "value(part)",
            );
            unproved(
                report,
                "Unguarded",
                ir::DefinednessObligationKind::NonZeroDivisor,
                "part / part",
            );
            unproved(
                report,
                "IndependentElements",
                ir::DefinednessObligationKind::NonZeroDivisor,
                "rational(0,1) / rhs",
            );
        },
    );
}
