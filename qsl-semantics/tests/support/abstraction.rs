// SPDX-License-Identifier: AGPL-3.0-or-later
//! The abstraction-relation fixture (FR-304, FR-305, FR-307): the object
//! type `ConfigVersion` (fields `version` and `note`, the operation
//! `attemptUpdate(n, m)` framed on `version`) in the population `configs`,
//! and `Base` (field `f`, operation `op`) with its subtype `Sub`, outside
//! any population. `Panel` is a systems interface, which has an object type
//! record but no object type node.

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

fn parameter(operation: &str, name: &str) -> Value {
    let node = format!("{operation}/{name}");
    json!({
        "identity": id(&node), "name": name, "typeRef": INTEGER,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": base::origin(&node),
    })
}

/// The operation `name` of `owner` over the Integer `parameters`, framed
/// `frame`.
fn operation(owner: &str, name: &str, parameters: &[&str], frame: &Value) -> Value {
    let node = format!("{owner}/{name}");
    let params: Vec<Value> = parameters
        .iter()
        .map(|parameter_name| parameter(&node, parameter_name))
        .collect();
    json!({
        "identity": id(&node), "name": name, "params": params, "pre": [], "post": [],
        "frame": frame, "origin": base::origin(&node),
    })
}

fn object_type(name: &str, supertypes: &[&str], fields: &[&str], operations: Vec<Value>) -> Value {
    let identity = format!("{name}_id");
    let owner = supertypes.first().copied().unwrap_or(name);
    let mut all = Vec::new();
    if supertypes.is_empty() {
        all.push(base::field(name, &identity, INTEGER));
    }
    all.extend(fields.iter().map(|field| base::field(name, field, INTEGER)));
    base::declaration(
        name,
        "thing",
        json!({
            "supertypes": supertypes.iter().map(|ty| id(ty)).collect::<Vec<_>>(),
            "identityFields": [id(&format!("{owner}/{owner}_id"))],
            "fields": all,
            "operations": operations,
        }),
    )
}

/// The domain package document.
pub(crate) fn document() -> Value {
    let frame = json!({
        "modifies": [id("ConfigVersion/version")], "creates": [], "deletes": [],
    });
    let nothing = json!({"modifies": [], "creates": [], "deletes": []});
    let types = vec![
        object_type(
            "ConfigVersion",
            &[],
            &["version", "note"],
            vec![operation(
                "ConfigVersion",
                "attemptUpdate",
                &["n", "m"],
                &frame,
            )],
        ),
        object_type(
            "Base",
            &[],
            &["f"],
            vec![operation("Base", "op", &[], &nothing)],
        ),
        object_type("Sub", &["Base"], &[], Vec::new()),
        base::declaration(
            "Panel",
            "interface",
            json!({
                "supertypes": [],
                "fields": [base::field("Panel", "level", INTEGER)],
                "operations": [],
                "featureOrder": [id("Panel/level")],
            }),
        ),
    ];
    let mut document = base::domain_document(
        types,
        vec![
            base::construct("thing", meaning::OBJECT_TYPE),
            base::construct("population", meaning::POPULATION),
            base::construct("interface", meaning::SYSTEMS_INTERFACE),
        ],
    );
    document["populations"] = json!([{
        "identity": id("configs"), "displayName": id("configs"),
        "kind": {"module": base::BUSINESS, "name": "population"},
        "members": [id("ConfigVersion")], "extent": "closed",
        "origin": base::origin("configs"),
    }]);
    document
}
