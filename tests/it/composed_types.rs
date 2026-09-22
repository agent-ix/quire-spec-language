// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-119: public composed type admission over real /2 producer and binding inputs.
//! Typed values retain pending proof obligations; no test supplies observations.

use crate::support::composed_types as setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::checking::composed::work::{Dimension, Work};
use quire_spec_language::checking::composed::{
    self, CauseKind, ObligationKind, Prerequisite, TypeDisposition, TypeLimits, TypeReport,
};
use quire_spec_language::checking::NativeType;
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::linking::composed::binding::{self, Disposition as BindingDisposition};
use quire_spec_language::linking::composed::binding_work::Limits as BindingLimits;
use quire_spec_language::linking::composed::scopes::{Anchor, BinderKind};
use quire_spec_language::linking::composed::DeclarationId;

fn id(binding: &binding::Report<'_>, name: &str) -> DeclarationId {
    let [id] = binding.namespace().lookup(name) else {
        panic!("one authored declaration named {name}")
    };
    *id
}

fn inspect(body: &str, test: impl FnOnce(&binding::Report<'_>, &[FormalSource])) {
    let model = setup::model("Primary");
    let sources = [setup::source("main", &model, body)];
    let formal = setup::formal_sources(&sources);
    setup::with_binding(&sources, &[&model], BindingLimits::default(), |binding| {
        assert!(binding.complete(), "{:?}", binding.exhaustion());
        test(binding, &formal);
    });
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-3")]
fn model_values_use_selected_inventory_and_ambiguous_inputs_refuse_upstream() {
    let selected = setup::model("SelectedValues");
    let spare = setup::model("UnusedValues");
    let sources = [setup::source("model-values", &selected,
        "post Changed using S on M::Node::step { self.signed = pre(self.signed + delta) and result }")];
    let formal = setup::formal_sources(&sources);
    setup::with_binding(
        &sources,
        &[&spare, &selected],
        BindingLimits::default(),
        |binding| {
            let report = composed::admit_types(binding, &formal, TypeLimits::default());
            let declaration = id(binding, "Changed");
            assert_eq!(
                report.disposition(declaration),
                Some(TypeDisposition::Typed)
            );
            let typed = report.declaration(declaration).unwrap();
            let delta = typed
                .nodes()
                .iter()
                .find(|node| sources[0].slice(node.span) == Some("delta"))
                .unwrap();
            let Some(NativeType::Scalar { model, role, .. }) = &delta.ty else {
                panic!("selected model input must retain its nominal scalar type");
            };
            assert_eq!(model.digest(), selected.digest());
            assert_eq!(model.environment().owner(), selected.environment().owner());
            assert_eq!(role.name.as_str(), "Signed");
            assert!(report.exhaustion().is_none());
        },
    );
    // Equal duplicate inputs cannot manufacture an unambiguous model binding.
    setup::with_binding(
        &sources,
        &[&selected, &selected],
        BindingLimits::default(),
        |binding| {
            let declaration = id(binding, "Changed");
            assert_eq!(
                binding.disposition(declaration),
                Some(BindingDisposition::Refused)
            );
            let report = composed::admit_types(binding, &formal, TypeLimits::default());
            refused(&report, "Changed", CauseKind::UpstreamBinding);
            assert!(report
                .declaration(declaration)
                .unwrap()
                .causes()
                .iter()
                .all(|cause| cause.profile.is_none()));
        },
    );
}

fn scalar_name<'a>(ty: &'a NativeType<'_>) -> &'a str {
    let NativeType::Scalar { role, .. } = ty else {
        panic!("expected actual nominal scalar, got {ty:?}")
    };
    role.name.as_str()
}

fn refused(report: &TypeReport<'_, '_>, name: &str, expected: CauseKind) {
    let declaration = id(report.binding(), name);
    assert_eq!(
        report.disposition(declaration),
        Some(TypeDisposition::Refused)
    );
    let typed = report.declaration(declaration).unwrap();
    assert!(
        typed.causes().iter().any(|cause| cause.kind == expected),
        "{name}: {:?}",
        typed.causes()
    );
    for cause in typed.causes() {
        assert_eq!(cause.site.declaration, declaration);
        assert_eq!(cause.site.unit, typed.unit());
        let unit = report.binding().namespace().unit(cause.site.unit).unwrap();
        assert!(unit.source().slice(cause.site.span).is_some());
        if let Some(expression) = cause.site.expression {
            let original = unit.expression(expression).unwrap();
            assert!(original.span.start <= cause.site.span.start);
            assert!(original.span.end >= cause.site.span.end);
        }
    }
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-3", "FR-040-AC-4")]
fn contextual_literals_keep_exact_native_types_and_original_occurrences() {
    inspect(
        "predicate Exact using S (fraction: M::Q, amount: M::Amount, label: M::Label): Boolean {\n\
         let saved = rational(2,4) in saved = fraction and amount >= 1 and label = \"Ω🙂\"\n}",
        |binding, formal| {
            let declaration = id(binding, "Exact");
            assert_eq!(
                binding.disposition(declaration),
                Some(BindingDisposition::NamesResolved)
            );
            let report = composed::admit_types(binding, formal, TypeLimits::default());
            assert!(report.exhaustion().is_none());
            assert_eq!(
                report.disposition(declaration),
                Some(TypeDisposition::Typed)
            );
            let typed = report.declaration(declaration).unwrap();
            assert!(typed.complete());
            assert!(typed.causes().is_empty(), "{:?}", typed.causes());
            let unit = binding.namespace().unit(typed.unit()).unwrap();
            for node in typed.nodes() {
                assert_eq!(unit.expression(node.expression).unwrap().span, node.span);
                assert!(node.ty.is_some(), "{:?}", node);
            }
            for (original, expected) in [
                ("rational(2,4)", "Q"),
                ("1", "Amount"),
                ("\"Ω🙂\"", "Label"),
            ] {
                let node = typed
                    .nodes()
                    .iter()
                    .find(|node| unit.source().slice(node.span) == Some(original))
                    .unwrap();
                assert_eq!(scalar_name(node.ty.as_ref().unwrap()), expected);
                if expected == "Q" {
                    assert_eq!(node.normalized_rational, Some((1, 2)));
                    let NativeType::Scalar {
                        model,
                        representation: ir::ValueType::Rational { value },
                        ..
                    } = node.ty.as_ref().unwrap()
                    else {
                        panic!("actual rational IR producer type")
                    };
                    assert_eq!(
                        (
                            value.numerator_minimum(),
                            value.numerator_maximum(),
                            value.maximum_denominator()
                        ),
                        (-1, 1, 2)
                    );
                    assert_eq!(
                        model.environment().owner().package().as_str(),
                        "test/Primary"
                    );
                }
            }
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-3", "FR-040-AC-5")]
fn optional_and_sequence_relations_supply_literal_context_without_flattening() {
    inspect(
        "predicate Relations using S (input: M::Node): Boolean {\n\
         (present(input.optionalQ) implies value(input.optionalQ) = rational(2,4)) and\n\
         contains(input.amounts, 1) and\n\
         forall(item in input.amounts: item >= 1) and exists(otherItem in input.amounts: otherItem = 1) and\n\
         size<M::Tally>(filter(kept in input.amounts: kept >= 1)) >= 0 and\n\
         contains(map(projected in input.amounts: projected >= 1), true) and\n\
         count<M::Tally>(counted in input.amounts: counted >= 1) >= 0 and\n\
         sum<M::Total>(summand in input.amounts: summand) >= 0\n}",
        |binding, formal| {
            let declaration = id(binding, "Relations");
            assert_eq!(binding.disposition(declaration), Some(BindingDisposition::NamesResolved));
            let report = composed::admit_types(binding, formal, TypeLimits::default());
            assert_eq!(report.disposition(declaration), Some(TypeDisposition::Typed));
            let typed = report.declaration(declaration).unwrap();
            let source = binding.namespace().unit(typed.unit()).unwrap().source();
            let filtered = typed.nodes().iter().find(|node| source.slice(node.span) == Some("filter(kept in input.amounts: kept >= 1)")).unwrap();
            let NativeType::Sequence { element, maximum } = filtered.ty.as_ref().unwrap() else {
                panic!("filter preserves an ordered sequence")
            };
            assert_eq!(*maximum, 5);
            assert_eq!(scalar_name(element), "Amount");
            let mapped = typed.nodes().iter().find(|node| source.slice(node.span) == Some("map(projected in input.amounts: projected >= 1)")).unwrap();
            assert!(matches!(mapped.ty, Some(NativeType::Sequence { ref element, maximum: 5 }) if matches!(**element, NativeType::Boolean)));
            // Presence and accumulated-prefix discharge remain later work.
            for kind in [ObligationKind::Presence, ObligationKind::SumPrefixes] {
                assert!(typed.obligations().iter().any(|obligation| obligation.kind == kind));
            }
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-2", "FR-040-AC-3")]
fn forbidden_operators_and_container_equality_refuse_even_in_unused_or_skipped_syntax() {
    for expression in [
        "input.signed / input.signed = input.signed",
        "input.signed mod input.signed = input.signed",
        "true < false",
        "input.mode < M::Mode::Ready",
        "input.optionalQ = input.optionalQ",
        "input.amounts = input.amounts",
        "input.plain = input.plain",
        "input.distance * input.distance = input.distance",
        "input.measured / input.measured = input.measured",
    ] {
        let body = format!(
            "predicate Unused using S (input: M::Node): Boolean {{ false implies ({expression}) }}\n\
             predicate Independent using S (): Boolean {{ true }}"
        );
        inspect(&body, |binding, formal| {
            let report = composed::admit_types(binding, formal, TypeLimits::default());
            refused(&report, "Unused", CauseKind::ForbiddenOperator);
            assert_eq!(
                report.disposition(id(binding, "Independent")),
                Some(TypeDisposition::Typed)
            );
        });
    }
}

#[test]
#[trace("TC-119", "FR-040-AC-3")]
fn structural_similarity_does_not_coerce_nominal_scalar_units_or_literal_types() {
    for expression in [
        "input.fraction = input.twin",
        "input.distance = input.duration",
        "input.signed = input.fraction",
        "input.fraction = 1",
        "input.signed = rational(1,1)",
        "if true then input.fraction else input.twin",
        "contains(input.amounts, input.fraction)",
        "value(input.optionalQ) = input.twin",
        "input.label = true",
    ] {
        let body =
            format!("predicate Mismatch using S (input: M::Node): Boolean {{ {expression} }}");
        inspect(&body, |binding, formal| {
            let report = composed::admit_types(binding, formal, TypeLimits::default());
            refused(&report, "Mismatch", CauseKind::TypeMismatch);
        });
    }
    inspect(
        "predicate Ambiguous using S (): Boolean { 1 = 1 }",
        |binding, formal| {
            let report = composed::admit_types(binding, formal, TypeLimits::default());
            refused(&report, "Ambiguous", CauseKind::AmbiguousType);
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-3", "FR-040-AC-4")]
fn exact_literal_domain_failures_are_distinct_from_zero_denominator() {
    for (expression, expected) in [
        ("input.fraction = rational(1,3)", CauseKind::LiteralDomain),
        ("input.fraction = rational(2,1)", CauseKind::LiteralDomain),
        ("input.fraction = rational(1,0)", CauseKind::ZeroDenominator),
        ("input.amount = 0", CauseKind::LiteralDomain),
        ("input.label = \"Ω🙂ab\"", CauseKind::LiteralDomain),
    ] {
        let body =
            format!("predicate Literal using S (input: M::Node): Boolean {{ {expression} }}");
        inspect(&body, |binding, formal| {
            let report = composed::admit_types(binding, formal, TypeLimits::default());
            refused(&report, "Literal", expected);
        });
    }
}

#[test]
#[trace("TC-119", "TC-126", "FR-040-AC-1", "FR-040-AC-2", "FR-046-AC-2")]
fn exact_profiles_are_checked_locally_and_a_wider_caller_cannot_upgrade_a_callee() {
    inspect(
        "predicate Base using S (input: M::Node): Boolean { input.n >= 0 }\n\
         predicate CorePredicate using C (): Boolean { true }\n\
         invariant CoreCall using C on M::Node at current { false implies Base(self) }\n\
         invariant CoreQuery using C on M::Node at current { false implies contains(filter(item in self.amounts: true), 1) }\n\
         predicate Narrow using S (input: M::Node): Boolean { deref(input.peer).n >= 0 }\n\
         predicate Wider using G (input: M::Node): Boolean { Narrow(input) }\n\
         predicate Graph using G (input: M::Node): Boolean { deref(input.peer).n >= 0 and reaches(input, input, parent) }",
        |binding, formal| {
            let report = composed::admit_types(binding, formal, TypeLimits::default());
            refused(&report, "CorePredicate", CauseKind::UpstreamBinding);
            assert!(report.declaration(id(binding, "CorePredicate")).unwrap()
                .causes().iter().all(|cause| cause.profile.is_none()));
            for name in ["CoreCall", "CoreQuery", "Narrow"] {
                refused(&report, name, CauseKind::ProfilePermission);
            }
            refused(&report, "Wider", CauseKind::Dependency { target: id(binding, "Narrow") });
            assert_eq!(report.disposition(id(binding, "Graph")), Some(TypeDisposition::Typed));
            let graph = report.declaration(id(binding, "Graph")).unwrap();
            assert!(graph.obligations().iter().any(|obligation| obligation.kind == ObligationKind::GraphClosure));
            let causes = report.declaration(id(binding, "Narrow")).unwrap().causes();
            let definitions = binding.definitions().unwrap();
            for cause in causes.iter().filter(|cause| cause.kind == CauseKind::ProfilePermission) {
                assert_eq!(definitions.declarations[id(binding, "Narrow").index()].uses[cause.profile.unwrap()].closure[0],
                    quire_spec_language::linking::composed::definition_source::RegisteredDefinition::StateQueries);
            }
        },
    );
}

#[test]
#[trace(
    "TC-119",
    "TC-126",
    "FR-040-AC-1",
    "FR-040-AC-3",
    "FR-046-AC-1",
    "FR-046-AC-2"
)]
fn call_arguments_keep_declared_arity_order_and_contextual_types() {
    inspect(
        "predicate Callee using S (flag: Boolean, fraction: M::Q, label: M::Label): Boolean { flag and fraction = rational(1,2) and label = \"ok\" }\n\
         predicate Valid using S (): Boolean { Callee(true, rational(2,4), \"Ω\") }\n\
         predicate Short using S (): Boolean { Callee(true, rational(1,2)) }\n\
         predicate Long using S (): Boolean { Callee(true, rational(1,2), \"ok\", false) }\n\
         predicate Reordered using S (): Boolean { Callee(\"ok\", rational(1,2), true) }\n\
         predicate NotBoolean using S (valueIn: M::Q): Boolean { valueIn }",
        |binding, formal| {
            let report = composed::admit_types(binding, formal, TypeLimits::default());
            assert_eq!(report.disposition(id(binding, "Valid")), Some(TypeDisposition::Typed));
            let valid = report.declaration(id(binding, "Valid")).unwrap();
            assert!(valid.obligations().iter().any(|obligation| obligation.kind == ObligationKind::PredicateTotal { target: id(binding, "Callee") }));
            refused(&report, "Short", CauseKind::CallArity { expected: 3, actual: 2 });
            refused(&report, "Long", CauseKind::CallArity { expected: 3, actual: 4 });
            refused(&report, "Reordered", CauseKind::TypeMismatch);
            refused(&report, "NotBoolean", CauseKind::TypeMismatch);
        },
    );
}

#[test]
#[trace(
    "TC-119",
    "TC-126",
    "FR-040-AC-1",
    "FR-040-AC-3",
    "FR-046-AC-1",
    "FR-046-AC-2"
)]
fn equal_type_spellings_in_distinct_units_retain_their_exact_model_owners() {
    let first = setup::model("First");
    let second = setup::model("Second");
    let sources = [
        setup::source("callee", &first, "predicate Accept using S (input: M::Q): Boolean { input = rational(1,2) }"),
        setup::source("compatible", &first, "predicate SameOwner using S (input: M::Q): Boolean { Accept(input) }"),
        setup::source("foreign", &second, "predicate Foreign using S (input: M::Q): Boolean { Accept(input) }\npredicate Local using S (input: M::Q): Boolean { input = rational(1,2) }"),
    ];
    let formal = setup::formal_sources(&sources);
    setup::with_binding(
        &sources,
        &[&first, &second],
        BindingLimits::default(),
        |binding| {
            assert!(binding.complete());
            let report = composed::admit_types(binding, &formal, TypeLimits::default());
            for name in ["Accept", "SameOwner", "Local"] {
                assert_eq!(
                    report.disposition(id(binding, name)),
                    Some(TypeDisposition::Typed)
                );
            }
            refused(&report, "Foreign", CauseKind::TypeMismatch);
            let mut seen = Vec::new();
            for (name, expected) in [
                ("Accept", "test/First"),
                ("SameOwner", "test/First"),
                ("Local", "test/Second"),
            ] {
                let typed = report.declaration(id(binding, name)).unwrap();
                let unit = binding.namespace().unit(typed.unit()).unwrap();
                let input = typed
                    .nodes()
                    .iter()
                    .find(|node| unit.source().slice(node.span) == Some("input"))
                    .unwrap();
                let NativeType::Scalar { model, role, .. } = input.ty.as_ref().unwrap() else {
                    panic!("model-owned Q parameter")
                };
                assert_eq!(model.environment().owner().package().as_str(), expected);
                assert_eq!(role.name.as_str(), "Q");
                seen.push((typed.unit(), input.expression));
            }
            assert_ne!(seen[0].0, seen[1].0);
            assert_ne!(seen[1].0, seen[2].0);
        },
    );
}

#[test]
#[trace(
    "TC-119",
    "TC-126",
    "FR-040-AC-1",
    "FR-040-AC-6",
    "FR-040-AC-7",
    "FR-046-AC-1",
    "FR-046-AC-8"
)]
fn type_facts_keep_original_let_query_activation_and_invocation_anchors() {
    inspect(
        "post Changed using S on M::Node::step { self.signed = pre(let prior = self.signed in prior + delta) and result }\n\
         predicate Query using S (input: M::Node): Boolean { forall(item in input.amounts: item >= 1) }\n\
         temporal Due using T over (view: M::Node) clock \"clock\" on each (trigger: M::Node) when (trigger.n >= 0) {\n\
           capture saved: M::Q = trigger.fraction; eventually[0,1] holds(saved = view.fraction)\n}\n\
         protocol Flow using P over (view: M::Node) on origin {\n\
           capture initial: M::Q = view.fraction; role Actor on M::Node;\n\
           run sequence Main { event Happened by Actor as (eventValue: M::Node) { eventValue.fraction = initial };\n\
             check CoreCheck using C { view.n >= 0 }; }\n\
           finish Closed as (closed: M::Node) { closed.fraction = initial };\n}",
        |binding, formal| {
            let report = composed::admit_types(binding, formal, TypeLimits::default());
            for name in ["Changed", "Query", "Due", "Flow"] {
                let declaration = id(binding, name);
                assert_eq!(report.disposition(declaration), Some(TypeDisposition::Typed), "{name}: {:?}", report.declaration(declaration).unwrap().causes());
                let typed = report.declaration(declaration).unwrap();
                let scope = binding.scopes().unwrap().declaration(declaration).unwrap();
                for binder in typed.binders() {
                    assert_eq!(binder.anchor, scope.binders[binder.binder].anchor);
                    assert!(binder.ty.is_some());
                }
                for occurrence in &scope.values {
                    let node = typed.node(occurrence.expression).unwrap();
                    assert_eq!(node.binder, Some(occurrence.target));
                    assert_eq!(node.anchor, Some(scope.binders[occurrence.target.index()].anchor));
                    assert_eq!(node.span, occurrence.span);
                    assert!(node.ty.is_some());
                }
            }
            for (declaration, name, kind, anchor) in [
                ("Changed", "prior", BinderKind::Let, Anchor::InvocationPre),
                ("Changed", "delta", BinderKind::InvocationParameter, Anchor::InvocationInput),
                ("Query", "item", BinderKind::Query, Anchor::Predicate),
                ("Due", "saved", BinderKind::Capture, Anchor::Activation),
                ("Flow", "initial", BinderKind::Capture, Anchor::Activation),
            ] {
                let scope = binding.scopes().unwrap().declaration(id(binding, declaration)).unwrap();
                let binder = scope.binders.iter().find(|binder| binder.name.as_deref() == Some(name)).unwrap();
                assert_eq!(binder.kind, kind);
                assert_eq!(binder.anchor, anchor);
            }
            let changed = report.declaration(id(binding, "Changed")).unwrap();
            for kind in [ObligationKind::InvocationContext, ObligationKind::ArithmeticRange] {
                assert!(changed.obligations().iter().any(|obligation| obligation.kind == kind));
            }
            let flow = report.declaration(id(binding, "Flow")).unwrap();
            let source = binding.namespace().unit(flow.unit()).unwrap().source();
            let core = flow.nodes().iter().find(|node| source.slice(node.span) == Some("view.n >= 0")).unwrap();
            assert_eq!(binding.definitions().unwrap().declarations[id(binding, "Flow").index()].uses[core.profile].closure[0],
                quire_spec_language::linking::composed::definition_source::RegisteredDefinition::StateCore);
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-3", "FR-040-AC-6")]
fn pre_requires_an_eligible_read_and_cannot_retag_an_outer_capture() {
    inspect(
        "post ParameterOnly using S on M::Node::step { pre(delta) = delta }\n\
         post CapturedPost using S on M::Node::step { let captured = self.signed in pre(captured) = delta }\n\
         post Composite using S on M::Node::step { pre(self.signed + delta) = self.signed }",
        |binding, formal| {
            let report = composed::admit_types(binding, formal, TypeLimits::default());
            refused(&report, "ParameterOnly", CauseKind::InvalidPreSelection);
            refused(&report, "CapturedPost", CauseKind::InvalidPreSelection);
            assert_eq!(report.disposition(id(binding, "Composite")), Some(TypeDisposition::Typed));
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-3", "FR-040-AC-5")]
fn aggregate_domain_checks_are_distinct_from_pending_prefix_proofs() {
    for (expression, expected) in [
        (
            "size<M::Count>(input.amounts) >= 0",
            CauseKind::InvalidAggregateDomain,
        ),
        (
            "count<M::Count>(item in input.amounts: true) >= 0",
            CauseKind::InvalidAggregateDomain,
        ),
        (
            "sum<M::Distance>(item in input.amounts: item) >= 0",
            CauseKind::InvalidAggregateDomain,
        ),
        (
            "sum<M::Q>(item in input.amounts: item) = input.fraction",
            CauseKind::InvalidAggregateDomain,
        ),
        (
            "sum<M::Amount>(item in input.amounts: item) >= 1",
            CauseKind::InvalidAggregateDomain,
        ),
    ] {
        let body =
            format!("predicate Aggregate using S (input: M::Node): Boolean {{ {expression} }}");
        inspect(&body, |binding, formal| {
            let report = composed::admit_types(binding, formal, TypeLimits::default());
            refused(&report, "Aggregate", expected);
        });
    }
    // A zero declaration maximum is refused by the actual IR producer before
    // type admission; it is not an admitted empty-sequence type.
    let error = setup::try_model_with_maximum("ZeroMaximum", 0).unwrap_err();
    let quire_spec_language::model_source::ModelSourceCause::Formal(errors) = &error.cause else {
        panic!("actual IR collection refusal: {error:?}")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, ir::DiagnosticCode::UnboundedCollection);

    // The native admission ceiling precedes this type-only stage.
    let excess = setup::try_model_with_maximum("ExcessTypeMaximum", 10_001).unwrap_err();
    assert_eq!(
        excess.code(),
        quire_spec_language::Code::UnrepresentableConstraint
    );
    assert!(!excess.is_incomplete());
    let quire_spec_language::model_source::ModelSourceCause::Admission(cause) = &excess.cause
    else {
        panic!("native admission must own the sequence refusal: {excess}")
    };
    assert_eq!(
        cause.code,
        quire_spec_language::Code::UnrepresentableConstraint
    );
    assert_eq!(cause.phase, quire_spec_language::Phase::Link);
    assert_eq!(cause.source.identity, "model:ExcessTypeMaximum");
    assert_eq!(cause.source.revision, "authored");
    assert_eq!(cause.path, "ExcessTypeMaximum.json");
    assert_eq!(&cause.source, excess.source().source().identity());
    assert_eq!(
        cause.span,
        excess
            .source()
            .source()
            .locate(quire_spec_language::Span { start: 0, end: 0 })
            .unwrap()
    );

    for maximum in [1, 6, 10_000] {
        let model = setup::model_with_maximum("Bounded", maximum);
        let sources = [setup::source("bounded", &model, "predicate Fold using S (input: M::Node): Boolean { size<M::Wide>(input.amounts) >= 0 and sum<M::Total>(item in input.amounts: item) >= 0 }")];
        let formal = setup::formal_sources(&sources);
        setup::with_binding(&sources, &[&model], BindingLimits::default(), |binding| {
            let report = composed::admit_types(binding, &formal, TypeLimits::default());
            let declaration = id(binding, "Fold");
            assert_eq!(
                report.disposition(declaration),
                Some(TypeDisposition::Typed)
            );
            let typed = report.declaration(declaration).unwrap();
            assert!(typed.nodes().iter().any(|node| matches!(node.ty, Some(NativeType::Sequence { maximum: actual, .. }) if actual == maximum)));
            // Six or more Amounts may exceed Total, which this type-only stage
            // records as a prefix obligation rather than a successful proof.
            assert!(typed
                .obligations()
                .iter()
                .any(|obligation| obligation.kind == ObligationKind::SumPrefixes));
        });
    }
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-4", "FR-040-AC-8")]
fn normalization_requires_exact_formal_source_without_erasing_inferred_types() {
    inspect(
        "predicate Fraction using S (input: M::Q): Boolean { input = rational(2,4) }",
        |binding, formal| {
            for offered in [Vec::new(), vec![formal[0].clone(), formal[0].clone()]] {
                let report = composed::admit_types(binding, &offered, TypeLimits::default());
                refused(
                    &report,
                    "Fraction",
                    CauseKind::UnsupportedPrerequisite(Prerequisite::FormalSource),
                );
                let typed = report.declaration(id(binding, "Fraction")).unwrap();
                let source = binding.namespace().unit(typed.unit()).unwrap().source();
                let literal = typed
                    .nodes()
                    .iter()
                    .find(|node| source.slice(node.span) == Some("rational(2,4)"))
                    .unwrap();
                assert_eq!(scalar_name(literal.ty.as_ref().unwrap()), "Q");
                assert!(literal.normalized_rational.is_none());
            }
            let source = formal[0].source();
            let changed = quire_spec_language::Source::read(
                source.identity().clone(),
                "changed.native",
                format!("{}\n", source.text()).as_bytes(),
                1_048_576,
            )
            .unwrap();
            let changed = setup::formal_sources(&[changed]);
            let report = composed::admit_types(binding, &changed, TypeLimits::default());
            refused(
                &report,
                "Fraction",
                CauseKind::UnsupportedPrerequisite(Prerequisite::FormalSource),
            );
            let retried = composed::admit_types(binding, formal, TypeLimits::default());
            assert_eq!(
                retried.disposition(id(binding, "Fraction")),
                Some(TypeDisposition::Typed)
            );
        },
    );
    inspect("predicate WideRaw using S (input: M::Q): Boolean { input = rational(9223372036854775808,9223372036854775808) }", |binding, formal| {
        let report = composed::admit_types(binding, formal, TypeLimits::default());
        refused(&report, "WideRaw", CauseKind::UnsupportedPrerequisite(Prerequisite::RationalNormalization));
    });
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-3")]
fn binding_and_type_refusals_remain_dependency_local() {
    inspect(
        "predicate Missing using S (): Boolean { neverDeclared }\n\
         predicate BoundDependent using S (): Boolean { Missing() }\n\
         predicate IllTyped using S (valueIn: M::Q): Boolean { valueIn }\n\
         predicate TypeDependent using S (valueIn: M::Q): Boolean { IllTyped(valueIn) }\n\
         predicate Independent using S (): Boolean { true }",
        |binding, formal| {
            let report = composed::admit_types(binding, formal, TypeLimits::default());
            refused(&report, "Missing", CauseKind::UpstreamBinding);
            assert_eq!(
                report.disposition(id(binding, "BoundDependent")),
                Some(TypeDisposition::Refused)
            );
            refused(&report, "IllTyped", CauseKind::TypeMismatch);
            refused(
                &report,
                "TypeDependent",
                CauseKind::Dependency {
                    target: id(binding, "IllTyped"),
                },
            );
            assert_eq!(
                report.disposition(id(binding, "Independent")),
                Some(TypeDisposition::Typed)
            );
            assert!(std::ptr::eq(report.binding(), binding));
        },
    );
    let model = setup::model("Partial");
    let sources = [setup::source(
        "partial",
        &model,
        "predicate Pending using S (): Boolean { true }",
    )];
    let formal = setup::formal_sources(&sources);
    setup::with_binding(
        &sources,
        &[&model],
        BindingLimits {
            bindings: 0,
            ..BindingLimits::default()
        },
        |binding| {
            assert!(!binding.complete());
            let report = composed::admit_types(binding, &formal, TypeLimits::default());
            assert_eq!(
                report.disposition(id(binding, "Pending")),
                Some(TypeDisposition::Unfinished)
            );
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-9")]
fn exhaustion_preserves_earlier_types_but_never_admits_an_unfinished_forward_dependency() {
    inspect(
        "predicate Caller using S (input: M::Q): Boolean { Fraction(input) }\n\
         predicate Independent using S (): Boolean { true }\n\
         predicate Fraction using S (input: M::Q): Boolean { input = rational(2,4) }",
        |binding, formal| {
            let original: Vec<_> = binding
                .namespace()
                .units()
                .iter()
                .map(|unit| unit.source().digest())
                .collect();
            // One bounded IR rational normalization reserves exactly 128 steps,
            // independent of gcd input values. The Boolean/call nodes need none.
            let exact = TypeLimits {
                normalization: 128,
                ..TypeLimits::default()
            };
            for available in [0, 127] {
                let failed = composed::admit_types(
                    binding,
                    formal,
                    TypeLimits {
                        normalization: available,
                        ..exact
                    },
                );
                let exhausted = failed.exhaustion().unwrap();
                assert_eq!(exhausted.dimension, Dimension::Normalization);
                assert_eq!(
                    (exhausted.limit, exhausted.prior, exhausted.requested),
                    (available, 0, 128)
                );
                assert_eq!(exhausted.site.declaration, id(binding, "Fraction"));
                let unit = binding.namespace().unit(exhausted.site.unit).unwrap();
                assert_eq!(
                    unit.source().slice(exhausted.site.span),
                    Some("rational(2,4)")
                );
                assert_eq!(
                    failed.disposition(id(binding, "Fraction")),
                    Some(TypeDisposition::Unfinished)
                );
                assert_eq!(
                    failed.disposition(id(binding, "Caller")),
                    Some(TypeDisposition::Unfinished)
                );
                assert_eq!(
                    failed.disposition(id(binding, "Independent")),
                    Some(TypeDisposition::Typed)
                );
                let previous_usage = failed.usage();
                let retried = composed::admit_types(binding, formal, exact);
                assert!(retried.exhaustion().is_none());
                assert_eq!(retried.usage().normalization, 128);
                for name in ["Caller", "Independent", "Fraction"] {
                    assert_eq!(
                        retried.disposition(id(binding, name)),
                        Some(TypeDisposition::Typed)
                    );
                }
                assert_eq!(failed.usage(), previous_usage);
                assert_eq!(
                    failed.disposition(id(binding, "Caller")),
                    Some(TypeDisposition::Unfinished)
                );
                assert_eq!(
                    binding
                        .namespace()
                        .units()
                        .iter()
                        .map(|unit| unit.source().digest())
                        .collect::<Vec<_>>(),
                    original
                );
            }
            for (dimension, limits) in [
                (Dimension::Bytes, TypeLimits { bytes: 0, ..exact }),
                (
                    Dimension::Declarations,
                    TypeLimits {
                        declarations: 0,
                        ..exact
                    },
                ),
                (
                    Dimension::Expressions,
                    TypeLimits {
                        expressions: 0,
                        ..exact
                    },
                ),
                (
                    Dimension::Constraints,
                    TypeLimits {
                        constraints: 0,
                        ..exact
                    },
                ),
                (Dimension::Edges, TypeLimits { edges: 0, ..exact }),
                (
                    Dimension::Records,
                    TypeLimits {
                        records: 0,
                        ..exact
                    },
                ),
                (Dimension::Depth, TypeLimits { depth: 0, ..exact }),
            ] {
                let failed = composed::admit_types(binding, formal, limits);
                assert_eq!(failed.exhaustion().unwrap().dimension, dimension);
                assert_eq!(failed.limits(), limits);
                assert_eq!(
                    failed.disposition(id(binding, "Caller")),
                    Some(TypeDisposition::Unfinished)
                );
            }
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-9")]
fn public_accounting_clamps_limits_and_refuses_overflow_atomically() {
    inspect(
        "predicate Only using S (): Boolean { true }",
        |binding, _| {
            assert_eq!(composed::work::VERSION, "composed-type-work/1");
            let maximum = TypeLimits {
                bytes: usize::MAX,
                declarations: usize::MAX,
                expressions: usize::MAX,
                constraints: usize::MAX,
                edges: usize::MAX,
                normalization: usize::MAX,
                records: usize::MAX,
                depth: usize::MAX,
            };
            assert_eq!(maximum.bounded(), TypeLimits::default());
            assert_eq!(
                TypeLimits::default(),
                TypeLimits {
                    bytes: 8_388_608,
                    declarations: 10_000,
                    expressions: 100_000,
                    constraints: 1_000_000,
                    edges: 100_000,
                    normalization: 1_000_000,
                    records: 200_000,
                    depth: 64,
                }
            );
            let declaration = id(binding, "Only");
            let site = composed::Site {
                declaration,
                unit: binding.namespace().declaration(declaration).unwrap().unit(),
                expression: None,
                span: binding.namespace().syntax(declaration).unwrap().span,
            };
            for dimension in [
                Dimension::Bytes,
                Dimension::Declarations,
                Dimension::Expressions,
                Dimension::Constraints,
                Dimension::Edges,
                Dimension::Normalization,
                Dimension::Records,
            ] {
                let mut work = Work::new(maximum);
                assert_eq!(work.limits(), TypeLimits::default());
                work.charge(dimension, 1, site).unwrap();
                let previous = work.usage();
                let failure = work.charge(dimension, usize::MAX, site).unwrap_err();
                assert_eq!(failure.dimension, dimension);
                assert_eq!((failure.prior, failure.requested), (1, usize::MAX));
                assert_eq!(failure.site, site);
                assert_eq!(work.usage(), previous);
            }
            let mut depth = Work::new(maximum);
            depth.charge(Dimension::Depth, 64, site).unwrap();
            depth.charge(Dimension::Depth, 1, site).unwrap();
            assert_eq!(depth.usage().max_depth, 64);
            let failure = depth.charge(Dimension::Depth, 65, site).unwrap_err();
            assert_eq!(
                (failure.limit, failure.prior, failure.requested),
                (64, 64, 65)
            );
            assert_eq!(depth.usage().max_depth, 64);
        },
    );
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-9")]
fn partially_created_upstream_records_leave_forward_targets_unfinished() {
    let model = setup::model("Partial");
    let sources = [setup::source(
        "partial",
        &model,
        "predicate Caller using S (): Boolean { Later() }\n\
         predicate Independent using S (): Boolean { true }\n\
         predicate Later using S (): Boolean { true }",
    )];
    let formal = setup::formal_sources(&sources);
    setup::with_binding(
        &sources,
        &[&model],
        BindingLimits {
            bindings: 1,
            ..BindingLimits::default()
        },
        |binding| {
            assert_eq!(binding.namespace().declarations().len(), 3);
            assert_eq!(binding.declarations().len(), 1);
            assert!(binding.exhaustion().is_some());
            let report = composed::admit_types(binding, &formal, TypeLimits::default());
            assert!(report.exhaustion().is_none());
            assert_eq!(report.declarations().len(), 1);
            for name in ["Caller", "Independent", "Later"] {
                assert_eq!(
                    report.disposition(id(binding, name)),
                    Some(TypeDisposition::Unfinished)
                );
            }
            assert!(report.declarations()[0].nodes().is_empty());
            assert!(report.declarations()[0].causes().is_empty());
        },
    );
}
