// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-320 (TC-827): the union type node, the construction node and the
//! `case` node, lowered from source, emitted as `quire.checked-package/v2`
//! and read back through QSL's whole I2 read. The package ids compared here
//! are the ones the emitter declares, so FR-319-AC-1's "same `package_id`"
//! is read from the emitted bytes, never recomputed.

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

/// QSpec FR-441's member key digest, from the preimage text: the SHA-256 of
/// the RFC 8785 bytes of `{declaration_node_id, member, version}`.
fn member_key(union: &str, member: &str) -> String {
    let preimage = format!(
        "{{\"declaration_node_id\":{{\"digest\":\"{union}\",\
         \"domain\":\"quire.checked-semantic-node/v1\"}},\
         \"member\":\"{member}\",\"version\":\"quire.union-member-node/v1\"}}"
    );
    Sha256::digest(preimage.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
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
    let union_id = union["node_id"]["digest"].as_str().expect("a digest");
    assert_eq!(exports["Shape"], union_id);

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
        Some(union_id.to_owned())
    );
    let CompositeShape::Union(members) = declaration.shape() else {
        panic!("Shape is a union")
    };
    for member in members {
        let variant = member.member().expect("sealed").variant();
        assert_eq!(
            variant.to_string(),
            member_key(union_id, member.identifier().as_str()),
            "{}",
            member.identifier().as_str()
        );
    }

    // The digest-addressed source recompiles to the emitted package.
    let (_, recompiled) = emit_from_text(&format!("{SHAPE}{FUNCTIONS}"));
    assert_eq!(
        recompiled.package.package_id(),
        emission.package.package_id()
    );
}

#[trace("FR-319-AC-1", "TC-826")]
#[test]
fn the_package_id_is_stable_under_trivia_and_arm_order_and_moves_with_the_union() {
    let base = emitted(&format!("{SHAPE}{FUNCTIONS}")).package.package_id();
    let changed = "// shapes\n\
         union   Shape {\n    Empty,\n    Circle( Integer ),   // round\n    \
         Rect( Integer,  Integer ),\n}\n\n\
         function area using v(s: Shape): Integer pure {\n    case s {\n        \
         Rect(w, h): w * h;   // two sides\n        Empty: 0;\n        \
         Circle(r): r;\n    }\n}\n\n\
         function rect using v(): Shape pure { Shape::Rect(2,   3) }\n";
    assert_eq!(emitted(changed).package.package_id(), base);

    let rect = "function rect using v(): Shape pure { Shape::Rect(2, 3) }\n";
    for (label, declarations) in [
        (
            "renamed member",
            format!(
                "union Shape {{ Nothing, Circle(Integer), Rect(Integer, Integer) }}\n\
                 function area using v(s: Shape): Integer pure \
                 {{ case s {{ Nothing: 0; Circle(r): r; Rect(w, h): w * h; }} }}\n{rect}"
            ),
        ),
        (
            "added member",
            format!(
                "union Shape {{ Empty, Circle(Integer), Rect(Integer, Integer), Line(Integer) }}\n\
                 function area using v(s: Shape): Integer pure \
                 {{ case s {{ Empty: 0; Circle(r): r; Rect(w, h): w * h; Line(n): n; }} }}\n{rect}"
            ),
        ),
        (
            "changed payload type",
            format!(
                "union Shape {{ Empty, Circle(Boolean), Rect(Integer, Integer) }}\n\
                 function area using v(s: Shape): Integer pure \
                 {{ case s {{ Empty: 0; Circle(r): 1; Rect(w, h): w * h; }} }}\n{rect}"
            ),
        ),
    ] {
        assert_ne!(emitted(&declarations).package.package_id(), base, "{label}");
    }
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
    let written: BTreeSet<&str> = nodes(&wire)
        .iter()
        .map(|node| node["node_id"]["digest"].as_str().expect("a digest"))
        .collect();
    for omission in &emission.omitted {
        assert!(!written.contains(&*omission.node.digest), "{omission:?}");
    }
    assert!(!written.is_empty());
    for node in nodes(&wire) {
        for dependency in node["dependencies"].as_array().expect("dependencies") {
            let target = dependency["digest"].as_str().expect("a digest");
            assert!(written.contains(target), "{node} names an omitted node");
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
        // The retained key is the application-preimage key, rebuilt here from
        // the wire node alone, and is not the structural-preimage digest.
        let retained = value["node_id"]["digest"].as_str().expect("a digest");
        assert_eq!(rebuilt_key(value, &[]), retained, "{member}");
        assert_ne!(rebuilt_structural_key(value, &[]), retained, "{member}");
    }
    // QSL's whole I2 read admits those keys.
    verified_exports(&emission);
}
