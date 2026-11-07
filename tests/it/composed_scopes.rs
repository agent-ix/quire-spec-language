// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-114/115: lexical and structural stages through their public Rust boundary.
//! Unresolved authored model types remain syntax in model-independent scope cases.
use crate::support::native_rule_model;

use ix_trace_rs::trace;
use qsl_foundation::{Source, SourceIdentity};
use quire_spec_language::linking::composed::binding_work::{
    Dimension, Limits as BindingLimits, Work,
};
use quire_spec_language::linking::composed::models::{bind_models, ModelInput};
use quire_spec_language::linking::composed::scopes::{
    self, Anchor, BinderKind, BinderType, DeclarationScope, ScopeDisposition, ScopeIssue,
    ScopeReport,
};
use quire_spec_language::linking::composed::{
    admit_namespace, DeclarationId, ExpectedSource, SourceInventory, SyntaxNamespace, WorkLimits,
};
use quire_spec_language::Limits;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile S = \"test:unresolved-definition\";\n";
fn program(body: &str) -> String {
    format!("{HEADER}{body}")
}
fn with_namespace(texts: &[String], test: impl FnOnce(&SyntaxNamespace)) {
    let sources: Vec<_> = texts
        .iter()
        .enumerate()
        .map(|(index, text)| {
            Source::read(
                SourceIdentity {
                    authority: "test".into(),
                    identity: format!("scope:{index}"),
                },
                format!("scope-{index}.native"),
                text.as_bytes(),
                Limits::default().source_bytes,
            )
            .unwrap()
        })
        .collect();
    let inventory = SourceInventory {
        language: "ix:native".into(),
        edition: "1-draft".into(),
        units: sources
            .iter()
            .map(|source| ExpectedSource {
                authority: source.identity().identity.clone(),
                identity: source.identity().clone(),
            })
            .collect(),
    };
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(report.issues().is_empty(), "{:?}", report.issues());
    assert!(report.exhaustion().is_none());
    let namespace = report.namespace().unwrap();
    assert!(namespace.dependencies_complete());
    test(namespace);
}
fn id(namespace: &SyntaxNamespace, name: &str) -> DeclarationId {
    let [id] = namespace.lookup(name) else {
        panic!("one {name}")
    };
    *id
}
fn scope<'a>(
    namespace: &SyntaxNamespace,
    report: &'a ScopeReport,
    name: &str,
) -> &'a DeclarationScope {
    report.declaration(id(namespace, name)).unwrap()
}
fn resolved(namespace: &SyntaxNamespace, report: &ScopeReport, name: &str) {
    let declaration = scope(namespace, report, name);
    assert_eq!(
        declaration.disposition(),
        ScopeDisposition::Resolved,
        "{name}: {:?}",
        declaration.issues
    );
}
fn inspect(body: &str, test: impl FnOnce(&SyntaxNamespace, &ScopeReport)) {
    with_namespace(&[program(body)], |namespace| {
        let models = bind_models(namespace, &[], &mut Work::new(BindingLimits::default()));
        let report = scopes::resolve(namespace, &models, &mut Work::new(BindingLimits::default()));
        assert!(report.exhaustion.is_none(), "{:?}", report.exhaustion);
        test(namespace, &report);
    });
}
fn unavailable(scope: &DeclarationScope, name: &str) {
    assert_eq!(scope.disposition(), ScopeDisposition::Refused);
    assert!(
        scope.issues.iter().any(
            |issue| matches!(issue, ScopeIssue::OutOfScope { name: found, .. } if found == name)
        ),
        "{:?}",
        scope.issues
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-2", "FR-036-AC-4")]
fn lexical_bodies_keep_parameters_let_and_query_domain_scopes() {
    inspect("predicate Good using S (input: Boolean): Boolean { let saved = input in forall(item in input: saved and item) }\n\
        predicate Initializer using S (): Boolean { let later = later in later }\n\
        predicate QueryDomain using S (): Boolean { forall(element in element: element) }\n\
        predicate Shadow using S (valueIn: Boolean): Boolean { let valueIn = true in valueIn }\n\
        predicate Alias using S (S: Boolean): Boolean { S }\n\
        predicate Ambient using S (): Boolean { pre(self) = result }", |namespace, report| {
        resolved(namespace, report, "Good");
        let good = scope(namespace, report, "Good");
        assert_eq!(good.binders.iter().map(|binder| binder.kind).collect::<Vec<_>>(), [BinderKind::Parameter, BinderKind::Let, BinderKind::Query]);
        for occurrence in &good.values {
            assert_eq!(occurrence.unit, good.unit);
            let binder = &good.binders[occurrence.target.index()];
            assert_eq!(namespace.unit(good.unit).unwrap().source().slice(occurrence.span), binder.name.as_deref());
        }
        unavailable(scope(namespace, report, "Initializer"), "later");
        unavailable(scope(namespace, report, "QueryDomain"), "element");
        assert!(scope(namespace, report, "Shadow").issues.iter().any(|issue| matches!(issue, ScopeIssue::DuplicateBinder { .. })));
        assert!(scope(namespace, report, "Alias").issues.iter().any(|issue| matches!(issue, ScopeIssue::ReservedBinder { .. })));
        assert!(scope(namespace, report, "Ambient").issues.iter().any(|issue| matches!(issue, ScopeIssue::AmbientUnavailable { .. })));
    });
}

const TEMPORAL: &str = "temporal Timing using S over (input: Boolean) clock \"clock\" on each (trigger: Boolean) when (trigger and input) {\n\
    capture firstValue: Boolean = trigger;\n\
    capture secondValue: Boolean = firstValue and input;\n\
    eventually [0,2] holds(secondValue and input)\n}";

#[test]
#[trace("TC-114", "FR-036-AC-2", "FR-036-AC-4")]
fn activation_guard_sequential_captures_and_formula_have_distinct_environments() {
    inspect(TEMPORAL, |namespace, report| {
        resolved(namespace, report, "Timing");
        let timing = scope(namespace, report, "Timing");
        let capture = timing
            .binders
            .iter()
            .position(|binder| binder.name.as_deref() == Some("secondValue"))
            .unwrap();
        assert_eq!(timing.binders[capture].anchor, Anchor::Activation);
        let occurrences: Vec<_> = timing
            .values
            .iter()
            .filter(|occurrence| occurrence.target.index() == capture)
            .collect();
        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0].evaluation_anchor, Anchor::TemporalInstant);
    });
    for (from, to, name) in [
        (
            "holds(secondValue and input)",
            "holds(trigger and input)",
            "trigger",
        ),
        (
            "when (trigger and input)",
            "when (firstValue and input)",
            "firstValue",
        ),
        (
            "firstValue: Boolean = trigger",
            "firstValue: Boolean = secondValue",
            "secondValue",
        ),
    ] {
        assert!(TEMPORAL.contains(from));
        inspect(&TEMPORAL.replacen(from, to, 1), |namespace, report| {
            unavailable(scope(namespace, report, "Timing"), name)
        });
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-4", "FR-036-AC-7")]
fn equal_binder_names_in_distinct_units_keep_owners_and_exact_scope_limits_retry() {
    with_namespace(
        &[
            program("predicate First using S (input: Boolean): Boolean { input }"),
            program("predicate Second using S (input: Boolean): Boolean { input }"),
        ],
        |namespace| {
            let models = bind_models(namespace, &[], &mut Work::new(BindingLimits::default()));
            let report =
                scopes::resolve(namespace, &models, &mut Work::new(BindingLimits::default()));
            let first = scope(namespace, &report, "First");
            let second = scope(namespace, &report, "Second");
            assert_ne!(first.unit, second.unit);
            assert_ne!(first.declaration, second.declaration);
            assert_eq!(first.values[0].unit, first.unit);
            assert_eq!(second.values[0].unit, second.unit);
            assert_eq!(first.binders[0].name, second.binders[0].name);
        },
    );
    with_namespace(
        &[program(
            "predicate Only using S (input: Boolean): Boolean { input }",
        )],
        |namespace| {
            let models = bind_models(namespace, &[], &mut Work::new(BindingLimits::default()));
            // One declaration + binder + frame + occurrence; declaration, binder-name,
            // profile-alias, value-node and name inspections; one lexical frame edge.
            let exact = BindingLimits {
                bindings: 4,
                references: 5,
                edges: 1,
                ..BindingLimits::default()
            };
            for (limits, dimension) in [
                (
                    BindingLimits {
                        bindings: 0,
                        ..exact
                    },
                    Dimension::Bindings,
                ),
                (
                    BindingLimits {
                        bindings: 3,
                        ..exact
                    },
                    Dimension::Bindings,
                ),
                (
                    BindingLimits {
                        references: 4,
                        ..exact
                    },
                    Dimension::References,
                ),
                (BindingLimits { edges: 0, ..exact }, Dimension::Edges),
            ] {
                let mut work = Work::new(limits);
                let failed = scopes::resolve(namespace, &models, &mut work);
                assert_eq!(failed.exhaustion.unwrap().dimension, dimension);
                assert_eq!(
                    failed.disposition(id(namespace, "Only")),
                    ScopeDisposition::Unfinished
                );
                let prior_usage = work.usage();
                let retried = scopes::resolve(namespace, &models, &mut Work::new(exact));
                resolved(namespace, &retried, "Only");
                assert_eq!(work.usage(), prior_usage);
                assert_eq!(
                    failed.disposition(id(namespace, "Only")),
                    ScopeDisposition::Unfinished
                );
            }
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-4")]
fn actual_model_operation_inputs_result_and_pre_self_seed_the_state_scope() {
    let baseline = native_rule_model::parts().model();
    assert!(baseline.roles().operations[0].parameters.is_empty());
    assert_eq!(
        baseline.roles().operations[0].name,
        native_rule_model::symbol("step")
    );
    let mut document: serde_json::Value =
        serde_json::from_str(native_rule_model::fixture()).unwrap();
    document["values"].as_array_mut().unwrap().push(serde_json::json!({"name":"delta", "kind":"input", "type":{"kind":"scalar", "name":"Version"}}));
    document["operations"][0]["parameters"] = serde_json::json!(["delta"]);
    let model = native_rule_model::from_text(
        &serde_json::to_string(&document).unwrap(),
        "scope-model.json",
    )
    .unwrap()
    .model();
    let source = format!(
        "{HEADER}model M = \"{}\";\n\
        post Changed using S on M::Node::step {{ self.n = pre(self.n) + delta and result }}",
        model.environment().owner().package().as_str()
    );
    with_namespace(&[source], |namespace| {
        let inputs = [ModelInput::Native(&model)];
        let models = bind_models(namespace, &inputs, &mut Work::new(BindingLimits::default()));
        let report = scopes::resolve(namespace, &models, &mut Work::new(BindingLimits::default()));
        resolved(namespace, &report, "Changed");
        let scope = scope(namespace, &report, "Changed");
        let delta = scope
            .binders
            .iter()
            .find(|binder| binder.name.as_deref() == Some("delta"))
            .unwrap();
        assert_eq!(delta.kind, BinderKind::InvocationParameter);
        assert_eq!(delta.anchor, Anchor::InvocationInput);
        assert!(matches!(delta.ty, BinderType::ModelValue(_)));
        assert!(scope
            .values
            .iter()
            .any(
                |occurrence| occurrence.evaluation_anchor == Anchor::InvocationPre
                    && scope.binders[occurrence.target.index()].kind == BinderKind::SelfValue
            ));
        assert!(scope
            .values
            .iter()
            .any(|occurrence| scope.binders[occurrence.target.index()].kind
                == BinderKind::ResultValue));
    });
}

#[test]
#[trace("TC-114", "FR-036-AC-2", "FR-036-AC-4")]
fn an_undeclared_value_retains_its_missing_name_and_original_expression() {
    inspect(
        "predicate Missing using S (): Boolean { neverDeclared }\n\
         predicate Independent using S (input: Boolean): Boolean { input }",
        |namespace, report| {
            let missing = scope(namespace, report, "Missing");
            assert_eq!(missing.disposition(), ScopeDisposition::Refused);
            let [ScopeIssue::MissingValue {
                expression,
                span,
                name,
            }] = missing.issues.as_slice()
            else {
                panic!("one missing value: {:?}", missing.issues)
            };
            assert_eq!(name, "neverDeclared");
            let unit = namespace.unit(missing.unit).unwrap();
            assert_eq!(unit.expression(*expression).unwrap().span, *span);
            assert_eq!(unit.source().slice(*span), Some("neverDeclared"));
            assert!(missing.values.is_empty());
            resolved(namespace, report, "Independent");
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-2")]
fn an_unavailable_model_operation_does_not_invent_missing_input_names() {
    let model = native_rule_model::parts().model();
    let source = format!(
        "{HEADER}model M = \"{}\";\n\
         pre Missing using S on M::Node::absent {{ invocationInput }}\n\
         post Available using S on M::Node::step {{ result }}",
        model.environment().owner().package().as_str()
    );
    with_namespace(&[source], |namespace| {
        let inputs = [ModelInput::Native(&model)];
        let models = bind_models(namespace, &inputs, &mut Work::new(BindingLimits::default()));
        let report = scopes::resolve(namespace, &models, &mut Work::new(BindingLimits::default()));
        assert!(report.exhaustion.is_none());
        let missing = scope(namespace, &report, "Missing");
        assert_eq!(missing.disposition(), ScopeDisposition::Refused);
        let [ScopeIssue::ModelOperationUnavailable { span }] = missing.issues.as_slice() else {
            panic!("one unavailable operation: {:?}", missing.issues)
        };
        assert_eq!(
            namespace.unit(missing.unit).unwrap().source().slice(*span),
            Some("absent")
        );
        assert!(missing.values.is_empty());
        resolved(namespace, &report, "Available");
    });
}
