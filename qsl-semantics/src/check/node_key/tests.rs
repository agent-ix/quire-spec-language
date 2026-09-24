// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit tests for the FR-322 application-node key, and the opt-in
//! conformance check against QSpec's published `operation_vectors`.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use ix_trace_rs::trace;

use super::*;
use crate::value::semantic_node::NodeIdDocument;

// ---------------------------------------------------------------------
// Test-only wire decoding. Production never reads these types from wire
// bytes (ADR-013 R-10: a wire node id becomes a key only by lookup in a
// checked package); the conformance test below does, to rebuild QSpec's
// vector inputs.
// ---------------------------------------------------------------------

impl<'de> Deserialize<'de> for NodeRef {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        NodeIdDocument::deserialize(deserializer)?
            .wire_id()
            .map(|id| NodeRef(NodeKey::from_digest(*id.as_bytes())))
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

/// The mode value whose `as_str` spells `text`, among `candidates`.
fn spelled<E: serde::de::Error, T: Copy>(
    value: Value,
    candidates: &[T],
    spelling: fn(T) -> &'static str,
) -> Result<T, E> {
    let Value::String(text) = value else {
        return Err(E::custom(format!("mode value {value} is not a string")));
    };
    candidates
        .iter()
        .copied()
        .find(|candidate| spelling(*candidate) == text)
        .ok_or_else(|| E::custom(format!("unknown mode value {text}")))
}

impl<'de> Deserialize<'de> for OperationMode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            kind: String,
            value: Value,
        }
        let wire = Wire::deserialize(deserializer)?;
        let value = wire.value;
        match wire.kind.as_str() {
            "rounding" => spelled::<D::Error, _>(value, &RoundingMode::ALL, RoundingMode::as_str)
                .map(Self::Rounding),
            "text_profile" => spelled::<D::Error, _>(value, &TextProfile::ALL, TextProfile::as_str)
                .map(Self::TextProfile),
            "absence" => spelled::<D::Error, _>(
                value,
                &[
                    AbsenceMode::Undefined,
                    AbsenceMode::Empty,
                    AbsenceMode::Refused,
                ],
                AbsenceMode::as_str,
            )
            .map(Self::Absence),
            other => Err(D::Error::custom(format!("unknown mode kind {other}"))),
        }
    }
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
        node_tag: NodeTag::Expression,
        semantic_form: "binary",
        semantic_type: key(3),
        declaration: None,
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
    let declaration = identifiers(&["pkg", "total"]);
    let node = ApplicationNode {
        node_tag: NodeTag::Function,
        semantic_form: "function",
        semantic_type: key(3),
        declaration: Some(&declaration),
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
            r#"{{"body":{{"arguments":[{{"target":{one},"term":"reference"}},"#,
            r#"{{"target":{nine},"term":"reference"}}],"#,
            r#""operation":{{"identity":"quire.op.integer.add","laws":[],"leaves":[],"member":null,"mode":null}},"#,
            r#""operator":"binary","result_type":{three},"term":"application"}},"#,
            r#""declaration":{{"qualified_name":["pkg","total"]}},"#,
            r#""node_tag":"function","recursion":null,"#,
            r#""semantic_form":"function","semantic_type":{three},"#,
            r#""version":"quire.application-node/v1"}}"#,
        ),
        one = node_ref(1),
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

/// [`group_keys`] with unbounded work.
fn keys_of(members: &[NodeInput<'_>], handles: &[NodeKey]) -> Result<GroupKeys, NodeKeyRefusal> {
    group_keys(members, handles, &mut |_| Ok(()))
}

/// The input of an application node inside a group.
fn in_group<'a>(body: &'a SemanticTerm) -> NodeInput<'a> {
    NodeInput {
        owner: None,
        node_tag: NodeTag::Expression,
        semantic_form: "binary",
        semantic_type: Some(key(3)),
        declaration: None,
        body,
    }
}

#[test]
fn group_references_are_rewritten_in_every_nested_term() {
    let first = SemanticTerm::Aggregate {
        members: vec![
            SemanticTerm::Binding {
                name: "x".to_owned(),
                value: Box::new(reference(2)),
            },
            add(vec![reference(1), add(vec![reference(2), reference(9)])]),
        ],
    };
    let second = add(vec![reference(1)]);
    let group =
        keys_of(&[in_group(&first), in_group(&second)], &[key(1), key(2)]).expect("the group keys");

    let preimage: Value = serde_json::from_slice(&group.members[0].preimage).expect("JSON");

    let group_reference = |ordinal: usize| json!({"term": "group_reference", "ordinal": ordinal});
    let (own, other) = (group.ordinals[0], group.ordinals[1]);
    let members = &preimage["body"]["members"];
    assert_eq!(members[0]["value"], group_reference(other));
    assert_eq!(members[1]["arguments"][0], group_reference(own));
    assert_eq!(
        members[1]["arguments"][1]["arguments"][0],
        group_reference(other)
    );
    assert_eq!(
        members[1]["arguments"][1]["arguments"][1],
        json!({"term": "reference", "target": {"domain": NODE_KEY_DOMAIN, "digest": key(9).to_string()}}),
        "a reference outside the group stays a reference"
    );
    assert_eq!(
        preimage["recursion"],
        json!({"size": 2, "ordinal": own}),
        "an application node's recursion has no group member"
    );
}

#[test]
fn the_key_does_not_depend_on_group_member_handles() {
    let first_bodies = [
        add(vec![reference(2), reference(9)]),
        add(vec![reference(1), reference(1)]),
    ];
    let second_bodies = [
        add(vec![reference(8), reference(9)]),
        add(vec![reference(7), reference(7)]),
    ];
    let first = keys_of(
        &[in_group(&first_bodies[0]), in_group(&first_bodies[1])],
        &[key(1), key(2)],
    )
    .expect("the group keys");
    let second = keys_of(
        &[in_group(&second_bodies[0]), in_group(&second_bodies[1])],
        &[key(7), key(8)],
    )
    .expect("the group keys");
    let outside = node_key(&in_group(&first_bodies[0])).expect("the node keys");

    assert_eq!(first, second);
    assert_ne!(first.members[0].key, outside.key);
}

#[test]
fn declaration_and_recursion_each_enter_the_key() {
    let body = add(vec![reference(1), reference(9)]);
    let declaration = identifiers(&["pkg", "total"]);
    let other_declaration = identifiers(&["pkg", "sum"]);
    let bare = node(&body);
    let declared = ApplicationNode {
        declaration: Some(&declaration),
        ..bare
    };
    let renamed = ApplicationNode {
        declaration: Some(&other_declaration),
        ..bare
    };

    let mut keys: Vec<NodeKey> = [bare, declared, renamed]
        .map(|node| {
            application_node_key(&node)
                .expect("node has an application")
                .key
        })
        .into();
    keys.push(
        keys_of(&[in_group(&body)], &[key(1)])
            .expect("a one-member group keys")
            .members[0]
            .key,
    );
    let distinct: std::collections::BTreeSet<_> = keys.iter().collect();
    assert_eq!(distinct.len(), keys.len(), "{keys:?}");
    assert_eq!(preimage_json(&bare)["declaration"], Value::Null);
    assert_eq!(preimage_json(&bare)["recursion"], Value::Null);
}

#[test]
fn a_body_without_an_application_is_refused() {
    let literal = SemanticTerm::literal(key(3), LiteralValue::Integer(Integer::from(1_i64)));
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
            Err(NodeKeyRefusal::NoApplication)
        );
    }
    assert!(application_node_key(&node(&nested)).is_ok());
}

/// FR-092: an integer literal is its canonical decimal string at any
/// magnitude, never a JSON number, so no integer is outside the preimage's
/// range; a rational is `"n/d"`, reduced.
#[trace("FR-092-AC-4", "TC-414")]
#[test]
fn integer_and_rational_literals_are_spelled_as_strings() {
    let huge: Integer = "123456789012345678901234567890"
        .parse()
        .expect("canonical integer");
    let body = add(vec![
        SemanticTerm::literal(key(4), LiteralValue::Integer(huge)),
        SemanticTerm::literal(key(4), LiteralValue::Integer(Integer::from(-7_i64))),
        SemanticTerm::literal(
            key(5),
            LiteralValue::Rational(
                Rational::new(Integer::from(2_i64), Integer::from(-4_i64)).expect("nonzero"),
            ),
        ),
    ]);
    let preimage = preimage_json(&node(&body));
    let arguments = &preimage["body"]["arguments"];
    assert_eq!(
        arguments[0]["value"],
        json!("123456789012345678901234567890")
    );
    assert_eq!(arguments[1]["value"], json!("-7"));
    assert_eq!(arguments[2]["value"], json!("-1/2"));
    assert_eq!(arguments[2]["value_kind"], json!("rational"));
}

#[test]
fn a_member_position_outside_the_exact_range_is_refused() {
    let safe = JCS_SAFE_INTEGER;
    let positioned = |position: u64| {
        let SemanticTerm::Application {
            operator,
            mut operation,
            result_type,
            arguments,
        } = add(Vec::new())
        else {
            unreachable!("add builds an application")
        };
        operation.member = Some(Member::Position {
            declaration: key(12),
            position,
        });
        SemanticTerm::Application {
            operator,
            operation,
            result_type,
            arguments,
        }
    };
    let edge = positioned(safe);
    let beyond = positioned(safe + 1);

    assert!(application_node_key(&node(&edge)).is_ok());
    assert_eq!(
        application_node_key(&node(&beyond)),
        Err(NodeKeyRefusal::UnsafeInteger {
            site: IntegerSite::MemberPosition,
            value: JCS_SAFE_INTEGER + 1
        })
    );
}

#[test]
fn a_recursion_group_needs_members_with_distinct_handles() {
    let body = add(vec![reference(1)]);
    assert_eq!(keys_of(&[], &[]), Err(NodeKeyRefusal::InvalidGroup));
    assert_eq!(
        keys_of(&[in_group(&body), in_group(&body)], &[key(1), key(1)]),
        Err(NodeKeyRefusal::InvalidGroup)
    );
    assert_eq!(
        keys_of(&[in_group(&body)], &[key(1), key(2)]),
        Err(NodeKeyRefusal::InvalidGroup)
    );
}

/// FR-092 G1: a one-member `option` group over itself, keyed through the key
/// function directly, to the vector's preimage bytes, key, signatures and
/// group digest.
#[trace("FR-092-AC-11", "TC-413")]
#[test]
fn a_one_member_group_keys_to_g1() {
    let handle = key(0x5a);
    let body = SemanticTerm::Aggregate {
        members: vec![SemanticTerm::reference(handle)],
    };
    let node = NodeInput {
        owner: None,
        node_tag: NodeTag::CompositeType,
        semantic_form: "option",
        semantic_type: None,
        declaration: None,
        body: &body,
    };

    let group = keys_of(&[node], &[handle]).expect("G1 keys");

    let expected = r#"{"body":{"members":[{"ordinal":0,"term":"group_reference"}],"term":"aggregate"},"declaration":null,"node_tag":"composite_type","recursion":{"group":"4a005f58e201e284473264dd016bbcc0a1cfcd8a26031428f8dac6a969e9b14b","ordinal":0,"size":1},"semantic_form":"option","semantic_type":null,"version":"quire.structural-node/v1"}"#;
    assert_eq!(
        String::from_utf8(group.members[0].preimage.clone()).expect("UTF-8"),
        expected
    );
    assert_eq!(
        group.members[0].key.to_string(),
        "7b2e6632de9e716f1f7b6a3155ea4e6a36129ea1e4473f539d52a75001ae5e31"
    );
    assert_eq!(
        hex(&group.digest),
        "4a005f58e201e284473264dd016bbcc0a1cfcd8a26031428f8dac6a969e9b14b"
    );
    let signature = "3ae296ebb5c73914192b56dcb1ed43464dc277339747b6840db238a5d65f51e4";
    assert_eq!(group.anonymous, [signature]);
    assert_eq!(group.full, [signature]);
    assert_eq!((group.ordinals.as_slice(), group.size), (&[0][..], 1));
}

#[test]
fn empty_names_and_forms_are_refused() {
    let body = add(vec![reference(1)]);
    let empty_name: [Identifier; 0] = [];
    let empty_binding = SemanticTerm::Binding {
        name: String::new(),
        value: Box::new(add(Vec::new())),
    };

    assert_eq!(
        application_node_key(&ApplicationNode {
            declaration: Some(&empty_name),
            ..node(&body)
        }),
        Err(NodeKeyRefusal::EmptyQualifiedName)
    );
    assert_eq!(
        application_node_key(&ApplicationNode {
            semantic_form: "",
            ..node(&body)
        }),
        Err(NodeKeyRefusal::EmptySemanticForm)
    );
    assert_eq!(
        application_node_key(&node(&empty_binding)),
        Err(NodeKeyRefusal::EmptyBindingName)
    );
}

/// A body `depth` terms deep: `depth - 1` bindings around one application.
fn nested(depth: u64) -> SemanticTerm {
    (1..depth).fold(add(Vec::new()), |inner, _| SemanticTerm::Binding {
        name: "x".to_owned(),
        value: Box::new(inner),
    })
}

#[test]
fn a_body_deeper_than_the_checking_limit_is_refused() {
    let at_limit = nested(MAX_CHECKING_DEPTH);
    let beyond = nested(MAX_CHECKING_DEPTH + 1);
    let nested_arguments = (1..=MAX_CHECKING_DEPTH).fold(reference(1), |inner, _| add(vec![inner]));

    assert!(application_node_key(&node(&at_limit)).is_ok());
    assert_eq!(
        application_node_key(&node(&beyond)),
        Err(NodeKeyRefusal::TooDeep {
            limit: MAX_CHECKING_DEPTH
        })
    );
    assert_eq!(
        application_node_key(&node(&nested_arguments)),
        Err(NodeKeyRefusal::TooDeep {
            limit: MAX_CHECKING_DEPTH
        }),
        "application arguments count toward depth"
    );
}

/// M1: the operation encoding (law, all three mode kinds, a `position`
/// member, `position:N`/`inner`/`field:` leaf segments) and literal terms
/// (integer, text with an escaped quote, null), pinned against
/// hand-written bytes with fill-byte keys. None of this comes from QSpec.
#[test]
fn operation_and_literal_bytes_are_pinned() {
    let law = OperationLaw {
        role: LawRole::TextProfile,
        definition: DefinitionReference {
            authority: "agent-ix".to_owned(),
            identity: "test.law/v1".to_owned(),
            revision: crate::value::definition::DefinitionRevision {
                namespace: "test".to_owned(),
                value: "1".to_owned(),
            },
            digest_domain: "quire.definition.bytes/v1".to_owned(),
            digest: key(11).to_string(),
        },
    };
    let literal = |fill: u8, value: LiteralValue| SemanticTerm::literal(key(fill), value);
    let body = SemanticTerm::Application {
        operator: Operator::Binary,
        operation: Operation {
            identity: "quire.op.decimal.div".to_owned(),
            laws: vec![law],
            mode: Some(OperationMode::Rounding(RoundingMode::NearestEven)),
            member: Some(Member::Position {
                declaration: key(12),
                position: 2,
            }),
            leaves: vec![
                OperationLeaf {
                    path: vec![LeafSegment::Position(1), LeafSegment::Inner],
                    laws: Vec::new(),
                    mode: Some(OperationMode::TextProfile(TextProfile::Nfc)),
                },
                OperationLeaf {
                    path: vec![LeafSegment::Field(
                        Identifier::new("name").expect("test identifier is valid"),
                    )],
                    laws: Vec::new(),
                    mode: Some(OperationMode::Absence(AbsenceMode::Empty)),
                },
            ],
        },
        result_type: NodeRef(key(3)),
        arguments: vec![
            literal(4, LiteralValue::Integer(Integer::from(42_i64))),
            literal(5, LiteralValue::Text("a\"b".to_owned())),
            literal(6, LiteralValue::None),
        ],
    };
    let node_ref = |fill: u8| {
        format!(
            r#"{{"digest":"{}","domain":"quire.checked-semantic-node/v1"}}"#,
            key(fill)
        )
    };
    let expected = format!(
        concat!(
            r#"{{"body":{{"arguments":["#,
            r#"{{"term":"literal","type":{four},"value":"42","value_kind":"integer"}},"#,
            r#"{{"term":"literal","type":{five},"value":"a\"b","value_kind":"text"}},"#,
            r#"{{"term":"literal","type":{six},"value":null,"value_kind":"none"}}],"#,
            r#""operation":{{"identity":"quire.op.decimal.div","#,
            r#""laws":[{{"definition":{{"authority":"agent-ix","digest":"{law_digest}","#,
            r#""digest_domain":"quire.definition.bytes/v1","identity":"test.law/v1","#,
            r#""revision":{{"namespace":"test","value":"1"}}}},"role":"text_profile"}}],"#,
            r#""leaves":[{{"laws":[],"mode":{{"kind":"text_profile","value":"nfc"}},"path":["position:1","inner"]}},"#,
            r#"{{"laws":[],"mode":{{"kind":"absence","value":"empty"}},"path":["field:name"]}}],"#,
            r#""member":{{"declaration":{twelve},"kind":"position","position":2}},"#,
            r#""mode":{{"kind":"rounding","value":"nearest-even"}}}},"#,
            r#""operator":"binary","result_type":{three},"term":"application"}},"#,
            r#""declaration":null,"node_tag":"expression","recursion":null,"#,
            r#""semantic_form":"binary","semantic_type":{three},"#,
            r#""version":"quire.application-node/v1"}}"#,
        ),
        four = node_ref(4),
        five = node_ref(5),
        six = node_ref(6),
        twelve = node_ref(12),
        three = node_ref(3),
        law_digest = key(11),
    );

    let computed = application_node_key(&node(&body)).expect("node has an application");

    assert_eq!(
        String::from_utf8(computed.preimage.clone()).expect("preimage is UTF-8"),
        expected
    );
    assert_eq!(
        computed.key,
        NodeKey::from_digest(Sha256::digest(expected.as_bytes()).into())
    );
}

// ---------------------------------------------------------------------
// Opt-in conformance against QSpec's published vectors.
// ---------------------------------------------------------------------

/// A published vector preimage, decoded into this module's input types.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VectorPreimage {
    version: String,
    node_tag: NodeTag,
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
            node_tag: decoded.node_tag,
            semantic_form: &decoded.semantic_form,
            semantic_type: decoded.semantic_type.0,
            declaration: declaration.as_deref(),
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

// ---------------------------------------------------------------------
// FR-092 `quire.structural-node/v1`.
// ---------------------------------------------------------------------

fn owner() -> Owner {
    Owner::Source(SourceOwner::new("a", "u").expect("nonempty owner"))
}

fn empty_aggregate() -> SemanticTerm {
    SemanticTerm::Aggregate {
        members: Vec::new(),
    }
}

fn structural<'a>(body: &'a SemanticTerm) -> NodeInput<'a> {
    NodeInput {
        owner: None,
        node_tag: NodeTag::ScalarType,
        semantic_form: "boolean",
        semantic_type: None,
        declaration: None,
        body,
    }
}

/// FR-092 T1, byte for byte: a body with no application is keyed by
/// `quire.structural-node/v1`, with a `null` semantic type for a self-typed
/// node and no `owner` member for an undeclared one.
#[trace("FR-092-AC-1", "TC-413")]
#[test]
fn a_structural_preimage_is_pinned() {
    let body = empty_aggregate();
    let keyed = node_key(&structural(&body)).expect("a builtin scalar keys");
    let expected = r#"{"body":{"members":[],"term":"aggregate"},"declaration":null,"node_tag":"scalar_type","recursion":null,"semantic_form":"boolean","semantic_type":null,"version":"quire.structural-node/v1"}"#;
    assert_eq!(String::from_utf8(keyed.preimage).expect("UTF-8"), expected);
    assert_eq!(
        keyed.key.to_string(),
        "9964390677844ad66b781babdbfa95933bc2b16ef1e86f67005966b77e6db3aa"
    );
}

/// FR-092: `owner` is present exactly when `declaration` is, and enters the
/// key; an application body never carries an owner and always a type.
#[trace("FR-092-AC-2", "TC-413")]
#[test]
fn an_owner_enters_a_declared_structural_key_only() {
    let body = empty_aggregate();
    let name = identifiers(&["Point"]);
    let u = owner();
    let w = Owner::Source(SourceOwner::new("a", "w").expect("nonempty owner"));
    let declared = |owner: &Owner| {
        node_key(&NodeInput {
            owner: Some(owner),
            node_tag: NodeTag::CompositeType,
            semantic_form: "record",
            declaration: Some(&name),
            ..structural(&body)
        })
        .expect("a declared record keys")
    };
    let under_u = declared(&u);
    let preimage: Value = serde_json::from_slice(&under_u.preimage).expect("JSON");
    assert_eq!(
        preimage["owner"],
        json!({"kind": "source", "authority": "a", "identity": "u"})
    );
    assert_ne!(under_u.key, declared(&w).key);
    let bare: Value = serde_json::from_slice(&node_key(&structural(&body)).expect("keys").preimage)
        .expect("JSON");
    assert!(
        bare.get("owner").is_none(),
        "an undeclared node has no owner member"
    );

    assert_eq!(
        node_key(&NodeInput {
            owner: Some(&u),
            ..structural(&body)
        }),
        Err(NodeKeyRefusal::OwnerDeclarationMismatch)
    );
    assert_eq!(
        node_key(&NodeInput {
            declaration: Some(&name),
            ..structural(&body)
        }),
        Err(NodeKeyRefusal::OwnerDeclarationMismatch)
    );
    // FR-094: a model-owned node's `declaration` is `null`.
    let model = Owner::Model(
        ModelOwner::new("acme/orders", "1.0.0", "ix://acme/orders/Order").expect("nonempty"),
    );
    assert_eq!(
        node_key(&NodeInput {
            owner: Some(&model),
            declaration: Some(&name),
            ..structural(&body)
        }),
        Err(NodeKeyRefusal::OwnerDeclarationMismatch)
    );
    let application = add(vec![reference(1)]);
    assert_eq!(
        node_key(&NodeInput {
            owner: Some(&u),
            declaration: Some(&name),
            semantic_type: Some(key(3)),
            ..structural(&application)
        }),
        Err(NodeKeyRefusal::OwnedApplication)
    );
    assert_eq!(
        node_key(&structural(&application)),
        Err(NodeKeyRefusal::UntypedApplication)
    );
}

/// FR-092-AC-8: the nominal enum declaration and member are keyed by
/// QSpec's own preimages, never `quire.structural-node/v1`: QSpec's
/// `enum-status` and `enum-status-ready` vectors, read at run time from
/// `$QSPEC_DIR`, recompute their recorded `sha256` through
/// `value::enumeration`, and the member key `check` gives an enum-member
/// literal (`mint_variant_id`) equals the member vector's. Skipped (and
/// passing) when `QSPEC_DIR` is unset; `make conformance` requires it.
#[trace("FR-092-AC-8", "TC-413")]
#[test]
fn conformance_fr092_nominal_enum_keys_match_qspec_vectors() {
    use crate::value::enumeration::{mint_variant_id, EnumDeclarationPreimage, EnumMemberPreimage};
    use crate::value::semantic_node::NodeIdentityPreimage;

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
    let vector = |name: &str| {
        document["vectors"]
            .as_array()
            .and_then(|vectors| vectors.iter().find(|vector| vector["name"] == name))
            .unwrap_or_else(|| panic!("{} has no `{name}` vector", path.display()))
            .clone()
    };
    let declaration = vector("enum-status");
    let member = vector("enum-status-ready");
    assert_eq!(
        declaration["preimage"]["version"], "quire.enum-declaration-node/v1",
        "enum-status is a nominal declaration preimage"
    );
    assert_eq!(member["preimage"]["version"], "quire.enum-member-node/v1");

    let declaration_key = EnumDeclarationPreimage::from_json(declaration["preimage"].clone())
        .expect("enum-status decodes")
        .digest()
        .expect("enum-status digests");
    assert_eq!(
        Some(hex(&declaration_key).as_str()),
        declaration["sha256"].as_str()
    );
    let member_key = EnumMemberPreimage::from_json(member["preimage"].clone())
        .expect("enum-status-ready decodes")
        .digest()
        .expect("enum-status-ready digests");
    assert_eq!(Some(hex(&member_key).as_str()), member["sha256"].as_str());
    let minted = mint_variant_id(NodeKey::from_digest(declaration_key), "READY");
    assert_eq!(minted.as_bytes(), &member_key);
    println!(
        "conformance: 2 nominal enum vectors match ({})",
        path.display()
    );
}

fn hex(bytes: &[u8; 32]) -> String {
    NodeKey::from_digest(*bytes).to_string()
}
