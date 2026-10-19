// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-842 (FR-332): an infinite-trace temporal item settles only through
//! negotiation. The clause is real source over the ConfigVersion model; the
//! negotiation dispositions are the test's fixtures, as CG's `negotiate_*`
//! produces them outside this repository.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_foundation::bound::{DomainKind, FiniteBound};
use qsl_foundation::diagnostic::{CatalogCoded, Category};
use qsl_foundation::digest::ByteDigest;
use qsl_foundation::{BackendId, SourceIdentity};
use qsl_replay::{Execution, ResultLabel};
use qsl_route::request::{
    items_from_requirements, BoundRefusal, ExtentClassification, RequestWriter,
};
use qsl_route::routing::{route, Disposition};
use qsl_route::{
    BackendDescriptor, Candidate, CandidateOutcome, ManifestDigest, Mode, ProviderOrigin, Registry,
};
use qsl_semantics::check::{Capability, CheckedGraph, PackageDeclarations};
use quire_semantic_value::checking::CheckingLimits;

/// ADR-018 §6's `Reaches`: `eventually holds(c.versionNumber = 2)` under the
/// infinite-trace profile.
fn reaches() -> CheckedGraph {
    let document = crate::support::config_version_spine::spine::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]);
    let digest = packages.keys().next().expect("one package");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.infinite-trace/v1\";\n\
         model Config = \"example/config-version\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         temporal Reaches using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ eventually holds(c.versionNumber = 2) }}\n",
        qsl_semantics::model::key::hex(digest)
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "test:negotiation", "fixture", "fixture:1"),
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
        .expect("the unit checks")
}

fn backend(id: &str, mode: Mode) -> BackendDescriptor {
    BackendDescriptor::new(
        Candidate::new(BackendId::new(id)),
        ProviderOrigin::Linked,
        ManifestDigest::from_digest(ByteDigest::of(id.as_bytes()).as_bytes()),
        [(Capability::TemporalSatisfaction, mode)],
        // A bounded pair names the domains it can bound (FR-290).
        match mode {
            Mode::Bounded => Some(BTreeSet::from([DomainKind::Collection])),
            Mode::Unbounded => None,
        },
    )
    .expect("a well-formed descriptor")
}

fn registry_of(descriptors: impl IntoIterator<Item = BackendDescriptor>) -> Registry {
    let mut registry = Registry::new();
    for descriptor in descriptors {
        registry.register(descriptor).expect("one registration");
    }
    registry
}

/// The candidates the one `Reaches` item gets from `registry`.
fn candidates_of(graph: &CheckedGraph, registry: &Registry) -> (Capability, Vec<Candidate>) {
    let items = items_from_requirements(graph.requirements(), registry, None);
    let [item] = items.as_slice() else {
        panic!("one temporal item: {items:#?}");
    };
    let CandidateOutcome::Candidates(set) = item.candidates() else {
        panic!("a computed candidate set");
    };
    (set.kind(), set.candidates().to_vec())
}

/// `Reaches` is written as one `unbounded` item with no finite bound
/// available, and a bounded request for it refuses and writes no item.
#[trace("TC-842", "FR-332-AC-1")]
#[test]
fn an_infinite_trace_item_is_unbounded_and_refuses_a_bounded_request() {
    let graph = reaches();
    let (occurrence, record) = graph
        .requirements()
        .iter()
        .next()
        .expect("the clause's requirement record");
    let requirements = record.requirements();
    assert_eq!(requirements.kind(), Capability::TemporalSatisfaction);

    let mut writer = RequestWriter::new();
    writer.item(occurrence.clone(), requirements);
    let [item] = writer.items() else {
        panic!("one item");
    };
    assert_eq!(item.kind(), Capability::TemporalSatisfaction);
    assert_eq!(
        item.extent(),
        ExtentClassification::Unbounded {
            finite_bound_available: false
        }
    );

    let qsl_semantics::family::ClaimExtent::Unbounded(domains) = requirements.extent() else {
        panic!("an unbounded extent");
    };
    let (formula, kind) = domains.iter().next().expect("the formula domain");
    assert_eq!(kind, DomainKind::InfiniteTrace);
    let before = writer.items().len();
    let refusal = writer
        .bounded_item(
            occurrence.clone(),
            requirements,
            BTreeMap::from([(formula.clone(), FiniteBound::cardinality(8))]),
        )
        .expect_err("no finite bound stands for an infinite-trace formula");
    assert!(
        matches!(refusal, BoundRefusal::UnboundableDomain { .. }),
        "{refusal:?}"
    );
    let code = refusal.catalog_code();
    assert_eq!(
        (code.code(), code.cause()),
        ("invalid_runtime_input", "invalid-value")
    );
    assert_eq!(
        writer.items().len(),
        before,
        "a refused request writes no item"
    );
}

/// Against an empty registry the item has an empty candidate set naming
/// `temporal-satisfaction` and its fixture settlement routes nowhere; a
/// bounded-only descriptor is a candidate whose fixture settlement
/// (`unsupported`, `unbounded-extent`) routes nowhere; an unbounded
/// descriptor's `supported` settlement routes to it.
#[trace("TC-842", "FR-332-AC-2")]
#[test]
fn an_infinite_trace_item_routes_only_to_a_backend_negotiation_supports() {
    let graph = reaches();

    let (kind, none) = candidates_of(&graph, &Registry::new());
    assert_eq!(kind.to_string(), "temporal-satisfaction");
    assert!(none.is_empty(), "{none:?}");
    assert_eq!(route(&[Disposition::Unsupported]), vec![None]);

    let bounded = backend("bounded-only", Mode::Bounded);
    let modes: Vec<Mode> = bounded.advertises().map(|(_, mode)| mode).collect();
    assert_eq!(modes, [Mode::Bounded]);
    let (_, only) = candidates_of(&graph, &registry_of([bounded.clone()]));
    assert_eq!(only, [bounded.candidate().clone()]);
    assert_eq!(route(&[Disposition::Unsupported]), vec![None]);

    let unbounded = backend("liveness", Mode::Unbounded);
    let (_, sole) = candidates_of(&graph, &registry_of([unbounded.clone()]));
    let candidate = unbounded.candidate().clone();
    assert_eq!(sole, std::slice::from_ref(&candidate));
    assert_eq!(
        route(&[Disposition::Supported(candidate.clone())]),
        vec![Some(&candidate)]
    );
}

/// Each FR-360 label settles in its ADR-014 A-5 category; `failed` is
/// incomplete under `resource-incomplete` and an internal failure under any
/// other execution.
#[trace("TC-842", "FR-332-AC-3")]
#[test]
fn each_backend_result_label_maps_to_its_category() {
    let completed = Execution::Completed;
    assert_eq!(ResultLabel::Proved.category(completed), Category::Success);
    assert_eq!(
        ResultLabel::Refuted.category(completed),
        Category::Violation
    );
    assert_eq!(
        ResultLabel::Inconclusive.category(completed),
        Category::Inconclusive
    );
    assert_eq!(
        ResultLabel::Unsupported.category(completed),
        Category::Unsupported
    );
    assert_eq!(
        ResultLabel::Failed.category(Execution::ResourceIncomplete),
        Category::Incomplete
    );
    for other in [Execution::Completed, Execution::Failed] {
        assert_eq!(
            ResultLabel::Failed.category(other),
            Category::InternalFailure
        );
    }
}
