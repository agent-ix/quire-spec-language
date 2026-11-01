// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-320 (TC-827): the union type node, the construction node and the
//! `case` node, lowered from source, emitted as `quire.checked-package/v2`
//! and read back through QSL's whole I2 read.

use super::*;

const SHAPE: &str = "union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }\n";

/// FR-318-AC-1's `area` and a function returning `Shape::Rect(2, 3)`.
const FUNCTIONS: &str = "function area using v(s: Shape): Integer pure \
     { case s { Empty: 0; Circle(r): r; Rect(w, h): w * h; } }\n\
     function rect using v(): Shape pure { Shape::Rect(2, 3) }\n";

/// `declarations` emitted.
fn emitted(declarations: &str) -> Emission {
    emit_from_text(declarations).1
}

/// The nodes of `wire` with `form`.
fn of_form<'w>(wire: &'w Value, form: &str) -> Vec<&'w Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["semantic_form"] == form)
        .collect()
}

#[trace("FR-320-AC-1", "TC-827")]
#[test]
fn the_union_keeps_its_id_and_its_members_their_identities_through_emission_and_the_i2_read() {
    let (_, package) = package_from_text(&format!("{SHAPE}{FUNCTIONS}"));
    let emission = emit_checked(&package).expect("the package emits");
    assert_eq!(emission.omitted, [], "nothing is omitted");
    let wire = wire(&emission);

    // QSpec's spelling: one `composite_type`/`union`, one `union_value`, one
    // `expression`/`case`.
    let [union] = of_form(&wire, "union")[..] else {
        panic!("one union node")
    };
    assert_eq!(union["node_tag"], "composite_type");
    assert_eq!(union["declaration"], json!({"qualified_name": ["Shape"]}));
    assert_eq!(of_form(&wire, "union_value").len(), 1);
    let [case] = of_form(&wire, "case")[..] else {
        panic!("one case node")
    };
    assert_eq!(case["node_tag"], "expression");

    // The I2 read admits it, and the id it exports is the emitted id.
    let exports = verified_exports(&emission);
    let union_id = qsl_foundation::node_wire::node_key_from_json(&union["node_id"])
        .expect("a structural union key");
    assert_eq!(exports["Shape"], union_id.to_string());

    // Each member's identity, checked before emission, is the FR-441 key over
    // the id the I2 read returned.
    let graph = package.graph();
    let types = graph.scope().types();
    let declaration = types
        .composites()
        .find(|declaration| declaration.name() == "Shape")
        .expect("Shape is declared");
    assert_eq!(
        types
            .union_key(declaration.key())
            .map(|key| key.to_string()),
        Some(union_id.to_string())
    );
    let CompositeShape::Union(members) = declaration.shape() else {
        panic!("Shape is a union")
    };
    for member in members {
        let variant = member.member().expect("sealed").variant().clone();
        assert_eq!(
            variant,
            qsl_semantics::value::enumeration::mint_union_variant_id(
                union_id.clone(),
                member.identifier(),
            )
            .expect("a union variant"),
            "{}",
            member.identifier()
        );
    }

    // The same source recompiles to the same package bytes.
    let (_, recompiled) = emit_from_text(&format!("{SHAPE}{FUNCTIONS}"));
    assert_eq!(recompiled.package.bytes(), emission.package.bytes());
}

#[trace("FR-320-AC-2", "TC-827")]
#[test]
fn an_ir_that_cannot_decode_the_union_omits_it_and_every_node_naming_it() {
    let (text, package) = package_from_text(&format!("{SHAPE}{FUNCTIONS}"));
    let regions = |_: &Location| {
        Some(SourceRegion::new(package.graph().source().clone(), 0, text.len() as u64).unwrap())
    };
    let cannot_decode_union = |tag: CheckedNodeTag, form: &str| {
        !(tag == CheckedNodeTag::CompositeType && form == "union")
            && CheckedNodeKind::decode(tag, form).is_some()
    };
    let emission = emit_package_decoding(&package, regions, &cannot_decode_union)
        .expect("the remaining nodes emit");

    let graph = package.graph().semantic_graph();
    let id_of = |form: &str| -> CheckedNodeId {
        let nodes: Vec<_> = graph
            .nodes()
            .filter(|node| node.semantic_form() == form)
            .collect();
        let [node] = nodes[..] else {
            panic!("one {form} node")
        };
        node_id(node.key())
    };
    let union = id_of("union");
    let cause_of = |id: &CheckedNodeId| {
        emission
            .omitted
            .iter()
            .find(|omission| omission.node == *id)
            .map(|omission| &omission.cause)
    };
    assert!(matches!(
        cause_of(&union),
        Some(OmissionCause::UnsupportedForm {
            node_tag: NodeTag::CompositeType,
            semantic_form: "union"
        })
    ));
    // Everything that names the union, directly or through the nodes that
    // name it, is omitted for naming an omitted node: the construction, the
    // `case`, both functions and the parameter typed at the union.
    for form in ["union_value", "case", "parameter"] {
        let omitted = graph
            .nodes()
            .filter(|node| node.semantic_form() == form)
            .map(|node| node_id(node.key()))
            .filter(|id| cause_of(id).is_some())
            .count();
        assert!(omitted >= 1, "a {form} node names the union");
    }
    let functions: Vec<_> = graph
        .nodes()
        .filter(|node| node.semantic_form() == "pure_function")
        .map(|node| node_id(node.key()))
        .collect();
    assert_eq!(functions.len(), 2);
    for function in &functions {
        assert!(
            matches!(cause_of(function), Some(OmissionCause::NamesOmittedNode(_))),
            "{function:?}: {:?}",
            emission.omitted
        );
    }
    // The scalar `Integer` node names no union and is written.
    let integer = id_of("integer");
    assert_eq!(cause_of(&integer), None);

    // The wire holds none of the omitted nodes and none names one.
    let wire = wire(&emission);
    let graph = qsl_semantics::check::wire_id::WireGraph::of_package_value(&wire).unwrap();
    let written: BTreeSet<NodeKey> = graph.keys().iter().cloned().collect();
    for omission in &emission.omitted {
        let id = serde_json::to_value(&omission.node).expect("an omitted id encodes");
        let key =
            qsl_foundation::node_wire::node_key_from_json(&id).expect("an omitted structural key");
        assert!(!written.contains(&key), "{omission:?}");
    }
    assert!(!written.is_empty());
    for node in nodes(&wire) {
        for dependency in node["dependencies"].as_array().expect("dependencies") {
            let target = graph
                .key_of_value(dependency)
                .expect("a structural dependency");
            assert!(written.contains(&target), "{node} names an omitted node");
        }
    }
}

#[trace("FR-320-AC-4", "FR-319-AC-4", "TC-827")]
#[test]
fn a_construction_is_emitted_with_its_application_preimage_key() {
    let emission = emitted(&format!(
        "{SHAPE}function empty using v(): Shape pure {{ Shape::Empty }}\n\
         function rect using v(a: Integer, b: Integer): Shape pure {{ Shape::Rect(a, b) }}\n"
    ));
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let values = of_form(&wire, "union_value");
    assert_eq!(
        values.len(),
        2,
        "the nullary and the reference-payload vector"
    );
    let [union] = of_form(&wire, "union")[..] else {
        panic!("one union node")
    };
    for value in values {
        let member = value["body"]["members"][0]["name"]
            .as_str()
            .expect("a member name");
        let payload = &value["body"]["members"][0]["value"];
        assert_eq!(value["body"]["term"], "aggregate", "{member}");
        assert_eq!(value["body"]["members"].as_array().map(Vec::len), Some(1));
        assert_eq!(payload["term"], "aggregate", "{member}");
        let payload_terms = payload["members"].as_array().expect("payload terms");
        assert_eq!(payload_terms.len(), if member == "Empty" { 0 } else { 2 });
        assert!(
            payload_terms.iter().all(|term| term["term"] == "reference"),
            "{member}: payloads are reference terms"
        );
        assert_eq!(value["semantic_type"], union["node_id"], "{member}");
        // No `declaration`, no `owner`, no recursion group.
        assert!(value.get("declaration").is_none(), "{member}");
        assert!(value.get("owner").is_none(), "{member}");
        assert!(value.get("recursion_group").is_none(), "{member}");
    }
    // QSL's whole I2 read admits those keys.
    verified_exports(&emission);
}
