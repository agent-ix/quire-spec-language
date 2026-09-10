// SPDX-License-Identifier: AGPL-3.0-only
//! FR-018/NFR-006: bounded structural inspection and deterministic artifact output.

use std::io::{self, Write};

use quire_contract_ir as ir;
use serde::Serialize;

use super::input::{
    self, ArtifactLimits, ArtifactUsage, DraftPathSegment, FieldBinding, InputError,
    InvocationDraft, ModelBinding, ObjectIdentity, QualifiedName, SnapshotDraft, ValueBinding,
    ValueId, ValueNode,
};
use crate::{ByteDigest, Code, SourceIdentity};

type Result<T> = std::result::Result<T, Failure>;

#[derive(Debug)]
pub(super) struct Failure {
    code: Code,
    path: Vec<DraftPathSegment>,
    message: &'static str,
}

impl Failure {
    fn with_identity(self, identity: SourceIdentity, usage: ArtifactUsage) -> Box<InputError> {
        Box::new(InputError {
            code: self.code,
            identity,
            path: self.path,
            usage,
            message: self.message,
        })
    }
}

#[derive(Clone, Debug)]
pub(super) struct Artifact<T> {
    pub identity: SourceIdentity,
    pub draft: T,
    pub bytes: Vec<u8>,
    pub digest: ByteDigest,
    pub usage: ArtifactUsage,
}

/// Closed internal seam shared by the two distinct public artifact constructors.
pub(super) trait Body: Serialize {
    const KIND: &'static str;
    fn arena(&self) -> &[ValueNode];
    fn inspect(&self, budget: &mut Budget) -> Result<()>;
}

impl<T: Body> Artifact<T> {
    /// Adopt an externally decoded body after the reader verified its byte digest.
    /// Usage describes the actual constructor pass; retained bytes preserve the
    /// caller's accepted layout rather than the constructor's normalized encoding.
    pub fn from_external(
        identity: SourceIdentity,
        draft: T,
        bytes: &[u8],
        digest: ByteDigest,
        limits: ArtifactLimits,
    ) -> std::result::Result<Self, Box<InputError>> {
        let constructed = Self::new(identity, draft, limits)?;
        Ok(Self {
            bytes: bytes.to_vec(),
            digest,
            ..constructed
        })
    }

    pub fn new(
        identity: SourceIdentity,
        draft: T,
        limits: ArtifactLimits,
    ) -> std::result::Result<Self, Box<InputError>> {
        let mut budget = Budget::new(limits);
        let result = (|| {
            budget.field("identity", |budget| budget.identity(&identity))?;
            // Reject excessive caller vectors before allocating depth/index storage.
            budget.field("arena", |budget| {
                budget.ceiling(draft.arena().len(), budget.limits.nodes, "arena node limit")
            })?;
            draft.inspect(&mut budget)?;
            budget.depths(draft.arena())?;
            budget.encode(&identity, &draft)
        })();
        // Move even oversized input labels into the error; cloning them on the
        // failure path would add work before their content budget was admitted.
        match result {
            Ok(bytes) => Ok(Self {
                identity,
                draft,
                digest: ByteDigest::of(&bytes),
                bytes,
                usage: budget.usage,
            }),
            Err(error) => Err(error.with_identity(identity, budget.usage)),
        }
    }
}

pub(super) fn reference_identity(
    identity: SourceIdentity,
) -> std::result::Result<SourceIdentity, Box<InputError>> {
    let mut budget = Budget::new(ArtifactLimits::default());
    match budget.field("identity", |budget| budget.identity(&identity)) {
        Ok(()) => Ok(identity),
        Err(error) => Err(error.with_identity(identity, budget.usage)),
    }
}

pub(super) struct Budget {
    limits: ArtifactLimits,
    usage: ArtifactUsage,
    path: Vec<DraftPathSegment>,
}

impl Budget {
    fn new(limits: ArtifactLimits) -> Self {
        Self {
            limits: limits.bounded(),
            usage: ArtifactUsage::default(),
            path: Vec::new(),
        }
    }

    fn failure(&self, code: Code, message: &'static str) -> Failure {
        Failure {
            code,
            path: self.path.clone(),
            message,
        }
    }

    fn at<T>(
        &mut self,
        segment: DraftPathSegment,
        inspect: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        self.path.push(segment);
        let result = inspect(self);
        self.path.pop();
        result
    }

    fn field<T>(
        &mut self,
        field: &'static str,
        inspect: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        self.at(DraftPathSegment::Field(field), inspect)
    }

    fn ceiling(&self, amount: usize, maximum: usize, message: &'static str) -> Result<()> {
        if amount > maximum {
            Err(self.failure(Code::ResourceExhausted, message))
        } else {
            Ok(())
        }
    }

    fn entries(&mut self, count: usize) -> Result<()> {
        let next = self
            .usage
            .entries
            .checked_add(count)
            .ok_or_else(|| self.failure(Code::ResourceExhausted, "entry count overflow"))?;
        self.ceiling(next, self.limits.entries, "aggregate entry limit")?;
        self.usage.entries = next;
        Ok(())
    }

    fn vector<T>(
        &mut self,
        name: &'static str,
        values: &[T],
        mut inspect: impl FnMut(&mut Self, &T) -> Result<()>,
    ) -> Result<()> {
        self.field(name, |budget| {
            budget.entries(values.len())?;
            for (index, value) in values.iter().enumerate() {
                budget.at(DraftPathSegment::Index(index), |budget| {
                    inspect(budget, value)
                })?;
            }
            Ok(())
        })
    }

    fn text(&mut self, value: &str) -> Result<()> {
        let next = self
            .usage
            .text_bytes
            .checked_add(value.len())
            .ok_or_else(|| self.failure(Code::ResourceExhausted, "text byte count overflow"))?;
        self.ceiling(next, self.limits.artifact_bytes, "input text content limit")?;
        self.usage.text_bytes = next;
        Ok(())
    }

    fn identity(&mut self, identity: &SourceIdentity) -> Result<()> {
        for (name, value) in [
            ("identity", identity.identity.as_str()),
            ("revision", identity.revision.as_str()),
        ] {
            self.field(name, |budget| {
                budget.text(value)?;
                if value.is_empty() {
                    return Err(budget.failure(
                        Code::InvalidSourceIdentity,
                        "native input identity and revision must be nonempty",
                    ));
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    fn model(&mut self, model: &ir::RequirementRef) -> Result<()> {
        self.field("package", |budget| budget.text(model.package().as_str()))?;
        self.field("requirement", |budget| {
            budget.text(model.requirement().as_str())
        })
    }

    fn qualified(&mut self, declaration: &QualifiedName) -> Result<()> {
        self.field("model", |budget| budget.model(&declaration.model))?;
        self.field("name", |budget| budget.text(declaration.name.as_str()))
    }

    fn object(&mut self, identity: &ObjectIdentity) -> Result<()> {
        self.field("model", |budget| budget.model(&identity.model))?;
        self.field("record", |budget| budget.text(identity.record.as_str()))?;
        self.field("universe", |budget| budget.text(identity.universe.as_str()))?;
        self.field("key", |budget| budget.text(&identity.key))
    }

    fn models(&mut self, models: &[ModelBinding]) -> Result<()> {
        self.vector("models", models, |budget, binding| {
            budget.field("model", |budget| budget.model(&binding.model))
        })
    }

    fn value(&self, value: ValueId, upper: usize) -> Result<()> {
        let index = usize::try_from(value.index()).map_err(|_| {
            self.failure(
                Code::InvalidRuntimeInput,
                "local value index is not addressable",
            )
        })?;
        if index >= upper {
            return Err(self.failure(
                Code::InvalidRuntimeInput,
                "local root is out of range or child does not precede its parent",
            ));
        }
        Ok(())
    }

    fn fields(&mut self, fields: &[FieldBinding], upper: usize) -> Result<()> {
        self.vector("fields", fields, |budget, field| {
            budget.field("name", |budget| budget.text(field.name.as_str()))?;
            budget.field("value", |budget| budget.value(field.value, upper))
        })
    }

    fn bindings(
        &mut self,
        name: &'static str,
        bindings: &[ValueBinding],
        upper: usize,
    ) -> Result<()> {
        self.vector(name, bindings, |budget, binding| {
            budget.field("declaration", |budget| {
                budget.qualified(&binding.declaration)
            })?;
            budget.field("value", |budget| budget.value(binding.value, upper))
        })
    }

    fn arena(&mut self, arena: &[ValueNode]) -> Result<()> {
        self.field("arena", |budget| {
            for (index, node) in arena.iter().enumerate() {
                // The complete arena length was checked before any inspection.
                budget.usage.nodes = index.checked_add(1).ok_or_else(|| {
                    budget.failure(Code::ResourceExhausted, "arena node count overflow")
                })?;
                budget.at(DraftPathSegment::Index(index), |budget| {
                    budget.node(node, index)
                })?;
            }
            Ok(())
        })
    }

    fn node(&mut self, node: &ValueNode, index: usize) -> Result<()> {
        match node {
            ValueNode::Boolean { .. } | ValueNode::Integer { .. } | ValueNode::Absent => Ok(()),
            ValueNode::Text { value } => self.field("value", |budget| budget.text(value)),
            ValueNode::Enum {
                declaration,
                variant,
            } => {
                self.field("declaration", |budget| budget.qualified(declaration))?;
                self.field("variant", |budget| budget.text(variant.as_str()))
            }
            ValueNode::Record {
                declaration,
                fields,
            } => {
                self.field("declaration", |budget| budget.qualified(declaration))?;
                self.fields(fields, index)
            }
            ValueNode::Present { value } => self.field("value", |budget| {
                budget.entries(1)?;
                budget.value(*value, index)
            }),
            ValueNode::Sequence { values } => self.vector("values", values, |budget, value| {
                budget.value(*value, index)
            }),
            ValueNode::Reference { identity } | ValueNode::Object { identity } => {
                self.field("identity", |budget| budget.object(identity))
            }
        }
    }

    fn greatest_child_depth(
        &self,
        children: impl IntoIterator<Item = ValueId>,
        depths: &[usize],
    ) -> Result<usize> {
        let mut greatest = 0;
        for child in children {
            let index = usize::try_from(child.index()).map_err(|_| {
                self.failure(
                    Code::InvalidRuntimeInput,
                    "local child index is not addressable",
                )
            })?;
            let depth = depths.get(index).ok_or_else(|| {
                self.failure(
                    Code::InvalidRuntimeInput,
                    "child does not precede its parent",
                )
            })?;
            greatest = greatest.max(*depth);
        }
        Ok(greatest)
    }

    fn depths(&mut self, arena: &[ValueNode]) -> Result<()> {
        // Node, vector-entry and text lengths have all passed admission. Each
        // node's depth is calculated once, without expanding shared subtrees.
        let mut depths = Vec::with_capacity(arena.len());
        self.field("arena", |budget| {
            for (index, node) in arena.iter().enumerate() {
                budget.at(DraftPathSegment::Index(index), |budget| {
                    let child_depth = match node {
                        ValueNode::Record { fields, .. } => budget.greatest_child_depth(
                            fields.iter().map(|field| field.value),
                            &depths,
                        )?,
                        ValueNode::Sequence { values } => {
                            budget.greatest_child_depth(values.iter().copied(), &depths)?
                        }
                        ValueNode::Present { value } => {
                            budget.greatest_child_depth([*value], &depths)?
                        }
                        ValueNode::Boolean { .. }
                        | ValueNode::Integer { .. }
                        | ValueNode::Text { .. }
                        | ValueNode::Enum { .. }
                        | ValueNode::Absent
                        | ValueNode::Reference { .. }
                        | ValueNode::Object { .. } => 0,
                    };
                    let depth = child_depth.checked_add(1).ok_or_else(|| {
                        budget.failure(Code::ResourceExhausted, "value depth overflow")
                    })?;
                    budget.ceiling(depth, budget.limits.depth, "structural value depth limit")?;
                    budget.usage.depth = budget.usage.depth.max(depth);
                    depths.push(depth);
                    Ok(())
                })?;
            }
            Ok(())
        })
    }

    fn encode<T: Body>(&mut self, identity: &SourceIdentity, draft: &T) -> Result<Vec<u8>> {
        #[derive(Serialize)]
        struct Envelope<'a, T> {
            version: &'static str,
            kind: &'static str,
            #[serde(serialize_with = "input::serialize_identity")]
            identity: &'a SourceIdentity,
            body: &'a T,
        }
        let envelope = Envelope {
            version: input::FORMAT,
            kind: T::KIND,
            identity,
            body: draft,
        };
        let mut writer = BoundedBytes {
            maximum: self.limits.artifact_bytes,
            bytes: Vec::new(),
        };
        let result = serde_json::to_writer(&mut writer, &envelope);
        self.usage.artifact_bytes = writer.bytes.len();
        // The two closed Serialize bodies contain no caller serializer. Their
        // only fallible output boundary is this content-limited writer.
        result
            .map_err(|_| self.failure(Code::ResourceExhausted, "encoded artifact content limit"))?;
        Ok(writer.bytes)
    }
}

impl Body for SnapshotDraft {
    const KIND: &'static str = "snapshot";

    fn arena(&self) -> &[ValueNode] {
        &self.arena
    }

    fn inspect(&self, budget: &mut Budget) -> Result<()> {
        budget.models(&self.models)?;
        budget.vector("populations", &self.populations, |budget, population| {
            budget.field("model", |budget| budget.model(&population.model))?;
            budget.field("record", |budget| budget.text(population.record.as_str()))?;
            budget.field("universe", |budget| {
                budget.text(population.universe.as_str())
            })?;
            budget.vector("objects", &population.objects, |budget, object| {
                budget.field("key", |budget| budget.text(&object.key))?;
                budget.fields(&object.fields, self.arena.len())
            })
        })?;
        budget.bindings("values", &self.values, self.arena.len())?;
        budget.arena(&self.arena)
    }
}

impl Body for InvocationDraft {
    const KIND: &'static str = "invocation";

    fn arena(&self) -> &[ValueNode] {
        &self.arena
    }

    fn inspect(&self, budget: &mut Budget) -> Result<()> {
        budget.models(&self.models)?;
        budget.field("context", |budget| budget.qualified(&self.context))?;
        budget.field("operation", |budget| budget.text(self.operation.as_str()))?;
        budget.field("anchor", |budget| budget.text(self.anchor.as_str()))?;
        budget.field("self_object", |budget| budget.object(&self.self_object))?;
        budget.field("pre", |budget| {
            budget.field("identity", |budget| budget.identity(self.pre.identity()))
        })?;
        budget.field("post", |budget| {
            budget.field("identity", |budget| budget.identity(self.post.identity()))
        })?;
        budget.bindings("parameters", &self.parameters, self.arena.len())?;
        if let Some(result) = self.result {
            budget.field("result", |budget| {
                budget.entries(1)?;
                budget.value(result, self.arena.len())
            })?;
        }
        budget.vector("created", &self.created, |budget, identity| {
            budget.object(identity)
        })?;
        budget.vector("deleted", &self.deleted, |budget, identity| {
            budget.object(identity)
        })?;
        budget.arena(&self.arena)
    }
}

struct BoundedBytes {
    bytes: Vec<u8>,
    maximum: usize,
}

impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .filter(|next| *next <= self.maximum)
            .ok_or_else(|| io::Error::other("native input content limit"))?;
        self.bytes.extend_from_slice(bytes);
        debug_assert_eq!(self.bytes.len(), next);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
