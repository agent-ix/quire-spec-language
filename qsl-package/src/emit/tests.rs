// SPDX-License-Identifier: AGPL-3.0-or-later
//! Behavioural tests of the S4 v2 emission arm (TC-416, FR-093; FR-065-AC-2
//! through QSL's full I2 read). Every emitted package is read back through
//! `read_checked_package_v2`, which runs IR's v2 reader, and the checks
//! below are made on the bytes it admitted.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_forms::{
    BinaryOperator, BuiltinType, DeclarationSpans, Expression, ExpressionSpans,
    FunctionDeclaration, TypeForm,
};
use qsl_foundation::source::provenance::{RawSourceRef, SourceRegion};
use qsl_semantics::check::{CheckingLimits, PackageDeclarations};
use qsl_semantics::library::{LibraryName, PinnedRequest, Selection};
use qsl_semantics::value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
};
use qsl_semantics::value::{CatalogRole, DefinitionLock};
use quire_contract_ir::{CheckedArtifactLocator, CheckedPackageEvidence};
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, NodeKey, Presence, ValueType, NODE_KEY_DOMAIN,
};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

use super::*;
use crate::checked_v2::{read_v2, Read, V2ReadLimits};

/// The unit the fixture packages are read from; every occurrence's region is
/// the whole of it.
const TEXT: &[u8] = b"function t using v(): Boolean pure { if true then true else true }";

const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

/// The fixture unit admitted as (`a`, `u`, `git`, `1`): its owner `(a, u)`
/// is the one FR-092's golden vectors are keyed under.
fn source() -> RawSourceRef {
    qsl_semantics::check::admitted_source(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        TEXT,
    )
}

/// Places every occurrence at the whole fixture unit.
fn whole_unit(_: &Location) -> Option<SourceRegion> {
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
    Expression::Name(name.to_owned())
}

/// FR-093-AC-1's `t`: `if true then true else true`.
fn t() -> FunctionDeclaration {
    function(
        "t",
        &[],
        Expression::If {
            condition: Box::new(Expression::Boolean(true)),
            then: Box::new(Expression::Boolean(true)),
            otherwise: Box::new(Expression::Boolean(true)),
        },
    )
}

/// FR-092's `f`: `true`.
fn f() -> FunctionDeclaration {
    function("f", &[], Expression::Boolean(true))
}

/// FR-092's `both(a, b)`: `a and b`.
fn both() -> FunctionDeclaration {
    function(
        "both",
        &["a", "b"],
        Expression::Binary {
            operator: BinaryOperator::And,
            left: Box::new(name("a")),
            right: Box::new(name("b")),
        },
    )
}

/// FR-093's `nb(a)`: `both(a, true)`.
fn nb() -> FunctionDeclaration {
    function(
        "nb",
        &["a"],
        Expression::Call {
            name: "both".to_owned(),
            arguments: vec![name("a"), Expression::Boolean(true)],
        },
    )
}

/// FR-093's `h(a)`: `let y = a in y`.
fn h() -> FunctionDeclaration {
    function(
        "h",
        &["a"],
        Expression::Let {
            name: "y".to_owned(),
            value: Box::new(name("a")),
            body: Box::new(name("y")),
        },
    )
}

fn package(functions: Vec<FunctionDeclaration>) -> CheckedPackage {
    CheckedPackage::link(
        PackageDeclarations {
            functions,
            ..PackageDeclarations::new(source())
        }
        .check(CheckingLimits::default())
        .expect("the fixture functions check"),
    )
}

/// `record Tree { kids: Sequence<Tree>[0, 3]; }`, FR-092's G7 to G9.
fn tree() -> CheckedPackage {
    let tree = NodeKey::from_digest([6; 32]);
    let kids = ValueType::collection(CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Composite(tree),
        CardinalityBound::new(0, 3).unwrap(),
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
            ..PackageDeclarations::new(source())
        }
        .check(CheckingLimits::default())
        .expect("Tree checks"),
    )
}

fn emit(package: &CheckedPackage) -> Emission {
    emit_package(package, whole_unit).expect("the package emits")
}

fn wire(emission: &Emission) -> Value {
    serde_json::from_slice(emission.package.bytes()).expect("the wire is JSON")
}

fn jcs(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn library() -> LibraryName {
    LibraryName::new(vec!["pkg".to_owned()]).unwrap()
}

/// Every artifact reference in `value`, recorded as current evidence.
fn locked_artifacts(value: &Value, evidence: &mut CheckedPackageEvidence) {
    match value {
        Value::Object(members) => {
            if let (Some(authority), Some(identity), Some(revision), Some(domain), Some(digest)) = (
                members.get("authority").and_then(Value::as_str),
                members.get("identity").and_then(Value::as_str),
                members.get("revision"),
                members.get("digest_domain").and_then(Value::as_str),
                members.get("digest").and_then(Value::as_str),
            ) {
                evidence.insert_artifact_digest(
                    CheckedArtifactLocator {
                        authority: authority.into(),
                        identity: identity.into(),
                        revision_namespace: revision["namespace"].as_str().unwrap().into(),
                        revision_value: revision["value"].as_str().unwrap().into(),
                        domain: domain.into(),
                    },
                    digest,
                );
            }
            members
                .values()
                .for_each(|member| locked_artifacts(member, evidence));
        }
        Value::Array(items) => items
            .iter()
            .for_each(|item| locked_artifacts(item, evidence)),
        _ => {}
    }
}

/// QSL's full I2 read of `emission`, pinned at its own `package_id`, with
/// the emitted lock's artifacts as current evidence.
fn read_back(emission: &Emission) -> Read {
    let wire = wire(emission);
    let mut evidence = CheckedPackageEvidence::new();
    locked_artifacts(&wire["lock"], &mut evidence);
    locked_artifacts(&wire["diagnostics"], &mut evidence);
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let pinned: PinnedRequest = qsl_semantics::library::fixtures::single_pin(
        library(),
        Selection {
            version: "1".to_owned(),
            package_id: emission.package.package_id(),
        },
    );
    read_v2(
        emission.package.bytes(),
        library(),
        "1".to_owned(),
        V2ReadLimits::default(),
        &evidence,
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

fn nodes(wire: &Value) -> &[Value] {
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
            ..PackageDeclarations::new(source())
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
/// `DefinitionLock` catalog's rows, digests included, and IR admits them.
#[trace("FR-093-AC-7", "TC-416")]
#[test]
fn the_lock_selects_the_catalog_definitions() {
    let emission = emit(&package(vec![t()]));
    assert!(matches!(read_back(&emission), Read::Verified { .. }));
    let wire = wire(&emission);
    let lock = DefinitionLock::pinned();
    let row = |role: CatalogRole| {
        let entry = lock.entry(role).unwrap();
        json!({
            "authority": entry.authority,
            "identity": entry.identity,
            "revision": {"namespace": entry.revision_namespace, "value": entry.revision_value},
            "digest_domain": "quire.definition.bytes/v1",
            "digest": entry.digest,
        })
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
    assert_eq!(wire["lock"]["dependency_selections"], json!([]));
    assert_eq!(
        wire["lock"]["sources"][0]["digest"],
        json!(sha256_hex(TEXT))
    );
    assert_eq!(
        wire["identity_preimage"]["edition"],
        wire["lock"]["edition"]
    );
}

/// FR-322's `application_node_preimage` of a wire node, or FR-092's
/// structural preimage under owner (`a`, `u`), rebuilt by the test from the
/// wire alone. `group` is the node's recursion group in graph order.
fn rebuilt_key(node: &Value, group: &[&Value]) -> String {
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
        if application {
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
    if !application && !declaration.is_null() {
        preimage["owner"] = json!({"kind": "source", "authority": "a", "identity": "u"});
    }
    sha256_hex(&jcs(&preimage))
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
        Expression::If {
            condition: Box::new(Expression::Binary {
                operator: BinaryOperator::Greater,
                left: Box::new(name("x")),
                right: Box::new(Expression::Integer(0_i64.into())),
            }),
            then: Box::new(Expression::Call {
                name: "f".to_owned(),
                arguments: vec![Expression::Binary {
                    operator: BinaryOperator::Subtract,
                    left: Box::new(name("x")),
                    right: Box::new(Expression::Integer(1_i64.into())),
                }],
            }),
            otherwise: Box::new(Expression::Boolean(true)),
        },
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
            ..PackageDeclarations::new(source())
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

/// QSL-237 (FR-322): `check` records a `declaration`
/// occurrence for a declared record and tuple, located at the declared
/// name, so both are written with their `declaration` and read back
/// Verified: D1 and D5, exported under their declared names.
#[trace("FR-092-AC-9", "TC-416")]
#[test]
fn a_record_and_a_tuple_are_written_with_their_declarations() {
    let package = declared_types(Vec::new(), Vec::new());
    for name in ["Point", "Pair"] {
        let site = Location {
            origin: qsl_semantics::check::Origin::TypeDeclaration {
                name: name.to_owned(),
            },
            path: Vec::new(),
        };
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
        ..PackageDeclarations::new(source())
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

/// QSL-237: `record Tree { kids: Sequence<Tree>[0, 3]; }`'s recursion group
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
        EnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage,
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
    let key = NodeKey::from_digest(preimage.digest().unwrap());
    let declaration = EnumDeclaration::admit(preimage, key, &owners).expect("admitted");
    let members = ["Ready", "Done"]
        .into_iter()
        .map(|case| {
            let member = EnumMemberPreimage::from_json(json!({
                "version": "quire.enum-member-node/v1",
                "declaration_node_id": {"domain": NODE_KEY_DOMAIN, "digest": key.to_string()},
                "case": case,
            }))
            .unwrap();
            let member_key = NodeKey::from_digest(member.digest().unwrap());
            declaration.admit_member(&member, member_key).unwrap()
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
        Expression::If {
            condition: Box::new(Expression::Boolean(true)),
            then: Box::new(Expression::Boolean(true)),
            otherwise: Box::new(Expression::Call {
                name: "g".to_owned(),
                arguments: Vec::new(),
            }),
        },
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
                .filter(|node| matches!(node.body(), SemanticTerm::Application { .. }))
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

/// TC-416 step 3 (FR-093-CON-2): the `package` crate's non-test code calls
/// no node body term constructor, no node key function and no `NodeKey`
/// constructor. A source scan: the property is the absence of a call.
#[trace("FR-093-CON-2", "TC-416")]
#[test]
fn the_package_crate_builds_no_term_and_mints_no_key() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let forbidden = [
        "SemanticTerm::reference(",
        "SemanticTerm::binding(",
        "SemanticTerm::literal(",
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

/// QSL-239 (FR-096, ADR-013 O-12): `emit_checked` places every occurrence
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
        Expression::Binary {
            operator: BinaryOperator::Add,
            left: Box::new(name("x")),
            right: Box::new(Expression::Integer(quire_exact::Integer::from(1_i64))),
        },
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

/// QSL-8 (FR-096): the checker's `generated` nodes (the `Int` scalar type
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
            ..PackageDeclarations::new(source)
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
    profile v = \"quire.value.complete/v1\" version \"1\" digest \
    \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
    record Point { x: Integer; y: Integer; }\n\
    function one using v(): Integer pure { 1 + 0 }\n\
    function both using v(a: Boolean, b: Boolean): Boolean pure { a and b }\n";

/// FR-091 end to end: source text goes through S1 (`qsl_cst::parse`), S2
/// (`qsl_forms::build_unit`), the E3 assembler, S3 `check`, S4 `link` and
/// `emit_checked`, and I2 reads the bytes back Verified with nothing
/// omitted: `both`'s `value`/`parameter` nodes are written (IR-280). The
/// record's `declaration` occurrence is placed at its declared name, each
/// parameter's occurrences at regions of `both`, and every occurrence at a
/// region of the unit.
#[trace("FR-091-AC-10", "FR-096-AC-1", "TC-426")]
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
    let unit = qsl_forms::build_unit(&parsed, qsl_forms::FormsLimits::default())
        .expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(parsed.source().reference().clone(), unit)
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
const METRE: [u8; 32] = [
    0x79, 0x63, 0x76, 0x23, 0xa4, 0x6d, 0x29, 0xe8, 0x84, 0xb6, 0x2c, 0x6f, 0xa2, 0x92, 0xae, 0xb2,
    0x9d, 0x41, 0xe4, 0xec, 0xc4, 0xe8, 0x00, 0xb4, 0xd7, 0xee, 0x91, 0x0a, 0x3e, 0xaf, 0x23, 0xa4,
];

/// QSpec's `dimension-length` node key (FR-094's vector key).
const LENGTH: &str = "b6cc14ab93b670cb0fc74a80dd18131ef7b06e3eee6a730e5ca092266314e22b";

/// The `metre` unit of dimension `Length`, owned by a definition, as the
/// quantity table `check` types a `Length` quantity against.
fn metre_units() -> qsl_semantics::value::quantity::UnitTable {
    use qsl_semantics::value::{
        DimensionPreimage, NodeOwner, OwnerSelection, OwnerSubject, UnitGraph, UnitPreimage,
    };
    let owner = json!({"kind": "definition", "authority": "agent-ix", "identity": "example-model"});
    let length = DimensionPreimage::from_json(json!({
        "version": "quire.dimension-node/v1",
        "owner": owner,
        "qualified_declaration": ["Example", "Length"],
        "terms": [],
    }))
    .expect("a base dimension");
    let metre = UnitPreimage::from_json(json!({
        "version": "quire.unit-node/v1",
        "owner": owner,
        "qualified_declaration": ["Example", "metre"],
        "dimension_node_id": {"domain": NODE_KEY_DOMAIN, "digest": LENGTH},
        "target_unit_node_id": null,
        "scale": {"numerator": "1", "denominator": "1"},
        "offset": {"numerator": "0", "denominator": "1"},
    }))
    .expect("a root unit");
    let length_key: [u8; 32] = std::array::from_fn(|index| {
        u8::from_str_radix(&LENGTH[2 * index..2 * index + 2], 16).unwrap()
    });
    let graph = UnitGraph::admit(
        [(length, NodeKey::from_digest(length_key))],
        [(metre, NodeKey::from_digest(METRE))],
        &OwnerSelection::new([NodeOwner::Definition(OwnerSubject {
            authority: "agent-ix".into(),
            identity: "example-model".into(),
        })]),
    )
    .expect("the QSpec unit vectors admit");
    qsl_semantics::value::quantity::UnitTable::declared(&graph)
}

/// IR-280: IR's v2 vocabulary holds `scalar_type`/`compound_unit` (FR-094),
/// so a compound unit node is never omitted for its form. `q(a: Length):
/// Boolean { a * a == a * a }` forms the `metre^2` compound unit node; it is
/// omitted only because it names the `metre` unit node, which lowering names
/// by key but does not build.
#[trace("TC-416")]
#[test]
fn a_compound_unit_is_omitted_only_for_its_absent_unit() {
    let square = || Expression::Binary {
        operator: BinaryOperator::Multiply,
        left: Box::new(name("a")),
        right: Box::new(name("a")),
    };
    let q = FunctionDeclaration::new(
        "q",
        vec![("a".to_owned(), TypeForm::name("Length", SPAN))],
        boolean(),
        None,
        Expression::Binary {
            operator: BinaryOperator::Equal,
            left: Box::new(square()),
            right: Box::new(square()),
        },
    );
    let metre = quire_exact::UnitId::declared(NodeKey::from_digest(METRE));
    let package = CheckedPackage::link(
        PackageDeclarations {
            types: TypeEnvironment::default().with_units(metre_units()),
            aliases: vec![("Length".to_owned(), ValueType::Quantity(metre))],
            functions: vec![q, t()],
            ..PackageDeclarations::new(source())
        }
        .check(CheckingLimits::default())
        .expect("q checks"),
    );
    let compound = package
        .graph()
        .semantic_graph()
        .nodes()
        .find(|node| node.semantic_form() == "compound_unit")
        .expect("a * a forms a compound unit node");
    let emission = emit(&package);
    let cause = emission
        .omitted
        .iter()
        .find(|omission| *omission.node.digest == compound.key().to_string())
        .map(|omission| &omission.cause);
    assert_eq!(
        cause,
        Some(&OmissionCause::NamesAbsentNode(node_id(
            NodeKey::from_digest(METRE)
        )))
    );
    assert!(
        !emission
            .omitted
            .iter()
            .any(|omission| matches!(omission.cause, OmissionCause::UnsupportedForm { .. })),
        "{:?}",
        emission.omitted
    );
    assert!(verified_exports(&emission).contains_key("t"));
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
    locked_artifacts(&wire["lock"], &mut evidence);
    locked_artifacts(&wire["diagnostics"], &mut evidence);
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let pinned: PinnedRequest = qsl_semantics::library::fixtures::single_pin(
        library(),
        Selection {
            version: "1".to_owned(),
            package_id,
        },
    );
    let outcome = read_v2(
        &jcs(&wire),
        library(),
        "1".to_owned(),
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
    let unit = qsl_forms::build_unit(&parsed, qsl_forms::FormsLimits::default())
        .expect("S2 builds the unit");
    let graph = PackageDeclarations::assemble(raw, unit)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks");
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let exports = verified_exports(&emission);
    for name in ["Point", "seven", "px"] {
        assert!(
            exports.contains_key(name),
            "{name} is not exported: {exports:?}"
        );
    }
}
