// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1043's package: the object types `Counter` (fields `value` and `step`)
//! and `Gauge` (field `d`), and the operations `bump`, `merge`, `noop`,
//! `own`, `all`, `same`, `drop` and `dispose` on `Counter`, each with the
//! model-document `frame` the test case lists (FR-154 "Inputs"). `same`
//! is fixture R's operation whose frame equals `all`'s (step 2b).

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

/// The model-document `frame` of the operation `name` (TC-1043's table).
pub(crate) fn frame_of(name: &str) -> Value {
    let value = id("Counter/value");
    let step = id("Counter/step");
    match name {
        "bump" => json!({
            "modifies": [value], "creates": [], "deletes": [],
            "modifies_receiver": [step], "deletes_receiver": true,
        }),
        "merge" => json!({
            "modifies": [], "creates": [], "deletes": [id("Counter")],
            "deletes_parameters": ["b", "a"], "deletes_receiver": true,
        }),
        "noop" => json!({"modifies": [], "creates": [], "deletes": []}),
        "own" => json!({
            "modifies": [], "creates": [], "deletes": [],
            "modifies_receiver": [value],
        }),
        "all" | "same" => json!({"modifies": [value], "creates": [], "deletes": []}),
        "drop" => json!({
            "modifies": [], "creates": [], "deletes": [], "deletes_receiver": true,
        }),
        "dispose" => json!({
            "modifies": [], "creates": [], "deletes": [id("Gauge")],
            "deletes_receiver": true,
        }),
        other => panic!("{other} is no TC-1043 operation"),
    }
}

fn parameter(operation: &str, name: &str) -> Value {
    let node = format!("{operation}/{name}");
    json!({
        "identity": id(&node), "name": name, "typeRef": id("Counter"),
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": base::origin(&node),
    })
}

/// The operation `name` of `Counter` with `frame`.
fn operation(name: &str, frame: Value) -> Value {
    let node = format!("Counter/{name}");
    let params: Vec<Value> = if name == "merge" {
        vec![parameter(&node, "a"), parameter(&node, "b")]
    } else {
        Vec::new()
    };
    json!({
        "identity": id(&node), "name": name, "params": params, "pre": [], "post": [],
        "frame": frame, "origin": base::origin(&node),
    })
}

/// The package holding `operations` of `Counter`, with the frames of
/// `overrides` in place of the table's.
pub(crate) fn document(operations: &[&str], overrides: &[(&str, Value)]) -> Value {
    let operations: Vec<Value> = operations
        .iter()
        .map(|name| {
            let frame = overrides
                .iter()
                .find(|(overridden, _)| overridden == name)
                .map_or_else(|| frame_of(name), |(_, frame)| frame.clone());
            operation(name, frame)
        })
        .collect();
    let types = vec![
        base::declaration(
            "Counter",
            "thing",
            json!({
                "identityFields": [id("Counter/counter_id")],
                "fields": [
                    base::field("Counter", "counter_id", "ix://quire/native/Integer"),
                    base::field("Counter", "value", "ix://quire/native/Integer"),
                    base::field("Counter", "step", "ix://quire/native/Integer"),
                ],
                "operations": operations,
            }),
        ),
        base::declaration(
            "Gauge",
            "thing",
            json!({
                "identityFields": [id("Gauge/gauge_id")],
                "fields": [
                    base::field("Gauge", "gauge_id", "ix://quire/native/Integer"),
                    base::field("Gauge", "d", "ix://quire/native/Integer"),
                ],
            }),
        ),
    ];
    let mut document = base::domain_document(
        types,
        vec![
            base::construct("thing", meaning::OBJECT_TYPE),
            base::construct("population", meaning::POPULATION),
        ],
    );
    document["populations"] = json!([{
        "identity": id("counters"), "displayName": id("counters"),
        "kind": {"module": base::BUSINESS, "name": "population"},
        "members": [id("Counter"), id("Gauge")], "extent": "closed",
        "origin": base::origin("counters"),
    }]);
    document
}

/// The unit text naming each of `operations` by its own postcondition clause
/// (`DoneBump`, ...) so the package holds its frame node.
pub(crate) fn posts(operations: &[&str]) -> String {
    operations
        .iter()
        .map(|name| {
            let mut title = name.to_string();
            title[..1].make_ascii_uppercase();
            format!("post Done{title} using v on M::Counter::{name} {{ true }}\n")
        })
        .collect()
}
