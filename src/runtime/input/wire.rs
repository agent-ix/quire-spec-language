// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-024: closed wire records and scalar constructors, owned by Deserialize types.

use quire_contract_ir as ir;
use serde::Deserialize;

use super::ValueId;
use qsl_foundation::serde_object::Object;
use qsl_foundation::{ByteDigest, SourceIdentity};

// The foreign RequirementRef decoder permits unknown keys. Keep the native
// record closed, then delegate identifier semantics to its existing constructor.
#[derive(Deserialize)]
#[serde(try_from = "Object<RequirementFields>")]
struct Requirement(ir::RequirementRef);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequirementFields {
    package: String,
    requirement: String,
    revision: u64,
}

impl TryFrom<Object<RequirementFields>> for Requirement {
    type Error = ir::Diagnostic;

    fn try_from(Object(value): Object<RequirementFields>) -> Result<Self, Self::Error> {
        ir::RequirementRef::parse(&value.package, &value.requirement, value.revision).map(Self)
    }
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
pub(super) struct Symbol(ir::SymbolName);

impl TryFrom<String> for Symbol {
    type Error = ir::Diagnostic;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ir::SymbolName::new(value).map(Self)
    }
}

#[derive(Deserialize)]
#[serde(try_from = "Object<Labels>")]
pub(in crate::runtime) struct Identity(pub(in crate::runtime) SourceIdentity);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Labels {
    authority: String,
    identity: String,
    revision_namespace: String,
    revision: String,
}

impl TryFrom<Object<Labels>> for Identity {
    type Error = Box<super::InputError>;

    fn try_from(Object(value): Object<Labels>) -> Result<Self, Self::Error> {
        crate::runtime::construction::reference_identity(SourceIdentity {
            authority: value.authority,
            identity: value.identity,
            revision_namespace: value.revision_namespace,
            revision: value.revision,
        })
        .map(Self)
    }
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
struct Digest(ByteDigest);

impl TryFrom<String> for Digest {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ByteDigest::from_hex(&value)
            .map(Self)
            .map_err(|_| "expected 64 lowercase hexadecimal digits")
    }
}

// Deserialize the field through deserialize_any, not deserialize_option: Serde
// supplies a special missing-field deserializer which otherwise becomes None.
// A present JSON null is still the explicit no-result representation.
#[derive(Deserialize)]
#[serde(untagged)]
enum InvocationResult {
    None(()),
    Value(ValueId),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct QualifiedName {
    model: Requirement,
    name: Symbol,
}

impl From<Object<QualifiedName>> for super::QualifiedName {
    fn from(Object(value): Object<QualifiedName>) -> Self {
        Self {
            model: value.model.0,
            name: value.name.0,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ModelBinding {
    model: Requirement,
    digest: Digest,
}

impl From<Object<ModelBinding>> for super::ModelBinding {
    fn from(Object(value): Object<ModelBinding>) -> Self {
        Self {
            model: value.model.0,
            digest: value.digest.0,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ObjectIdentity {
    model: Requirement,
    record: Symbol,
    universe: Symbol,
    key: String,
}

impl From<Object<ObjectIdentity>> for super::ObjectIdentity {
    fn from(Object(value): Object<ObjectIdentity>) -> Self {
        Self {
            model: value.model.0,
            record: value.record.0,
            universe: value.universe.0,
            key: value.key,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FieldBinding {
    name: Symbol,
    value: ValueId,
}

impl From<Object<FieldBinding>> for super::FieldBinding {
    fn from(Object(value): Object<FieldBinding>) -> Self {
        Self {
            name: value.name.0,
            value: value.value,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ValueBinding {
    declaration: super::QualifiedName,
    value: ValueId,
}

impl From<Object<ValueBinding>> for super::ValueBinding {
    fn from(Object(value): Object<ValueBinding>) -> Self {
        Self {
            declaration: value.declaration,
            value: value.value,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ValueNode {
    Boolean {
        value: bool,
    },
    Integer {
        value: i64,
    },
    Text {
        value: String,
    },
    Enum {
        declaration: super::QualifiedName,
        variant: Symbol,
    },
    Record {
        declaration: super::QualifiedName,
        fields: Vec<super::FieldBinding>,
    },
    // An empty struct variant refuses extra fields; Serde's unit variant does not.
    Absent {},
    Present {
        value: ValueId,
    },
    Sequence {
        values: Vec<ValueId>,
    },
    Reference {
        identity: super::ObjectIdentity,
    },
    Object {
        identity: super::ObjectIdentity,
    },
}

impl From<Object<ValueNode>> for super::ValueNode {
    fn from(Object(value): Object<ValueNode>) -> Self {
        match value {
            ValueNode::Boolean { value } => Self::Boolean { value },
            ValueNode::Integer { value } => Self::Integer { value },
            ValueNode::Text { value } => Self::Text { value },
            ValueNode::Enum {
                declaration,
                variant,
            } => Self::Enum {
                declaration,
                variant: variant.0,
            },
            ValueNode::Record {
                declaration,
                fields,
            } => Self::Record {
                declaration,
                fields,
            },
            ValueNode::Absent {} => Self::Absent,
            ValueNode::Present { value } => Self::Present { value },
            ValueNode::Sequence { values } => Self::Sequence { values },
            ValueNode::Reference { identity } => Self::Reference { identity },
            ValueNode::Object { identity } => Self::Object { identity },
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ObjectEntry {
    key: String,
    fields: Vec<super::FieldBinding>,
}

impl From<Object<ObjectEntry>> for super::ObjectEntry {
    fn from(Object(value): Object<ObjectEntry>) -> Self {
        Self {
            key: value.key,
            fields: value.fields,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Population {
    model: Requirement,
    record: Symbol,
    universe: Symbol,
    complete: bool,
    objects: Vec<super::ObjectEntry>,
}

impl From<Object<Population>> for super::Population {
    fn from(Object(value): Object<Population>) -> Self {
        Self {
            model: value.model.0,
            record: value.record.0,
            universe: value.universe.0,
            complete: value.complete,
            objects: value.objects,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SnapshotDraft {
    observation: ir::StateObservation,
    models: Vec<super::ModelBinding>,
    populations: Vec<super::Population>,
    values: Vec<super::ValueBinding>,
    arena: Vec<super::ValueNode>,
}

impl From<Object<SnapshotDraft>> for super::SnapshotDraft {
    fn from(Object(value): Object<SnapshotDraft>) -> Self {
        Self {
            observation: value.observation,
            models: value.models,
            populations: value.populations,
            values: value.values,
            arena: value.arena,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InvocationDraft {
    models: Vec<super::ModelBinding>,
    context: super::QualifiedName,
    operation: Symbol,
    anchor: ir::AnchorName,
    self_object: super::ObjectIdentity,
    pre: super::SnapshotRef,
    post: super::SnapshotRef,
    parameters: Vec<super::ValueBinding>,
    result: InvocationResult,
    created: Vec<super::ObjectIdentity>,
    deleted: Vec<super::ObjectIdentity>,
    arena: Vec<super::ValueNode>,
}

impl From<Object<InvocationDraft>> for super::InvocationDraft {
    fn from(Object(value): Object<InvocationDraft>) -> Self {
        Self {
            models: value.models,
            context: value.context,
            operation: value.operation.0,
            anchor: value.anchor,
            self_object: value.self_object,
            pre: value.pre,
            post: value.post,
            parameters: value.parameters,
            result: match value.result {
                InvocationResult::None(()) => None,
                InvocationResult::Value(value) => Some(value),
            },
            created: value.created,
            deleted: value.deleted,
            arena: value.arena,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reference {
    identity: Identity,
    digest: Digest,
}

impl From<Object<Reference>> for super::SnapshotRef {
    fn from(Object(value): Object<Reference>) -> Self {
        Self {
            identity: value.identity.0,
            digest: value.digest.0,
        }
    }
}

impl From<Object<Reference>> for super::InvocationRef {
    fn from(Object(value): Object<Reference>) -> Self {
        Self {
            identity: value.identity.0,
            digest: value.digest.0,
        }
    }
}
