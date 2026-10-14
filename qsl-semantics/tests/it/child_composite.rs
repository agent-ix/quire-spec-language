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
use quire_semantic_value::location::Origin;
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

/// The one refusal of `declarations`, which must be a type mismatch located
/// in the body of `function` at child-index `path` from its root expression:
/// the argument, record field or operand it names, not just any site.
fn refused_at(declarations: &str, function: &str, path: &[usize]) {
    let refusals = check(declarations).expect_err("the unit does not check");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].cause, mismatch(), "{refusals:?}");
    assert!(
        matches!(&refusals[0].location.origin,
            Origin::Body { function: found, .. } if found == function),
        "refused in the body of {function}: {:?}",
        refusals[0].location
    );
    assert_eq!(refusals[0].location.path(), path, "{declarations}");
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
    refused_at(
        "function promised using v(e: M::OrderPlaced): Integer pure { e.promised_by }",
        "promised",
        &[],
    );
}

/// FR-359-AC-8: a parent is refused at a child-typed parameter, an unrelated
/// record at a parent-typed one, and an intermediate ancestor at a
/// descendant's.
#[trace("FR-359-AC-8", "TC-1001")]
#[test]
fn a_parent_or_an_unrelated_record_is_refused_at_a_child_typed_argument() {
    for (declarations, path) in [
        (
            "function needs using v(m: M::TaxedMoney): Integer pure { 0 }\n\
             function bad using v(m: M::Money): Integer pure { needs(m) }",
            &[0][..],
        ),
        (
            "function needs using v(m: M::Money): Integer pure { 0 }\n\
             function bad using v(f: M::Fee): Integer pure { needs(f) }",
            &[0],
        ),
        (
            "function needs using v(m: M::GiftTaxedMoney): Integer pure { 0 }\n\
             function bad using v(m: M::TaxedMoney): Integer pure { needs(m) }",
            &[0],
        ),
        // The refusal names the second parameter, not the first.
        (
            "function needs using v(a: M::Money, b: M::Money): Integer pure { 0 }\n\
             function bad using v(m: M::TaxedMoney, f: M::Fee): Integer pure { needs(m, f) }",
            &[1],
        ),
    ] {
        refused_at(declarations, "bad", path);
    }
    // A result is refused at the body that produces it.
    refused_at(
        "function bad using v(m: M::Money): M::TaxedMoney pure { m }",
        "bad",
        &[],
    );
}

/// FR-359-AC-7: a type is not covariant, so a collection or an optional of
/// the child is refused at a parameter naming the parent.
#[trace("FR-359-AC-7", "TC-1000")]
#[test]
fn a_collection_or_option_of_the_child_is_refused_at_a_parent_slot() {
    refused_at(
        "function takes using v(s: Sequence<M::Money>[0, 4]): Integer pure { 0 }\n\
         function bad using v(s: Sequence<M::TaxedMoney>[0, 4]): Integer pure { takes(s) }",
        "bad",
        &[0],
    );
    refused_at(
        "function takes using v(o: Option<M::Money>): Integer pure { 0 }\n\
         function bad using v(o: Option<M::TaxedMoney>): Integer pure { takes(o) }",
        "bad",
        &[0],
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
    // The equality expression is the body's root, whose children are the two
    // operands: the refusal is at the comparison, in either operand order.
    refused_at(
        "function mixed using v(c: M::TaxedMoney, b: M::Money): Boolean pure { c = b }",
        "mixed",
        &[],
    );
    refused_at(
        "function mixed using v(b: M::Money, c: M::TaxedMoney): Boolean pure { b = c }",
        "mixed",
        &[],
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
    refused_at(
        "function bad using v(m: M::Money): M::TaxedInvoice pure { M::TaxedInvoice { total: m } }",
        "bad",
        &[0],
    );
    refused_at(
        "function bad using v(f: M::Fee): M::Invoice pure { M::Invoice { total: f } }",
        "bad",
        &[0],
    );
    // The refusal names the field: `discount`, the record's second member.
    refused_at(
        "function bad using v(m: M::TaxedMoney, f: M::Fee): M::Invoice pure { M::Invoice { total: m, discount: f } }",
        "bad",
        &[1],
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

/// The refusals of `declarations` over the event fixture package.
fn refused_with_events(declarations: &str) -> Vec<CheckRefusal> {
    assemble_document(&fixture::document_with_event(), "M", declarations)
        .check(CheckingLimits::default())
        .expect_err("an event has no equality")
}

/// FR-359-AC-5: FR-208 gives an event type no equality, so `=` and `!=` over
/// event operands, and over an optional or a sequence of events, are `ill_typed`/`operator-ineligible` whether or not the
/// two operands have one type. Each refusal is located in the comparing
/// function's own body.
#[trace("FR-359-AC-5", "TC-999")]
#[test]
fn equality_over_event_operands_stays_refused() {
    for (function, declaration) in [
        (
            "same",
            "function same using v(a: M::Shipped, b: M::Shipped): Boolean pure { a = b }",
        ),
        (
            "differ",
            "function differ using v(a: M::Shipped, b: M::Shipped): Boolean pure { a != b }",
        ),
        (
            "maybe",
            "function maybe using v(a: Option<M::Shipped>, b: Option<M::Shipped>): Boolean pure { a = b }",
        ),
        (
            "logs",
            "function logs using v(a: Sequence<M::Shipped>[0, 2], b: Sequence<M::Shipped>[0, 2]): Boolean pure { a != b }",
        ),
    ] {
        let refusals = refused_with_events(declaration);
        assert_eq!(refusals.len(), 1, "{function}: {refusals:?}");
        assert_eq!(
            refusals[0].cause,
            CheckCause::IllTyped(IllTypedCause::OperatorIneligible),
            "{function}"
        );
        assert!(
            matches!(&refusals[0].location.origin,
                Origin::Body { function: found, .. } if found == function),
            "{function}: {:?}",
            refusals[0].location
        );
    }
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

/// FR-359-AC-3 (records): every record value type is a model-owned
/// `record_value_type` node, none is a `composite_type`, and
/// no node carries a `supertypes` member or a conversion.
#[trace("FR-359-AC-3", "TC-998")]
#[test]
fn records_are_model_owned_nodes_without_supertypes() {
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

/// FR-359-AC-3, event half: a package whose unit takes an event-typed
/// parameter holds the event as one `model`/`event_type` node owned by its
/// declaring package and node, beside the record nodes, with no
/// `composite_type` and no `supertypes` member. The unit needs no read of the
/// event's Timestamp field, whose lowering waits on PR #789.
#[trace("FR-359-AC-3", "TC-998")]
#[test]
fn an_event_type_is_one_model_owned_event_type_node_in_a_package() {
    let graph = assemble_document(
        &fixture::document_with_event(),
        "M",
        "function shipped using v(e: M::Shipped): Boolean pure { true }\n\
         function placed using v(e: M::OrderPlaced): Boolean pure { true }",
    )
    .check(CheckingLimits::default())
    .expect("the event unit checks");
    let nodes = model_nodes(&graph);
    let events: Vec<_> = nodes
        .iter()
        .filter(|(_, form, _)| *form == "event_type")
        .collect();
    assert_eq!(events.len(), 1, "one event_type node: {nodes:?}");
    let owner = &events[0].2["owner"];
    assert_eq!(owner["kind"], "model");
    assert_eq!(owner["identity"], fixture::PACKAGE);
    assert_eq!(owner["node"], fixture::id("Shipped"));
    assert!(
        nodes
            .iter()
            .any(|(_, form, preimage)| *form == "record_value_type"
                && preimage["owner"]["node"] == fixture::id("OrderPlaced")),
        "the record sits beside the event"
    );
    for node in graph.semantic_graph().nodes() {
        let preimage: Value = serde_json::from_slice(node.preimage()).unwrap();
        assert!(!contains_member(&preimage, "supertypes"));
        assert_ne!(node.node_tag(), NodeTag::CompositeType);
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

/// FR-359-AC-1: a field typed UUID or Timestamp is read, and compared, at a
/// parent-typed slot like any other. Lowering a UUID, Timestamp or String leaf
/// now lowers (PR #789); the `=` the unit uses on them is still refused.
#[trace("FR-359-AC-1", "TC-998")]
#[test]
#[ignore = "String, UUID and Timestamp now lower, but `=` on them is refused ill_typed/operator-ineligible; equality on native scalars is not yet admitted"]
fn uuid_timestamp_and_string_fields_are_read_through_a_child() {
    assemble_document(
        &fixture::document_with_native_leaves(),
        "M",
        "function token using v(c: M::Credential): Boolean pure { c.token = c.token }\n\
         function issued using v(c: M::Credential): Boolean pure { c.issued = c.issued }\n\
         function label using v(t: M::Ticket): Boolean pure { t.label = t.label }\n\
         function viaChild using v(t: M::StampedTicket): Boolean pure { label(t) }",
    )
    .check(CheckingLimits::default())
    .expect("native leaves check");
}

// ---------------------------------------------------------------------------
// FR-359-AC-9 / FR-106: the `record` value form in a snapshot
// ---------------------------------------------------------------------------

use std::collections::BTreeMap;

use qsl_semantics::model::observation::{
    admit_current_snapshot, AdmissionFailure, AdmissionRecord, AdmissionUsage, DocumentRef,
    ObservationLimits, PathStep,
};
use serde_json::json;

/// `{"record": {"type": <name>, "fields": {<field>: {"integer": ..}}}}`.
fn record_value(name: &str, fields: &[(&str, &str)]) -> Value {
    let fields: serde_json::Map<String, Value> = fields
        .iter()
        .map(|(field, integer)| ((*field).to_owned(), json!({"integer": integer})))
        .collect();
    json!({"record": {"type": fixture::id(name), "fields": fields}})
}

fn money(amount: &str) -> Value {
    record_value("Money", &[("amount_minor", amount), ("currency", "978")])
}

fn taxed(amount: &str, tax: &str) -> Value {
    record_value(
        "TaxedMoney",
        &[
            ("amount_minor", amount),
            ("currency", "978"),
            ("tax_minor", tax),
        ],
    )
}

fn gift(amount: &str) -> Value {
    record_value(
        "GiftTaxedMoney",
        &[
            ("amount_minor", amount),
            ("currency", "978"),
            ("tax_minor", "20"),
            ("wrap_minor", "3"),
        ],
    )
}

/// The one `Order` object's fields, with `total` and `items` replaced by
/// the given values; `discount` and `taxed` are absent unless supplied.
fn order_fields(total: Value, items: Vec<Value>) -> serde_json::Map<String, Value> {
    let mut fields = serde_json::Map::new();
    fields.insert("order_id".to_owned(), json!({"integer": "1"}));
    fields.insert("total".to_owned(), total);
    fields.insert("discount".to_owned(), json!({"absent": {}}));
    fields.insert("taxed".to_owned(), json!({"absent": {}}));
    fields.insert("chain".to_owned(), json!({"absent": {}}));
    fields.insert("bundle".to_owned(), json!({"absent": {}}));
    fields.insert("items".to_owned(), json!({"sequence": items}));
    fields
}

/// Admit a current snapshot holding one `Order` named `o1` with `fields`.
fn admit_order(fields: serde_json::Map<String, Value>) -> Result<(), AdmissionFailure> {
    admit_order_within(fields, ObservationLimits::default())
}

/// [`admit_order`] under the given observation limits.
fn admit_order_within(
    fields: serde_json::Map<String, Value>,
    limits: ObservationLimits,
) -> Result<(), AdmissionFailure> {
    admit_order_text(fields, limits, |text| text)
}

/// [`admit_order_within`] over the snapshot text `edit` rewrites, which may
/// nest deeper than `serde_json::Value` can hold.
fn admit_order_text(
    fields: serde_json::Map<String, Value>,
    limits: ObservationLimits,
    edit: impl FnOnce(String) -> String,
) -> Result<(), AdmissionFailure> {
    admit_order_usage(fields, limits, edit).map(|_| ())
}

/// [`admit_order_text`], returning what the admitted document used.
fn admit_order_usage(
    fields: serde_json::Map<String, Value>,
    limits: ObservationLimits,
    edit: impl FnOnce(String) -> String,
) -> Result<AdmissionUsage, AdmissionFailure> {
    let document = serde_json::to_vec(&fixture::document()).expect("the fixture serializes");
    let packages = package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one model package");
    };
    let model_digest = qsl_semantics::model::key::hex(digest);
    let graph = check("function noop using v(): Boolean pure { true }").expect("the unit checks");
    let label = DocumentRef {
        authority: "test".to_owned(),
        identity: "orders".to_owned(),
        revision_namespace: "fixture".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    let text = serde_json::to_string(&json!({
        "format": "quire.state.snapshot/v1",
        "identity": {
            "authority": label.authority, "identity": label.identity,
            "revision_namespace": label.revision_namespace, "revision": label.revision,
        },
        "observation": "current",
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {
            "identity": fixture::PACKAGE, "version": "1",
            "digest": format!("sha256-jcs:{model_digest}"),
        },
        "populations": [{
            "population": fixture::id("orders"),
            "complete": true,
            "objects": [{"key": "o1", "type": fixture::id("Order"), "fields": fields}],
        }],
    }))
    .expect("the snapshot serializes");
    let bytes = edit(text).into_bytes();
    let parsed = quire_canonical::read(&bytes, u64::MAX).expect("the snapshot is JSON");
    let digest = *quire_canonical::sha256(&parsed, quire_canonical::Limits::new(u64::MAX))
        .expect("the snapshot is canonical")
        .as_bytes();
    let selected = DocumentRef { digest, ..label };
    let snapshots = BTreeMap::from([(digest, bytes)]);
    admit_current_snapshot(
        graph.model_selections(),
        graph.scope().types(),
        &packages,
        ModelNormalizationLimits::default(),
        &snapshots,
        &selected,
        limits,
    )
    .map(|(_, usage)| usage)
}

/// The refusal of `result`: its cause and its object, field and nested-path
/// components.
fn refusal_of(result: Result<(), AdmissionFailure>) -> Refusal {
    let Err(AdmissionFailure::Refused(AdmissionRecord {
        code,
        cause,
        fields,
        path,
    })) = result
    else {
        panic!("a refusal, not {result:?}");
    };
    assert_eq!(code.as_str(), "invalid_runtime_input");
    let component = |name: &str| fields.get(name).cloned().unwrap_or_default();
    (
        cause.to_owned(),
        component("object"),
        component("field"),
        path,
    )
}

/// A refusal's cause, object, field and typed path.
type Refusal = (String, String, String, Vec<PathStep>);

fn refusal(cause: &str, field: &str, path: Vec<PathStep>) -> Refusal {
    (cause.to_owned(), "o1".to_owned(), field.to_owned(), path)
}

fn field_step(name: &str) -> PathStep {
    PathStep::Field(name.to_owned())
}

/// FR-359-AC-9: a snapshot whose `Order.total: Money` holds a `TaxedMoney`
/// admits, and so does a `GiftTaxedMoney` there, a `Money`, the same record as
/// the payload of the optional `discount: Money?` and as an element of the
/// `items: Sequence<Money>` field; a `GiftTaxedMoney` admits at the
/// `taxed: TaxedMoney?` slot.
#[trace("FR-359-AC-9", "TC-1002")]
#[test]
fn a_snapshot_admits_a_child_record_at_an_ancestor_typed_field() {
    admit_order(order_fields(money("100"), vec![])).expect("a Money at Money");
    admit_order(order_fields(taxed("100", "20"), vec![])).expect("a TaxedMoney at Money");
    admit_order(order_fields(gift("100"), vec![])).expect("a GiftTaxedMoney at Money");
    let mut fields = order_fields(money("1"), vec![]);
    fields.insert("discount".to_owned(), json!({"present": taxed("5", "1")}));
    admit_order(fields).expect("a TaxedMoney as the payload of Money?");
    admit_order(order_fields(
        money("1"),
        vec![taxed("2", "1"), money("3"), gift("4")],
    ))
    .expect("children as the elements of Sequence<Money>");
    let mut fields = order_fields(money("1"), vec![]);
    fields.insert("taxed".to_owned(), json!({"present": gift("9")}));
    admit_order(fields).expect("a GiftTaxedMoney at a TaxedMoney? field");
}

/// FR-359-AC-9: a record that is only an ancestor of the slot's type, an
/// unrelated record, a `type` naming an object type or no record, and an
/// object reference in place of a record refuse `wrong-value-kind` at the
/// slot.
#[trace("FR-359-AC-9", "FR-106-AC-15", "TC-1002")]
#[test]
fn a_snapshot_refuses_a_record_that_does_not_conform_to_its_slot() {
    let mut ancestor_at_child = order_fields(money("1"), vec![]);
    ancestor_at_child.insert("taxed".to_owned(), json!({"present": money("1")}));
    assert_eq!(
        refusal_of(admit_order(ancestor_at_child)),
        refusal("wrong-value-kind", "taxed", vec![PathStep::OptionValue])
    );
    assert_eq!(
        refusal_of(admit_order(order_fields(
            record_value("Fee", &[("fee_minor", "1")]),
            vec![]
        ))),
        refusal("wrong-value-kind", "total", vec![])
    );
    for ty in [fixture::id("Order"), fixture::id("NoSuchRecord")] {
        let value = json!({"record": {"type": ty, "fields": {}}});
        assert_eq!(
            refusal_of(admit_order(order_fields(value, vec![]))),
            refusal("wrong-value-kind", "total", vec![]),
            "{ty}"
        );
    }
    let reference = json!({"reference": {"population": fixture::id("orders"), "key": "o1"}});
    assert_eq!(
        refusal_of(admit_order(order_fields(reference, vec![]))),
        refusal("wrong-value-kind", "total", vec![])
    );
}

/// FR-359-AC-9, FR-106-AC-15: a `TaxedMoney` missing `tax_minor` refuses
/// `missing-member` naming the nested field, though a `Money` without it
/// admits; an extra field refuses `unknown-member`; a malformed scalar
/// refuses `invalid-value`. The path names the element for a sequence.
#[trace("FR-359-AC-9", "FR-106-AC-15", "TC-1002")]
#[test]
fn a_snapshot_record_is_checked_for_its_own_effective_fields() {
    let without_tax = record_value(
        "TaxedMoney",
        &[("amount_minor", "100"), ("currency", "978")],
    );
    assert_eq!(
        refusal_of(admit_order(order_fields(without_tax.clone(), vec![]))),
        refusal("missing-member", "total", vec![field_step("tax_minor")])
    );
    assert_eq!(
        refusal_of(admit_order(order_fields(
            money("1"),
            vec![money("1"), without_tax]
        ))),
        refusal(
            "missing-member",
            "items",
            vec![PathStep::Index(1), field_step("tax_minor")]
        )
    );
    let extra = record_value(
        "Money",
        &[
            ("amount_minor", "1"),
            ("currency", "978"),
            ("tax_minor", "2"),
        ],
    );
    assert_eq!(
        refusal_of(admit_order(order_fields(extra, vec![]))),
        refusal("unknown-member", "total", vec![field_step("tax_minor")])
    );
    let bad_integer = record_value(
        "Money",
        &[("amount_minor", "not-a-number"), ("currency", "978")],
    );
    assert_eq!(
        refusal_of(admit_order(order_fields(bad_integer, vec![]))),
        refusal("invalid-value", "total", vec![field_step("amount_minor")])
    );
}

/// FR-106-AC-15: the refusal names the first path reached in field order: of
/// two defective fields of one record the earlier declared one, and of two
/// defective fields of the object the earlier declared one.
#[trace("FR-359-AC-9", "FR-106-AC-15", "TC-1002")]
#[test]
fn a_snapshot_refusal_names_the_first_path_in_field_order() {
    let two_bad = json!({"record": {"type": fixture::id("TaxedMoney"), "fields": {
        "tax_minor": {"boolean": true},
        "currency": {"integer": "978"},
        "amount_minor": {"integer": "x"},
    }}});
    assert_eq!(
        refusal_of(admit_order(order_fields(two_bad.clone(), vec![]))),
        refusal("invalid-value", "total", vec![field_step("amount_minor")])
    );
    // `total` is declared before `items`, so its defect is the first reached
    // even though `items` also holds one.
    assert_eq!(
        refusal_of(admit_order(order_fields(
            two_bad,
            vec![record_value("Fee", &[("fee_minor", "1")])]
        ))),
        refusal("invalid-value", "total", vec![field_step("amount_minor")])
    );
    // Within a sequence the first defective element is reached first.
    let fee = || record_value("Fee", &[("fee_minor", "1")]);
    assert_eq!(
        refusal_of(admit_order(order_fields(
            money("1"),
            vec![money("1"), fee(), fee()]
        ))),
        refusal("wrong-value-kind", "items", vec![PathStep::Index(1)])
    );
}

/// The text of a chain of `depth` `Node { level, next: Node? }` records,
/// each holding the next as a `present` value; `serde_json::Value` cannot
/// hold it, so it is spliced into the snapshot text.
fn node_chain(depth: usize) -> String {
    let node = fixture::id("Node");
    let mut text = String::new();
    for level in 0..depth {
        text.push_str(&format!(
            r#"{{"record":{{"type":"{node}","fields":{{"level":{{"integer":"{level}"}},"next":"#
        ));
        text.push_str(if level + 1 < depth {
            r#"{"present":"#
        } else {
            r#"{"absent":{}}"#
        });
    }
    for level in (0..depth).rev() {
        if level + 1 < depth {
            text.push('}');
        }
        text.push_str("}}}");
    }
    text
}

/// The `Order` fields with `chain` holding the chain text's placeholder.
fn chain_fields() -> serde_json::Map<String, Value> {
    let mut fields = order_fields(money("1"), vec![]);
    fields.insert("chain".to_owned(), json!({"present": "CHAIN"}));
    fields
}

/// FR-106-AC-16: a recursive `Node { next?: Node }` nested 1,000 levels deep,
/// well past any fixed bound, within the byte and value limits, admits with
/// no depth refusal, on an explicit work stack.
#[trace("FR-106-AC-16", "FR-359-AC-9", "TC-1002")]
#[test]
fn a_recursive_record_nested_far_beyond_256_levels_admits() {
    let limits = ObservationLimits::default().with_values_per_document(1_000_000);
    for depth in [1_000, 10_000] {
        let limits = limits.with_document_bytes(4_000_000);
        admit_order_text(chain_fields(), limits, |text| {
            text.replacen("\"CHAIN\"", &node_chain(depth), 1)
        })
        .unwrap_or_else(|failure| panic!("a {depth}-deep chain admits: {failure:?}"));
    }
}

/// The record of a values-per-document refusal, asserting its typed
/// fields: `stage_limit_exceeded`/`node-count-exceeded` naming
/// `observation.values`, the bound and the count the refused value would
/// have reached (FR-255).
fn assert_values_limit(failure: AdmissionFailure, limit: u64) -> AdmissionRecord {
    let AdmissionFailure::Refused(record) = failure else {
        panic!("a refusal, not {failure:?}");
    };
    assert_eq!(record.code.as_str(), "stage_limit_exceeded");
    assert_eq!(record.cause, "node-count-exceeded");
    let field = |name: &str| record.fields.get(name).map(String::as_str);
    assert_eq!(field("setting"), Some("observation.values"));
    assert_eq!(field("kind"), Some("node-count-exceeded"));
    assert_eq!(field("bound"), Some(limit.to_string().as_str()));
    assert_eq!(field("actual"), Some((limit + 1).to_string().as_str()));
    record
}

/// FR-106-AC-16: at the exact values-per-document bound a document admits;
/// one below it, the last value refuses, before it is admitted.
#[trace("FR-106-AC-16")]
#[test]
fn the_exact_values_bound_admits_and_one_below_refuses_stage_limit_exceeded() {
    let rewrite = |text: String| text.replacen("\"CHAIN\"", &node_chain(20), 1);
    let used = admit_order_usage(chain_fields(), ObservationLimits::default(), rewrite)
        .expect("a 20-deep chain admits under the default bound")
        .values;
    assert!(used > 60, "the chain's forms are counted: {used}");
    let exact = ObservationLimits::default().with_values_per_document(used);
    admit_order_text(chain_fields(), exact, rewrite).expect("the exact bound admits");
    let below = ObservationLimits::default().with_values_per_document(used - 1);
    let Err(failure) = admit_order_text(chain_fields(), below, rewrite) else {
        panic!("one value past the bound refuses");
    };
    assert_values_limit(failure, used - 1);
}

/// FR-106-AC-16: a chain past the values-per-document bound refuses
/// `stage_limit_exceeded` naming `observation.values`, the bound and the
/// count reached, and a defect at the bottom of a chain names its whole
/// typed path: a field, an option-value step, a field, and so on.
#[trace("FR-106-AC-16")]
#[test]
fn a_deep_chain_is_bounded_by_the_value_limit_and_names_its_path() {
    let limit = 100;
    let limits = ObservationLimits::default().with_values_per_document(limit);
    let chain = node_chain(300);
    let Err(failure) = admit_order_text(chain_fields(), limits, |text| {
        text.replacen("\"CHAIN\"", &chain, 1)
    }) else {
        panic!("a chain past the value bound refuses");
    };
    assert_values_limit(failure, limit);
    // A defect two levels down: Order.chain -> Node.next -> Node.level.
    let bad = json!({"record": {"type": fixture::id("Node"), "fields": {
        "level": {"integer": "0"},
        "next": {"present": {"record": {"type": fixture::id("Node"), "fields": {
            "level": {"boolean": true}, "next": {"absent": {}}}}}},
    }}});
    let mut fields = order_fields(money("1"), vec![]);
    fields.insert("chain".to_owned(), json!({"present": bad}));
    assert_eq!(
        refusal_of(admit_order(fields)),
        refusal(
            "wrong-value-kind",
            "chain",
            vec![
                PathStep::OptionValue,
                field_step("next"),
                PathStep::OptionValue,
                field_step("level")
            ]
        )
    );
}

/// FR-106-AC-15: a `present` wrapper adds an option-value step; a scalar
/// element of a record's sequence field has its index; and a record field
/// inside a sequence element is a field step after the index step. A step
/// is typed, so a position never reads as a field name (a model field cannot
/// be named `0`: its identity must be `<Type>/<name>` with an identifier).
#[trace("FR-106-AC-15", "FR-359-AC-9", "TC-1002")]
#[test]
fn a_nested_path_carries_option_value_and_index_steps() {
    let mut fields = order_fields(money("1"), vec![]);
    fields.insert(
        "discount".to_owned(),
        json!({"present": record_value("TaxedMoney", &[("amount_minor", "1"), ("currency", "978")])}),
    );
    assert_eq!(
        refusal_of(admit_order(fields)),
        refusal(
            "missing-member",
            "discount",
            vec![PathStep::OptionValue, field_step("tax_minor")]
        )
    );
    let bundle = |amounts: Value, tagged: Value| {
        let mut fields = order_fields(money("1"), vec![]);
        fields.insert(
            "bundle".to_owned(),
            json!({"present": {"record": {"type": fixture::id("Bundle"), "fields": {
                "amounts": amounts, "tagged": tagged,
            }}}}),
        );
        fields
    };
    let tagged =
        |value: Value| json!({"record": {"type": fixture::id("Tagged"), "fields": {"tag": value}}});
    // A scalar element of a sequence field inside a record: option value,
    // the field, then the element's index.
    assert_eq!(
        refusal_of(admit_order(bundle(
            json!({"sequence": [{"integer": "1"}, {"integer": "x"}]}),
            json!({"sequence": []}),
        ))),
        refusal(
            "invalid-value",
            "bundle",
            vec![
                PathStep::OptionValue,
                field_step("amounts"),
                PathStep::Index(1)
            ]
        )
    );
    // A defect in the second element's field: the exact steps are the
    // position, then the field.
    assert_eq!(
        refusal_of(admit_order(bundle(
            json!({"sequence": []}),
            json!({"sequence": [tagged(json!({"integer": "1"})), tagged(json!({"integer": "x"}))]}),
        ))),
        refusal(
            "invalid-value",
            "bundle",
            vec![
                PathStep::OptionValue,
                field_step("tagged"),
                PathStep::Index(1),
                field_step("tag")
            ]
        )
    );
    assert_eq!(
        refusal_of(admit_order(bundle(
            json!({"sequence": []}),
            json!({"sequence": [{"record": {"type": fixture::id("Tagged"), "fields": {}}}]}),
        ))),
        refusal(
            "missing-member",
            "bundle",
            vec![
                PathStep::OptionValue,
                field_step("tagged"),
                PathStep::Index(0),
                field_step("tag")
            ]
        )
    );
}
