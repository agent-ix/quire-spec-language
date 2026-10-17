// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-843, TC-844 and TC-845: an absent collection bound is the unbounded
//! type from source to the package, and a claim over an unbounded
//! declaration reaches a request item whose settlement is the caller's
//! fixture (FR-333, FR-334, FR-335). The population half of TC-844 and the
//! S6a half of TC-845 are in `qsl-eval`'s tests.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::bound::{DomainKey, DomainKind, FiniteBound, ProofBound};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::{RequestIndex, SourceIdentity};
use qsl_route::request::{items_from_requirements, ExtentClassification, RequestWriter};
use qsl_route::routing::{route, Disposition};
use qsl_route::{CandidateOutcome, Registry};
use qsl_semantics::check::{
    Capability, CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations,
};
use qsl_semantics::family::{ClaimExtent, Requirements};
use quire_exact::{IllTypedCause, NodeKey};
use quire_semantic_value::checking::CheckingLimits;

use super::request_builder::{check, package, registry, HEADER};

/// `declarations` through S1, S2 and S3, whose refusals are returned.
fn try_check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(CheckingLimits::default())
}

/// The `package_id` the spine emits for `declarations`.
fn package_id(declarations: &str) -> String {
    let text = format!("{HEADER}{declarations}\n");
    let emitted = crate::support::front_end::emitted(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        &BTreeMap::new(),
        &qsl_replay::spine::DependencyInput::default(),
    )
    .expect("the unit compiles");
    format!("{:?}", emitted.package().package_id())
}

/// The parameter node `index` of function `name`.
fn parameter(graph: &CheckedGraph, name: &str, index: usize) -> NodeKey {
    graph
        .semantic_graph()
        .node(graph.function_identity(name).expect("the function is declared"))
        .and_then(|function| function.function_parameters())
        .expect("a function node")[index]
}

/// The type node of parameter `index` of function `name`.
fn parameter_type(graph: &CheckedGraph, name: &str, index: usize) -> NodeKey {
    graph
        .semantic_graph()
        .node(parameter(graph, name, index))
        .and_then(|node| node.semantic_type())
        .expect("a typed parameter")
}

/// The semantic form of node `key`, and its body as printed.
fn form_and_body(graph: &CheckedGraph, key: NodeKey) -> (&'static str, String) {
    let node = graph.semantic_graph().node(key).expect("a graph node");
    (node.semantic_form(), format!("{:?}", node.body()))
}

fn has_collection_bounds(graph: &CheckedGraph) -> bool {
    graph
        .semantic_graph()
        .nodes()
        .any(|node| node.semantic_form() == "collection_bounds")
}

fn is_type_mismatch(refusals: &[CheckRefusal]) -> bool {
    matches!(
        refusals,
        [refusal] if matches!(refusal.cause, CheckCause::IllTyped(IllTypedCause::TypeMismatch))
    )
}

/// FR-333-AC-1: a collection type with no bound compiles through the spine
/// as its composite node alone, for every kind, and differs from the same
/// type with a bound.
#[trace("TC-843", "FR-333-AC-1")]
#[test]
fn an_absent_collection_bound_compiles_to_the_unbounded_type() {
    for (spelling, form) in [
        ("Sequence", "sequence"),
        ("Set", "set"),
        ("Bag", "bag"),
        ("OrderedSet", "ordered_set"),
    ] {
        let unbounded = check(&format!(
            "function f using v(s: {spelling}<Integer>): Integer pure {{ 0 }}"
        ));
        let (found, _) = form_and_body(&unbounded, parameter_type(&unbounded, "f", 0));
        assert_eq!(found, form, "{spelling}");
        assert!(!has_collection_bounds(&unbounded), "{spelling}");
    }
    let unbounded = check("function f using v(s: Set<Integer>): Integer pure { 0 }");
    let bounded = check("function f using v(s: Set<Integer>[0, 8]): Integer pure { 0 }");
    assert_ne!(
        parameter_type(&unbounded, "f", 0),
        parameter_type(&bounded, "f", 0)
    );
    let (found, _) = form_and_body(&bounded, parameter_type(&bounded, "f", 0));
    assert_eq!(found, "collection_bounds");
}

/// FR-333-AC-3: an inverted bound refuses `ill_typed`/`type-mismatch`; a
/// non-numeric bound fails at S1 with a diagnostic at the offending token.
#[trace("TC-843", "FR-333-AC-3")]
#[test]
fn an_inverted_bound_refuses_and_a_non_numeric_bound_fails_at_s1() {
    let refusals = try_check("function f using v(s: Set<Integer>[3, 2]): Integer pure { 0 }")
        .expect_err("an inverted bound refuses");
    assert!(is_type_mismatch(&refusals), "{refusals:?}");

    let text = format!("{HEADER}function f using v(s: Set<Integer>[0, x]): Integer pure {{ 0 }}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(!parsed.is_admissible());
    let diagnostic = parsed.diagnostics().first().expect("a parse diagnostic");
    let span = diagnostic.byte_span().expect("a located diagnostic");
    assert_eq!(span.start, text.find("x]").expect("x is in the source"));
}

/// FR-334-AC-1: bound presence is part of type identity, and the bounded
/// node carries `minimum` and `maximum` as the root definition states.
#[trace("TC-844", "FR-334-AC-1")]
#[test]
fn bound_presence_changes_the_node_key_and_the_package_id() {
    let unbounded = "function f using v(s: Set<Integer>): Integer pure { 0 }";
    let bounded =
        "function f using v(s: Set<Integer>[0, 18446744073709551615]): Integer pure { 0 }";
    let (unbounded_graph, bounded_graph) = (check(unbounded), check(bounded));
    let unbounded_type = parameter_type(&unbounded_graph, "f", 0);
    let bounded_type = parameter_type(&bounded_graph, "f", 0);
    assert_ne!(unbounded_type, bounded_type);
    assert_ne!(package_id(unbounded), package_id(bounded));

    let (form, body) = form_and_body(&unbounded_graph, unbounded_type);
    assert_eq!(form, "set");
    assert!(!body.contains("min") && !body.contains("max"), "{body}");
    let (form, body) = form_and_body(&bounded_graph, bounded_type);
    assert_eq!(form, "collection_bounds");
    assert!(body.contains("min") && body.contains("max"), "{body}");
    assert!(body.contains("18446744073709551615"), "{body}");
}

/// FR-334-AC-3: the same bounded unit compiles to the same node keys and
/// `package_id`.
#[trace("TC-844", "FR-334-AC-3")]
#[test]
fn compiling_a_bounded_unit_twice_gives_the_same_identities() {
    let unit = "function f using v(s: Bag<Integer>[1, 5]): Integer pure { 0 }";
    let (first, second) = (check(unit), check(unit));
    assert_eq!(
        parameter_type(&first, "f", 0),
        parameter_type(&second, "f", 0)
    );
    assert_eq!(package_id(unit), package_id(unit));
}

const UNBOUNDED_CLAIM: &str =
    "function f using v(s: Set<Int[0, 9999]>): Integer pure { size(s) + 1 }";

/// The domain key of parameter 0 of `f`.
fn domain_of_parameter(graph: &CheckedGraph) -> DomainKey {
    DomainKey::Node {
        node: WireNodeId::from_digest(*parameter(graph, "f", 0).as_bytes()),
        path: Vec::new(),
    }
}

/// FR-335-AC-1: with an empty registry the claim's item is
/// (`value-validity`, unbounded) with one collection domain at `s`, has no
/// candidate, and its `unsupported` settlement gets no target; the empty
/// candidate set names `value-validity` for the warning.
#[trace("TC-845", "FR-335-AC-1")]
#[test]
fn a_claim_over_an_unbounded_set_has_no_candidate_with_an_empty_registry() {
    let package = package(UNBOUNDED_CLAIM);
    let graph = package.graph();
    let items = items_from_requirements(graph.requirements(), &Registry::new(), None);
    let [item] = items.as_slice() else {
        panic!("one item for the `+`: {items:#?}");
    };
    assert_eq!(item.kind(), Capability::ValueValidity);
    assert_eq!(
        item.extent(),
        ExtentClassification::Unbounded {
            finite_bound_available: true
        }
    );
    assert_eq!(
        item.unbounded(),
        [(domain_of_parameter(graph), DomainKind::Collection)]
    );
    let CandidateOutcome::Candidates(candidates) = item.candidates() else {
        panic!("a computed candidate set");
    };
    assert!(candidates.is_empty());
    assert_eq!(candidates.kind(), Capability::ValueValidity);
    assert_eq!(route(&[Disposition::Unsupported]), vec![None]);
}

/// FR-335-AC-2: with only a bounded-mode backend the item is written with a
/// finite bound available, and its `requires-bound` settlement gets no
/// target.
#[trace("TC-845", "FR-335-AC-2")]
#[test]
fn a_bounded_only_backend_leaves_the_unbounded_item_requiring_a_bound() {
    let package = package(UNBOUNDED_CLAIM);
    let (registry, kani) = registry(Capability::ValueValidity);
    let items = items_from_requirements(package.graph().requirements(), &registry, None);
    let [item] = items.as_slice() else {
        panic!("one item: {items:#?}");
    };
    assert_eq!(
        item.extent(),
        ExtentClassification::Unbounded {
            finite_bound_available: true
        }
    );
    let CandidateOutcome::Candidates(candidates) = item.candidates() else {
        panic!("a computed candidate set");
    };
    assert_eq!(candidates.candidates(), std::slice::from_ref(&kani));
    assert_eq!(route(&[Disposition::RequiresBound]), vec![None]);
}

/// FR-335-AC-3: a bounded request is a new item with its own index,
/// `bounded`, distinct for `maximum: 8` and `maximum: 9`; the unbounded item
/// keeps its classification.
#[trace("TC-845", "FR-335-AC-3")]
#[test]
fn a_bounded_request_is_its_own_item_distinct_per_bound() {
    let package = package(UNBOUNDED_CLAIM);
    let graph = package.graph();
    let records = graph.requirements();
    let (registry, _) = registry(Capability::ValueValidity);
    let items = items_from_requirements(records, &registry, None);
    let [item] = items.as_slice() else {
        panic!("one item: {items:#?}");
    };
    let record = &records[item.occurrence()];
    let s = domain_of_parameter(graph);
    let mut writer = RequestWriter::new();
    let unbounded = writer.item(item.occurrence().clone(), record.requirements());
    let mut bounded = |maximum: u64| {
        writer
            .bounded_item(
                item.occurrence().clone(),
                record.requirements(),
                BTreeMap::from([(s.clone(), FiniteBound::cardinality(maximum))]),
            )
            .expect("one cardinality bound for the collection domain")
    };
    let eight = bounded(8);
    let nine = bounded(9);
    let written = writer.finish();
    assert_eq!(
        [unbounded, eight, nine].map(RequestIndex::get),
        [0, 1, 2],
        "each item has its own request index"
    );
    assert_eq!(
        written[unbounded.get()].extent(),
        ExtentClassification::Unbounded {
            finite_bound_available: true
        }
    );
    let eight = &written[eight.get()];
    assert_eq!(eight.extent(), ExtentClassification::Bounded);
    assert_eq!(eight.occurrence(), item.occurrence());
    assert_eq!(
        eight.domains(),
        [ProofBound::new(
            s,
            Some(DomainKind::Collection),
            FiniteBound::cardinality(8)
        )
        .expect("a cardinality bound pairs with a collection domain")]
    );
    assert_ne!(eight.domains(), written[nine.get()].domains());
    assert_ne!(eight, &written[nine.get()]);
}

/// FR-335-AC-4: a claim with a quantity root beside the collection is
/// written with no finite bound available, and its `unbounded-extent`
/// settlement (`unsupported`) gets no target.
#[trace("TC-845", "FR-335-AC-4")]
#[test]
fn a_quantity_root_leaves_no_finite_bound_available() {
    let package = package(UNBOUNDED_CLAIM);
    let graph = package.graph();
    let s = domain_of_parameter(graph);
    let quantity = DomainKey::Node {
        node: WireNodeId::from_digest([7; 32]),
        path: Vec::new(),
    };
    let requirements = Requirements::new(
        Capability::ValueValidity,
        ClaimExtent::from_domains(
            [(s, DomainKind::Collection), (quantity, DomainKind::Quantity)]
                .into_iter()
                .collect(),
        ),
    );
    let record_occurrence = graph
        .requirements()
        .keys()
        .next()
        .cloned()
        .expect("the claim's record");
    let mut writer = RequestWriter::new();
    writer.item(record_occurrence, &requirements);
    let [item] = writer.items() else {
        panic!("one item");
    };
    assert_eq!(
        item.extent(),
        ExtentClassification::Unbounded {
            finite_bound_available: false
        }
    );
    assert_eq!(route(&[Disposition::Unsupported]), vec![None]);
}
