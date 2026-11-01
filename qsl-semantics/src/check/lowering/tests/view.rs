// SPDX-License-Identifier: AGPL-3.0-or-later
//! A test-only JSON view of a lowered node's structured content. The tests
//! read a node's body, operation, owner and declaration through this view;
//! it is built from the node's own terms, never from a recorded wire form,
//! and no node key is parsed back out of it (a node reference renders as its
//! key's display text).

use serde_json::{json, Value as Json};

use super::*;
use crate::check::node_key::{
    BindingValue, GroupMember, LeafSegment, LeafTerm, LiteralValue, MemberTerm, NodeRef,
    OperationMode, TupleMember,
};

/// The node's content as JSON: `version`, `node_tag`, `semantic_form`,
/// `semantic_type`, `declaration`, `owner`, `owner_model`, `recursion` and
/// `body`.
pub(crate) fn preimage(node: &SemanticNode) -> Json {
    let application = matches!(node.body(), BodyTerm::Application(_))
        && node.declaration().is_none()
        && node.owner().is_none();
    json!({
        "version": if application {
            "quire.application-node/v1"
        } else {
            "quire.structural-node/v1"
        },
        "node_tag": node_tag_text(node.node_tag()),
        "semantic_form": node.semantic_form(),
        "semantic_type": node
            .content
            .semantic_type
            .as_ref()
            .map_or(Json::Null, node_ref),
        "declaration": node.declaration().map_or(Json::Null, |names| {
            json!({"qualified_name": names.iter().map(|name| name.as_str()).collect::<Vec<_>>()})
        }),
        "owner": node
            .owner()
            .map_or(Json::Null, |owner| serde_json::to_value(owner).expect("an owner serializes")),
        "owner_model": node
            .owner_model()
            .map_or(Json::Null, |key| node_ref(&NodeRef::Admitted(key))),
        "recursion": node.recursion().map_or(Json::Null, |recursion| {
            json!({"ordinal": recursion.ordinal, "size": recursion.size})
        }),
        "body": body_json(node.body()),
    })
}

/// A node reference as `{domain, digest}`; `digest` holds the key's display
/// text, which tests compare against `key.to_string()`.
pub(crate) fn node_ref(reference: &NodeRef) -> Json {
    match reference {
        NodeRef::Admitted(key) => {
            json!({"domain": "quire.checked-semantic-node/v1", "digest": key.to_string()})
        }
        NodeRef::Pending(_) => json!({"pending": true}),
    }
}

fn node_tag_text(tag: NodeTag) -> &'static str {
    tag.as_str()
}

fn snake(text: &str) -> String {
    let mut out = String::new();
    for (at, character) in text.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if at > 0 {
                out.push('_');
            }
            out.push(character.to_ascii_lowercase());
        } else {
            out.push(character);
        }
    }
    out
}

fn body_json(body: &BodyTerm) -> Json {
    match body {
        BodyTerm::Leaf(leaf) => leaf_json(leaf),
        BodyTerm::Application(application) => json!({
            "term": "application",
            "operator": snake(&format!("{:?}", application.operator)),
            "operation": operation_json(&application.operation),
            "result_type": node_ref(&application.result_type),
            "arguments": application.arguments.iter().map(member_json).collect::<Vec<_>>(),
        }),
        BodyTerm::Aggregate(aggregate) => json!({
            "term": "aggregate",
            "members": aggregate.members.iter().map(member_json).collect::<Vec<_>>(),
        }),
        BodyTerm::Frame(frame) => json!({
            "term": "frame",
            "creates": frame.creates.iter().map(node_ref).collect::<Vec<_>>(),
            "deletes": frame.deletes.iter().map(node_ref).collect::<Vec<_>>(),
        }),
        BodyTerm::Abstraction(_) => json!({"term": "abstraction_relation"}),
    }
}

fn leaf_json(leaf: &LeafTerm) -> Json {
    match leaf {
        LeafTerm::Literal { ty, value } => {
            let (kind, text) = literal_json(value);
            json!({"term": "literal", "type": node_ref(ty), "value_kind": kind, "value": text})
        }
        LeafTerm::Reference { target } => json!({"term": "reference", "target": node_ref(target)}),
        LeafTerm::DependencyReference { node } => {
            json!({"term": "dependency_reference", "node": node_ref(node)})
        }
    }
}

fn literal_json(value: &LiteralValue) -> (&'static str, Json) {
    match value {
        LiteralValue::Boolean(value) => ("boolean", json!(value)),
        LiteralValue::Integer(value) => ("integer", json!(value.to_string())),
        LiteralValue::Count(value) => ("integer", json!(value)),
        LiteralValue::Rational(value) => ("rational", json!(value.to_string())),
        LiteralValue::Float32Bits(bits) => ("float32_bits", json!(format!("{bits:08x}"))),
        LiteralValue::Float64Bits(bits) => ("float64_bits", json!(format!("{bits:016x}"))),
        LiteralValue::Quantity(value) => ("quantity", json!(value.to_string())),
        LiteralValue::Text(text) => ("text", json!(text)),
        LiteralValue::String(text) => ("string", json!(text)),
        LiteralValue::Uuid(value) => ("uuid", json!(value.to_string())),
        LiteralValue::Timestamp(value) => ("timestamp", json!(value.to_string())),
        LiteralValue::None => ("none", Json::Null),
        LiteralValue::Enum(case) => ("enum", json!(case)),
    }
}

fn member_json(member: &MemberTerm) -> Json {
    match member {
        MemberTerm::Leaf(leaf) => leaf_json(leaf),
        MemberTerm::Group(group) => group_json(group),
        MemberTerm::Binding(binding) => {
            json!({"term": "binding", "name": binding.name, "value": binding_value_json(&binding.value)})
        }
    }
}

fn group_json(group: &GroupTerm) -> Json {
    json!({
        "term": "aggregate",
        "members": group.members.iter().map(|member| match member {
            GroupMember::Leaf(leaf) => leaf_json(leaf),
            GroupMember::Binding(binding) => {
                json!({"term": "binding", "name": binding.name, "value": leaf_json(&binding.value)})
            }
        }).collect::<Vec<_>>(),
    })
}

fn binding_value_json(value: &BindingValue) -> Json {
    match value {
        BindingValue::Leaf(leaf) => leaf_json(leaf),
        BindingValue::Group(group) => group_json(group),
        BindingValue::Tuple(tuple) => json!({
            "term": "aggregate",
            "members": tuple.members.iter().map(|member| match member {
                TupleMember::Leaf(leaf) => leaf_json(leaf),
                TupleMember::Group(group) => group_json(group),
            }).collect::<Vec<_>>(),
        }),
        BindingValue::Aggregate(aggregate) => json!({
            "term": "aggregate",
            "members": aggregate.members.iter().map(member_json).collect::<Vec<_>>(),
        }),
        BindingValue::ExecutionPoint(_) => json!({"term": "execution_point"}),
    }
}

fn operation_json(operation: &Operation) -> Json {
    json!({
        "identity": operation.identity(),
        "laws": operation.laws().iter().map(law_json).collect::<Vec<_>>(),
        "mode": operation.mode().map_or(Json::Null, mode_json),
        "member": operation.member().map_or(Json::Null, member_kind_json),
        "leaves": operation.leaves().iter().map(|leaf| json!({
            "path": leaf.path.iter().map(segment_text).collect::<Vec<_>>(),
            "laws": leaf.laws.iter().map(law_json).collect::<Vec<_>>(),
            "mode": leaf.mode.as_ref().map_or(Json::Null, mode_json),
        })).collect::<Vec<_>>(),
    })
}

pub(crate) fn law_json(law: &OperationLaw) -> Json {
    json!({
        "role": law.role.as_str(),
        "definition": serde_json::to_value(&law.definition).expect("a definition serializes"),
    })
}

fn mode_json(mode: &OperationMode) -> Json {
    match mode {
        OperationMode::Rounding(mode) => {
            json!({"kind": "rounding", "value": mode.as_str()})
        }
        OperationMode::TextProfile(profile) => {
            json!({"kind": "text_profile", "value": profile.as_str()})
        }
        OperationMode::Absence(mode) => {
            json!({"kind": "absence", "value": snake(&format!("{mode:?}"))})
        }
    }
}

fn segment_text(segment: &LeafSegment) -> String {
    match segment {
        LeafSegment::Field(name) => format!("field:{}", name.as_str()),
        LeafSegment::Position(at) => format!("position:{at}"),
        LeafSegment::Inner => "inner".to_owned(),
        LeafSegment::Member(name) => format!("member:{}", name.as_str()),
        LeafSegment::Recursion(depth) => format!("recursion:{depth}"),
    }
}

fn declared(key: &quire_exact::NodeKey) -> Json {
    node_ref(&NodeRef::Admitted(key.clone()))
}

fn member_kind_json(member: &Member) -> Json {
    match member {
        Member::Field {
            declaration, name, ..
        } => {
            json!({"kind": "field", "declaration": declared(declaration), "name": name.as_str()})
        }
        Member::Position {
            declaration,
            position,
        } => {
            json!({"kind": "position", "declaration": declared(declaration), "position": position})
        }
        Member::Element { declaration } => {
            json!({"kind": "element", "declaration": declared(declaration)})
        }
        Member::RelationshipEnd {
            declaration,
            name,
            destination_type,
            ..
        } => {
            json!({"kind": "relationship_end", "declaration": declared(declaration), "name": name.as_str(), "destination_type": declared(destination_type)})
        }
        Member::Operation { declaration, name } => {
            json!({"kind": "operation", "declaration": declared(declaration), "name": name.as_str()})
        }
        Member::TypeArgument { declaration } => {
            json!({"kind": "type_argument", "declaration": declared(declaration)})
        }
        Member::ProfileOperator { operator } => {
            json!({"kind": "profile_operator", "operator": operator.as_str()})
        }
        _ => json!({"kind": "other"}),
    }
}
