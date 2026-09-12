// SPDX-License-Identifier: AGPL-3.0-only
//! FR-049: immutable typed state inputs and closed outcomes.

use std::sync::Arc;

use crate::protocol_artifact::{wire, ProtocolNumber};

/// Required Producer interface semantic revision.
pub const PRODUCER_CONTRACT_REVISION: &str = "6259d3a5b99088740df9bcc8e8d60f3720aaa603";
/// Required observation contract semantic revision.
pub const OBSERVATION_CONTRACT_REVISION: &str = "4d6230eb8aa9766ff3017360962f2d6368d74cb3";

/// Exact selection of one declaration-local value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationRequest {
    pub declaration: u32,
    pub value: wire::Handle,
}

/// Concrete observation occurrence and its compiled anchor role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationKey {
    pub anchor: wire::Handle,
    pub snapshot: ObservationIdentity,
    pub window: Option<ObservationIdentity>,
    pub record: ObservationIdentity,
}

/// Opaque F-owned identity, deliberately distinct from compiled ArtifactRef.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationIdentity(pub String);

/// Complete object storage identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectKey {
    pub observation: ObservationKey,
    pub model: u32,
    pub universe: wire::ExportRef,
    pub object_type: wire::ExportRef,
    pub identifier: String,
}

/// Producer canonical digest tuple from the selected Producer 1.2 interface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalDigest {
    pub algorithm: String,
    pub domain: String,
    pub value: String,
}

/// F-owned digest spelling, kept distinct from Producer canonical and compiled-byte digests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationDigest(pub String);

/// Producer-owned static identities required by composed evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticAuthority {
    pub interface_version: String,
    pub document_identity: String,
    pub document_digest: CanonicalDigest,
    pub model_identity: String,
    pub model_digest: CanonicalDigest,
    pub profile_identity: String,
    pub profile_digest: CanonicalDigest,
    pub configuration_identity: String,
    pub configuration_digest: CanonicalDigest,
}

/// Assessment-owned finite population and observation selections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssessmentAuthority {
    pub population_identity: String,
    pub membership_digest: ObservationDigest,
    pub membership_complete: bool,
    pub snapshot_identity: String,
    pub snapshot_digest: ObservationDigest,
    pub window_identity: Option<String>,
    pub window_digest: Option<ObservationDigest>,
    pub closure_identity: String,
    pub closure_digest: ObservationDigest,
}

/// Independently selected producer/observation inputs and compiled requirement authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorityEvidence {
    pub producer_contract_revision: String,
    pub observation_contract_revision: String,
    pub producer: wire::ArtifactRef,
    pub observation: wire::ArtifactRef,
    pub compiled: wire::ArtifactRef,
    pub adapter: Option<AuthorityAdapter>,
    pub static_selection: StaticAuthority,
    pub assessment_selection: AssessmentAuthority,
}

/// Explicit caller-selected mapping binding the otherwise distinct authority domains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorityAdapter {
    pub artifact: wire::ArtifactRef,
    pub compiled: wire::ArtifactRef,
    pub requirement: wire::ArtifactRef,
    pub producer: wire::ArtifactRef,
    pub observation: wire::ArtifactRef,
    pub producer_contract_revision: String,
    pub observation_contract_revision: String,
    pub static_selection: StaticAuthority,
    pub assessment_selection: AssessmentAuthority,
}

/// Exact reason that a typed runtime position has no semantic value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MissingInput {
    Observation(ObservationKey),
    Field {
        object: ObjectKey,
        field: wire::ExportRef,
    },
    Member {
        requirement: wire::Handle,
        index: usize,
    },
    Membership(wire::Handle),
    Sequence(wire::Handle),
    Population(wire::Handle),
    Closure(wire::Handle),
}

/// One available value node or an unavailable typed position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputSlot {
    Available(Value),
    Unavailable(MissingInput),
}

/// One available contextual model-field payload or its exact unavailable cause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextualSlot {
    Available(ContextualValue),
    Unavailable(MissingInput),
}

/// A field payload whose nominal type is supplied by its enclosing field export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextualValue {
    kind: Arc<ContextualValueKind>,
}

impl ContextualValue {
    /// Construct an immutable contextual field node without a synthetic wire index.
    pub fn new(kind: ContextualValueKind) -> Self {
        Self {
            kind: Arc::new(kind),
        }
    }

    /// Exact semantic shape of this immutable contextual node.
    pub fn kind(&self) -> &ContextualValueKind {
        &self.kind
    }
}

/// Closed recursively typed field vocabulary interpreted under admitted model metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextualValueKind {
    Boolean(bool),
    Number(ProtocolNumber),
    Text(String),
    Enum(wire::ExportRef),
    Record(Vec<FieldInput>),
    Option(Option<Box<ContextualSlot>>),
    Sequence(Vec<ContextualSlot>),
    Reference(ObjectKey),
    Object(ObjectKey),
}

/// Either an ordinary compiled-index value or a model-field-context value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldValue {
    Compiled(InputSlot),
    Contextual(ContextualSlot),
}

/// One record/object field, preserving authored occurrence order and duplicates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldInput {
    pub field: wire::ExportRef,
    pub value: FieldValue,
}

/// Exact semantic value, including nominal wire type index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Value {
    pub value_type: u32,
    kind: Arc<ValueKind>,
}

impl Value {
    /// Construct one immutable value node; clones share its recursively immutable payload.
    pub fn new(value_type: u32, kind: ValueKind) -> Self {
        Self {
            value_type,
            kind: Arc::new(kind),
        }
    }

    /// Exact semantic shape of this immutable node.
    pub fn kind(&self) -> &ValueKind {
        &self.kind
    }
}

/// Closed composed value vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueKind {
    Boolean(bool),
    Number(ProtocolNumber),
    Text(String),
    Enum(wire::ExportRef),
    Record(Vec<FieldInput>),
    Option(Option<Box<InputSlot>>),
    Sequence(Vec<InputSlot>),
    Reference(ObjectKey),
    Object(ObjectKey),
}

/// One exact declaration-local binder input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinderInput {
    pub binder: wire::Handle,
    pub requirement: Option<u32>,
    pub authority: Option<AuthorityEvidence>,
    pub value: InputSlot,
}

/// One object payload in a finite population.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectInput {
    pub key: ObjectKey,
    pub fields: Vec<FieldInput>,
}

/// Finite population plus independently selected membership and closure premises.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationInput {
    pub requirement: wire::Handle,
    pub closure_requirement: wire::Handle,
    pub authority: AuthorityEvidence,
    pub membership: Result<(), MissingInput>,
    pub closure: Result<(), MissingInput>,
    pub objects: Vec<ObjectInput>,
}

/// Caller-owned immutable state view.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StateView {
    pub binders: Vec<BinderInput>,
    pub populations: Vec<PopulationInput>,
}

/// Closed defensive or input-admission refusal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Refusal {
    RequestDeclaration(u32),
    RequestValue(wire::Handle),
    Owner(wire::Handle),
    DuplicateBinding(wire::Handle),
    MissingBinding(wire::Handle),
    SurplusBinding(wire::Handle),
    /// A graph value names the right nominal domain under an observation
    /// occurrence for which no selected population was supplied.
    PopulationDomain(ObjectKey),
    DuplicateObject(ObjectKey),
    AuthorityRevision,
    Authority(wire::Handle),
    UngroundedAuthorityMapping(wire::Handle),
    Type {
        expected: u32,
        actual: u32,
    },
    ValueShape(u32),
    Field(wire::ExportRef),
    ContextualValue(wire::ExportRef),
    Bounds(u32),
    Dangling(ObjectKey),
    AdmittedInvariant(wire::Handle),
}

/// Exactly one terminal evaluation disposition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvaluationOutcome {
    Completed(Value),
    Incomplete(MissingInput),
    Refused(Refusal),
    Exhausted(super::Exhaustion),
}

/// Effective limits, successful usage and the unique terminal outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationReport {
    pub(super) limits: super::Limits,
    pub(super) usage: super::Usage,
    pub(super) outcome: EvaluationOutcome,
}

impl EvaluationReport {
    pub fn limits(&self) -> super::Limits {
        self.limits
    }
    pub fn usage(&self) -> super::Usage {
        self.usage
    }
    pub fn outcome(&self) -> &EvaluationOutcome {
        &self.outcome
    }
    pub fn accounting_version(&self) -> &'static str {
        super::ACCOUNTING_VERSION
    }
}
