// SPDX-License-Identifier: AGPL-3.0-or-later
//! Behavioural tests of the S4 v2 emission arm (TC-416, FR-093; FR-065-AC-2
//! through QSL's full I2 read). Every emitted package is read back through
//! `read_checked_package_v2`, which runs IR's v2 reader, and the checks
//! below are made on the bytes it admitted.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ix_trace_rs::trace;
use qsl_forms::{
    BinaryOperator, BuiltinType, DeclarationSpans, Expression, ExpressionSpans,
    FunctionDeclaration, TypeForm,
};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::{OccurrenceKey, RawSourceRef, SourceRegion};
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::library::{LibraryName, PinnedRequest};
use qsl_semantics::value::{native_diagnostics_catalog, CatalogRole, DefinitionLock};
use quire_contract_model::CheckedPackageEvidence;
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, NodeKey, Presence, Role, ValueType,
    NODE_KEY_DOMAIN,
};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use super::*;
use crate::checked_v2::{read_v2, Read, V2ReadLimits};

mod admission_corpus;
mod deep_bodies;
mod golden;
mod identity_depth;
mod owners;

/// The unit the fixture packages are read from; every occurrence's region is
/// the whole of it.
const TEXT: &[u8] = b"function t using v(): Boolean pure { if true then true else true }";

const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

/// The fixture unit admitted as (`a`, `u`, `git`, `1`): its owner `(a, u)`
/// is the one FR-092's golden vectors are keyed under.
pub(super) fn source() -> RawSourceRef {
    qsl_semantics::check::admitted_source(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        TEXT,
    )
}

/// Places every occurrence at the whole fixture unit.
pub(super) fn whole_unit(_: &Location) -> Option<SourceRegion> {
    Some(SourceRegion::new(source(), 0, TEXT.len() as u64).unwrap())
}

fn boolean() -> TypeForm {
    TypeForm::builtin(BuiltinType::Boolean, SPAN)
}

fn function(name: &str, parameters: &[&str], body: Expression) -> FunctionDeclaration {
    FunctionDeclaration::new(
        name,
        parameters
            .iter()
            .map(|parameter| ((*parameter).to_owned(), boolean()))
            .collect(),
        boolean(),
        None,
        body,
    )
}

fn name(name: &str) -> Expression {
    Expression::name(name.to_owned())
}

/// FR-093-AC-1's `t`: `if true then true else true`.
fn t() -> FunctionDeclaration {
    function(
        "t",
        &[],
        Expression::if_then_else(
            Expression::boolean(true),
            Expression::boolean(true),
            Expression::boolean(true),
        ),
    )
}

/// FR-092's `f`: `true`.
fn f() -> FunctionDeclaration {
    function("f", &[], Expression::boolean(true))
}

/// FR-092's `both(a, b)`: `a and b`.
fn both() -> FunctionDeclaration {
    function(
        "both",
        &["a", "b"],
        Expression::binary(BinaryOperator::And, name("a"), name("b")),
    )
}

/// FR-093's `nb(a)`: `both(a, true)`.
fn nb() -> FunctionDeclaration {
    function(
        "nb",
        &["a"],
        Expression::call(
            "both".to_owned(),
            vec![name("a"), Expression::boolean(true)],
        ),
    )
}

/// FR-093's `h(a)`: `let y = a in y`.
fn h() -> FunctionDeclaration {
    function(
        "h",
        &["a"],
        Expression::let_in("y".to_owned(), name("a"), name("y")),
    )
}

fn graph_of(functions: Vec<FunctionDeclaration>) -> qsl_semantics::check::CheckedGraph {
    PackageDeclarations {
        functions,
        ..PackageDeclarations::new(source(), qsl_foundation::IdentityLimits::default())
    }
    .check(CheckingLimits::default())
    .expect("the fixture functions check")
}

fn package(functions: Vec<FunctionDeclaration>) -> CheckedPackage {
    CheckedPackage::link(graph_of(functions))
}

/// `record Tree { kids: Sequence<Tree>[0, 3]; }`, FR-092's G7 to G9.
fn tree() -> CheckedPackage {
    let tree = NodeKey::from_digest([6; 32]);
    let kids = ValueType::collection(CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Composite(tree),
        Some(CardinalityBound::new(0, 3).unwrap()),
    ));
    let types = TypeEnvironment::new(
        [CompositeDeclaration::new(
            tree,
            "Tree",
            CompositeShape::Record(vec![FieldDeclaration::new(
                "kids",
                kids,
                Presence::Required,
            )]),
        )],
        [],
    )
    .expect("FR-143 admits Tree");
    CheckedPackage::link(
        PackageDeclarations {
            types,
            ..PackageDeclarations::new(source(), qsl_foundation::IdentityLimits::default())
        }
        .check(CheckingLimits::default())
        .expect("Tree checks"),
    )
}

/// `package` emitted with every occurrence placed at the whole of the unit it
/// was checked from, so each declared node's regions name its owner's source.
fn emit(package: &CheckedPackage) -> Emission {
    emit_under(package, package.graph().source())
}

/// `package` emitted with every occurrence placed at the whole of `source`.
fn emit_under(package: &CheckedPackage, source: &RawSourceRef) -> Emission {
    let source = source.clone();
    emit_package(package, move |_| {
        Some(SourceRegion::new(source.clone(), 0, TEXT.len() as u64).unwrap())
    })
    .expect("the package emits")
}

pub(super) fn wire(emission: &Emission) -> Value {
    serde_json::from_slice(emission.package.bytes()).expect("the wire is JSON")
}

fn jcs(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn library() -> LibraryName {
    LibraryName::new("pkg").unwrap()
}

/// QSL's full I2 read of `emission`, pinned at its own `package_id`, with
/// the emitted lock's required features as evidence.
fn read_back(emission: &Emission) -> Read {
    read_with(emission, &read_evidence(emission))
}

/// The emitted lock's required features as evidence.
fn read_evidence(emission: &Emission) -> CheckedPackageEvidence {
    let wire = wire(emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
}

/// QSL's full I2 read of `emission` against `evidence`, pinned at its own
/// `package_id`.
fn read_with(emission: &Emission, evidence: &CheckedPackageEvidence) -> Read {
    let pinned: PinnedRequest =
        qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id());
    read_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        evidence,
        &pinned,
    )
}

/// The verified package's exports: declared name to node id hex.
fn verified_exports(emission: &Emission) -> BTreeMap<String, String> {
    match read_back(emission) {
        Read::Verified { package, .. } => {
            assert_eq!(package.package_id(), emission.package.package_id());
            package
                .into_import_view()
                .exports()
                .map(|(name, key)| (name.to_owned(), key.node.to_string()))
                .collect()
        }
        other => panic!("expected Verified, got {other:?}"),
    }
}

pub(super) fn nodes(wire: &Value) -> &[Value] {
    wire["semantic_graph"]["nodes"].as_array().unwrap()
}

/// FR-065-AC-2 (TC-163): a function's checked identity is one value right
/// after `check`, after S4 linking, and after QSL's full I2 read of the
/// emitted v2 bytes (IR's reader included), which returns Verified.
/// Reordering the package's other declaration leaves it unchanged at all
/// three points.
#[trace("FR-065-AC-2", "TC-163")]
#[test]
fn a_function_identity_survives_emission_and_the_i2_read() {
    let checked = |functions| {
        PackageDeclarations {
            functions,
            ..PackageDeclarations::new(source(), qsl_foundation::IdentityLimits::default())
        }
        .check(CheckingLimits::default())
        .expect("t and f check")
    };
    let mut identities = BTreeSet::new();
    for functions in [vec![t(), f()], vec![f(), t()]] {
        let graph = checked(functions);
        let after_check = graph.function_identity("t").expect("t is declared");
        let package = CheckedPackage::link(graph);
        let after_link = package
            .graph()
            .function_identity("t")
            .expect("t is declared");
        let emission = emit(&package);
        assert_eq!(emission.omitted, []);
        let after_read = verified_exports(&emission)["t"].clone();
        assert_eq!(after_check, after_link);
        assert_eq!(after_read, after_check.to_string());
        identities.insert(after_read);
    }
    assert_eq!(identities.len(), 1, "reordering changed t's identity");
}

/// The wire lock's edition and definition selections are the
/// `DefinitionLock` catalog's rows, each exactly `{authority, identity}`;
/// the diagnostics catalog is QSpec's native diagnostics `DefinitionRef`,
/// `{agent-ix, quire.native.diagnostics/v1}` and no other member; each
/// source row, and each source-map region's source, keeps its
/// `quire.source.bytes/v1` digest; and IR's reader admits the package.
#[trace("TC-416", "FR-093-AC-7", "FR-093-AC-17", "FR-093-AC-20")]
#[test]
fn the_lock_selects_the_catalog_definitions() {
    let emission = emit(&package(vec![t()]));
    assert!(matches!(read_back(&emission), Read::Verified { .. }));
    let wire = wire(&emission);
    let lock = DefinitionLock::pinned();
    let row = |role: CatalogRole| {
        let entry = lock.entry(role);
        json!({"authority": entry.authority, "identity": entry.identity})
    };
    assert_eq!(
        wire["lock"]["edition"],
        json!({"role": "edition", "definition": row(CatalogRole::Edition)})
    );
    let expected: Vec<Value> = lock
        .always_roles()
        .iter()
        .filter(|role| **role != CatalogRole::Edition)
        .map(|role| row(*role))
        .collect();
    assert_eq!(wire["lock"]["definition_selections"], json!(expected));
    assert_eq!(wire["lock"]["profile_selections"], json!([]));
    assert_eq!(wire["lock"]["dependency_selections"], json!([]));
    let source = json!({
        "authority": "a",
        "identity": "u",
        "digest_domain": "quire.source.bytes/v1",
        "digest": sha256_hex(TEXT),
    });
    assert_eq!(wire["lock"]["sources"], json!([source]));
    for entry in wire["source_map"].as_array().unwrap() {
        for region in entry["regions"].as_array().unwrap() {
            assert_eq!(region["source"], source);
        }
    }
    assert_eq!(
        wire["identity_preimage"]["edition"],
        wire["lock"]["edition"]
    );
    assert_eq!(
        wire["identity_preimage"]["definition_selections"],
        wire["lock"]["definition_selections"]
    );
    let diagnostics = native_diagnostics_catalog();
    assert_eq!(
        wire["diagnostics"]["catalog"],
        json!({"authority": diagnostics.authority, "identity": diagnostics.identity})
    );
    assert_eq!(
        wire["diagnostics"]["catalog"],
        json!({"authority": "agent-ix", "identity": "quire.native.diagnostics/v1"})
    );
}

/// An operation law's `definition` is the catalog row of its role, exactly
/// `{authority, identity}`, and is one of the lock's `definition_selections`
/// rows; IR's reader admits the package (IR FR-038's law join).
#[trace("TC-416", "FR-093-AC-20")]
#[test]
fn a_law_names_its_definition_by_authority_and_identity() {
    let emission = emit(&golden::float_add_of(BuiltinType::Float64, "nearest-even"));
    assert!(matches!(read_back(&emission), Read::Verified { .. }));
    let wire = wire(&emission);
    let ieee = DefinitionLock::pinned()
        .entry(CatalogRole::IeeeProfile)
        .reference();
    let ieee = json!({"authority": ieee.authority, "identity": ieee.identity});
    let laws: Vec<&Value> = wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["body"]["term"] == "application")
        .flat_map(|node| node["body"]["operation"]["laws"].as_array().unwrap())
        .collect();
    assert!(!laws.is_empty(), "the float addition carries its IEEE law");
    for law in laws {
        assert_eq!(law["role"], json!("ieee_profile"));
        assert_eq!(law["definition"], ieee);
    }
    assert!(wire["lock"]["definition_selections"]
        .as_array()
        .unwrap()
        .contains(&ieee));
}

/// An emitted v2 artifact identical except that one `application` argument is
/// an inline `application` term is refused as `malformed_wire`, and no limit
/// is reported.
#[trace("TC-739", "FR-264-AC-3")]
#[test]
fn an_inline_nested_application_argument_is_a_malformed_wire() {
    let emission = emit(&golden::float_add_of(BuiltinType::Float64, "nearest-even"));
    assert!(matches!(read_back(&emission), Read::Verified { .. }));
    let mut tampered = wire(&emission);
    let nodes = tampered["semantic_graph"]["nodes"].as_array_mut().unwrap();
    let node = nodes
        .iter_mut()
        .find(|node| node["body"]["term"] == "application")
        .expect("the float addition is an application node");
    let inline = node["body"].clone();
    node["body"]["arguments"][0] = inline;
    let pinned: PinnedRequest =
        qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id());
    let outcome = read_v2(
        &jcs(&tampered),
        library(),
        V2ReadLimits::default(),
        &read_evidence(&emission),
        &pinned,
    );
    match outcome {
        Read::Refused(crate::checked_v2::V2ReadRefusal::Envelope { refusal, .. }) => {
            assert_eq!(
                refusal.code,
                quire_contract_model::CheckedPackageRefusalCode::MalformedWire
            );
        }
        other => panic!("expected Refused(Envelope(MalformedWire)), got {other:?}"),
    }
}

/// FR-322's `application_node_preimage` of a wire node, or FR-092's
/// structural preimage under the owner the wire node's own `owner` member
/// names, rebuilt by the test from the wire alone. `group` is the node's recursion group in graph order.
pub(crate) fn rebuilt_key(node: &Value, group: &[&Value]) -> String {
    sha256_hex(&jcs(&rebuilt_preimage(node, group, true)))
}

/// The preimage [`rebuilt_key`] hashes. With `labelled` false, a recursion
/// group member's preimage omits `recursion.group`: its group-local form,
/// whose digests the group label hashes. The owner is read from the wire
/// node's own `owner` member, never from the owner the checked unit carries.
pub(super) fn rebuilt_preimage(node: &Value, group: &[&Value], labelled: bool) -> Value {
    let ordinal = |id: &Value| group.iter().position(|member| member["node_id"] == *id);
    let in_group = |term: &Value| -> Value {
        fn walk(term: &Value, ordinal: &dyn Fn(&Value) -> Option<usize>) -> Value {
            match term {
                Value::Object(members) => {
                    if members.get("term") == Some(&json!("reference")) {
                        if let Some(position) = ordinal(&members["target"]) {
                            return json!({"term": "group_reference", "ordinal": position});
                        }
                    }
                    Value::Object(
                        members
                            .iter()
                            .map(|(key, value)| (key.clone(), walk(value, ordinal)))
                            .collect(),
                    )
                }
                Value::Array(items) => {
                    Value::Array(items.iter().map(|item| walk(item, ordinal)).collect())
                }
                other => other.clone(),
            }
        }
        walk(term, &ordinal)
    };
    let body = in_group(&node["body"]);
    let application = serde_json::to_string(&body)
        .unwrap()
        .contains("\"application\"");
    let declaration = node.get("declaration").cloned().unwrap_or(Value::Null);
    let recursion = node.get("recursion_group").map(|label| {
        let position = ordinal(&node["node_id"]).unwrap();
        if application || !labelled {
            json!({"ordinal": position, "size": group.len()})
        } else {
            json!({"group": label, "ordinal": position, "size": group.len()})
        }
    });
    let semantic_type = if node["semantic_type"] == node["node_id"] && !application {
        Value::Null
    } else {
        match ordinal(&node["semantic_type"]) {
            Some(position) => json!({"term": "group_reference", "ordinal": position}),
            None => node["semantic_type"].clone(),
        }
    };
    let mut preimage = json!({
        "version": if application { "quire.application-node/v1" } else { "quire.structural-node/v1" },
        "node_tag": node["node_tag"],
        "semantic_form": node["semantic_form"],
        "semantic_type": semantic_type,
        "declaration": declaration,
        "recursion": recursion.unwrap_or(Value::Null),
        "body": body,
    });
    if !application {
        if let Some(owner) = node.get("owner") {
            preimage["owner"] = owner.clone();
        }
    }
    preimage
}

/// Each node with the members of its recursion group in graph order.
fn with_groups(nodes: &[Value]) -> Vec<(&Value, Vec<&Value>)> {
    nodes
        .iter()
        .map(|node| {
            let group = match node.get("recursion_group") {
                Some(label) => nodes
                    .iter()
                    .filter(|member| member.get("recursion_group") == Some(label))
                    .collect(),
                None => Vec::new(),
            };
            (node, group)
        })
        .collect()
}

/// `f(x: Int[0, 9])`, FR-092's self-recursive G4 to G6.
fn recursive_f() -> CheckedPackage {
    let recursive = FunctionDeclaration::new(
        "f",
        vec![(
            "x".to_owned(),
            TypeForm::builtin(BuiltinType::Int, SPAN)
                .with_bounds(vec!["0".to_owned(), "9".to_owned()]),
        )],
        boolean(),
        Some(name("x")),
        Expression::if_then_else(
            Expression::binary(
                BinaryOperator::Greater,
                name("x"),
                Expression::integer(0_i64),
            ),
            Expression::call(
                "f".to_owned(),
                vec![Expression::binary(
                    BinaryOperator::Subtract,
                    name("x"),
                    Expression::integer(1_i64),
                )],
            ),
            Expression::boolean(true),
        ),
    );
    package(vec![recursive])
}

/// Every node of `package` as the emission arm writes it, in graph order,
/// including any node the emitter would omit.
fn written_nodes(package: &CheckedPackage) -> Vec<Value> {
    let graph = package.graph();
    let mut recorded = recorded_occurrences(graph).expect("every role is FR-322's");
    let candidates: Vec<Candidate<'_>> = graph
        .semantic_graph()
        .nodes()
        .map(|node| Candidate::of(node, recorded.remove(&node.key()).unwrap_or_default()))
        .collect();
    let all: Vec<&Candidate<'_>> = candidates.iter().collect();
    graph_order(&all)
        .into_iter()
        .map(|candidate| serde_json::to_value(candidate.wire_node().unwrap()).unwrap())
        .collect()
}

/// The packages of TC-416 steps 1, 5 and 6.
fn tc_416_packages() -> [CheckedPackage; 3] {
    [
        package(vec![both(), nb(), h(), f(), t()]),
        recursive_f(),
        tree(),
    ]
}

/// TC-416 steps 1, 2 and 5 (FR-093-AC-7): each node the emission arm writes,
/// rebuilt from the written node by FR-322's application rule or FR-092's
/// structural rule, keys to its `node_id`, recursion-group members by their
/// place in graph order.
#[trace("FR-093-AC-7", "TC-416")]
#[test]
fn every_written_node_recomputes_to_its_node_id() {
    for package in tc_416_packages() {
        let nodes = written_nodes(&package);
        assert!(!nodes.is_empty());
        for (node, group) in with_groups(&nodes) {
            assert_eq!(
                json!(rebuilt_key(node, &group)),
                node["node_id"]["digest"],
                "{node}"
            );
        }
    }
}

/// TC-416 step 5 (FR-093-AC-7): a recursion group's members carry the group
/// digest as their label and are written together in ordinal order: the
/// recursive `f`'s three members under FR-092's label, and `Tree`'s three.
#[trace("FR-093-AC-7", "TC-416")]
#[test]
fn recursion_group_members_are_written_in_ordinal_order() {
    for (package, label) in [
        (
            recursive_f(),
            Some("0b9e8d18320d0ce587699e40ac33a25fd41c4a640226bda4b8b1521edc5e4c50"),
        ),
        (tree(), None),
    ] {
        let expected: BTreeMap<String, usize> = package
            .graph()
            .semantic_graph()
            .nodes()
            .filter_map(|node| {
                node.recursion()
                    .map(|group| (node.key().to_string(), group.ordinal()))
            })
            .collect();
        let nodes = written_nodes(&package);
        let members: Vec<(usize, &Value)> = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.get("recursion_group").is_some())
            .collect();
        assert_eq!(members.len(), 3);
        assert_eq!(members[2].0 - members[0].0, 2, "the members are contiguous");
        for (ordinal, (_, member)) in members.iter().enumerate() {
            assert_eq!(
                expected[member["node_id"]["digest"].as_str().unwrap()],
                ordinal
            );
            assert_eq!(member["recursion_group"], members[0].1["recursion_group"]);
        }
        if let Some(label) = label {
            assert_eq!(members[0].1["recursion_group"], json!(label));
        }
    }
}

/// FR-093 "Node dependencies" rules 1 to 3, rebuilt from a written node by a
/// test-side walker: body reference targets, operation member declarations,
/// and a `bounded_domain` node's semantic type.
fn rebuilt_dependencies(node: &Value) -> Vec<Value> {
    fn walk(term: &Value, found: &mut BTreeSet<String>) {
        match term {
            Value::Object(members) => {
                if members.get("term") == Some(&json!("reference")) {
                    found.insert(members["target"]["digest"].as_str().unwrap().to_owned());
                }
                if let Some(declaration) = members
                    .get("operation")
                    .and_then(|operation| operation.get("member"))
                    .and_then(|member| member.get("declaration"))
                {
                    found.insert(declaration["digest"].as_str().unwrap().to_owned());
                }
                members.values().for_each(|value| walk(value, found));
            }
            Value::Array(items) => items.iter().for_each(|item| walk(item, found)),
            _ => {}
        }
    }
    let mut found = BTreeSet::new();
    walk(&node["body"], &mut found);
    if node["node_tag"] == "bounded_domain" {
        found.insert(node["semantic_type"]["digest"].as_str().unwrap().to_owned());
    }
    found
        .into_iter()
        .map(|digest| json!({"domain": "quire.checked-semantic-node/v1", "digest": digest}))
        .collect()
}

/// TC-416 step 6 (FR-093-AC-12): every written node's `dependencies` equal
/// the list the rules rebuild from the node as written. `Tree`'s three group
/// members each list exactly one other member, and together form one
/// cycle (G7 lists G9, G8 lists G7, G9 lists G8).
#[trace("FR-093-AC-12", "TC-416")]
#[test]
fn dependencies_are_the_lists_the_rules_rebuild() {
    for package in tc_416_packages() {
        for node in written_nodes(&package) {
            assert_eq!(
                node["dependencies"],
                json!(rebuilt_dependencies(&node)),
                "{node}"
            );
        }
    }
    let nodes = written_nodes(&tree());
    let members: Vec<&Value> = nodes
        .iter()
        .filter(|node| node.get("recursion_group").is_some())
        .collect();
    let digest = |id: &Value| id["digest"].as_str().unwrap().to_owned();
    let ids: BTreeSet<String> = members
        .iter()
        .map(|node| digest(&node["node_id"]))
        .collect();
    let mut next = BTreeMap::new();
    for member in &members {
        let dependencies = member["dependencies"].as_array().unwrap();
        assert_eq!(dependencies.len(), 1, "{member}");
        assert!(ids.contains(&digest(&dependencies[0])));
        next.insert(digest(&member["node_id"]), digest(&dependencies[0]));
    }
    let start = digest(&members[0]["node_id"]);
    let mut at = start.clone();
    for _ in 0..3 {
        at = next[&at].clone();
    }
    assert_eq!(at, start, "the group's dependencies form one cycle");
}

/// TC-416 step 4 (FR-093-AC-9): every emitted node has at least one
/// occurrence, each is exactly an occurrence `check` recorded, and each
/// maps to its region. `t`'s literal `true` has three `expression`
/// occurrences.
#[trace("FR-093-AC-9", "TC-416")]
#[test]
fn every_emitted_node_has_its_recorded_occurrences() {
    let package = package(vec![t(), f()]);
    let emission = emit(&package);
    let wire = wire(&emission);
    let recorded: BTreeSet<(String, String, u64)> = package
        .graph()
        .occurrences()
        .map(|(key, origin, _)| (key.to_string(), origin.role().to_string(), origin.ordinal()))
        .collect();
    let mut literal_expressions = 0;
    for node in nodes(&wire) {
        let occurrences = node["occurrences"].as_array().unwrap();
        assert!(!occurrences.is_empty(), "{node}");
        for occurrence in occurrences {
            let key = (
                node["node_id"]["digest"].as_str().unwrap().to_owned(),
                occurrence["role"].as_str().unwrap().to_owned(),
                occurrence["ordinal"].as_u64().unwrap(),
            );
            assert!(recorded.contains(&key), "{key:?}");
        }
        if node["semantic_form"] == "literal" && node["body"]["value"] == json!(true) {
            literal_expressions += occurrences
                .iter()
                .filter(|occurrence| occurrence["role"] == "expression")
                .count();
        }
    }
    assert_eq!(literal_expressions, 4, "three in t, one in f");
    let entries = wire["source_map"].as_array().unwrap();
    let occurrences: usize = nodes(&wire)
        .iter()
        .map(|node| node["occurrences"].as_array().unwrap().len())
        .sum();
    assert_eq!(entries.len(), occurrences);
    for entry in entries {
        assert_eq!(
            entry["regions"],
            json!([{
                "source": wire["lock"]["sources"][0],
                "start": 0,
                "end": TEXT.len(),
            }])
        );
    }
}

/// IR-280: IR's v2 vocabulary holds `value`/`parameter` (FR-092). A
/// package holding `both(a, b)` and `t` is written with nothing omitted,
/// `both`'s two parameter nodes included, and reads back Verified with both
/// functions exported.
#[trace("FR-093-AC-7", "TC-416")]
#[test]
fn a_function_with_parameters_is_written_whole() {
    let package = package(vec![both(), t()]);
    let both_key = package.graph().function_identity("both").unwrap();
    let emission = emit(&package);
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let parameters = nodes(&wire)
        .iter()
        .filter(|node| node["node_tag"] == "value" && node["semantic_form"] == "parameter")
        .count();
    assert_eq!(parameters, 2, "a and b");
    let exports = verified_exports(&emission);
    assert_eq!(exports.get("both"), Some(&both_key.to_string()));
    assert!(exports.contains_key("t"));
}

/// `Int[0, 9]`.
fn int_0_9() -> ValueType {
    ValueType::Int(quire_exact::IntegerInterval::new(0_i64.into(), 9_i64.into()).unwrap())
}

/// `record Point { x: Int[0, 9]; y: Int[0, 9]; }` and
/// `tuple Pair(Int[0, 9], Int[0, 9]);`, FR-092's D1 and D5.
fn point_and_pair_types() -> TypeEnvironment {
    TypeEnvironment::new(
        [
            CompositeDeclaration::new(
                NodeKey::from_digest([1; 32]),
                "Point",
                CompositeShape::Record(vec![
                    FieldDeclaration::new("x", int_0_9(), Presence::Required),
                    FieldDeclaration::new("y", int_0_9(), Presence::Required),
                ]),
            ),
            CompositeDeclaration::new(
                NodeKey::from_digest([2; 32]),
                "Pair",
                CompositeShape::Tuple(vec![int_0_9(), int_0_9()]),
            ),
        ],
        [],
    )
    .expect("FR-143 admits Point and Pair")
}

/// A package declaring [`point_and_pair_types`], `enums` and `functions`.
fn declared_types(
    enums: Vec<qsl_semantics::check::EnumBinding>,
    functions: Vec<FunctionDeclaration>,
) -> CheckedPackage {
    CheckedPackage::link(
        PackageDeclarations {
            types: point_and_pair_types(),
            enums,
            functions,
            ..PackageDeclarations::new(source(), qsl_foundation::IdentityLimits::default())
        }
        .check(CheckingLimits::default())
        .expect("the declared types check"),
    )
}

/// The written node declaring `name`.
fn declared<'w>(wire: &'w Value, name: &str) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| node["declaration"]["qualified_name"] == json!([name]))
        .unwrap_or_else(|| panic!("{name} is written"))
}

/// FR-322: `check` records a `declaration`
/// occurrence for a declared record and tuple, located at the declared
/// name, so both are written with their `declaration` and read back
/// Verified: D1 and D5, exported under their declared names.
#[trace("FR-092-AC-9", "TC-416")]
#[test]
fn a_record_and_a_tuple_are_written_with_their_declarations() {
    let package = declared_types(Vec::new(), Vec::new());
    for name in ["Point", "Pair"] {
        let site = Location::root(quire_semantic_value::location::Origin::TypeDeclaration {
            name: name.to_owned(),
        });
        assert!(
            package
                .graph()
                .occurrences()
                .any(
                    |(_, origin, location)| origin.role().as_str() == "declaration"
                        && *location == site
                ),
            "{name} has a declaration occurrence at its name"
        );
    }
    let emission = emit(&package);
    assert_eq!(emission.omitted, []);
    let exports = verified_exports(&emission);
    let wire = wire(&emission);
    for (name, form, digest) in [
        (
            "Point",
            "record",
            "45ff50317a846ffbc0853f4a4837507f244a3d302d0fa8605a7edf36da532ae4",
        ),
        (
            "Pair",
            "tuple",
            "e519e1b5b543cfa0cf021c9e489d6d5bac5d13b6545a124e8247200195aed560",
        ),
    ] {
        let node = declared(&wire, name);
        assert_eq!(node["semantic_form"], form);
        assert_eq!(node["node_id"]["digest"], digest);
        assert_eq!(
            node["occurrences"],
            json!([{"role": "declaration", "ordinal": 0}]),
            "{name}"
        );
        assert_eq!(exports.get(name).map(String::as_str), Some(digest));
    }
}

/// FR-322's declaration rule, as the emitter applies it: a record that
/// carries `declaration` with no `declaration` occurrence is omitted with
/// `DeclarationOccurrenceMismatch`, and the tuple is still written. `check`
/// always records the occurrence, so the test drops it from the recorded
/// set the emitter reads.
#[trace("TC-416")]
#[test]
fn a_declaration_without_its_occurrence_is_omitted() {
    let package = declared_types(Vec::new(), Vec::new());
    let graph = package.graph();
    let mut recorded = recorded_occurrences(graph).expect("every role is FR-322's");
    let point = graph
        .semantic_graph()
        .nodes()
        .find(|node| node.semantic_form() == "record")
        .expect("Point's node");
    let tuple = graph
        .semantic_graph()
        .nodes()
        .find(|node| node.semantic_form() == "tuple")
        .expect("Pair's node");
    let candidates: BTreeMap<CheckedNodeId, Candidate<'_>> = graph
        .semantic_graph()
        .nodes()
        .map(|node| {
            let mut occurrences = recorded.remove(&node.key()).unwrap_or_default();
            if node.key() == point.key() {
                occurrences.retain(|(occurrence, _)| {
                    occurrence.role != CheckedOccurrenceRole::Declaration
                });
            }
            let candidate = Candidate::of(node, occurrences);
            (candidate.id.clone(), candidate)
        })
        .collect();
    let omitted = omissions(&candidates, graph.source());
    assert_eq!(
        omitted,
        [OmittedNode {
            node: node_id(point.key()),
            cause: OmissionCause::DeclarationOccurrenceMismatch,
        }]
    );
    assert!(!omitted
        .iter()
        .any(|omission| omission.node == node_id(tuple.key())));
}

/// A type declared by hand carries no span of its name (only the FR-091
/// assembler records one), so `emit_checked` cannot place its
/// `declaration` occurrence and refuses a package declaring one.
#[trace("TC-426")]
#[test]
fn emit_checked_cannot_place_a_type_declaration() {
    assert!(matches!(
        emit_checked(&declared_types(Vec::new(), Vec::new())),
        Err(EmitRefusal::UnlocatedOccurrence { .. })
    ));
}

/// A declared name segment that is not an identifier refuses naming that
/// segment, not an empty qualified name.
#[trace("TC-416")]
#[test]
fn a_declared_name_segment_that_is_no_identifier_refuses_by_that_segment() {
    let types = TypeEnvironment::new(
        [CompositeDeclaration::new(
            NodeKey::from_digest([3; 32]),
            "Geo::1Point",
            CompositeShape::Tuple(vec![int_0_9()]),
        )],
        [],
    )
    .expect("the environment admits the shape");
    let refusals = PackageDeclarations {
        types,
        ..PackageDeclarations::new(source(), qsl_foundation::IdentityLimits::default())
    }
    .check(CheckingLimits::default())
    .expect_err("the name has no identifier segments");
    assert!(
        refusals.iter().any(|refusal| refusal.cause
            == qsl_semantics::check::CheckCause::NodePreimage(
                qsl_semantics::check::NodeKeyRefusal::InvalidQualifiedNameSegment {
                    segment: "1Point".to_owned(),
                }
            )),
        "{refusals:?}"
    );
}

/// Two enum bindings of one admitted declaration are one node with one
/// `declaration` occurrence.
#[trace("TC-416")]
#[test]
fn one_enum_declaration_bound_twice_is_declared_once() {
    let owner = || json!({"kind": "source", "authority": "a", "identity": "u"});
    let first = status_enum(owner(), "Status");
    let second = status_enum(owner(), "Status");
    let key = first.declaration.key();
    assert_eq!(key, second.declaration.key());
    let package = declared_types(vec![first, second], Vec::new());
    let declarations = package
        .graph()
        .occurrences()
        .filter(|(node, origin, _)| *node == key && origin.role().as_str() == "declaration")
        .count();
    assert_eq!(declarations, 1);
}

/// `record Tree { kids: Sequence<Tree>[0, 3]; }`'s recursion group
/// is written whole: the record with its `declaration` and occurrence, its
/// sequence and its bound, and the package reads back Verified.
#[trace("TC-416")]
#[test]
fn a_recursive_record_is_written_with_its_declaration() {
    let package = tree();
    let emission = emit(&package);
    assert_eq!(emission.omitted, []);
    assert!(verified_exports(&emission).contains_key("Tree"));
    let wire = wire(&emission);
    let record = declared(&wire, "Tree");
    assert_eq!(
        record["occurrences"],
        json!([{"role": "declaration", "ordinal": 0}])
    );
    let members = nodes(&wire)
        .iter()
        .filter(|node| node.get("recursion_group").is_some())
        .count();
    assert_eq!(members, 3);
}

/// An enum declaration preimage under `owner`, and its admitted
/// declaration and members, bound as `name`.
fn status_enum(owner: Value, name: &str) -> qsl_semantics::check::EnumBinding {
    use qsl_semantics::value::enumeration::{
        AdmittedEnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage,
    };
    use qsl_semantics::value::{NodeIdentityPreimage, NodeOwner, OwnerSelection};

    let preimage = EnumDeclarationPreimage::from_json(json!({
        "version": "quire.enum-declaration-node/v1",
        "owner": owner,
        "qualified_declaration": [name],
        "ordered": true,
        "members": ["Ready", "Done"],
    }))
    .expect("a schema-valid preimage");
    let owners = OwnerSelection::new([NodeOwner::clone(preimage.owner())]);
    let key = NodeKey::from_digest(
        preimage
            .digest(qsl_foundation::IdentityLimits::default())
            .unwrap(),
    );
    let declaration = AdmittedEnumDeclaration::admit(
        preimage,
        key,
        &owners,
        qsl_foundation::IdentityLimits::default(),
    )
    .expect("admitted");
    let members = ["Ready", "Done"]
        .into_iter()
        .map(|case| {
            let member = EnumMemberPreimage::from_json(json!({
                "version": "quire.enum-member-node/v1",
                "declaration_node_id": {"domain": NODE_KEY_DOMAIN, "digest": key.to_string()},
                "case": case,
            }))
            .unwrap();
            let member_key = NodeKey::from_digest(
                member
                    .digest(qsl_foundation::IdentityLimits::default())
                    .unwrap(),
            );
            declaration
                .admit_member(
                    &member,
                    member_key,
                    qsl_foundation::IdentityLimits::default(),
                )
                .unwrap()
        })
        .collect();
    qsl_semantics::check::EnumBinding {
        name: name.to_owned(),
        declaration,
        members,
    }
}

/// QSL-238 (FR-092 rule 1): lowering builds the enum declaration and member
/// nodes it names by key. A package with a record, a tuple, an enum
/// `Status` owned by the checked unit, `ready(): Status { Status::Ready }`
/// (an enum literal) and `keep(s: Status): Status { s }` (an enum-typed
/// parameter) reads back Verified: IR re-derives each nominal node's key
/// from its `nominal_identity_preimage`. Each member depends on the
/// declaration, and no node names an absent one. `keep` and its parameter
/// are written (IR-280). An enum owned by a definition the lock does not
/// select is omitted, and the rest is still written.
#[trace("TC-416")]
#[test]
fn enum_declaration_and_member_nodes_are_written() {
    let local = status_enum(
        json!({"kind": "source", "authority": "a", "identity": "u"}),
        "Status",
    );
    let foreign = status_enum(
        json!({"kind": "definition", "authority": "agent-ix", "identity": "example-model"}),
        "Foreign",
    );
    let status = local.declaration.key();
    let foreign_key = foreign.declaration.key();
    let status_type = || TypeForm::name("Status", SPAN);
    let ready = FunctionDeclaration::new(
        "ready",
        Vec::new(),
        status_type(),
        None,
        name("Status::Ready"),
    );
    let keep = FunctionDeclaration::new(
        "keep",
        vec![("s".to_owned(), status_type())],
        status_type(),
        None,
        name("s"),
    );
    let package = declared_types(vec![local, foreign], vec![ready, keep]);
    let parameter = package
        .graph()
        .semantic_graph()
        .nodes()
        .find(|node| node.semantic_form() == "parameter")
        .expect("keep's parameter node");
    assert_eq!(parameter.semantic_type(), Some(status));
    let keep_key = package.graph().function_identity("keep").unwrap();
    let emission = emit(&package);
    let omitted: BTreeMap<String, &OmissionCause> = emission
        .omitted
        .iter()
        .map(|omission| (omission.node.digest.to_string(), &omission.cause))
        .collect();
    assert_eq!(
        omitted.get(&foreign_key.to_string()),
        Some(&&OmissionCause::UnlockedOwner)
    );
    assert!(
        !omitted
            .values()
            .any(|cause| matches!(cause, OmissionCause::NamesAbsentNode(_))),
        "{omitted:?}"
    );
    assert!(!omitted.contains_key(&parameter.key().to_string()));
    // The foreign enum's two members name its declaration.
    assert_eq!(omitted.len(), 3, "{omitted:?}");
    let exports = verified_exports(&emission);
    for name in ["Point", "Pair", "Status", "ready"] {
        assert!(exports.contains_key(name), "{name}: {exports:?}");
    }
    assert_eq!(exports.get("keep"), Some(&keep_key.to_string()));
    let wire = wire(&emission);
    let declaration = declared(&wire, "Status");
    assert_eq!(declaration["node_id"]["digest"], json!(status.to_string()));
    assert_eq!(declaration["node_tag"], "scalar_type");
    assert_eq!(declaration["semantic_form"], "enum");
    assert_eq!(declaration["semantic_type"], declaration["node_id"]);
    assert_eq!(declaration["dependencies"], json!([]));
    assert_eq!(
        declaration["nominal_identity_preimage"],
        json!({
            "version": "quire.enum-declaration-node/v1",
            "owner": {"kind": "source", "authority": "a", "identity": "u"},
            "qualified_declaration": ["Status"],
            "ordered": true,
            "members": ["Ready", "Done"],
        })
    );
    let members: Vec<&Value> = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "enum_value")
        .collect();
    let cases: BTreeSet<&str> = members
        .iter()
        .map(|node| node["body"]["value"].as_str().unwrap())
        .collect();
    assert_eq!(cases, BTreeSet::from(["Ready", "Done"]));
    let ready_node = declared(&wire, "ready");
    let ready_member = members
        .iter()
        .find(|node| node["body"]["value"] == "Ready")
        .unwrap();
    assert_eq!(
        ready_node["body"]["members"][1]["value"]["target"], ready_member["node_id"],
        "ready's body is the Ready member node"
    );
    assert!(ready_member["occurrences"]
        .as_array()
        .unwrap()
        .contains(&json!({"role": "expression", "ordinal": 0})));
    for member in members {
        assert_eq!(member["semantic_type"], declaration["node_id"]);
        assert_eq!(member["dependencies"], json!([declaration["node_id"]]));
        assert_eq!(
            member["nominal_identity_preimage"]["declaration_node_id"],
            declaration["node_id"]
        );
        assert!(member.get("declaration").is_none());
    }
}

/// IR-242: IR keys an application node in a recursion group by FR-322's
/// `{size, ordinal}` in graph order, as `check` keys it. `g(): Boolean { if
/// true then true else g() }` puts two application nodes in `g`'s group, and
/// the recursive `f(x: Int[0, 9])` puts three nodes in its group: each
/// package is written with nothing omitted and reads back Verified.
#[trace("FR-093-AC-7", "TC-416")]
#[test]
fn a_recursion_group_holding_an_application_is_written() {
    let g = function(
        "g",
        &[],
        Expression::if_then_else(
            Expression::boolean(true),
            Expression::boolean(true),
            Expression::call("g".to_owned(), Vec::new()),
        ),
    );
    for (package, function) in [(package(vec![g, t()]), "g"), (recursive_f(), "f")] {
        let members: Vec<&SemanticNode> = package
            .graph()
            .semantic_graph()
            .nodes()
            .filter(|node| node.recursion().is_some())
            .collect();
        let key = package.graph().function_identity(function).unwrap();
        assert!(
            members.iter().any(|node| node.key() == key),
            "{function} is in its own group"
        );
        assert!(
            members
                .iter()
                .filter(|node| matches!(
                    node.body(),
                    qsl_semantics::check::BodyTerm::Application(_)
                ))
                .count()
                >= 2,
            "{function}'s group holds application nodes"
        );
        let emission = emit(&package);
        assert_eq!(emission.omitted, [], "{function}");
        let wire = wire(&emission);
        let written = nodes(&wire)
            .iter()
            .filter(|node| node.get("recursion_group").is_some())
            .count();
        assert_eq!(
            written,
            members.len(),
            "{function}'s whole group is written"
        );
        let exports = verified_exports(&emission);
        assert_eq!(exports.get(function), Some(&key.to_string()));
    }
}

/// FR-322 admits no empty semantic graph: an empty package refuses with no
/// bytes.
#[trace("FR-093-AC-7", "TC-416")]
#[test]
fn a_package_with_nothing_writable_refuses() {
    let empty = emit_package(&package(Vec::new()), whole_unit).unwrap_err();
    assert_eq!(
        empty,
        EmitRefusal::NothingToEmit {
            omitted: Vec::new()
        }
    );
    assert_eq!(empty.code(), Code::UnsupportedProjection);
}

/// An occurrence the caller's region conversion cannot place refuses the
/// emission, naming the occurrence.
#[trace("FR-093-AC-9", "TC-416")]
#[test]
fn an_unplaced_occurrence_refuses() {
    let refusal = emit_package(&package(vec![t()]), |_| None).unwrap_err();
    assert!(
        matches!(refusal, EmitRefusal::UnlocatedOccurrence { .. }),
        "{refusal:?}"
    );
    assert_eq!(refusal.code(), Code::UnsupportedProjection);
    // IR refuses an empty region (`start >= end`), so it places nothing.
    let empty = emit_package(&package(vec![t()]), |_| {
        Some(SourceRegion::new(source(), 3, 3).unwrap())
    })
    .unwrap_err();
    assert!(
        matches!(empty, EmitRefusal::UnlocatedOccurrence { .. }),
        "{empty:?}"
    );
}

/// FR-062-AC-5 third clause, S4 emitter half (TC-160 step 6):
/// `emit_checked` returns an emission or an `EmitRefusal`, never
/// `Incomplete`, across a fixture set of a package that writes every node, a
/// package with nothing writable, and three whose occurrences it cannot
/// place (`t` and `q`/`t` built without spans, and a declared type). `classify` matches `Result<Emission, EmitRefusal>` and
/// every `EmitRefusal` variant with no wildcard arm, so a variant added to the
/// refusal type, such as an `Incomplete`, stops this test compiling until it
/// says what the emitter does with it; the exact classification per fixture
/// fails if `emit_checked` answered any of them differently.
#[trace("TC-160", "FR-062-AC-5")]
#[test]
fn emit_checked_never_returns_incomplete_across_the_fixture_set() {
    #[derive(Debug, Eq, PartialEq)]
    enum Seen {
        Written { omitting: bool },
        NothingToEmit,
        UnlocatedOccurrence,
        UnknownOccurrenceRole,
        Encoding,
    }
    fn classify(result: Result<Emission, EmitRefusal>) -> Seen {
        match result {
            Ok(emission) => Seen::Written {
                omitting: !emission.omitted.is_empty(),
            },
            Err(EmitRefusal::NothingToEmit { .. }) => Seen::NothingToEmit,
            Err(EmitRefusal::UnlocatedOccurrence { .. }) => Seen::UnlocatedOccurrence,
            Err(EmitRefusal::UnknownOccurrenceRole { .. }) => Seen::UnknownOccurrenceRole,
            Err(EmitRefusal::Encoding { .. }) => Seen::Encoding,
            Err(EmitRefusal::Cancelled { .. }) => panic!("no handle was cancelled"),
        }
    }
    assert_eq!(
        classify(emit_checked(&package(vec![t_read_from_text()]))),
        Seen::Written { omitting: false }
    );
    assert_eq!(
        classify(emit_checked(&q_and_t_package())),
        Seen::UnlocatedOccurrence
    );
    assert_eq!(
        classify(emit_checked(&package(Vec::new()))),
        Seen::NothingToEmit
    );
    assert_eq!(
        classify(emit_checked(&package(vec![t()]))),
        Seen::UnlocatedOccurrence
    );
    assert_eq!(
        classify(emit_checked(&declared_types(Vec::new(), Vec::new()))),
        Seen::UnlocatedOccurrence
    );
}

/// FR-276: a handle cancelled before the call stops the emitter at its first
/// charge, and it writes no bytes.
#[test]
fn a_cancelled_handle_stops_the_emitter_before_it_writes_a_node() {
    let cancel = quire_exact::Cancel::new();
    cancel.cancel(quire_exact::CancelCause::Deadline);
    match emit_checked_with_cancel(&package(vec![t_read_from_text()]), &cancel) {
        Err(EmitRefusal::Cancelled { cause }) => {
            assert_eq!(cause, quire_exact::CancelCause::Deadline);
        }
        Err(other) => panic!("expected a cancellation, got {other:?}"),
        Ok(_) => panic!("a cancelled emitter wrote bytes"),
    }
}

/// TC-416 step 3 (FR-093-CON-2): the `package` crate's non-test code calls
/// no node body term constructor, no node key function and no `NodeKey`
/// constructor. A source scan: the property is the absence of a call.
#[trace("FR-093-CON-2", "TC-416")]
#[test]
fn the_package_crate_builds_no_term_and_mints_no_key() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let forbidden = [
        "LeafTerm::reference(",
        "LeafTerm::literal(",
        "MemberTerm::binding(",
        "MemberTerm::bound(",
        "BodyTerm::literal(",
        "BodyTerm::application(",
        "node_key(",
        "group_keys(",
        "NodeKey::from_digest(",
        "NodeKey::from(",
    ];
    let mut scanned = 0;
    for file in ["lib.rs", "checked.rs", "checked_v2.rs", "emit.rs"] {
        let text = std::fs::read_to_string(root.join(file)).unwrap();
        let shipped = text.split("#[cfg(test)]").next().unwrap();
        for token in forbidden {
            assert!(!shipped.contains(token), "{file} calls {token}");
        }
        scanned += 1;
    }
    assert_eq!(scanned, 4);
}

/// `t` read from [`TEXT`]: its form carries the span of its declaration and
/// of every body node (FR-091-AC-10).
fn t_read_from_text() -> FunctionDeclaration {
    let text = std::str::from_utf8(TEXT).unwrap();
    let find = |needle: &str, from: usize| {
        let start = from + text[from..].find(needle).unwrap();
        qsl_foundation::Span {
            start,
            end: start + needle.len(),
        }
    };
    let whole = find("if true then true else true", 0);
    let mut body = ExpressionSpans::new(whole).unwrap();
    let mut from = whole.start + "if".len();
    for _ in 0..3 {
        let literal = find("true", from);
        body.push_child(body.root(), literal).unwrap();
        from = literal.end;
    }
    t().with_spans(DeclarationSpans {
        declaration: qsl_foundation::Span {
            start: 0,
            end: TEXT.len(),
        },
        body,
        measure: None,
    })
    .expect("the spans fit t")
}

/// FR-096, ADR-013 O-12: `emit_checked` places every occurrence
/// through the checked unit's own form spans, with no caller conversion.
/// The package reads back Verified, and each region's bytes are the source
/// text of an expression of `t`: its whole body, or one `true` literal.
#[trace("FR-096-AC-1", "FR-091-AC-10", "TC-426")]
#[test]
fn emit_checked_places_occurrences_at_the_form_spans() {
    let emission = emit_checked(&package(vec![t_read_from_text()])).expect("t emits");
    assert!(matches!(read_back(&emission), Read::Verified { .. }));
    let wire = wire(&emission);
    let entries = wire["source_map"].as_array().unwrap();
    assert!(!entries.is_empty());
    let mut texts = BTreeSet::new();
    for entry in entries {
        for region in entry["regions"].as_array().unwrap() {
            assert_eq!(region["source"], wire["lock"]["sources"][0]);
            let start = usize::try_from(region["start"].as_u64().unwrap()).unwrap();
            let end = usize::try_from(region["end"].as_u64().unwrap()).unwrap();
            texts.insert(std::str::from_utf8(&TEXT[start..end]).unwrap().to_owned());
        }
    }
    assert!(texts.contains("if true then true else true"), "{texts:?}");
    assert!(texts.contains("true"), "{texts:?}");
    for text in &texts {
        assert!(
            ["if true then true else true", "true"].contains(&text.as_str()),
            "{text:?} is no expression of t"
        );
    }
    // A package whose form carries no spans has no region to place.
    assert!(matches!(
        emit_checked(&package(vec![t()])),
        Err(EmitRefusal::UnlocatedOccurrence { .. })
    ));
}

/// A unit whose function `f` calls another, `g` -- FR-065-AC-3's own
/// scenario: a call's own source occurrence, not a declaration's.
const CALL_TEXT: &[u8] =
    b"function g using v(): Boolean pure { true }\nfunction f using v(): Boolean pure { g() }\n";

/// The unique "g()" span in [`CALL_TEXT`].
fn call_span() -> qsl_foundation::Span {
    let text = std::str::from_utf8(CALL_TEXT).unwrap();
    let start = text.rfind("g()").expect("the call is in the fixture");
    qsl_foundation::Span {
        start,
        end: start + "g()".len(),
    }
}

/// `f`'s own declaration span in [`CALL_TEXT`]: from `function f` to the
/// unit's end.
fn f_declaration_span() -> qsl_foundation::Span {
    let text = std::str::from_utf8(CALL_TEXT).unwrap();
    let start = text
        .rfind("function f")
        .expect("f's declaration is in the fixture");
    qsl_foundation::Span {
        start,
        end: text.len(),
    }
}

/// `g`, with real spans: every occurrence `emit_checked` walks must resolve
/// a region (`emit_checked_places_occurrences_at_the_form_spans`'s own
/// "a package whose form carries no spans has no region to place" case,
/// above), so `g` needs its own spans just as much as `f` does, even though
/// this test's assertions are all about `f`'s call.
fn g_with_spans() -> FunctionDeclaration {
    let text = std::str::from_utf8(CALL_TEXT).unwrap();
    let declaration_end = text
        .find("function f")
        .expect("f's declaration is in the fixture");
    let body_start = text.find("true").expect("g's body is in the fixture");
    let body = qsl_foundation::Span {
        start: body_start,
        end: body_start + "true".len(),
    };
    function("g", &[], Expression::boolean(true))
        .with_spans(DeclarationSpans {
            declaration: qsl_foundation::Span {
                start: 0,
                end: declaration_end,
            },
            body: ExpressionSpans::new(body).expect("g's body span admits a root"),
            measure: None,
        })
        .expect("g's body is its own root, with no children")
}

/// `g` (`true`) and `f` (`g()`), with `f`'s call span set to `call` -- the
/// unit's own "g()" text for the real fixture, or a one-byte-wider
/// corruption of it for the alternate-package control.
fn f_calls_g(call: qsl_foundation::Span) -> (FunctionDeclaration, FunctionDeclaration) {
    let g = g_with_spans();
    let spans = DeclarationSpans {
        declaration: f_declaration_span(),
        body: ExpressionSpans::new(call).expect("a call span admits a root"),
        measure: None,
    };
    let f = FunctionDeclaration::new(
        "f",
        Vec::new(),
        boolean(),
        None,
        Expression::call("g".to_owned(), Vec::new()),
    )
    .with_spans(spans)
    .expect("the call is the body's own root, with no children");
    (g, f)
}

/// `g`/`f`'s own source, distinct from [`source`]'s [`TEXT`] fixture.
fn call_source() -> RawSourceRef {
    qsl_semantics::check::admitted_source(
        qsl_foundation::SourceIdentity::new("a", "call-occurrence", "git", "1"),
        CALL_TEXT,
    )
}

/// `functions`, checked (not yet linked) against [`call_source`].
fn checked_call_package(functions: Vec<FunctionDeclaration>) -> qsl_semantics::check::CheckedGraph {
    PackageDeclarations {
        functions,
        ..PackageDeclarations::new(call_source(), qsl_foundation::IdentityLimits::default())
    }
    .check(CheckingLimits::default())
    .expect("g and f check cleanly")
}

/// The call node's own key and (identity, role, ordinal) occurrence,
/// resolved against `checked`.
fn call_identity_and_location(
    checked: &qsl_semantics::check::CheckedGraph,
) -> (NodeKey, Origin, Location) {
    let identity = checked
        .semantic_graph()
        .nodes()
        .find(|node| node.semantic_form() == "call")
        .map(|node| node.key())
        .expect("the call node was lowered");
    let origin = Origin::new(Role::new("expression"), 0);
    let location = checked
        .occurrence(identity, &origin)
        .expect("check records the call's own occurrence")
        .clone();
    (identity, origin, location)
}

/// FR-065-AC-3: the call's own source occurrence resolves to the
/// same byte span through a real `quire.checked-package/v2` emit/decode
/// round trip -- a leg `qsl-eval`'s own minimal `quire.checked-function-
/// package/v2` encoding (deleted with qsl-eval's second v2 producer) never
/// could exercise, since that encoding carried no source map at all (only a declared function's
/// own name and identity). `emit_checked`'s real source map does, so this
/// is where FR-065-AC-3's v2 checkpoint actually lives.
#[trace("FR-065-AC-3", "TC-163")]
#[test]
fn emit_checked_places_the_calls_occurrence_at_its_own_source_span() {
    let call = call_span();
    let (g, f) = f_calls_g(call);
    let checked = checked_call_package(vec![g, f]);
    let (identity, origin, location) = call_identity_and_location(&checked);
    let pre_link = checked
        .region(&location)
        .expect("the call's region resolves before linking");
    assert_eq!(
        qsl_foundation::Span {
            start: usize::try_from(pre_link.start()).unwrap(),
            end: usize::try_from(pre_link.end()).unwrap(),
        },
        call,
        "the resolved region must be the call's own source span"
    );

    let linked = CheckedPackage::link(checked);
    let after_linking = linked
        .graph()
        .region(&location)
        .expect("the call's region resolves after S4 linking");
    assert_eq!(
        after_linking, pre_link,
        "S4 linking must not move or drop the call's own source region"
    );
    let emission = emit_checked(&linked).expect("g and f emit");
    let outcome = read_back(&emission);
    let Read::Verified { source_map, .. } = outcome else {
        panic!("expected Verified, got {outcome:?}");
    };
    let key = OccurrenceKey::new(
        WireNodeId::from_digest(*identity.as_bytes()),
        origin.clone(),
    );
    let regions = source_map
        .regions(&key)
        .expect("the decoded source map carries the call's own occurrence");
    assert_eq!(
        regions.len(),
        1,
        "the call has exactly one recorded occurrence"
    );
    assert_eq!(
        regions[0], pre_link,
        "the call's region must survive a real v2 emit/decode round trip unchanged"
    );

    // A hand-built alternate package whose `DeclarationSpans` is one byte
    // wider, fed through the same real `check` -> `region()` pipeline as
    // the fixture above.
    let corrupted_call = qsl_foundation::Span {
        start: call.start,
        end: call.end + 1,
    };
    let (alternate_g, alternate_f) = f_calls_g(corrupted_call);
    let alternate = checked_call_package(vec![alternate_g, alternate_f]);
    let (alternate_identity, alternate_origin, alternate_location) =
        call_identity_and_location(&alternate);
    let alternate_region = alternate
        .region(&alternate_location)
        .expect("the corrupted alternate's call region resolves");
    assert_ne!(
        alternate_region, pre_link,
        "a one-byte-wider span must resolve to a genuinely different region"
    );
    // Sanity: the corrupted alternate still records the call under the same
    // (identity, role, ordinal) key -- content, not the occurrence key
    // shape, is what differs.
    assert_eq!(alternate_identity, identity);
    assert_eq!(alternate_origin, origin);
}

/// A unit whose function takes and returns bounded integers.
const INT_TEXT: &[u8] = b"function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }";

/// `inc` read from [`INT_TEXT`], with the span of its declaration and of
/// every body node (FR-091-AC-10).
fn inc_read_from_text() -> FunctionDeclaration {
    let text = std::str::from_utf8(INT_TEXT).unwrap();
    // The last occurrence: every body text sits after the signature.
    let find = |needle: &str| {
        let start = text.rfind(needle).unwrap();
        qsl_foundation::Span {
            start,
            end: start + needle.len(),
        }
    };
    let int = |low: &str, high: &str| {
        TypeForm::builtin(BuiltinType::Int, SPAN).with_bounds(vec![low.into(), high.into()])
    };
    let mut body = ExpressionSpans::new(find("x + 1")).unwrap();
    body.push_child(body.root(), find("x")).unwrap();
    body.push_child(body.root(), find("1")).unwrap();
    FunctionDeclaration::new(
        "inc",
        vec![("x".to_owned(), int("0", "9"))],
        int("0", "10"),
        None,
        Expression::binary(BinaryOperator::Add, name("x"), Expression::integer(1_i64)),
    )
    .with_spans(DeclarationSpans {
        declaration: qsl_foundation::Span {
            start: 0,
            end: INT_TEXT.len(),
        },
        body,
        measure: None,
    })
    .expect("the spans fit inc")
}

/// FR-096: the checker's `generated` nodes (the `Int` scalar type
/// under each bounded domain) are placed at the body of the declaration
/// that names them, so a package using an integer emits through
/// `emit_checked` and reads back Verified.
#[trace("FR-096-AC-1", "TC-426")]
#[test]
fn generated_nodes_are_placed_at_their_enclosing_declaration() {
    let source = qsl_semantics::check::admitted_source(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        INT_TEXT,
    );
    let package = CheckedPackage::link(
        PackageDeclarations {
            functions: vec![inc_read_from_text()],
            ..PackageDeclarations::new(source, qsl_foundation::IdentityLimits::default())
        }
        .check(CheckingLimits::default())
        .expect("inc checks"),
    );
    let emission = emit_checked(&package).expect("inc emits");
    assert!(
        matches!(read_back(&emission), Read::Verified { .. }),
        "{:?}",
        read_back(&emission)
    );
    let wire = wire(&emission);
    let generated: Vec<&Value> = wire["source_map"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["role"] == "generated")
        .collect();
    assert!(!generated.is_empty(), "{}", wire["source_map"]);
    for entry in generated {
        for region in entry["regions"].as_array().unwrap() {
            let start = usize::try_from(region["start"].as_u64().unwrap()).unwrap();
            let end = usize::try_from(region["end"].as_u64().unwrap()).unwrap();
            assert_eq!(&INT_TEXT[start..end], b"x + 1");
        }
    }
}

/// A unit with a declared record, an `Integer` function and a function with
/// parameters, the FR-091 round trip's source.
const SPINE_TEXT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    record Point { x: Integer; y: Integer; }\n\
    function one using v(): Integer pure { 1 + 0 }\n\
    function both using v(a: Boolean, b: Boolean): Boolean pure { a and b }\n";

/// FR-091 end to end: source text goes through S1 (`qsl_cst::parse`), S2
/// (`qsl_forms::build_unit`), the E3 assembler, S3 `check`, S4 `link` and
/// `emit_checked`, and I2 reads the bytes back Verified with nothing
/// omitted: `both`'s `value`/`parameter` nodes are written (IR-280). The
/// record's `declaration` occurrence is placed at its declared name, each
/// parameter's occurrences at regions of `both`, every occurrence at a
/// region of the unit, and (FR-341-AC-10) every occurrence of a
/// `value`/`parameter` node has role `expression`.
#[trace("FR-091-AC-10", "FR-096-AC-1", "QSpec-FR-341-AC-10", "TC-426")]
#[test]
fn source_text_compiles_through_the_spine_and_reads_back_verified() {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        SPINE_TEXT.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        qsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the assembler builds the package declarations");
    let package = CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the package checks"),
    );
    let emission = emit_checked(&package).expect("the package emits with its source map");
    assert_eq!(emission.omitted, []);
    let exports = verified_exports(&emission);
    assert!(exports.contains_key("Point"), "{exports:?}");
    assert!(exports.contains_key("one"), "{exports:?}");
    assert!(exports.contains_key("both"), "{exports:?}");

    let wire = wire(&emission);
    let point = declared(&wire, "Point");
    let entries: Vec<&Value> = wire["source_map"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["node_id"] == point["node_id"] && entry["role"] == "declaration")
        .collect();
    let [entry] = entries.as_slice() else {
        panic!("Point has one declaration entry: {}", wire["source_map"]);
    };
    let region = &entry["regions"][0];
    let start = usize::try_from(region["start"].as_u64().unwrap()).unwrap();
    let end = usize::try_from(region["end"].as_u64().unwrap()).unwrap();
    assert_eq!(&SPINE_TEXT[start..end], "Point");
    let both_start = SPINE_TEXT.find("function both").unwrap();
    let both_end = both_start + SPINE_TEXT[both_start..].find('}').unwrap() + 1;
    let both = both_start..both_end;
    let parameters: Vec<&Value> = nodes(&wire)
        .iter()
        .filter(|node| node["node_tag"] == "value" && node["semantic_form"] == "parameter")
        .collect();
    assert_eq!(parameters.len(), 2, "a and b");
    for parameter in parameters {
        let mut placed = 0;
        for entry in wire["source_map"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| entry["node_id"] == parameter["node_id"])
        {
            assert_eq!(
                entry["role"], "expression",
                "FR-341-AC-10: {parameter}'s occurrence has role expression: {entry}"
            );
            for region in entry["regions"].as_array().unwrap() {
                let start = usize::try_from(region["start"].as_u64().unwrap()).unwrap();
                let end = usize::try_from(region["end"].as_u64().unwrap()).unwrap();
                assert!(
                    both.start <= start && end <= both.end,
                    "{entry} lies in both ({both:?})"
                );
                placed += 1;
            }
        }
        assert!(placed > 0, "{parameter} has a placed occurrence");
    }
    for entry in wire["source_map"].as_array().unwrap() {
        for region in entry["regions"].as_array().unwrap() {
            let end = usize::try_from(region["end"].as_u64().unwrap()).unwrap();
            assert!(end <= SPINE_TEXT.len(), "{entry}");
        }
    }
}

/// QSpec's `unit-metre` node key (FR-094's vector key).
pub(super) const METRE: [u8; 32] = [
    0x79, 0x63, 0x76, 0x23, 0xa4, 0x6d, 0x29, 0xe8, 0x84, 0xb6, 0x2c, 0x6f, 0xa2, 0x92, 0xae, 0xb2,
    0x9d, 0x41, 0xe4, 0xec, 0xc4, 0xe8, 0x00, 0xb4, 0xd7, 0xee, 0x91, 0x0a, 0x3e, 0xaf, 0x23, 0xa4,
];

/// The `metre` unit of dimension `Length`, owned by a definition, as the
/// quantity table `check` types a `Length` quantity against. Its keys are
/// QSpec's vector keys ([`METRE`] and `dimension-length`).
pub(super) fn metre_units() -> quire_semantic_value::quantity::UnitTable {
    let owner = json!({"kind": "definition", "authority": "agent-ix", "identity": "example-model"});
    let (table, metre) = metre_units_owned_by(owner);
    assert_eq!(metre, NodeKey::from_digest(METRE), "QSpec's unit-metre key");
    table
}

/// The `metre` unit of dimension `Length`, declared by the fixture unit's
/// own source, whose owner the emitted lock selects; and its unit id.
pub(super) fn source_metre_units() -> (
    quire_semantic_value::quantity::UnitTable,
    quire_exact::UnitId,
) {
    let owner = json!({"kind": "source", "authority": "a", "identity": "u"});
    let (table, metre) = metre_units_owned_by(owner);
    (table, quire_exact::UnitId::declared(metre))
}

/// The root unit `metre` of the base dimension `Length` under `owner`,
/// admitted under their recomputed keys, and the `metre` key.
fn metre_units_owned_by(owner: Value) -> (quire_semantic_value::quantity::UnitTable, NodeKey) {
    use qsl_semantics::value::{
        admit_unit_graph, DimensionPreimage, NodeIdentityPreimage, NodeOwner, OwnerSelection,
        UnitPreimage,
    };
    let length = DimensionPreimage::from_json(json!({
        "version": "quire.dimension-node/v1",
        "owner": owner,
        "qualified_declaration": ["Example", "Length"],
        "terms": [],
    }))
    .expect("a base dimension");
    let length_key = NodeKey::from_digest(
        length
            .digest(qsl_foundation::IdentityLimits::default())
            .expect("the dimension digests"),
    );
    let metre = UnitPreimage::from_json(json!({
        "version": "quire.unit-node/v1",
        "owner": owner,
        "qualified_declaration": ["Example", "metre"],
        "dimension_node_id": {"domain": NODE_KEY_DOMAIN, "digest": length_key.to_string()},
        "target_unit_node_id": null,
        "scale": {"numerator": "1", "denominator": "1"},
        "offset": {"numerator": "0", "denominator": "1"},
    }))
    .expect("a root unit");
    let metre_key = NodeKey::from_digest(
        metre
            .digest(qsl_foundation::IdentityLimits::default())
            .expect("the unit digests"),
    );
    let selection: NodeOwner = serde_json::from_value(owner).expect("an owner");
    let graph = admit_unit_graph(
        [(length, length_key)],
        [(metre, metre_key)],
        &OwnerSelection::new([selection]),
        qsl_foundation::IdentityLimits::default(),
    )
    .expect("the unit graph admits");
    (
        quire_semantic_value::quantity::UnitTable::declared(&graph),
        metre_key,
    )
}

/// TC-160 step 8's package: `q(a: Length): Boolean { a * a == a * a }`,
/// whose `metre^2` compound unit node names the definition-owned `metre`
/// unit node, which the emitted lock does not select, beside `t`, which
/// names no omitted node.
fn q_and_t_package() -> CheckedPackage {
    let metre = quire_exact::UnitId::declared(NodeKey::from_digest(METRE));
    q_and_t_package_over(metre_units(), metre)
}

/// TC-160 step 8's `q` and `t`, with `Length` the quantity of `metre` in
/// `units`.
pub(super) fn q_and_t_package_over(
    units: quire_semantic_value::quantity::UnitTable,
    metre: quire_exact::UnitId,
) -> CheckedPackage {
    let square = || Expression::binary(BinaryOperator::Multiply, name("a"), name("a"));
    let q = FunctionDeclaration::new(
        "q",
        vec![("a".to_owned(), TypeForm::name("Length", SPAN))],
        boolean(),
        None,
        Expression::binary(BinaryOperator::Equal, square(), square()),
    );
    CheckedPackage::link(
        PackageDeclarations {
            types: TypeEnvironment::default().with_units(units),
            aliases: vec![("Length".to_owned(), ValueType::Quantity(metre))],
            functions: vec![q, t()],
            ..PackageDeclarations::new(source(), qsl_foundation::IdentityLimits::default())
        }
        .check(CheckingLimits::default())
        .expect("q checks"),
    )
}

/// IR-280: IR's v2 vocabulary holds `scalar_type`/`compound_unit` (FR-094),
/// so a compound unit node is never omitted for its form. `q(a: Length):
/// Boolean { a * a == a * a }` forms the `metre^2` compound unit node; it is
/// omitted only because it names the `metre` unit node, which lowering
/// builds (FR-094) and the emission omits: its definition owner is no lock
/// entry (`UnlockedOwner`), and so is the `Length` dimension node's.
///
/// FR-062-AC-9 (TC-160 step 8, as amended): the emitter is
/// all-or-nothing over the nodes a node names. `q`'s declaration node and
/// each node on its path to the omitted unit are omitted with
/// `NamesOmittedNode`; `t` and its body are written; QSL's I2 read is
/// Verified and exports `t` and not `q`. Were the omission not to close over
/// dependents, `q` and its `==`/`*` nodes would be written naming an omitted
/// node and the omitted set below would shrink to the direct namers.
#[trace("TC-416", "TC-160", "FR-062-AC-9")]
#[test]
fn a_compound_unit_is_omitted_only_for_its_omitted_unit() {
    let package = q_and_t_package();
    let graph = package.graph().semantic_graph();
    let key_of = |form: &str| -> Vec<CheckedNodeId> {
        graph
            .nodes()
            .filter(|node| node.semantic_form() == form)
            .map(|node| node_id(node.key()))
            .collect()
    };
    let compound = key_of("compound_unit");
    assert_eq!(compound.len(), 1, "a * a forms one compound unit node");
    let emission = emit(&package);
    let exports = verified_exports(&emission);
    assert_eq!(
        exports.keys().collect::<Vec<_>>(),
        ["t"],
        "the I2 read exports t and not q"
    );
    let t_key = exports["t"].clone();
    let q_key = graph
        .nodes()
        .filter(|node| node.semantic_form() == "pure_function")
        .find(|node| node.key().to_string() != t_key)
        .map(|node| node_id(node.key()))
        .expect("q's declaration node");
    let metre_id = node_id(NodeKey::from_digest(METRE));
    let cause_of = |id: &CheckedNodeId| {
        emission
            .omitted
            .iter()
            .find(|omission| omission.node == *id)
            .map(|omission| &omission.cause)
    };
    // Lowering builds the unit and dimension nodes; their owner is unlocked.
    let units: Vec<CheckedNodeId> = key_of("unit")
        .into_iter()
        .chain(key_of("dimension"))
        .collect();
    assert_eq!(units.len(), 2, "the metre unit and Length dimension nodes");
    assert!(units.contains(&metre_id));
    for id in &units {
        assert_eq!(cause_of(id), Some(&OmissionCause::UnlockedOwner));
    }
    // The compound unit is omitted only for its omitted unit.
    assert_eq!(
        cause_of(&compound[0]),
        Some(&OmissionCause::NamesOmittedNode(metre_id.clone()))
    );
    assert!(
        !emission
            .omitted
            .iter()
            .any(|omission| matches!(omission.cause, OmissionCause::UnsupportedForm { .. })),
        "{:?}",
        emission.omitted
    );
    // Everything that names an omitted node is omitted with
    // `NamesOmittedNode`: `q`'s `==` and `*` applications and `q` itself.
    let binaries = key_of("binary");
    assert_eq!(binaries.len(), 2, "q's `==` and `*` applications");
    for id in binaries.iter().chain([&q_key]) {
        assert!(
            matches!(cause_of(id), Some(OmissionCause::NamesOmittedNode(_))),
            "{id:?}: {:?}",
            emission.omitted
        );
    }
    // The omitted set is exactly the unit and dimension nodes, the compound
    // unit, the parameter (typed by `metre`), and the nodes that reach them:
    // `t`'s nodes are not in it.
    let expected: BTreeSet<CheckedNodeId> = compound
        .into_iter()
        .chain(units)
        .chain(key_of("parameter"))
        .chain(binaries)
        .chain([q_key.clone()])
        .collect();
    let omitted: BTreeSet<CheckedNodeId> = emission
        .omitted
        .iter()
        .map(|omission| omission.node.clone())
        .collect();
    assert_eq!(omitted, expected);
    assert!(omitted.iter().all(|id| *id.digest != *t_key));
    // `q` is absent from the wire's exports, `t` is present.
    assert!(!exports.values().any(|key| **key == *q_key.digest));
}

/// FR-062-AC-9 (TC-160 step 8, second half): the same package with an
/// occurrence the region conversion cannot place refuses with
/// `EmitRefusal::UnlocatedOccurrence` and no bytes (the `Err` carries no
/// package). Placing every occurrence emits it, so the refusal is the
/// conversion's doing.
#[trace("TC-160", "FR-062-AC-9")]
#[test]
fn the_q_and_t_package_with_an_unplaced_occurrence_refuses() {
    let package = q_and_t_package();
    assert!(emit_package(&package, whole_unit).is_ok());
    let refusal = emit_package(&package, |_| None).unwrap_err();
    assert!(
        matches!(refusal, EmitRefusal::UnlocatedOccurrence { .. }),
        "{refusal:?}"
    );
    assert_eq!(refusal.code(), Code::UnsupportedProjection);
}

/// FR-105-AC-6: the injectable failure point AC-6 needs, proven
/// generically over `emit_package`'s own node-emission loop -- the same loop
/// FR-105's `state`/`frame` node goes through once a clause names one. This
/// test's fixture holds three plain function nodes and no `state`/`frame`
/// node, so it proves the mechanism, not AC-6's own frame-node claim; the
/// frame-node case (a fault on the `frame` node of a ConfigVersion
/// state-clause package) is `qsl-replay`'s
/// `a_fault_on_the_frame_node_refuses_the_whole_config_version_package`
/// (`qsl-replay/src/spine/clause/tests.rs`), which reuses this same seam
/// through `qsl_package::emit_checked_with_fault`.
///
/// When the fault fires on a node partway through the graph-order node
/// list, the whole emission refuses and no later node is even attempted:
/// `collect` on a `Result` iterator (`emit_package_inner`'s `nodes` step)
/// stops at the first `Err`, so there is no `Emission` left half-built for a
/// caller to read a partial node list out of.
#[trace("TC-463", "FR-105-AC-6")]
#[test]
fn a_fault_injected_partway_through_node_emission_writes_nothing() {
    let package = package(vec![both(), nb(), h()]);
    let clean = emit(&package);
    let total = nodes(&wire(&clean)).len();
    assert!(
        total >= 3,
        "the fixture needs several nodes to prove a *partial* set is never \
         written, not just an empty one; got {total}"
    );

    // Fault the node in the middle of the canonical graph order (never the
    // first or the last): a node before it must never appear in a written
    // package, and a node after it must never even be attempted.
    let target = total / 2;
    let attempts = std::cell::Cell::new(0usize);
    let fault_reason = "fault injection (test): forced encoding failure";
    let refusal = emit_package_with_fault(&package, whole_unit, |_id| {
        let seen = attempts.get();
        attempts.set(seen + 1);
        if seen == target {
            Some(EmitRefusal::Encoding {
                reason: fault_reason.to_owned(),
            })
        } else {
            None
        }
    })
    .expect_err("a mid-emission fault refuses the whole package");

    // The fault actually fired: this is its own refusal, not some earlier,
    // unrelated one the fixture happened to trip.
    assert!(
        matches!(&refusal, EmitRefusal::Encoding { reason } if reason == fault_reason),
        "{refusal:?}"
    );
    // Nothing past the faulted node was even considered.
    assert_eq!(
        attempts.get(),
        target + 1,
        "node emission must stop at the fault, not run past it"
    );
    // `EmitRefusal::Encoding` carries no package, and every path from here
    // to a built `Emission` (the lock, the identity preimage, the wire
    // bytes) runs only after all of `nodes` has already written -- a step
    // this fault never lets the function reach. There is no partial
    // `Emission` and no partial bytes for a caller to observe: the
    // all-or-nothing violation this AC guards against is unrepresentable by
    // this function's own return type, not merely untested.
}

// No `#[trace]` tag: no TC names the I2 reader's own preimage refusals; the
// test backs `read_checked_package_v2`'s `InvalidPreimage` arm.
/// An enum declaration node whose `declaration` is dropped (its declaration
/// occurrence re-roled `expression`, so it keeps one) passes IR's v2 reader:
/// IR's `validate_declaration_names` checks the nominal join only on a node
/// carrying both. QSL's I2 read refuses it as
/// `DeclarationNominalMismatch` with no declared name, through the
/// `InvalidPreimage` arm, and yields no package.
#[test]
fn a_nominal_node_without_its_declaration_is_refused_by_the_i2_read() {
    let local = status_enum(
        json!({"kind": "source", "authority": "a", "identity": "u"}),
        "Status",
    );
    let status = json!(local.declaration.key().to_string());
    let emission = emit(&declared_types(vec![local], vec![]));
    let mut wire = wire(&emission);
    let strip = |node: &mut Value| {
        if node["node_id"]["digest"] == status {
            node.as_object_mut().unwrap().remove("declaration").unwrap();
            for occurrence in node
                .get_mut("occurrences")
                .and_then(Value::as_array_mut)
                .into_iter()
                .flatten()
            {
                if occurrence["role"] == "declaration" {
                    occurrence["role"] = json!("expression");
                }
            }
        }
    };
    wire["semantic_graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .for_each(strip);
    wire["identity_preimage"]["identity_projection"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .for_each(strip);
    let mut re_roled = 0;
    for entry in wire["source_map"].as_array_mut().unwrap() {
        if entry["node_id"]["digest"] == status && entry["role"] == "declaration" {
            entry["role"] = json!("expression");
            re_roled += 1;
        }
    }
    assert_eq!(re_roled, 1, "Status had one declaration entry");
    let package_id = PackageId::of_preimage(&jcs(&wire["identity_preimage"]));
    wire["package_id"]["digest"] = json!(package_id.hex());
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let pinned: PinnedRequest = qsl_semantics::library::fixtures::single_pin(library(), package_id);
    let outcome = read_v2(
        &jcs(&wire),
        library(),
        V2ReadLimits::default(),
        &evidence,
        &pinned,
    );
    let Read::Refused(crate::checked_v2::V2ReadRefusal::Structural(structural)) = outcome else {
        panic!("expected a structural refusal, got {outcome:?}");
    };
    let qsl_semantics::library::LibraryRefusal::InvalidPreimage {
        library: refused,
        defect:
            qsl_semantics::library::PreimageDefect::DeclarationNominalMismatch {
                node,
                declared: None,
                nominal,
            },
    } = *structural
    else {
        panic!("expected InvalidPreimage(DeclarationNominalMismatch), got {structural:?}");
    };
    assert_eq!(refused, library());
    assert_eq!(json!(node.to_string()), status);
    assert_eq!(nominal, "Status");
}

/// FR-027-AC-5 (TC-435 step 1): the complete-V1 compile fixture, a record,
/// an Integer function and a function with parameters, goes S1 to S4 as
/// spine `compile` runs it, is written with nothing omitted and reads back
/// Verified, exporting all three declarations.
#[trace("TC-435", "FR-027-AC-5")]
#[test]
fn the_spine_compile_fixture_reads_back_verified_with_nothing_omitted() {
    let emission = emit_checked(&spine_compile_package()).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let exports = verified_exports(&emission);
    for name in ["Point", "seven", "px"] {
        assert!(
            exports.contains_key(name),
            "{name} is not exported: {exports:?}"
        );
    }
}

/// `tests/fixtures/spine-compile.native` through S1, S2, the assembler,
/// check and link, as spine `compile` runs it.
fn spine_compile_package() -> CheckedPackage {
    const FIXTURE: &[u8] = include_bytes!("../../../tests/fixtures/spine-compile.native");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine", "fixture", "fixture:1"),
        "program.native",
        FIXTURE,
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the fixture");
    assert_eq!(parsed.diagnostics(), []);
    let raw = parsed.source().reference().clone();
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let graph = PackageDeclarations::assemble(raw, unit, qsl_semantics::model::intake::SelectedModels::default(), Vec::new())
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    CheckedPackage::link(graph)
}

/// The domain package document `spine-model.native`'s `model M` selects.
const SPINE_MODEL_DOCUMENT: &[u8] =
    include_bytes!("../../../tests/fixtures/spine-model.semantic-ir.json");

/// `unit` run S1, S2, I1 (against `packages`) and the assembler, as spine
/// `compile` runs it.
fn assemble_with_models(
    unit: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
) -> Result<PackageDeclarations, String> {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine-model", "fixture", "fixture:1"),
        "program.native",
        unit,
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert_eq!(parsed.diagnostics(), []);
    let raw = parsed.source().reference().clone();
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .map_err(|refusal| format!("intake: {refusal:?}"))?;
    PackageDeclarations::assemble(raw, unit, models, Vec::new())
        .map_err(|refusal| format!("{refusal:?}"))
}

/// The spine-model fixture's inherited field access, which QSL's I2 read
/// admits (see [`inherited_field_access_is_accepted_by_the_i2_read`]).
const FIELD_ACCESS: &str = "function code using v(g: M::Gadget): Integer pure { deref(g).code }\n";

/// The spine-model fixture's equality over conforming references, which
/// QSL's I2 read admits (see
/// [`conforming_reference_equality_is_accepted_by_the_i2_read`]).
const CONFORMING_EQUALITY: &str =
    "function same using v(g: M::Gadget, w: M::Widget): Boolean pure { g = w }\n";

/// The spine-model fixture with `removed` taken out, each of which the
/// fixture holds.
fn spine_model_without(removed: &[&str]) -> String {
    const FIXTURE: &str = include_str!("../../../tests/fixtures/spine-model.native");
    removed.iter().fold(FIXTURE.to_owned(), |unit, line| {
        assert!(unit.contains(line), "the fixture holds {line:?}");
        unit.replace(line, "")
    })
}

/// `unit` through S1, S2, I1, the assembler, check, link and the v2
/// emitter against the spine-model document, with nothing omitted, and a
/// read of the bytes through QSL's I2 reader under the document's
/// `sha256-jcs` digest as domain package evidence (or `evidence_digest`,
/// when given). Returns the emission, its wire, the document's digest and
/// the read.
fn emit_model_unit(unit: &str, evidence_digest: Option<&str>) -> (Emission, Value, String, Read) {
    let packages = qsl_semantics::model::intake::package_input([SPINE_MODEL_DOCUMENT]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(evidence_digest.unwrap_or(&digest), SPINE_MODEL_DOCUMENT);
    let read = read_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
        &qsl_semantics::library::fixtures::single_pin(library(), emission.package.package_id()),
    );
    (emission, wire, digest, read)
}

/// FR-027-AC-9, FR-056-AC-9 (TC-442 step 1): the spine-model fixture
/// without its field access (`FIELD_ACCESS`) and its conforming reference
/// equality (`CONFORMING_EQUALITY`) goes
/// S1, S2, I1, the assembler, check, link and the v2 emitter with nothing
/// omitted. The assembler declares `M::Gadget` and `M::Widget` in the
/// package's `TypeEnvironment`, `Gadget` conforming to `Widget` through its
/// declared supertype. The lock and the identity preimage select the domain
/// package by identity and the `sha256-jcs` digest of the supplied
/// document, with no version, while the emitted model nodes are keyed by the
/// content-only `ModelOwner`, which carries no version. QSL's I2 read, given that digest
/// as domain package evidence, returns Verified exporting `keep` and `held`.
#[trace("TC-442", "FR-027-AC-9", "FR-056-AC-9")]
#[test]
fn a_model_bearing_unit_emits_its_model_selection_and_reads_back_verified() {
    let unit = spine_model_without(&[FIELD_ACCESS, CONFORMING_EQUALITY]);
    let packages = qsl_semantics::model::intake::package_input([SPINE_MODEL_DOCUMENT]);
    let declarations =
        assemble_with_models(unit.as_bytes(), &packages).expect("the unit assembles");
    assert_eq!(declarations.models.len(), 1);
    let views = qsl_semantics::model::intake::admit_unit(
        &unit_selections(&unit),
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the package admits");
    let object = |artifact: &str| {
        let key = qsl_semantics::model::key::DeclarationKey {
            package: "acme/orders".to_owned(),
            node: format!("ix://acme/orders/{artifact}"),
        };
        let id = views[0].view.type_identities()[&key];
        let declared = declarations
            .types
            .object_type(id)
            .unwrap_or_else(|| panic!("{artifact} is declared"));
        assert_eq!(declared.name(), format!("M::{artifact}"));
        id
    };
    let (gadget, widget) = (object("Gadget"), object("Widget"));
    assert!(declarations.types.conforms(gadget, widget));
    assert!(!declarations.types.conforms(widget, gadget));

    let (emission, wire, digest, read) = emit_model_unit(&unit, None);
    let selection = json!([{
        "identity": "acme/orders",
        "digest_domain": "sha256-jcs",
        "digest": digest,
    }]);
    assert_eq!(wire["lock"]["model_selections"], selection);
    assert_eq!(wire["identity_preimage"]["model_selections"], selection);
    assert!(
        nodes(&wire).iter().any(|node| node["node_tag"] == "model"),
        "a model node is emitted"
    );
    // FR-094-AC-1 (QSpec FR-322-AC-28): each emitted model node is keyed by
    // the content-only `ModelOwner`, `{kind, identity, node}` with no
    // version, the key IR's reader recomputes from the lock selection.
    let model_key = |artifact: &str| {
        sha256_hex(
            format!(
                r#"{{"body":{{"members":[],"term":"aggregate"}},"declaration":null,"node_tag":"model","owner":{{"identity":"acme/orders","kind":"model","node":"ix://acme/orders/{artifact}"}},"recursion":null,"semantic_form":"object_type","semantic_type":null,"version":"quire.structural-node/v1"}}"#
            )
            .as_bytes(),
        )
    };
    let emitted: BTreeSet<String> = nodes(&wire)
        .iter()
        .filter(|node| node["node_tag"] == "model")
        .map(|node| node["node_id"]["digest"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        emitted,
        BTreeSet::from([model_key("Gadget"), model_key("Widget")])
    );
    match read {
        Read::Verified { package, .. } => {
            assert_eq!(package.package_id(), emission.package.package_id());
            let exports: BTreeSet<String> = package
                .into_import_view()
                .exports()
                .map(|(name, _)| name.to_owned())
                .collect();
            for name in ["keep", "held"] {
                assert!(exports.contains(name), "{name}: {exports:?}");
            }
        }
        other => panic!("expected Verified, got {other:?}"),
    }
    // Adverse: evidence naming another document refuses the read.
    let (.., other) = emit_model_unit(&unit, Some(&"0".repeat(64)));
    assert!(matches!(other, Read::Refused(_)));
}

/// FR-027-AC-9, FR-056-AC-9 (TC-442 step 1): the whole spine-model fixture,
/// `deref(g).code` included, assembles, checks and emits with nothing
/// omitted, and QSL's I2 read, given the domain package document as
/// evidence, resolves the field member through the lock-selected domain
/// package (FR-322 "Model-owned members", IR-285) and returns Verified
/// exporting `code`.
#[trace("TC-442", "FR-027-AC-9", "FR-056-AC-9")]
#[test]
fn inherited_field_access_is_accepted_by_the_i2_read() {
    let unit = spine_model_without(&[CONFORMING_EQUALITY]);
    let (emission, .., read) = emit_model_unit(&unit, None);
    match read {
        Read::Verified { package, .. } => {
            assert_eq!(package.package_id(), emission.package.package_id());
            let exports: BTreeSet<String> = package
                .into_import_view()
                .exports()
                .map(|(name, _)| name.to_owned())
                .collect();
            assert!(exports.contains("code"), "{exports:?}");
        }
        other => panic!("expected Verified, got {other:?}"),
    }
}

/// FR-027-AC-9, FR-056-AC-9 (TC-442 step 1): `g = w` over a `Gadget` and the
/// `Widget` it conforms to checks (QSpec FR-153-AC-6, TC-198 L08: two
/// conforming references compare by identity) and emits as
/// `quire.op.reference.eq`. IR-285's QVC checked-operation catalog
/// (STD-101/102) admits conforming operands for that operation, so QSL's I2
/// read returns Verified exporting `same`.
#[trace("TC-442", "FR-027-AC-9", "FR-056-AC-9")]
#[test]
fn conforming_reference_equality_is_accepted_by_the_i2_read() {
    let unit = spine_model_without(&[FIELD_ACCESS]);
    let (emission, .., read) = emit_model_unit(&unit, None);
    match read {
        Read::Verified { package, .. } => {
            assert_eq!(package.package_id(), emission.package.package_id());
            let exports: BTreeSet<String> = package
                .into_import_view()
                .exports()
                .map(|(name, _)| name.to_owned())
                .collect();
            assert!(exports.contains("same"), "{exports:?}");
        }
        other => panic!("expected Verified, got {other:?}"),
    }
}

/// FR-056-AC-9 (TC-442 step 3): `M::Nope` names no object type of the
/// admitted domain package and refuses at the assembler, at the name's
/// span, as `missing_declaration`; the same unit given no domain package
/// refuses at I1, at its `model` declaration, as `missing_import`.
#[trace("TC-442", "FR-056-AC-9")]
#[test]
fn an_unknown_model_type_refuses_at_the_assembler_and_a_missing_package_at_intake() {
    const UNIT: &str = include_str!("../../../tests/fixtures/spine-model.native");
    let unknown = UNIT.replace("w: Reference<M::Widget>", "w: Reference<M::Nope>");
    let packages = qsl_semantics::model::intake::package_input([SPINE_MODEL_DOCUMENT]);
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine-model", "fixture", "fixture:1"),
        "program.native",
        unknown.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .unwrap();
    let unit = qsl_forms::build_unit(&parsed).unwrap();
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .unwrap();
    let refusal = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        models,
        Vec::new(),
    )
    .expect_err("M::Nope refuses");
    let [error] = &refusal.errors[..] else {
        panic!("one error: {refusal:?}");
    };
    assert_eq!(
        error.cause,
        qsl_semantics::check::AssemblyCause::UnresolvedTypeName {
            name: "M::Nope".to_owned()
        }
    );
    assert_eq!(error.cause.code(), qsl_foundation::Code::MissingDeclaration);
    assert_eq!(&unknown[error.span.start..error.span.end], "M::Nope");

    let missing = qsl_semantics::model::intake::admit_unit(
        &unit_selections(UNIT),
        &BTreeMap::new(),
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect_err("no domain package is supplied");
    assert_eq!(missing.cause.code(), qsl_foundation::Code::MissingImport);
    assert!(UNIT[missing.span.start..missing.span.end].starts_with("model M = "));
}

/// `unit`'s S2 selections.
fn unit_selections(unit: &str) -> Vec<qsl_foundation::selection::ModelSelection> {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine-model", "fixture", "fixture:1"),
        "program.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .unwrap();
    qsl_forms::build_unit(&parsed)
        .unwrap()
        .selections()
        .models
        .clone()
}

/// FR-056-AC-9 (TC-442 step 3): the assembler given no admitted model for
/// the unit's `model M` refuses `UnadmittedModel` at the declaration, as
/// `missing_import`.
#[trace("TC-442", "FR-056-AC-9")]
#[test]
fn a_model_declaration_with_no_admitted_package_refuses_at_the_assembler() {
    const UNIT: &str = include_str!("../../../tests/fixtures/spine-model.native");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine-model", "fixture", "fixture:1"),
        "program.native",
        UNIT.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .unwrap();
    let unit = qsl_forms::build_unit(&parsed).unwrap();
    let refusal = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        qsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect_err("no package is admitted for M");
    let first = &refusal.errors[0];
    assert_eq!(
        first.cause,
        qsl_semantics::check::AssemblyCause::UnadmittedModel {
            alias: "M".to_owned()
        }
    );
    assert_eq!(first.cause.code(), qsl_foundation::Code::MissingImport);
    assert!(UNIT[first.span.start..first.span.end].starts_with("model M = "));
}

/// FR-056-AC-9 (TC-442 step 3): a refusal after admission and a
/// normalization ceiling each stop I1 at the `model` declaration: a
/// supertype cycle (`Widget` and `Gadget` generalizing each other) is
/// refused by the Semantic IR record reader as
/// `invalid_model_binding`/`malformed-declaration`, and a
/// `declaration_records` ceiling of one is a limit, `stage_limit_exceeded`.
#[trace("TC-442", "FR-056-AC-9")]
#[test]
fn normalization_refusals_and_limits_stop_intake_at_the_declaration() {
    const UNIT: &str = include_str!("../../../tests/fixtures/spine-model.native");
    let selections = unit_selections(UNIT);
    let limited = qsl_semantics::model::intake::admit_unit(
        &selections,
        &qsl_semantics::model::intake::package_input([SPINE_MODEL_DOCUMENT]),
        qsl_semantics::model::accounting::ModelNormalizationLimits {
            declaration_records: 1,
            ..Default::default()
        },
    )
    .expect_err("four records exceed a ceiling of one");
    assert!(
        matches!(
            limited.cause,
            qsl_semantics::model::intake::UnitIntakeCause::Limit(_)
        ),
        "{limited:?}"
    );
    assert_eq!(
        limited.cause.code(),
        qsl_foundation::Code::StageLimitExceeded
    );
    assert!(UNIT[limited.span.start..limited.span.end].starts_with("model M = "));

    let mut cyclic: Value = serde_json::from_slice(SPINE_MODEL_DOCUMENT).unwrap();
    for node in cyclic["types"].as_array_mut().unwrap() {
        if node["identity"] == "ix://acme/orders/Widget" {
            node["supertypes"] = json!(["ix://acme/orders/Gadget"]);
        }
    }
    let cyclic = serde_json::to_vec(&cyclic).unwrap();
    let packages = qsl_semantics::model::intake::package_input([cyclic.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let text = UNIT.replace(
        &UNIT[UNIT.find("sha256-jcs:").unwrap()..][..75],
        &format!("sha256-jcs:{}", qsl_semantics::model::key::hex(digest)),
    );
    let refused = qsl_semantics::model::intake::admit_unit(
        &unit_selections(&text),
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect_err("a supertype cycle does not normalize");
    let qsl_semantics::model::intake::UnitIntakeCause::Refused(refusals) = &refused.cause else {
        panic!("expected a refusal, got {refused:?}");
    };
    assert_eq!(refusals[0].code, qsl_foundation::Code::InvalidModelBinding);
    assert_eq!(refusals[0].cause.as_str(), "malformed-declaration");
    assert!(text[refused.span.start..refused.span.end].starts_with("model M = "));
}

/// A checked graph of `t` read from [`TEXT`], so `emit_checked` places its
/// occurrences: an E4 dependency's `package_id` is recomputed by emitting it.
fn dependency() -> CheckedPackage {
    package(vec![t_read_from_text()])
}

/// The `package_id` `package` emits under.
fn emitted_id(package: &CheckedPackage) -> PackageId {
    emit_checked(package)
        .expect("the dependency emits")
        .package
        .package_id()
}

fn root_graph() -> qsl_semantics::check::CheckedGraph {
    graph_of(vec![f()])
}

fn lib(identity: &str) -> LibraryName {
    LibraryName::new(identity).expect("a non-empty library identity")
}

fn import(identity: &str, package: CheckedPackage) -> crate::Import {
    crate::Import {
        identity: lib(identity),
        package: Arc::new(package),
    }
}

/// `mid`-style package: `t` read from [`TEXT`], linked with `imports`.
fn linked(imports: Vec<crate::Import>) -> CheckedPackage {
    CheckedPackage::link_with(graph_of(vec![t_read_from_text()]), imports)
        .expect("the intermediate package links")
}

/// QSL's I2 read of `package`, a package with a dependency closure, admits
/// it: `read_import_view` supplies IR's reader the admitted package of every
/// closure entry (QSpec FR-322-AC-36).
fn read_linked(package: &CheckedPackage) {
    let emission = emit(package);
    let mut evidence = read_evidence(&emission);
    crate::checked_v2::supply_closure(package, &mut evidence).expect("each dependency reads back");
    let outcome = read_with(&emission, &evidence);
    assert!(matches!(outcome, Read::Verified { .. }), "{outcome:?}");
}

/// The closure of `package` as `(identity, package_id, path)`.
fn closure(package: &CheckedPackage) -> Vec<(LibraryName, PackageId, Vec<LibraryName>)> {
    package
        .dependency_selections()
        .iter()
        .map(|(identity, resolved)| (identity.clone(), resolved.package_id, resolved.path.clone()))
        .collect()
}

fn path(identities: &[&str]) -> Vec<LibraryName> {
    identities.iter().map(|identity| lib(identity)).collect()
}

/// FR-322 `dependency_selections`, FR-307, ADR-011 §2.4: the E4
/// closure is written as one `{identity, package_id}` entry per
/// library identity, in ascending UTF-8 byte order, identically in the lock
/// and the identity preimage. The package reads back Verified through IR's
/// reader, and the entries enter its `package_id`. A dependency's own
/// selections join the closure, through a chain of any depth, each with the
/// path that reached it.
#[trace("FR-093-AC-16", "TC-416")]
#[test]
fn the_dependency_closure_is_written_in_the_lock_and_the_preimage() {
    let d = emitted_id(&dependency());
    let unlinked = emit(&CheckedPackage::link(root_graph()));
    let linked_root = CheckedPackage::link_with(
        root_graph(),
        vec![
            import("test/units", dependency()),
            import("test/geometry", dependency()),
        ],
    )
    .expect("two imports of one package link");
    assert_eq!(linked_root.dependencies().keys().collect::<Vec<_>>(), [&d]);
    let emission = emit(&linked_root);
    read_linked(&linked_root);
    let written = wire(&emission);
    let entry = |identity: &str| {
        json!({
            "identity": identity,
            "package_id": {"domain": PACKAGE_DOMAIN_V2, "algorithm": "sha256", "digest": d.hex()},
        })
    };
    let expected = json!([entry("test/geometry"), entry("test/units")]);
    assert_eq!(written["lock"]["dependency_selections"], expected);
    assert_eq!(
        written["identity_preimage"]["dependency_selections"],
        expected
    );
    assert_ne!(emission.package.package_id(), unlinked.package.package_id());

    // A chain four packages deep: root -> b -> c -> units.
    let c = linked(vec![import("test/units", dependency())]);
    let c_id = emitted_id(&c);
    let b = linked(vec![import("test/c", c)]);
    let b_id = emitted_id(&b);
    let root = CheckedPackage::link_with(root_graph(), vec![import("test/b", b)])
        .expect("the root links through b and c");
    assert_eq!(
        closure(&root),
        [
            (lib("test/b"), b_id, path(&["test/b"])),
            (lib("test/c"), c_id, path(&["test/b", "test/c"])),
            (
                lib("test/units"),
                d,
                path(&["test/b", "test/c", "test/units"])
            ),
        ]
    );
    let emission = emit(&root);
    read_linked(&root);
    assert_eq!(
        wire(&emission)["lock"]["dependency_selections"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

/// FR-307's diamond rule, admitted side: `test/units` reached by two paths
/// (root -> units, root -> mid -> units) with one `package_id`
/// unifies to one selection, which keeps the first path.
#[trace("FR-087-AC-14", "TC-253")]
#[test]
fn a_diamond_selecting_one_package_unifies() {
    let d = emitted_id(&dependency());
    let mid = linked(vec![import("test/units", dependency())]);
    let root = CheckedPackage::link_with(
        root_graph(),
        vec![import("test/units", dependency()), import("test/mid", mid)],
    )
    .expect("one selection of test/units by two paths unifies");
    let units = &root.dependency_selections()[&lib("test/units")];
    assert_eq!(units.package_id, d);
    assert_eq!(units.path, path(&["test/units"]));
    read_linked(&root);
}

/// FR-322 orders `dependency_selections` by UTF-8 bytes. `test/\u{FF61}`
/// (UTF-8 `EF BD A1`) is written before `test/\u{1F600}` (UTF-8 `F0 …`),
/// the reverse of their UTF-16 order (`FF61` after the surrogate `D83D`),
/// and IR's reader admits that order.
#[trace("FR-093-AC-16", "TC-416")]
#[test]
fn dependency_selections_are_written_in_utf8_byte_order() {
    let root = CheckedPackage::link_with(
        root_graph(),
        vec![
            import("test/\u{1F600}", dependency()),
            import("test/\u{FF61}", dependency()),
        ],
    )
    .expect("two identities link");
    let emission = emit(&root);
    read_linked(&root);
    let identities: Vec<String> = wire(&emission)["lock"]["dependency_selections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["identity"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(identities, ["test/\u{FF61}", "test/\u{1F600}"]);
}

/// ADR-011 §4 dependency binding at E4: FR-307's diamond rule refuses two
/// selections of one identity with different `package_id`s as
/// `invalid_package`/`conflicting-definition` listing both dependency
/// paths. A dependency that does not emit refuses too. None yields a
/// package.
#[trace("FR-087-AC-14", "TC-253")]
#[test]
fn e4_refuses_a_conflicting_diamond() {
    let d = emitted_id(&dependency());

    // Two `package_id`s: mid's units is `dependency()`, and mid2's units is
    // another package.
    let another = linked(vec![import("test/x", dependency())]);
    let another_id = emitted_id(&another);
    assert_ne!(another_id, d);
    let mid = linked(vec![import("test/units", dependency())]);
    let mid2 = linked(vec![import("test/units", another)]);
    let split = CheckedPackage::link_with(
        root_graph(),
        vec![import("test/mid", mid), import("test/mid2", mid2)],
    )
    .expect_err("one identity at two package_ids does not unify");
    let crate::LinkRefusal::ConflictingDefinition {
        identity,
        selections,
    } = &split
    else {
        panic!("expected ConflictingDefinition, got {split:?}");
    };
    assert_eq!(*identity, lib("test/units"));
    assert_eq!(selections[0].package_id, d);
    assert_eq!(selections[0].path, path(&["test/mid", "test/units"]));
    assert_eq!(selections[1].package_id, another_id);
    assert_eq!(selections[1].path, path(&["test/mid2", "test/units"]));

    // A dependency whose occurrences have no source region does not emit,
    // so its `package_id` cannot be recomputed.
    let unplaced = CheckedPackage::link_with(
        root_graph(),
        vec![crate::Import {
            identity: lib("test/units"),
            package: Arc::new(package(vec![t()])),
        }],
    )
    .expect_err("an unplaceable dependency does not emit");
    assert!(
        matches!(
            &unplaced,
            crate::LinkRefusal::DependencyEmission {
                refusal: Some(EmitRefusal::UnlocatedOccurrence { .. }),
                ..
            }
        ),
        "{unplaced:?}"
    );
    assert_eq!(unplaced.code(), Code::UnsupportedProjection);
}

// ---------------------------------------------------------------------
// FR-114: a protocol `attempt`'s own `operation_anchor`/`frame`
// nodes, all the way through emission. emit.rs needs no new code for
// this node shape, but that was confirmed only in the absence of
// real FR-114 nodes; this exercises it with a real attempt and a real
// `post` clause naming the same operation, so their frame occurrences and
// `operation-contract` requirement record are expected to be shared
// exactly once (FR-114-AC-1).
// ---------------------------------------------------------------------

/// `Config::ConfigVersion`, with a `versionNumber` field and an
/// `attemptUpdate` operation whose effect frame modifies it: a real,
/// non-trivial frame for a real attempt to bind against. With `with_sub`,
/// the package also declares `Sub`, specializing `ConfigVersion` and
/// inheriting `attemptUpdate` (FR-114-AC-4).
fn config_version_model(with_sub: bool) -> qsl_semantics::model::intake::SelectedModels {
    use qsl_semantics::model::accounting::ModelNormalizationLimits;
    use qsl_semantics::model::domain_package::{
        DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord, Multiplicity,
        NativeValueType, ObjectTypeRecord, OperationEffect, OperationMemberRecord, ValueTypeRef,
    };
    use qsl_semantics::model::key::DeclarationKey;

    let key = |name: &str| DeclarationKey {
        package: "example/config-version".to_owned(),
        node: format!("ix://example/config-version/{name}"),
    };
    let mut records = vec![
        DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key: key("ConfigVersion"),
            interface_features: None,
            abstract_type: false,
            supertypes: Vec::new(),
        }),
        DomainPackageRecord::FieldMember(FieldMemberRecord {
            key: key("ConfigVersion/versionNumber"),
            owner: key("ConfigVersion"),
            value_type: ValueTypeRef::Native(NativeValueType::Integer),
            multiplicity: Multiplicity {
                lower: 1,
                upper: Some(1),
                ordered: false,
                unique: true,
            },
            presence: Presence::Required,
            subsets: Vec::new(),
            redefines: None,
        }),
        DomainPackageRecord::OperationMember(OperationMemberRecord {
            key: key("ConfigVersion/attemptUpdate"),
            owner: key("ConfigVersion"),
            parameters: Vec::new(),
            result: None,
            effect: OperationEffect {
                modifies: vec![key("ConfigVersion/versionNumber")],
                creates: Vec::new(),
                deletes: Vec::new(),
            },
            own_postcondition_clauses: Vec::new(),
            has_body: false,
            redefines: None,
        }),
    ];
    if with_sub {
        records.push(DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key: key("Sub"),
            interface_features: None,
            abstract_type: false,
            supertypes: vec![key("ConfigVersion")],
        }));
    }
    let package = DomainPackage::new(
        DomainPackageRef {
            identity: "example/config-version".to_owned(),
            version: "1".to_owned(),
            digest: [0_u8; 32],
        },
        records,
    );
    qsl_semantics::model::intake::SelectedModels::fixture("Config", SPAN,
        package, ModelNormalizationLimits::UNLIMITED)
        .expect("the ConfigVersion fixture normalizes")
}

/// A unit declaring [`config_version_model`]'s `Config` alias, then
/// `declarations`, then a protocol whose `run sequence` holds one `attempt
/// Update by R on {operation} contracts [{contracts}]`, every body the bare
/// literal `true` (the protocol content the checker covers in full).
fn attempt_frame_unit(declarations: &str, operation: &str, contracts: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model Config = \"example/config-version\" version \"1\" digest \
         \"sha256-jcs:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\";\n\
         {declarations}\n\
         protocol Flow using v over (input: Boolean) on origin {{\n\
         role R on Config::ConfigVersion;\n\
         run sequence Main {{\n\
         attempt Update by R on {operation} contracts [{contracts}] \
         as (tried: Boolean) {{ true }};\n\
         }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}"
    )
}

/// The `post` clause FR-114-AC-1's unit names in its attempt's `contracts`.
const VERSION_UNCHANGED: &str =
    "post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate { true }";

/// S1, S2 and the assembler over `unit` against [`config_version_model`],
/// then `check`, or its refusals.
fn check_attempt_unit(
    unit: &str,
    with_sub: bool,
) -> Result<qsl_semantics::check::CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>> {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:qsl-309", "fixture", "fixture:1"),
        "program.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert_eq!(parsed.diagnostics(), []);
    let raw = parsed.source().reference().clone();
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(raw, unit, config_version_model(with_sub), Vec::new())
        .expect("the unit assembles")
        .check(CheckingLimits::default())
}

/// What FR-114's emission assertions read from one attempt unit: the
/// checked attempt, each state clause's identity, the number of
/// requirement records keyed at the attempt's frame node, and the wire.
struct AttemptEmission {
    attempt: qsl_semantics::check::CheckedAttempt,
    clauses: Vec<quire_exact::NodeKey>,
    frame_records: usize,
    wire: Value,
}

/// [`check_attempt_unit`], then `CheckedPackage::link` and `emit_checked`
/// with nothing omitted.
fn emit_attempt_unit(unit: &str, with_sub: bool) -> AttemptEmission {
    let graph = check_attempt_unit(unit, with_sub)
        .unwrap_or_else(|refusals| panic!("{unit}: the package checks: {refusals:?}"));
    let [protocol] = graph.protocols() else {
        panic!("one protocol: {:?}", graph.protocols());
    };
    let [attempt] = protocol.attempts.as_slice() else {
        panic!("one attempt: {:?}", protocol.attempts);
    };
    let attempt = attempt.clone();
    let clauses = graph
        .state_clauses()
        .iter()
        .map(|clause| clause.identity())
        .collect();
    let frame = qsl_foundation::digest::WireNodeId::from_digest(*attempt.frame.as_bytes());
    let frame_records = graph
        .requirements()
        .keys()
        .filter(|key| key.node() == frame)
        .count();
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    AttemptEmission {
        attempt,
        clauses,
        frame_records,
        wire: wire(&emission),
    }
}

/// The one wire node of `semantic_form`, expecting exactly one.
fn only_node<'w>(wire: &'w Value, semantic_form: &str) -> &'w Value {
    let matching: Vec<&Value> = nodes(wire)
        .iter()
        .filter(|node| node["semantic_form"] == semantic_form)
        .collect();
    let [node] = matching[..] else {
        panic!("one {semantic_form} node: {:#?}", nodes(wire));
    };
    node
}

/// `key`'s wire node id digest, as `node_id.digest` spells it.
fn digest_of(key: quire_exact::NodeKey) -> String {
    qsl_foundation::digest::WireNodeId::from_digest(*key.as_bytes()).to_string()
}

/// TC-513 step 1/FR-114-AC-1: the checked attempt holds the identities of
/// `attemptUpdate`'s anchor node, its frame node and `VersionUnchanged`'s
/// clause node; the compiled package holds exactly one anchor node and one
/// frame node for `attemptUpdate` -- the very nodes the attempt names, the
/// frame's `modifies` exactly `versionNumber` -- and exactly one
/// `operation-contract` record for the frame. The "emit.rs needs no
/// new code" claim, exercised against real nodes rather than their absence.
#[trace("TC-513", "FR-114-AC-1")]
#[test]
fn an_attempt_and_its_clause_share_one_frame_node_through_emission() {
    let emitted = emit_attempt_unit(
        &attempt_frame_unit(
            VERSION_UNCHANGED,
            "Config::ConfigVersion::attemptUpdate",
            "VersionUnchanged",
        ),
        false,
    );
    assert_eq!(emitted.attempt.contracts, emitted.clauses);
    assert_eq!(emitted.clauses.len(), 1);
    let anchor = only_node(&emitted.wire, "operation_anchor");
    let frame = only_node(&emitted.wire, "frame");
    assert_eq!(
        anchor["node_id"]["digest"],
        digest_of(emitted.attempt.anchor)
    );
    assert_eq!(frame["node_id"]["digest"], digest_of(emitted.attempt.frame));
    assert_frame_modifies_version_number(frame);
    assert_eq!(emitted.frame_records, 1);
}

/// The frame body's `modifies` is exactly the `versionNumber` field, and it
/// creates and deletes nothing.
fn assert_frame_modifies_version_number(frame: &Value) {
    let modifies: Vec<&Value> = frame["body"]["modifies"]
        .as_array()
        .expect("a frame body lists `modifies`")
        .iter()
        .map(|entry| &entry["name"])
        .collect();
    assert_eq!(modifies, [&json!("versionNumber")], "{frame:#}");
    assert_eq!(frame["body"]["creates"], json!([]), "{frame:#}");
    assert_eq!(frame["body"]["deletes"], json!([]), "{frame:#}");
}

/// TC-513 step 2/FR-114-AC-2 (SR-770 FND-002, SR-771 FND-002): with no
/// clause naming `attemptUpdate` -- only the attempt, `contracts []` --
/// the package still checks, emits with nothing omitted, and holds exactly
/// one anchor node, one frame node and one frame record for it. The
/// attempt's own occurrences are the only ones placing these nodes, so this
/// fails if an attempt's generated occurrences have no enclosing region.
#[trace("TC-513", "FR-114-AC-2")]
#[test]
fn an_operation_named_only_by_an_attempt_emits_its_anchor_frame_and_record() {
    let emitted = emit_attempt_unit(
        &attempt_frame_unit("", "Config::ConfigVersion::attemptUpdate", ""),
        false,
    );
    assert!(emitted.attempt.contracts.is_empty());
    assert!(emitted.clauses.is_empty());
    let anchor = only_node(&emitted.wire, "operation_anchor");
    let frame = only_node(&emitted.wire, "frame");
    assert_eq!(
        anchor["node_id"]["digest"],
        digest_of(emitted.attempt.anchor)
    );
    assert_eq!(frame["node_id"]["digest"], digest_of(emitted.attempt.frame));
    assert_frame_modifies_version_number(frame);
    assert_eq!(emitted.frame_records, 1);
}

/// TC-513 step 4/FR-114-AC-4 (SR-771 FND-004): over a package where `Sub`
/// specializes `ConfigVersion`, an attempt on `Config::Sub::attemptUpdate`
/// and a `post` on `Config::ConfigVersion::attemptUpdate` share one anchor
/// node and one frame node, whose context is `ConfigVersion`'s own object
/// node -- the same one the unit without `Sub` binds.
#[trace("TC-513", "FR-114-AC-4")]
#[test]
fn an_inherited_operation_binds_its_declaring_types_anchor_and_frame() {
    let emitted = emit_attempt_unit(
        &attempt_frame_unit(VERSION_UNCHANGED, "Config::Sub::attemptUpdate", ""),
        true,
    );
    let anchor = only_node(&emitted.wire, "operation_anchor");
    let frame = only_node(&emitted.wire, "frame");
    assert_eq!(
        anchor["node_id"]["digest"],
        digest_of(emitted.attempt.anchor)
    );
    assert_eq!(frame["node_id"]["digest"], digest_of(emitted.attempt.frame));
    // The context both nodes name is `ConfigVersion`'s own object node:
    // the node the same unit binds over the package without `Sub`, where no
    // other object type exists for it to be.
    let context = &frame["semantic_type"]["digest"];
    assert_eq!(
        anchor["body"]["members"][0]["value"]["target"]["digest"],
        *context
    );
    let without_sub = emit_attempt_unit(
        &attempt_frame_unit(
            VERSION_UNCHANGED,
            "Config::ConfigVersion::attemptUpdate",
            "",
        ),
        false,
    );
    let config_version = &only_node(&without_sub.wire, "frame")["semantic_type"]["digest"];
    assert_eq!(context, config_version);
    assert_eq!(
        only_node(&without_sub.wire, "object_type")["node_id"]["digest"],
        *context
    );
}

/// SR-770 FND-001 (the reviewer's own probe): a unit with a function and a
/// protocol whose content no checker reads -- undeclared types
/// (`Nope::Input`, `Nope::Actor`, `Undeclared`), a `send` via an undeclared
/// channel and ill-typed bodies (`1 + true`) -- refuses at check, rather
/// than checking and emitting a package the protocol is silently missing
/// from.
#[trace("TC-513", "FR-114")]
#[test]
fn a_protocol_with_unchecked_garbage_content_refuses_at_check() {
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\";\n\
        model Config = \"example/config-version\" version \"1\" digest \
        \"sha256-jcs:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\";\n\
        function f using v (): Boolean pure { true }\n\
        protocol Flow using v over (input: Nope::Input) on origin {\n\
        role R on Nope::Actor;\n\
        run sequence Main {\n\
        send S via Nope as (x: Undeclared) { 1 + true };\n\
        }\n\
        finish End as (outcome: Boolean) { 1 + true };\n\
        }";
    let refusals = check_attempt_unit(unit, false)
        .map(|_| ())
        .expect_err("a protocol with unchecked content must not check");
    assert!(
        refusals
            .iter()
            .any(|refusal| refusal.cause.code().as_str() == "unsupported_construct"),
        "{refusals:?}"
    );
}

/// SR-770 FND-001/SR-771 FND-005 (probe C): the same garbage beside a
/// valid, resolving attempt still refuses at check. Each part is refused on
/// its own: an ill-typed body as not yet implemented, and an undeclared
/// role or binder type as a missing name. SR-773 FND-002 adds the rest of
/// the content `protocol_clause::content` checks rather than refuses (a
/// `using` naming no profile, a `by` naming no role, two roles of one name)
/// and the one refused construct no other test writes (`relationship`).
#[trace("TC-513", "FR-114")]
#[test]
fn a_valid_attempt_does_not_let_garbage_content_through() {
    let valid = attempt_frame_unit("", "Config::ConfigVersion::attemptUpdate", "");
    for (from, to, code) in [
        ("{ true };\n}", "{ 1 + true };\n}", "unsupported_construct"),
        (
            "role R on Config::ConfigVersion;",
            "role R on Nope::Actor;",
            "missing_declaration",
        ),
        (
            "(tried: Boolean)",
            "(tried: Undeclared)",
            "missing_declaration",
        ),
        (
            "over (input: Boolean)",
            "over (input: Nope::Input)",
            "missing_declaration",
        ),
        (
            "protocol Flow using v",
            "protocol Flow using Config",
            "missing_declaration",
        ),
        (
            "attempt Update by R",
            "attempt Update by Q",
            "missing_declaration",
        ),
        (
            "role R on Config::ConfigVersion;",
            "role R on Config::ConfigVersion;\nrole R on Config::ConfigVersion;",
            "ambiguous_declaration",
        ),
        (
            "role R on Config::ConfigVersion;",
            "role R on Config::ConfigVersion;\nrelationship rel = Config::ConfigVersion;",
            "unsupported_construct",
        ),
    ] {
        let unit = valid.replacen(from, to, 1);
        assert_ne!(unit, valid, "{from}");
        let refusals = check_attempt_unit(&unit, false)
            .map(|_| ())
            .expect_err("garbage content beside a valid attempt must not check");
        assert!(
            refusals
                .iter()
                .all(|refusal| refusal.cause.code().as_str() == code),
            "{unit}: {refusals:?}"
        );
    }
}

/// FR-114 "Behavior" (SR-770 FND-004): the operation's anchor node gets
/// one `anchor` occurrence per clause or attempt naming it, ordinals in
/// source order -- whichever is written first, the protocol's attempt or
/// the `post` clause, gets ordinal 0.
#[trace("TC-513", "FR-114")]
#[test]
fn anchor_occurrence_ordinals_follow_source_order() {
    let protocol = attempt_frame_unit(
        "",
        "Config::ConfigVersion::attemptUpdate",
        "VersionUnchanged",
    );
    let protocol_first = format!("{protocol}\n{VERSION_UNCHANGED}");
    let clause_first = attempt_frame_unit(
        VERSION_UNCHANGED,
        "Config::ConfigVersion::attemptUpdate",
        "VersionUnchanged",
    );
    // Each unit's second declaration: the first anchor occurrence lies
    // before it, the second one inside it.
    for (unit, second) in [
        (&protocol_first, "post VersionUnchanged"),
        (&clause_first, "protocol Flow"),
    ] {
        let emitted = emit_attempt_unit(unit, false);
        let anchor = &only_node(&emitted.wire, "operation_anchor")["node_id"];
        let mut entries: Vec<(u64, usize)> = emitted.wire["source_map"]
            .as_array()
            .expect("the source map is a list")
            .iter()
            .filter(|entry| entry["node_id"] == *anchor && entry["role"] == "anchor")
            .map(|entry| {
                let start = entry["regions"][0]["start"]
                    .as_u64()
                    .expect("a region start");
                (
                    entry["ordinal"].as_u64().expect("an ordinal"),
                    usize::try_from(start).expect("a region start fits usize"),
                )
            })
            .collect();
        entries.sort_unstable();
        let starts: Vec<usize> = entries.iter().map(|(_, start)| *start).collect();
        assert_eq!(entries.len(), 2, "{unit}: {entries:?}");
        assert!(
            starts.windows(2).all(|pair| pair[0] < pair[1]),
            "{unit}: {entries:?}"
        );
        let boundary = unit.find(second).expect("the unit holds both declarations");
        assert!(
            starts[0] < boundary && boundary <= starts[1],
            "{unit}: {entries:?}"
        );
    }
}

/// `declarations` after the complete-V1 profile header, run S1, S2, the
/// assembler, check and link as spine `compile` runs them, and emitted.
fn emit_from_text(declarations: &str) -> (String, Emission) {
    let (text, package) = package_from_text(declarations);
    let emission = emit_checked(&package).expect("the package emits");
    (text, emission)
}

/// `declarations` after the complete-V1 profile header, run S1, S2, the
/// assembler, check and link as spine `compile` runs them: the unit's text
/// and the linked package.
fn package_from_text(declarations: &str) -> (String, CheckedPackage) {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         {declarations}"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert_eq!(parsed.diagnostics(), []);
    let raw = parsed.source().reference().clone();
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let graph = PackageDeclarations::assemble(raw, unit, qsl_semantics::model::intake::SelectedModels::default(), Vec::new())
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    (text, CheckedPackage::link(graph))
}

/// The source text each `generated` source-map entry of `node` covers.
fn generated_texts<'t>(wire: &Value, text: &'t str, node: &Value) -> Vec<&'t str> {
    wire["source_map"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["node_id"] == *node && entry["role"] == "generated")
        .flat_map(|entry| entry["regions"].as_array().unwrap())
        .map(|region| {
            let start = usize::try_from(region["start"].as_u64().unwrap()).unwrap();
            let end = usize::try_from(region["end"].as_u64().unwrap()).unwrap();
            &text[start..end]
        })
        .collect()
}

/// QSL-349: `declarations` emits with nothing omitted, and each enum
/// member node, which no literal names, carries one `generated` occurrence
/// whose region is `placed`.
fn members_are_placed_at(declarations: &str, placed: &str) -> Emission {
    let (text, emission) = emit_from_text(declarations);
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let members: Vec<&Value> = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "enum_value")
        .collect();
    assert_eq!(members.len(), 2, "{members:?}");
    for member in members {
        assert_eq!(
            member["occurrences"],
            json!([{"role": "generated", "ordinal": 0}])
        );
        assert_eq!(
            generated_texts(&wire, &text, &member["node_id"]),
            [placed],
            "{member}"
        );
    }
    emission
}

/// An ordered enum named only by two parameter types, compared with `<`.
const ORDERED_COMPARISON: &str = "ordered enum Status { READY, DONE }\n\
    function before using v(a: Status, b: Status): Boolean pure { a < b }\n";

/// QSL-349: an ordered enum named only by two parameter types, compared
/// with `<`, places each member's `generated` occurrence at the body of the
/// function that reaches the enum, though no member literal is written.
/// The read back is
/// [`an_ordered_comparison_of_enum_parameters_reads_back_verified`].
#[trace("FR-093-AC-18", "TC-416")]
#[test]
fn enum_members_no_literal_names_are_placed_under_an_ordered_comparison() {
    members_are_placed_at(ORDERED_COMPARISON, "a < b");
}

/// FR-093-AC-18: the package of
/// [`enum_members_no_literal_names_are_placed_under_an_ordered_comparison`]
/// reads back Verified.
#[trace("FR-093-AC-18", "TC-416")]
#[test]
fn an_ordered_comparison_of_enum_parameters_reads_back_verified() {
    let emission = members_are_placed_at(ORDERED_COMPARISON, "a < b");
    let read = read_back(&emission);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
}

/// QSL-349: the same enum compared with `=` places each member the same
/// way, and the package reads back Verified.
#[trace("FR-093-AC-18", "TC-416")]
#[test]
fn enum_members_no_literal_names_are_placed_under_an_equality() {
    let emission = members_are_placed_at(
        "ordered enum Status { READY, DONE }\n\
         function same using v(a: Status, b: Status): Boolean pure { a = b }\n",
        "a = b",
    );
    let read = read_back(&emission);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
}

/// QSL-349: an enum and a record no function names place each member, and
/// the record's field type node, at the declared name, and each package
/// reads back Verified.
#[trace("FR-093-AC-18", "TC-416")]
#[test]
fn types_no_function_names_are_placed_at_their_declared_names() {
    let emission = members_are_placed_at(
        "ordered enum Status { READY, DONE }\n\
         function t using v(): Boolean pure { true }\n",
        "Status",
    );
    let read = read_back(&emission);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let (text, emission) = emit_from_text(
        "record P { x: Int[0, 9]; }\n\
         function t using v(): Boolean pure { true }\n",
    );
    assert_eq!(emission.omitted, []);
    let read = read_back(&emission);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let wire = wire(&emission);
    let field_type = nodes(&wire)
        .iter()
        .find(|node| node["semantic_form"] == "integer_range")
        .expect("x's Int[0, 9] node");
    assert_eq!(generated_texts(&wire, &text, &field_type["node_id"]), ["P"]);
}

/// QSL-349: two records no function names, `Q` declared before `P`, share
/// the `Int[0, 9]` node; it is placed at the least declared name, `P`, by
/// byte order and not by source order.
#[trace("FR-093-AC-18", "TC-416")]
#[test]
fn a_node_two_unnamed_types_share_is_placed_at_the_least_name() {
    let (text, emission) = emit_from_text(
        "record Q { y: Int[0, 9]; }\n\
         record P { x: Int[0, 9]; }\n\
         function t using v(): Boolean pure { true }\n",
    );
    assert_eq!(emission.omitted, []);
    let read = read_back(&emission);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
    let wire = wire(&emission);
    let shared = only_node(&wire, "integer_range");
    assert_eq!(generated_texts(&wire, &text, &shared["node_id"]), ["P"]);
}

/// QSL-349: a node a state clause reaches keeps its clause placement when
/// a declared type no function names also reaches it, though a type name
/// sorts before a state clause. `VersionUnchanged`'s body `1 < 2` and
/// `record R { x: Int[0, 9]; }` both reach the `Integer` scalar node.
#[trace("FR-093-AC-18", "TC-416")]
#[test]
fn a_node_a_state_clause_places_does_not_move_to_a_type_name() {
    const CLAUSE: &str =
        "post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate { 1 < 2 }";
    let unit = attempt_frame_unit(
        &format!("record R {{ x: Int[0, 9]; }}\n{CLAUSE}"),
        "Config::ConfigVersion::attemptUpdate",
        "VersionUnchanged",
    );
    let emitted = emit_attempt_unit(&unit, false);
    let integer = only_node(&emitted.wire, "integer");
    assert_eq!(
        integer["occurrences"],
        json!([{"role": "generated", "ordinal": 0}])
    );
    assert_eq!(
        generated_texts(&emitted.wire, &unit, &integer["node_id"]),
        ["1 < 2"]
    );
}
