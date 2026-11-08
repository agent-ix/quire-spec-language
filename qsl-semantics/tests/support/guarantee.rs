// SPDX-License-Identifier: AGPL-3.0-or-later
//! The guarantee-declaration fixture (QSpec TC-982, TC-984): the compare-and-set
//! counter model. The object type `Counter` has the field `value` and the
//! operation `inc` framed on `value`; `with_dec` adds a second operation
//! `dec`. `counters` is its population.

#![allow(dead_code, reason = "each test binary uses part of the fixture")]

use qsl_semantics::model::intake::meaning;
use serde_json::{json, Value};

#[path = "tc235_k2.rs"]
#[allow(dead_code, reason = "this fixture uses part of the shared K2 builder")]
#[allow(
    clippy::duplicate_mod,
    reason = "a test binary also loads the K2 builder directly"
)]
mod base;
pub(crate) use base::{id, PACKAGE};

const INTEGER: &str = "ix://quire/native/Integer";

/// The operation `name` of `Counter`, framed on `value`.
fn operation(name: &str) -> Value {
    let node = format!("Counter/{name}");
    json!({
        "identity": id(&node), "name": name, "params": [], "pre": [], "post": [],
        "frame": {"modifies": [id("Counter/value")], "creates": [], "deletes": []},
        "origin": base::origin(&node),
    })
}

/// The domain package document with the operations `operations`.
pub(crate) fn document(operations: &[&str]) -> Value {
    let counter = base::declaration(
        "Counter",
        "thing",
        json!({
            "supertypes": [],
            "identityFields": [id("Counter/counter_id")],
            "fields": [
                base::field("Counter", "counter_id", INTEGER),
                base::field("Counter", "value", INTEGER),
            ],
            "operations": operations.iter().map(|name| operation(name)).collect::<Vec<_>>(),
        }),
    );
    let mut document = base::domain_document(
        vec![counter],
        vec![
            base::construct("thing", meaning::OBJECT_TYPE),
            base::construct("population", meaning::POPULATION),
        ],
    );
    document["populations"] = json!([{
        "identity": id("counters"), "displayName": id("counters"),
        "kind": {"module": base::BUSINESS, "name": "population"},
        "members": [id("Counter")], "extent": "closed",
        "origin": base::origin("counters"),
    }]);
    document
}

/// The same document as a second package, `test/other`.
pub(crate) fn other_document(operations: &[&str]) -> Value {
    let text = serde_json::to_string(&document(operations)).expect("the fixture serializes");
    serde_json::from_str(&text.replace(PACKAGE, "test/other")).expect("the fixture reads")
}
