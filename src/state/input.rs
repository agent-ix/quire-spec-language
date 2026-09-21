// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-049: immutable typed state inputs and closed outcomes.

use std::sync::Arc;

use crate::protocol_artifact::{wire, ProtocolNumber};

/// Required observation contract semantic revision.
pub const OBSERVATION_CONTRACT_REVISION: &str = "782c1ce39a197cd52b8b35b50adf2e5e3ecedd0f";

/// Exact selection of one declaration-local value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationRequest {
    /// Index of the compiled declaration to evaluate.
    pub declaration: u32,
    /// Handle of the selected value expression within that declaration.
    pub value: wire::Handle,
}

/// Concrete observation occurrence and its compiled anchor role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationKey {
    /// Handle of the compiled anchor this observation occurrence is bound to.
    pub anchor: wire::Handle,
    /// F-owned identity of the observation snapshot.
    pub snapshot: ObservationIdentity,
    /// F-owned identity of the observation window, absent when the anchor role uses none.
    pub window: Option<ObservationIdentity>,
    /// F-owned identity of the observation record.
    pub record: ObservationIdentity,
}

/// Opaque F-owned identity, deliberately distinct from compiled ArtifactRef.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationIdentity(pub String);

/// Complete object storage identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectKey {
    /// Observation occurrence this object was read under.
    pub observation: ObservationKey,
    /// Index into the package's `models` table for the object's compiled model.
    pub model: u32,
    /// Exported universe (population domain) the object belongs to.
    pub universe: wire::ExportRef,
    /// Exported object type of this object.
    pub object_type: wire::ExportRef,
    /// Caller-supplied unique identifier of the object within its universe.
    pub identifier: String,
}

/// Producer canonical digest tuple, at the authority's static selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalDigest {
    /// Digest algorithm identifier, e.g. `sha256`.
    pub algorithm: String,
    /// Canonicalization domain the digest was computed over, e.g. `filament-canonical-json-1`.
    pub domain: String,
    /// Computed digest value, in the algorithm's own encoding.
    pub value: String,
}

/// F-owned digest spelling, kept distinct from Producer canonical and compiled-byte digests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationDigest(pub String);

/// Static document, model, profile and configuration identity and digest
/// required by composed evaluation.
///
/// Each identity is paired with a `sha256` digest in the
/// `filament-canonical-json-1` domain, checked by `valid_digest` in
/// `crate::state::evaluation`. `StaticAuthority` is a plain Rust struct the
/// caller builds directly from those checked identities and digests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticAuthority {
    /// Identity of the selected document.
    pub document_identity: String,
    /// Canonical digest of the selected document.
    pub document_digest: CanonicalDigest,
    /// Identity of the selected model.
    pub model_identity: String,
    /// Canonical digest of the selected model.
    pub model_digest: CanonicalDigest,
    /// Identity of the selected profile.
    pub profile_identity: String,
    /// Canonical digest of the selected profile.
    pub profile_digest: CanonicalDigest,
    /// Identity of the selected configuration.
    pub configuration_identity: String,
    /// Canonical digest of the selected configuration.
    pub configuration_digest: CanonicalDigest,
}

/// Assessment-owned finite population and observation selections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssessmentAuthority {
    /// F-owned identity of the selected finite population.
    pub population_identity: String,
    /// F-owned digest of the population's membership evidence.
    pub membership_digest: ObservationDigest,
    /// Whether the supplied membership evidence is asserted complete.
    pub membership_complete: bool,
    /// F-owned identity of the selected observation snapshot.
    pub snapshot_identity: String,
    /// F-owned digest of the observation snapshot.
    pub snapshot_digest: ObservationDigest,
    /// F-owned identity of the selected observation window, absent when none applies.
    pub window_identity: Option<String>,
    /// F-owned digest of the observation window, absent when none applies.
    pub window_digest: Option<ObservationDigest>,
    /// F-owned identity of the selected closure evidence.
    pub closure_identity: String,
    /// F-owned digest of the closure evidence.
    pub closure_digest: ObservationDigest,
}

/// Independently selected producer/observation inputs and compiled requirement authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorityEvidence {
    /// Observation contract revision this evidence was compiled against.
    pub observation_contract_revision: String,
    /// Reference to the selected producer artifact.
    pub producer: wire::ArtifactRef,
    /// Reference to the selected observation artifact.
    pub observation: wire::ArtifactRef,
    /// Reference to the compiled requirement artifact.
    pub compiled: wire::ArtifactRef,
    /// Explicit adapter grounding the otherwise distinct authority domains, when supplied.
    pub adapter: Option<AuthorityAdapter>,
    /// Static document, model, profile and configuration selection.
    pub static_selection: StaticAuthority,
    /// Assessment-owned population and observation selection.
    pub assessment_selection: AssessmentAuthority,
}

/// Explicit caller-selected mapping binding the otherwise distinct authority domains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorityAdapter {
    /// Reference to the independently published adapter artifact itself.
    pub artifact: wire::ArtifactRef,
    /// Reference to the compiled requirement artifact the adapter grounds.
    pub compiled: wire::ArtifactRef,
    /// Reference to the requirement artifact the adapter grounds.
    pub requirement: wire::ArtifactRef,
    /// Reference to the producer artifact the adapter grounds.
    pub producer: wire::ArtifactRef,
    /// Reference to the observation artifact the adapter grounds.
    pub observation: wire::ArtifactRef,
    /// Observation contract revision the adapter was published against.
    pub observation_contract_revision: String,
    /// Static authority selection the adapter grounds.
    pub static_selection: StaticAuthority,
    /// Assessment authority selection the adapter grounds.
    pub assessment_selection: AssessmentAuthority,
}

/// Exact reason that a typed runtime position has no semantic value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MissingInput {
    /// The observation occurrence named by this key was not supplied.
    Observation(ObservationKey),
    /// A field of a supplied object was not itself supplied.
    Field {
        /// Storage identity of the object the missing field belongs to.
        object: ObjectKey,
        /// Exported field on that object with no supplied value.
        field: wire::ExportRef,
    },
    /// A member nested inside a supplied binder or population value root was not supplied.
    Member {
        /// Handle of the binder or population root the missing member is nested under.
        requirement: wire::Handle,
        /// Complete authored path from that root down to the missing member.
        path: Vec<ValuePathSegment>,
    },
    /// A population's membership evidence was not supplied, naming the population's own binding.
    Membership(wire::Handle),
    /// A sequence-typed binder's value was not supplied, naming the binder's handle.
    Sequence(wire::Handle),
    /// A population-kind binder's value was not supplied, naming the binder's handle.
    Population(wire::Handle),
    /// A population's closure evidence was not supplied, naming the closure's own binding.
    Closure(wire::Handle),
}

/// One exact authored step from a supplied binder or population value root.
///
/// The complete path makes nested unavailable members injective: an inner
/// member index cannot be detached from its enclosing field, option or member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValuePathSegment {
    /// Step into a named field.
    Field {
        /// Storage identity the field step is nested under, absent when the enclosing
        /// value is a contextual record rather than a stored object.
        object: Option<ObjectKey>,
        /// The field entered at this step.
        field: wire::ExportRef,
    },
    /// Step into a supplied option's wrapped value.
    OptionValue,
    /// Step into a sequence element at this zero-based index.
    Member(usize),
}

/// One available value node or an unavailable typed position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputSlot {
    /// A value was supplied at this position.
    Available(Value),
    /// No value was supplied at this position, for the given exact reason.
    Unavailable(MissingInput),
}

/// One available contextual model-field payload or its exact unavailable cause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextualSlot {
    /// A contextual field payload was supplied at this position.
    Available(ContextualValue),
    /// No contextual field payload was supplied at this position, for the given exact reason.
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
    /// A boolean field value.
    Boolean(bool),
    /// A numeric field value.
    Number(ProtocolNumber),
    /// A text field value.
    Text(String),
    /// An enum field value, naming its exact exported member.
    Enum(wire::ExportRef),
    /// A nested record field value, as its own ordered fields.
    Record(Vec<FieldInput>),
    /// An optional field value, present or absent.
    Option(Option<Box<ContextualSlot>>),
    /// A sequence field value, as an ordered list of element slots.
    Sequence(Vec<ContextualSlot>),
    /// A reference field value naming the referenced object's storage identity.
    Reference(ObjectKey),
    /// A nested object field value naming its storage identity.
    Object(ObjectKey),
}

/// Either an ordinary compiled-index value or a model-field-context value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldValue {
    /// The field's value, typed against an ordinary compiled type index.
    Compiled(InputSlot),
    /// The field's value, typed against its enclosing model-field export instead
    /// of a compiled type index.
    Contextual(ContextualSlot),
}

/// One record/object field, preserving authored occurrence order and duplicates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldInput {
    /// The exported field this value belongs to.
    pub field: wire::ExportRef,
    /// The field's supplied value.
    pub value: FieldValue,
}

/// Exact semantic value, including nominal wire type index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Value {
    /// Index into the package's `types` table naming this value's nominal wire type.
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
    /// A boolean value.
    Boolean(bool),
    /// A numeric value.
    Number(ProtocolNumber),
    /// A text value.
    Text(String),
    /// An enum value, naming its exact exported member.
    Enum(wire::ExportRef),
    /// A record value, as its own ordered fields.
    Record(Vec<FieldInput>),
    /// An optional value, present or absent.
    Option(Option<Box<InputSlot>>),
    /// A sequence value, as an ordered list of element slots.
    Sequence(Vec<InputSlot>),
    /// A reference value naming the referenced object's storage identity.
    Reference(ObjectKey),
    /// A nested object value naming its storage identity.
    Object(ObjectKey),
}

/// One exact declaration-local binder input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinderInput {
    /// Handle of the compiled binder this input is supplied for.
    pub binder: wire::Handle,
    /// Index into the declaration's runtime-requirement table this binder's authority
    /// is checked against, absent when the binder has none.
    pub requirement: Option<u32>,
    /// Authority evidence backing this binder's value, when the binder has a runtime requirement.
    pub authority: Option<AuthorityEvidence>,
    /// The binder's supplied value, or the exact reason it is unavailable.
    pub value: InputSlot,
}

/// One object payload in a finite population.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectInput {
    /// Complete storage identity of this object.
    pub key: ObjectKey,
    /// The object's field payloads, in authored occurrence order.
    pub fields: Vec<FieldInput>,
}

/// Finite population plus independently selected membership and closure premises.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationInput {
    /// Handle of the runtime requirement this population satisfies.
    pub requirement: wire::Handle,
    /// Handle of the runtime requirement for this population's closure evidence.
    pub closure_requirement: wire::Handle,
    /// Authority evidence backing this population's selection.
    pub authority: AuthorityEvidence,
    /// Whether membership evidence was supplied, or the exact reason it is missing.
    pub membership: Result<(), MissingInput>,
    /// Whether closure evidence was supplied, or the exact reason it is missing.
    pub closure: Result<(), MissingInput>,
    /// The population's finite member objects.
    pub objects: Vec<ObjectInput>,
}

/// Caller-owned immutable state view.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StateView {
    /// Every declaration-local binder input the caller supplied.
    pub binders: Vec<BinderInput>,
    /// Every finite population the caller supplied.
    pub populations: Vec<PopulationInput>,
}

/// Closed defensive or input-admission refusal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// Version-2 compiler output has not been independently published and read.
    UnpublishedArtifact,
    /// The evaluation request names a declaration that does not exist in the compiled package.
    RequestDeclaration(u32),
    /// The evaluation request names a value that does not exist within its declaration.
    RequestValue(wire::Handle),
    /// A handle names a declaration other than the one that actually owns it.
    Owner(wire::Handle),
    /// The same binder or population was supplied more than once.
    DuplicateBinding(wire::Handle),
    /// A binder or population required by the evaluated declaration was not supplied.
    MissingBinding(wire::Handle),
    /// A supplied binder or population is not required by the evaluated declaration.
    SurplusBinding(wire::Handle),
    /// A graph value names the right nominal domain under an observation
    /// occurrence for which no selected population was supplied.
    PopulationDomain(ObjectKey),
    /// Two supplied objects in the same population share the same storage identity.
    DuplicateObject(ObjectKey),
    /// The supplied authority evidence's observation contract revision or identities
    /// do not check out.
    AuthorityRevision,
    /// The supplied authority evidence does not match what the named binding requires.
    Authority(wire::Handle),
    /// The supplied authority adapter does not ground the offered authority evidence.
    UngroundedAuthorityMapping(wire::Handle),
    /// A value's compiled type index does not match the position's expected type.
    Type {
        /// Index into `types` for the type the position required.
        expected: u32,
        /// Index into `types` for the type the supplied value actually carries.
        actual: u32,
    },
    /// A value's shape does not match its compiled type index.
    ValueShape(u32),
    /// A supplied field is not exported by its object's or record's compiled type.
    Field(wire::ExportRef),
    /// A supplied contextual field payload does not match its enclosing field
    /// export's declared shape.
    ContextualValue(wire::ExportRef),
    /// A numeric value falls outside its compiled type's authored bounds.
    Bounds(u32),
    /// A reference names an object that no supplied population provides.
    Dangling(ObjectKey),
    /// The compiled package violates an invariant the runtime assumes without reverifying.
    AdmittedInvariant(wire::Handle),
}

/// Exactly one terminal evaluation disposition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvaluationOutcome {
    /// Evaluation produced a complete value.
    Completed(Value),
    /// Evaluation stopped because a required input was not supplied.
    Incomplete(MissingInput),
    /// Evaluation stopped because the request or its inputs were invalid.
    Refused(Refusal),
    /// Evaluation stopped because a resource limit was reached.
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
    /// The effective limits this evaluation was bounded by.
    pub fn limits(&self) -> super::Limits {
        self.limits
    }
    /// The cumulative work this evaluation actually charged.
    pub fn usage(&self) -> super::Usage {
        self.usage
    }
    /// The unique terminal disposition this evaluation reached.
    pub fn outcome(&self) -> &EvaluationOutcome {
        &self.outcome
    }
    /// The accounting scheme version these limits and usage counters are interpreted under.
    pub fn accounting_version(&self) -> &'static str {
        super::ACCOUNTING_VERSION
    }
}
