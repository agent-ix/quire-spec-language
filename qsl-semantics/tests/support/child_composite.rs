// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FR-359 domain package: record value types in a
//! parent/child/grandchild chain, the records that hold them at ancestor
//! slots, and one object type, shared by the QSL semantics, evaluator and
//! replay tests (TC-998 to TC-1002).
//!
//! The effective fields of a type are ordered by their own declaration keys
//! (`<Owner>/<name>`), so the fixture's names make a child's fields NOT a
//! positional prefix of its parent's: `ExpressOrderPlaced` is
//! `[promised_by, occurred_at, order_id]` while `OrderPlaced` is
//! `[occurred_at, order_id]`.

#![allow(dead_code, reason = "each test binary uses part of the fixture")]

use serde_json::{json, Value};

#[path = "tc235_k2.rs"]
#[allow(dead_code, reason = "this fixture uses part of the shared K2 builder")]
#[allow(
    clippy::duplicate_mod,
    reason = "a test binary also loads the K2 builder directly"
)]
mod base;
pub(crate) use base::{id, PACKAGE};

const NATIVE: &str = "ix://quire/native";

fn native(name: &str) -> String {
    format!("{NATIVE}/{name}")
}

/// A field `owner/name` of `type_ref`.
fn plain(owner: &str, name: &str, type_ref: &str) -> Value {
    base::field(owner, name, type_ref)
}

/// An optional field: absent or one value.
fn optional(owner: &str, name: &str, type_ref: &str) -> Value {
    let mut field = base::field(owner, name, type_ref);
    field["presence"] = json!("optional");
    field
}

/// A field holding between zero and `upper` values with the given
/// `ordered` and `unique` flags.
fn many(owner: &str, name: &str, type_ref: &str, ordered: bool, unique: bool) -> Value {
    let mut field = base::field(owner, name, type_ref);
    field["multiplicity"] = json!({"lower": 0, "upper": 8, "ordered": ordered, "unique": unique});
    field
}

/// A field of `owner` that redefines `parent_owner/name` under the same
/// name, typed `type_ref`.
fn redefining(owner: &str, parent_owner: &str, name: &str, type_ref: &str) -> Value {
    let mut field = base::field(owner, name, type_ref);
    field["redefines"] = json!(id(&format!("{parent_owner}/{name}")));
    field
}

/// A field of `owner` called `name` that redefines `parent_owner/parent_name`
/// under another name.
fn renaming(
    owner: &str,
    name: &str,
    parent_owner: &str,
    parent_name: &str,
    type_ref: &str,
) -> Value {
    let mut field = base::field(owner, name, type_ref);
    field["redefines"] = json!(id(&format!("{parent_owner}/{parent_name}")));
    field
}

/// A record value type `name`, specializing `supertypes`.
fn record(name: &str, supertypes: &[&str], fields: Vec<Value>) -> Value {
    base::declaration(
        name,
        "money",
        json!({
            "supertypes": supertypes.iter().map(|ty| id(ty)).collect::<Vec<_>>(),
            "fields": fields,
        }),
    )
}

/// `Digit`, the integers `0..=9`: a bounded `value` type over the native
/// Integer, declared with its own construct (a `value` construct with a
/// target and constraints has no reader).
fn digit() -> Value {
    let identity = id("Digit");
    let constraint = |keyword: &str, value: &str| {
        json!({
            "identity": format!("{identity}/constraints/{keyword}"),
            "keyword": keyword,
            "operands": {"value": value},
            "appliesTo": native("Integer"),
            "diagnosticCode": format!("bound.{keyword}"),
            "origin": base::origin("Digit"),
        })
    };
    let mut digit = base::declaration("Digit", "bounded", json!({}));
    digit["scalar"] = json!("integer");
    digit["constraints"] = json!([constraint("min", "0"), constraint("max", "9")]);
    digit
}

/// An operation `owner/name` taking `parameters` (name, type) and returning
/// `result`.
fn operation_with(owner: &str, name: &str, parameters: &[(&str, String)], result: &str) -> Value {
    let node = format!("{owner}/{name}");
    let params: Vec<Value> = parameters
        .iter()
        .map(|(parameter, type_ref)| {
            let identity = format!("{node}/{parameter}");
            json!({
                "identity": id(&identity), "name": parameter, "typeRef": type_ref,
                "presence": "required", "nullable": false, "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": base::origin(&identity),
            })
        })
        .collect();
    json!({
        "identity": id(&node), "name": name, "params": params,
        "returns": {
            "typeRef": result,
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        },
        "pre": [], "post": [],
        "frame": {"modifies": [], "creates": [], "deletes": []},
        "origin": base::origin(&node),
    })
}

/// The FR-359 package document.
///
/// - `Money <- TaxedMoney <- GiftTaxedMoney`, and the unrelated `Fee`.
/// - `OrderPlaced <- ExpressOrderPlaced <- RushOrderPlaced`.
/// - `Pair { first: Money; second: Money }`, `Holder` of an `OrderPlaced`
///   (required, optional and a sequence), `Invoice` of an optional `Money`
///   `total` and `discount`, and `Lines` of a `Money` sequence, set, bag and
///   ordered set.
/// - `Quant { n: Integer }` with `NarrowQuant` redefining `n` as `Digit`;
///   `Stamp { at: OrderPlaced }` with `ExpressStamp` redefining `at` as an
///   `ExpressOrderPlaced`, and `RenamedStamp` redefining `at` as `when`.
/// - The object type `Order` with a `Money` total, an optional discount and a
///   sequence of items, and `Ledger` with `pay(m: Money): Money`.
pub(crate) fn document() -> Value {
    let money = id("Money");
    let placed = id("OrderPlaced");
    let types = vec![
        digit(),
        record(
            "Money",
            &[],
            vec![
                plain("Money", "amount_minor", &native("Integer")),
                plain("Money", "currency", &native("Integer")),
            ],
        ),
        record(
            "TaxedMoney",
            &["Money"],
            vec![plain("TaxedMoney", "tax_minor", &native("Integer"))],
        ),
        record(
            "GiftTaxedMoney",
            &["TaxedMoney"],
            vec![plain("GiftTaxedMoney", "wrap_minor", &native("Integer"))],
        ),
        record(
            "Fee",
            &[],
            vec![plain("Fee", "fee_minor", &native("Integer"))],
        ),
        record(
            "OrderPlaced",
            &[],
            vec![
                plain("OrderPlaced", "order_id", &native("Integer")),
                plain("OrderPlaced", "occurred_at", &native("Integer")),
            ],
        ),
        record(
            "ExpressOrderPlaced",
            &["OrderPlaced"],
            vec![plain(
                "ExpressOrderPlaced",
                "promised_by",
                &native("Integer"),
            )],
        ),
        record(
            "RushOrderPlaced",
            &["ExpressOrderPlaced"],
            vec![plain("RushOrderPlaced", "rush_by", &native("Integer"))],
        ),
        record(
            "Pair",
            &[],
            vec![
                plain("Pair", "first", &money),
                plain("Pair", "second", &money),
            ],
        ),
        record(
            "Holder",
            &[],
            vec![
                plain("Holder", "placed", &placed),
                optional("Holder", "maybe", &placed),
                many("Holder", "many", &placed, true, false),
            ],
        ),
        record(
            "Invoice",
            &[],
            vec![
                optional("Invoice", "total", &money),
                optional("Invoice", "discount", &money),
            ],
        ),
        record(
            "Lines",
            &[],
            vec![
                many("Lines", "lines", &money, true, false),
                many("Lines", "tags", &money, false, true),
                many("Lines", "counted", &money, false, false),
                many("Lines", "queue", &money, true, true),
            ],
        ),
        record(
            "TaxedInvoice",
            &[],
            vec![optional("TaxedInvoice", "total", &id("TaxedMoney"))],
        ),
        record("Quant", &[], vec![plain("Quant", "n", &native("Integer"))]),
        record(
            "NarrowQuant",
            &["Quant"],
            vec![redefining("NarrowQuant", "Quant", "n", &id("Digit"))],
        ),
        record("Stamp", &[], vec![plain("Stamp", "at", &placed)]),
        record(
            "ExpressStamp",
            &["Stamp"],
            vec![redefining(
                "ExpressStamp",
                "Stamp",
                "at",
                &id("ExpressOrderPlaced"),
            )],
        ),
        record(
            "RenamedStamp",
            &["Stamp"],
            vec![renaming(
                "RenamedStamp",
                "when",
                "Stamp",
                "at",
                &id("ExpressOrderPlaced"),
            )],
        ),
        {
            let mut order = base::declaration(
                "Order",
                "thing",
                json!({
                    "identityFields": [id("Order/order_id")],
                    "fields": [
                        plain("Order", "order_id", &native("Integer")),
                        plain("Order", "total", &money),
                        optional("Order", "discount", &money),
                        optional("Order", "taxed", &id("TaxedMoney")),
                        many("Order", "items", &money, true, false),
                    ],
                }),
            );
            order["operations"] = json!([]);
            order
        },
        {
            let mut ledger = base::declaration(
                "Ledger",
                "thing",
                json!({
                    "identityFields": [id("Ledger/ledger_id")],
                    "fields": [plain("Ledger", "ledger_id", &native("Integer"))],
                }),
            );
            ledger["operations"] = json!([
                operation_with("Ledger", "pay", &[("m", money.clone())], &money),
                operation_with("Ledger", "payTaxed", &[("m", id("TaxedMoney"))], &money),
            ]);
            ledger
        },
    ];
    let constructs = vec![
        base::construct("money", base_meaning::RECORD_VALUE_TYPE),
        base::construct("thing", base_meaning::OBJECT_TYPE),
        base::construct("population", base_meaning::POPULATION),
        {
            let mut bounded = base::construct("bounded", base_meaning::VALUE_TYPE);
            bounded["construct"]["members"] = json!({});
            bounded["construct"]["shape"] = json!("record");
            bounded
        },
    ];
    let mut document = base::domain_document(types, constructs);
    document["populations"] = json!([
        population("orders", "Order"),
        population("ledgers", "Ledger")
    ]);
    document
}

/// The closed population `name` of the object type `member`.
fn population(name: &str, member: &str) -> Value {
    json!({
        "identity": id(name), "displayName": id(name),
        "kind": {"module": base::BUSINESS, "name": "population"},
        "members": [id(member)], "extent": "closed",
        "origin": base::origin(name),
    })
}

use qsl_semantics::model::intake::meaning as base_meaning;

/// [`document`] with the event type `Shipped`, whose occurrence field is a
/// native Timestamp through the `Instant` value type. A unit over it checks
/// only while no function reaches the event's Timestamp field: lowering a
/// Timestamp leaf is not implemented on this ref, so it serves the checker's
/// refusals, which come before lowering.
pub(crate) fn document_with_event() -> Value {
    let mut document = document();
    let mut instant = base::declaration("Instant", "value", json!({}));
    instant["target"] = json!(native("Timestamp"));
    let event = base::declaration(
        "Shipped",
        "happened",
        json!({
            "supertypes": [],
            "occurrenceField": id("Shipped/occurred_at"),
            "fields": [
                plain("Shipped", "order_id", &native("Integer")),
                plain("Shipped", "occurred_at", &id("Instant")),
            ],
        }),
    );
    // A record holding an event directly, as an optional and in a sequence.
    let dispatch = record(
        "Dispatch",
        &[],
        vec![
            plain("Dispatch", "event", &id("Shipped")),
            optional("Dispatch", "maybe", &id("Shipped")),
            many("Dispatch", "log", &id("Shipped"), true, false),
        ],
    );
    document["types"]
        .as_array_mut()
        .unwrap()
        .extend([instant, event, dispatch]);
    document["constructs"].as_array_mut().unwrap().extend([
        base::construct("value", base_meaning::VALUE_TYPE),
        base::construct("happened", base_meaning::EVENT_TYPE),
    ]);
    refresh_digests(&mut document);
    document
}

/// `document` with every digest its construct manifests and package header
/// carry recomputed from its `types`.
fn refresh_digests(document: &mut Value) {
    let digest = base::jcs_digest(&document["types"]);
    document["source"]["digest"] = json!(digest);
    document["package"]["manifestDigest"] = json!(digest);
    document["package"]["lockDigest"] = json!(digest);
    for construct in document["constructs"].as_array_mut().unwrap() {
        construct["manifestDigest"] = json!(digest);
    }
}

/// [`document_with_event`] plus the native leaves PR #789 lowers: a
/// `Ticket { label: String }` with a `StampedTicket` child, and a
/// `Credential { token: UUID, issued: Timestamp }`. A unit that reads or
/// compares these fails lowering until #789 merges; the ignored tests that
/// use it name that blocker.
pub(crate) fn document_with_native_leaves() -> Value {
    let mut document = document_with_event();
    let types = [
        record(
            "Ticket",
            &[],
            vec![plain("Ticket", "label", &native("String"))],
        ),
        record(
            "StampedTicket",
            &["Ticket"],
            vec![plain("StampedTicket", "stamp", &native("Integer"))],
        ),
        record(
            "Credential",
            &[],
            vec![
                plain("Credential", "token", &native("UUID")),
                plain("Credential", "issued", &id("Instant")),
            ],
        ),
    ];
    document["types"].as_array_mut().unwrap().extend(types);
    refresh_digests(&mut document);
    document
}
