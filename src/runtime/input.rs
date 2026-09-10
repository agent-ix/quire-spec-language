// SPDX-License-Identifier: AGPL-3.0-only
//! FR-018/024: flat native input drafts, closed decoding and structural error context.

use quire_contract_ir as ir;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

use crate::serde_object::{
    deserialize_empty_object as deserialize_absent, deserialize_objects,
    from_object as deserialize_object,
};
use crate::{ByteDigest, Code, SourceIdentity};

/// One version shared by native input construction and reading.
pub(super) const FORMAT: &str = crate::wire_format::WireFormat::RuntimeInput.as_str();

/// Artifact-local arena index; it carries no model or cross-artifact identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ValueId(u32);

impl ValueId {
    /// Select a local index, checked by the consuming artifact constructor.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Original unsigned local index.
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Exact model-owned declaration selector; this does not admit its semantics.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualifiedName {
    /// Exact model owner and revision.
    #[serde(deserialize_with = "deserialize_requirement")]
    pub model: ir::RequirementRef,
    /// Declaration name within that model.
    #[serde(deserialize_with = "deserialize_symbol")]
    pub name: ir::SymbolName,
}

/// Exact input binding to an already admitted native model artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelBinding {
    /// Model requirement owner, including its selected revision.
    #[serde(deserialize_with = "deserialize_requirement")]
    pub model: ir::RequirementRef,
    /// Complete native model artifact digest.
    #[serde(
        serialize_with = "serialize_digest",
        deserialize_with = "deserialize_digest"
    )]
    pub digest: ByteDigest,
}

/// Object identity within an exact model, type and declared universe.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectIdentity {
    /// Exact model requirement.
    #[serde(deserialize_with = "deserialize_requirement")]
    pub model: ir::RequirementRef,
    /// Object payload record name.
    #[serde(deserialize_with = "deserialize_symbol")]
    pub record: ir::SymbolName,
    /// Declared population universe.
    #[serde(deserialize_with = "deserialize_symbol")]
    pub universe: ir::SymbolName,
    /// Exact Unicode key; its model bound is checked during runtime validation.
    pub key: String,
}

/// One named field pointing into its containing artifact's arena.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldBinding {
    /// Original field name, without duplicate coalescing.
    #[serde(deserialize_with = "deserialize_symbol")]
    pub name: ir::SymbolName,
    /// Local field value.
    pub value: ValueId,
}

/// A State or invocation-parameter declaration pointing into the local arena.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValueBinding {
    /// Exact declaration owner and name; runtime validation checks its role.
    #[serde(deserialize_with = "deserialize_object")]
    pub declaration: QualifiedName,
    /// Local value root.
    pub value: ValueId,
}

/// Flat value structure. Container children must precede their parent node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ValueNode {
    /// Exact Boolean value.
    Boolean {
        /// Boolean payload.
        value: bool,
    },
    /// Exact signed integer; nominal bounds come from its later model site.
    Integer {
        /// Signed payload without floating conversion.
        value: i64,
    },
    /// Unicode scalar text without normalization.
    Text {
        /// Exact text payload.
        value: String,
    },
    /// Explicit model-owned enum variant.
    Enum {
        /// Enum declaration.
        #[serde(deserialize_with = "deserialize_object")]
        declaration: QualifiedName,
        /// Variant name.
        #[serde(deserialize_with = "deserialize_symbol")]
        variant: ir::SymbolName,
    },
    /// Structural record, distinct from object identity.
    Record {
        /// Exact record declaration.
        #[serde(deserialize_with = "deserialize_object")]
        declaration: QualifiedName,
        /// Original field bindings, preserving order and duplicates.
        #[serde(deserialize_with = "deserialize_objects")]
        fields: Vec<FieldBinding>,
    },
    /// Explicit optional absence; this is not missing input.
    #[serde(deserialize_with = "deserialize_absent")]
    Absent,
    /// Present optional payload.
    Present {
        /// Earlier node holding the payload.
        value: ValueId,
    },
    /// Ordered duplicate-preserving sequence.
    Sequence {
        /// Earlier nodes, one per occurrence.
        values: Vec<ValueId>,
    },
    /// Opaque object reference captured at its containing observation.
    Reference {
        /// Target identity; no snapshot retagging is supplied here.
        #[serde(deserialize_with = "deserialize_object")]
        identity: ObjectIdentity,
    },
    /// Identity-bearing object access, distinct from an opaque reference.
    Object {
        /// Object identity within the containing observation.
        #[serde(deserialize_with = "deserialize_object")]
        identity: ObjectIdentity,
    },
}

/// One object's original field bindings.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectEntry {
    /// Exact key within the enclosing population.
    pub key: String,
    /// Fields with artifact-local roots.
    #[serde(deserialize_with = "deserialize_objects")]
    pub fields: Vec<FieldBinding>,
}

/// An explicitly declared finite population; completeness is still an assertion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Population {
    /// Exact model owner.
    #[serde(deserialize_with = "deserialize_requirement")]
    pub model: ir::RequirementRef,
    /// Object payload record.
    #[serde(deserialize_with = "deserialize_symbol")]
    pub record: ir::SymbolName,
    /// Declared universe.
    #[serde(deserialize_with = "deserialize_symbol")]
    pub universe: ir::SymbolName,
    /// Whether the caller asserts this offered population is complete.
    pub complete: bool,
    /// Original objects, without duplicate identity coalescing.
    #[serde(deserialize_with = "deserialize_objects")]
    pub objects: Vec<ObjectEntry>,
}

/// Caller-owned snapshot draft, structurally checked by Snapshot::new.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotDraft {
    /// Explicit observation role; invocation validation checks correspondence.
    pub observation: ir::StateObservation,
    /// Exact native model artifact bindings.
    #[serde(deserialize_with = "deserialize_objects")]
    pub models: Vec<ModelBinding>,
    /// Offered finite populations.
    #[serde(deserialize_with = "deserialize_objects")]
    pub populations: Vec<Population>,
    /// Named State roots; self/parameter/result roles are checked later.
    #[serde(deserialize_with = "deserialize_objects")]
    pub values: Vec<ValueBinding>,
    /// Flat local values, including any unused nodes.
    #[serde(deserialize_with = "deserialize_objects")]
    pub arena: Vec<ValueNode>,
}

/// Caller-owned recorded invocation; frame permissions come only from the model.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationDraft {
    /// Exact native model artifact bindings.
    #[serde(deserialize_with = "deserialize_objects")]
    pub models: Vec<ModelBinding>,
    /// Exact operation context declaration.
    #[serde(deserialize_with = "deserialize_object")]
    pub context: QualifiedName,
    /// Operation name within the context.
    #[serde(deserialize_with = "deserialize_symbol")]
    pub operation: ir::SymbolName,
    /// Original selected operation anchor.
    pub anchor: ir::AnchorName,
    /// Invocation self identity.
    #[serde(deserialize_with = "deserialize_object")]
    pub self_object: ObjectIdentity,
    /// Explicit pre observation selection.
    #[serde(deserialize_with = "deserialize_object")]
    pub pre: SnapshotRef,
    /// Explicit post observation selection.
    #[serde(deserialize_with = "deserialize_object")]
    pub post: SnapshotRef,
    /// Ordered parameter bindings captured at pre.
    #[serde(deserialize_with = "deserialize_objects")]
    pub parameters: Vec<ValueBinding>,
    /// Optional declared result root captured at post.
    #[serde(deserialize_with = "deserialize_required_option")]
    pub result: Option<ValueId>,
    /// Original declared created identities.
    #[serde(deserialize_with = "deserialize_objects")]
    pub created: Vec<ObjectIdentity>,
    /// Original declared deleted identities.
    #[serde(deserialize_with = "deserialize_objects")]
    pub deleted: Vec<ObjectIdentity>,
    /// Flat local parameter/result values.
    #[serde(deserialize_with = "deserialize_objects")]
    pub arena: Vec<ValueNode>,
}

/// Expected snapshot identity and complete byte digest, distinct from invocation.
///
/// The roles cannot be substituted at a call boundary:
/// ```compile_fail,E0308
/// use quire_spec_language::runtime::{InvocationRef, SnapshotRef};
/// fn swapped(value: InvocationRef) -> SnapshotRef { value }
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotRef {
    #[serde(
        serialize_with = "serialize_identity",
        deserialize_with = "deserialize_identity"
    )]
    identity: SourceIdentity,
    #[serde(
        serialize_with = "serialize_digest",
        deserialize_with = "deserialize_digest"
    )]
    digest: ByteDigest,
}

impl SnapshotRef {
    /// Select exact expected bytes after checking the supplied native labels.
    pub fn new(identity: SourceIdentity, digest: ByteDigest) -> Result<Self, Box<InputError>> {
        super::construction::reference_identity(identity).map(|identity| Self { identity, digest })
    }

    /// Exact selected identity/revision labels.
    pub fn identity(&self) -> &SourceIdentity {
        &self.identity
    }

    /// Expected complete artifact digest.
    pub const fn digest(&self) -> ByteDigest {
        self.digest
    }

    pub(super) fn from_artifact(identity: SourceIdentity, digest: ByteDigest) -> Self {
        Self { identity, digest }
    }
}

/// Expected invocation identity and byte digest, distinct from a snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationRef {
    #[serde(
        serialize_with = "serialize_identity",
        deserialize_with = "deserialize_identity"
    )]
    identity: SourceIdentity,
    #[serde(
        serialize_with = "serialize_digest",
        deserialize_with = "deserialize_digest"
    )]
    digest: ByteDigest,
}

impl InvocationRef {
    /// Select exact expected bytes after checking the supplied native labels.
    pub fn new(identity: SourceIdentity, digest: ByteDigest) -> Result<Self, Box<InputError>> {
        super::construction::reference_identity(identity).map(|identity| Self { identity, digest })
    }

    /// Exact selected identity/revision labels.
    pub fn identity(&self) -> &SourceIdentity {
        &self.identity
    }

    /// Expected complete artifact digest.
    pub const fn digest(&self) -> ByteDigest {
        self.digest
    }

    pub(super) fn from_artifact(identity: SourceIdentity, digest: ByteDigest) -> Self {
        Self { identity, digest }
    }
}

/// Caller-lowered structural construction ceilings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactLimits {
    /// Separate text-content and encoded-content ceilings, each at most 1 MiB.
    pub artifact_bytes: usize,
    /// Arena node count, at most 100,000.
    pub nodes: usize,
    /// Aggregate metadata and child entries, at most 100,000.
    pub entries: usize,
    /// Maximum structural value depth, at most 64.
    pub depth: usize,
}

impl Default for ArtifactLimits {
    fn default() -> Self {
        Self {
            artifact_bytes: 1_048_576,
            nodes: 100_000,
            entries: 100_000,
            depth: 64,
        }
    }
}

impl ArtifactLimits {
    pub(super) fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            artifact_bytes: self.artifact_bytes.min(hard.artifact_bytes),
            nodes: self.nodes.min(hard.nodes),
            entries: self.entries.min(hard.entries),
            depth: self.depth.min(hard.depth),
        }
    }
}

/// Actual construction counters, distinct from validation or reference fuel.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ArtifactUsage {
    /// Successfully emitted content bytes.
    pub artifact_bytes: usize,
    /// Admitted input-string UTF-8 bytes.
    pub text_bytes: usize,
    /// Arena nodes admitted for structural inspection.
    pub nodes: usize,
    /// Admitted metadata/child entries, including duplicates.
    pub entries: usize,
    /// Greatest admitted value depth.
    pub depth: usize,
}

/// Typed location component within a caller's original flat draft.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DraftPathSegment {
    /// Exact input field; names are static schema fields, not parsed expressions.
    Field(&'static str),
    /// Original numeric vector position.
    Index(usize),
}

/// Construction refusal with draft provenance, without fabricated source spans.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct InputError {
    /// Stable native code.
    pub code: Code,
    /// Original supplied labels, moved into the error even when oversized.
    pub identity: SourceIdentity,
    /// Original draft path; empty denotes the whole emitted artifact.
    pub path: Vec<DraftPathSegment>,
    /// Work admitted before the stop.
    pub usage: ArtifactUsage,
    /// Bounded explanation; this is not parsed to recover structured fields.
    pub message: &'static str,
}

impl InputError {
    /// Whether a construction ceiling prevented completion.
    pub fn is_incomplete(&self) -> bool {
        self.code == Code::ResourceExhausted
    }
}

pub(super) fn serialize_identity<S: Serializer>(
    identity: &SourceIdentity,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Labels<'a> {
        identity: &'a str,
        revision: &'a str,
    }
    Labels {
        identity: &identity.identity,
        revision: &identity.revision,
    }
    .serialize(serializer)
}

fn serialize_digest<S: Serializer>(digest: &ByteDigest, serializer: S) -> Result<S::Ok, S::Error> {
    // native-state-input/1 specifies raw lowercase hex; ByteDigest's general
    // display additionally identifies its algorithm. This stays a byte domain.
    let display = digest.to_string();
    serializer.serialize_str(display.trim_start_matches("sha256:"))
}

fn deserialize_digest<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ByteDigest, D::Error> {
    let hex = String::deserialize(deserializer)?;
    format!("sha256:{hex}").parse().map_err(D::Error::custom)
}

pub(super) fn deserialize_identity<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<SourceIdentity, D::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Labels {
        identity: String,
        revision: String,
    }
    let labels: Labels = deserialize_object(deserializer)?;
    super::construction::reference_identity(SourceIdentity {
        identity: labels.identity,
        revision: labels.revision,
    })
    .map_err(D::Error::custom)
}

fn deserialize_requirement<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<ir::RequirementRef, D::Error> {
    // The upstream value's general Deserialize surface is permissive about keys.
    // Decode this closed native wire shape, then use its existing constructors.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Reference {
        package: String,
        requirement: String,
        revision: u64,
    }
    let value: Reference = deserialize_object(deserializer)?;
    ir::RequirementRef::parse(&value.package, &value.requirement, value.revision)
        .map_err(D::Error::custom)
}

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    // A present null is valid; omission of the invocation result field is not.
    Option::deserialize(deserializer)
}

fn deserialize_symbol<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<ir::SymbolName, D::Error> {
    ir::SymbolName::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
}
