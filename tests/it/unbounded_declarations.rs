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
use qsl_foundation::{Code, RequestIndex, SourceIdentity};
use qsl_package::CheckedPackage;
use qsl_route::request::{items_from_requirements, ExtentClassification, RequestWriter};
use qsl_route::routing::{route, Disposition};
use qsl_route::{CandidateOutcome, Registry};
use qsl_semantics::check::{
    AssemblyCause, AssemblyRefusal, Capability, CheckedGraph, PackageDeclarations, TypeFormFault,
};
use qsl_semantics::family::{ClaimExtent, Requirements};
use quire_exact::{NodeKey, PathSegment};

use super::request_builder::{check, package, registry, HEADER};

/// `declarations` through S1, S2 and the assembler, whose refusal is returned.
fn try_assemble(declarations: &str) -> Result<PackageDeclarations, AssemblyRefusal> {
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
        "fixture".to_string(),
        unit,
        Vec::new(),
        Vec::new(),
    )
}

/// The parameter node `index` of function `name`.
fn parameter(graph: &CheckedGraph, name: &str, index: usize) -> NodeKey {
    graph
        .semantic_graph()
        .node(
            graph
                .function_identity(name)
                .expect("the function is declared"),
        )
        .and_then(|function| function.function_parameters())
        .expect("a function node")[index]
        .clone()
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

/// The S4 document, including its structural node ids.
fn emitted(graph: CheckedGraph) -> serde_json::Value {
    let emission =
        qsl_package::emit_checked(&CheckedPackage::link(graph)).expect("the checked unit emits");
    assert!(emission.omitted().is_empty(), "every fixture node emits");
    serde_json::from_slice(emission.package().bytes()).expect("a v2 document")
}

/// The only emitted type node with `form` in these single-parameter units.
fn emitted_type<'a>(document: &'a serde_json::Value, form: &str) -> &'a serde_json::Value {
    let nodes: Vec<_> = document["semantic_graph"]["nodes"]
        .as_array()
        .expect("the emitted graph")
        .iter()
        .filter(|node| node["semantic_form"] == form)
        .collect();
    let [node] = nodes.as_slice() else {
        panic!("one {form} type: {nodes:?}");
    };
    node
}

/// A ConfigVersion unit used by the published population type vectors.
fn config_graph(declaration: &str) -> CheckedGraph {
    let packages = qsl_semantics::model::intake::package_input([
        crate::support::config_version_spine::spine::DOMAIN_PACKAGE.as_bytes(),
    ])
    .expect("the ConfigVersion model package loads");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model Config = \"example/config-version\";\n\
         {declaration}\n"
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new(
            "agent-ix",
            "test:fr334-type-vectors",
            "fixture",
            "fixture:1",
        ),
        "types.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the model type unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let raw = parsed.source().reference().clone();
    let unit = build_unit(&parsed).expect("S2 builds the model type unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the ConfigVersion model is admitted");
    PackageDeclarations::assemble(raw, "std610".to_string(), unit, models, Vec::new())
        .expect("the model type unit assembles")
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{declaration}: {refusals:?}"))
}

/// Resolve the compact structural-key references published by QSpec.
fn expand_vector_key(keys: &serde_json::Value, name: &str) -> serde_json::Value {
    let entry = &keys[name];
    let segments = entry["segments"].as_array().expect("vector key segments");
    let expanded: Vec<_> = segments
        .iter()
        .map(|segment| {
            if let Some(reference) = segment.get("node_key_ref").and_then(|value| value.as_str()) {
                expand_vector_key(keys, reference)
            } else {
                segment.clone()
            }
        })
        .collect();
    serde_json::json!({
        "ordinal": entry["ordinal"],
        "package": entry["package"],
        "path": expanded,
    })
}

/// Compare the emitted structural identity with one expanded QSpec key.
fn assert_emitted_vector_key(
    document: &serde_json::Value,
    keys: &serde_json::Value,
    expected: &str,
) {
    let expected_key =
        qsl_foundation::node_wire::node_key_from_json(&expand_vector_key(keys, expected))
            .expect("the published vector key is a structural NodeKey");
    let matches: Vec<_> = document["semantic_graph"]["nodes"]
        .as_array()
        .expect("the emitted semantic graph")
        .iter()
        .filter(|node| {
            qsl_foundation::node_wire::node_key_from_json(&node["node_id"])
                .is_ok_and(|key| key == expected_key)
        })
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "expected one emitted node for {expected_key:?}"
    );
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
    let declaration = "function f using v(s: Set<Integer>[3, 2]): Integer pure { 0 }";
    let refusal = try_assemble(declaration)
        .map(|_| ())
        .expect_err("an inverted bound refuses at assembly");
    let [error] = refusal.errors.as_slice() else {
        panic!("one error: {refusal:?}");
    };
    assert_eq!(
        error.cause,
        AssemblyCause::IllFormedBounds(TypeFormFault::EmptyCardinality)
    );
    assert_eq!(error.cause.code(), Code::IllTyped);
    assert_eq!(error.cause.catalog_code().cause(), "type-mismatch");
    let source = format!("{HEADER}{declaration}\n");
    let start = source
        .find("Set<Integer>[3, 2]")
        .expect("the type is in the source");
    assert_eq!(
        (error.span.start, error.span.end),
        (start, start + "Set<Integer>[3, 2]".len())
    );

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
fn a_collection_bound_adds_minimum_and_maximum_to_the_type_node() {
    let unbounded = "function f using v(s: Set<Integer>): Integer pure { 0 }";
    let bounded =
        "function f using v(s: Set<Integer>[0, 18446744073709551615]): Integer pure { 0 }";
    let (unbounded_graph, bounded_graph) = (check(unbounded), check(bounded));
    let unbounded_type = parameter_type(&unbounded_graph, "f", 0);
    let bounded_type = parameter_type(&bounded_graph, "f", 0);
    let (form, body) = form_and_body(&unbounded_graph, unbounded_type);
    assert_eq!(form, "set");
    assert!(!body.contains("min") && !body.contains("max"), "{body}");
    let (form, body) = form_and_body(&bounded_graph, bounded_type);
    assert_eq!(form, "collection_bounds");
    assert!(body.contains("min") && body.contains("max"), "{body}");
    assert!(body.contains("18446744073709551615"), "{body}");
    let unbounded_key = parameter_type(&unbounded_graph, "f", 0);
    let bounded_key = parameter_type(&bounded_graph, "f", 0);
    assert_ne!(
        unbounded_key, bounded_key,
        "bound presence distinguishes the checked type keys"
    );
    let unbounded = emitted(unbounded_graph);
    let bounded = emitted(bounded_graph);
    let key = |document: &serde_json::Value, node: &serde_json::Value| {
        qsl_semantics::check::wire_id::WireGraph::of_package_value(document)
            .expect("the emitted document supplies its operand-index context")
            .key_of_value(&node["node_id"])
            .expect("an emitted structural node id")
    };
    let emitted_unbounded_key = key(&unbounded, emitted_type(&unbounded, "set"));
    let emitted_bounded_key = key(&bounded, emitted_type(&bounded, "collection_bounds"));
    assert_eq!(
        emitted_unbounded_key, unbounded_key,
        "the emitted unbounded type retains its checked key"
    );
    assert_eq!(
        emitted_bounded_key, bounded_key,
        "the emitted bounded type retains its checked key"
    );
    assert_ne!(
        emitted_unbounded_key, emitted_bounded_key,
        "S4 preserves the bound distinction in its emitted type keys"
    );
    let bounds: BTreeMap<&str, u64> = emitted_type(&bounded, "collection_bounds")["body"]
        ["members"]
        .as_array()
        .expect("the bound's aggregate members")
        .iter()
        .map(|member| {
            let name = member["name"].as_str().expect("a named bound member");
            let value = member["value"]["value"]
                .as_str()
                .expect("an exact integer literal")
                .parse()
                .expect("the bound fits u64");
            (name, value)
        })
        .collect();
    assert_eq!(bounds, BTreeMap::from([("min", 0), ("max", u64::MAX)]));
    assert_ne!(unbounded, bounded, "the emitted package values differ");
}

/// FR-334-AC-3: the same bounded unit compiles to the same node keys.
#[trace("TC-844", "FR-334-AC-3")]
#[test]
fn compiling_a_bounded_unit_twice_gives_the_same_identities() {
    let unit = "function f using v(s: Bag<Integer>[1, 5]): Integer pure { 0 }";
    let (first, second) = (check(unit), check(unit));
    assert_eq!(
        parameter_type(&first, "f", 0),
        parameter_type(&second, "f", 0)
    );
    assert_eq!(emitted(first), emitted(second));
}

const UNBOUNDED_CLAIM: &str =
    "function f using v(s: Set<Int[0, 9999]>): Integer pure { size(s) + 1 }";

/// The domain key of parameter 0 of `f`.
fn domain_of_parameter(graph: &CheckedGraph) -> DomainKey {
    DomainKey::Node {
        node: parameter(graph, "f", 0),
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
    let (registry, kani) = registry(Capability::ValueValidity);
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
        [
            ProofBound::new(s, Some(DomainKind::Collection), FiniteBound::cardinality(8))
                .expect("a cardinality bound pairs with a collection domain")
        ]
    );
    assert_ne!(eight.domains(), written[nine.get()].domains());
    assert_ne!(eight, &written[nine.get()]);
    // Compare the claim and substituted bounds without the request index:
    // different indices alone must not establish different obligations.
    let identity = |index: RequestIndex| {
        let item = &written[index.get()];
        (
            item.occurrence(),
            item.kind(),
            item.extent(),
            item.domains(),
        )
    };
    assert_ne!(identity(unbounded), identity(eight.index()));
    assert_ne!(identity(unbounded), identity(nine));
    assert_ne!(identity(eight.index()), identity(nine));
    let dispositions = [
        Disposition::RequiresBound,
        Disposition::Supported(kani.clone()),
        Disposition::Supported(kani.clone()),
    ];
    let targets = route(&dispositions);
    assert_eq!(targets[unbounded.get()], None);
    assert_eq!(targets[eight.index().get()], Some(&kani));
    assert_eq!(targets[nine.get()], Some(&kani));
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
        node: NodeKey::new("fixture", vec![PathSegment::Text("quantity".into())], 0)
            .expect("a valid key"),
        path: Vec::new(),
    };
    let requirements = Requirements::new(
        Capability::ValueValidity,
        ClaimExtent::from_domains(
            [
                (s, DomainKind::Collection),
                (quantity, DomainKind::Quantity),
            ]
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

/// FR-334 AC4: every collection and population vector is compiled and emitted
/// by QSL, then compared directly by structural node key with QSpec's source
/// vector. The source vectors remain in the QSpec checkout named by QSPEC_DIR.
#[trace(
    "TC-844",
    "FR-334-AC-4",
    "QSpec-FR-144-AC-9",
    "QSpec-FR-144-AC-13",
    "QSpec-FR-153-AC-9"
)]
#[test]
fn emitted_collection_and_population_type_keys_match_qspec_vectors() {
    let Some(qspec) = std::env::var_os("QSPEC_DIR") else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let path = std::path::PathBuf::from(qspec)
        .join("proposals/checked-package-v2/collection-population-type-vectors.json");
    let vectors: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display())),
    )
    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    assert_eq!(
        vectors["version"], "quire.collection-population-type-vectors/v1",
        "the checked vector contract is current"
    );
    let keys = &vectors["node_keys"];

    let collections = vectors["collection_type_cases"]
        .as_array()
        .expect("collection type cases");
    assert!(
        !collections.is_empty(),
        "collection vector coverage is nonzero"
    );
    let mut collection_keys = BTreeMap::new();
    for case in collections {
        let id = case["id"].as_str().expect("collection case id");
        let source_type = case["source_type"]
            .as_str()
            .expect("collection source type");
        let expected = case["expected_node_key"]
            .as_str()
            .expect("collection expected key");
        let graph = config_graph(&format!(
            "function f using v(x: {source_type}): Integer pure {{ 0 }}"
        ));
        let document = emitted(graph);
        assert_emitted_vector_key(&document, keys, expected);
        let expected_key =
            qsl_foundation::node_wire::node_key_from_json(&expand_vector_key(keys, expected))
                .expect("the collection key expands");
        collection_keys.insert(id.to_string(), expected_key);
    }
    assert_ne!(
        collection_keys["CT-01"], collection_keys["CT-02"],
        "unbounded and bounded collection types have distinct structural keys"
    );

    let populations = vectors["population_type_cases"]
        .as_array()
        .expect("population type cases");
    assert!(
        !populations.is_empty(),
        "population vector coverage is nonzero"
    );
    let mut population_keys = BTreeMap::new();
    for case in populations {
        let id = case["id"].as_str().expect("population case id");
        let source_type = case["source_type"]
            .as_str()
            .expect("population source type");
        let expected = case["expected_node_key"]
            .as_str()
            .expect("population expected key");
        let graph = config_graph(&format!(
            "function f using v(x: {source_type}): Integer pure {{ 0 }}"
        ));
        let document = emitted(graph);
        assert_emitted_vector_key(&document, keys, expected);
        let expected_key =
            qsl_foundation::node_wire::node_key_from_json(&expand_vector_key(keys, expected))
                .expect("the population key expands");
        population_keys.insert(id.to_string(), expected_key);
    }
    assert_ne!(
        population_keys["PT-01"], population_keys["PT-02"],
        "unbounded and zero-bounded population types have distinct structural keys"
    );
    assert_ne!(
        population_keys["PT-02"], population_keys["PT-03"],
        "different population maxima have distinct structural keys"
    );
    println!(
        "conformance: {} collection and {} population type vector keys emitted and matched",
        collections.len(),
        populations.len()
    );
}
