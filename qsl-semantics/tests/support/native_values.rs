// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FR-106 native payload fixture (TC-1029 and TC-1036): the object type
//! `Item` with String, UUID and Timestamp fields, a present option, a
//! sequence and a reference of each, and operations that grant or do not
//! grant a change to them.
//!
//! - `Item { item_id: Integer; token: UUID; stamp: Timestamp; label: String;
//!   prior?: UUID; note?: String; seen: Sequence<Timestamp>; tags:
//!   Sequence<String>; versionNumber: Integer; parent?: Reference<Item> }`.
//! - `touch(id: UUID, at: Timestamp): UUID` modifying `token` and `stamp`;
//!   `rename(name: String): String` modifying `label`; `hold` and `keep`,
//!   the same signatures granting nothing.

#![allow(dead_code, reason = "each test binary uses part of the fixture")]

use serde_json::{json, Value};

#[path = "tc235_k2.rs"]
#[allow(dead_code, reason = "this fixture uses part of the shared K2 builder")]
#[allow(
    clippy::duplicate_mod,
    reason = "a test binary also loads the K2 builder directly"
)]
mod base;
#[path = "variant_fields.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "a test binary may load the variant fixture directly"
)]
mod variants;
pub(crate) use variants::{id, PACKAGE};

const NATIVE: &str = "ix://quire/native";

fn type_ref(name: &str) -> String {
    format!("{NATIVE}/{name}")
}

/// The package document.
pub(crate) fn document() -> Value {
    build(false)
}

/// The package document with the Text value types `Title` (`Text[1, 8;
/// nfc]`), `Tag` (`Text[0, 4; unicode-scalars]`) and `Code` (`Text[0, 4;
/// binary-utf8]`), declared in the form of FR-056-AC-37, and `Item` also
/// holding, after its other fields, `title: Title`, `subtitle?: Tag`, `code:
/// Code`, `address: Address` (record `Address { street: Title }`) and
/// `aliases: Sequence<Tag>`. `retitle(name: Title): Title` modifies
/// `title`; `rekeep` is the same signature granting nothing.
pub(crate) fn text_document() -> Value {
    build(true)
}

/// A Text value type `name`: `minLength`, `maxLength` and a `format` naming
/// `profile`, each applying to the value type itself.
pub(crate) fn text_value_type(name: &str, min: u64, max: u64, profile: &str) -> Value {
    let constraint = |keyword: &str, operands: Value| {
        json!({
            "identity": id(&format!("constraint/{name}/{keyword}")),
            "keyword": keyword,
            "operands": operands,
            "appliesTo": id(name),
            "diagnosticCode": format!("K.{}_{}", name.to_uppercase(), keyword.to_uppercase()),
            "origin": base::origin(&format!("constraint/{name}/{keyword}")),
        })
    };
    base::declaration(
        name,
        "scalar",
        json!({
            "scalar": "string",
            "constraints": [
                constraint("minLength", json!({"value": min})),
                constraint("maxLength", json!({"value": max})),
                constraint("format", json!({"name": format!("text-profile:{profile}")})),
            ],
        }),
    )
}

/// The package document with `Item` also holding, after its other fields, the
/// collection fields of TC-1171: `ids: Set<UUID>[0, 3]`, `counts:
/// Bag<Digit>[0, 4]`, `order: OrderedSet<Digit>[0, 3]`, `stamps:
/// OrderedSet<Timestamp>[0, 2]` and `peers: Set<Reference<Item>>[0, 2]`
/// (`Digit` is `Int[0, 9]`), a second population `others` of `Item`, and the
/// operation `retag(ids: Set<UUID>[0, 3], counts: Bag<Digit>[0, 4], order:
/// OrderedSet<Digit>[0, 3]): Set<UUID>[0, 3]`, which grants no field, and
/// `restock`, the same signature granting `counts`.
pub(crate) fn collection_document() -> Value {
    build_with(Flavor::Collections)
}

#[derive(Clone, Copy, PartialEq)]
enum Flavor {
    Plain,
    Text,
    Collections,
}

/// A field of `Item` called `name` of `type_ref`, with the multiplicity of a
/// collection of `kind`, `lower..=upper` entries.
fn collection_field(name: &str, type_ref: &str, ordered: bool, unique: bool, upper: u64) -> Value {
    let mut field = variants::many("Item", name, type_ref, ordered, unique);
    field["multiplicity"]["upper"] = json!(upper);
    field
}

fn build(text: bool) -> Value {
    build_with(if text { Flavor::Text } else { Flavor::Plain })
}

fn build_with(flavor: Flavor) -> Value {
    let text = flavor == Flavor::Text;
    let collections = flavor == Flavor::Collections;
    let mut item = base::declaration(
        "Item",
        "thing",
        json!({
            "identityFields": [id("Item/item_id")],
            "fields": [
                variants::field("Item", "item_id", &type_ref("Integer")),
                variants::field("Item", "token", &type_ref("UUID")),
                variants::field("Item", "stamp", &type_ref("Timestamp")),
                variants::field("Item", "label", &type_ref("String")),
                variants::optional("Item", "prior", &type_ref("UUID")),
                variants::optional("Item", "note", &type_ref("String")),
                variants::many("Item", "seen", &type_ref("Timestamp"), true, false),
                variants::many("Item", "tags", &type_ref("String"), true, false),
                variants::field("Item", "versionNumber", &type_ref("Integer")),
                variants::optional("Item", "parent", &id("Item")),
            ],
        }),
    );
    let uuid = type_ref("UUID");
    let timestamp = type_ref("Timestamp");
    let string = type_ref("String");
    let touch_parameters = [("id", uuid.clone()), ("at", timestamp)];
    let rename_parameters = [("name", string.clone())];
    let mut touch = variants::operation_with("Item", "touch", &touch_parameters, &uuid);
    touch["frame"]["modifies"] = json!([id("Item/token"), id("Item/stamp")]);
    let mut rename = variants::operation_with("Item", "rename", &rename_parameters, &string);
    rename["frame"]["modifies"] = json!([id("Item/label")]);
    item["operations"] = json!([
        touch,
        rename,
        variants::operation_with("Item", "hold", &touch_parameters, &uuid),
        variants::operation_with("Item", "keep", &rename_parameters, &string),
    ]);
    let mut types = vec![];
    if collections {
        let digit = id("Digit");
        item["fields"]
            .as_array_mut()
            .expect("Item has fields")
            .extend([
                collection_field("ids", &uuid, false, true, 3),
                collection_field("counts", &digit, false, false, 4),
                collection_field("order", &digit, true, true, 3),
                collection_field("stamps", &type_ref("Timestamp"), true, true, 2),
                collection_field("peers", &id("Item"), false, true, 2),
            ]);
        let signature = [
            ("ids", uuid.clone()),
            ("counts", digit.clone()),
            ("order", digit.clone()),
        ];
        let mut retag = variants::operation_with("Item", "retag", &signature, &uuid);
        // `restock` is `retag` granting `counts`.
        let mut restock = variants::operation_with("Item", "restock", &signature, &uuid);
        restock["frame"]["modifies"] = json!([id("Item/counts")]);
        for operation in [&mut retag, &mut restock] {
            for (index, (ordered, unique, upper)) in
                [(false, true, 3), (false, false, 4), (true, true, 3)]
                    .into_iter()
                    .enumerate()
            {
                operation["params"][index]["multiplicity"] =
                    json!({"lower": 0, "upper": upper, "ordered": ordered, "unique": unique});
            }
            operation["returns"]["multiplicity"] =
                json!({"lower": 0, "upper": 3, "ordered": false, "unique": true});
        }
        item["operations"]
            .as_array_mut()
            .expect("Item has operations")
            .extend([retag, restock]);
        let constraint = |keyword: &str, value: &str| {
            json!({
                "identity": format!("{digit}/constraints/{keyword}"),
                "keyword": keyword,
                "operands": {"value": value},
                "appliesTo": type_ref("Integer"),
                "diagnosticCode": format!("bound.{keyword}"),
                "origin": base::origin("Digit"),
            })
        };
        let mut bounded = base::declaration("Digit", "bounded", json!({}));
        bounded["scalar"] = json!("integer");
        bounded["constraints"] = json!([constraint("min", "0"), constraint("max", "9")]);
        types.push(bounded);
    }
    if text {
        let (title, tag, code) = (id("Title"), id("Tag"), id("Code"));
        item["fields"]
            .as_array_mut()
            .expect("Item has fields")
            .extend([
                variants::field("Item", "title", &title),
                variants::optional("Item", "subtitle", &tag),
                variants::field("Item", "code", &code),
                variants::field("Item", "address", &id("Address")),
                variants::many("Item", "aliases", &tag, true, false),
            ]);
        let mut retitle =
            variants::operation_with("Item", "retitle", &[("name", title.clone())], &title);
        retitle["frame"]["modifies"] = json!([id("Item/title")]);
        let keep = variants::operation_with("Item", "rekeep", &[("name", title.clone())], &title);
        item["operations"]
            .as_array_mut()
            .expect("Item has operations")
            .extend([retitle, keep]);
        types.extend([
            text_value_type("Title", 1, 8, "nfc"),
            text_value_type("Tag", 0, 4, "unicode-scalars"),
            text_value_type("Code", 0, 4, "binary-utf8"),
            // FR-056-AC-21: the record value type `Address { street: Title }`.
            base::declaration(
                "Address",
                "money",
                json!({"fields": [variants::field("Address", "street", &title)]}),
            ),
        ]);
    }
    types.push(item);
    let mut document = variants::package(types, true);
    if text {
        // A value type carrying a `scalar` is a record-shaped construct
        // (the `value` construct is the `target` alias).
        let mut scalar = json!({
            "kind": {"module": base::BUSINESS, "name": "scalar"},
            "moduleVersion": "1.0.0",
            "construct": {
                "identity": "none", "shape": "record", "members": {},
                "meaning": "quire.meaning.model.value-type/v1",
            },
        });
        scalar["manifestDigest"] = document["package"]["manifestDigest"].clone();
        document["constructs"]
            .as_array_mut()
            .expect("constructs")
            .push(scalar);
    }
    if collections {
        let manifest = document["package"]["manifestDigest"].clone();
        document["constructs"]
            .as_array_mut()
            .expect("constructs")
            .push({
                let mut bounded = json!({
                    "kind": {"module": base::BUSINESS, "name": "bounded"},
                    "moduleVersion": "1.0.0",
                    "construct": {
                        "identity": "none", "shape": "record", "members": {},
                        "meaning": "quire.meaning.model.value-type/v1",
                    },
                });
                bounded["manifestDigest"] = manifest;
                bounded
            });
    }
    let population = |name: &str| {
        json!({
            "identity": id(name), "displayName": id(name),
            "kind": {"module": base::BUSINESS, "name": "population"},
            "members": [id("Item")], "extent": "closed",
            "origin": base::origin(name),
        })
    };
    document["populations"] = if collections {
        json!([population("items"), population("others")])
    } else {
        json!([population("items")])
    };
    document
}
