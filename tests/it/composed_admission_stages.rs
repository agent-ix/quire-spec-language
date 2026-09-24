// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-115: static meaning, every requested capability, historical compatibility.
//! Request dispositions are compiler-side records, not domain-package intake evidence.

// The fixture also supports compiler, graph and operation integration tests.
use crate::support::runtime_setup as setup;

// The frozen vectors also carry canonical bytes used by the FR-020 controls.
use crate::support::package_vector_setup;

use ix_trace_rs::trace;
use qsl_foundation::{ByteDigest, Code, Source, SourceIdentity};
use qsl_semantics::check::Capability;
use quire_spec_language::linking::composed::binding::{self, Disposition as BindingDisposition};
use quire_spec_language::linking::composed::binding_work::Limits as BindingLimits;
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::linking::composed::definitions::{Artifact, Inventory, RuleInput};
use quire_spec_language::linking::composed::models::{ImportRefusal, ModelInput};
use quire_spec_language::linking::composed::requests::{
    self, Aggregate, Assessment, Disposition, Family, InventoryGap, Request,
};
use quire_spec_language::linking::composed::subject::{
    BuildProvenance, ComponentKind, StaticSubject,
};
use quire_spec_language::linking::composed::{
    admit_namespace, ExpectedSource, NamespaceReport, SourceInventory, WorkLimits,
};
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::package::{
    NativePackage, NativePackageRef, PackageLimits, PackageReadLimits, PackageStage, PackageSupport,
};
use quire_spec_language::runtime::{
    ArtifactLimits, InputReadCause, InputReadStage, Snapshot, SnapshotRef,
};
use quire_spec_language::Limits;
use serde_json::Value;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Composed fixture, following the TC-114 controls in tests/composed_binding.rs.
// ---------------------------------------------------------------------------

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
                    digest: ByteDigest::of(rule.path.as_bytes()),
                    bytes: rule.path.as_bytes(),
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
            authority: "test".into(),
            identity: id.into(),
            revision_namespace: "test".into(),
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

const STATE: &str = "predicate Positive using S (amount: M::Version): Boolean { amount >= 0 }\n\
     invariant Healthy using S on M::Node at current { Positive(self.n) }";
const TEMPORAL: &str = "temporal Due using T over (view: M::Node) clock \"orders\" \
     on each (started: M::Node) when (Positive(started.n)) \
     { capture saved: M::Version = started.n; \
       eventually[0,1] holds(Positive(view.n) and saved >= 0) }";
const PROTOCOL: &str = "protocol Flow using P over (view: M::Node) on origin \
     { role Service on M::Node; requires temporal Due; \
       run sequence Main { event Happened by Service as (happened: M::Node) \
         { Positive(happened.n) }; check HealthyNow using S { Positive(view.n) }; } \
       finish Closed as (closed: M::Node) { Positive(closed.n) }; }";

/// The three-family inventory of TC-115 step 1, with a selectable state profile.
fn ecosystem(model: &NativeModel, state: R) -> Vec<Source> {
    vec![
        source("state", model, &[("S", state)], STATE),
        source("temporal", model, &[("T", R::EventPosition)], TEMPORAL),
        source(
            "protocol",
            model,
            &[("S", R::StateGraph), ("P", R::Protocol)],
            PROTOCOL,
        ),
    ]
}

/// Link an inventory and hand both stage reports to the control.
///
/// This is the TC-114 harness with the namespace report retained, so that build
/// provenance and static components can be read from the same invocation.
fn with_reports<T>(
    sources: &[Source],
    models: &[ModelInput<'_>],
    package_limits: WorkLimits,
    parser_limits: Limits,
    binding_limits: BindingLimits,
    test: impl FnOnce(&SourceInventory, &NamespaceReport<'_>, &binding::Report<'_>) -> T,
) -> T {
    let selected = inventory(sources);
    let admitted = admit_namespace(&selected, sources, package_limits, parser_limits);
    assert!(admitted.issues().is_empty(), "{:?}", admitted.issues());
    let namespace = admitted.namespace().expect("closed source namespace");
    let (definitions, rules) = artifacts();
    let selected_definitions = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let report = binding::bind(namespace, &selected_definitions, models, binding_limits);
    test(&selected, &admitted, &report)
}

/// Link the default three-family subject and retain its static components.
fn subject_of(model: &NativeModel, state: R) -> StaticSubject {
    let sources = ecosystem(model, state);
    let models = [ModelInput::Native(model)];
    with_reports(
        &sources,
        &models,
        WorkLimits::default(),
        Limits::default(),
        BindingLimits::default(),
        |inventory, _, report| {
            assert!(report.complete(), "{:?}", report.exhaustion());
            StaticSubject::of(inventory, report).expect("complete binding retains its components")
        },
    )
}

// ---------------------------------------------------------------------------
// FR-036-AC-5: static meaning, assessment inputs and resource configuration.
// ---------------------------------------------------------------------------

/// FR-077: `Assessment` carries no backend field any more -- backend
/// selection and support are computed downstream, by the `#185` registry
/// and `quire-contract-codegen`'s `negotiate_*`, never read here. This test
/// now varies only the assessment inputs `requests::report` still accepts
/// (population, window, trace).
#[test]
#[trace("TC-115", "FR-036-AC-5")]
fn assessment_selections_leave_every_declared_component_unchanged() {
    let model = setup::native_rule_model::parts().model();
    let baseline = subject_of(&model, R::StateGraph);
    assert_eq!(baseline.sources().len(), 3);
    assert_eq!(baseline.language().language, "ix:native");
    assert_eq!(baseline.language().edition, "1-draft");
    assert!(baseline.language().refusal.is_none());
    assert!(!baseline.profiles().is_empty());
    assert_eq!(baseline.models().len(), 3);
    assert!(!baseline.dependencies().is_empty());
    assert!(!baseline.roles().is_empty());

    // Population, window and trace each vary independently.
    let assessments = [
        Assessment {
            population: Some("orders-q1"),
            window: None,
            trace: None,
        },
        Assessment {
            population: None,
            window: Some("2026-01/2026-02"),
            trace: None,
        },
        Assessment {
            population: None,
            window: None,
            trace: Some("trace:replay-7"),
        },
    ];

    let mut provenance = Vec::new();
    for assessment in &assessments {
        let sources = ecosystem(&model, R::StateGraph);
        let models = [ModelInput::Native(&model)];
        let (subject, retained) = with_reports(
            &sources,
            &models,
            WorkLimits::default(),
            Limits::default(),
            BindingLimits::default(),
            |inventory, _, report| {
                let subject = StaticSubject::of(inventory, report).unwrap();
                let healthy = report.namespace().lookup("Healthy")[0];
                let requested = [Request {
                    declaration: healthy,
                    capability: Capability::OperationContract,
                    required: true,
                }];
                let requests = requests::report(report, assessment, &requested);
                (subject, requests.assessment().clone())
            },
        );
        assert_eq!(subject.differences(&baseline), Vec::new());
        assert_eq!(subject, baseline);
        provenance.push(retained);
    }
    // Each assessment selection is retained and each one is distinct.
    for (index, first) in provenance.iter().enumerate() {
        for second in &provenance[index + 1..] {
            assert_ne!(first, second);
        }
    }
    assert_eq!(provenance[0].population.as_deref(), Some("orders-q1"));
    assert_eq!(provenance[1].window.as_deref(), Some("2026-01/2026-02"));
    assert_eq!(provenance[2].trace.as_deref(), Some("trace:replay-7"));
}

#[test]
#[trace("TC-115", "FR-036-AC-5")]
fn a_resource_only_configuration_change_stays_visible_without_changing_meaning() {
    let model = setup::native_rule_model::parts().model();
    let sources = ecosystem(&model, R::StateGraph);
    let models = [ModelInput::Native(&model)];

    let lowered_package = WorkLimits {
        source_bytes: 1_000_000,
        units: 16,
        declarations: 64,
        references: 500_000,
        dependency_edges: 500_000,
    };
    let lowered_parser = Limits {
        source_bytes: 262_144,
        ..Limits::default()
    };
    let lowered_binding = BindingLimits {
        bytes: 16_000_000,
        definitions: 64,
        ..BindingLimits::default()
    };

    let configurations = [
        (
            WorkLimits::default(),
            Limits::default(),
            BindingLimits::default(),
        ),
        (lowered_package, lowered_parser, lowered_binding),
    ];
    let mut retained = Vec::new();
    for (package_limits, parser_limits, binding_limits) in configurations {
        retained.push(with_reports(
            &sources,
            &models,
            package_limits,
            parser_limits,
            binding_limits,
            |inventory, namespace, report| {
                assert!(report.complete(), "{:?}", report.exhaustion());
                (
                    StaticSubject::of(inventory, report).unwrap(),
                    BuildProvenance::of(namespace, report),
                )
            },
        ));
    }
    let (first, first_build) = &retained[0];
    let (second, second_build) = &retained[1];

    // Static meaning is unchanged in every declared component.
    assert_eq!(second.differences(first), Vec::new());
    assert_eq!(second, first);

    // The resource-only change remains visible in build provenance alone.
    assert_ne!(second_build, first_build);
    assert_ne!(second_build.package_limits, first_build.package_limits);
    assert_ne!(second_build.parser_limits, first_build.parser_limits);
    assert_ne!(second_build.binding_limits, first_build.binding_limits);
    assert_eq!(second_build.package_usage, first_build.package_usage);
    assert_eq!(second_build.binding_usage, first_build.binding_usage);
}

#[test]
#[trace("TC-115", "FR-036-AC-5")]
fn changing_a_required_profile_selection_changes_the_static_subject() {
    let model = setup::native_rule_model::parts().model();
    let graph = subject_of(&model, R::StateGraph);
    let queries = subject_of(&model, R::StateQueries);

    assert_ne!(queries, graph);
    // The selection is authored in the state unit's own text, so that unit's
    // bytes move with it and so do the source regions its roles retain. The
    // model and native dependency components are untouched by the change.
    assert_eq!(
        queries.differences(&graph),
        vec![
            ComponentKind::Sources,
            ComponentKind::Profiles,
            ComponentKind::BindingRequirements,
        ]
    );
    assert_ne!(queries.sources()[0].digest, graph.sources()[0].digest);
    assert_eq!(queries.sources()[1..], graph.sources()[1..]);
    assert_eq!(queries.models(), graph.models());
    assert_eq!(queries.dependencies(), graph.dependencies());

    // Only the changed unit's roles moved; the other families keep theirs.
    let elsewhere = |subject: &StaticSubject| {
        subject
            .roles()
            .iter()
            .filter(|role| role.declaration == "Due" || role.declaration == "Flow")
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(elsewhere(&queries), elsewhere(&graph));
    assert!(!elsewhere(&graph).is_empty());

    let changed: Vec<_> = queries
        .profiles()
        .iter()
        .zip(graph.profiles())
        .filter(|(new, old)| new.closure != old.closure)
        .collect();
    assert!(!changed.is_empty());
    for (new, old) in changed {
        assert_eq!(new.declaration, old.declaration);
        assert_eq!(new.alias, old.alias);
        assert_eq!(new.closure[0], R::StateQueries);
        assert_eq!(old.closure[0], R::StateGraph);
    }
}

#[test]
#[trace("TC-115", "FR-036-AC-5")]
fn a_stale_model_selection_refuses_instead_of_inheriting_the_new_meaning() {
    // The units select the original model; a later model is supplied instead.
    let authored = setup::native_rule_model::parts().model();
    let replacement = setup::authored_model(|document| {
        document["requirement"] = Value::from("Replacement");
    });
    assert_ne!(authored.digest(), replacement.digest());

    let sources = ecosystem(&authored, R::StateGraph);
    let models = [ModelInput::Native(&replacement)];
    with_reports(
        &sources,
        &models,
        WorkLimits::default(),
        Limits::default(),
        BindingLimits::default(),
        |inventory, _, report| {
            assert!(report.complete(), "{:?}", report.exhaustion());
            let subject = StaticSubject::of(inventory, report).unwrap();
            for component in subject.models() {
                assert_eq!(component.selection, Err(ImportRefusal::StaleSelection));
            }
            // No declaration inherits the replacement's meaning.
            for entry in report.declarations() {
                assert_eq!(
                    report.disposition(entry.declaration()),
                    Some(BindingDisposition::Refused)
                );
            }
        },
    );
}

// ---------------------------------------------------------------------------
// FR-036-AC-6: requested capability dispositions and aggregate availability.
// ---------------------------------------------------------------------------

/// One supported state request and one unsupported temporal projection.
fn requested(report: &binding::Report<'_>) -> [Request; 2] {
    let namespace = report.namespace();
    [
        Request {
            declaration: namespace.lookup("Healthy")[0],
            capability: Capability::OperationContract,
            required: true,
        },
        Request {
            declaration: namespace.lookup("Due")[0],
            capability: Capability::TemporalSatisfaction,
            required: true,
        },
    ]
}

/// FR-077 (ADR-010 OBS-003): with no `Backend` parameter, `report` has no
/// backend state to consult, so both a state and a temporal request over
/// applicable families are `Admitted` here -- whether a backend actually
/// supports either is settled downstream (the `#185` registry and
/// `quire-contract-codegen`'s `negotiate_*`), never by this function.
#[test]
#[trace("TC-115", "FR-036-AC-6")]
fn two_required_requests_over_different_capability_kinds_both_stay_admitted_in_caller_order() {
    let model = setup::native_rule_model::parts().model();
    let sources = ecosystem(&model, R::StateGraph);
    let models = [ModelInput::Native(&model)];
    let assessment = Assessment {
        population: Some("orders-q1"),
        window: None,
        trace: None,
    };
    with_reports(
        &sources,
        &models,
        WorkLimits::default(),
        Limits::default(),
        BindingLimits::default(),
        |_, _, report| {
            let namespace = report.namespace();
            let positive = namespace.lookup("Positive")[0];
            let healthy = namespace.lookup("Healthy")[0];
            let due = namespace.lookup("Due")[0];
            let flow = namespace.lookup("Flow")[0];
            let asked = requested(report);
            let requests = requests::report(report, &assessment, &asked);

            assert_eq!(requests.responses().len(), 2);
            assert_eq!(requests.responses()[0].request, asked[0]);
            assert_eq!(requests.responses()[1].request, asked[1]);
            assert_eq!(requests.responses()[0].family, Some(Family::State));
            assert_eq!(requests.responses()[1].family, Some(Family::Temporal));

            // Both requests -- over different capability kinds -- remain
            // admitted, in caller order (FR-036-AC-6, FR-077-AC-3).
            assert_eq!(
                requests.disposition(healthy, Capability::OperationContract),
                Some(Disposition::Admitted)
            );
            assert_eq!(
                requests.disposition(due, Capability::TemporalSatisfaction),
                Some(Disposition::Admitted)
            );

            // Complete aggregate success is attainable: nothing at this
            // admission-only layer settled a required pair unavailable.
            assert_eq!(requests.aggregate(), Aggregate::Attainable);

            // FR-057 "Family-body admission": body admission is unconditional
            // on binding disposition, not on `requests` above -- every
            // declaration whose names resolved is an admitted body,
            // including `Positive` and `Flow`, neither of which was asked
            // about at all.
            assert_eq!(
                requests.admitted_bodies(),
                vec![positive, healthy, due, flow]
            );
        },
    );
}

#[test]
#[trace("TC-115", "FR-036-AC-6")]
fn deleting_either_requested_entry_fails_the_requested_inventory_check() {
    let model = setup::native_rule_model::parts().model();
    let sources = ecosystem(&model, R::StateGraph);
    let models = [ModelInput::Native(&model)];
    let assessment = Assessment {
        population: None,
        window: None,
        trace: None,
    };
    with_reports(
        &sources,
        &models,
        WorkLimits::default(),
        Limits::default(),
        BindingLimits::default(),
        |_, _, report| {
            let asked = requested(report);
            let requests = requests::report(report, &assessment, &asked);
            assert_eq!(requests.retains(&asked), Ok(()));

            // Deleting the state entry from the consumer input.
            assert_eq!(
                requests.retains(&asked[1..]),
                Err(InventoryGap::MismatchedResponse { request: 0 })
            );
            // Deleting the temporal entry from the consumer input.
            assert_eq!(
                requests.retains(&asked[..1]),
                Err(InventoryGap::ExtraResponse { response: 1 })
            );
            // A report that dropped a requested pair fails the same check.
            let partial = requests::report(report, &assessment, &asked[..1]);
            assert_eq!(
                partial.retains(&asked),
                Err(InventoryGap::MissingResponse { request: 1 })
            );
        },
    );
}

/// FR-077: `InapplicableCapability` survives the backend-disposition removal
/// unchanged -- it is a structural check (does this capability kind apply to
/// this declaration's family at all, per `requests::families`), independent
/// of any backend: a kind applicable to a declaration's own family is
/// `Admitted`, one that is not is `InapplicableCapability`, regardless of
/// backend support (settled downstream). FR-057's "Family-body admission"
/// section is a distinct, unconditional rule tested separately below
/// (`admitted_bodies` does not depend on either disposition here).
#[test]
#[trace("TC-115", "FR-036-AC-6")]
fn a_claim_inapplicable_to_a_family_is_inapplicable_while_an_applicable_one_is_admitted() {
    let model = setup::native_rule_model::parts().model();
    let sources = ecosystem(&model, R::StateGraph);
    let models = [ModelInput::Native(&model)];
    let assessment = Assessment {
        population: None,
        window: None,
        trace: None,
    };
    let subject = with_reports(
        &sources,
        &models,
        WorkLimits::default(),
        Limits::default(),
        BindingLimits::default(),
        |inventory, _, report| {
            let namespace = report.namespace();
            let positive = namespace.lookup("Positive")[0];
            let due = namespace.lookup("Due")[0];
            let healthy = namespace.lookup("Healthy")[0];
            let flow = namespace.lookup("Flow")[0];
            let asked = [
                Request {
                    declaration: due,
                    capability: Capability::TemporalSatisfaction,
                    required: true,
                },
                Request {
                    declaration: healthy,
                    capability: Capability::OperationContract,
                    required: true,
                },
                // A claim that is not defined for this family at all.
                Request {
                    declaration: healthy,
                    capability: Capability::FiniteReplay,
                    required: false,
                },
            ];
            let requests = requests::report(report, &assessment, &asked);

            // Each kind applicable to its declaration's own family is
            // admitted; only the structurally inapplicable claim is refused.
            assert_eq!(
                requests.disposition(due, Capability::TemporalSatisfaction),
                Some(Disposition::Admitted)
            );
            assert_eq!(
                requests.disposition(healthy, Capability::OperationContract),
                Some(Disposition::Admitted)
            );
            assert_eq!(
                requests.disposition(healthy, Capability::FiniteReplay),
                Some(Disposition::InapplicableCapability(Family::State))
            );
            // The inapplicable request is not required, so aggregate
            // success is still attainable.
            assert_eq!(requests.aggregate(), Aggregate::Attainable);

            // FR-057 "Family-body admission": every declaration whose names
            // resolved is an admitted body, unconditionally -- `Positive`
            // and `Flow` are admitted bodies here despite neither being
            // named in `asked` at all, and `healthy`'s inapplicable,
            // unrequired `FiniteReplay` request does not withhold its body
            // either.
            assert_eq!(
                requests.admitted_bodies(),
                vec![positive, healthy, due, flow]
            );

            // Its authored syntax stays inspectable and unrewritten.
            let syntax = namespace.syntax(due).expect("original declaration syntax");
            assert_eq!(syntax.name.value, "Due");
            assert_eq!(syntax.profile.value, "T");
            // Binding resolved names only; that is not a checked body.
            assert_eq!(
                report.disposition(due),
                Some(BindingDisposition::NamesResolved)
            );
            StaticSubject::of(inventory, report).unwrap()
        },
    );
    // Recording requests rewrites no source, profile or model selection.
    let baseline = subject_of(&model, R::StateGraph);
    assert_eq!(subject.differences(&baseline), Vec::new());
}

#[test]
#[trace("TC-115", "FR-036-AC-6")]
fn refused_and_unknown_subjects_keep_their_own_dispositions() {
    let model = setup::native_rule_model::parts().model();
    let replacement = setup::authored_model(|document| {
        document["requirement"] = Value::from("Replacement");
    });
    let assessment = Assessment {
        population: None,
        window: None,
        trace: None,
    };

    // A declaration handle from the full three-family namespace.
    let sources = ecosystem(&model, R::StateGraph);
    let models = [ModelInput::Native(&model)];
    let outer = with_reports(
        &sources,
        &models,
        WorkLimits::default(),
        Limits::default(),
        BindingLimits::default(),
        |_, _, report| report.namespace().lookup("Flow")[0],
    );

    // The same handle is outside a smaller namespace of one declaration.
    let single = [source(
        "state",
        &model,
        &[("S", R::StateGraph)],
        "predicate Positive using S (amount: M::Version): Boolean { amount >= 0 }",
    )];
    with_reports(
        &single,
        &models,
        WorkLimits::default(),
        Limits::default(),
        BindingLimits::default(),
        |_, _, report| {
            let requests = requests::report(
                report,
                &assessment,
                &[Request {
                    declaration: outer,
                    capability: Capability::FiniteReplay,
                    required: true,
                }],
            );
            assert_eq!(
                requests.responses()[0].disposition,
                Disposition::UnknownSubject
            );
            assert_eq!(requests.responses()[0].family, None);
            assert_eq!(requests.aggregate(), Aggregate::Unavailable { response: 0 });
        },
    );

    // A statically refused subject keeps its own distinct disposition.
    let stale = [ModelInput::Native(&replacement)];
    with_reports(
        &sources,
        &stale,
        WorkLimits::default(),
        Limits::default(),
        BindingLimits::default(),
        |_, _, report| {
            let healthy = report.namespace().lookup("Healthy")[0];
            let requests = requests::report(
                report,
                &assessment,
                &[Request {
                    declaration: healthy,
                    capability: Capability::OperationContract,
                    required: true,
                }],
            );
            assert_eq!(
                requests.responses()[0].disposition,
                Disposition::RefusedSubject
            );
            assert_eq!(requests.admitted_bodies(), Vec::new());
        },
    );
}

// ---------------------------------------------------------------------------
// FR-077: no backend negotiation; requests still fully recorded as data.
// ---------------------------------------------------------------------------

/// FR-077-AC-3 (TC-200): recording does not depend on the negotiation logic
/// FR-077 removes. At least four admitted pairs spanning at least three
/// distinct capability kinds and both `required` values are all present in
/// the report, each with its original declaration, capability kind,
/// `required` flag and request index preserved -- none dropped, merged or
/// reordered.
#[test]
#[trace("TC-200", "FR-077-AC-3")]
fn four_admitted_pairs_across_three_kinds_stay_recorded_as_data() {
    let model = setup::native_rule_model::parts().model();
    let sources = ecosystem(&model, R::StateGraph);
    let models = [ModelInput::Native(&model)];
    let assessment = Assessment {
        population: None,
        window: None,
        trace: None,
    };
    with_reports(
        &sources,
        &models,
        WorkLimits::default(),
        Limits::default(),
        BindingLimits::default(),
        |_, _, report| {
            let namespace = report.namespace();
            let positive = namespace.lookup("Positive")[0];
            let healthy = namespace.lookup("Healthy")[0];
            let due = namespace.lookup("Due")[0];
            let flow = namespace.lookup("Flow")[0];
            let asked = [
                Request {
                    declaration: healthy,
                    capability: Capability::OperationContract,
                    required: true,
                },
                Request {
                    declaration: due,
                    capability: Capability::TemporalSatisfaction,
                    required: true,
                },
                Request {
                    declaration: flow,
                    capability: Capability::FiniteReplay,
                    required: false,
                },
                Request {
                    declaration: positive,
                    capability: Capability::ValueValidity,
                    required: false,
                },
            ];
            let requests = requests::report(report, &assessment, &asked);

            assert_eq!(requests.responses().len(), asked.len());
            for (index, (response, request)) in
                requests.responses().iter().zip(asked.iter()).enumerate()
            {
                assert_eq!(
                    response.request, *request,
                    "request {index} was dropped, merged or reordered"
                );
                assert_eq!(
                    requests.response(index).map(|response| response.request),
                    Some(*request),
                    "request index {index} does not resolve back to its own pair"
                );
            }
            // Every one of the four pairs is admitted: recording does not
            // depend on the (now-removed) negotiation logic.
            assert!(requests
                .responses()
                .iter()
                .all(|response| response.disposition == Disposition::Admitted));
        },
    );
}

// ---------------------------------------------------------------------------
// FR-036-AC-8: historical package and runner compatibility.
// ---------------------------------------------------------------------------

fn historical_model() -> NativeModel {
    setup::native_rule_model::from_text(
        include_str!("../fixtures/native-package/model-source.json"),
        "package-model.json",
        "1",
    )
    .expect("frozen package model fixture must decode")
    .model()
}

fn read_relabelled(
    value: &Value,
    bindings: quire_spec_language::checking::CheckBindings,
    models: &[NativeModel],
) -> Box<quire_spec_language::package::PackageError> {
    let bytes = serde_json::to_vec(value).unwrap();
    NativePackage::read_verified(
        &bytes,
        NativePackageRef::new(ByteDigest::of(&bytes)),
        bindings,
        models,
        &PackageSupport::default(),
        PackageReadLimits::default(),
    )
    .expect_err("a relabelled composed report must not enter the historical reader")
}

#[test]
#[trace("TC-115", "FR-036-AC-8")]
fn historical_package_identities_and_atomic_refusals_are_unchanged() {
    let models = [historical_model()];
    for vector in package_vector_setup::cases() {
        let checked = package_vector_setup::checked(&vector, &models);
        let bindings = checked.bindings().clone();
        let package = NativePackage::new(checked, PackageLimits::default()).unwrap();
        assert_eq!(package.bytes(), vector.artifact, "{}", vector.name);
        assert_eq!(
            package.canonical_identity().to_string(),
            vector.digest.trim_end(),
            "{}",
            vector.name
        );
        let reference = package.reference();
        let reread = NativePackage::read_verified(
            package.bytes(),
            reference,
            bindings,
            &models,
            &PackageSupport::default(),
            PackageReadLimits::default(),
        )
        .expect("frozen historical bytes still read through the unchanged reader");
        assert_eq!(reread.bytes(), vector.artifact, "{}", vector.name);
        assert_eq!(reread.canonical_identity(), package.canonical_identity());
        assert_eq!(reread.reference(), reference);
    }
}

#[test]
#[trace("TC-115", "FR-036-AC-8")]
fn a_changed_profile_label_cannot_admit_composed_input_into_the_historical_reader() {
    let models = [historical_model()];
    let vector = &package_vector_setup::cases()[0];
    let checked = package_vector_setup::checked(vector, &models);
    let bindings = checked.bindings().clone();
    let package = NativePackage::new(checked, PackageLimits::default()).unwrap();
    let original: Value = serde_json::from_slice(package.bytes()).unwrap();

    // Only the label changes; every other byte of the artifact is the original.
    for (pointer, label, code, field) in [
        (
            "/semantics/edition",
            "1-draft",
            Code::UnknownEdition,
            "edition",
        ),
        (
            "/semantics/syntax_profile",
            "composed-native/1-draft",
            Code::UnknownProfile,
            "syntax_profile",
        ),
        (
            "/semantics/model_profile",
            "native-composed-model/1",
            Code::UnknownProfile,
            "model_profile",
        ),
        (
            "/semantics/checking_contract",
            "native-composed-clauses/1",
            Code::UnknownProfile,
            "checking_contract",
        ),
    ] {
        let mut relabelled = original.clone();
        *relabelled.pointer_mut(pointer).expect(pointer) = Value::from(label);
        let error = read_relabelled(&relabelled, bindings.clone(), &models);
        assert_eq!(error.code, code, "{pointer}");
        assert_eq!(error.stage, PackageStage::Decode, "{pointer}");
        assert_eq!(
            format!("{:?}", error.path),
            format!("[Field(\"semantics\"), Field({field:?})]"),
            "{pointer}"
        );
    }

    // A composed wire label is refused on its own code, not the profile's.
    let mut retagged = original.clone();
    retagged["format"] = Value::from("native-composed-package/1");
    let error = read_relabelled(&retagged, bindings.clone(), &models);
    assert_eq!(error.code, Code::UnknownWire);
    assert_eq!(error.stage, PackageStage::Decode);
    assert_eq!(format!("{:?}", error.path), "[Field(\"format\")]");

    // Relabelling under the original reference is a stale selection, atomically.
    let mut relabelled = original.clone();
    relabelled["semantics"]["syntax_profile"] = Value::from("composed-native/1-draft");
    let bytes = serde_json::to_vec(&relabelled).unwrap();
    let error = *NativePackage::read_verified(
        &bytes,
        package.reference(),
        bindings,
        &models,
        &PackageSupport::default(),
        PackageReadLimits::default(),
    )
    .expect_err("the historical reference cannot carry relabelled bytes");
    assert_eq!(error.code, Code::StaleDependency);
    assert_eq!(error.stage, PackageStage::Decode);
    assert!(error.path.is_empty());

    // The accepted fixture bytes and identities are untouched by every attempt.
    assert_eq!(package.bytes(), vector.artifact);
    assert_eq!(
        package.canonical_identity().to_string(),
        vector.digest.trim_end()
    );
}

#[test]
#[trace("TC-115", "FR-036-AC-8")]
fn the_historical_runner_refuses_a_relabelled_composed_input() {
    let model = setup::native_rule_model::parts().model();
    let snapshot = setup::snapshot(setup::draft(&model));
    let original = snapshot.bytes().to_vec();
    let accepted = snapshot.reference();

    // The unchanged runner still admits its own artifact bytes.
    let reread = Snapshot::read_verified(&accepted, &original, ArtifactLimits::default())
        .expect("historical runner input is unchanged");
    assert_eq!(reread.bytes(), original);
    assert_eq!(reread.reference(), accepted);

    // Relabelling only the envelope version does not widen admission.
    let mut envelope: Value = serde_json::from_slice(&original).unwrap();
    envelope["version"] = Value::from("native-composed-input/1");
    let relabelled = serde_json::to_vec(&envelope).unwrap();
    let expected = SnapshotRef::new(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:runtime-current".into(),
            revision_namespace: "test".into(),
            revision: "1".into(),
        },
        ByteDigest::of(&relabelled),
    )
    .unwrap();
    let error = *Snapshot::read_verified(&expected, &relabelled, ArtifactLimits::default())
        .expect_err("a composed label cannot enter the historical runner");
    assert_eq!(error.code, Code::UnknownWire);
    assert_eq!(error.stage, InputReadStage::Envelope);
    assert!(matches!(
        error.cause,
        InputReadCause::Version { ref actual } if actual == "native-composed-input/1"
    ));

    // Submitting the relabelled bytes under the accepted reference is stale.
    let error = *Snapshot::read_verified(&accepted, &relabelled, ArtifactLimits::default())
        .expect_err("the accepted reference cannot carry relabelled bytes");
    assert_eq!(error.code, Code::StaleDependency);
    assert_eq!(error.stage, InputReadStage::Selection);
    assert!(matches!(error.cause, InputReadCause::DigestMismatch { .. }));

    // The accepted artifact bytes and identity are unchanged.
    assert_eq!(snapshot.bytes(), original);
    assert_eq!(snapshot.reference(), accepted);
}
