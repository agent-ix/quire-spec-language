// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-220: an independent recompute of an FR-051/FR-052 content identity
//! from a parsed document, not from the producer's preimage structs.

use serde_json::Value;
use sha2::{Digest as _, Sha256};

/// `value` with every JSON integer replaced by its decimal string, the
/// spelling the identity preimage uses.
fn decimal_integers(value: &mut Value) {
    match value {
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            *value = Value::String(number.to_string());
        }
        Value::Array(items) => items.iter_mut().for_each(decimal_integers),
        Value::Object(members) => members.values_mut().for_each(decimal_integers),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

/// Asserts that `value`'s `identity` member (a document's or a position's)
/// is its content identity under `domain`: `identity` removed, integers as
/// decimal strings, RFC 8785 text from `quire-canonical`, and the
/// `u64be(len(domain)) || domain || text` frame hashed here by hand.
pub fn assert_identity(value: &Value, domain: &str) {
    let mut preimage = value.clone();
    let identity = preimage
        .as_object_mut()
        .expect("identity-bearing object")
        .remove("identity")
        .expect("identity member");
    decimal_integers(&mut preimage);
    let limits = quire_canonical::Limits::new(1 << 24, 64).expect("limits");
    let text = quire_canonical::to_vec(&preimage, limits).expect("RFC 8785 preimage");
    let mut digest = Sha256::new();
    digest.update((domain.len() as u64).to_be_bytes());
    digest.update(domain.as_bytes());
    digest.update(&text);
    assert_eq!(identity, format!("{:x}", digest.finalize()));
}
