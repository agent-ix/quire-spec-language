// SPDX-License-Identifier: AGPL-3.0-only
//! FR-008: constant-size immutable runtime views preserving captured observations.

use super::super::{FieldBinding, ObjectIdentity, ValueId, ValueNode};
use quire_contract_ir as ir;

#[derive(Clone, Copy)]
pub(super) struct Arena<'a> {
    pub nodes: &'a [ValueNode],
    pub observation: ir::StateObservation,
}

#[derive(Clone, Copy)]
pub(super) enum Value<'a> {
    Boolean(bool),
    Integer(i64),
    Text(&'a str),
    Enumeration {
        owner: &'a ir::RequirementRef,
        name: &'a ir::SymbolName,
        variant: &'a str,
    },
    Record {
        fields: &'a [FieldBinding],
        arena: Arena<'a>,
    },
    Option {
        value: Option<ValueId>,
        arena: Arena<'a>,
    },
    Sequence {
        values: &'a [ValueId],
        arena: Arena<'a>,
    },
    Reference {
        identity: &'a ObjectIdentity,
        observation: ir::StateObservation,
    },
    Object {
        identity: &'a ObjectIdentity,
        observation: ir::StateObservation,
    },
}

impl<'a> Value<'a> {
    pub fn object_identity(self) -> Option<(&'a ObjectIdentity, ir::StateObservation)> {
        match self {
            Self::Object {
                identity,
                observation,
            }
            | Self::Reference {
                identity,
                observation,
            } => Some((identity, observation)),
            _ => None,
        }
    }
}

impl<'a> Arena<'a> {
    pub fn value(self, id: ValueId) -> Option<Value<'a>> {
        Some(match self.nodes.get(usize::try_from(id.index()).ok()?)? {
            ValueNode::Boolean { value } => Value::Boolean(*value),
            ValueNode::Integer { value } => Value::Integer(*value),
            ValueNode::Text { value } => Value::Text(value),
            ValueNode::Enum {
                declaration,
                variant,
            } => Value::Enumeration {
                owner: &declaration.model,
                name: &declaration.name,
                variant: variant.as_str(),
            },
            ValueNode::Record { fields, .. } => Value::Record {
                fields,
                arena: self,
            },
            ValueNode::Absent => Value::Option {
                value: None,
                arena: self,
            },
            ValueNode::Present { value } => Value::Option {
                value: Some(*value),
                arena: self,
            },
            ValueNode::Sequence { values } => Value::Sequence {
                values,
                arena: self,
            },
            ValueNode::Reference { identity } => Value::Reference {
                identity,
                observation: self.observation,
            },
            ValueNode::Object { identity } => Value::Object {
                identity,
                observation: self.observation,
            },
        })
    }
}
