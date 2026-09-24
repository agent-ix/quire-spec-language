// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-417, TC-418 and TC-419: FR-094's model-owned, `Reference<T>`,
//! `Population<T>[N]`, clause-function and quantity type nodes, checked
//! against FR-094's golden vectors. The vectors are read from this
//! repository's own FR-094 text at compile time, so the spec and the test
//! cannot drift apart.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_forms::{
    BinaryOperator, BuiltinType, DeclaredClauseKind, Expression, FunctionDeclaration, TypeForm,
    TypeFormHead,
};
use quire_exact::{Integer, IntegerInterval, Presence};
use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};

use super::super::{generated_location, LockEvidence, Lowering, SemanticGraph, SemanticNode};
use super::*;
use crate::check::checked_dispatch::{checked_dispatch_operation, DispatchRoot, OperationClauses};
use crate::check::family::fixtures::{empty_scope, fixture_owner};
use crate::check::family::OccurrenceMap;
use crate::check::{CheckedGraph, CheckingLimits, PackageDeclarations};
use crate::model::accounting::{Meter, ModelNormalizationLimits};
use crate::model::dispatch::GeneralizationClosure;
use crate::model::domain_package::{
    FieldMemberRecord, Multiplicity, NativeValueType, ObjectTypeRecord, OperationEffect,
    OperationMemberRecord, OperationResult, RelationshipDirection, RelationshipEnd,
    RelationshipRecord, ValueTypeRef,
};
use crate::model::normalize::{normalize, NormalizeOutcome};
use crate::value::declaration::{FieldDeclaration, ObjectTypeDeclaration, TypeEnvironment};

mod expression_depth;

const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

const FR_094: &str = include_str!(
    "../../../../../spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md"
);

/// FR-094's golden vectors by name: `(key, preimage)`.
fn vectors() -> BTreeMap<String, (String, String)> {
    let mut vectors = BTreeMap::new();
    let mut lines = FR_094.lines();
    while let Some(line) = lines.next() {
        let Some(rest) = line.strip_prefix("**") else {
            continue;
        };
        let Some((name, _)) = rest.split_once("**: ") else {
            continue;
        };
        if lines.next() != Some("") || lines.next() != Some("```json") {
            continue;
        }
        let preimage = lines.next().expect("a vector preimage line").to_owned();
        assert_eq!(lines.next(), Some("```"), "{name}: one preimage line");
        assert_eq!(lines.next(), Some(""));
        let key = lines
            .next()
            .and_then(|line| line.strip_prefix("Key: `"))
            .and_then(|line| line.strip_suffix('`'))
            .expect("a vector key line")
            .to_owned();
        vectors.insert(name.to_owned(), (key, preimage));
    }
    assert_eq!(vectors.len(), 39, "FR-094 publishes 39 golden vectors");
    vectors
}

fn vector_key(name: &str) -> NodeKey {
    let hex = &vectors()[name].0;
    let mut digest = [0_u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * index..2 * index + 2], 16).expect("hex");
    }
    NodeKey::from_digest(digest)
}

/// Assert `key` names vector `name` in `graph`, bytes and key.
fn assert_vector(graph: &SemanticGraph, key: NodeKey, name: &str) {
    let (expected_key, expected_preimage) = &vectors()[name];
    let node = graph
        .node(key)
        .unwrap_or_else(|| panic!("{name}: no node keyed {key}"));
    assert_eq!(
        std::str::from_utf8(node.preimage()).expect("UTF-8"),
        expected_preimage,
        "{name}: preimage bytes"
    );
    assert_eq!(&key.to_string(), expected_key, "{name}: key");
}

/// Assert `graph` holds vector `name`, byte for byte.
fn assert_holds(graph: &SemanticGraph, name: &str) {
    assert_vector(graph, vector_key(name), name);
}

fn preimage(node: &SemanticNode) -> Json {
    serde_json::from_slice(node.preimage()).expect("a preimage is JSON")
}

/// The parameter node named `name` at `level`.
fn parameter<'g>(graph: &'g SemanticGraph, name: &str, level: u64) -> &'g SemanticNode {
    graph
        .nodes()
        .find(|node| {
            let preimage = preimage(node);
            node.semantic_form() == "parameter"
                && preimage["body"]["members"][0]["value"]["value"] == json!(name)
                && preimage["body"]["members"][1]["value"]["value"] == json!(level.to_string())
        })
        .unwrap_or_else(|| panic!("no parameter node {name} at level {level}"))
}

/// Every application node whose operation is `identity`.
fn applications<'g>(graph: &'g SemanticGraph, identity: &str) -> Vec<&'g SemanticNode> {
    graph
        .nodes()
        .filter(|node| preimage(node)["body"]["operation"]["identity"] == json!(identity))
        .collect()
}

fn member_declaration(node: &SemanticNode) -> Json {
    preimage(node)["body"]["operation"]["member"]["declaration"]["digest"].clone()
}

// ---------------------------------------------------------------------
// The `acme/orders` fixture of FR-094's Golden vectors section.
// ---------------------------------------------------------------------

const PACKAGE: &str = "acme/orders";

fn key(name: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: format!("ix://acme/orders/{name}"),
    }
}

fn one() -> Multiplicity {
    Multiplicity {
        lower: 1,
        upper: Some(1),
        ordered: false,
        unique: true,
    }
}

fn object(name: &str, supertypes: &[&str]) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: key(name),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes.iter().map(|name| key(name)).collect(),
    })
}

fn query(name: &str, owner: &str, redefines: Option<&str>) -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: key(name),
        owner: key(owner),
        parameters: Vec::new(),
        result: Some(OperationResult {
            value_type: ValueTypeRef::Native(NativeValueType::Integer),
            multiplicity: one(),
        }),
        effect: OperationEffect::default(),
        own_postcondition_clauses: Vec::new(),
        has_body: true,
        redefines: redefines.map(key),
    })
}

/// `acme/orders` at `version`: object types `Order` (attribute `total`,
/// queries `size` and `count`), `Invoice` and `Sub` (supertype `Order`,
/// redefining `size`), and the relationship `billedTo`.
fn acme(version: &str) -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef {
            identity: PACKAGE.to_owned(),
            version: version.to_owned(),
            digest: Sha256::digest(format!("acme/orders@{version}").as_bytes()).into(),
        },
        vec![
            object("Order", &[]),
            object("Invoice", &[]),
            object("Sub", &["Order"]),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: key("Order/total"),
                owner: key("Order"),
                value_type: ValueTypeRef::Native(NativeValueType::Integer),
                multiplicity: one(),
                subsets: Vec::new(),
                redefines: None,
            }),
            query("Order/size", "Order", None),
            query("Order/count", "Order", None),
            query("Sub/size", "Sub", Some("Order/size")),
            DomainPackageRecord::Relationship(RelationshipRecord {
                key: key("billedTo"),
                source: RelationshipEnd {
                    type_identity: key("Order"),
                    role: Some("order".to_owned()),
                    multiplicity: one(),
                },
                target: RelationshipEnd {
                    type_identity: key("Invoice"),
                    role: Some("invoice".to_owned()),
                    multiplicity: one(),
                },
                direction: RelationshipDirection::SourceToTarget,
            }),
        ],
    )
}

/// `acme/orders` at `version`, normalized and admitted.
struct Acme {
    view: EffectiveView,
    model: AdmittedModel,
    order: EffectiveId,
    invoice: EffectiveId,
    sub: EffectiveId,
}

fn admitted(version: &str) -> Acme {
    let package = acme(version);
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("acme/orders {version} normalizes");
    };
    let model = AdmittedModel::new(&package, &view).expect("the view is the package's own");
    let id = |name: &str| view.type_identities()[&key(name)];
    Acme {
        order: id("Order"),
        invoice: id("Invoice"),
        sub: id("Sub"),
        model,
        view,
    }
}

fn int(lower: i64, upper: i64) -> ValueType {
    ValueType::Int(IntegerInterval::new(Integer::from(lower), Integer::from(upper)).unwrap())
}

/// `M::Order`, `M::Invoice` and `M::Sub`, each with the attribute `total:
/// Int[0, 9]` where it has one.
fn types(acme: &Acme) -> TypeEnvironment {
    let total = || {
        vec![FieldDeclaration::new(
            "total",
            int(0, 9),
            Presence::Required,
        )]
    };
    TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(acme.order, "M::Order", total()),
            ObjectTypeDeclaration::new(acme.invoice, "M::Invoice", Vec::new()),
            ObjectTypeDeclaration::new(acme.sub, "M::Sub", total())
                .with_supertypes(vec![acme.order]),
        ],
    )
    .expect("the acme object types admit")
}

fn named(name: &str) -> TypeForm {
    TypeForm::name(name, SPAN)
}

fn population(target: &str, maximum: u64) -> TypeForm {
    TypeForm::new(TypeFormHead::Population, SPAN)
        .with_arguments(vec![named(target)])
        .with_bounds(vec![maximum.to_string()])
}

fn builtin(builtin: BuiltinType) -> TypeForm {
    TypeForm::builtin(builtin, SPAN)
}

fn name(name: &str) -> Expression {
    Expression::Name(name.to_owned())
}

fn function(
    function_name: &str,
    parameters: &[(&str, TypeForm)],
    result: TypeForm,
    body: Expression,
) -> FunctionDeclaration {
    FunctionDeclaration::new(
        function_name,
        parameters
            .iter()
            .map(|(name, form)| ((*name).to_owned(), form.clone()))
            .collect(),
        result,
        None,
        body,
    )
}

fn check(acme: &Acme, functions: Vec<FunctionDeclaration>) -> CheckedGraph {
    PackageDeclarations {
        types: types(acme),
        models: vec![acme.model.clone()],
        functions,
        ..PackageDeclarations::new(fixture_owner())
    }
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("the acme functions check: {refusals:?}"))
}

/// The `OperationClauses` of `Order.size` (precondition `true`, body `7`),
/// `Order.count` (precondition `true`, body `1`) and `Sub.size`
/// (precondition `false`, body `8`), each over `self`.
fn clauses(acme: &Acme) -> OperationClauses {
    let mut clauses = OperationClauses::default();
    let operations = [
        ("Order/size", "size", acme.order, true, 7_i64),
        ("Order/count", "count", acme.order, true, 1),
        ("Sub/size", "size", acme.sub, false, 8),
    ];
    for (operation, member, receiver, precondition, body) in operations {
        let operation = key(operation);
        clauses.member.insert(operation.clone(), member.to_owned());
        clauses.parameters.insert(
            operation.clone(),
            vec![("self".to_owned(), ValueType::Reference(receiver))],
        );
        clauses.result.insert(operation.clone(), ValueType::Integer);
        clauses
            .own_precondition
            .insert(operation.clone(), Expression::Boolean(precondition));
        clauses
            .own_body
            .insert(operation, Expression::Integer(Integer::from(body)));
    }
    clauses
}

/// `checked_dispatch_operation` rooted at `root`, with the acme types.
fn dispatch(acme: &Acme, root: &str) -> PackageDeclarations {
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    let mut declarations = checked_dispatch_operation(
        &acme.view,
        &DispatchRoot {
            key: key(root),
            closure: GeneralizationClosure::Closed,
        },
        &clauses(acme),
        fixture_owner(),
        &mut meter,
    )
    .unwrap_or_else(|refusal| panic!("{root} links and checks: {refusal:?}"));
    declarations.types = types(acme);
    declarations
}

fn check_declarations(declarations: PackageDeclarations) -> CheckedGraph {
    declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"))
}

/// A lowering over the admitted `models`, for keying a node directly.
fn lower<T>(
    models: &[AdmittedModel],
    units: crate::value::quantity::UnitTable,
    build: impl FnOnce(&mut Lowering<'_>) -> T,
) -> (T, SemanticGraph) {
    let scope = empty_scope();
    let lock = LockEvidence::default();
    let mut occurrences = OccurrenceMap::default();
    let owner = fixture_owner();
    let mut meter = quire_exact::Meter::new(crate::check::family::SCALAR_LIMITS_UNLIMITED);
    let mut lowering = Lowering::new(
        &scope,
        &owner,
        models,
        units,
        &lock,
        crate::check::MAX_CHECKING_DEPTH,
        0,
        &mut occurrences,
        &mut meter,
    );
    let built = build(&mut lowering);
    (built, lowering.finish(&generated_location()).graph)
}

// ---------------------------------------------------------------------
// TC-417: model declaration, `Reference<T>` and `Population<T>[N]` nodes.
// ---------------------------------------------------------------------

/// TC-417 steps 1-2 (FR-094-AC-1, AC-2): `r`'s type node is R1 over M1 and
/// `s`'s model node M3; under `2.0.0`, M2 and R2. The correspondence holds
/// exactly the two model nodes.
#[trace("FR-094-AC-1", "FR-094-AC-2", "FR-094-CON-2", "TC-417")]
#[test]
fn reference_types_key_over_their_model_nodes_and_record_the_correspondence() {
    let g = || {
        function(
            "g",
            &[("r", named("M::Order")), ("s", named("M::Invoice"))],
            builtin(BuiltinType::Boolean),
            Expression::Boolean(true),
        )
    };
    let acme = admitted("1.0.0");
    let checked = check(&acme, vec![g()]);
    let graph = checked.semantic_graph();
    assert_eq!(
        parameter(graph, "r", 0).semantic_type(),
        Some(vector_key("R1"))
    );
    for vector in ["M1", "R1", "M3", "R5"] {
        assert_holds(graph, vector);
    }
    let m1 = preimage(graph.node(vector_key("M1")).unwrap());
    assert_eq!(
        m1["owner"],
        json!({"kind": "model", "identity": "acme/orders", "version": "1.0.0", "node": "ix://acme/orders/Order"})
    );
    assert!(m1["declaration"].is_null());
    assert!(preimage(graph.node(vector_key("R1")).unwrap())
        .get("owner")
        .is_none());

    // FR-094-AC-2: exactly (M1, Order) and (M3, Invoice).
    let model_nodes: Vec<NodeKey> = graph
        .nodes()
        .filter(|node| matches!(node.node_tag(), NodeTag::Model | NodeTag::Relation))
        .map(SemanticNode::key)
        .collect();
    assert_eq!(model_nodes, {
        let mut expected = vec![vector_key("M1"), vector_key("M3")];
        expected.sort();
        expected
    });
    assert_eq!(
        checked.resolve_declaration(vector_key("M1")),
        Some(&key("Order"))
    );
    assert_eq!(
        checked.resolve_declaration(vector_key("M3")),
        Some(&key("Invoice"))
    );
    assert_eq!(checked.resolve_declaration(vector_key("R1")), None);

    // FR-094-CON-2: every owned node without a source declaration is
    // model-owned and carries a `ModelOwner`.
    for node in graph.nodes().filter(|node| node.declaration().is_none()) {
        if let Some(owner) = preimage(node).get("owner") {
            assert_eq!(owner["kind"], json!("model"), "{}", node.key());
        }
    }

    let acme_2 = admitted("2.0.0");
    let checked_2 = check(&acme_2, vec![g()]);
    let graph_2 = checked_2.semantic_graph();
    assert_eq!(
        parameter(graph_2, "r", 0).semantic_type(),
        Some(vector_key("R2"))
    );
    assert_holds(graph_2, "M2");
    assert_holds(graph_2, "R2");
    assert_ne!(vector_key("M1"), vector_key("M2"));
    assert_ne!(vector_key("R1"), vector_key("R2"));
}

/// FR-093 `Equality` row (FR-093-AC-3): `r = s` over two
/// `Reference<M::Order>` parameters is `quire.op.reference.eq`, with no
/// member, mode, law or leaf.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn reference_equality_is_the_reference_eq_operation() {
    let acme = admitted("1.0.0");
    let checked = check(
        &acme,
        vec![function(
            "same",
            &[("r", named("M::Order")), ("s", named("M::Order"))],
            builtin(BuiltinType::Boolean),
            Expression::Binary {
                operator: BinaryOperator::Equal,
                left: Box::new(name("r")),
                right: Box::new(name("s")),
            },
        )],
    );
    let equal = applications(checked.semantic_graph(), "quire.op.reference.eq");
    assert_eq!(equal.len(), 1);
    let equal = preimage(equal[0]);
    assert_eq!(equal["body"]["operator"], "binary");
    let operation = &equal["body"]["operation"];
    assert_eq!(operation["member"], Json::Null);
    assert_eq!(operation["mode"], Json::Null);
    assert_eq!(operation["laws"], json!([]));
    assert_eq!(operation["leaves"], json!([]));
    assert_eq!(equal["body"]["arguments"].as_array().map(Vec::len), Some(2));
}

/// TC-417 step 3 (FR-094-AC-1): the relationship `billedTo` keys to M4.
#[trace("FR-094-AC-1", "TC-417")]
#[test]
fn a_relationship_keys_to_its_relation_node() {
    let acme = admitted("1.0.0");
    let (billed_to, graph) = lower(
        std::slice::from_ref(&acme.model),
        Default::default(),
        |lowering| lowering.model_node(&key("billedTo"), &generated_location()),
    );
    assert_vector(&graph, billed_to.expect("billedTo keys"), "M4");
}

/// TC-417 steps 4-5 (FR-094-AC-3, AC-4): the population and reference
/// parameters, their type nodes and the model rows are P5, PO1 (over S1),
/// P6, E4 (S2), E5 (R1), E6 (R3), E7, E8 and E9; `Population<Order>[5]` and
/// `Population<Invoice>[3]` are PO2 and PO3.
///
/// FR-093-AC-8: the checked `Attribute`, `AllInstances`, `Lookup` (both
/// absence modes) and `Dispatch` nodes lower to their rows' operation,
/// member, mode and arguments. A `Pre` node is checked only in a standalone
/// postcondition expression, which `check` does not lower into the package
/// graph.
#[trace("FR-094-AC-3", "FR-094-AC-4", "TC-417", "FR-093-AC-8", "TC-415")]
#[test]
fn population_reference_and_model_rows_match_their_vectors() {
    let acme = admitted("1.0.0");
    let parameters = [("p", population("M::Order", 3)), ("r", named("M::Order"))];
    let all_instances = Expression::AllInstances {
        target: named("M::Order"),
        population: Box::new(name("p")),
    };
    let lookup = |absence| Expression::Lookup {
        target: named("M::Order"),
        population: Box::new(name("p")),
        reference: Box::new(name("r")),
        absence,
    };
    let mut declarations = dispatch(&acme, "Order/size");
    declarations.functions.extend([
        function(
            "f1",
            &parameters,
            builtin(BuiltinType::Integer),
            Expression::Size(Box::new(all_instances)),
        ),
        function(
            "f2",
            &parameters,
            named("M::Order"),
            lookup(qsl_foundation::absence::AbsenceMode::Undefined),
        ),
        function(
            "f3",
            &parameters,
            builtin(BuiltinType::Boolean),
            Expression::Present(Box::new(lookup(
                qsl_foundation::absence::AbsenceMode::Empty,
            ))),
        ),
        function(
            "f4",
            &parameters,
            builtin(BuiltinType::Int).with_bounds(vec!["0".to_owned(), "9".to_owned()]),
            Expression::Field {
                operand: Box::new(Expression::Deref(Box::new(name("r")))),
                field: "total".to_owned(),
            },
        ),
        // A dispatched call checks only inside a clause (FR-151).
        FunctionDeclaration::clause(
            "f5",
            parameters
                .iter()
                .map(|(name, form)| ((*name).to_owned(), form.clone()))
                .collect(),
            builtin(BuiltinType::Boolean),
            None,
            Expression::Binary {
                operator: BinaryOperator::GreaterOrEqual,
                left: Box::new(Expression::Dispatch {
                    receiver: Box::new(name("r")),
                    member: "size".to_owned(),
                    arguments: Vec::new(),
                }),
                right: Box::new(Expression::Integer(Integer::from(0_i64))),
            },
            DeclaredClauseKind::Precondition,
        ),
        function(
            "f6",
            &[
                ("p", population("M::Order", 5)),
                ("q", population("M::Invoice", 3)),
            ],
            builtin(BuiltinType::Boolean),
            Expression::Boolean(true),
        ),
    ]);
    let checked = check_declarations(declarations);
    let graph = checked.semantic_graph();

    assert_vector(graph, parameter(graph, "p", 0).key(), "P5");
    assert_vector(graph, parameter(graph, "r", 1).key(), "P6");
    for vector in [
        "PO1", "S1", "E4", "S2", "E5", "E6", "R3", "E7", "E8", "E9", "PO2", "PO3", "S3", "R5",
    ] {
        assert_holds(graph, vector);
    }
    assert_eq!(
        graph.node(vector_key("PO1")).unwrap().semantic_type(),
        Some(vector_key("S1"))
    );
    assert_ne!(vector_key("PO1"), vector_key("PO3"));
    assert_eq!(
        member_declaration(graph.node(vector_key("E8")).unwrap()),
        json!(vector_key("M1").to_string())
    );
    assert_eq!(
        member_declaration(graph.node(vector_key("E9")).unwrap()),
        json!(vector_key("M1").to_string())
    );
}

/// TC-417 step 6 (FR-094-AC-4): over `s: Reference<M::Sub>` (R4), the
/// attribute and dispatch members name `Sub`'s model node M5, not `Order`'s
/// M1, though `Order` declares `total`.
#[trace("FR-094-AC-4", "TC-417")]
#[test]
fn model_members_name_the_static_object_type() {
    let acme = admitted("1.0.0");
    let mut declarations = dispatch(&acme, "Sub/size");
    declarations.functions.extend([
        function(
            "total_of",
            &[("s", named("M::Sub"))],
            builtin(BuiltinType::Int).with_bounds(vec!["0".to_owned(), "9".to_owned()]),
            Expression::Field {
                operand: Box::new(Expression::Deref(Box::new(name("s")))),
                field: "total".to_owned(),
            },
        ),
        FunctionDeclaration::clause(
            "sized",
            vec![("s".to_owned(), named("M::Sub"))],
            builtin(BuiltinType::Boolean),
            None,
            Expression::Binary {
                operator: BinaryOperator::GreaterOrEqual,
                left: Box::new(Expression::Dispatch {
                    receiver: Box::new(name("s")),
                    member: "size".to_owned(),
                    arguments: Vec::new(),
                }),
                right: Box::new(Expression::Integer(Integer::from(0_i64))),
            },
            DeclaredClauseKind::Precondition,
        ),
    ]);
    let checked = check_declarations(declarations);
    let graph = checked.semantic_graph();
    assert_eq!(
        parameter(graph, "s", 0).semantic_type(),
        Some(vector_key("R4"))
    );
    assert_holds(graph, "R4");
    assert_holds(graph, "M5");
    let m5 = json!(vector_key("M5").to_string());
    let project = applications(graph, "quire.op.record.project");
    let call = applications(graph, "quire.op.model.dispatch_call");
    assert_eq!(project.len(), 1);
    assert_eq!(call.len(), 1);
    assert_eq!(member_declaration(project[0]), m5);
    assert_eq!(member_declaration(call[0]), m5);
}

/// TC-417 step 7 (FR-094-AC-7): an unmapped `EffectiveId`, an unadmitted
/// package, an empty node and a record kind no checked node names each
/// refuse as an internal fault naming the value, with no key.
#[trace("FR-094-AC-7", "TC-417")]
#[test]
fn unmappable_model_inputs_are_internal_faults() {
    let acme = admitted("1.0.0");
    let models = std::slice::from_ref(&acme.model);
    let location = generated_location();
    let unknown = EffectiveId::from_digest([9; 32]);
    let other = DeclarationKey {
        package: "acme/other".to_owned(),
        node: "ix://acme/other/Order".to_owned(),
    };
    let empty = DeclarationKey {
        package: PACKAGE.to_owned(),
        node: String::new(),
    };
    let ((by_id, by_package, by_node, by_kind), graph) =
        lower(models, Default::default(), |lowering| {
            (
                lowering.reference_type(unknown, &location),
                lowering.model_node(&other, &location),
                lowering.model_node(&empty, &location),
                lowering.model_node(&key("Order/total"), &location),
            )
        });
    let fault = |result: Result<NodeKey, CheckRefusal>| match result {
        Err(CheckRefusal {
            cause: CheckCause::InternalFault(fault),
            ..
        }) => *fault,
        other => panic!("expected an internal fault, got {other:?}"),
    };
    assert_eq!(fault(by_id), KeyFault::UnknownEffectiveId(unknown));
    assert_eq!(fault(by_package), KeyFault::UnadmittedPackage(other));
    assert_eq!(fault(by_node), KeyFault::EmptyNode(empty));
    assert_eq!(
        fault(by_kind),
        KeyFault::UnnamedRecordKind(key("Order/total"))
    );
    assert_eq!(graph.nodes().count(), 0, "no node was keyed");
    let refusal = CheckCause::InternalFault(Box::new(KeyFault::UnknownEffectiveId(unknown)));
    assert_eq!(
        refusal.code(),
        qsl_foundation::diagnostic::Code::RuntimeInvariant
    );
    assert_eq!(refusal.cause(), Some("established-invariant-broken"));
}

/// TC-417 step 8, TC-418 step 6, TC-419 step 4 (FR-094-CON-1): the
/// record-kind, clause-kind and unit-domain matches have no `_` arm.
#[trace("FR-094-CON-1", "TC-417", "TC-418", "TC-419")]
#[test]
fn the_model_matches_have_no_catch_all_arm() {
    let source = include_str!("../model.rs");
    let body_of = |signature: &str| {
        let start = source.find(signature).expect("the function exists");
        let end = source[start..]
            .find("\n}\n")
            .or_else(|| source[start..].find("\n    }\n"))
            .map(|offset| start + offset)
            .expect("the function ends");
        &source[start..end]
    };
    for signature in [
        "fn record_form(",
        "fn clause_spelling(",
        "fn quantity_type(",
    ] {
        assert!(
            !body_of(signature).contains("_ =>"),
            "{signature} has a catch-all arm"
        );
    }
}

// ---------------------------------------------------------------------
// TC-418: clause functions.
// ---------------------------------------------------------------------

/// The clause function `graph` keys for `member`'s `clause`, by owner node
/// and `clause` binding.
fn clause_function<'g>(
    graph: &'g SemanticGraph,
    member: &str,
    clause: &str,
) -> Vec<&'g SemanticNode> {
    graph
        .nodes()
        .filter(|node| {
            let preimage = preimage(node);
            node.node_tag() == NodeTag::Function
                && preimage["owner"]["node"] == json!(format!("ix://acme/orders/{member}"))
                && preimage["body"]["members"]
                    .as_array()
                    .and_then(|members| members.last())
                    .is_some_and(|last| {
                        last["name"] == "clause" && last["value"]["value"] == clause
                    })
        })
        .collect()
}

/// TC-418 steps 1, 3 and 5 (FR-094-AC-5): `Order.size`'s precondition and
/// body are C1 and C2 over P7, `Order.count`'s precondition is C4, and
/// renaming the synthesized labels leaves every key unchanged.
#[trace("FR-094-AC-5", "FR-094-CON-2", "TC-418")]
#[test]
fn clause_functions_key_under_their_operation_members_model_owner() {
    let acme = admitted("1.0.0");
    let size = check_declarations(dispatch(&acme, "Order/size"));
    let graph = size.semantic_graph();
    assert_holds(graph, "P7");
    assert_holds(graph, "C1");
    assert_holds(graph, "C2");
    let c1 = preimage(graph.node(vector_key("C1")).unwrap());
    assert_eq!(
        c1["owner"],
        json!({"kind": "model", "identity": "acme/orders", "version": "1.0.0", "node": "ix://acme/orders/Order/size"})
    );
    assert!(c1["declaration"].is_null());

    let count = check_declarations(dispatch(&acme, "Order/count"));
    assert_holds(count.semantic_graph(), "C4");

    let mut relabelled = dispatch(&acme, "Order/size");
    for (index, function) in relabelled.functions.iter_mut().enumerate() {
        function.name = format!("renamed_{index}");
    }
    let relabelled = check_declarations(relabelled);
    let keys = |graph: &SemanticGraph| graph.nodes().map(SemanticNode::key).collect::<Vec<_>>();
    assert_eq!(keys(relabelled.semantic_graph()), keys(graph));
}

/// TC-418 step 2 (FR-094-AC-5): under `2.0.0`, the receiver parameter
/// keys to P9 over R2 and `Order.size`'s precondition to C3, which
/// references P9 and differs from C1.
#[trace("FR-094-AC-5", "TC-418")]
#[test]
fn a_clause_function_under_another_package_version_is_another_node() {
    let acme = admitted("2.0.0");
    let checked = check_declarations(dispatch(&acme, "Order/size"));
    let graph = checked.semantic_graph();
    for vector in ["R2", "P9", "C3"] {
        assert_holds(graph, vector);
    }
    let c3 = preimage(graph.node(vector_key("C3")).unwrap());
    assert_eq!(c3["owner"]["version"], json!("2.0.0"));
    assert_eq!(
        c3["body"]["members"][0]["value"]["members"][0]["target"]["digest"],
        json!(vector_key("P9").to_string())
    );
    assert_ne!(vector_key("C3"), vector_key("C1"));
}

/// FR-094: a check selects one version of each domain package identity;
/// two admitted versions of `acme/orders` refuse as a broken invariant
/// instead of keying owners from whichever comes first.
#[trace("FR-094-AC-5", "TC-418")]
#[test]
fn two_admitted_versions_of_one_model_identity_refuse() {
    let (first, second) = (admitted("1.0.0"), admitted("2.0.0"));
    let mut declarations = dispatch(&first, "Order/size");
    declarations.models.push(second.model.clone());
    let refusals = declarations
        .check(CheckingLimits::default())
        .expect_err("two versions of one identity refuse");
    assert!(
        matches!(
            &refusals[0].cause,
            CheckCause::InternalFault(fault)
                if **fault == KeyFault::DuplicateModelSelection("acme/orders".to_owned())
        ),
        "{refusals:?}"
    );
}

/// TC-418 step 4 (FR-094-AC-5): `Sub.size`'s authored precondition is C5 and
/// its effective precondition C6 (over E10) in both dispatch operations,
/// one node each; `Order.size`'s authored precondition stays C1.
#[trace("FR-094-AC-5", "TC-418")]
#[test]
fn a_redefining_candidates_clauses_key_once_under_its_own_owner() {
    let acme = admitted("1.0.0");
    for root in ["Order/size", "Sub/size"] {
        let checked = check_declarations(dispatch(&acme, root));
        let graph = checked.semantic_graph();
        assert_holds(graph, "P8");
        for vector in ["L4", "E10", "C5", "C6"] {
            assert_holds(graph, vector);
        }
        assert_eq!(clause_function(graph, "Sub/size", "precondition").len(), 2);
        let c6 = preimage(graph.node(vector_key("C6")).unwrap());
        assert_eq!(c6["owner"]["node"], json!("ix://acme/orders/Sub/size"));
        if root == "Order/size" {
            assert_holds(graph, "C1");
        }
    }
}

// ---------------------------------------------------------------------
// TC-419: quantity type nodes.
// ---------------------------------------------------------------------

mod quantities {
    use super::*;
    use crate::value::quantity::UnitTable;
    use crate::value::semantic_node::{NodeOwner, OwnerSelection, OwnerSubject};
    use crate::value::unit::{DimensionPreimage, UnitGraph, UnitPreimage};

    fn owner_json() -> Json {
        json!({"kind": "definition", "authority": "agent-ix", "identity": "example-model"})
    }

    fn node_id(key: &str) -> Json {
        json!({"domain": "quire.checked-semantic-node/v1", "digest": key})
    }

    fn dimension(name: &str) -> DimensionPreimage {
        DimensionPreimage::from_json(json!({
            "version": "quire.dimension-node/v1",
            "owner": owner_json(),
            "qualified_declaration": ["Example", name],
            "terms": [],
        }))
        .expect("a base dimension")
    }

    fn unit(name: &str, dimension: &str) -> UnitPreimage {
        UnitPreimage::from_json(json!({
            "version": "quire.unit-node/v1",
            "owner": owner_json(),
            "qualified_declaration": ["Example", name],
            "dimension_node_id": node_id(dimension),
            "target_unit_node_id": null,
            "scale": {"numerator": "1", "denominator": "1"},
            "offset": {"numerator": "0", "denominator": "1"},
        }))
        .expect("a root unit")
    }

    const LENGTH: &str = "b6cc14ab93b670cb0fc74a80dd18131ef7b06e3eee6a730e5ca092266314e22b";
    const TIME: &str = "628777f8df3c2700593d56dc20fb110640214dd09bc6cb961d802fdb9b7f6077";
    const METRE: &str = "79637623a46d29e884b62c6fa292aeb29d41e4ecc4e800b4d7ee910a3eaf23a4";
    const SECOND: &str = "d0f0c5d24d6accb2f1d9afff7c6cdda3143df382e50af4bda0704d48c67ed998";

    fn node_key(hex: &str) -> NodeKey {
        let mut digest = [0_u8; 32];
        for (index, byte) in digest.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[2 * index..2 * index + 2], 16).expect("hex");
        }
        NodeKey::from_digest(digest)
    }

    /// QSpec's `dimension-length`, `dimension-time`, `unit-metre` and
    /// `unit-second` nodes, admitted under their recomputed keys (`admit`
    /// refuses a stale key, so the keys are QSpec's vector keys FR-094
    /// names).
    fn units() -> UnitTable {
        let graph = UnitGraph::admit(
            [
                (dimension("Length"), node_key(LENGTH)),
                (dimension("Time"), node_key(TIME)),
            ],
            [
                (unit("metre", LENGTH), node_key(METRE)),
                (unit("second", TIME), node_key(SECOND)),
            ],
            &OwnerSelection::new([NodeOwner::Definition(OwnerSubject {
                authority: "agent-ix".into(),
                identity: "example-model".into(),
            })]),
        )
        .expect("the QSpec unit vectors admit");
        UnitTable::declared(&graph)
    }

    fn quantity(hex: &str) -> ValueType {
        ValueType::Quantity(UnitId::declared(node_key(hex)))
    }

    /// TC-419 steps 1-2 (FR-094-AC-6): a declared unit's quantity is typed
    /// by the unit's nominal node, with no `compound_unit` node; the result
    /// type of `a * a`, formed while `check` types the body, is U1.
    ///
    /// `a / t`, `a / a` and `a * a / a` do not check as a function body:
    /// no guard proves a quantity divisor nonzero (`facts`'s
    /// `NodeKind::Quantity(Divide)` refuses `unproved-nonzero`), so their
    /// result types U2 to U4 are keyed from the units `result_unit` forms
    /// for them, the same forming `check` does while typing.
    #[trace("FR-094-AC-6", "TC-419")]
    #[test]
    fn quantity_types_key_to_the_unit_node_or_a_compound_unit_node() {
        let package = |body: Expression| PackageDeclarations {
            types: TypeEnvironment::default().with_units(units()),
            aliases: vec![
                ("Length".to_owned(), quantity(METRE)),
                ("Time".to_owned(), quantity(SECOND)),
            ],
            functions: vec![function(
                "q",
                &[("a", named("Length")), ("t", named("Time"))],
                builtin(BuiltinType::Boolean),
                body,
            )],
            ..PackageDeclarations::new(fixture_owner())
        };
        let plain = check_declarations(package(Expression::Boolean(true)));
        let graph = plain.semantic_graph();
        assert_eq!(
            parameter(graph, "a", 0).semantic_type(),
            Some(node_key(METRE))
        );
        assert!(graph
            .nodes()
            .all(|node| node.semantic_form() != "compound_unit"));

        let square = Expression::Binary {
            operator: BinaryOperator::Multiply,
            left: Box::new(name("a")),
            right: Box::new(name("a")),
        };
        let squared = check_declarations(package(Expression::Binary {
            operator: BinaryOperator::Equal,
            left: Box::new(square.clone()),
            right: Box::new(square),
        }));
        let graph = squared.semantic_graph();
        assert_holds(graph, "U1");
        let product = applications(graph, "quire.op.quantity.mul");
        assert_eq!(product.len(), 1);
        assert_eq!(product[0].semantic_type(), Some(vector_key("U1")));

        use crate::value::quantity::{result_unit, UnitOperation};
        let table = units();
        let unit = |hex: &str| {
            table
                .get(UnitId::declared(node_key(hex)))
                .expect("an admitted unit")
                .clone()
        };
        let (metre, second) = (unit(METRE), unit(SECOND));
        let form = |operation, left: &QuantityUnit, right: &QuantityUnit| {
            result_unit(operation, left, right).expect("the unit forms")
        };
        let per_second = form(UnitOperation::Divide, &metre, &second);
        let ratio = form(UnitOperation::Divide, &metre, &metre);
        let square = form(UnitOperation::Multiply, &metre, &metre);
        let back = form(UnitOperation::Divide, &square, &metre);
        let mut formed = table.clone();
        formed.extend([per_second.clone(), ratio.clone(), back.clone()]);
        let (keys, graph) = lower(&[], formed, |lowering| {
            [&per_second, &ratio, &back].map(|unit| {
                lowering
                    .type_node(&ValueType::Quantity(unit.id()), &generated_location())
                    .expect("a held compound unit keys")
            })
        });
        for (key, vector) in keys.into_iter().zip(["U2", "U3", "U4"]) {
            assert_vector(&graph, key, vector);
        }
        let u2 = preimage(graph.node(vector_key("U2")).unwrap());
        assert_eq!(
            u2["body"]["members"][0]["members"][0]["value"]["target"]["digest"],
            json!(METRE)
        );
        assert_ne!(vector_key("U4"), node_key(METRE));
    }

    /// FR-093 `ConvertScalar` quantity row (FR-093-AC-3): converting a
    /// metre quantity to kilometres is `quire.op.quantity.convert`, with
    /// member `type_argument` of the kilometre unit's node and mode
    /// `rounding` = `exact`.
    #[trace("FR-093-AC-3", "TC-415")]
    #[test]
    fn a_quantity_conversion_is_exact() {
        use crate::value::semantic_node::NodeIdentityPreimage;
        let kilometre = UnitPreimage::from_json(json!({
            "version": "quire.unit-node/v1",
            "owner": owner_json(),
            "qualified_declaration": ["Example", "kilometre"],
            "dimension_node_id": node_id(LENGTH),
            "target_unit_node_id": node_id(METRE),
            "scale": {"numerator": "1000", "denominator": "1"},
            "offset": {"numerator": "0", "denominator": "1"},
        }))
        .expect("a derived unit");
        let km = NodeKey::from_digest(kilometre.digest().expect("the unit digests"));
        let graph = UnitGraph::admit(
            [
                (dimension("Length"), node_key(LENGTH)),
                (dimension("Time"), node_key(TIME)),
            ],
            [
                (unit("metre", LENGTH), node_key(METRE)),
                (unit("second", TIME), node_key(SECOND)),
                (kilometre, km),
            ],
            &OwnerSelection::new([NodeOwner::Definition(OwnerSubject {
                authority: "agent-ix".into(),
                identity: "example-model".into(),
            })]),
        )
        .expect("metre, second and kilometre admit");
        let km_type = ValueType::Quantity(UnitId::declared(km));
        let checked = check_declarations(PackageDeclarations {
            types: TypeEnvironment::default().with_units(UnitTable::declared(&graph)),
            aliases: vec![
                ("Length".to_owned(), quantity(METRE)),
                ("Km".to_owned(), km_type),
            ],
            functions: vec![function(
                "to_km",
                &[("a", named("Length"))],
                named("Km"),
                Expression::Convert {
                    target: named("Km"),
                    operand: Box::new(name("a")),
                },
            )],
            ..PackageDeclarations::new(fixture_owner())
        });
        let convert = applications(checked.semantic_graph(), "quire.op.quantity.convert");
        assert_eq!(convert.len(), 1);
        let convert = preimage(convert[0]);
        assert_eq!(convert["body"]["operator"], "convert");
        let operation = &convert["body"]["operation"];
        assert_eq!(
            operation["member"],
            json!({"kind": "type_argument", "declaration": node_id(&km.to_string())})
        );
        assert_eq!(
            operation["mode"],
            json!({"kind": "rounding", "value": "exact"})
        );
        assert_eq!(operation["laws"], json!([]));
        assert_eq!(operation["leaves"], json!([]));
    }

    /// TC-419 step 3 (FR-094-AC-7): a compound `UnitId` the unit scope does
    /// not hold is an internal fault naming it, with no key.
    #[trace("FR-094-AC-7", "TC-419")]
    #[test]
    fn an_unheld_compound_unit_is_an_internal_fault() {
        let unheld = UnitId::compound([5; 32]);
        let (result, graph) = lower(&[], units(), |lowering| {
            lowering.type_node(&ValueType::Quantity(unheld), &generated_location())
        });
        assert!(matches!(
            result,
            Err(CheckRefusal {
                cause: CheckCause::InternalFault(fault),
                ..
            }) if *fault == KeyFault::UnheldUnit(unheld)
        ));
        assert_eq!(graph.nodes().count(), 0);
    }
}
