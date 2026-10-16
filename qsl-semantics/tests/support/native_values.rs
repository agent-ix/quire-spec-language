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
    let mut document = variants::package(vec![item], true);
    document["populations"] = json!([{
        "identity": id("items"), "displayName": id("items"),
        "kind": {"module": base::BUSINESS, "name": "population"},
        "members": [id("Item")], "extent": "closed",
        "origin": base::origin("items"),
    }]);
    document
}
