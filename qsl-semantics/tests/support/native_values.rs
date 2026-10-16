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
/// Code` and `aliases: Sequence<Tag>`. `retitle(name: Title): Title` modifies
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

fn build(text: bool) -> Value {
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
    if text {
        let (title, tag, code) = (id("Title"), id("Tag"), id("Code"));
        item["fields"]
            .as_array_mut()
            .expect("Item has fields")
            .extend([
                variants::field("Item", "title", &title),
                variants::optional("Item", "subtitle", &tag),
                variants::field("Item", "code", &code),
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
    document["populations"] = json!([{
        "identity": id("items"), "displayName": id("items"),
        "kind": {"module": base::BUSINESS, "name": "population"},
        "members": [id("Item")], "extent": "closed",
        "origin": base::origin("items"),
    }]);
    document
}
