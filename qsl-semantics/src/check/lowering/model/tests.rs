// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-417, TC-418 and TC-419: FR-094's model-owned, `Reference<T>`,
//! `Population<T>[N]`, clause-function and quantity type nodes, checked
//! structurally: which nodes exist, what each preimage carries and which
//! node each reference names.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_forms::{
    BinaryOperator, BuiltinType, DeclaredClauseKind, Expression, FunctionDeclaration, TypeForm,
    TypeFormHead,
};
use quire_exact::{Integer, IntegerInterval, Presence};
use serde_json::{json, Value as Json};

use super::super::{generated_location, type_declaration, Lowering, SemanticGraph, SemanticNode};
use super::super::tests::view::preimage;
use super::*;
use crate::check::checked_dispatch::{checked_dispatch_operation, DispatchRoot, OperationClauses};
use crate::check::family::fixtures::{empty_scope, fixture_source};
use crate::check::family::{OccurrenceMap, OccurrenceRole};
use crate::check::{CheckedGraph, PackageDeclarations};
use crate::model::accounting::{Meter, ModelNormalizationLimits};
use crate::model::dispatch::GeneralizationClosure;
use crate::model::domain_package::{
    FieldMemberRecord, Multiplicity, NativeValueType, ObjectTypeRecord, OperationEffect,
    OperationMemberRecord, OperationParameterRecord, OperationResult, PersistenceInterfaceRecord,
    RecordValueTypeRecord, RelationshipDirection, RelationshipEnd, RelationshipRecord,
    StateMachineRecord, StateMachineStateRecord, StateMachineTransitionRecord, ValueTypeRef,
};
use crate::model::normalize::{normalize, NormalizeOutcome};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::{FieldDeclaration, ObjectTypeDeclaration, TypeEnvironment};
use quire_semantic_value::location::Location;

mod expression_depth;
mod place_rows;

const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

/// Every application node whose operation is `identity`.
fn applications<'g>(graph: &'g SemanticGraph, identity: &str) -> Vec<&'g SemanticNode> {
    graph
        .nodes()
        .filter(|node| preimage(node)["body"]["operation"]["identity"] == json!(identity))
        .collect()
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

fn member_declaration(node: &SemanticNode) -> Json {
    preimage(node)["body"]["operation"]["member"]["declaration"]["digest"].clone()
}

/// The model node of the declaration `name` of `acme/orders` in `graph`.
fn model_node_of(graph: &SemanticGraph, name: &str) -> NodeKey {
    let found: Vec<NodeKey> = graph
        .nodes()
        .filter(|node| {
            preimage(node)["owner"]["node"] == json!(format!("ix://acme/orders/{name}"))
                && matches!(node.node_tag(), NodeTag::Model | NodeTag::Relation)
        })
        .map(SemanticNode::key)
        .collect();
    assert_eq!(found.len(), 1, "one model node for {name}");
    found[0]
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
    query_over(name, owner, redefines, &[])
}

/// [`query`] over the `Integer` parameters `parameters`, keyed
/// `<name>/<parameter>`.
fn query_over(
    name: &str,
    owner: &str,
    redefines: Option<&str>,
    parameters: &[&str],
) -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: key(name),
        owner: key(owner),
        parameters: parameters
            .iter()
            .map(|parameter| OperationParameterRecord {
                key: key(&format!("{name}/{parameter}")),
                value_type: ValueTypeRef::Native(NativeValueType::Integer),
                multiplicity: one(),
            })
            .collect(),
        result: Some(OperationResult {
            value_type: ValueTypeRef::Native(NativeValueType::Integer),
            multiplicity: one(),
        }),
        effect: OperationEffect::default(),
        requires: Vec::new(),
        ensures: Vec::new(),
        has_body: true,
        redefines: redefines.map(key),
    })
}

/// `acme/orders` at `version`: object types `Order` (attribute `total`,
/// queries `size`, `count` and `scaled(n)`), `Invoice` and `Sub` (supertype
/// `Order`, redefining `size`, and `scaled` as `scaled(m)`), and the
/// relationship `billedTo`.
fn acme() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef {
            identity: PACKAGE.to_owned(),
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
                presence: Presence::Required,
                subsets: Vec::new(),
                redefines: None,
                position: 0,
            }),
            query("Order/size", "Order", None),
            query("Order/count", "Order", None),
            query("Sub/size", "Sub", Some("Order/size")),
            query_over("Order/scaled", "Order", None, &["n"]),
            query_over("Sub/scaled", "Sub", Some("Order/scaled"), &["m"]),
            DomainPackageRecord::Relationship(RelationshipRecord {
                key: key("billedTo"),
                declared_at: None,
                source: RelationshipEnd {
                    type_identity: key("Order"),
                    role: Some("order".to_owned()),
                    multiplicity: Some(one()),
                },
                target: RelationshipEnd {
                    type_identity: key("Invoice"),
                    role: Some("invoice".to_owned()),
                    multiplicity: Some(one()),
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
    let package = acme();
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

/// `M::Order`, `M::Invoice` and `M::Sub`, as the acme model declares them:
/// `Order` declares `total: Int[0, 9]` and `Sub` inherits it.
fn types(acme: &Acme) -> TypeEnvironment {
    let total = vec![FieldDeclaration::new(
        "total",
        int(0, 9),
        Presence::Required,
    )];
    TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(acme.order, "M::Order", total),
            ObjectTypeDeclaration::new(acme.invoice, "M::Invoice", Vec::new()),
            ObjectTypeDeclaration::new(acme.sub, "M::Sub", Vec::new())
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
    Expression::name(name.to_owned())
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
        ..PackageDeclarations::new(fixture_source(), "fixture", qsl_foundation::IdentityLimits::default())
    }
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("the acme functions check: {refusals:?}"))
}

/// The `OperationClauses` of `Order.size` (precondition `true`, body `7`),
/// `Order.count` (precondition `true`, body `1`) and `Sub.size`
/// (precondition `false`, body `8`), each over `self`; and of
/// `Order.scaled(self, n: Integer)` (precondition `true`, body `n`) and
/// `Sub.scaled(this, m: Integer)` (precondition `false`, body `m`), whose
/// redefinition renames both parameters.
fn clauses(acme: &Acme) -> OperationClauses {
    let mut clauses = OperationClauses::default();
    let literal = |value: i64| Expression::integer(Integer::from(value));
    let operations = [
        (
            "Order/size",
            "size",
            acme.order,
            "self",
            None,
            true,
            literal(7),
        ),
        (
            "Order/count",
            "count",
            acme.order,
            "self",
            None,
            true,
            literal(1),
        ),
        (
            "Sub/size",
            "size",
            acme.sub,
            "self",
            None,
            false,
            literal(8),
        ),
        (
            "Order/scaled",
            "scaled",
            acme.order,
            "self",
            Some("n"),
            true,
            name("n"),
        ),
        (
            "Sub/scaled",
            "scaled",
            acme.sub,
            "this",
            Some("m"),
            false,
            name("m"),
        ),
    ];
    for (operation, member, receiver, receiver_name, argument, precondition, body) in operations {
        let operation = key(operation);
        clauses.member.insert(operation.clone(), member.to_owned());
        let mut parameters = vec![(receiver_name.to_owned(), ValueType::Reference(receiver))];
        parameters.extend(argument.map(|argument| (argument.to_owned(), ValueType::Integer)));
        clauses.parameters.insert(operation.clone(), parameters);
        clauses.result.insert(operation.clone(), ValueType::Integer);
        clauses
            .own_precondition
            .insert(operation.clone(), Expression::boolean(precondition));
        clauses.own_body.insert(operation, body);
    }
    clauses
}

/// `checked_dispatch_operation` rooted at `root`, with the acme types.
fn dispatch(acme: &Acme, root: &str) -> PackageDeclarations {
    dispatch_with(acme, root, &clauses(acme))
}

/// [`dispatch`] over `clauses` in place of [`clauses`].
fn dispatch_with(acme: &Acme, root: &str, clauses: &OperationClauses) -> PackageDeclarations {
    let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
    let mut declarations = checked_dispatch_operation(
        &acme.view,
        &DispatchRoot {
            key: key(root),
            closure: GeneralizationClosure::Closed,
        },
        clauses,
        fixture_source(),
        "fixture".to_owned(),
        &mut meter,
        qsl_foundation::IdentityLimits::default(),
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
    units: quire_semantic_value::quantity::UnitTable,
    build: impl FnOnce(&mut Lowering<'_>) -> T,
) -> (T, SemanticGraph) {
    let (built, graph, _) = lower_with_occurrences(models, units, build);
    (built, graph)
}

/// Shared lowering setup for tests that need to inspect occurrences.
fn lower_with_occurrences<T>(
    models: &[AdmittedModel],
    units: quire_semantic_value::quantity::UnitTable,
    build: impl FnOnce(&mut Lowering<'_>) -> T,
) -> (T, SemanticGraph, OccurrenceMap<Location>) {
    let scope = empty_scope();
    let mut occurrences = OccurrenceMap::default();
    let owner = crate::check::SourceOwner::from(&fixture_source());
    let mut meter = quire_exact::Meter::new(crate::check::family::SCALAR_LIMITS_UNLIMITED);
    let mut lowering = Lowering::new(
        &scope,
        &owner,
        models,
        units,
        None,
        0,
        &mut occurrences,
        &mut meter,
        "fixture",
    );
    let built = build(&mut lowering);
    let graph = lowering.finish(&generated_location()).graph;
    (built, graph, occurrences)
}

// ---------------------------------------------------------------------
// TC-417: model declaration, `Reference<T>` and `Population<T>[N]` nodes.
// ---------------------------------------------------------------------

/// TC-417 steps 1-2 (FR-094-AC-1, AC-2): `r`'s type node is a reference
/// type over `Order`'s model node and `s`'s is over `Invoice`'s; the
/// correspondence holds exactly the two model nodes.
#[trace("FR-094-AC-1", "FR-094-AC-2", "FR-094-CON-2", "TC-417")]
#[test]
fn reference_types_key_over_their_model_nodes_and_record_the_correspondence() {
    let g = function(
        "g",
        &[("r", named("M::Order")), ("s", named("M::Invoice"))],
        builtin(BuiltinType::Boolean),
        Expression::boolean(true),
    );
    let acme = admitted("1.0.0");
    let checked = check(&acme, vec![g]);
    let graph = checked.semantic_graph();
    let r_type = parameter(graph, "r", 0).semantic_type().expect("typed");
    let s_type = parameter(graph, "s", 1).semantic_type().expect("typed");
    assert_ne!(r_type, s_type);
    let m1 = model_node_of(graph, "Order");
    let m3 = model_node_of(graph, "Invoice");
    let m1_preimage = preimage(graph.node(m1).unwrap());
    assert_eq!(
        m1_preimage["owner"],
        json!({"kind": "model", "identity": "acme/orders", "node": "ix://acme/orders/Order"})
    );
    assert!(
        m1_preimage["owner"].get("version").is_none(),
        "no version member"
    );
    assert!(m1_preimage["declaration"].is_null());
    let r = preimage(graph.node(r_type).unwrap());
    assert!(r.get("owner").is_none());
    assert!(
        serde_json::to_string(&r["body"])
            .unwrap()
            .contains(&m1.to_string()),
        "the reference type's body references Order's model node"
    );

    // FR-094-AC-2: exactly (Order, M1) and (Invoice, M3).
    let model_nodes: Vec<NodeKey> = graph
        .nodes()
        .filter(|node| matches!(node.node_tag(), NodeTag::Model | NodeTag::Relation))
        .map(SemanticNode::key)
        .collect();
    assert_eq!(model_nodes, {
        let mut expected = vec![m1, m3];
        expected.sort();
        expected
    });
    assert_eq!(checked.resolve_declaration(m1), Some(&key("Order")));
    assert_eq!(checked.resolve_declaration(m3), Some(&key("Invoice")));
    assert_eq!(checked.resolve_declaration(r_type), None);

    // FR-094-CON-2: every owned node without a source declaration is
    // model-owned and carries a `ModelOwner`.
    for node in graph.nodes().filter(|node| node.declaration().is_none()) {
        if let Some(owner) = preimage(node).get("owner") {
            assert_eq!(owner["kind"], json!("model"), "{}", node.key());
        }
    }
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
            Expression::binary(BinaryOperator::Equal, name("r"), name("s")),
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

/// TC-417 step 3 (FR-094-AC-1): the relationship `billedTo` keys to a
/// relation node.
#[trace("FR-094-AC-1", "TC-417")]
#[test]
fn a_relationship_keys_to_its_relation_node() {
    let acme = admitted("1.0.0");
    let (billed_to, graph) = lower(
        std::slice::from_ref(&acme.model),
        Default::default(),
        |lowering| lowering.model_node(&key("billedTo"), &generated_location()),
    );
    let node = graph
        .node(billed_to.expect("billedTo keys"))
        .expect("the relation node is in the graph");
    assert_eq!(node.node_tag(), NodeTag::Relation);
}

/// A selected relationship is a model graph root even when no checked value
/// or frame names it yet.
#[trace("FR-093-AC-27", "FR-094-AC-9", "TC-417")]
#[test]
fn selected_relationships_are_rooted_in_the_checked_graph() {
    let acme = admitted("1.0.0");
    let model = acme.model.clone().with_alias("M");
    let declaration = type_declaration("M");
    let (result, graph, occurrences) = lower_with_occurrences(
        std::slice::from_ref(&model),
        Default::default(),
        |lowering| {
            lowering.relationship_nodes(&BTreeMap::from([("M".to_owned(), declaration.clone())]))
        },
    );
    result.expect("the selected relationship keys");
    let relations: Vec<_> = graph
        .nodes()
        .filter(|node| node.node_tag() == NodeTag::Relation)
        .collect();
    assert_eq!(relations.len(), 1, "billedTo is one relation node");
    assert_eq!(relations[0].semantic_form(), "relationship");
    let generated: Vec<_> = occurrences.iter_role(OccurrenceRole::Generated).collect();
    assert_eq!(generated.len(), 1, "the unreferenced relation has one root");
    assert_eq!(generated[0].0, relations[0].key());
    assert_eq!(generated[0].2, &declaration);
}

/// FR-093-AC-8: the checked `Attribute`, `AllInstances`, `Lookup` (both
/// absence modes) and `Dispatch` nodes lower to their rows' operation,
/// member, mode and arguments. A `Pre` node is checked only in a standalone
/// postcondition expression, which `check` does not lower into the package
/// graph.
#[trace("FR-094-AC-3", "FR-094-AC-4", "TC-417", "FR-093-AC-8", "TC-415")]
#[test]
fn population_reference_and_model_rows_lower_to_their_operations() {
    let acme = admitted("1.0.0");
    let parameters = [("p", population("M::Order", 3)), ("r", named("M::Order"))];
    let all_instances = Expression::all_instances(named("M::Order"), name("p"));
    let lookup = |absence| Expression::lookup(named("M::Order"), name("p"), name("r"), absence);
    let mut declarations = dispatch(&acme, "Order/size");
    declarations.functions.extend([
        function(
            "f1",
            &parameters,
            builtin(BuiltinType::Integer),
            Expression::size(all_instances),
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
            Expression::present(lookup(qsl_foundation::absence::AbsenceMode::Empty)),
        ),
        function(
            "f4",
            &parameters,
            builtin(BuiltinType::Int).with_bounds(vec!["0".to_owned(), "9".to_owned()]),
            Expression::field(Expression::deref(name("r")), "total".to_owned()),
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
            Expression::binary(
                BinaryOperator::GreaterOrEqual,
                Expression::dispatch(name("r"), "size".to_owned(), Vec::new()),
                Expression::integer(0_i64),
            ),
            DeclaredClauseKind::Precondition,
        ),
        function(
            "f6",
            &[
                ("p", population("M::Order", 5)),
                ("q", population("M::Invoice", 3)),
            ],
            builtin(BuiltinType::Boolean),
            Expression::boolean(true),
        ),
    ]);
    let checked = check_declarations(declarations);
    let graph = checked.semantic_graph();
    let m1 = model_node_of(graph, "Order");

    // FR-094-AC-3: `p` is a population parameter at level 0 and `r` a
    // reference parameter at level 1.
    let p = parameter(graph, "p", 0);
    let r = parameter(graph, "r", 1);
    let population_type = graph
        .node(p.semantic_type().expect("typed"))
        .expect("the population type node");
    assert!(population_type.semantic_type().is_some());
    assert_ne!(p.semantic_type(), r.semantic_type());
    // A different bound or target gives a different population type node.
    let other_populations: BTreeSet<NodeKey> = graph
        .nodes()
        .filter(|node| node.semantic_form() == "parameter")
        .filter_map(|node| node.semantic_type())
        .collect();
    assert!(
        other_populations.len() >= 4,
        "[3], [5] and Invoice[3] differ"
    );

    let all = applications(graph, "quire.op.model.all_instances");
    assert_eq!(all.len(), 1);
    let lookups = applications(graph, "quire.op.model.lookup");
    assert_eq!(lookups.len(), 2, "one lookup node per absence mode");
    let modes: BTreeSet<String> = lookups
        .iter()
        .map(|node| preimage(node)["body"]["operation"]["mode"].to_string())
        .collect();
    assert_eq!(modes.len(), 2);
    assert!(
        modes
            .iter()
            .any(|mode| mode.contains("\"absence\"") && mode.contains("\"empty\"")),
        "a lookup with `absent empty` carries mode absence = empty: {modes:?}"
    );

    // FR-094-AC-4: the attribute and dispatch members name `Order`'s model
    // node.
    let deref = applications(graph, "quire.op.record.project");
    assert_eq!(deref.len(), 1);
    assert_eq!(member_declaration(deref[0]), json!(m1.to_string()));
    let dispatch_call = applications(graph, "quire.op.model.dispatch_call");
    assert_eq!(dispatch_call.len(), 1);
    assert_eq!(member_declaration(dispatch_call[0]), json!(m1.to_string()));
}

/// TC-417 step 6 (FR-094-AC-4): over `s: Reference<M::Sub>`, the attribute
/// and dispatch members name `Sub`'s model node, not `Order`'s, though
/// `Order` declares `total`.
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
            Expression::field(Expression::deref(name("s")), "total".to_owned()),
        ),
        FunctionDeclaration::clause(
            "sized",
            vec![("s".to_owned(), named("M::Sub"))],
            builtin(BuiltinType::Boolean),
            None,
            Expression::binary(
                BinaryOperator::GreaterOrEqual,
                Expression::dispatch(name("s"), "size".to_owned(), Vec::new()),
                Expression::integer(0_i64),
            ),
            DeclaredClauseKind::Precondition,
        ),
    ]);
    let checked = check_declarations(declarations);
    let graph = checked.semantic_graph();
    assert!(parameter(graph, "s", 0).semantic_type().is_some());
    let m5 = model_node_of(graph, "Sub");
    assert_ne!(m5, model_node_of(graph, "Order"));
    let m5 = json!(m5.to_string());
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

/// TC-796 step 4 (FR-303-AC-4), at the S3 lowering: when lowering keys a
/// model declaration `d2` to node `n` while the correspondence already
/// holds `(n, d1)`, keying refuses `runtime_invariant`/
/// `established-invariant-broken` in category internal failure, naming
/// `n`, `d1` and `d2`; `n` still resolves to `d1`. The first entry is
/// seeded straight into the lowering's correspondence: no admitted input
/// keys two declarations to one node.
#[trace("TC-796", "FR-303-AC-4")]
#[test]
fn keying_a_second_declaration_onto_a_recorded_node_refuses() {
    let acme = admitted("1.0.0");
    let models = std::slice::from_ref(&acme.model);
    let location = generated_location();
    let (d1, d2) = (key("Invoice"), key("Order"));
    let (n, _) = lower(models, Default::default(), |lowering| {
        lowering.model_node(&d2, &location)
    });
    let n = n.expect("`Order` keys");
    let ((keyed, resolved), _) = lower(models, Default::default(), |lowering| {
        assert_eq!(lowering.correspondence.record(n, d1.clone()), Ok(()));
        let keyed = lowering.model_node(&d2, &location);
        (keyed, lowering.correspondence.resolve(n).cloned())
    });
    let Err(CheckRefusal {
        cause: CheckCause::InternalFault(fault),
        ..
    }) = keyed
    else {
        panic!("expected an internal fault, got {keyed:?}");
    };
    assert_eq!(
        *fault,
        KeyFault::CorrespondenceConflict(Box::new(
            crate::check::CorrespondenceConflict::NodeRebound {
                node: n,
                recorded: d1.clone(),
                offered: d2,
            }
        ))
    );
    let cause = CheckCause::InternalFault(fault);
    assert_eq!(
        cause.code(),
        qsl_foundation::diagnostic::Code::RuntimeInvariant
    );
    assert_eq!(cause.cause(), Some("established-invariant-broken"));
    assert_eq!(
        qsl_foundation::diagnostic::category_of(&quire_semantic_value::catalog::CatalogCode::new(
            cause.code().as_str(),
            "established-invariant-broken",
        )),
        Some(quire_semantic_value::catalog::Category::InternalFailure)
    );
    assert_eq!(resolved, Some(d1));
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

/// FR-094: a check selects one version of each domain package identity;
/// two admitted versions of `acme/orders` refuse as a broken invariant
/// instead of resolving declarations against whichever admitted view comes
/// first.
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
/// body are clause functions owned by the operation member's model node, and
/// renaming the synthesized labels leaves every key unchanged.
#[trace("FR-094-AC-5", "FR-094-CON-2", "TC-418")]
#[test]
fn clause_functions_key_under_their_operation_members_model_owner() {
    let acme = admitted("1.0.0");
    let size = check_declarations(dispatch(&acme, "Order/size"));
    let graph = size.semantic_graph();
    let precondition = clause_function(graph, "Order/size", "precondition");
    let body = clause_function(graph, "Order/size", "body");
    assert_eq!(precondition.len(), 1);
    assert_eq!(body.len(), 1);
    assert_ne!(precondition[0].key(), body[0].key());
    let c1 = preimage(precondition[0]);
    assert_eq!(
        c1["owner"],
        json!({"kind": "model", "identity": "acme/orders", "node": "ix://acme/orders/Order/size"})
    );
    assert!(c1["declaration"].is_null());

    let count = check_declarations(dispatch(&acme, "Order/count"));
    assert_eq!(
        clause_function(count.semantic_graph(), "Order/count", "precondition").len(),
        1
    );

    let mut relabelled = dispatch(&acme, "Order/size");
    for (index, function) in relabelled.functions.iter_mut().enumerate() {
        function.name = format!("renamed_{index}");
    }
    let relabelled = check_declarations(relabelled);
    let keys = |graph: &SemanticGraph| graph.nodes().map(SemanticNode::key).collect::<Vec<_>>();
    assert_eq!(keys(relabelled.semantic_graph()), keys(graph));
}

/// TC-418 step 4 (FR-094-AC-5): `Sub.size`'s authored and effective
/// preconditions are one node each in both dispatch operations, under its own
/// owner; `Order.size`'s authored precondition stays one node.
#[trace("FR-094-AC-5", "TC-418")]
#[test]
fn a_redefining_candidates_clauses_key_once_under_its_own_owner() {
    let acme = admitted("1.0.0");
    for root in ["Order/size", "Sub/size"] {
        let checked = check_declarations(dispatch(&acme, root));
        let graph = checked.semantic_graph();
        let sub = clause_function(graph, "Sub/size", "precondition");
        assert_eq!(sub.len(), 2, "the authored and the effective precondition");
        assert_ne!(sub[0].key(), sub[1].key());
        for node in &sub {
            assert_eq!(
                preimage(node)["owner"]["node"],
                json!("ix://acme/orders/Sub/size")
            );
        }
        if root == "Order/size" {
            assert_eq!(
                clause_function(graph, "Order/size", "precondition").len(),
                1
            );
        }
    }
}

// ---------------------------------------------------------------------
// TC-419: quantity type nodes.
// ---------------------------------------------------------------------

mod quantities {
    use super::*;
    use crate::value::semantic_node::{NodeOwner, OwnerSelection, OwnerSubject};
    use crate::value::unit::{admit_unit_graph, DimensionPreimage, UnitPreimage};
    use quire_semantic_value::quantity::UnitTable;

    fn owner() -> NodeOwner {
        NodeOwner::Definition(OwnerSubject::new(
            "agent-ix",
            "example-model",
            "example-model",
        ))
    }

    fn dimension(name: &str) -> DimensionPreimage {
        DimensionPreimage::new(
            owner(),
            vec!["Example".to_owned(), name.to_owned()],
            Vec::new(),
        )
        .expect("a base dimension")
    }

    fn unit(name: &str, dimension: &str) -> UnitPreimage {
        UnitPreimage::new(
            owner(),
            vec!["Example".to_owned(), name.to_owned()],
            node_key(dimension),
            None,
            &Rational::from(1),
            &Rational::from(0),
        )
        .expect("a root unit")
    }

    // Four nominal nodes, by name; each is admitted under its structural key.
    const LENGTH: &str = "dimension-length";
    const TIME: &str = "dimension-time";
    const METRE: &str = "unit-metre";
    const SECOND: &str = "unit-second";

    fn node_key(name: &str) -> NodeKey {
        match name {
            LENGTH => dimension("Length").key(),
            TIME => dimension("Time").key(),
            METRE => unit("metre", LENGTH).key(),
            SECOND => unit("second", TIME).key(),
            other => panic!("unknown nominal node {other}"),
        }
        .expect("a nominal preimage keys")
    }

    /// The `dimension-length`, `dimension-time`, `unit-metre` and
    /// `unit-second` nodes, admitted under the keys their preimages give.
    fn units() -> UnitTable {
        UnitTable::declared(&unit_graph())
    }

    /// The admitted graph [`units`] reads.
    fn unit_graph() -> quire_semantic_value::unit::UnitGraph {
        admit_unit_graph(
            [
                (dimension("Length"), node_key(LENGTH)),
                (dimension("Time"), node_key(TIME)),
            ],
            [
                (unit("metre", LENGTH), node_key(METRE)),
                (unit("second", TIME), node_key(SECOND)),
            ],
            &OwnerSelection::new([owner()]),
        )
        .expect("the unit nodes admit")
    }

    fn quantity(hex: &str) -> ValueType {
        ValueType::Quantity(UnitId::declared(node_key(hex)))
    }

    /// TC-419 steps 1-2 (FR-094-AC-6): a declared unit's quantity is typed
    /// by the unit's nominal node, with no `compound_unit` node; the result
    /// type of `a * a`, formed while `check` types the body, is one
    /// compound unit node.
    ///
    /// `a / t`, `a / a` and `a * a / a` do not check as a function body:
    /// no guard proves a quantity divisor nonzero (`facts`'s
    /// `NodeKind::Quantity(Divide)` refuses `unproved-nonzero`), so their
    /// result types are keyed from the units `result_unit` forms for them,
    /// the same forming `check` does while typing.
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
            ..PackageDeclarations::new(fixture_source(), "fixture", qsl_foundation::IdentityLimits::default())
        };
        let plain = check_declarations(package(Expression::boolean(true)));
        let graph = plain.semantic_graph();
        assert_eq!(
            parameter(graph, "a", 0).semantic_type(),
            Some(node_key(METRE))
        );
        assert!(graph
            .nodes()
            .all(|node| node.semantic_form() != "compound_unit"));
        // The type node is the unit's nominal node, typed by its dimension's
        // nominal node, which the graph holds too.
        let metre = graph.node(node_key(METRE)).expect("the metre unit node");
        assert_eq!(
            (metre.node_tag(), metre.semantic_form()),
            (NodeTag::ScalarType, "unit")
        );
        assert!(matches!(metre.nominal(), Some(NominalNode::Unit(None))));
        assert_eq!(metre.semantic_type(), Some(node_key(LENGTH)));
        let length = graph
            .node(node_key(LENGTH))
            .expect("the Length dimension node");
        assert_eq!(length.semantic_form(), "dimension");
        assert!(matches!(
            length.nominal(),
            Some(NominalNode::Dimension(None))
        ));
        assert!(graph.node(node_key(SECOND)).is_some() && graph.node(node_key(TIME)).is_some());

        let square = Expression::binary(BinaryOperator::Multiply, name("a"), name("a"));
        let squared = check_declarations(package(Expression::binary(
            BinaryOperator::Equal,
            square.clone(),
            square,
        )));
        let graph = squared.semantic_graph();
        let product = applications(graph, "quire.op.quantity.mul");
        assert_eq!(product.len(), 1);
        let u1 = product[0].semantic_type().expect("a typed product");
        assert_eq!(graph.node(u1).unwrap().semantic_form(), "compound_unit");
        assert_ne!(u1, node_key(METRE));

        use quire_semantic_value::quantity::{result_unit, UnitOperation};
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
        for unit in [&per_second, &ratio, &back] {
            formed
                .insert(unit.clone())
                .expect("a compound unit has an id");
        }
        let (keys, graph) = lower(&[], formed, |lowering| {
            [&per_second, &ratio, &back].map(|unit| {
                lowering
                    .type_node(
                        &ValueType::Quantity(unit.id().expect("a compound unit has an id")),
                        &generated_location(),
                    )
                    .expect("a held compound unit keys")
            })
        });
        assert_eq!(keys.iter().collect::<BTreeSet<_>>().len(), 3);
        let u2 = preimage(graph.node(keys[0]).unwrap());
        assert_eq!(
            u2["body"]["members"][0]["members"][0]["value"]["target"]["digest"],
            json!(resolved(METRE))
        );
        assert_ne!(keys[2], node_key(METRE));
    }

    /// FR-093 `ConvertScalar` quantity row (FR-093-AC-3): converting a
    /// metre quantity to kilometres is `quire.op.quantity.convert`, with
    /// member `type_argument` of the kilometre unit's node and mode
    /// `rounding` = `exact`.
    #[trace("FR-093-AC-3", "TC-415")]
    #[test]
    fn a_quantity_conversion_is_exact() {
        let kilometre = UnitPreimage::new(
            owner(),
            vec!["Example".to_owned(), "kilometre".to_owned()],
            node_key(LENGTH),
            Some(node_key(METRE)),
            &Rational::from(1000),
            &Rational::from(0),
        )
        .expect("a derived unit");
        let km = kilometre.key().expect("the unit keys");
        let graph = admit_unit_graph(
            [
                (dimension("Length"), node_key(LENGTH)),
                (dimension("Time"), node_key(TIME)),
            ],
            [
                (unit("metre", LENGTH), node_key(METRE)),
                (unit("second", TIME), node_key(SECOND)),
                (kilometre, km.clone()),
            ],
            &OwnerSelection::new([owner()]),
        )
        .expect("metre, second and kilometre admit");
        let km_type = ValueType::Quantity(UnitId::declared(km.clone()));
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
                Expression::convert(named("Km"), name("a")),
            )],
            ..PackageDeclarations::new(fixture_source(), "fixture", qsl_foundation::IdentityLimits::default())
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

    /// FR-094-AC-7: a table built from the graph's units through the public
    /// `UnitTable::insert`, not `UnitTable::declared`, holds the declared
    /// `metre` unit but no admitted node to build it from. Keying a `metre`
    /// quantity is an internal fault naming the unit's key, with no key and
    /// no node.
    #[trace("FR-094-AC-7", "TC-419")]
    #[test]
    fn a_declared_unit_with_no_admitted_node_is_an_internal_fault() {
        let mut table = UnitTable::default();
        for unit in unit_graph().units() {
            table
                .insert(QuantityUnit::Declared(Box::new(unit.clone())))
                .expect("a declared unit's id is its key");
        }
        assert!(table.get(UnitId::declared(node_key(METRE))).is_some());
        let (result, graph) = lower(&[], table, |lowering| {
            lowering.type_node(&quantity(METRE), &generated_location())
        });
        assert!(matches!(
            result,
            Err(CheckRefusal {
                cause: CheckCause::InternalFault(fault),
                ..
            }) if *fault == KeyFault::UnheldNominal(node_key(METRE))
        ));
        assert_eq!(graph.nodes().count(), 0);
    }

    /// FR-359-AC-3: a record value type is a `model`/`record_value_type`
    /// node and an event type a `model`/`event_type` node, each keyed by its
    /// `ModelOwner` alone, so one declaration identity keys the two forms
    /// differently and equal declarations key equally.
    #[trace("FR-359-AC-3", "TC-998")]
    #[test]
    fn a_record_and_an_event_type_are_model_nodes_keyed_by_their_owner() {
        use crate::model::domain_package::{EventTypeRecord, RecordValueTypeRecord};
        let key = DeclarationKey {
            package: "acme/orders".to_owned(),
            node: "ix://acme/orders/Money".to_owned(),
        };
        let record = DomainPackageRecord::RecordValueType(RecordValueTypeRecord {
            key: key.clone(),
            identity_fields: Vec::new(),
            supertypes: Vec::new(),
        });
        let event = DomainPackageRecord::EventType(EventTypeRecord {
            key: key.clone(),
            identity_fields: Vec::new(),
            occurrence_field: None,
            supertypes: vec![DeclarationKey {
                package: "acme/orders".to_owned(),
                node: "ix://acme/orders/Base".to_owned(),
            }],
        });
        assert_eq!(
            record_form(&record),
            Some((NodeTag::Model, "record_value_type"))
        );
        assert_eq!(record_form(&event), Some((NodeTag::Model, "event_type")));
        let keyed = |record: &DomainPackageRecord, form| {
            model_declaration_key(
                &record.key().package,
                NodeTag::Model,
                form,
                &record.key().node,
            )
            .unwrap()
        };
        let record_key = keyed(&record, "record_value_type");
        let event_key = keyed(&event, "event_type");
        assert_ne!(record_key, event_key);
        assert_eq!(keyed(&event, "event_type"), event_key);
    }
}

/// QSpec FR-322-AC-25: lowering keys a state machine as a `model` node of
/// semantic form `state_machine`, so a state clause or operation anchor on it
/// has its node instead of an unnamed-record-kind fault.
#[trace("QSpec-FR-322-AC-25")]
#[test]
fn state_machine_declaration_keys_as_a_state_machine_model_node() {
    let package = DomainPackage::new(
        DomainPackageRef {
            identity: PACKAGE.to_owned(),
        },
        vec![
            DomainPackageRecord::StateMachine(StateMachineRecord {
                key: key("Lifecycle"),
                supertypes: Vec::new(),
                states: ["Draft", "Placed"]
                    .map(|name| StateMachineStateRecord {
                        key: key(&format!("Lifecycle/{name}")),
                        name: name.to_owned(),
                    })
                    .to_vec(),
                transitions: vec![StateMachineTransitionRecord {
                    key: key("Lifecycle/transition0"),
                    from: key("Lifecycle/Draft"),
                    to: key("Lifecycle/Placed"),
                    trigger: key("Lifecycle/place"),
                    guard: None,
                    emits: Vec::new(),
                    path: "/types/0/transitions/0".to_owned(),
                    origin: json!({}),
                }],
            }),
            query("Lifecycle/place", "Lifecycle", None),
        ],
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("the state machine normalizes");
    };
    let model = AdmittedModel::new(&package, &view).expect("the view is the package's own");
    let models = std::slice::from_ref(&model);
    let location = generated_location();
    let (node, graph) = lower(models, Default::default(), |lowering| {
        lowering.model_node(&key("Lifecycle"), &location)
    });
    let node = node.expect("a state machine has a model node");
    let node = graph.node(node).expect("the node is in the graph");
    assert_eq!(node.node_tag(), NodeTag::Model);
    assert_eq!(node.semantic_form(), "state_machine");
}

/// FR-056 "K2 canonical native String, UUID and Timestamp payloads": the
/// three natives a record or event field may carry are distinct scalar type
/// nodes, so a composite holding one keys instead of refusing the whole
/// check as `ill_typed`/`type-mismatch`.
#[test]
fn native_string_uuid_and_timestamp_key_as_distinct_scalar_nodes() {
    let location = generated_location();
    let ((string, uuid, timestamp), graph) = lower(&[], Default::default(), |lowering| {
        (
            lowering.type_node(&ValueType::String, &location),
            lowering.type_node(&ValueType::Uuid, &location),
            lowering.type_node(&ValueType::Timestamp, &location),
        )
    });
    let forms: Vec<_> = [string, uuid, timestamp]
        .into_iter()
        .map(|result| {
            let node = graph
                .node(result.expect("a native scalar keys"))
                .expect("the node is in the graph");
            (node.node_tag(), node.semantic_form())
        })
        .collect();
    assert_eq!(
        forms,
        [
            (NodeTag::ScalarType, "string"),
            (NodeTag::ScalarType, "uuid"),
            (NodeTag::ScalarType, "timestamp"),
        ]
    );
}

/// The node tag and form `name` keys to in a package holding `records`.
fn keyed_form(records: Vec<DomainPackageRecord>, name: &str) -> (NodeTag, String) {
    let package = DomainPackage::new(
        DomainPackageRef {
            identity: PACKAGE.to_owned(),
        },
        records,
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("the package normalizes");
    };
    let model = AdmittedModel::new(&package, &view).expect("the view is the package's own");
    let location = generated_location();
    let (node, graph) = lower(
        std::slice::from_ref(&model),
        Default::default(),
        |lowering| lowering.model_node(&key(name), &location),
    );
    let node = graph
        .node(node.expect("the declaration has a model node"))
        .expect("the node is in the graph");
    (node.node_tag(), node.semantic_form().to_owned())
}

/// QSpec FR-322-AC-25: lowering keys a persistence interface as a `model`
/// node, so an anchor on one has its node instead of an unnamed-record-kind
/// fault.
#[trace("QSpec-FR-322-AC-25")]
#[test]
fn persistence_interface_declaration_keys_as_a_model_node() {
    let form = keyed_form(
        vec![
            object("Order", &[]),
            DomainPackageRecord::PersistenceInterface(PersistenceInterfaceRecord {
                key: key("Store"),
                persists: vec![key("Order")],
            }),
            query("Store/save", "Store", None),
        ],
        "Store",
    );
    assert_eq!(form, (NodeTag::Model, "persistence_interface".to_owned()));
}

/// QSpec FR-340: lowering keys a record value type as a `model` node, so a
/// frame field entry on one has its owner node instead of an
/// unnamed-record-kind fault.
#[test]
fn record_value_type_declaration_keys_as_a_model_node() {
    let form = keyed_form(
        vec![DomainPackageRecord::RecordValueType(
            RecordValueTypeRecord {
                key: key("Money"),
                identity_fields: Vec::new(),
                supertypes: Vec::new(),
            },
        )],
        "Money",
    );
    assert_eq!(form, (NodeTag::Model, "record_value_type".to_owned()));
}
