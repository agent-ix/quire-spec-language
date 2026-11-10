// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1395: a registered process plugin changes no matrix outcome and only
//! the candidate sets it advertises (FR-566).

use ix_trace_rs::trace;
use qsl_analyze::analyze::{analyze_items, Engine, Engines};
use qsl_analyze::external_provider::{
    provider_item, ExternalProvider, ProviderRequest, ProviderVerdict,
};
use qsl_conformance::{lifecycle_matrix, run_case_with, MatrixCell};
use qsl_foundation::{BackendId, RequestIndex};
use qsl_package::CheckedPackage;
use qsl_replay::value_check::ValueTarget;
use qsl_replay::{Certification, ProofBasis, TerminalValue};
use qsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, CandidateOutcome, ManifestMembers,
    ProviderOrigin, Registry,
};
use qsl_semantics::check::{BodyTerm, Capability, PackageDeclarations};
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::value::{admit_integer_division, DefinitionRole};
use quire_exact::Cancel;
use quire_semantic_value::checking::CheckingLimits;

const PLUGIN: &str = "qsl.test-plugin";
const LINKED: &str = "qsl.test-linked";

/// The in-process stand-in for the test plugin: it answers `proved`.
#[derive(Debug)]
struct Standin;

impl ExternalProvider for Standin {
    fn check(&self, _request: &ProviderRequest<'_, '_>, _cancel: &Cancel) -> ProviderVerdict {
        ProviderVerdict::Proved
    }
}

fn descriptor(id: &str, origin: ProviderOrigin, kind: Capability, mode: &str) -> BackendDescriptor {
    descriptor_from_manifest(
        Candidate::new(BackendId::new(id)),
        origin,
        [(Some(kind.to_wire()), Some(mode))],
        ManifestMembers {
            domains: Some(&["integer"]),
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    )
    .expect("a well-formed manifest is admitted")
}

/// The linked descriptor the first slice stands for: not QSL's Kani manifest
/// (the driver links that one), only a registered linked backend.
fn linked() -> BackendDescriptor {
    descriptor(
        LINKED,
        ProviderOrigin::Linked,
        Capability::OperationContract,
        "bounded",
    )
}

/// The test plugin's descriptor: `Process` origin, advertising
/// (`value-validity`, `unbounded`) and nothing else.
fn plugin() -> BackendDescriptor {
    descriptor(
        PLUGIN,
        ProviderOrigin::Process,
        Capability::ValueValidity,
        "unbounded",
    )
}

fn engines_with_plugin() -> Engines {
    let mut engines = Engines::qsl().expect("the layer-A manifests register");
    engines
        .with(Engine::external(plugin(), Box::new(Standin)).expect("an unbounded value arm"))
        .expect("a new backend");
    engines
}

fn registry(descriptors: impl IntoIterator<Item = BackendDescriptor>) -> Registry {
    let mut registry = Registry::new();
    for descriptor in descriptors {
        registry.register(descriptor).expect("a new backend");
    }
    registry
}

/// FR-566 configurations 1, 2 and 3.
fn configurations() -> [(Registry, Engines); 3] {
    [
        (Registry::new(), Engines::qsl().expect("they register")),
        (registry([linked()]), Engines::qsl().expect("they register")),
        (registry([linked(), plugin()]), engines_with_plugin()),
    ]
}

/// FR-566-AC-1: every `Case` gives an equal outcome in the three
/// configurations. `run_case_with` ignores the registry (the cases route
/// their items by hand), so configurations 1 and 2 cannot differ by design,
/// which FR-566 permits; configuration 3 differs only in `Engines`. The
/// `package` case is FR-275-AC-1's package: the package operation takes
/// neither registry nor engines, so its output cannot change, and the typed
/// package document, which holds no bytes, is the surface compared. The
/// FR-566-AC-1 tag stays open because its package-byte equality control is
/// not asserted here.
#[trace("TC-1395")]
#[test]
fn every_case_is_unchanged_by_the_registered_descriptors_and_the_plugin_provider() {
    let configurations = configurations();
    for cell in lifecycle_matrix() {
        let MatrixCell::Case(case) = cell else {
            continue;
        };
        let outcomes: Vec<_> = configurations
            .iter()
            .map(|(registry, engines)| {
                run_case_with(case, registry, engines)
                    .unwrap_or_else(|error| panic!("{:?}/{:?}: {error}", case.verb, case.kind))
            })
            .collect();
        let label = format!("{:?}/{:?}", case.verb, case.kind);
        crate::support::assert_same_outcome(&outcomes[0], &outcomes[1], &label);
        crate::support::assert_same_outcome(&outcomes[1], &outcomes[2], &label);
    }
}

const GUARDED: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    function q using v(a: Integer, b: Integer): Integer pure { if b != 0 then a div b else 0 }\n";

fn guarded_package() -> CheckedPackage {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("quire", "guarded.native"),
        "guarded.native",
        GUARDED.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "package".to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .expect("the assembler builds the declarations");
    declarations.integer_division = Some(
        admit_integer_division(
            &[DefinitionRole::IntegerDivisionEuclidean.reference()],
            None,
        )
        .expect("Euclidean division is admitted"),
    );
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the package checks"),
    )
}

/// FR-566-AC-1's control: in configuration 3 FR-281-AC-14's item settled
/// `Supported(qsl.test-plugin)` runs on the plugin provider, and its record
/// names `qsl.test-plugin` with certification `trusted`.
#[trace("TC-1395")]
#[test]
fn configuration_3_runs_an_item_on_the_plugin_provider() {
    let package = guarded_package();
    let key = package
        .graph()
        .requirements()
        .iter()
        .filter(|(_, record)| {
            record.requirements().kind() == Capability::ValueValidity && record.owner() == Some("q")
        })
        .map(|(key, _)| key.clone())
        .find(|key| {
            package
                .graph()
                .semantic_graph()
                .node(key.node().clone())
                .is_some_and(|node| {
                    matches!(node.body(), BodyTerm::Application(application)
                    if application.operation.identity() == "quire.op.integer.div")
                })
        })
        .expect("the division's value-validity requirement");
    let target = ValueTarget::select(&package, "q", &key);
    let records = analyze_items(
        &engines_with_plugin(),
        vec![provider_item(
            RequestIndex::new(0),
            &package,
            &key,
            &BackendId::new(PLUGIN),
            target,
        )],
        &Cancel::new(),
    );
    let terminal = records[0].terminal().expect("a terminal record");
    assert_eq!(terminal.backend().map(BackendId::as_str), Some(PLUGIN));
    assert_eq!(
        terminal.value(),
        &TerminalValue::Proved {
            basis: ProofBasis::Inductive { depth: 0 },
            certification: Certification::Trusted,
        }
    );
}

fn candidates(registry: &Registry, kind: Capability) -> Vec<String> {
    match registry.candidates(kind, None) {
        CandidateOutcome::Candidates(set) => set
            .candidates()
            .iter()
            .map(|candidate| candidate.id().as_str().to_owned())
            .collect(),
        CandidateOutcome::UnknownBackend(id) => panic!("no backend is named: {id:?}"),
    }
}

/// FR-566-AC-2: the plugin changes only the candidate sets of the kind it
/// advertises, and registration order changes nothing.
#[trace("TC-1395", "FR-566-AC-2")]
#[test]
fn the_plugin_adds_itself_only_to_the_kind_it_advertises() {
    let [_, (linked_only, _), (with_plugin, _)] = configurations();
    // FR-281-AC-1's item is a probabilistic satisfaction item.
    let unadvertised = Capability::ProbabilisticSatisfaction;
    assert_eq!(
        candidates(&with_plugin, unadvertised),
        candidates(&linked_only, unadvertised)
    );
    // The linked backend's own kind is unchanged too.
    assert_eq!(
        candidates(&with_plugin, Capability::OperationContract),
        candidates(&linked_only, Capability::OperationContract)
    );
    let advertised = Capability::ValueValidity;
    let mut expected = candidates(&linked_only, advertised);
    expected.push(PLUGIN.to_owned());
    expected.sort();
    assert_eq!(candidates(&with_plugin, advertised), expected);

    let plugin_first = registry([plugin(), linked()]);
    for kind in [unadvertised, advertised, Capability::OperationContract] {
        assert_eq!(
            candidates(&plugin_first, kind),
            candidates(&with_plugin, kind),
            "{kind:?}"
        );
    }
}
