// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-119: real three-family type admission preserves cross-unit dependencies.
//! This is a static type pipeline; proof, runtime and artifact delivery remain open.

#[path = "support/composed_types/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{
    self, CauseKind, ObligationKind, TypeDisposition, TypeLimits,
};
use quire_spec_language::checking::NativeType;
use quire_spec_language::linking::composed::binding::Disposition as BindingDisposition;
use quire_spec_language::linking::composed::binding_work::Limits as BindingLimits;
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::Source;

fn ecosystem(model: &NativeModel, predicate: &str) -> Vec<Source> {
    vec![
        setup::source("state", model, &format!(
            "predicate Positive using S (amount: M::Q): Boolean {{ {predicate} }}\n\
             invariant Healthy using S on M::Node at current {{ Positive(self.fraction) }}\n\
             predicate Independent using S (): Boolean {{ true }}"
        )),
        setup::source("temporal", model,
            "temporal Due using T over (view: M::Node) clock \"orders\" on each (started: M::Node) when (Positive(started.fraction)) {\n\
               capture saved: M::Q = started.fraction;\n\
               eventually[0,1] holds(Positive(view.fraction) and saved >= rational(0,1))\n\
             }"
        ),
        setup::source("protocol", model,
            "protocol Flow using P over (view: M::Node) on origin {\n\
               role Service on M::Node; requires temporal Due;\n\
               run sequence Main {\n\
                 event Happened by Service as (happened: M::Node) { Positive(happened.fraction) };\n\
                 check HealthyNow using S { Positive(view.fraction) };\n\
               }\n\
               finish Closed as (closed: M::Node) { Positive(closed.fraction) };\n\
             }"
        ),
    ]
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-3", "FR-040-AC-6", "FR-040-AC-7")]
fn three_family_types_retain_exact_callee_profiles_and_authored_source_owners() {
    let model = setup::model("Ecosystem");
    let sources = ecosystem(&model, "amount >= rational(0,1)");
    let formal = setup::formal_sources(&sources);
    setup::with_binding(&sources, &[&model], BindingLimits::default(), |binding| {
        assert!(binding.complete(), "{:?}", binding.exhaustion());
        let namespace = binding.namespace();
        assert_eq!(namespace.units().len(), 3);
        assert_eq!(namespace.declarations().len(), 5);
        let positive = namespace.lookup("Positive")[0];
        let definitions = binding.definitions().unwrap();
        let report = composed::admit_types(binding, &formal, TypeLimits::default());
        assert!(report.exhaustion().is_none(), "{:?}", report.exhaustion());
        for (name, owner, profile, calls) in [
            ("Positive", "unit:state", R::StateQueries, 0),
            ("Healthy", "unit:state", R::StateQueries, 1),
            ("Independent", "unit:state", R::StateQueries, 0),
            ("Due", "unit:temporal", R::EventPosition, 2),
            ("Flow", "unit:protocol", R::Protocol, 3),
        ] {
            let id = namespace.lookup(name)[0];
            assert_eq!(
                binding.disposition(id),
                Some(BindingDisposition::NamesResolved)
            );
            let typed = report.declaration(id).unwrap();
            assert_eq!(
                typed.disposition(),
                TypeDisposition::Typed,
                "{name}: {:?}",
                typed.causes()
            );
            let unit = namespace.unit(typed.unit()).unwrap();
            assert_eq!(unit.source().identity().identity, owner);
            assert_eq!(
                definitions.declarations[id.index()].uses[0].closure[0],
                profile
            );
            let call_obligations: Vec<_> = typed
                .obligations()
                .iter()
                .filter(|obligation| {
                    obligation.kind == ObligationKind::PredicateTotal { target: positive }
                })
                .collect();
            assert_eq!(call_obligations.len(), calls, "{name}");
            for obligation in call_obligations {
                assert_eq!(obligation.site.declaration, id);
                assert_eq!(obligation.site.unit, typed.unit());
                let expression = obligation.site.expression.unwrap();
                let node = typed.node(expression).unwrap();
                assert_eq!(node.span, unit.expression(expression).unwrap().span);
                assert!(matches!(node.ty, Some(NativeType::Boolean)));
                assert!(unit
                    .source()
                    .slice(obligation.site.span)
                    .unwrap()
                    .starts_with("Positive("));
                let selected = definitions.declarations[id.index()].uses[node.profile].closure[0];
                let expected = if unit.source().slice(node.span) == Some("Positive(view.fraction)")
                    && name == "Flow"
                {
                    R::StateQueries
                } else {
                    profile
                };
                assert_eq!(selected, expected);
            }
        }
        // Normalized literals remain in each authored unit, not in a model's
        // source or a synthetic callee copy. No concrete runtime inputs exist.
        for name in ["Positive", "Due"] {
            let typed = report.declaration(namespace.lookup(name)[0]).unwrap();
            let unit = namespace.unit(typed.unit()).unwrap();
            let literals: Vec<_> = typed
                .nodes()
                .iter()
                .filter(|node| node.normalized_rational.is_some())
                .collect();
            assert_eq!(literals.len(), 1);
            assert_eq!(literals[0].normalized_rational, Some((0, 1)));
            assert_eq!(unit.source().slice(literals[0].span), Some("rational(0,1)"));
        }
        let temporal = report.declaration(namespace.lookup("Due")[0]).unwrap();
        assert!(temporal
            .obligations()
            .iter()
            .any(|obligation| obligation.kind == ObligationKind::CaptureInput));
    });
}

#[test]
#[trace("TC-119", "FR-040-AC-1", "FR-040-AC-2", "FR-040-AC-3")]
fn one_ill_typed_callee_refuses_dependent_families_without_erasing_independent_types() {
    let model = setup::model("Ecosystem");
    let sources = ecosystem(&model, "amount");
    let formal = setup::formal_sources(&sources);
    setup::with_binding(&sources, &[&model], BindingLimits::default(), |binding| {
        let namespace = binding.namespace();
        let report = composed::admit_types(binding, &formal, TypeLimits::default());
        assert!(report.exhaustion().is_none());
        let positive = namespace.lookup("Positive")[0];
        assert!(report
            .declaration(positive)
            .unwrap()
            .causes()
            .iter()
            .any(|cause| cause.kind == CauseKind::TypeMismatch));
        for name in ["Positive", "Healthy", "Due", "Flow"] {
            let id = namespace.lookup(name)[0];
            assert_eq!(
                binding.disposition(id),
                Some(BindingDisposition::NamesResolved)
            );
            assert_eq!(
                report.disposition(id),
                Some(TypeDisposition::Refused),
                "{name}"
            );
            if id != positive {
                assert!(
                    report
                        .declaration(id)
                        .unwrap()
                        .causes()
                        .iter()
                        .any(|cause| { cause.kind == CauseKind::Dependency { target: positive } }),
                    "{name}: {:?}",
                    report.declaration(id).unwrap().causes()
                );
            }
        }
        let independent = report
            .declaration(namespace.lookup("Independent")[0])
            .unwrap();
        assert_eq!(independent.disposition(), TypeDisposition::Typed);
        assert!(independent.causes().is_empty());
        assert!(independent.obligations().is_empty());
        assert_eq!(independent.nodes().len(), 1);
        assert!(matches!(
            independent.nodes()[0].ty,
            Some(NativeType::Boolean)
        ));
    });
}
