// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-451's body order, key and targets.

use super::super::{node_key, BodyTerm, NodeInput, NodeTag};
use super::*;

fn key(byte: u8) -> NodeKey {
    NodeKey::from_digest([byte; 32])
}

fn path(segments: &[&str]) -> Vec<String> {
    segments
        .iter()
        .map(|segment| (*segment).to_owned())
        .collect()
}

fn object(ty: u8, rust_field: &str) -> ObjectEntry {
    ObjectEntry::new(
        key(ty),
        path(&["store", "Thing"]),
        vec![
            ("note".to_owned(), "note".to_owned()),
            ("amount".to_owned(), rust_field.to_owned()),
        ],
    )
}

fn frame(context: u8, operation: &str) -> FrameEntry {
    FrameEntry::new(
        key(context),
        operation.to_owned(),
        path(&["update"]),
        "self".to_owned(),
        vec![
            ("n".to_owned(), "count".to_owned()),
            ("m".to_owned(), "limit".to_owned()),
        ],
    )
}

fn term(rust_field: &str) -> AbstractionTerm {
    AbstractionTerm::new(
        vec![object(2, rust_field), object(1, "amount")],
        vec![
            PopulationEntry::new(key(5), path(&["state", "things"])),
            PopulationEntry::new(key(4), path(&["state", "others"])),
        ],
        vec![frame(2, "b"), frame(2, "a"), frame(1, "z")],
    )
}

fn digest(key: u8) -> String {
    format!("{key:02x}").repeat(32)
}

/// QSpec FR-451 "Canonical order": `objects`, `populations` and `frames`
/// ascend by digest and then operation, `fields` and `parameters` by
/// name, whatever order the entries were given in.
#[test]
fn the_arrays_are_in_canonical_order_whatever_order_they_were_built_in() {
    let body = serde_json::to_value(term("amount")).expect("the body serializes");
    let digests = |array: &str, member: &str| -> Vec<String> {
        body[array]
            .as_array()
            .expect("an array")
            .iter()
            .map(|entry| {
                entry[member]["digest"]
                    .as_str()
                    .expect("a digest")
                    .to_owned()
            })
            .collect()
    };
    assert_eq!(digests("objects", "type"), [digest(1), digest(2)]);
    assert_eq!(digests("populations", "population"), [digest(4), digest(5)]);
    assert_eq!(
        digests("frames", "context"),
        [digest(1), digest(2), digest(2)]
    );
    let operations: Vec<&str> = body["frames"]
        .as_array()
        .expect("an array")
        .iter()
        .map(|entry| entry["operation"].as_str().expect("a name"))
        .collect();
    assert_eq!(operations, ["z", "a", "b"]);
    let fields: Vec<&str> = body["objects"][0]["fields"]
        .as_array()
        .expect("an array")
        .iter()
        .map(|entry| entry["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(fields, ["amount", "note"]);
    let parameters: Vec<&str> = body["frames"][0]["parameters"]
        .as_array()
        .expect("an array")
        .iter()
        .map(|entry| entry["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(parameters, ["m", "n"]);
}

/// QSpec FR-451 "node_id": every binding is part of the key.
#[test]
fn the_key_covers_every_binding() {
    let limits = IdentityLimits::default();
    let first = abstraction_node_key(&term("amount"), limits).expect("the body is keyed");
    let again = abstraction_node_key(&term("amount"), limits).expect("the body is keyed");
    let rebound = abstraction_node_key(&term("sum"), limits).expect("the body is keyed");
    assert_eq!(first, again);
    assert_ne!(first.key, rebound.key);
}

/// QSpec FR-451 "dependencies": the unique set of every target.
#[test]
fn the_targets_are_the_unique_set_of_every_type_population_and_context() {
    let targets: Vec<NodeKey> = term("amount").targets().into_iter().collect();
    assert_eq!(targets, [key(1), key(2), key(4), key(5)]);
}

/// FR-092's preimages key no abstraction body: FR-451 keys it by its own.
#[test]
fn the_generic_node_key_refuses_an_abstraction_body() {
    let body = BodyTerm::Abstraction(term("amount"));
    let input = NodeInput {
        owner: None,
        node_tag: NodeTag::Correspondence,
        semantic_form: "abstraction_relation",
        semantic_type: None,
        declaration: None,
        body: &body,
    };
    assert_eq!(
        node_key(&input, IdentityLimits::default()),
        Err(NodeKeyRefusal::AbstractionBody)
    );
}
