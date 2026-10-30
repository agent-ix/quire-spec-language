// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-805 (FR-306-AC-3, FR-306-AC-4): carrying the abstraction relation in a
//! checked package changes no requirement record, no `route` result and no
//! clause disposition. Each case compiles FR-108's ConfigVersion unit twice,
//! once as written and once with a relation declaration after its last
//! declaration, so no span of an earlier declaration moves.

use ix_trace_rs::trace;
use qsl_foundation::{BackendId, SourceIdentity};
use qsl_replay::spine::{run_clause, ClauseRunStage};
use qsl_route::request::items_from_requirements;
use qsl_route::routing::{route, Disposition};
use qsl_route::{
    descriptor_from_manifest, Candidate, CandidateOutcome, ManifestMembers, ProviderOrigin,
    Registry,
};
use qsl_semantics::check::{CheckedGraph, PackageDeclarations};
use quire_semantic_value::checking::CheckingLimits;

use crate::support::config_version_spine::{cases::CASES, spine};

/// FR-304-AC-1's relation over the ConfigVersion package: an object binding
/// with a field, the `config_history` population and `attemptUpdate`.
const RELATION: &str = "abstraction A using v {\n\
    type Config::ConfigVersion to \"store::Version\" { versionNumber to \"number\"; };\n\
    population Config::config_history to \"store::history\";\n\
    operation Config::ConfigVersion::attemptUpdate to \"store::update\" receiver \"self\" ();\n\
    }\n";

fn with_relation(unit: &str) -> String {
    format!("{unit}{RELATION}")
}

fn graph_of(text: &str) -> CheckedGraph {
    let packages =
        qsl_semantics::model::intake::package_input([spine::DOMAIN_PACKAGE.as_bytes()]).unwrap();
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "test:abstraction-inert", "fixture", "fixture:1"),
        "program.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let raw = parsed.source().reference().clone();
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the model is admitted");
    PackageDeclarations::assemble(raw, unit, models, Vec::new())
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"))
}

/// One backend per capability the records need, each advertising its kind.
fn registry(graph: &CheckedGraph) -> Registry {
    let mut registry = Registry::new();
    let mut kinds: Vec<&'static str> = graph
        .requirements()
        .values()
        .map(|record| record.requirements().kind().to_wire())
        .collect();
    kinds.sort_unstable();
    kinds.dedup();
    for kind in kinds {
        let descriptor = descriptor_from_manifest(
            Candidate::new(BackendId::new(format!("backend-{kind}"))),
            ProviderOrigin::Process,
            [(Some(kind), Some("unbounded"))],
            ManifestMembers {
                domains: None,
                fairness_kinds: None,
                evidence_kinds: None,
                memory_models: None,
                reductions: None,
            },
        )
        .expect("a valid manifest");
        registry.register(descriptor).expect("admitted");
    }
    registry
}

/// The `route` result of every requirement item of `graph` over `registry`:
/// an item with exactly one candidate settles `supported` with it, any other
/// item `unsupported`.
fn routes(graph: &CheckedGraph, registry: &Registry) -> Vec<Option<Candidate>> {
    let items = items_from_requirements(graph.requirements(), registry, None);
    let dispositions: Vec<Disposition> = items
        .iter()
        .map(|item| match item.candidates() {
            CandidateOutcome::Candidates(set) => match set.candidates() {
                [one] => Disposition::Supported(one.clone()),
                _ => Disposition::Unsupported,
            },
            _ => Disposition::Unsupported,
        })
        .collect();
    route(&dispositions)
        .into_iter()
        .map(|target| target.cloned())
        .collect()
}

/// FR-306-AC-3: the ConfigVersion unit's requirement records are equal with
/// and without its relation declaration, and so are their `route` results
/// over the same backend descriptors.
#[trace("TC-805", "FR-306-AC-3")]
#[test]
fn a_relation_changes_no_requirement_record_or_route_result() {
    let without = graph_of(&spine::unit_text());
    let with = graph_of(&with_relation(&spine::unit_text()));
    assert!(without.abstraction().is_none());
    assert_eq!(
        with.abstraction().map(|relation| relation.len()),
        Some(3),
        "the declaration reaches the checked graph"
    );

    assert!(
        without.requirements().len() >= 3,
        "the unit's clauses write requirement records"
    );
    assert_eq!(with.requirements(), without.requirements());

    let registry = registry(&without);
    let route_without = routes(&without, &registry);
    assert_eq!(route_without.len(), without.requirements().len());
    assert!(
        route_without.iter().any(Option::is_some),
        "the registry routes at least one record"
    );
    assert_eq!(routes(&with, &registry), route_without);
}

/// FR-306-AC-4: every ConfigVersion corpus case of FR-108 run through
/// `run_clause` gives the same disposition for the unit with its relation
/// declaration as for the unit without it.
#[trace("TC-805", "FR-306-AC-4")]
#[test]
fn a_relation_changes_no_clause_disposition() {
    let directory = tempfile::tempdir().expect("a directory");
    let mut evaluated = 0;
    for &case in CASES {
        let path = directory.path().join(case.id());
        spine::write(&path, case).expect("the spine files");
        let plain = spine::request(&path, case).expect("a request");
        let mut carried = spine::request(&path, case).expect("a request");
        let qsl_replay::spine::ClauseRunSource::Program { bytes, .. } = &mut carried.source else {
            panic!("a program source");
        };
        let text = String::from_utf8(bytes.clone()).expect("UTF-8 source");
        *bytes = with_relation(&text).into_bytes();

        let plain = run_clause(plain).expect("a plain report");
        let carried = run_clause(carried).expect("a report with the relation");
        // A compile-stage refusal names the digest of the source bytes, which
        // the declaration changes by being there; its stage and category are
        // what must agree. Every other disposition is compared whole.
        if plain.disposition.stage() == ClauseRunStage::Compile {
            assert_eq!(carried.disposition.stage(), ClauseRunStage::Compile);
            assert_eq!(
                carried.disposition.category(),
                plain.disposition.category(),
                "{}",
                case.id()
            );
        } else {
            assert_eq!(
                format!("{:?}", carried.disposition),
                format!("{:?}", plain.disposition),
                "{}",
                case.id()
            );
            evaluated += 1;
        }
        assert_eq!(carried.disposition.truth(), plain.disposition.truth());
    }
    assert!(evaluated >= 10, "the corpus runs past compile: {evaluated}");
}
