// SPDX-License-Identifier: AGPL-3.0-only
//! TC-114: one real multi-unit static binding path, with dependent refusals.
//! Native rule-model production is exercised; this does not claim D's IT-009.

#[path = "support/native_rule_model.rs"]
mod native_rule_model;

use ix_trace_rs::trace;
use quire_spec_language::linking::composed::binding::{self, Disposition, Refusal};
use quire_spec_language::linking::composed::binding_work::Limits as BindingLimits;
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::linking::composed::definitions::{Artifact, Inventory, RuleInput};
use quire_spec_language::linking::composed::models::ModelInput;
use quire_spec_language::linking::composed::{
    admit_namespace, ExpectedSource, SourceInventory, WorkLimits,
};
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::{ByteDigest, Limits, Source, SourceIdentity};
use std::collections::BTreeMap;

fn artifacts() -> (Vec<Artifact<'static>>, Vec<RuleInput<'static>>) {
    let definitions = R::all()
        .iter()
        .map(|definition| Artifact {
            selection: definition.selection(),
            bytes: definition.bytes(),
        })
        .collect();
    let rules = R::all()
        .iter()
        .flat_map(|definition| definition.rules())
        .map(|rule| {
            (
                rule.path,
                RuleInput {
                    path: rule.path,
                    digest: ByteDigest::of(rule.bytes),
                    bytes: rule.bytes,
                },
            )
        })
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect();
    (definitions, rules)
}

fn source(id: &str, model: &NativeModel, profiles: &[(&str, R)], body: &str) -> Source {
    let mut text = "language \"ix:native\" edition \"1-draft\";\n".to_owned();
    for (alias, profile) in profiles {
        let selection = profile.selection();
        text.push_str(&format!(
            "profile {alias} = \"{}\" version \"{}\" digest \"{}\";\n",
            selection.identity, selection.revision, selection.digest
        ));
    }
    text.push_str(&format!(
        "model M = \"{}\" version \"{}\" digest \"{}\";\n",
        model.environment().owner().package().as_str(),
        model.environment().owner().revision().get(),
        model.digest()
    ));
    text.push_str(body);
    Source::read(
        SourceIdentity {
            identity: id.into(),
            revision: "test:source".into(),
        },
        format!("{id}.native"),
        text.as_bytes(),
        Limits::default().source_bytes,
    )
    .unwrap()
}

fn inventory(sources: &[Source]) -> SourceInventory {
    SourceInventory {
        language: "ix:native".into(),
        edition: "1-draft".into(),
        units: sources
            .iter()
            .map(|source| ExpectedSource {
                authority: source.identity().identity.clone(),
                identity: source.identity().clone(),
                digest: source.digest(),
            })
            .collect(),
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-4")]
fn one_binding_path_resolves_three_families_and_preserves_owned_anchors() {
    let model = native_rule_model::parts().model();
    let sources = [
        source("state", &model, &[("S", R::StateGraph)], "predicate Positive using S (amount: M::Version): Boolean { amount >= 0 }\ninvariant Healthy using S on M::Node at current { Positive(self.n) }"),
        source("temporal", &model, &[("T", R::EventPosition)], "temporal Due using T over (view: M::Node) clock \"orders\" on each (started: M::Node) when (Positive(started.n)) { capture saved: M::Version = started.n; eventually[0,1] holds(Positive(view.n) and saved >= 0) }"),
        source("protocol", &model, &[("S", R::StateGraph), ("P", R::Protocol)], "protocol Flow using P over (view: M::Node) on origin { role Service on M::Node; requires temporal Due; run sequence Main { event Happened by Service as (happened: M::Node) { Positive(happened.n) }; check HealthyNow using S { Positive(view.n) }; } finish Closed as (closed: M::Node) { Positive(closed.n) }; }"),
    ];
    let sources_selected = inventory(&sources);
    let namespace_report = admit_namespace(
        &sources_selected,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(
        namespace_report.issues().is_empty(),
        "{:?}",
        namespace_report.issues()
    );
    let namespace = namespace_report.namespace().unwrap();
    let (definitions, rules) = artifacts();
    let selected = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let models = [ModelInput::Native(&model)];
    let report = binding::bind(namespace, &selected, &models, BindingLimits::default());
    assert!(report.complete(), "{report:#?}");
    for entry in report.declarations() {
        assert_eq!(
            report.disposition(entry.declaration()),
            Some(Disposition::NamesResolved),
            "{report:#?}"
        );
    }
    assert_eq!(report.declarations().len(), 4);
    let due = namespace.lookup("Due")[0];
    let flow = namespace.lookup("Flow")[0];
    let scopes = report.scopes().unwrap();
    let due_scope = scopes.declaration(due).unwrap();
    let flow_scope = scopes.declaration(flow).unwrap();
    let due_view = due_scope
        .binders
        .iter()
        .find(|binder| binder.name.as_deref() == Some("view"))
        .unwrap();
    let flow_view = flow_scope
        .binders
        .iter()
        .find(|binder| binder.name.as_deref() == Some("view"))
        .unwrap();
    assert_ne!(due_scope.declaration, flow_scope.declaration);
    assert_ne!(due_scope.unit, flow_scope.unit);
    assert_ne!(due_view.anchor, flow_view.anchor);
    let captured = due_scope
        .binders
        .iter()
        .find(|binder| binder.name.as_deref() == Some("saved"))
        .unwrap();
    assert_eq!(
        captured.kind,
        quire_spec_language::linking::composed::scopes::BinderKind::Capture
    );
    assert!(report
        .exports()
        .iter()
        .all(|entry| entry.complete && entry.refusals.is_empty()));
    assert!(report.exports().iter().flat_map(|entry| &entry.occurrences).any(|occurrence| matches!(&occurrence.target,
        quire_spec_language::linking::composed::models::ModelTarget::Type(ty)
        if ty.location().identity.key == quire_spec_language::linking::DeclarationKey::Scalar(native_rule_model::symbol("Version")))));
    assert!(std::ptr::eq(report.namespace(), namespace));
    assert_eq!(model.source().source().text(), native_rule_model::FIXTURE);
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn failed_definition_invalidates_caller_while_independent_profile_survives() {
    let model = native_rule_model::parts().model();
    let sources = [source("shared", &model, &[("Q", R::StateQueries), ("S", R::StateCore)], "predicate MissingProfile using Q (): Boolean { true }\ninvariant Dependent using S on M::Node at current { MissingProfile() }\ninvariant Independent using S on M::Node at current { true }")];
    let sources_selected = inventory(&sources);
    let namespace_report = admit_namespace(
        &sources_selected,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(namespace_report.issues().is_empty());
    let namespace = namespace_report.namespace().unwrap();
    let (mut definitions, rules) = artifacts();
    definitions.retain(|definition| definition.selection.identity != R::StateQueries.identity());
    let selected = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let models = [ModelInput::Native(&model)];
    let report = binding::bind(namespace, &selected, &models, BindingLimits::default());
    assert!(report.complete());
    let failed = namespace.lookup("MissingProfile")[0];
    let dependent = namespace.lookup("Dependent")[0];
    let independent = namespace.lookup("Independent")[0];
    assert_eq!(report.disposition(failed), Some(Disposition::Refused));
    assert_eq!(report.disposition(dependent), Some(Disposition::Refused));
    assert_eq!(
        report.disposition(independent),
        Some(Disposition::NamesResolved)
    );
    assert!(report.declarations()[dependent.index()]
        .refusals()
        .iter()
        .any(|cause| matches!(cause, Refusal::Dependency { target, .. } if *target == failed)));
}

#[test]
#[trace("TC-114", "FR-036-AC-2", "FR-036-AC-7")]
fn illegal_activation_scope_and_exhaustion_never_become_empty_success() {
    let model = native_rule_model::parts().model();
    let sources = [source("scopes", &model, &[("T", R::EventPosition)], "temporal Due using T over (view: M::Node) clock \"clock\" on each (started: M::Node) { eventually[0,1] holds(started.n >= 0) }")];
    let sources_selected = inventory(&sources);
    let namespace_report = admit_namespace(
        &sources_selected,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let namespace = namespace_report.namespace().unwrap();
    let (definitions, rules) = artifacts();
    let selected = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let models = [ModelInput::Native(&model)];
    let due = namespace.lookup("Due")[0];
    let unfinished = binding::bind(
        namespace,
        &selected,
        &models,
        BindingLimits {
            models: 0,
            ..BindingLimits::default()
        },
    );
    assert!(!unfinished.complete());
    assert!(unfinished.exhaustion().is_some());
    assert_eq!(unfinished.disposition(due), Some(Disposition::Unfinished));
    let completed = binding::bind(namespace, &selected, &models, BindingLimits::default());
    assert!(completed.complete());
    assert_eq!(completed.disposition(due), Some(Disposition::Refused));
    assert!(completed.scopes().unwrap().declaration(due).unwrap().issues.iter().any(|issue| matches!(issue, quire_spec_language::linking::composed::scopes::ScopeIssue::OutOfScope { name, .. } if name == "started")));
    assert_eq!(unfinished.disposition(due), Some(Disposition::Unfinished));
}

#[test]
#[trace("TC-114", "FR-036-AC-2")]
fn unused_cross_kind_alias_conflicts_refuse_their_unit_and_dependents() {
    let model = native_rule_model::parts().model();
    let sources = [
        source("conflict", &model, &[("M", R::StateQueries), ("Q", R::StateQueries)], "predicate Root using Q (): Boolean { true }"),
        source("caller", &model, &[("Q", R::StateQueries)], "predicate Caller using Q (): Boolean { Root() } predicate Independent using Q (): Boolean { true }"),
    ];
    let sources_selected = inventory(&sources);
    let namespace_report = admit_namespace(
        &sources_selected,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    let namespace = namespace_report.namespace().unwrap();
    let (definitions, rules) = artifacts();
    let selected = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let models = [ModelInput::Native(&model)];
    let report = binding::bind(namespace, &selected, &models, BindingLimits::default());
    assert!(report.complete());
    assert_eq!(report.alias_conflicts().len(), 1);
    let conflict = &report.alias_conflicts()[0];
    let unit = namespace.unit(conflict.unit).unwrap();
    assert_eq!(unit.source().slice(conflict.first), Some("M"));
    assert_eq!(unit.source().slice(conflict.repeated), Some("M"));
    assert_ne!(conflict.first, conflict.repeated);
    assert_eq!(
        report.disposition(namespace.lookup("Root")[0]),
        Some(Disposition::Refused)
    );
    assert_eq!(
        report.disposition(namespace.lookup("Caller")[0]),
        Some(Disposition::Refused)
    );
    assert_eq!(
        report.disposition(namespace.lookup("Independent")[0]),
        Some(Disposition::NamesResolved)
    );
}
