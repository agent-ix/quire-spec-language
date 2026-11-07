// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compound quantity type source forms through the producer and strict reader.

use std::path::Path;

use ix_trace_rs::trace;
use serde_json::Value;

use crate::emit_checked_v2::{producer_bytes, read_back};

#[trace("TC-483", "FR-091-AC-74")]
#[test]
fn compound_quantity_type_metadata_survives_emission_and_strict_read() {
    let source = "language \"ix:native\" edition \"1-draft\";
        profile v = \"quire.value.complete/v1\";
        dimension Mass; dimension Length; dimension Time;
        unit kg : Mass = rational(1, 1);
        unit m : Length = rational(1, 1);
        unit s : Time = rational(1, 1);
        dimension Energy = Mass * Length^2 * Time^-2;
        unit J : Energy = rational(1, 1);
        function echo using v(q: Quantity<kg*m^2*s^-2>): Quantity<s^-2*kg*m*m> pure { q }
        function converted using v(q: Quantity<J>): Quantity<kg*m^2*s^-2> pure { convert<Quantity<kg*m^2*s^-2>>(q) }
        function single using v(q: Quantity<kg^1>): Quantity<kg*m^0> pure { q }
        function empty using v(q: Quantity<m*m^-1>): Quantity<m^0> pure { q }
        function legacy using v(q: Quantity<kg, Int[0, 10]>): Quantity<kg, Int[0, 10]> pure { q }";
    let bytes = producer_bytes(Path::new("compound-quantity-types.native"), source, &[]);
    read_back(&bytes, &[]);
    let wire: Value = serde_json::from_slice(&bytes).expect("producer JSON");
    let mut paths = Vec::new();
    let mut pending = vec![&wire];
    while let Some(value) = pending.pop() {
        match value {
            Value::Object(object) => {
                if let Some(path) = object.get("path").and_then(Value::as_array) {
                    if path.first().is_some_and(|part| part == "scalar_type")
                        && path.get(1).is_some_and(|part| part == "compound_unit")
                    {
                        paths.push(path.clone());
                    }
                }
                pending.extend(object.values());
            }
            Value::Array(values) => pending.extend(values),
            _ => {}
        }
    }
    assert!(
        paths.iter().any(|path| path.len() == 8 && {
            let mut exponents = [
                path[3].as_str().unwrap(),
                path[5].as_str().unwrap(),
                path[7].as_str().unwrap(),
            ];
            exponents.sort();
            exponents == ["-2", "1", "2"]
        }),
        "three exact canonical root exponents: {paths:?}"
    );
    assert!(
        paths.iter().any(|path| path.len() == 4 && path[3] == "1"),
        "one-term compound: {paths:?}"
    );
    assert!(
        paths.iter().any(|path| path.len() == 2),
        "dimensionless compound: {paths:?}"
    );
}
