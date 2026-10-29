// SPDX-License-Identifier: AGPL-3.0-or-later
//! The key vectors of FR-091 (`Q1` to `Q13`, `N1` to `N4`), recomputed at
//! test time.
//!
//! FR-091 states each vector's preimage content and that its key is the
//! SHA-256 of the RFC 8785 bytes of the preimage. This module writes those
//! preimages out by hand (every object below lists its members in ascending
//! key order, which is RFC 8785's order for these ASCII keys, and every
//! number is a decimal string) and hashes the bytes. It shares no code with
//! the production preimage types, and no key is recorded anywhere.
//!
//! The file depends on `serde_json` and `qsl_foundation` only, so the
//! integration tests include it by path as well.

use serde_json::{json, Value};

/// The lower-case hex SHA-256 of the RFC 8785 bytes of `preimage`, through
/// `quire-canonical` (the only canonical encoder in QSL).
fn hex(preimage: &Value) -> String {
    quire_canonical::sha256(preimage, quire_canonical::Limits::new(1 << 20))
        .expect("a preimage encodes canonically")
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn owner(identity: &str) -> Value {
    json!({"authority": "a", "identity": identity, "kind": "source"})
}

fn node_id(digest: &str) -> Value {
    json!({"digest": digest, "domain": "quire.checked-semantic-node/v1"})
}

fn rational(numerator: &str, denominator: &str) -> Value {
    json!({"denominator": denominator, "numerator": numerator})
}

/// The key of the node whose preimage is `preimage`.
fn key_of(preimage: &Value) -> String {
    hex(preimage)
}

fn dimension(name: &str, terms: &[(&str, &str)]) -> String {
    let terms: Vec<Value> = terms
        .iter()
        .map(|(id, exponent)| json!({"dimension_node_id": node_id(id), "exponent": exponent}))
        .collect();
    key_of(&json!({
        "owner": owner("u"),
        "qualified_declaration": [name],
        "terms": terms,
        "version": "quire.dimension-node/v1",
    }))
}

fn unit(
    name: &str,
    dimension_id: &str,
    target: Option<&str>,
    scale: (&str, &str),
    offset: (&str, &str),
) -> String {
    key_of(&json!({
        "dimension_node_id": node_id(dimension_id),
        "offset": rational(offset.0, offset.1),
        "owner": owner("u"),
        "qualified_declaration": [name],
        "scale": rational(scale.0, scale.1),
        "target_unit_node_id": target.map(node_id),
        "version": "quire.unit-node/v1",
    }))
}

fn equivalence(a: &str, b: &str, root_scale: (&str, &str)) -> String {
    let mut ids = [a.to_owned(), b.to_owned()];
    ids.sort();
    key_of(&json!({
        "dimension_node_ids": ids.map(|id| node_id(&id)),
        "owner": owner("u"),
        "root_scale": rational(root_scale.0, root_scale.1),
        "version": "quire.dimension-equivalence-node/v1",
    }))
}

fn enum_declaration(name: &str, identity: &str, ordered: bool, members: &[&str]) -> String {
    key_of(&json!({
        "members": members,
        "ordered": ordered,
        "owner": owner(identity),
        "qualified_declaration": [name],
        "version": "quire.enum-declaration-node/v1",
    }))
}

/// FR-091 vector `id`'s key, recomputed from the preimage content the spec
/// states for it. `#[string_edge]`: `id` is the spec's own vector label
/// (`Q1`..`N4`), the intake of this hand-written test-vector table.
#[ix_trace_rs::string_edge]
pub fn key(id: &str) -> String {
    let (one, zero) = (("1", "1"), ("0", "1"));
    match id {
        "Q1" => dimension("Length", &[]),
        "Q2" => dimension("Time", &[]),
        "Q3" => dimension("Speed", &[(&key("Q2"), "-1"), (&key("Q1"), "1")]),
        "Q4" => dimension("Accel", &[(&key("Q2"), "-2"), (&key("Q1"), "1")]),
        "Q5" => dimension("Temperature", &[]),
        "Q6" => unit("m", &key("Q1"), None, one, zero),
        "Q7" => unit("km", &key("Q1"), Some(&key("Q6")), ("1000", "1"), zero),
        "Q8" => unit("cm", &key("Q1"), Some(&key("Q6")), ("1", "100"), zero),
        "Q9" => unit("K", &key("Q5"), None, one, zero),
        "Q10" => unit("C", &key("Q5"), Some(&key("Q9")), one, ("5463", "20")),
        "Q11" => dimension("Distance", &[]),
        "Q12" => equivalence(&key("Q11"), &key("Q1"), ("1000", "1")),
        "Q13" => equivalence(&key("Q11"), &key("Q1"), ("1", "200")),
        "N1" => enum_declaration("Status", "u", true, &["READY", "DONE"]),
        "N2" => key_of(&json!({
            "case": "READY",
            "declaration_node_id": node_id(&key("N1")),
            "version": "quire.enum-member-node/v1",
        })),
        "N3" => enum_declaration("Color", "u", false, &["BLUE", "RED"]),
        "N4" => enum_declaration("Status", "w", true, &["READY", "DONE"]),
        _ => panic!("FR-091 has no vector {id}"),
    }
}
