// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runtime TC-235 vectors using the single shared K2 producer fixture.

use crate::tc235_k2::{id, k2, mutate_k2, type_named_mut};
use serde_json::json;

#[test]
fn tc235_eval_uses_the_shared_k2_fixture_without_reencoding_it() {
    let original = k2();
    let unchanged = mutate_k2(|_| {});
    assert_eq!(original, unchanged);
    assert_eq!(original["types"].as_array().unwrap().len(), 8);
    assert_eq!(
        original["types"][1]["kind"],
        json!({"module": "test/business", "name": "value"})
    );
    assert_eq!(
        original["types"][1]["target"],
        "ix://quire/native/Timestamp"
    );
    assert_eq!(
        type_named_mut(&mut original.clone(), "OrderLifecycle")["identity"],
        id("OrderLifecycle")
    );
}
