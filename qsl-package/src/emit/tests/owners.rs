// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093-AC-21 and AC-22 (TC-416 steps 12 and 13): the v2 emission writes each
//! structural node's `owner` onto the node and its `identity_projection`
//! entry, exactly where the node's `quire.structural-node/v1` preimage carries
//! one. Each key is recomputed from the wire node alone, and every package is
//! read back through IR's v2 reader.

use qsl_semantics::check::{
    checked_dispatch_operation, AdmittedModel, DispatchRoot, OperationClauses,
};
use qsl_semantics::model::accounting::{Meter, ModelNormalizationLimits};
use qsl_semantics::model::dispatch::GeneralizationClosure;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Multiplicity, NativeValueType,
    ObjectTypeRecord, OperationEffect, OperationMemberRecord, OperationResult, ValueTypeRef,
};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::normalize::{normalize, NormalizeOutcome};
use quire_exact::{EffectiveId, Integer};
use quire_semantic_value::declaration::ObjectTypeDeclaration;

use super::*;

fn source_of(authority: &str, identity: &str) -> RawSourceRef {
    qsl_semantics::check::admitted_source(
        qsl_foundation::SourceIdentity::new(authority, identity, "git", "1"),
        TEXT,
    )
}

/// The owner kind FR-322's presence rule requires of a wire node.
fn required_kind(node: &Value) -> Option<&'static str> {
    let structural =
        node.get("nominal_identity_preimage").is_none() && node["body"]["term"] != "application";
    if !structural {
        None
    } else if node.get("declaration").is_some() {
        Some("source")
    } else if matches!(
        (node["node_tag"].as_str(), node["semantic_form"].as_str()),
        (Some("model"), Some("object_type" | "systems_interface"))
            | (Some("relation"), Some("relationship"))
            | (Some("function"), _)
    ) {
        Some("model")
    } else {
        None
    }
}

/// The group label FR-092 computes, rebuilt from the wire alone: the digest of
/// the array of the members' group-local preimage digests in ordinal order.
fn rebuilt_label(group: &[&Value]) -> String {
    let local: Vec<String> = group
        .iter()
        .map(|member| sha256_hex(&jcs(&rebuilt_preimage(member, group, false))))
        .collect();
    sha256_hex(&jcs(&json!(local)))
}

/// Every owner check of FR-093-AC-21 over one emitted package: each node's
/// and projection entry's `owner` equals the owner of the checked node's
/// preimage (none for a nominal node), follows FR-322's presence rule, each
/// key and each group label recomputes from the wire alone, and `read` admits
/// the package. Returns the wire.
fn assert_owners(package: &CheckedPackage, emission: &Emission, read: impl Fn(&Emission)) -> Value {
    let expected: BTreeMap<String, Option<Value>> = package
        .graph()
        .semantic_graph()
        .nodes()
        .map(|node| {
            let owner = if node.nominal().is_some() {
                None
            } else {
                let preimage: Value = serde_json::from_slice(node.preimage()).unwrap();
                preimage.get("owner").cloned()
            };
            (node.key().to_string(), owner)
        })
        .collect();
    let wire = wire(emission);
    let projection = wire["identity_preimage"]["identity_projection"]
        .as_array()
        .unwrap();
    let written = nodes(&wire);
    assert_eq!(projection.len(), written.len());
    for (node, projected) in written.iter().zip(projection) {
        let id = node["node_id"]["digest"].as_str().unwrap();
        assert_eq!(
            node.get("owner").cloned(),
            expected[id],
            "{} {} carries the owner of its preimage",
            node["node_tag"],
            node["semantic_form"]
        );
        assert_eq!(projected.get("owner"), node.get("owner"), "{id} projection");
        assert_eq!(
            node.get("owner").and_then(|owner| owner["kind"].as_str()),
            required_kind(node),
            "{id}: FR-322's presence rule"
        );
    }
    for (node, group) in with_groups(written)
        .into_iter()
        .filter(|(node, _)| node.get("nominal_identity_preimage").is_none())
    {
        assert_eq!(
            json!(rebuilt_key(node, &group)),
            node["node_id"]["digest"],
            "{node}"
        );
        if let Some(label) = node["recursion_group"].as_str() {
            assert_eq!(rebuilt_label(&group), label, "the group label recomputes");
        }
    }
    read(emission);
    wire
}

fn verified(emission: &Emission) {
    assert!(matches!(read_back(emission), Read::Verified { .. }));
}

fn owner_json(authority: &str, identity: &str) -> Value {
    json!({"kind": "source", "authority": authority, "identity": identity})
}

fn node_where<'w>(wire: &'w Value, tag: &str, form: &str) -> Vec<&'w Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["node_tag"] == tag && node["semantic_form"] == form)
        .collect()
}

/// `record P { x: Int[0, 9]; }` and `record Tree { kids: Sequence<Tree>[0, 3]; }`.
fn p_and_tree_types() -> TypeEnvironment {
    let tree = NodeKey::from_digest([6; 32]);
    let kids = ValueType::collection(CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Composite(tree),
        Some(CardinalityBound::new(0, 3).unwrap()),
    ));
    TypeEnvironment::new(
        [
            CompositeDeclaration::new(
                NodeKey::from_digest([1; 32]),
                "P",
                CompositeShape::Record(vec![FieldDeclaration::new(
                    "x",
                    int_0_9(),
                    Presence::Required,
                )]),
            ),
            CompositeDeclaration::new(
                tree,
                "Tree",
                CompositeShape::Record(vec![FieldDeclaration::new(
                    "kids",
                    kids,
                    Presence::Required,
                )]),
            ),
        ],
        [],
    )
    .expect("FR-143 admits P and Tree")
}

/// FR-093-AC-21 (TC-416 step 12), source-owned nodes: under owner (`a`, `u`)
/// `record P`, the recursive `Tree` and a function over `ordered enum Status`
/// carry the `SourceOwner`; the `Int[0, 9]`, `Sequence<Tree>`,
/// `collection_bounds`, parameter and expression nodes and the enum's
/// declaration and member nodes carry none.
#[trace("FR-093-AC-21", "TC-416")]
#[test]
fn source_declared_nodes_carry_their_source_owner() {
    let status = status_enum(owner_json("a", "u"), "Status");
    let status_type = || TypeForm::name("Status", SPAN);
    let keep = FunctionDeclaration::new(
        "keep",
        vec![("s".to_owned(), status_type())],
        status_type(),
        None,
        name("s"),
    );
    let package = CheckedPackage::link(
        PackageDeclarations {
            types: p_and_tree_types(),
            enums: vec![status],
            functions: vec![keep],
            ..PackageDeclarations::new(source(), qsl_foundation::IdentityLimits::default())
        }
        .check(CheckingLimits::default())
        .expect("the package checks"),
    );
    let emission = emit(&package);
    assert_eq!(emission.omitted, []);
    let wire = assert_owners(&package, &emission, verified);
    let a_u = owner_json("a", "u");
    for name in ["P", "Tree", "keep"] {
        assert_eq!(declared(&wire, name)["owner"], a_u, "{name}");
    }
    for (tag, form) in [
        ("bounded_domain", "integer_range"),
        ("bounded_domain", "collection_bounds"),
        ("composite_type", "sequence"),
        ("value", "parameter"),
    ] {
        let found = node_where(&wire, tag, form);
        assert!(!found.is_empty(), "{tag}/{form} is written");
        assert!(found.iter().all(|node| node.get("owner").is_none()));
    }
    let nominal = nodes(&wire)
        .iter()
        .filter(|node| node.get("nominal_identity_preimage").is_some())
        .count();
    assert!(nominal >= 3, "the enum and its members are written");
}

fn one() -> Multiplicity {
    Multiplicity {
        lower: 1,
        upper: Some(1),
        ordered: false,
        unique: true,
    }
}

fn model_key(name: &str) -> DeclarationKey {
    DeclarationKey {
        package: "acme/orders".to_owned(),
        node: format!("ix://acme/orders/{name}"),
    }
}

fn object(name: &str, supertypes: &[&str]) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: model_key(name),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes.iter().map(|name| model_key(name)).collect(),
    })
}

fn query(name: &str, owner: &str, redefines: Option<&str>) -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: model_key(name),
        owner: model_key(owner),
        parameters: Vec::new(),
        result: Some(OperationResult {
            value_type: ValueTypeRef::Native(NativeValueType::Integer),
            multiplicity: one(),
        }),
        effect: OperationEffect::default(),
        own_postcondition_clauses: Vec::new(),
        has_body: true,
        redefines: redefines.map(model_key),
    })
}

/// The JSON document the lock selects for `acme/orders`, which IR's reader
/// reads to join every `ModelOwner`: object types `Order` (operations `size`
/// and `count`) and `Sub` (operation `size`).
fn acme_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(SPINE_MODEL_DOCUMENT).unwrap();
    let operation = |owner: &str, name: &str| {
        json!({
            "identity": format!("ix://acme/orders/{owner}/{name}"),
            "name": name,
            "params": [],
            "returns": {
                "typeRef": "ix://quire/native/Integer",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false}
            },
            "pre": [],
        })
    };
    let template = document["types"][0].clone();
    let object_type = |name: &str, supertypes: &[&str], operations: Vec<Value>| {
        let mut declared = template.clone();
        declared["identity"] = json!(format!("ix://acme/orders/{name}"));
        declared["displayName"] = json!(name);
        declared["supertypes"] = json!(supertypes
            .iter()
            .map(|name| format!("ix://acme/orders/{name}"))
            .collect::<Vec<_>>());
        declared["fields"] = json!([]);
        declared["operations"] = json!(operations);
        declared
    };
    document["types"] = json!([
        object_type(
            "Order",
            &[],
            vec![operation("Order", "size"), operation("Order", "count")]
        ),
        object_type("Sub", &["Order"], vec![operation("Sub", "size")]),
    ]);
    serde_json::to_vec(&document).unwrap()
}

/// Clause functions of `Order.size` (precondition `true`, body `7`),
/// `Order.count` (precondition `true`, body `1`) and `Sub.size` (precondition
/// `false`, body `8`), each over `self`.
fn clauses(order: EffectiveId, sub: EffectiveId) -> OperationClauses {
    let mut clauses = OperationClauses::default();
    for (operation, member, receiver, precondition, body) in [
        ("Order/size", "size", order, true, 7_i64),
        ("Order/count", "count", order, true, 1),
        ("Sub/size", "size", sub, false, 8),
    ] {
        let operation = model_key(operation);
        clauses.member.insert(operation.clone(), member.to_owned());
        clauses.parameters.insert(
            operation.clone(),
            vec![("self".to_owned(), ValueType::Reference(receiver))],
        );
        clauses.result.insert(operation.clone(), ValueType::Integer);
        clauses
            .own_precondition
            .insert(operation.clone(), Expression::boolean(precondition));
        clauses
            .own_body
            .insert(operation, Expression::integer(Integer::from(body)));
    }
    clauses
}

/// FR-093-AC-21 (TC-416 step 12), model-owned nodes: `checked_dispatch_operation`
/// over `acme/orders`' effective view builds the clause functions, and a
/// declared function over a `Reference<M::Order>` parameter joins them. The
/// model declaration nodes and every clause function carry their `ModelOwner`
/// (an operation member or an object type), the declared function its
/// `SourceOwner` (`a`, `u`), and the reference, parameter and expression nodes
/// none.
#[trace("FR-093-AC-21", "TC-416")]
#[test]
fn model_declaration_nodes_and_clause_functions_carry_their_model_owner() {
    let document = acme_document();
    let digest: [u8; 32] =
        Sha256::digest(jcs(&serde_json::from_slice::<Value>(&document).unwrap())).into();
    let package = DomainPackage::new(
        DomainPackageRef {
            identity: "acme/orders".to_owned(),
            version: "1.0.0".to_owned(),
            digest,
        },
        vec![
            object("Order", &[]),
            object("Sub", &["Order"]),
            query("Order/size", "Order", None),
            query("Order/count", "Order", None),
            query("Sub/size", "Sub", Some("Order/size")),
        ],
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("acme/orders normalizes");
    };
    let model = AdmittedModel::new(&package, &view).expect("the view is the package's own");
    let id = |name: &str| view.type_identities()[&model_key(name)];
    let (order, sub) = (id("Order"), id("Sub"));
    let types = TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(order, "M::Order", Vec::new()),
            ObjectTypeDeclaration::new(sub, "M::Sub", Vec::new()).with_supertypes(vec![order]),
        ],
    )
    .expect("the acme object types admit");
    let held = FunctionDeclaration::new(
        "held",
        vec![("r".to_owned(), TypeForm::name("M::Order", SPAN))],
        boolean(),
        None,
        Expression::boolean(true),
    );
    let mut seen = BTreeSet::new();
    let mut model_nodes = BTreeSet::new();
    for root in ["Order/size", "Order/count", "Sub/size"] {
        let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
        let mut declarations = checked_dispatch_operation(
            &view,
            &DispatchRoot {
                key: model_key(root),
                closure: GeneralizationClosure::Closed,
            },
            &clauses(order, sub),
            source(),
            &mut meter,
            qsl_foundation::IdentityLimits::default(),
        )
        .unwrap_or_else(|refusal| panic!("{root} links and checks: {refusal:?}"));
        declarations.types = types.clone();
        if declarations.models.is_empty() {
            declarations.models.push(model.clone());
        }
        declarations.functions.push(held.clone());
        let checked = CheckedPackage::link(
            declarations
                .check(CheckingLimits::default())
                .unwrap_or_else(|refusals| panic!("{root} checks: {refusals:?}")),
        );
        let emission = emit(&checked);
        assert_eq!(emission.omitted, []);
        let read = |emission: &Emission| {
            let mut evidence = read_evidence(emission);
            let wire = wire(emission);
            let digest = wire["lock"]["model_selections"][0]["digest"]
                .as_str()
                .unwrap();
            evidence.insert_domain_package_document(digest, &document[..]);
            assert!(
                matches!(read_with(emission, &evidence), Read::Verified { .. }),
                "{root}: IR's v2 reader admits the package"
            );
        };
        let wire = assert_owners(&checked, &emission, read);
        assert_eq!(declared(&wire, "held")["owner"], owner_json("a", "u"));
        let models = node_where(&wire, "model", "object_type");
        assert!(!models.is_empty(), "{root}: a model declaration node");
        for node in &models {
            assert_eq!(node["owner"]["kind"], "model");
            assert_eq!(node["owner"]["identity"], "acme/orders");
            assert!(node["owner"].get("version").is_none());
        }
        model_nodes.extend(
            models
                .iter()
                .map(|node| node["owner"]["node"].as_str().unwrap().to_owned()),
        );
        for node in node_where(&wire, "function", "pure_function")
            .into_iter()
            .filter(|node| node.get("declaration").is_none())
        {
            seen.insert(node["owner"]["node"].as_str().unwrap().to_owned());
        }
        for node in node_where(&wire, "composite_type", "reference") {
            assert!(node.get("owner").is_none());
        }
    }
    // FR-093-AC-21: exactly the document's two object types own a node.
    assert_eq!(
        model_nodes,
        BTreeSet::from([
            "ix://acme/orders/Order".to_owned(),
            "ix://acme/orders/Sub".to_owned(),
        ]),
        "the model owners' nodes"
    );
    assert_eq!(
        seen,
        BTreeSet::from([
            "ix://acme/orders/Order/count".to_owned(),
            "ix://acme/orders/Order/size".to_owned(),
            "ix://acme/orders/Sub/size".to_owned(),
        ]),
        "the clause functions are owned by their operation members"
    );
}

/// `record Point { x: Integer; }` and `record List { next?: List; }`.
fn point_and_list_package(owner: &RawSourceRef) -> CheckedPackage {
    let list = NodeKey::from_digest([7; 32]);
    let types = TypeEnvironment::new(
        [
            CompositeDeclaration::new(
                NodeKey::from_digest([8; 32]),
                "Point",
                CompositeShape::Record(vec![FieldDeclaration::new(
                    "x",
                    ValueType::Integer,
                    Presence::Required,
                )]),
            ),
            CompositeDeclaration::new(
                list,
                "List",
                CompositeShape::Record(vec![FieldDeclaration::new(
                    "next",
                    ValueType::Composite(list),
                    Presence::Optional,
                )]),
            ),
        ],
        [],
    )
    .expect("FR-143 admits Point and List");
    CheckedPackage::link(
        PackageDeclarations {
            types,
            ..PackageDeclarations::new(owner.clone(), qsl_foundation::IdentityLimits::default())
        }
        .check(CheckingLimits::default())
        .expect("Point and List check"),
    )
}

fn two_owner_emission(identity: &str) -> (CheckedPackage, Emission) {
    let owner = source_of("agent-ix", identity);
    let package = point_and_list_package(&owner);
    let emission = emit_under(&package, &owner);
    (package, emission)
}

/// The node ids of one package by role: `Point`, the `List` group's members in
/// graph order and its label, `Integer`.
fn two_owner_ids(wire: &Value) -> (Value, Vec<Value>, Value, Value) {
    let point = declared(wire, "Point")["node_id"].clone();
    let members: Vec<&Value> = nodes(wire)
        .iter()
        .filter(|node| node.get("recursion_group").is_some())
        .collect();
    let label = members[0]["recursion_group"].clone();
    let integer = node_where(wire, "scalar_type", "integer")[0]["node_id"].clone();
    (
        point,
        members.iter().map(|node| node["node_id"].clone()).collect(),
        label,
        integer,
    )
}

/// FR-093-AC-22 (TC-416 step 13): one source under owners `example-a` and
/// `example-b` gives two packages whose `Point` ids, `List` group labels and
/// member ids differ and whose `Integer` id is equal; their `package_id`s
/// differ and IR's v2 reader admits both.
#[trace("FR-093-AC-22", "TC-416")]
#[test]
fn one_source_under_two_owners_differs_exactly_on_the_owner_keyed_nodes() {
    let (package_a, emission_a) = two_owner_emission("example-a");
    let (package_b, emission_b) = two_owner_emission("example-b");
    let wire_a = assert_owners(&package_a, &emission_a, verified);
    let wire_b = assert_owners(&package_b, &emission_b, verified);
    assert_eq!(
        declared(&wire_a, "Point")["owner"],
        owner_json("agent-ix", "example-a")
    );
    assert_eq!(
        declared(&wire_b, "Point")["owner"],
        owner_json("agent-ix", "example-b")
    );
    let (point_a, list_a, label_a, integer_a) = two_owner_ids(&wire_a);
    let (point_b, list_b, label_b, integer_b) = two_owner_ids(&wire_b);
    assert_ne!(point_a, point_b);
    assert_ne!(label_a, label_b);
    assert_eq!(list_a.len(), 2, "the List record and its Option node");
    assert!(list_a.iter().zip(&list_b).all(|(a, b)| a != b));
    assert_eq!(integer_a, integer_b);
    assert_ne!(
        emission_a.package.package_id(),
        emission_b.package.package_id()
    );
}

/// The emitter reproduces the nodes of QSpec's published two-owner and
/// recursive-record fixtures (QSpec FR-322-AC-53): equal node ids, owners,
/// group labels and bodies for `Point`, `List`, its `Option` node and
/// `Integer`. Reads the fixtures from `$QSPEC_DIR` (skipped, and passing,
/// when it is unset; `make conformance` requires it); nothing of QSpec is
/// copied into this repository.
#[trace("FR-093-AC-22", "TC-416")]
#[test]
fn conformance_the_emitter_reproduces_qspec_two_owner_fixture_nodes() {
    let Some(qspec) = std::env::var_os("QSPEC_DIR") else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let directory = std::path::Path::new(&qspec).join("proposals/checked-package-v2/fixtures");
    for (file, identity) in [
        ("positive-two-owners-a.json", "example-a"),
        ("positive-two-owners-b.json", "example-b"),
    ] {
        let bytes = std::fs::read(directory.join(file)).unwrap();
        let fixture: Value = serde_json::from_slice(&bytes).unwrap();
        let (_, emission) = two_owner_emission(identity);
        let emitted = wire(&emission);
        let published = |node: &Value| {
            fixture["semantic_graph"]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|candidate| candidate["node_id"] == node["node_id"])
                .unwrap_or_else(|| panic!("{file} holds {}", node["node_id"]))
                .clone()
        };
        let compared = [
            "node_id",
            "node_tag",
            "semantic_form",
            "semantic_type",
            "declaration",
            "owner",
            "recursion_group",
            "body",
            "dependencies",
        ];
        for node in nodes(&emitted) {
            let other = published(node);
            for member in compared {
                assert_eq!(node.get(member), other.get(member), "{file}: {member}");
            }
        }
    }
}
