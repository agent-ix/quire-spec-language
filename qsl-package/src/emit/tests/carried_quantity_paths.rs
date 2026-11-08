// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092-AC-27 (TC-413): a `Quantity<u, C>` type node's structural path is
//! QSpec FR-322's `[scalar_type, carried_quantity, U, C, "rounding", T,
//! mode]`, its decimal carrier's path keeps the repeated text of equal
//! bounds, and IR's checked-package/v2 reader admits the package and refuses
//! a truncated carried path.

use super::converted_equality::emitted;
use super::*;

const UNITS: &str = "dimension Length;\n\
                     unit m : Length = rational(1, 1);\n";

const DECIMAL: &str = "Quantity<m, Decimal[-100, 100; 2, 2; exact]>";
const INTEGER: &str = "Quantity<m, Int[-2, 2]>";
const ROUNDED: &str = "Quantity<m, Int[-2, 2; nearest-even]>";

fn emission() -> Emission {
    emitted(&format!(
        "{UNITS}\
         function dec using v(a: {DECIMAL}): {DECIMAL} pure {{ a }}\n\
         function int using v(a: {INTEGER}): {INTEGER} pure {{ a }}\n\
         function rnd using v(a: {ROUNDED}): {ROUNDED} pure {{ a }}"
    ))
}

/// The wire node at `segment`, an index segment of a path.
fn operand<'w>(nodes: &'w [Value], segment: &Value) -> &'w Value {
    let index = segment["node"].as_u64().expect("an index segment") as usize;
    &nodes[index]
}

/// The path of `node` with each index segment replaced by the form of the node
/// it names.
fn path_forms(nodes: &[Value], node: &Value) -> Vec<Value> {
    node["node_id"]["path"]
        .as_array()
        .expect("a derived path")
        .iter()
        .map(|segment| match segment.get("node") {
            Some(_) => json!({ "node": operand(nodes, segment)["semantic_form"] }),
            None => segment.clone(),
        })
        .collect()
}

fn forms<'w>(nodes: &'w [Value], form: &str) -> Vec<&'w Value> {
    nodes
        .iter()
        .filter(|node| node["semantic_form"] == form)
        .collect()
}

#[trace("FR-092-AC-27", "TC-413")]
#[test]
fn a_carried_quantity_path_holds_unit_carrier_rounding_text_and_mode() {
    let emission = emission();
    let wire = wire(&emission);
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let carried = forms(nodes, "carried_quantity");
    assert_eq!(carried.len(), 3, "three carried forms, three nodes");
    let mut shapes = Vec::new();
    for node in carried {
        let path = path_forms(nodes, node);
        assert_eq!(path.len(), 7, "{path:?}");
        assert_eq!(path[0], "scalar_type");
        assert_eq!(path[1], "carried_quantity");
        assert_eq!(path[2], json!({ "node": "unit" }));
        assert_eq!(path[4], "rounding");
        assert_eq!(path[5], json!({ "node": "text" }));
        let raw = node["node_id"]["path"].as_array().unwrap();
        // The carrier and text operands are the body's own references.
        let members = node["body"]["members"].as_array().unwrap();
        assert_eq!(
            operand(nodes, &raw[3])["node_id"],
            nodes
                .iter()
                .find(|candidate| candidate["node_id"] == members[1]["value"]["target"])
                .unwrap()["node_id"]
        );
        assert_eq!(path[6], members[2]["value"]["value"], "the mode text");
        shapes.push((path[3].clone(), path[6].clone()));
    }
    shapes.sort_by_key(|(carrier, mode)| (carrier.to_string(), mode.to_string()));
    assert_eq!(
        shapes,
        [
            (json!({ "node": "decimal_range" }), json!("exact")),
            (json!({ "node": "integer_range" }), json!("exact")),
            (json!({ "node": "integer_range" }), json!("nearest-even")),
        ]
    );
    let Read::Verified { .. } = read_back(&emission) else {
        panic!("the strict reader admits the carried quantities");
    };
}

#[trace("FR-092-AC-27", "TC-413")]
#[test]
fn a_decimal_carrier_path_repeats_equal_bound_text_as_the_qspec_vector_does() {
    let emission = emission();
    let wire = wire(&emission);
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let decimal = forms(nodes, "decimal_range");
    assert_eq!(decimal.len(), 1);
    assert_eq!(
        path_forms(nodes, decimal[0]),
        [
            json!("bounded_domain"),
            json!("decimal_range"),
            json!("coefficient_min"),
            json!({ "node": "integer" }),
            json!("-100"),
            json!("coefficient_max"),
            json!("100"),
            json!("scale_min"),
            json!("2"),
            json!("scale_max"),
            json!("2"),
            json!("rounding"),
            json!({ "node": "text" }),
            json!("exact"),
            json!({ "node": "decimal" }),
        ],
        "QSpec's dec_cents path"
    );
}

#[trace("FR-092-AC-27", "TC-413")]
#[test]
fn a_truncated_carried_path_is_refused_by_the_strict_reader() {
    let emission = emission();
    let mut wire = wire(&emission);
    let carried = wire["semantic_graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["semantic_form"] == "carried_quantity")
        .expect("a carried node");
    carried["node_id"]["path"]
        .as_array_mut()
        .unwrap()
        .truncate(4);
    let read = read_v2(
        &jcs(&wire),
        library(),
        V2ReadLimits::fixture(),
        &read_evidence(&emission),
    );
    assert!(matches!(read, Read::Refused(_)), "{read:?}");
    assert!(
        format!("{read:?}").contains("InvalidDerivedShape"),
        "refused as a derived shape fault: {read:?}"
    );
}
