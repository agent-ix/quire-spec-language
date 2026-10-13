// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-359 / TC-998 to TC-1002: a child record value at an ancestor-typed slot,
//! decided by the checker, and the model-owned nodes and declaration keys the
//! checked package carries for record value types and event types.
//!
//! Evaluation and replay of the same fixture are in `qsl-eval` and
//! `qsl-replay`; every assertion here compares causes, node tags and member
//! paths, never message text.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{CheckCause, CheckRefusal, CheckedGraph, NodeTag, PackageDeclarations};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use quire_exact::IllTypedCause;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::Value;

#[path = "../support/child_composite.rs"]
mod fixture;

/// `declarations` as a unit selecting the FR-359 package as model `alias`.
fn assemble_as(alias: &str, declarations: &str) -> PackageDeclarations {
    assemble_document(&fixture::document(), alias, declarations)
}

fn assemble_document(document: &Value, alias: &str, declarations: &str) -> PackageDeclarations {
    let bytes = serde_json::to_vec(document).expect("the fixture serializes");
    let packages = package_input([bytes.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one model package");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model {alias} = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         {declarations}\n",
        package = fixture::PACKAGE,
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "child-composite", "fixture", "fixture:1"),
        "child-composite.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the source unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the source unit");
    let selected = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("I1 admits and normalizes the fixture package");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        selected,
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
}

fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    assemble_as("M", declarations).check(CheckingLimits::default())
}

fn mismatch() -> CheckCause {
    CheckCause::IllTyped(IllTypedCause::TypeMismatch)
}

/// The one refusal of `declarations`, which must refuse as a type mismatch.
fn refused_as_mismatch(declarations: &str) {
    let refusals = check(declarations).expect_err("the unit does not check");
    assert!(!refusals.is_empty());
    assert!(
        refusals.iter().all(|refusal| refusal.cause == mismatch()),
        "{refusals:?}"
    );
}

/// FR-359-AC-1, FR-359-AC-8: a child, and a grandchild, is admitted at a
/// parent-typed function argument, and read there by field name.
#[trace("FR-359-AC-1", "FR-359-AC-8", "TC-998", "TC-1001")]
#[test]
fn a_child_is_accepted_at_a_parent_typed_function_argument() {
    check(
        "function occurredAt using v(e: M::OrderPlaced): Integer pure { e.occurred_at }\n\
         function orderId using v(e: M::OrderPlaced): Integer pure { e.order_id }\n\
         function fromExpress using v(e: M::ExpressOrderPlaced): Integer pure { occurredAt(e) + orderId(e) }\n\
         function fromRush using v(e: M::RushOrderPlaced): Integer pure { occurredAt(e) }\n\
         function keep using v(m: M::Money): M::Money pure { m }\n\
         function passTaxed using v(m: M::TaxedMoney): M::Money pure { keep(m) }\n\
         function passGift using v(m: M::GiftTaxedMoney): M::Money pure { keep(m) }\n\
         function passGiftAsTaxed using v(m: M::GiftTaxedMoney): M::TaxedMoney pure { m }",
    )
    .expect("a child and a grandchild check at a parent-typed argument and result");
}

/// FR-359-AC-1: a parent-typed read of a field only the child declares keeps
/// the checker's `ill_typed`/`type-mismatch`.
#[trace("FR-359-AC-1", "TC-998")]
#[test]
fn a_parent_typed_read_of_a_child_field_is_refused() {
    refused_as_mismatch(
        "function promised using v(e: M::OrderPlaced): Integer pure { e.promised_by }",
    );
}

/// FR-359-AC-8: a parent is refused at a child-typed parameter, an unrelated
/// record at a parent-typed one, and an intermediate ancestor at a
/// descendant's.
#[trace("FR-359-AC-8", "TC-1001")]
#[test]
fn a_parent_or_an_unrelated_record_is_refused_at_a_child_typed_argument() {
    for refused in [
        "function needs using v(m: M::TaxedMoney): Integer pure { 0 }\n\
         function bad using v(m: M::Money): Integer pure { needs(m) }",
        "function needs using v(m: M::Money): Integer pure { 0 }\n\
         function bad using v(f: M::Fee): Integer pure { needs(f) }",
        "function needs using v(m: M::GiftTaxedMoney): Integer pure { 0 }\n\
         function bad using v(m: M::TaxedMoney): Integer pure { needs(m) }",
        "function bad using v(m: M::Money): M::TaxedMoney pure { m }",
    ] {
        refused_as_mismatch(refused);
    }
}

/// FR-359-AC-7: a type is not covariant, so a collection or an optional of
/// the child is refused at a parameter naming the parent.
#[trace("FR-359-AC-7", "TC-1000")]
#[test]
fn a_collection_or_option_of_the_child_is_refused_at_a_parent_slot() {
    refused_as_mismatch(
        "function takes using v(s: Sequence<M::Money>[0, 4]): Integer pure { 0 }\n\
         function bad using v(s: Sequence<M::TaxedMoney>[0, 4]): Integer pure { takes(s) }",
    );
    refused_as_mismatch(
        "function takes using v(o: Option<M::Money>): Integer pure { 0 }\n\
         function bad using v(o: Option<M::TaxedMoney>): Integer pure { takes(o) }",
    );
}

/// FR-359-AC-4, FR-359-AC-5: equality is over one static type; a child and
/// its parent are different static types and stay `ill_typed`.
#[trace("FR-359-AC-4", "TC-999")]
#[test]
fn equality_over_a_child_and_a_parent_static_type_stays_ill_typed() {
    check(
        "function same using v(a: M::Money, b: M::Money): Boolean pure { a = b }\n\
         function differ using v(a: M::Money, b: M::Money): Boolean pure { a != b }",
    )
    .expect("equality over parent-typed operands checks");
    refused_as_mismatch(
        "function mixed using v(c: M::TaxedMoney, b: M::Money): Boolean pure { c = b }",
    );
    refused_as_mismatch(
        "function mixed using v(b: M::Money, c: M::TaxedMoney): Boolean pure { b = c }",
    );
}

/// FR-359-AC-6: a child is admitted at an optional record field, with the
/// field read back as an `Option` of the parent's type; a parent at a
/// child-typed optional field and an unrelated record at a parent-typed one
/// are refused.
#[trace("FR-359-AC-6", "TC-1000")]
#[test]
fn an_optional_field_admits_a_child_and_refuses_a_parent_or_a_stranger() {
    check(
        "function build using v(m: M::TaxedMoney): M::Invoice pure { M::Invoice { total: m } }\n\
         function total using v(i: M::Invoice): Option<M::Money> pure { i.total }\n\
         function has using v(i: M::Invoice): Boolean pure { present(i.total) }\n\
         function unwrapTotal using v(i: M::Invoice, d: M::Money): M::Money pure { if present(i.total) then value(i.total) else d }",
    )
    .expect("a child checks at an optional field and reads back");
    refused_as_mismatch(
        "function bad using v(m: M::Money): M::TaxedInvoice pure { M::TaxedInvoice { total: m } }",
    );
    refused_as_mismatch(
        "function bad using v(f: M::Fee): M::Invoice pure { M::Invoice { total: f } }",
    );
}

/// FR-359-AC-7: a collection literal of the parent admits a child element.
#[trace("FR-359-AC-7", "TC-1000")]
#[test]
fn a_collection_literal_of_the_parent_admits_a_child_element() {
    check(
        "function sequenceOf using v(m: M::TaxedMoney, n: M::Money): Sequence<M::Money>[0, 4] pure { sequence[m, n] }\n\
         function setOf using v(m: M::TaxedMoney, n: M::Money): Set<M::Money>[0, 4] pure { set[m, n] }\n\
         function bagOf using v(m: M::TaxedMoney, n: M::Money): Bag<M::Money>[0, 4] pure { bag[m, n] }\n\
         function orderedOf using v(m: M::TaxedMoney, n: M::Money): OrderedSet<M::Money>[0, 4] pure { orderedSet[m, n] }\n\
         function found using v(s: Sequence<M::Money>[0, 4], m: M::TaxedMoney): Boolean pure { contains(s, m) }\n\
         function lines using v(m: M::TaxedMoney, n: M::Money): M::Lines pure { M::Lines { lines: sequence[m, n], tags: set[m, n], counted: bag[n, m], queue: orderedSet[m, n] } }",
    )
    .expect("a child checks as a collection element");
}

/// FR-359-AC-5: FR-208 gives an event type no equality, so `==` over event
/// operands, and over a record holding one, is `ill_typed`/`operator-ineligible`
/// whether or not the two have one type.
#[trace("FR-359-AC-5", "TC-999")]
#[test]
fn equality_over_event_operands_stays_refused() {
    let refusals = assemble_document(
        &fixture::document_with_event(),
        "M",
        "function same using v(a: M::Shipped, b: M::Shipped): Boolean pure { a = b }",
    )
    .check(CheckingLimits::default())
    .expect_err("an event has no equality");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(
        refusals[0].cause,
        CheckCause::IllTyped(IllTypedCause::OperatorIneligible)
    );
}

fn model_nodes(graph: &CheckedGraph) -> Vec<(NodeTag, &'static str, Value)> {
    graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.node_tag() == NodeTag::Model)
        .map(|node| {
            let preimage: Value =
                serde_json::from_slice(node.preimage()).expect("a node preimage is JSON");
            (node.node_tag(), node.semantic_form(), preimage)
        })
        .collect()
}

/// FR-359-AC-3: every record value type and event type is a model-owned
/// `record_value_type` or `event_type` node, none is a `composite_type`, and
/// no node carries a `supertypes` member or a conversion.
#[trace("FR-359-AC-3", "TC-998")]
#[test]
fn records_and_events_are_model_owned_nodes_without_supertypes() {
    let graph = check(
        "function occurredAt using v(e: M::OrderPlaced): Integer pure { e.occurred_at }\n\
         function fromExpress using v(e: M::ExpressOrderPlaced): Integer pure { occurredAt(e) }\n\
         function fromRush using v(e: M::RushOrderPlaced): Integer pure { occurredAt(e) }\n\
         function nothing using v(): Boolean pure { true }",
    )
    .expect("the unit checks");
    let nodes = model_nodes(&graph);
    for (name, form) in [
        ("OrderPlaced", "record_value_type"),
        ("ExpressOrderPlaced", "record_value_type"),
        ("RushOrderPlaced", "record_value_type"),
        ("Money", "record_value_type"),
    ] {
        let owned: Vec<_> = nodes
            .iter()
            .filter(|(_, node_form, preimage)| {
                *node_form == form && preimage["owner"]["node"] == fixture::id(name)
            })
            .collect();
        assert_eq!(owned.len(), 1, "{name} is one {form} node");
        assert_eq!(
            owned[0].2["owner"]["kind"], "model",
            "{name} has a ModelOwner"
        );
        assert_eq!(owned[0].2["owner"]["identity"], fixture::PACKAGE);
    }
    assert!(
        graph
            .semantic_graph()
            .nodes()
            .all(|node| node.node_tag() != NodeTag::CompositeType
                || !String::from_utf8_lossy(node.preimage()).contains("OrderPlaced")),
        "no record of the package is a composite_type"
    );
    for node in graph.semantic_graph().nodes() {
        let preimage: Value = serde_json::from_slice(node.preimage()).unwrap();
        assert!(
            !contains_member(&preimage, "supertypes"),
            "no node carries a supertypes member"
        );
        assert!(
            !contains_member(&preimage, "convert"),
            "no node is a conversion"
        );
    }
}

fn contains_member(value: &Value, member: &str) -> bool {
    match value {
        Value::Object(map) => {
            map.contains_key(member) || map.values().any(|value| contains_member(value, member))
        }
        Value::Array(items) => items.iter().any(|value| contains_member(value, member)),
        _ => false,
    }
}

/// FR-359-AC-3: the call to `f` takes its argument by `ref`, and the field
/// read is one `quire.op.record.project` naming `OrderPlaced`'s node and
/// `occurred_at`.
#[trace("FR-359-AC-3", "TC-998")]
#[test]
fn a_field_read_names_the_parent_node_and_the_call_adds_no_node() {
    let graph = check(
        "function occurredAt using v(e: M::OrderPlaced): Integer pure { e.occurred_at }\n\
         function fromExpress using v(e: M::ExpressOrderPlaced): Integer pure { occurredAt(e) }",
    )
    .expect("the unit checks");
    let placed = graph
        .semantic_graph()
        .nodes()
        .find(|node| {
            node.node_tag() == NodeTag::Model && node.semantic_form() == "record_value_type" && {
                let preimage: Value = serde_json::from_slice(node.preimage()).unwrap();
                preimage["owner"]["node"] == fixture::id("OrderPlaced")
            }
        })
        .expect("OrderPlaced is a model node")
        .key()
        .to_string();
    let all: Vec<Value> = graph
        .semantic_graph()
        .nodes()
        .map(|node| serde_json::from_slice(node.preimage()).unwrap())
        .collect();
    let projects: Vec<&Value> = all
        .iter()
        .filter(|preimage| preimage["body"]["operation"]["identity"] == "quire.op.record.project")
        .collect();
    assert_eq!(projects.len(), 1, "one record.project application");
    let member = &projects[0]["body"]["operation"]["member"];
    assert_eq!(member["kind"], "field");
    assert_eq!(member["name"], "occurred_at");
    assert_eq!(member["declaration"]["digest"], placed);
    assert!(
        all.iter()
            .all(|preimage| !contains_text(preimage, "convert") && !contains_text(preimage, "cast")),
        "no conversion node between the call and its argument"
    );
}

fn contains_text(value: &Value, needle: &str) -> bool {
    match value {
        Value::String(text) => text.contains(needle),
        Value::Object(map) => map.values().any(|value| contains_text(value, needle)),
        Value::Array(items) => items.iter().any(|value| contains_text(value, needle)),
        _ => false,
    }
}

/// FR-359-AC-11: the declaration key a model record has in the type
/// environment is its model node's key; two units that select the model under
/// different aliases give every record the same key.
#[trace("FR-359-AC-11", "TC-1000")]
#[test]
fn a_model_record_key_is_its_model_node_key_whatever_the_alias() {
    let keys = |alias: &str| {
        let declarations = assemble_as(alias, "function noop using v(): Boolean pure { true }");
        let types = declarations.types.clone();
        let graph = declarations
            .check(CheckingLimits::default())
            .expect("an empty unit checks");
        let nodes: std::collections::BTreeMap<_, _> = graph
            .semantic_graph()
            .nodes()
            .map(|node| (node.key(), node))
            .collect();
        let mut keys = std::collections::BTreeMap::new();
        for composite in types.composites() {
            let (_, name) = composite
                .name()
                .split_once("::")
                .expect("a model record is named alias::artifact");
            let node = nodes
                .get(&composite.key())
                .unwrap_or_else(|| panic!("{name} declares its model node's key"));
            assert_eq!(node.node_tag(), NodeTag::Model);
            let preimage: Value = serde_json::from_slice(node.preimage()).unwrap();
            assert_eq!(preimage["owner"]["node"], fixture::id(name));
            keys.insert(name.to_owned(), composite.key());
        }
        keys
    };
    let (m, n) = (keys("M"), keys("N"));
    assert!(m.len() >= 10, "the package declares its records: {m:?}");
    assert_eq!(m, n, "the alias does not enter a declaration key");
}
