// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit tests for the FR-322 application-node key, and the opt-in
//! conformance check against QSpec's published `operation_vectors`.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::*;
use crate::value::node::NodeIdDocument;

// ---------------------------------------------------------------------
// Test-only wire decoding. Production never reads these types from wire
// bytes (ADR-013 R-10: a wire node id becomes a key only by lookup in a
// checked package); the conformance test below does, to rebuild QSpec's
// vector inputs.
// ---------------------------------------------------------------------

impl<'de> Deserialize<'de> for NodeRef {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        NodeIdDocument::deserialize(deserializer)?
            .key()
            .map(NodeRef)
            .ok_or_else(|| D::Error::custom("not a canonical checked-semantic-node reference"))
    }
}

impl<'de> Deserialize<'de> for LeafSegment {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        if text == "inner" {
            return Ok(Self::Inner);
        }
        if let Some(name) = text.strip_prefix("field:") {
            return Identifier::new(name)
                .map(Self::Field)
                .map_err(D::Error::custom);
        }
        text.strip_prefix("position:")
            .filter(|digits| *digits == "0" || !digits.starts_with('0'))
            .and_then(|digits| digits.parse().ok())
            .map(Self::Position)
            .ok_or_else(|| D::Error::custom(format!("not a leaf segment: {text}")))
    }
}

/// `Operation.member` from its wire form (`OperationMember` or `null`).
pub(super) fn member<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Member>, D::Error> {
    let Some(wire) = Option::<Value>::deserialize(deserializer)? else {
        return Ok(None);
    };
    let text = |field: &str| {
        wire[field]
            .as_str()
            .ok_or_else(|| D::Error::custom(format!("member `{field}` is not a string")))
    };
    let identifier =
        |field: &str| text(field).and_then(|name| Identifier::new(name).map_err(D::Error::custom));
    let declaration = || {
        NodeRef::deserialize(&wire["declaration"])
            .map(|node| node.0)
            .map_err(D::Error::custom)
    };
    let member = match text("kind")? {
        "field" => Member::Field {
            declaration: declaration()?,
            name: identifier("name")?,
        },
        "position" => Member::Position {
            declaration: declaration()?,
            position: wire["position"]
                .as_u64()
                .ok_or_else(|| D::Error::custom("member `position` is not an integer"))?,
        },
        "element" => Member::Element {
            declaration: declaration()?,
        },
        "relationship_end" => Member::RelationshipEnd {
            declaration: declaration()?,
            name: identifier("name")?,
        },
        "operation" => Member::Operation {
            declaration: declaration()?,
            name: identifier("name")?,
        },
        "type_argument" => Member::TypeArgument {
            declaration: declaration()?,
        },
        "profile_operator" => Member::ProfileOperator {
            operator: identifier("operator")?,
        },
        other => return Err(D::Error::custom(format!("unknown member kind {other}"))),
    };
    // The decoded member must re-encode to exactly the wire it came from, so
    // a field this decoder drops cannot pass silently.
    if member.to_wire() != wire {
        return Err(D::Error::custom(format!(
            "member {wire} does not round-trip"
        )));
    }
    Ok(Some(member))
}

// ---------------------------------------------------------------------
// QSL-authored cases: the parts QSpec's vectors leave uncovered (every
// published vector has `declaration: null` and `recursion: null`).
// ---------------------------------------------------------------------

fn key(fill: u8) -> NodeKey {
    NodeKey::from_digest([fill; 32])
}

fn reference(fill: u8) -> SemanticTerm {
    SemanticTerm::Reference {
        target: NodeRef(key(fill)),
    }
}

fn add(arguments: Vec<SemanticTerm>) -> SemanticTerm {
    SemanticTerm::Application {
        operator: Operator::Binary,
        operation: Operation {
            identity: "quire.op.integer.add".to_owned(),
            laws: Vec::new(),
            mode: None,
            member: None,
            leaves: Vec::new(),
        },
        result_type: NodeRef(key(3)),
        arguments,
    }
}

fn identifiers(names: &[&str]) -> Vec<Identifier> {
    names
        .iter()
        .map(|name| Identifier::new(*name).expect("test identifier is valid"))
        .collect()
}

fn node<'a>(body: &'a SemanticTerm) -> ApplicationNode<'a> {
    ApplicationNode {
        node_tag: "expression",
        semantic_form: "binary",
        semantic_type: key(3),
        declaration: None,
        recursion: None,
        body,
    }
}

fn preimage_json(node: &ApplicationNode<'_>) -> Value {
    let key = application_node_key(node).expect("node has an application");
    serde_json::from_slice(&key.preimage).expect("preimage is JSON")
}

#[test]
fn preimage_bytes_are_pinned() {
    let body = add(vec![reference(1), reference(9)]);
    let group = [key(1), key(2)];
    let declaration = identifiers(&["pkg", "total"]);
    let node = ApplicationNode {
        node_tag: "function",
        semantic_form: "function",
        semantic_type: key(3),
        declaration: Some(&declaration),
        recursion: RecursionGroup::new(&group, 1),
        body: &body,
    };
    let node_ref = |fill: u8| {
        format!(
            r#"{{"digest":"{}","domain":"quire.checked-semantic-node/v1"}}"#,
            key(fill)
        )
    };
    let expected = format!(
        concat!(
            r#"{{"body":{{"arguments":[{{"ordinal":0,"term":"group_reference"}},"#,
            r#"{{"target":{nine},"term":"reference"}}],"#,
            r#""operation":{{"identity":"quire.op.integer.add","laws":[],"leaves":[],"member":null,"mode":null}},"#,
            r#""operator":"binary","result_type":{three},"term":"application"}},"#,
            r#""declaration":{{"qualified_name":["pkg","total"]}},"#,
            r#""node_tag":"function","recursion":{{"ordinal":1,"size":2}},"#,
            r#""semantic_form":"function","semantic_type":{three},"#,
            r#""version":"quire.application-node/v1"}}"#,
        ),
        nine = node_ref(9),
        three = node_ref(3),
    );

    let computed = application_node_key(&node).expect("node has an application");

    assert_eq!(
        String::from_utf8(computed.preimage.clone()).expect("preimage is UTF-8"),
        expected
    );
    assert_eq!(
        computed.key,
        NodeKey::from_digest(Sha256::digest(expected.as_bytes()).into())
    );
}

#[test]
fn group_references_are_rewritten_in_every_nested_term() {
    let body = SemanticTerm::Aggregate {
        members: vec![
            SemanticTerm::Binding {
                name: "x".to_owned(),
                value: Box::new(reference(2)),
            },
            add(vec![reference(1), add(vec![reference(2), reference(9)])]),
        ],
    };
    let group = [key(1), key(2)];
    let node = ApplicationNode {
        recursion: RecursionGroup::new(&group, 0),
        ..node(&body)
    };

    let preimage = preimage_json(&node);

    let group_reference = |ordinal: usize| json!({"term": "group_reference", "ordinal": ordinal});
    let members = &preimage["body"]["members"];
    assert_eq!(members[0]["value"], group_reference(1));
    assert_eq!(members[1]["arguments"][0], group_reference(0));
    assert_eq!(
        members[1]["arguments"][1]["arguments"][0],
        group_reference(1)
    );
    assert_eq!(
        members[1]["arguments"][1]["arguments"][1],
        json!({"term": "reference", "target": {"domain": NODE_KEY_DOMAIN, "digest": key(9).to_string()}}),
        "a reference outside the group stays a reference"
    );
    assert_eq!(preimage["recursion"], json!({"size": 2, "ordinal": 0}));
}

#[test]
fn the_key_does_not_depend_on_group_member_keys() {
    let first_body = add(vec![reference(1), reference(9)]);
    let second_body = add(vec![reference(7), reference(9)]);
    let first_group = [key(1), key(2)];
    let second_group = [key(7), key(8)];
    let first = ApplicationNode {
        recursion: RecursionGroup::new(&first_group, 1),
        ..node(&first_body)
    };
    let second = ApplicationNode {
        recursion: RecursionGroup::new(&second_group, 1),
        ..node(&second_body)
    };
    let outside = ApplicationNode {
        recursion: None,
        ..node(&first_body)
    };

    let key_of = |node: &ApplicationNode<'_>| {
        application_node_key(node)
            .expect("node has an application")
            .key
    };
    assert_eq!(key_of(&first), key_of(&second));
    assert_ne!(key_of(&first), key_of(&outside));
}

#[test]
fn declaration_and_recursion_each_enter_the_key() {
    let body = add(vec![reference(1), reference(9)]);
    let declaration = identifiers(&["pkg", "total"]);
    let other_declaration = identifiers(&["pkg", "sum"]);
    let group = [key(1), key(2)];
    let bare = node(&body);
    let declared = ApplicationNode {
        declaration: Some(&declaration),
        ..bare
    };
    let renamed = ApplicationNode {
        declaration: Some(&other_declaration),
        ..bare
    };
    let first = ApplicationNode {
        recursion: RecursionGroup::new(&group, 0),
        ..bare
    };
    let second = ApplicationNode {
        recursion: RecursionGroup::new(&group, 1),
        ..bare
    };

    let keys = [bare, declared, renamed, first, second].map(|node| {
        application_node_key(&node)
            .expect("node has an application")
            .key
    });
    let distinct: std::collections::BTreeSet<_> = keys.iter().collect();
    assert_eq!(distinct.len(), keys.len(), "{keys:?}");
    assert_eq!(preimage_json(&bare)["declaration"], Value::Null);
    assert_eq!(preimage_json(&bare)["recursion"], Value::Null);
}

#[test]
fn a_body_without_an_application_is_refused() {
    let literal = SemanticTerm::Literal {
        ty: NodeRef(key(3)),
        value_kind: LiteralKind::Integer,
        value: Some(LiteralValue::Integer(1)),
    };
    let bare_reference = reference(1);
    let aggregate = SemanticTerm::Aggregate {
        members: vec![literal.clone(), reference(2)],
    };
    let nested = SemanticTerm::Binding {
        name: "x".to_owned(),
        value: Box::new(SemanticTerm::Aggregate {
            members: vec![literal, add(Vec::new())],
        }),
    };

    for body in [&bare_reference, &aggregate] {
        assert_eq!(
            application_node_key(&node(body)),
            Err(ApplicationKeyRefusal::NoApplication)
        );
    }
    assert!(application_node_key(&node(&nested)).is_ok());
}

#[test]
fn a_literal_integer_outside_the_exact_range_is_refused() {
    let literal = |value: i64| {
        add(vec![SemanticTerm::Literal {
            ty: NodeRef(key(3)),
            value_kind: LiteralKind::Integer,
            value: Some(LiteralValue::Integer(value)),
        }])
    };
    let edge = literal(JCS_SAFE_INTEGER);
    let negative_edge = literal(-JCS_SAFE_INTEGER);
    let beyond = literal(JCS_SAFE_INTEGER + 1);
    let negative_beyond = literal(-JCS_SAFE_INTEGER - 1);

    assert!(application_node_key(&node(&edge)).is_ok());
    assert!(application_node_key(&node(&negative_edge)).is_ok());
    assert_eq!(
        application_node_key(&node(&beyond)),
        Err(ApplicationKeyRefusal::UnsafeInteger {
            value: JCS_SAFE_INTEGER + 1
        })
    );
    assert_eq!(
        application_node_key(&node(&negative_beyond)),
        Err(ApplicationKeyRefusal::UnsafeInteger {
            value: -JCS_SAFE_INTEGER - 1
        })
    );
}

#[test]
fn a_recursion_ordinal_must_be_a_group_position() {
    let group = [key(1), key(2)];
    assert!(RecursionGroup::new(&group, 1).is_some());
    assert_eq!(RecursionGroup::new(&group, 2), None);
    assert_eq!(RecursionGroup::new(&[], 0), None);
}

// ---------------------------------------------------------------------
// Opt-in conformance against QSpec's published vectors.
// ---------------------------------------------------------------------

/// A published vector preimage, decoded into this module's input types.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VectorPreimage {
    version: String,
    node_tag: String,
    semantic_form: String,
    semantic_type: NodeRef,
    declaration: Option<VectorDeclaration>,
    recursion: Option<Value>,
    body: SemanticTerm,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VectorDeclaration {
    qualified_name: Vec<String>,
}

/// Every QSpec `operation_vectors` entry, read at run time from
/// `$QSPEC_DIR/proposals/checked-package-v2/node-identity-vectors.json`:
/// this module rebuilds each vector's node from its preimage, and its own
/// preimage bytes and key must equal QSpec's. Skipped (and passing) when
/// `QSPEC_DIR` is unset; `make conformance` requires it. Nothing of QSpec is
/// copied into this repository.
#[test]
fn conformance_fr322_application_keys_match_qspec_operation_vectors() {
    let Some(qspec) = std::env::var_os("QSPEC_DIR") else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let path = std::path::Path::new(&qspec)
        .join("proposals/checked-package-v2/node-identity-vectors.json");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    let document: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parsing {}: {error}", path.display()));
    let vectors = document["operation_vectors"]
        .as_array()
        .unwrap_or_else(|| panic!("{} has no operation_vectors array", path.display()));
    assert!(
        !vectors.is_empty(),
        "{} publishes no operation vectors",
        path.display()
    );

    for vector in vectors {
        let name = vector["name"].as_str().expect("vector name is a string");
        let published = &vector["preimage"];
        let decoded = VectorPreimage::deserialize(published)
            .unwrap_or_else(|error| panic!("{name}: preimage does not decode: {error}"));
        assert_eq!(decoded.version, APPLICATION_NODE_VERSION, "{name}: version");
        assert_eq!(
            decoded.recursion, None,
            "{name}: a recursion member cannot be rebuilt from a preimage whose group references are already ordinals"
        );
        let declaration = decoded.declaration.map(|declaration| {
            declaration
                .qualified_name
                .iter()
                .map(|segment| {
                    Identifier::new(segment.as_str())
                        .unwrap_or_else(|_| panic!("{name}: `{segment}` is not an identifier"))
                })
                .collect::<Vec<_>>()
        });
        let node = ApplicationNode {
            node_tag: &decoded.node_tag,
            semantic_form: &decoded.semantic_form,
            semantic_type: decoded.semantic_type.0,
            declaration: declaration.as_deref(),
            recursion: None,
            body: &decoded.body,
        };

        let computed = application_node_key(&node)
            .unwrap_or_else(|refusal| panic!("{name}: refused: {refusal}"));

        assert_eq!(
            computed.preimage,
            serde_json::to_vec(published).expect("a parsed preimage serializes"),
            "{name}: preimage bytes"
        );
        assert_eq!(
            Some(computed.key.to_string().as_str()),
            vector["sha256"].as_str(),
            "{name}: key"
        );
    }
    println!(
        "conformance: {} of {} QSpec operation vectors match ({})",
        vectors.len(),
        vectors.len(),
        path.display()
    );
}
