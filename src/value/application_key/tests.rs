// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit tests for the FR-322 application-node key, and the opt-in
//! conformance check against QSpec's published `operation_vectors`.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

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
        node_tag: NodeTag::Function,
        semantic_form: "function",
        semantic_type: key(3),
        declaration: Some(&declaration),
        recursion: Some(RecursionGroup::new(&group, 1).expect("group is valid")),
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
        recursion: Some(RecursionGroup::new(&group, 0).expect("group is valid")),
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
        recursion: Some(RecursionGroup::new(&first_group, 1).expect("group is valid")),
        ..node(&first_body)
    };
    let second = ApplicationNode {
        recursion: Some(RecursionGroup::new(&second_group, 1).expect("group is valid")),
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
        recursion: Some(RecursionGroup::new(&group, 0).expect("group is valid")),
        ..bare
    };
    let second = ApplicationNode {
        recursion: Some(RecursionGroup::new(&group, 1).expect("group is valid")),
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
    let safe = i64::try_from(JCS_SAFE_INTEGER).expect("2^53 - 1 fits i64");
    let literal = |value: i64| {
        add(vec![SemanticTerm::Literal {
            ty: NodeRef(key(3)),
            value_kind: LiteralKind::Integer,
            value: Some(LiteralValue::Integer(value)),
        }])
    };
    let edge = literal(safe);
    let negative_edge = literal(-safe);
    let beyond = literal(safe + 1);
    let negative_beyond = literal(-safe - 1);

    assert!(application_node_key(&node(&edge)).is_ok());
    assert!(application_node_key(&node(&negative_edge)).is_ok());
    assert_eq!(
        application_node_key(&node(&beyond)),
        Err(ApplicationKeyRefusal::UnsafeInteger {
            site: IntegerSite::Literal,
            value: JCS_SAFE_INTEGER + 1
        })
    );
    assert_eq!(
        application_node_key(&node(&negative_beyond)),
        Err(ApplicationKeyRefusal::UnsafeInteger {
            site: IntegerSite::Literal,
            value: -JCS_SAFE_INTEGER - 1
        })
    );
}

#[test]
fn a_member_position_outside_the_exact_range_is_refused() {
    let safe = u64::try_from(JCS_SAFE_INTEGER).expect("2^53 - 1 fits u64");
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
        Err(ApplicationKeyRefusal::UnsafeInteger {
            site: IntegerSite::MemberPosition,
            value: JCS_SAFE_INTEGER + 1
        })
    );
}

#[test]
fn a_recursion_group_needs_an_in_range_ordinal_and_distinct_members() {
    let group = [key(1), key(2)];
    assert!(RecursionGroup::new(&group, 1).is_ok());
    assert_eq!(
        RecursionGroup::new(&group, 2),
        Err(RecursionGroupRefusal::OrdinalOutOfRange {
            ordinal: 2,
            size: 2
        })
    );
    assert_eq!(
        RecursionGroup::new(&[], 0),
        Err(RecursionGroupRefusal::OrdinalOutOfRange {
            ordinal: 0,
            size: 0
        })
    );
    assert_eq!(
        RecursionGroup::new(&[key(1), key(2), key(1)], 1),
        Err(RecursionGroupRefusal::DuplicateMember { member: key(1) })
    );
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
        Err(ApplicationKeyRefusal::EmptyQualifiedName)
    );
    assert_eq!(
        application_node_key(&ApplicationNode {
            semantic_form: "",
            ..node(&body)
        }),
        Err(ApplicationKeyRefusal::EmptySemanticForm)
    );
    assert_eq!(
        application_node_key(&node(&empty_binding)),
        Err(ApplicationKeyRefusal::EmptyBindingName)
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
        Err(ApplicationKeyRefusal::TooDeep {
            limit: MAX_CHECKING_DEPTH
        })
    );
    assert_eq!(
        application_node_key(&node(&nested_arguments)),
        Err(ApplicationKeyRefusal::TooDeep {
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
    let literal =
        |fill: u8, value_kind: LiteralKind, value: Option<LiteralValue>| SemanticTerm::Literal {
            ty: NodeRef(key(fill)),
            value_kind,
            value,
        };
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
            literal(4, LiteralKind::Integer, Some(LiteralValue::Integer(42))),
            literal(
                5,
                LiteralKind::Text,
                Some(LiteralValue::Text("a\"b".to_owned())),
            ),
            literal(6, LiteralKind::None, None),
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
            r#"{{"term":"literal","type":{four},"value":42,"value_kind":"integer"}},"#,
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
