// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-134 (TC-535): a compiled package's clause fairness set reaches the
//! request item's candidate filter, against backends registered from
//! manifests.

use ix_trace_rs::trace;
use qsl_foundation::digest::ByteDigest;
use qsl_foundation::{BackendId, SourceIdentity};
use qsl_route::request::items_from_requirements;
use qsl_route::{
    descriptor_from_manifest, Candidate, CandidateOutcome, FairnessKindsMember, FairnessLabel,
    ManifestDigest, ProviderOrigin, Registry,
};
use qsl_semantics::check::{Capability, PackageDeclarations};
use quire_semantic_value::checking::CheckingLimits;

/// The ConfigVersion unit with one infinite-trace clause whose body is `body`.
pub(crate) fn graph_of(body: &str) -> qsl_semantics::check::CheckedGraph {
    let document = crate::support::config_version_spine::spine::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]);
    let digest = packages.keys().next().expect("one package");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.infinite-trace/v1\";\n\
         model Config = \"example/config-version\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         temporal C using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ {body} }}\n",
        qsl_semantics::model::key::hex(digest)
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "test:fairness", "fixture", "fixture:1"),
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
        .unwrap_or_else(|refusals| panic!("{body}: {refusals:?}"))
}

/// A registry of manifest-registered temporal backends: `both` advertises
/// `weak` and `strong`, `weak-only` advertises `weak`.
fn registry() -> Registry {
    let mut registry = Registry::new();
    for (index, (id, labels)) in [
        (
            "both",
            &[FairnessLabel::Text("weak"), FairnessLabel::Text("strong")][..],
        ),
        ("weak-only", &[FairnessLabel::Text("weak")][..]),
    ]
    .into_iter()
    .enumerate()
    {
        let descriptor = descriptor_from_manifest(
            Candidate::new(BackendId::new(id)),
            ProviderOrigin::Process,
            ManifestDigest::from_digest(ByteDigest::of(&[index as u8]).as_bytes()),
            [(Some("temporal-satisfaction"), Some("unbounded"))],
            None,
            Some(FairnessKindsMember::List(labels)),
            None,
        )
        .expect("a valid temporal manifest");
        registry.register(descriptor).expect("admitted");
    }
    registry
}

fn candidates_for(body: &str) -> Vec<String> {
    let graph = graph_of(body);
    let registry = registry();
    let items = items_from_requirements(graph.requirements(), &registry, None);
    let [item] = items.as_slice() else {
        panic!("one temporal item: {items:#?}");
    };
    assert_eq!(item.kind(), Capability::TemporalSatisfaction);
    let CandidateOutcome::Candidates(set) = item.candidates() else {
        panic!("a computed candidate set");
    };
    set.candidates()
        .iter()
        .map(|candidate| candidate.id().to_string())
        .collect()
}

const EVENTUALLY_TWO: &str = "always eventually holds(c.versionNumber = 2)";

/// FR-134-AC-1 through the compiled item path: a `strong` clause is given
/// only the backend whose manifest advertises `strong`; a `weak` clause is
/// given both; a clause with no fairness is given both.
#[trace("TC-535")]
#[test]
fn a_compiled_clauses_fairness_set_filters_its_items_candidates() {
    let fair =
        |kind: &str| format!("fair {kind} Config::ConfigVersion::attemptUpdate; {EVENTUALLY_TWO}");
    assert_eq!(candidates_for(&fair("strong")), ["both"]);
    assert_eq!(candidates_for(&fair("weak")), ["both", "weak-only"]);
    assert_eq!(candidates_for(EVENTUALLY_TWO), ["both", "weak-only"]);
}
