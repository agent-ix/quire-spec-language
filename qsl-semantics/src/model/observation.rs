// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106: admit `quire.state.snapshot/v1` and `quire.state.invocation/v1`
//! documents into one typed [`AdmittedObservations`] value for S6a (FR-107),
//! running the eleven ordered admission checks FR-106's Behavior section
//! states, in order, stopping at the first failing condition.
//!
//! # Design note: re-deriving the domain package's effective view
//!
//! `check::CheckedGraph` resolves a state clause's context and operation
//! frame only as opaque [`EffectiveId`]s (ADR-013 O-05): the raw producer
//! identity strings, population declarations and operation effect frames
//! that admission needs to validate a document against are not retained on
//! the checked package once S3 finishes (they are `check`-internal,
//! consumed by `Lowering` and discarded). This module (which the model ->
//! check import edge must stay empty, FR-074-AC-3, so it never names that
//! type directly) re-admits and re-normalizes each of the caller's own
//! `check::CheckedGraph::model_selections()`'s domain packages, passed in
//! as [`ClauseFacts`] and `&[DomainPackageRef]`, through the same
//! [`crate::model::intake`]/[`crate::model::normalize`]
//! pipeline `crate::model::intake::admit_unit` already runs at compile time
//! (the caller supplies the same package input FR-056 already requires),
//! and reads population/type/field identities from the resulting
//! [`EffectiveView`]s. This is not a second, divergent source of truth: it
//! is the same admission and normalization `compile` already performed,
//! re-run against the same bytes, so it always succeeds for a package that
//! already compiled -- a re-admission failure is therefore an
//! [`InternalFault`], never a document defect.

use std::collections::BTreeMap;

use qsl_forms::StateClauseKind;
use qsl_foundation::diagnostic::{
    BudgetDenial, CatalogCoded, Code, InternalFault, LimitExceeded, ALLOCATION_FAILED,
};
use qsl_foundation::Setting;
use quire_exact::{EffectiveId, Identifier, ObjectId, ObjectReference, UniverseId, Value};

use crate::model::accounting::ModelNormalizationLimits;
use crate::model::domain_package::{DomainPackage, DomainPackageRef, OperationEffect};
use crate::model::intake::{admit_selections, read_records, DigestMismatch};
use crate::model::key::{hex, DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use crate::model::normalize::{normalize, EffectiveView, NormalizeOutcome};
use crate::model::object_environment::ObjectEnvironment;
use crate::model::operation::{OperationDeclaration, OperationLookup, OperationTable};
use quire_semantic_value::declaration::TypeEnvironment;

// ---------------------------------------------------------------------------
// Clause facts (the model -> check edge must stay empty, FR-074-AC-3): the
// plain model-level facts a caller reads off its own `check::
// CheckedStateClause`/`check::CheckedGraph` and passes in here, so this
// module depends on no `check` type. `qsl-replay/src/spine/clause.rs`
// builds these from the checked package it already holds.
// ---------------------------------------------------------------------------

/// The one operation a precondition's or postcondition's clause names
/// (`check::ClauseOperation`'s own two model-level fields, copied out by
/// the caller).
pub struct OperationFacts {
    /// The object type that declares the operation.
    pub declaring: EffectiveId,
    /// The operation itself.
    pub declaration: OperationDeclaration,
}

/// The facts [`admit_observations`] needs about the selected state clause
/// (FR-104's own `check::CheckedStateClause`, read by the caller before
/// this call: `qsl-replay/src/spine/clause.rs` builds this from
/// `CheckedGraph::state_clause`).
pub struct ClauseFacts {
    /// The clause's own minted node identity (FR-107's own lookup key).
    pub identity: quire_exact::NodeKey,
    /// Invariant, precondition or postcondition.
    pub kind: StateClauseKind,
    /// The context object type, by its effective identity.
    pub context: EffectiveId,
    /// The operation a precondition or postcondition names; `None` for an
    /// invariant.
    pub operation: Option<OperationFacts>,
    /// The package's declared operations. An invariant of a state machine
    /// selected by [`ClauseSelectionInput::Context`] names no operation, and
    /// the invocation's own is resolved in the machine it names (FR-106
    /// check 5).
    pub operations: OperationTable,
    /// The domain-package populations the clause names (FR-391), by
    /// declaration identity, each with the observation it reads them at:
    /// FR-106 requires each complete in that observation's snapshot.
    /// `check::CheckedStateClause::named_reads` gives them.
    pub named_populations: Vec<(String, crate::model::clause_observation::Observation)>,
}

// ---------------------------------------------------------------------------
// Limits
// ---------------------------------------------------------------------------

/// FR-106 Inputs: the document-shape ceilings the read enforces, each named
/// by its setting (FR-255). No ceiling bounds nesting depth: a document of
/// any depth is read under its byte, object and value limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationLimits {
    /// The document byte ceiling, `observation.input_bytes` (default 1 MiB).
    pub document_bytes: u64,
    /// The objects-per-document ceiling, `observation.objects` (default
    /// 10,000).
    pub objects_per_document: u64,
    /// The values-per-document ceiling, `observation.values` (default
    /// 100,000): every value form of the document counts, nested ones
    /// included.
    pub values_per_document: u64,
    /// How an out-of-range post-state integer is admitted (default
    /// [`PostStateRange::Refuse`]).
    pub post_state: PostStateRange,
}

/// FR-106 check 6.5's post-state rule: inputs are refused, outputs are
/// evidence. A pre-state value and an operation argument outside the
/// declared `Int[lower, upper]` are always refused `invalid_runtime_input`/
/// `invalid-value`. A postcondition invocation's post snapshot is what the
/// subject produced, so an integer outside its declared range there can be
/// admitted exactly, as an [`OutOfRange`] observation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PostStateRange {
    /// Refuse the post snapshot's out-of-range integer, as for any input.
    #[default]
    Refuse,
    /// Admit it exactly, never coerced or clamped, and report it in
    /// [`Observation::out_of_range`].
    Witness,
}

/// One integer a post snapshot holds outside its declared `Int[lower,
/// upper]` range: the witness of a violation of the operation's contract
/// (FR-106 check 6.5). The value is the exact one the document holds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutOfRange {
    /// The object holding the field.
    pub object: ObjectReference,
    /// The field: its declaring object type and declared name.
    pub field: quire_semantic_value::declaration::FieldRef,
    /// The position of the offending element when the field is a
    /// sequence, `None` for a scalar field.
    pub index: Option<usize>,
    /// The field's declared range.
    pub range: quire_exact::IntegerInterval,
    /// The value observed.
    pub observed: quire_exact::Integer,
    /// For an integer inside a record value, the record field names from the
    /// field (or sequence element) down to it, outermost first; empty
    /// otherwise.
    pub steps: Vec<PathStep>,
}

impl Default for ObservationLimits {
    fn default() -> Self {
        Self {
            document_bytes: 1_048_576,
            objects_per_document: 10_000,
            values_per_document: 100_000,
            post_state: PostStateRange::Refuse,
        }
    }
}

impl ObservationLimits {
    /// These limits with `observation.input_bytes` set to `bound`.
    #[must_use]
    pub const fn with_document_bytes(mut self, bound: u64) -> Self {
        self.document_bytes = bound;
        self
    }

    /// These limits with `observation.objects` set to `bound`.
    #[must_use]
    pub const fn with_objects_per_document(mut self, bound: u64) -> Self {
        self.objects_per_document = bound;
        self
    }

    /// These limits with `observation.values` set to `bound`.
    #[must_use]
    pub const fn with_values_per_document(mut self, bound: u64) -> Self {
        self.values_per_document = bound;
        self
    }
}

/// FR-255: the one mapping from each field to its setting.
impl qsl_foundation::SettingLimits for ObservationLimits {
    fn bounds(&self) -> Vec<(qsl_foundation::Setting, u64)> {
        use qsl_foundation::Setting;
        vec![
            (Setting::ObservationInputBytes, self.document_bytes),
            (Setting::ObservationObjects, self.objects_per_document),
            (Setting::ObservationValues, self.values_per_document),
        ]
    }

    fn set_bound(&mut self, setting: qsl_foundation::Setting, bound: u64) -> bool {
        use qsl_foundation::Setting;
        match setting {
            Setting::ObservationInputBytes => self.document_bytes = bound,
            Setting::ObservationObjects => self.objects_per_document = bound,
            Setting::ObservationValues => self.values_per_document = bound,
            _ => return false,
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Document identity and selection
// ---------------------------------------------------------------------------

/// FR-001's four source labels, naming one admitted document, plus its
/// `sha256-jcs` digest (FR-106's `DocumentRef`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentRef {
    /// The `authority` label.
    pub authority: String,
    /// The `identity` label.
    pub identity: String,
    /// The `revision_namespace` label.
    pub revision_namespace: String,
    /// The `revision` label.
    pub revision: String,
    /// The document's `sha256-jcs` digest.
    pub digest: [u8; 32],
}

/// The anchor an invariant's current snapshot names: `{kind, name}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedAnchor {
    /// `initialization` or `handler`.
    pub kind: AnchorKind,
    /// The named initialization or handler.
    pub name: String,
}

/// The anchor kind (FR-106 "Document forms").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnchorKind {
    /// `initialization`.
    Initialization,
    /// `handler`.
    Handler,
}

impl AnchorKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Initialization => "initialization",
            Self::Handler => "handler",
        }
    }
}

/// An object reference by population and key (FR-106's `ObjectRef`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedObject {
    /// The population identity.
    pub population: String,
    /// The object's declared key.
    pub key: String,
}

/// FR-106's `ClauseSelection`: names the clause by its `QualifiedName` and
/// one observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseSelection {
    /// The selected clause's declared name.
    pub name: String,
    /// The selected observation input.
    pub input: ClauseSelectionInput,
}

/// [`ClauseSelection`]'s observation half.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClauseSelectionInput {
    /// An invariant's current snapshot, anchor and self object.
    Current {
        /// The current snapshot document.
        snapshot: DocumentRef,
        /// The selected anchor.
        anchor: SelectedAnchor,
        /// The selected self object: none for a clause whose receiver is a
        /// state machine's context, which has no object identity.
        self_object: Option<SelectedObject>,
    },
    /// A precondition's pre-call observation: the operation's pre state,
    /// the self object and the parameter values, before the operation
    /// runs. It has no post snapshot, result or delta.
    PreCall {
        /// The pre snapshot document.
        snapshot: DocumentRef,
        /// The selected self object: none for a clause whose receiver is a
        /// state machine's context.
        self_object: Option<SelectedObject>,
        /// Each declared parameter's value, by parameter name.
        parameters: BTreeMap<Identifier, SnapshotValue>,
    },
    /// A precondition's or postcondition's invocation document.
    Invocation {
        /// The invocation document.
        invocation: DocumentRef,
    },
    /// The invariant of a state machine over one side of an invocation of
    /// one of its operations: the machine's context in the invocation's `pre`
    /// or its `post` snapshot (QSpec FR-208-AC-10).
    Context {
        /// The invocation document.
        invocation: DocumentRef,
        /// Which of the invocation's snapshots the invariant is read in.
        side: ContextSide,
    },
}

/// The snapshot of an invocation a [`ClauseSelectionInput::Context`] reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextSide {
    /// The invocation's `pre` snapshot.
    Pre,
    /// The invocation's `post` snapshot.
    Post,
}

impl ClauseSelectionInput {
    /// Which of the four observation forms this input is.
    pub fn form(&self) -> ObservationForm {
        match self {
            Self::Current { .. } => ObservationForm::Current,
            Self::PreCall { .. } => ObservationForm::PreCall,
            Self::Invocation { .. } => ObservationForm::Invocation,
            Self::Context { .. } => ObservationForm::Context,
        }
    }
}

/// The form of a [`ClauseSelectionInput`], without its documents.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationForm {
    /// A current snapshot, its anchor and the self object.
    Current,
    /// A pre snapshot, the self object and the parameters.
    PreCall,
    /// An invocation document.
    Invocation,
    /// One side of an invocation document, for a state machine's invariant.
    Context,
}

impl std::fmt::Display for ObservationForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Current => "current",
            Self::PreCall => "pre-call",
            Self::Invocation => "invocation",
            Self::Context => "context",
        })
    }
}

// ---------------------------------------------------------------------------
// Admission failure
// ---------------------------------------------------------------------------

/// One admission record: a catalog code and cause, and the input path
/// (document identity, population, object key, field) it names (FR-106
/// Outputs). Distinct from [`qsl_foundation::diagnostic::RefusalRecord`],
/// whose `Locus` names a *source* position: an admission defect names a
/// position in a *document*, which no `Locus` variant represents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionRecord {
    /// The catalog code, e.g. [`Code::InvalidRuntimeInput`]: typed, so a
    /// caller maps it (for example to an exit code) with no string lookup.
    pub code: Code,
    /// The catalog cause, e.g. `"wrong-role-mapping"`.
    pub cause: &'static str,
    /// The named input path components this record carries, keyed by name
    /// (`document`, `population`, `object`, `field`, `label`, ...).
    pub fields: BTreeMap<&'static str, String>,
    /// For a defect inside a nested value, the typed steps from the checked
    /// field, parameter or result down to it, outermost first (QSpec FR-207);
    /// empty when the defect is at the checked value itself.
    pub path: Vec<PathStep>,
    /// For a native UUID or Timestamp payload the kernel's canonical
    /// constructor refused (FR-106 native order, step 7), the kernel's own
    /// class beneath `invalid-value`; `None` for every other refusal.
    pub kernel: Option<quire_exact::ConstructionCause>,
}

/// One typed step of a nested value path (QSpec FR-207): never a rendered
/// string, so a record field named `0` cannot be taken for the first
/// element of a sequence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathStep {
    /// A record field, by its exact name.
    Field(String),
    /// The payload of a present optional value.
    OptionValue,
    /// A sequence element, by its position.
    Index(usize),
}

impl AdmissionRecord {
    fn new(code: Code, cause: &'static str) -> Self {
        Self {
            code,
            cause,
            fields: BTreeMap::new(),
            path: Vec::new(),
            kernel: None,
        }
    }

    #[must_use]
    fn with(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.fields.insert(key, value.into());
        self
    }

    /// This record naming the nested value `path`: the typed steps from the
    /// checked field, parameter or result down to it (FR-106 check 6.5).
    #[must_use]
    fn at(mut self, path: Vec<PathStep>) -> Self {
        self.path = path;
        self
    }

    /// This record carrying the kernel class beneath it.
    #[must_use]
    fn kernel_class(mut self, class: quire_exact::ConstructionCause) -> Self {
        self.kernel = Some(class);
        self
    }
}

/// FR-106 Outputs: admission's non-`Ok` result.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AdmissionFailure {
    /// A real input defect: no `AdmittedObservations` and no truth value.
    #[error("admission refused: {}/{}", .0.code.as_str(), .0.cause)]
    Refused(AdmissionRecord),
    /// Missing evidence, or an unresolved object/subtype closure: no
    /// `AdmittedObservations`, distinct from a refusal (FR-106 checks 1.1,
    /// 7).
    #[error("admission incomplete: {}/{}", .0.code.as_str(), .0.cause)]
    Incomplete(AdmissionRecord),
    /// A broken invariant of admission itself (never a document defect):
    /// the re-supplied domain packages do not re-normalize the way the
    /// already-compiled package did.
    #[error("internal fault in admission: {}: {}", .0.stage(), .0.invariant())]
    Fault(InternalFault),
}

fn refuse(record: AdmissionRecord) -> AdmissionFailure {
    AdmissionFailure::Refused(record)
}

fn incomplete(record: AdmissionRecord) -> AdmissionFailure {
    AdmissionFailure::Incomplete(record)
}

fn fault(invariant: &'static str) -> AdmissionFailure {
    AdmissionFailure::Fault(InternalFault::new("observation-admission", invariant))
}

// ---------------------------------------------------------------------------
// Admitted output
// ---------------------------------------------------------------------------

/// One admitted observation instant (current, pre or post): its document
/// identity and an [`ObjectEnvironment`] holding every field value of every
/// admitted object of that instant, plus which populations were admitted
/// `complete`.
#[derive(Clone, Debug)]
pub struct Observation {
    /// The document's identity and digest.
    pub identity: DocumentRef,
    /// Every field value of every object this instant admitted.
    pub environment: ObjectEnvironment,
    /// The populations this instant admitted, each with whether it was
    /// declared `complete`.
    pub populations: BTreeMap<String, bool>,
    /// Every integer this instant holds outside its declared range, in
    /// walk order: empty unless this is a post instant admitted under
    /// [`PostStateRange::Witness`].
    pub out_of_range: Vec<OutOfRange>,
}

/// The amount [`ObservationLimits`]' three document-shape ceilings actually
/// consumed, across every document one [`admit_observations`] or
/// [`admit_current_snapshot`] call successfully read (FR-109 Outputs'
/// "the admission work", distinct from `quire_exact::LimitKind`, which
/// counts `quire-exact`'s own value-family scalar charges: admission has
/// no analogue of `integer_bits`/`work_units`, so reusing that enum would
/// misreport kinds admission never touches, SR-751 FND-002 round 2).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AdmissionUsage {
    /// The sum of every admitted document's byte length.
    pub document_bytes: u64,
    /// The sum of every admitted document's object count.
    pub objects: u64,
    /// The sum of every admitted document's value-form count, nested forms
    /// included.
    pub values: u64,
}

impl AdmissionUsage {
    fn merged_with(self, other: Self) -> Self {
        Self {
            document_bytes: self.document_bytes + other.document_bytes,
            objects: self.objects + other.objects,
            values: self.values + other.values,
        }
    }
}

/// What `self` is in an admitted observation set (FR-107 "The receiver path").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelfKind {
    /// An object with a declared object identity (QSpec FR-204).
    Object,
    /// A state machine's context, with no declared object identity.
    Context,
}

impl SelfKind {
    fn of(state_machine: bool) -> Self {
        if state_machine {
            Self::Context
        } else {
            Self::Object
        }
    }
}

/// FR-106 Outputs: one typed observation set admitted for S6a.
#[derive(Clone, Debug)]
pub struct AdmittedObservations {
    /// FR-109 Outputs' admission usage, summed over every document this
    /// call actually read (`current` alone, or `pre` and/or `post`).
    pub usage: AdmissionUsage,
    /// The node identity of the state clause these observations were
    /// admitted for (FR-107's `ObservationsMismatch` check).
    pub clause: quire_exact::NodeKey,
    /// The observation an invariant is evaluated in: a current snapshot, or
    /// for a state machine's invariant selected by `Context` the snapshot of
    /// the selected side, holding the machine's one context.
    pub current: Option<Observation>,
    /// The pre observation: an invocation's pre snapshot, or a pre-call
    /// observation's one snapshot.
    pub pre: Option<Observation>,
    /// The post observation: present for an invocation selected for a
    /// postcondition; absent for a current observation and for a pre-call
    /// observation, whether selected by `PreCall` or by `Invocation`.
    pub post: Option<Observation>,
    /// The selected self object's reference.
    pub self_object: ObjectReference,
    /// The selected invocation, whatever kind `self` is: some for a
    /// postcondition and for a precondition selected by `Invocation`, none
    /// for an invariant and for a precondition selected by `PreCall`.
    /// Evaluators root receiver, parameter and result value paths at it.
    pub invocation: Option<DocumentRef>,
    /// Whether `self` is an object with a declared object identity or a
    /// state machine's context, which has none (QSpec FR-208).
    pub self_kind: SelfKind,
    /// The pre-observation receiver context's evaluator reference, when
    /// this is a state-machine invocation.
    pub receiver_pre: Option<ObjectReference>,
    /// The post-observation receiver context's evaluator reference, when
    /// this is a state-machine invocation with a post observation.
    pub receiver_post: Option<ObjectReference>,
    /// The parameter values of an invocation or a pre-call observation, in
    /// declared order (empty for an invariant).
    pub parameters: Vec<(String, Value)>,
    /// The invocation's result value, or `None` for an operation with no
    /// result (empty for an invariant).
    pub result: Option<Value>,
    /// The invocation's admitted `created` object references.
    pub created: Vec<ObjectReference>,
    /// The invocation's admitted `deleted` object references.
    pub deleted: Vec<ObjectReference>,
}

// ---------------------------------------------------------------------------
// Raw value grammar (FR-106 "Document forms")
// ---------------------------------------------------------------------------

/// One value in FR-106's snapshot value form ("Document forms"): an object
/// field's value in a snapshot, a parameter or result of an invocation, or
/// a parameter of a `PreCall` selection. An integer keeps its FR-038
/// decimal spelling; admission checks it against the declared type.
///
/// A value of any depth is held in one flat arena, so building, cloning,
/// comparing, printing and dropping it never recurse with its nesting
/// (FR-261, ADR-030): [`Self::form`] views the root, and each nested value
/// is reached through a [`RawValue`] handle.
#[derive(Clone, Debug)]
pub struct SnapshotValue {
    nodes: Vec<RawNode>,
    root: usize,
}

/// One value form of a [`SnapshotValue`]'s arena; a nested value is the
/// index of its own node.
#[derive(Clone, Debug, Eq, PartialEq)]
enum RawNode {
    Boolean(bool),
    Integer(String),
    /// A value object with a `kind` member, kept as read so admission judges
    /// it in the native payload order.
    Native(NativeObject),
    Absent,
    Present(usize),
    Reference(SelectedObject),
    Sequence(Vec<usize>),
    /// `{"record": {"type": .., "fields": {..}}}`: the record's `type`
    /// identity and its fields in document order, each the index of its
    /// value's node.
    Record {
        type_identity: String,
        fields: Vec<(String, usize)>,
    },
    /// `{"variant": {"type": .., "case": ..}}` as read: each of the two
    /// members when present, and whether the payload holds any other member
    /// (FR-360 item 9 judges those defects in its own order).
    Variant {
        type_identity: Option<String>,
        case: Option<String>,
        extra_member: bool,
    },
}

/// The form of one value of a [`SnapshotValue`], with nested values as
/// handles.
#[derive(Clone, Copy, Debug)]
pub enum RawForm<'v> {
    /// `{"boolean": true|false}`.
    Boolean(bool),
    /// `{"integer": "<decimal>"}`.
    Integer(&'v str),
    /// A value object with a `kind` member: `{"kind": "string", "text": ..}`,
    /// `{"kind": "uuid", "text": ..}` or `{"kind": "timestamp",
    /// "nanoseconds": ..}`, or a defective one, as read.
    Native(RawNative<'v>),
    /// `{"absent": {}}`.
    Absent,
    /// `{"present": <value>}`.
    Present(RawValue<'v>),
    /// `{"reference": {"population": ..., "key": ...}}`.
    Reference(&'v SelectedObject),
    /// `{"sequence": [<value>, ...]}`, its items in order.
    Sequence(RawItems<'v>),
    /// `{"record": {"type": <declaration identity>, "fields": {..}}}`.
    Record(RawRecord<'v>),
    /// `{"variant": {"type": <declaration identity>, "case": <case name>}}`.
    Variant(RawVariant<'v>),
}

/// A `variant` value as read: the members of its payload, whichever it
/// holds (FR-360 item 9).
#[derive(Clone, Copy, Debug)]
pub struct RawVariant<'v> {
    type_identity: Option<&'v str>,
    case: Option<&'v str>,
    extra_member: bool,
}

impl<'v> RawVariant<'v> {
    /// The `type` member: the IR node identity of the variant type, if the
    /// payload has one.
    #[must_use]
    pub fn type_identity(self) -> Option<&'v str> {
        self.type_identity
    }

    /// The `case` member: the exact declared case name, if the payload has
    /// one.
    #[must_use]
    pub fn case(self) -> Option<&'v str> {
        self.case
    }

    /// Whether the payload holds a member other than `type` and `case`.
    #[must_use]
    pub fn has_extra_member(self) -> bool {
        self.extra_member
    }
}

/// The kind of a native payload value (FR-056's K2 canonical native
/// payloads).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeKind {
    /// `"string"`.
    String,
    /// `"uuid"`.
    Uuid,
    /// `"timestamp"`.
    Timestamp,
    /// `"text"`: admitted only into a `Text[min, max; profile]` slot; every
    /// other slot refuses it `wrong-value-kind` (FR-106-AC-35).
    Text,
}

/// A native payload object's `kind` member as read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RawNativeKind {
    /// Not a JSON string.
    NotText,
    /// A string that names no native kind.
    Unknown,
    /// A native kind.
    Known(NativeKind),
}

/// A native payload object as read, outside its `kind` member: every other
/// member in document order, its text or `None` where it is not a JSON
/// string.
#[derive(Clone, Debug, Eq, PartialEq)]
struct NativeObject {
    kind: RawNativeKind,
    members: Vec<(String, Option<String>)>,
}

/// A native payload object as read (see [`RawForm::Native`]).
#[derive(Clone, Copy, Debug)]
pub struct RawNative<'v> {
    kind: RawNativeKind,
    members: &'v [(String, Option<String>)],
}

impl<'v> RawNative<'v> {
    /// The `kind` member.
    #[must_use]
    pub fn kind(self) -> RawNativeKind {
        self.kind
    }

    /// Every member other than `kind`, in document order: its name and its
    /// text, `None` where it is not a JSON string.
    pub fn members(self) -> impl Iterator<Item = (&'v str, Option<&'v str>)> {
        self.members
            .iter()
            .map(|(name, text)| (name.as_str(), text.as_deref()))
    }
}

/// One value inside a [`SnapshotValue`].
#[derive(Clone, Copy, Debug)]
pub struct RawValue<'v> {
    arena: &'v SnapshotValue,
    index: usize,
}

/// A `record` value: the identity of its most-specific record type and its
/// fields in document order.
#[derive(Clone, Copy, Debug)]
pub struct RawRecord<'v> {
    arena: &'v SnapshotValue,
    type_identity: &'v str,
    fields: &'v [(String, usize)],
}

impl<'v> RawRecord<'v> {
    /// The `type` member: the IR node identity of the value's most-specific
    /// record value type.
    #[must_use]
    pub fn type_identity(self) -> &'v str {
        self.type_identity
    }

    /// The `fields` members, in document order.
    pub fn fields(
        self,
    ) -> impl DoubleEndedIterator<Item = (&'v str, RawValue<'v>)> + ExactSizeIterator {
        self.fields.iter().map(move |(name, index)| {
            (
                name.as_str(),
                RawValue {
                    arena: self.arena,
                    index: *index,
                },
            )
        })
    }
}

/// The items of a `sequence` value, in order.
#[derive(Clone, Copy, Debug)]
pub struct RawItems<'v> {
    arena: &'v SnapshotValue,
    items: &'v [usize],
}

impl SnapshotValue {
    /// A value over an arena of nodes the reader built, rooted at `root`.
    fn from_arena(nodes: Vec<RawNode>, root: usize) -> Self {
        Self { nodes, root }
    }

    fn single(node: RawNode) -> Self {
        Self {
            nodes: vec![node],
            root: 0,
        }
    }

    /// `{"boolean": value}`.
    #[must_use]
    pub fn boolean(value: bool) -> Self {
        Self::single(RawNode::Boolean(value))
    }

    /// `{"integer": "<spelling>"}`.
    #[must_use]
    pub fn integer(spelling: impl Into<String>) -> Self {
        Self::single(RawNode::Integer(spelling.into()))
    }

    /// A native payload object: its `kind` and every other member in
    /// document order, each with its text or `None` where it is not a JSON
    /// string. Admission judges it in the native payload order, so a defective
    /// object is held as read.
    #[must_use]
    pub fn native(kind: RawNativeKind, members: Vec<(String, Option<String>)>) -> Self {
        Self::single(RawNode::Native(NativeObject { kind, members }))
    }

    /// `{"absent": {}}`.
    #[must_use]
    pub fn absent() -> Self {
        Self::single(RawNode::Absent)
    }

    /// `{"reference": {"population": ..., "key": ...}}`.
    #[must_use]
    pub fn reference(reference: SelectedObject) -> Self {
        Self::single(RawNode::Reference(reference))
    }

    /// `{"present": inner}`.
    #[must_use]
    pub fn present(inner: Self) -> Self {
        let mut nodes = inner.nodes;
        let at = nodes.len();
        nodes.push(RawNode::Present(inner.root));
        Self { nodes, root: at }
    }

    /// `{"sequence": [items...]}`.
    #[must_use]
    pub fn sequence(items: Vec<Self>) -> Self {
        let mut nodes =
            Vec::with_capacity(items.iter().map(|item| item.nodes.len()).sum::<usize>() + 1);
        let mut roots = Vec::with_capacity(items.len());
        for item in items {
            let offset = nodes.len();
            nodes.extend(item.nodes.into_iter().map(|node| node.shifted(offset)));
            roots.push(item.root + offset);
        }
        let at = nodes.len();
        nodes.push(RawNode::Sequence(roots));
        Self { nodes, root: at }
    }

    /// `{"record": {"type": type_identity, "fields": {..}}}`, its fields in
    /// the given order.
    #[must_use]
    pub fn record(type_identity: impl Into<String>, fields: Vec<(String, Self)>) -> Self {
        let mut nodes = Vec::new();
        let mut members = Vec::with_capacity(fields.len());
        for (name, value) in fields {
            let offset = nodes.len();
            nodes.extend(value.nodes.into_iter().map(|node| node.shifted(offset)));
            members.push((name, value.root + offset));
        }
        let at = nodes.len();
        nodes.push(RawNode::Record {
            type_identity: type_identity.into(),
            fields: members,
        });
        Self { nodes, root: at }
    }

    /// `{"variant": {"type": type_identity, "case": case}}`.
    #[must_use]
    pub fn variant(type_identity: impl Into<String>, case: impl Into<String>) -> Self {
        Self::single(RawNode::Variant {
            type_identity: Some(type_identity.into()),
            case: Some(case.into()),
            extra_member: false,
        })
    }

    /// A `variant` value read from a payload that may lack `type` or `case`
    /// or hold another member, which admission refuses in FR-360 item 9's
    /// order.
    #[must_use]
    pub fn variant_payload(
        type_identity: Option<String>,
        case: Option<String>,
        extra_member: bool,
    ) -> Self {
        Self::single(RawNode::Variant {
            type_identity,
            case,
            extra_member,
        })
    }

    /// The root value's form.
    #[must_use]
    pub fn form(&self) -> RawForm<'_> {
        self.at(self.root)
    }

    fn at(&self, index: usize) -> RawForm<'_> {
        match &self.nodes[index] {
            RawNode::Boolean(value) => RawForm::Boolean(*value),
            RawNode::Integer(spelling) => RawForm::Integer(spelling),
            RawNode::Native(native) => RawForm::Native(RawNative {
                kind: native.kind,
                members: &native.members,
            }),
            RawNode::Absent => RawForm::Absent,
            RawNode::Present(inner) => RawForm::Present(RawValue {
                arena: self,
                index: *inner,
            }),
            RawNode::Reference(reference) => RawForm::Reference(reference),
            RawNode::Sequence(items) => RawForm::Sequence(RawItems { arena: self, items }),
            RawNode::Record {
                type_identity,
                fields,
            } => RawForm::Record(RawRecord {
                arena: self,
                type_identity,
                fields,
            }),
            RawNode::Variant {
                type_identity,
                case,
                extra_member,
            } => RawForm::Variant(RawVariant {
                type_identity: type_identity.as_deref(),
                case: case.as_deref(),
                extra_member: *extra_member,
            }),
        }
    }
}

/// Structural equality, compared on an explicit heap stack so a value of any
/// depth compares in constant native stack, whatever order its arena was
/// built in.
impl PartialEq for SnapshotValue {
    fn eq(&self, other: &Self) -> bool {
        self.equals_by(other, None, &|_, _| None)
    }
}

/// The retained sequence of a Text payload under `declared`'s profile
/// (FR-141): the payload itself, or its normalization.
#[qsl_attrs::string_edge]
pub(super) fn retained_text(declared: &quire_exact::TextType, payload: &str) -> String {
    match declared.profile().normalization() {
        Some(form) => form.apply(payload).collect(),
        None => payload.to_owned(),
    }
}

/// A Text object's `text` member, if it is a string.
#[qsl_attrs::string_edge]
fn text_member(native: RawNative<'_>) -> Option<&str> {
    native
        .members()
        .find_map(|(name, text)| (name == "text").then_some(text).flatten())
}

impl SnapshotValue {
    /// Whether the two values are equal, a value of a declared Text type
    /// comparing by its retained sequence (FR-106-AC-40, FR-141): `declared`
    /// is this value's slot type, `None` where it is not known. Every other
    /// form compares as read. `field_type` gives the declared type of a
    /// record's field by the record's type identity and the field's name, so
    /// a Text inside a record compares by its retained sequence too.
    pub(super) fn equals_by<'t>(
        &self,
        other: &Self,
        declared: Option<&'t quire_exact::ValueType>,
        field_type: &dyn Fn(&str, &str) -> Option<&'t quire_exact::ValueType>,
    ) -> bool {
        use quire_exact::ValueType;
        let mut work = vec![(declared, self.form(), other.form())];
        while let Some((declared, left, right)) = work.pop() {
            match (left, right) {
                (RawForm::Boolean(left), RawForm::Boolean(right)) if left == right => {}
                (RawForm::Integer(left), RawForm::Integer(right)) if left == right => {}
                (RawForm::Native(left), RawForm::Native(right)) => {
                    let equal = match (declared, left.kind, right.kind) {
                        (
                            Some(ValueType::Text(text_type)),
                            RawNativeKind::Known(NativeKind::Text),
                            RawNativeKind::Known(NativeKind::Text),
                        ) => match (text_member(left), text_member(right)) {
                            (Some(left), Some(right)) => {
                                retained_text(text_type, left) == retained_text(text_type, right)
                            }
                            _ => false,
                        },
                        _ => left.kind == right.kind && left.members == right.members,
                    };
                    if !equal {
                        return false;
                    }
                }
                (RawForm::Absent, RawForm::Absent) => {}
                (RawForm::Reference(left), RawForm::Reference(right)) if left == right => {}
                (RawForm::Present(left), RawForm::Present(right)) => {
                    // A field's declared type is its bare type: `Option` only
                    // wraps an operation parameter or result.
                    let inner = match declared {
                        Some(ValueType::Option(payload)) => Some(&**payload),
                        other => other,
                    };
                    work.push((inner, left.form(), right.form()));
                }
                (RawForm::Sequence(left), RawForm::Sequence(right))
                    if left.iter().len() == right.iter().len() =>
                {
                    let element = match declared {
                        Some(ValueType::Collection(collection)) => Some(collection.element()),
                        _ => None,
                    };
                    work.extend(
                        left.iter()
                            .zip(right.iter())
                            .map(|(left, right)| (element, left.form(), right.form())),
                    );
                }
                (RawForm::Record(left_record), RawForm::Record(right_record))
                    if left_record.type_identity() == right_record.type_identity()
                        && left_record.fields().len() == right_record.fields().len() =>
                {
                    for ((left_name, left), (right_name, right)) in
                        left_record.fields().zip(right_record.fields())
                    {
                        if left_name != right_name {
                            return false;
                        }
                        work.push((
                            field_type(left_record.type_identity(), left_name),
                            left.form(),
                            right.form(),
                        ));
                    }
                }
                (RawForm::Variant(left), RawForm::Variant(right))
                    if left.type_identity() == right.type_identity()
                        && left.case() == right.case()
                        && left.has_extra_member() == right.has_extra_member() => {}
                _ => return false,
            }
        }
        true
    }
}

impl Eq for SnapshotValue {}

impl RawNode {
    /// This node with every nested index moved up by `offset`.
    fn shifted(self, offset: usize) -> Self {
        match self {
            Self::Present(inner) => Self::Present(inner + offset),
            Self::Sequence(items) => {
                Self::Sequence(items.into_iter().map(|item| item + offset).collect())
            }
            Self::Record {
                type_identity,
                fields,
            } => Self::Record {
                type_identity,
                fields: fields
                    .into_iter()
                    .map(|(name, index)| (name, index + offset))
                    .collect(),
            },
            other @ (Self::Boolean(_)
            | Self::Integer(_)
            | Self::Native(_)
            | Self::Absent
            | Self::Reference(_)
            | Self::Variant { .. }) => other,
        }
    }
}

impl<'v> RawValue<'v> {
    /// This value's form.
    #[must_use]
    pub fn form(self) -> RawForm<'v> {
        self.arena.at(self.index)
    }
}

impl<'v> RawItems<'v> {
    /// The items, in order.
    pub fn iter(self) -> impl DoubleEndedIterator<Item = RawValue<'v>> + ExactSizeIterator {
        self.items.iter().map(move |index| RawValue {
            arena: self.arena,
            index: *index,
        })
    }
}

// ---------------------------------------------------------------------------
// Re-derived model view
// ---------------------------------------------------------------------------

/// One re-normalized domain package's effective view, with the reverse
/// `EffectiveId -> DeclarationKey` index admission needs (the forward
/// direction, [`EffectiveView::type_identities`], is a checked package's own
/// input, never admission's own name-first lookup).
#[derive(Debug)]
pub(crate) struct ModelView {
    pub(crate) view: EffectiveView,
    by_effective_id: BTreeMap<EffectiveId, DeclarationKey>,
}

impl ModelView {
    /// The view with its reverse type-identity index.
    pub(crate) fn new(view: EffectiveView) -> Self {
        let by_effective_id = view
            .type_identities()
            .iter()
            .map(|(key, effective)| (*effective, key.clone()))
            .collect();
        Self {
            view,
            by_effective_id,
        }
    }

    fn type_name(&self, effective: EffectiveId) -> Option<&str> {
        self.by_effective_id
            .get(&effective)
            .map(|key| key.node.as_str())
    }

    pub(crate) fn declaration_key(&self, effective: EffectiveId) -> Option<&DeclarationKey> {
        self.by_effective_id.get(&effective)
    }
}

/// Re-admit and re-normalize every domain package `model_selections`
/// names (the caller's own `CheckedGraph::model_selections()`), against the
/// same package input a caller's `compile` already used. See the module
/// doc's design note.
pub(crate) fn model_views(
    model_selections: &[DomainPackageRef],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: ModelNormalizationLimits,
) -> Result<Vec<ModelView>, AdmissionFailure> {
    let admitted = admit_selections(
        model_selections,
        SHA256_JCS_DIGEST_DOMAIN,
        packages,
        limits.intake,
    )
    .map_err(|_| fault("model-reconsistent-admission"))?;
    let mut views = Vec::with_capacity(admitted.len());
    for (package_ref, document) in admitted {
        let records = read_records(&package_ref.identity, &document)
            .map_err(|_| fault("model-reconsistent-records"))?;
        let view = match normalize(&DomainPackage::new(package_ref, records), limits) {
            NormalizeOutcome::Completed(view) => view,
            // A reached ceiling of `ModelNormalizationLimits` is the caller's
            // budget wherever it is read (ADR-014 section 1), so it refuses
            // as the compile path does, naming its setting; any other
            // refusal is a broken invariant, since the same packages already
            // normalized at compile time.
            NormalizeOutcome::Incomplete(incomplete) => {
                return Err(caller_budget_refusal(
                    &incomplete.denial(),
                    INSUFFICIENT_NEXT_CHARGE,
                ))
            }
            NormalizeOutcome::Refused(refusals) => {
                return Err(refusals
                    .iter()
                    .find_map(|refusal| {
                        let denial = refusal.cause.denial()?;
                        Some(caller_budget_refusal(
                            &denial,
                            refusal.cause.catalog_code().cause(),
                        ))
                    })
                    .unwrap_or_else(|| fault("model-reconsistent-normalize")))
            }
        };
        views.push(ModelView::new(view));
    }
    Ok(views)
}

/// The one [`ModelView`] whose effective view names `effective`, and the
/// [`DeclarationKey`] it resolves to. `None` when no re-derived view names
/// it -- a broken invariant, since `effective` came from the same checked
/// package these views were re-derived from.
pub(crate) fn view_of(views: &[ModelView], effective: EffectiveId) -> Option<&ModelView> {
    views
        .iter()
        .find(|view| view.declaration_key(effective).is_some())
}

// ---------------------------------------------------------------------------
// sha256-jcs digest (FR-056)
// ---------------------------------------------------------------------------

/// FR-106's digest-first rule, checked against `expected` over `read`, the
/// one read of `bytes` through `quire-canonical`'s shared reader: bytes the
/// reader read are digested over their RFC 8785 canonical encoding, the
/// reader's tree encoded by `quire-canonical` directly -- the one sanctioned
/// encoder (ADR-013 §2); bytes the reader refuses are digested raw (FR-106
/// check 1.3). Observation never refuses evidence as malformed. The raw
/// fallback itself is `model::intake::check_package_digest` (widened, to
/// serve this second caller) -- never an ad hoc `ByteDigest::of` here, and
/// never a second call site of `model::key::raw_bytes_digest` outside that
/// already-exempt function (ADR-013 §2 O-05: one RFC 8785 encoder, one
/// raw-fallback call site).
///
/// The read ran under `limits.document_bytes`, and the encoding runs under
/// the same bound, so a byte error from either is check 1.2's
/// `stage_limit_exceeded`/`input-bytes-exceeded` naming
/// `observation.input_bytes` (FR-259 B4). A read or encoding that cannot
/// reserve memory refuses `resource_exhausted`/`allocation-failed` carrying
/// `requested` (FR-259 B6).
///
/// A document the reader reads that holds a number with no exact RFC 8785
/// spelling refuses `noncanonical_wire` (`inexact-integer` or
/// `inexact-number`) with the `document_pointer` of the first such number,
/// before the digest is compared (FR-056's rule, FR-106 check 1.3). A number
/// with no finite double is the reader's own refusal, classified from its
/// lexeme: it is named when the reader reaches it, so ahead of an earlier
/// inexact number, and the first reader fault decides when the bytes carry
/// several (`{"a":1,"a":2,"n":1e400}` and `[1e400` refuse it, while
/// `{"a":1,"a":2,"n":1e-400}` is a repeated name and digests raw). The rule
/// reads every number of the document, including members admission reads no
/// further, such as an invocation's `post`, `result`, `created` and
/// `deleted` under a precondition.
fn check_document_digest(
    bytes: &[u8],
    read: &Result<quire_canonical::Document, quire_canonical::ReadError>,
    expected: [u8; 32],
    limits: ObservationLimits,
) -> Result<(), AdmissionFailure> {
    let parsed_digest = match read {
        Ok(document) => {
            if let Some((pointer, inexact, _)) =
                crate::model::intake::first_inexact_number(document.root())
            {
                return Err(refuse(
                    AdmissionRecord::new(Code::NoncanonicalWire, inexact.as_str())
                        .with("document_pointer", pointer.as_str()),
                ));
            }
            Some(
                *quire_canonical::sha256(
                    document,
                    quire_canonical::Limits::new(limits.document_bytes),
                )
                .map_err(|error| digest_encode_refusal(&error, limits))?
                .as_bytes(),
            )
        }
        Err(error) => match digest_read_refusal(error, limits) {
            Some(refusal) => return Err(refusal),
            None => None,
        },
    };
    crate::model::intake::check_package_digest(expected, bytes, parsed_digest).map_err(|mismatch| {
        match mismatch {
            DigestMismatch::Content { recomputed } => refuse(
                AdmissionRecord::new(Code::StaleDependency, "content-mismatch")
                    .with("selected", hex(&expected))
                    .with("recomputed", hex(&recomputed)),
            ),
            DigestMismatch::RawBytes { digest } => refuse(
                AdmissionRecord::new(Code::StaleDependency, "byte-digest-mismatch")
                    .with("selected", hex(&expected))
                    .with("actual", hex(&digest)),
            ),
        }
    })
}

/// [`check_document_digest`]'s refusal for a refusal of the shared reader,
/// or `None` when the bytes are digested raw: malformed bytes, and any
/// other refusal of the bytes themselves.
fn digest_read_refusal(
    error: &quire_canonical::ReadError,
    limits: ObservationLimits,
) -> Option<AdmissionFailure> {
    match error {
        quire_canonical::ReadError::Limit(limit) => Some(input_bytes_exceeded(
            limits.document_bytes,
            u128::from(limit.required),
        )),
        quire_canonical::ReadError::Allocation { requested } => Some(allocation_failed(*requested)),
        // A number with no finite double (`1e400`) is not malformed bytes:
        // it refuses like any other number with no exact RFC 8785 spelling,
        // classified from its lexeme.
        quire_canonical::ReadError::NumberOutOfRange {
            pointer, lexeme, ..
        } => {
            crate::model::intake::out_of_range_number(pointer, lexeme).map(|(pointer, inexact)| {
                refuse(
                    AdmissionRecord::new(Code::NoncanonicalWire, inexact.as_str())
                        .with("document_pointer", pointer.as_str()),
                )
            })
        }
        _ => None,
    }
}

/// [`check_document_digest`]'s refusal for an error of the digest's
/// encoding.
fn digest_encode_refusal(
    error: &quire_canonical::Error,
    limits: ObservationLimits,
) -> AdmissionFailure {
    match error {
        quire_canonical::Error::Limit(limit) => {
            input_bytes_exceeded(limits.document_bytes, u128::from(limit.required))
        }
        quire_canonical::Error::Allocation { requested } => allocation_failed(*requested),
        // A read tree always has an RFC 8785 encoding: every number is a
        // finite double and every member name a string.
        _ => fault("read-document-has-no-canonical-encoding"),
    }
}

/// FR-106 check 1.2's refusal: `stage_limit_exceeded`/`input-bytes-exceeded`
/// naming `observation.input_bytes`, its bound and the length reached.
fn input_bytes_exceeded(bound: u64, actual: u128) -> AdmissionFailure {
    limit_refusal(&LimitExceeded::new(
        Setting::ObservationInputBytes,
        bound,
        actual,
    ))
}

/// A reached ceiling of a caller's budget (`ModelNormalizationLimitsV1` or
/// `PopulationAdmissionLimitsV1`, ADR-014 B-2) as an admission refusal: the compile path's
/// `resource_exhausted` with its `cause`, carrying FR-255 Behavior 1's
/// record from [`BudgetDenial::catalog_fields`].
fn caller_budget_refusal(denial: &BudgetDenial, cause: &'static str) -> AdmissionFailure {
    refuse(caller_budget_record(denial, cause))
}

/// [`caller_budget_refusal`]'s record.
fn caller_budget_record(denial: &BudgetDenial, cause: &'static str) -> AdmissionRecord {
    let mut record = AdmissionRecord::new(Code::ResourceExhausted, cause);
    record.fields.extend(denial.catalog_fields());
    record
}

/// `limit` as an admission refusal: `stage_limit_exceeded` with the limit's
/// cause and its `kind`, `bound`, `actual` and `setting` fields (FR-255
/// Behavior 1).
fn limit_refusal(limit: &LimitExceeded) -> AdmissionFailure {
    let mut record = AdmissionRecord::new(Code::StageLimitExceeded, limit.kind().catalog_cause());
    if let Some(fields) = limit.catalog_fields() {
        record.fields.extend(fields);
    }
    refuse(record)
}

/// FR-261 Behavior 4: the object and value counts of one document, charged
/// as the reader's tree is walked. `values` counts every value form of the
/// document, nested ones included.
struct ReadMeter {
    limits: ObservationLimits,
    objects: u64,
    values: u64,
}

impl ReadMeter {
    fn new(limits: ObservationLimits) -> Self {
        Self {
            limits,
            objects: 0,
            values: 0,
        }
    }

    /// Charge one object: `observation.objects`.
    fn object(&mut self) -> Result<(), AdmissionFailure> {
        self.objects = self.objects.saturating_add(1);
        if self.objects > self.limits.objects_per_document {
            return Err(limit_refusal(&LimitExceeded::new(
                Setting::ObservationObjects,
                self.limits.objects_per_document,
                u128::from(self.objects),
            )));
        }
        Ok(())
    }

    /// Charge one value form: `observation.values`.
    fn value(&mut self) -> Result<(), AdmissionFailure> {
        self.values = self.values.saturating_add(1);
        if self.values > self.limits.values_per_document {
            return Err(limit_refusal(&LimitExceeded::new(
                Setting::ObservationValues,
                self.limits.values_per_document,
                u128::from(self.values),
            )));
        }
        Ok(())
    }
}

/// The catalog cause of a denied accounting charge.
const INSUFFICIENT_NEXT_CHARGE: &str = "insufficient-next-charge";

/// FR-259 B6: reading or encoding the document could not reserve
/// `requested` bytes of memory.
fn allocation_failed(requested: usize) -> AdmissionFailure {
    refuse(
        AdmissionRecord::new(Code::ResourceExhausted, ALLOCATION_FAILED.cause())
            .with("requested", requested.to_string()),
    )
}

#[cfg(test)]
mod digest_tests {
    use super::*;
    use ix_trace_rs::trace;
    use sha2::{Digest, Sha256};

    /// `check_document_digest` over one read of `bytes` under the default
    /// limits, as admission makes it.
    fn check(bytes: &[u8], expected: [u8; 32]) -> Result<(), AdmissionFailure> {
        let limits = ObservationLimits::default();
        let read = quire_canonical::read(bytes, limits.document_bytes);
        check_document_digest(bytes, &read, expected, limits)
    }

    /// FR-106 check 1.3: bytes the shared reader refuses -- a repeated
    /// member name, a lone surrogate escape -- are digested raw and
    /// never refused as malformed. A repeated-name document therefore
    /// admits under its raw digest and refuses `byte-digest-mismatch` under
    /// the RFC 8785 digest of its last-wins value.
    #[trace("TC-465", "FR-106-AC-3")]
    #[test]
    fn bytes_the_shared_reader_refuses_are_digested_raw() {
        for text in [r#"{"a":1,"a":2}"#, r#"{"s":"\ud800"}"#] {
            let raw: [u8; 32] = Sha256::digest(text.as_bytes()).into();
            assert_eq!(check(text.as_bytes(), raw), Ok(()), "{text}");
        }
        let last_wins = quire_canonical::read(br#"{"a":2}"#, u64::MAX).unwrap();
        let canonical =
            *quire_canonical::sha256(&last_wins, quire_canonical::Limits::new(u64::MAX))
                .unwrap()
                .as_bytes();
        let raw: [u8; 32] = Sha256::digest(br#"{"a":1,"a":2}"#).into();
        assert_eq!(
            check(br#"{"a":1,"a":2}"#, canonical),
            Err(refuse(
                AdmissionRecord::new(Code::StaleDependency, "byte-digest-mismatch")
                    .with("selected", hex(&canonical))
                    .with("actual", hex(&raw))
            ))
        );
    }

    /// FR-056: a number with no finite double refuses `noncanonical_wire`
    /// at its pointer under any digest, raw or canonical, never
    /// `byte-digest-mismatch`: `1e400` and `-1e400` are `inexact-integer`,
    /// `1e-400` is `inexact-number`.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn a_number_with_no_finite_double_refuses_noncanonical_wire_under_any_digest() {
        let wide = format!("1{}.5", "0".repeat(400));
        let wide_whole = format!("1{}e-1", "0".repeat(400));
        for (text, cause, pointer) in [
            (r#"{"n":1e400}"#, "inexact-integer", "/n"),
            (r#"{"a":[0,-1e400]}"#, "inexact-integer", "/a/1"),
            ("1e400", "inexact-integer", ""),
            (r#"{"a":{"n":1e-400}}"#, "inexact-number", "/a/n"),
            (r#"{"a":1,"a":2,"n":1e400}"#, "inexact-integer", "/n"),
            ("[1e400", "inexact-integer", "/0"),
            (r#"{"x":[1e-400,1e400]}"#, "inexact-integer", "/x/1"),
            (wide.as_str(), "inexact-number", ""),
            (wide_whole.as_str(), "inexact-integer", ""),
        ] {
            let raw: [u8; 32] = Sha256::digest(text.as_bytes()).into();
            for digest in [raw, [0_u8; 32]] {
                assert_eq!(
                    check(text.as_bytes(), digest),
                    Err(refuse(
                        AdmissionRecord::new(Code::NoncanonicalWire, cause)
                            .with("document_pointer", pointer)
                    )),
                    "{text}"
                );
            }
        }
    }

    /// A repeated member name the reader detects before it reaches an
    /// underflowing number is the reader's refusal: digested raw.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn a_repeated_name_before_an_underflow_is_digested_raw() {
        let text = r#"{"a":1,"a":2,"n":1e-400}"#;
        let raw: [u8; 32] = Sha256::digest(text.as_bytes()).into();
        assert_eq!(check(text.as_bytes(), raw), Ok(()));
    }

    /// FR-106 check 1.3: a document the reader reads, offered under another
    /// digest, refuses `stale_dependency`/`content-mismatch` carrying the
    /// selected digest and the digest recomputed from the document.
    #[trace("TC-465", "FR-106-AC-3")]
    #[test]
    fn a_parsed_document_under_another_digest_refuses_content_mismatch() {
        let text = br#"{"a":2}"#;
        let recomputed = *quire_canonical::sha256(
            &quire_canonical::read(text, u64::MAX).unwrap(),
            quire_canonical::Limits::new(u64::MAX),
        )
        .unwrap()
        .as_bytes();
        let selected: [u8; 32] = Sha256::digest(b"another document").into();
        assert_eq!(
            check(text, selected),
            Err(refuse(
                AdmissionRecord::new(Code::StaleDependency, "content-mismatch")
                    .with("selected", hex(&selected))
                    .with("recomputed", hex(&recomputed))
            ))
        );
    }

    /// FR-261-AC-2 (TC-733 step 2): on a 512 KiB stack, digest admission over
    /// a document holding a value nested 100,000 deep reads the document's
    /// digest under its own digest, and under another digest refuses
    /// `content-mismatch` carrying the selected and the recomputed digest.
    #[trace("TC-733", "FR-261-AC-2")]
    #[test]
    fn a_document_nested_100_000_deep_is_digested_on_content() {
        let outcome = std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let depth = 100_000;
                let text = format!("{{\"a\":{}{}}}", "[".repeat(depth), "]".repeat(depth));
                let document = quire_canonical::read(text.as_bytes(), u64::MAX).unwrap();
                let own =
                    *quire_canonical::sha256(&document, quire_canonical::Limits::new(u64::MAX))
                        .unwrap()
                        .as_bytes();
                let other: [u8; 32] = Sha256::digest(b"another document").into();
                (
                    check(text.as_bytes(), own),
                    check(text.as_bytes(), other),
                    own,
                    other,
                )
            })
            .expect("the test thread spawns")
            .join()
            .expect("the read does not overflow the stack");
        let (admitted, refused, own, other) = outcome;
        assert_eq!(admitted, Ok(()));
        assert_eq!(
            refused,
            Err(refuse(
                AdmissionRecord::new(Code::StaleDependency, "content-mismatch")
                    .with("selected", hex(&other))
                    .with("recomputed", hex(&own))
            ))
        );
    }

    /// The reader's and the encoder's allocation failures refuse
    /// `resource_exhausted`/`allocation-failed` carrying `requested`, and a
    /// byte error is check 1.2's `input-bytes-exceeded`.
    #[trace("TC-729", "FR-259-AC-5")]
    #[test]
    fn an_allocation_failure_refuses_allocation_failed() {
        let expected = refuse(
            AdmissionRecord::new(Code::ResourceExhausted, "allocation-failed")
                .with("requested", "4096"),
        );
        assert_eq!(
            digest_read_refusal(
                &quire_canonical::ReadError::Allocation { requested: 4096 },
                ObservationLimits::default()
            ),
            Some(expected.clone())
        );
        assert_eq!(
            digest_encode_refusal(
                &quire_canonical::Error::Allocation { requested: 4096 },
                ObservationLimits::default()
            ),
            expected
        );
        let over = quire_canonical::to_vec("over", quire_canonical::Limits::new(1)).unwrap_err();
        let limits = ObservationLimits::default();
        assert!(matches!(
            digest_encode_refusal(&over, limits),
            AdmissionFailure::Refused(record) if record.cause == "input-bytes-exceeded"
        ));
    }
}

// ---------------------------------------------------------------------------
// Provisions
// ---------------------------------------------------------------------------

/// FR-106's snapshot and invocation provisions: maps from `sha256-jcs`
/// digest to document bytes.
pub struct Provisions<'a> {
    /// The snapshot provision.
    pub snapshots: &'a BTreeMap<[u8; 32], Vec<u8>>,
    /// The invocation provision.
    pub invocations: &'a BTreeMap<[u8; 32], Vec<u8>>,
}

/// FR-109's `Function` selection: admit the one current snapshot's
/// populations (checks 1, 5 to 8, over the snapshot alone -- a function has
/// no clause, so checks 2 to 4, 9 to 11 do not apply) into an
/// [`ObjectEnvironment`], for the caller to resolve each object argument
/// against (FR-109's own `wrong-role-mapping` refusal on an unresolved
/// one).
pub fn admit_current_snapshot(
    model_selections: &[DomainPackageRef],
    types: &TypeEnvironment,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    model_limits: ModelNormalizationLimits,
    provision: &BTreeMap<[u8; 32], Vec<u8>>,
    selected: &DocumentRef,
    limits: ObservationLimits,
) -> Result<(ObjectEnvironment, AdmissionUsage), AdmissionFailure> {
    admit_object_environment(
        model_selections,
        types,
        packages,
        model_limits,
        provision,
        selected,
        limits,
        &[],
    )
}

/// FR-330: one snapshot of a temporal clause's trace, admitted as
/// [`admit_current_snapshot`] admits it, except that `required`, the
/// populations the clause's `holds` atoms name (FR-391), are required to be
/// complete (FR-106 check 7), repeatedly through their reference fields and
/// relationship ends: a trace over one not listed `complete` is `Incomplete`
/// `incomplete_population`/`incomplete-scope`, as a state clause's
/// observation is.
#[allow(clippy::too_many_arguments)]
pub fn admit_trace_snapshot(
    model_selections: &[DomainPackageRef],
    types: &TypeEnvironment,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    model_limits: ModelNormalizationLimits,
    provision: &BTreeMap<[u8; 32], Vec<u8>>,
    selected: &DocumentRef,
    limits: ObservationLimits,
    required: &[String],
) -> Result<(ObjectEnvironment, AdmissionUsage), AdmissionFailure> {
    admit_current_environment(
        model_selections,
        types,
        (packages, model_limits),
        (provision, selected, limits),
        &[],
        required,
    )
}

/// FR-098 step 3: a replayed function's object environment, admitted as
/// [`admit_current_snapshot`] admits it, except that the populations holding
/// an object that equals one of the converted `arguments` are required to be
/// complete (FR-106 check 7), repeatedly through their reference fields and
/// relationship ends.
#[allow(clippy::too_many_arguments)]
pub fn admit_object_environment(
    model_selections: &[DomainPackageRef],
    types: &TypeEnvironment,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    model_limits: ModelNormalizationLimits,
    provision: &BTreeMap<[u8; 32], Vec<u8>>,
    selected: &DocumentRef,
    limits: ObservationLimits,
    arguments: &[ObjectReference],
) -> Result<(ObjectEnvironment, AdmissionUsage), AdmissionFailure> {
    admit_current_environment(
        model_selections,
        types,
        (packages, model_limits),
        (provision, selected, limits),
        arguments,
        &[],
    )
}

/// One current snapshot's object environment, with the populations holding
/// `arguments` and the populations `required` required to be complete.
fn admit_current_environment(
    model_selections: &[DomainPackageRef],
    types: &TypeEnvironment,
    (packages, model_limits): (&BTreeMap<[u8; 32], Vec<u8>>, ModelNormalizationLimits),
    (provision, selected, limits): (
        &BTreeMap<[u8; 32], Vec<u8>>,
        &DocumentRef,
        ObservationLimits,
    ),
    arguments: &[ObjectReference],
    required: &[String],
) -> Result<(ObjectEnvironment, AdmissionUsage), AdmissionFailure> {
    let views = model_views(model_selections, packages, model_limits)?;
    let read = read_document(DocumentKind::Snapshot, provision, selected, limits)?;
    let snapshot = read
        .as_snapshot()
        .ok_or_else(|| fault("expected-snapshot-document"))?;
    if snapshot.observation != document::ObservationRole::Current {
        return Err(refuse(AdmissionRecord::new(
            Code::WrongSnapshot,
            "wrong-observation",
        )));
    }
    check_model_any(&views, &snapshot.model)?;
    let admitted = document::admit_argument_populations(
        &views,
        types,
        snapshot,
        &read.identity,
        arguments,
        required,
        model_limits.ancestor_steps,
    )?;
    Ok((admitted.environment, read.usage))
}

/// The `sha256-jcs` digest of the document `bytes`, the one FR-106 check 1.3
/// compares with the selected digest.
///
/// # Errors
///
/// A `Fault` when `bytes` are not a document with an RFC 8785 form.
pub fn snapshot_document_digest(bytes: &[u8]) -> Result<[u8; 32], AdmissionFailure> {
    let document = quire_canonical::read(bytes, u64::MAX)
        .map_err(|_| fault("document-bytes-read-as-a-document"))?;
    quire_canonical::sha256(&document, quire_canonical::Limits::new(u64::MAX))
        .map(|digest| *digest.as_bytes())
        .map_err(|_| fault("document-has-a-canonical-form"))
}

/// FR-109's `Function` selection: `type_identity`'s own [`UniverseId`],
/// re-deriving `views` the same way [`admit_current_snapshot`] does (the
/// module doc's design note), for the caller to resolve an object
/// argument's population into the exact same universe admission itself
/// would assign that object.
pub fn population_universe_for(
    model_selections: &[DomainPackageRef],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    model_limits: ModelNormalizationLimits,
    type_identity: EffectiveId,
) -> Result<UniverseId, AdmissionFailure> {
    let views = model_views(model_selections, packages, model_limits)?;
    population_universe(&views, type_identity)
}

/// The effective identity of the declared type `declaration` and its
/// [`UniverseId`], derived through `population_universe` exactly as
/// [`population_universe_for`] resolves an object argument. `None` when no
/// selected model declares `declaration`.
pub fn declared_type_universe_for(
    model_selections: &[DomainPackageRef],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    model_limits: ModelNormalizationLimits,
    declaration: &DeclarationKey,
) -> Result<Option<(EffectiveId, UniverseId)>, AdmissionFailure> {
    let views = model_views(model_selections, packages, model_limits)?;
    let Some(effective) = views
        .iter()
        .find_map(|view| view.view.type_identities().get(declaration).copied())
    else {
        return Ok(None);
    };
    let universe = population_universe(&views, effective)?;
    Ok(Some((effective, universe)))
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// FR-106: admit the selected snapshot and invocation documents against
/// `model_selections`/`types` (the caller's own checked graph, re-derived
/// per the module doc's design note) and `clause` (the caller's own
/// selected `check::CheckedStateClause`, read into [`ClauseFacts`] before
/// this call), into one [`AdmittedObservations`] value, or return exactly
/// one [`AdmissionFailure`].
#[allow(clippy::too_many_arguments)]
pub fn admit_observations(
    model_selections: &[DomainPackageRef],
    types: &TypeEnvironment,
    clause: &ClauseFacts,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    model_limits: ModelNormalizationLimits,
    provisions: &Provisions<'_>,
    selection: &ClauseSelection,
    limits: ObservationLimits,
) -> Result<AdmittedObservations, AdmissionFailure> {
    let wrong_observation = || {
        refuse(AdmissionRecord::new(
            Code::WrongSnapshot,
            "wrong-observation",
        ))
    };
    // Check 2: selection form.
    match (clause.kind, &selection.input) {
        (StateClauseKind::Invariant, ClauseSelectionInput::Current { .. })
        | (StateClauseKind::Invariant, ClauseSelectionInput::Context { .. })
        | (StateClauseKind::Precondition, ClauseSelectionInput::PreCall { .. })
        | (
            StateClauseKind::Precondition | StateClauseKind::Postcondition,
            ClauseSelectionInput::Invocation { .. },
        ) => {}
        (
            StateClauseKind::Precondition | StateClauseKind::Postcondition,
            ClauseSelectionInput::Current { .. } | ClauseSelectionInput::Context { .. },
        )
        | (
            StateClauseKind::Invariant | StateClauseKind::Postcondition,
            ClauseSelectionInput::PreCall { .. },
        )
        | (StateClauseKind::Invariant, ClauseSelectionInput::Invocation { .. }) => {
            return Err(wrong_observation())
        }
    }

    let views = model_views(model_selections, packages, model_limits)?;
    let context_view =
        view_of(&views, clause.context).ok_or_else(|| fault("unresolved-context-type"))?;
    let context_name = context_view
        .type_name(clause.context)
        .ok_or_else(|| fault("unresolved-context-name"))?
        .to_owned();
    let state_machine = is_state_machine_context(context_view, &context_name);

    // Check 2, by the clause's receiver: a state machine's invariant is
    // selected by `Context` alone, an object type's never is, and a
    // `Current` or `PreCall` selection names a `self` exactly when the
    // receiver is an object.
    let named_self = match &selection.input {
        ClauseSelectionInput::Current { self_object, .. }
        | ClauseSelectionInput::PreCall { self_object, .. } => Some(self_object.is_some()),
        ClauseSelectionInput::Invocation { .. } | ClauseSelectionInput::Context { .. } => None,
    };
    let context_selected = matches!(selection.input, ClauseSelectionInput::Context { .. });
    if (clause.kind == StateClauseKind::Invariant && context_selected != state_machine)
        || named_self.is_some_and(|named| named == state_machine)
    {
        return Err(wrong_observation());
    }

    // A precondition's pre-call observation, selected by `PreCall` or by
    // `Invocation`, and a state machine's invariant on side `pre`, are
    // admitted against this one context.
    // FR-391: the populations the clause names, required (FR-106 check 7)
    // with their closure in the snapshot of the observation each is read at.
    let named = NamedReads::of(&clause.named_populations);
    macro_rules! pre_call_context {
        ($operation:expr, $as_context:expr) => {
            PreCallContext {
                views: &views,
                types,
                context_view,
                context_name: &context_name,
                clause_context: clause.context,
                state_machine,
                clause: clause.identity,
                operation: $operation,
                as_context: $as_context,
                ancestor_steps: model_limits.ancestor_steps,
                named: &named.pre,
            }
        };
    }
    let clause_operation = || -> Result<OperationSource<'_>, AdmissionFailure> {
        clause
            .operation
            .as_ref()
            .map(OperationSource::Clause)
            .ok_or_else(|| fault("clause-declares-no-operation"))
    };

    let admitted = match &selection.input {
        ClauseSelectionInput::Current {
            snapshot,
            anchor,
            self_object,
        } => admit_invariant(
            &views,
            types,
            context_view,
            &context_name,
            clause.identity,
            provisions,
            snapshot,
            anchor,
            self_object
                .as_ref()
                .ok_or_else(|| fault("object-invariant-names-no-self"))?,
            &named.current,
            limits,
            model_limits.ancestor_steps,
        ),
        ClauseSelectionInput::PreCall {
            snapshot,
            self_object,
            parameters,
        } => {
            let context = pre_call_context!(clause_operation()?, false);
            let read = read_document(
                DocumentKind::Snapshot,
                provisions.snapshots,
                snapshot,
                limits,
            )?;
            let parameters: Vec<(String, SnapshotValue)> = parameters
                .iter()
                .map(|(name, value)| (name.as_str().to_owned(), value.clone()))
                .collect();
            admit_pre_call(
                &context,
                PreCallInput {
                    call: None,
                    receiver_invocation: None,
                    invocation: None,
                    invocation_usage: AdmissionUsage::default(),
                    pre: read,
                    self_object: self_object.as_ref(),
                    parameters: &parameters,
                },
            )
        }
        ClauseSelectionInput::Invocation { invocation } => match clause.kind {
            StateClauseKind::Precondition => admit_pre_call_invocation(
                &pre_call_context!(clause_operation()?, false),
                provisions,
                invocation,
                limits,
            ),
            StateClauseKind::Postcondition => admit_operation(
                &views,
                types,
                context_view,
                &context_name,
                clause.identity,
                clause.context,
                OperationSource::Clause(
                    clause
                        .operation
                        .as_ref()
                        .ok_or_else(|| fault("clause-declares-no-operation"))?,
                ),
                false,
                &named,
                provisions,
                invocation,
                limits,
                model_limits,
            ),
            // Check 2 above refused an invariant's `Invocation` selection.
            StateClauseKind::Invariant => Err(fault("invariant-selected-by-invocation")),
        },
        ClauseSelectionInput::Context { invocation, side } => match side {
            ContextSide::Pre => admit_pre_call_invocation(
                &pre_call_context!(OperationSource::Any(&clause.operations), true),
                provisions,
                invocation,
                limits,
            ),
            ContextSide::Post => admit_operation(
                &views,
                types,
                context_view,
                &context_name,
                clause.identity,
                clause.context,
                OperationSource::Any(&clause.operations),
                true,
                &named,
                provisions,
                invocation,
                limits,
                model_limits,
            ),
        },
    }?;
    Ok(admitted)
}

/// FR-391 "Evaluation": the populations a clause names, by the observation
/// each is read at. Each is required (FR-106 check 7, "required
/// population") in that observation's snapshot, and so, repeatedly, is every
/// population a reference field of its objects names: one not listed
/// `complete` is `Incomplete` `incomplete_population`/`incomplete-scope`.
#[derive(Debug, Default)]
struct NamedReads {
    current: Vec<String>,
    pre: Vec<String>,
    post: Vec<String>,
}

impl NamedReads {
    fn of(named: &[(String, crate::model::clause_observation::Observation)]) -> Self {
        use crate::model::clause_observation::Observation as Read;
        let mut reads = Self::default();
        for (population, observation) in named {
            match observation {
                Read::Current => reads.current.push(population.clone()),
                Read::Pre => reads.pre.push(population.clone()),
                Read::Post => reads.post.push(population.clone()),
            }
        }
        reads
    }
}

// The remainder of this module (document reading, the eleven checks, and
// the frame/delta check) is implemented incrementally; see
// `admit_invariant`/`admit_operation` below.

mod document;
mod encode;
mod extents;
mod frame;
mod initial;
mod relationships;
mod tree;
mod write;

pub use document::ObservationRole;
use document::{read_document, DocumentKind};
pub use encode::ValueEncoder;
pub use extents::{
    BundleKey, BundlePart, BundleRefusal, Keyed, ObservationOrigin, PopulationEnumerationFailure,
    PopulationManifest, RelationshipExtents, ResolveBundle,
};
pub(crate) use initial::{admit_initial_state, observe_state, state_fields, state_reference};
pub use write::{
    write_environment_document, EnvironmentLabels, EnvironmentModel, EnvironmentObject,
    EnvironmentPopulation,
};

#[allow(clippy::too_many_arguments)]
fn admit_invariant(
    views: &[ModelView],
    types: &TypeEnvironment,
    context_view: &ModelView,
    context_name: &str,
    clause: quire_exact::NodeKey,
    provisions: &Provisions<'_>,
    selected: &DocumentRef,
    anchor: &SelectedAnchor,
    self_object: &SelectedObject,
    named: &[String],
    limits: ObservationLimits,
    ancestor_steps: u64,
) -> Result<AdmittedObservations, AdmissionFailure> {
    let read = read_document(
        DocumentKind::Snapshot,
        provisions.snapshots,
        selected,
        limits,
    )?;
    let snapshot = read
        .as_snapshot()
        .ok_or_else(|| fault("expected-snapshot-document"))?;

    // Check 3: observation role and anchor.
    if snapshot.observation != document::ObservationRole::Current {
        return Err(refuse(AdmissionRecord::new(
            Code::WrongSnapshot,
            "wrong-observation",
        )));
    }
    let document_anchor = snapshot.anchor.as_ref().ok_or_else(|| {
        refuse(AdmissionRecord::new(
            Code::WrongSnapshot,
            "wrong-observation",
        ))
    })?;
    // `AnchorKind` derives `PartialEq`; compared directly (SR-750
    // FND-011), never through `as_str()` string equality.
    if document_anchor.kind != anchor.kind || document_anchor.name != anchor.name {
        return Err(refuse(
            AdmissionRecord::new(Code::WrongSnapshot, "wrong-anchor")
                .with(
                    "required",
                    format!("{} {}", anchor.kind.as_str(), anchor.name),
                )
                .with(
                    "supplied",
                    format!("{} {}", document_anchor.kind.as_str(), document_anchor.name),
                ),
        ));
    }

    // Check 4: model.
    check_model(context_view, &snapshot.model)?;

    // Check 6-8: populations and values, completeness, closure.
    let environment = document::admit_populations(
        views,
        types,
        snapshot,
        &read.identity,
        Some(&self_object.population),
        named,
        ancestor_steps,
    )?;

    // Check 9: self.
    let self_reference = document::resolve_self(
        views,
        types,
        context_view,
        context_name,
        &environment.environment,
        self_object,
    )?;

    Ok(AdmittedObservations {
        usage: read.usage,
        clause,
        current: Some(Observation {
            identity: read.identity,
            environment: environment.environment,
            populations: environment.completeness,
            out_of_range: environment.out_of_range,
        }),
        pre: None,
        post: None,
        self_object: self_reference,
        invocation: None,
        self_kind: SelfKind::Object,
        receiver_pre: None,
        receiver_post: None,
        parameters: Vec::new(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    })
}

/// FR-106 check 4: `model` must match `context_view`'s own package model
/// selection -- "the package's model selection for the clause's alias"
/// (`FR-106-admit-snapshots-and-invocations.md:191-193`), never merely some
/// alias of the unit (SR-750 FND-012: matching against every view in
/// `views` would wrongly admit a document bound to a different alias's
/// model in a multi-alias unit).
fn check_model(
    context_view: &ModelView,
    model: &document::ModelHeader,
) -> Result<(), AdmissionFailure> {
    let selection = &context_view.view.model_selection();
    let matches = selection.identity == model.identity
        && selection.version == model.version
        && hex(&selection.digest) == model.digest;
    if matches {
        Ok(())
    } else {
        Err(refuse(AdmissionRecord::new(
            Code::InvalidModelBinding,
            "wrong-model-selection",
        )))
    }
}

/// [`check_model`], over every view in `views`: FR-109's `Function`
/// selection (`admit_current_snapshot`) carries no clause alias of its own
/// to scope this check to -- a bare `Function` selection is not `on
/// Alias::Type::op`-shaped -- so it stays scoped to the unit's whole model
/// selection set, as before.
fn check_model_any(
    views: &[ModelView],
    model: &document::ModelHeader,
) -> Result<(), AdmissionFailure> {
    let matches = views.iter().any(|view| {
        let selection = &view.view.model_selection();
        selection.identity == model.identity
            && selection.version == model.version
            && hex(&selection.digest) == model.digest
    });
    if matches {
        Ok(())
    } else {
        Err(refuse(AdmissionRecord::new(
            Code::InvalidModelBinding,
            "wrong-model-selection",
        )))
    }
}

/// The three documents FR-106 check 1 read for one invocation: the
/// invocation, then its pre and post snapshots, in that order.
struct InvocationDocuments {
    invocation: document::ReadDocument,
    pre: document::ReadDocument,
    post: document::ReadDocument,
}

impl InvocationDocuments {
    fn invocation(&self) -> Result<&document::InvocationDocument, AdmissionFailure> {
        self.invocation
            .as_invocation()
            .ok_or_else(|| fault("expected-invocation-document"))
    }

    fn pre(&self) -> Result<&document::SnapshotDocument, AdmissionFailure> {
        self.pre
            .as_snapshot()
            .ok_or_else(|| fault("expected-snapshot-document"))
    }

    fn post(&self) -> Result<&document::SnapshotDocument, AdmissionFailure> {
        self.post
            .as_snapshot()
            .ok_or_else(|| fault("expected-snapshot-document"))
    }
}

/// What FR-106 checks 1 and 3 to 10 admit from one invocation.
struct InvocationAdmission {
    documents: InvocationDocuments,
    pre: document::AdmittedEnvironment,
    post: document::AdmittedEnvironment,
    self_reference: ObjectReference,
    invocation: DocumentRef,
    self_kind: SelfKind,
    receiver_pre: Option<ObjectReference>,
    receiver_post: Option<ObjectReference>,
    /// The operation the invocation names, resolved by check 5.
    operation: OperationDeclaration,
    parameters: Vec<(String, Value)>,
    result: Option<Value>,
}

fn is_state_machine_context(context_view: &ModelView, context_name: &str) -> bool {
    context_view
        .view
        .domain_package()
        .records
        .iter()
        .any(|record| {
            matches!(
                record,
                crate::model::domain_package::DomainPackageRecord::StateMachine(machine)
                    if machine.key.node == context_name
            )
        })
}

/// Where FR-106 check 5 takes the invocation's operation from.
#[derive(Clone, Copy)]
enum OperationSource<'a> {
    /// The operation the clause names.
    Clause(&'a OperationFacts),
    /// Any effective operation of the state machine the invocation's
    /// `context` names, which a state machine's invariant allows, naming no
    /// operation of its own (QSpec FR-208-AC-10).
    Any(&'a OperationTable),
}

/// The clause's receiver as check 5 compares an invocation with it.
struct Receiver<'a> {
    views: &'a [ModelView],
    types: &'a TypeEnvironment,
    context_name: &'a str,
    context: EffectiveId,
    state_machine: bool,
}

impl InvocationAdmission {
    /// Every document's read usage.
    fn usage(&self) -> AdmissionUsage {
        self.documents
            .invocation
            .usage
            .merged_with(self.documents.pre.usage)
            .merged_with(self.documents.post.usage)
    }
}

/// FR-106 check 9's `self` rule: an object receiver needs an object
/// reference, a state machine's context has none, so its `self` is `null`.
fn wrong_self_kind() -> AdmissionFailure {
    refuse(
        AdmissionRecord::new(Code::InvalidRuntimeInput, "wrong-value-kind").with("field", "self"),
    )
}

/// FR-106 checks 1 and 3 to 10 over the invocation `selected`, of the
/// operation `source` names, on `context_view`'s type `context_name`.
/// `self` is required in the post snapshot only when `post_self` (a
/// postcondition, SR-750 FND-006).
#[allow(clippy::too_many_arguments)]
fn admit_invocation_documents(
    views: &[ModelView],
    types: &TypeEnvironment,
    context_view: &ModelView,
    context_name: &str,
    clause_context: EffectiveId,
    source: OperationSource<'_>,
    post_self: bool,
    named: &NamedReads,
    provisions: &Provisions<'_>,
    selected: &DocumentRef,
    limits: ObservationLimits,
    ancestor_steps: u64,
) -> Result<InvocationAdmission, AdmissionFailure> {
    // Check 1: read every selected document -- the invocation, then its
    // pre and post snapshots, in that order -- before any of checks 2 to 5
    // read any of their content (SR-750 FND-005: this must not interleave
    // with check 4/5, which is what admission did before this fix).
    let invocation_read = read_document(
        DocumentKind::Invocation,
        provisions.invocations,
        selected,
        limits,
    )?;
    let (pre_ref, post_ref) = {
        let invocation = invocation_read
            .as_invocation()
            .ok_or_else(|| fault("expected-invocation-document"))?;
        (invocation.call.pre.clone(), invocation.post.clone())
    };
    let pre_read = read_document(
        DocumentKind::Snapshot,
        provisions.snapshots,
        &pre_ref,
        limits,
    )?;
    let post_read = read_document(
        DocumentKind::Snapshot,
        provisions.snapshots,
        &post_ref,
        limits,
    )?;
    let documents = InvocationDocuments {
        invocation: invocation_read,
        pre: pre_read,
        post: post_read,
    };
    let invocation = documents.invocation()?;
    let pre_snapshot = documents.pre()?;
    let post_snapshot = documents.post()?;

    // Check 3: observation role (pre/post's own anchor is not applicable
    // to an invocation's snapshots).
    if pre_snapshot.observation != document::ObservationRole::Pre
        || post_snapshot.observation != document::ObservationRole::Post
    {
        return Err(refuse(AdmissionRecord::new(
            Code::WrongSnapshot,
            "wrong-observation",
        )));
    }

    // Check 4: model, for each document in the order it was read.
    check_model(context_view, &invocation.call.model)?;
    check_model(context_view, &pre_snapshot.model)?;
    check_model(context_view, &post_snapshot.model)?;

    // Check 5: operation, then the invocation each snapshot names.
    let state_machine = is_state_machine_context(context_view, context_name);
    let operation = check_operation(
        &invocation.call,
        &Receiver {
            views,
            types,
            context_name,
            context: clause_context,
            state_machine,
        },
        source,
    )?;
    document::check_snapshot_invocation(
        pre_snapshot,
        &documents.pre.identity,
        &documents.invocation.identity,
    )?;
    document::check_snapshot_invocation(
        post_snapshot,
        &documents.post.identity,
        &documents.invocation.identity,
    )?;

    // Checks 6 to 8: pre's own check 6, then post's, then pre's check 7,
    // then post's, then pre's check 8, then post's -- never fully
    // finishing one snapshot's checks 6 to 8 before starting the other's
    // check 6 (SR-750 FND-005).
    let self_object = invocation.call.self_object.as_ref();
    // FR-106 check 9: `self` is required in the post snapshot only for a
    // postcondition (SR-750 FND-006), never for a `Frame` run, whose
    // operation may delete it.
    let post_range = if post_self {
        limits.post_state
    } else {
        PostStateRange::Refuse
    };
    let read_machine = state_machine.then_some(clause_context);
    let mut pre_values = document::admit_population_values(
        views,
        types,
        pre_snapshot,
        &documents.pre.identity,
        read_machine,
        PostStateRange::Refuse,
        ancestor_steps,
    )?;
    let mut post_values = document::admit_population_values(
        views,
        types,
        post_snapshot,
        &documents.post.identity,
        read_machine,
        post_range,
        ancestor_steps,
    )?;
    let pre_self_population =
        self_object.and_then(|object| (!state_machine).then_some(object.population.as_str()));
    let post_self_population = self_object
        .and_then(|object| (!state_machine && post_self).then_some(object.population.as_str()));
    let mut pre_required =
        document::parameter_populations(views, &operation, &invocation.call.parameters);
    pre_required.extend(pre_values.read_context_populations());
    pre_required.extend(named.pre.iter().cloned());
    let mut post_required = post_values.read_context_populations();
    post_required.extend(named.post.iter().cloned());
    document::check_population_completeness(
        &pre_snapshot.populations,
        &pre_values.completeness,
        pre_self_population,
        &pre_required,
        &pre_values.extents,
    )?;
    document::check_population_completeness(
        &post_snapshot.populations,
        &post_values.completeness,
        post_self_population,
        &post_required,
        &post_values.extents,
    )?;
    document::check_population_closure(
        &pre_snapshot.populations,
        &pre_values.completeness,
        &pre_values.keys_by_population,
        &pre_values.extents,
    )?;
    document::check_population_closure(
        &post_snapshot.populations,
        &post_values.completeness,
        &post_values.keys_by_population,
        &post_values.extents,
    )?;
    document::check_context_closure(&[&pre_values, &post_values])?;

    // Check 9: self and context.
    let (pre_admitted, post_admitted, self_reference, receiver_pre, receiver_post) =
        if state_machine {
            if self_object.is_some() {
                return Err(wrong_self_kind());
            }
            let receiver_pre =
                pre_values.attach_read_context(context_name, &documents.pre.identity, selected)?;
            let receiver_post = if post_self {
                Some(post_values.attach_read_context(
                    context_name,
                    &documents.post.identity,
                    selected,
                )?)
            } else {
                None
            };
            let pre_admitted = document::finish_populations(types, pre_values)?;
            let post_admitted = document::finish_populations(types, post_values)?;
            let self_reference = receiver_post
                .clone()
                .unwrap_or_else(|| receiver_pre.clone());
            (
                pre_admitted,
                post_admitted,
                self_reference,
                Some(receiver_pre),
                receiver_post,
            )
        } else {
            let pre_admitted = document::finish_populations(types, pre_values)?;
            let post_admitted = document::finish_populations(types, post_values)?;
            let self_object = self_object.ok_or_else(wrong_self_kind)?;
            let reference = document::resolve_self(
                views,
                types,
                context_view,
                context_name,
                &pre_admitted.environment,
                self_object,
            )?;
            if post_self {
                let _ = document::resolve_self(
                    views,
                    types,
                    context_view,
                    context_name,
                    &post_admitted.environment,
                    self_object,
                )?;
            }
            (pre_admitted, post_admitted, reference, None, None)
        };

    // Check 10: parameters and result.
    // A reference parameter resolves against the pre snapshot's objects, a
    // reference result against the post snapshot's; either one naming a key
    // absent from its complete population refuses `dangling_reference`
    // (check 8's closure rule, SR-771 FND-001).
    let mut parameter_references =
        document::References::over(views, types, &pre_snapshot.populations);
    let mut result_references =
        document::References::over(views, types, &post_snapshot.populations);
    let parameters = document::admit_parameters(
        &mut parameter_references,
        &operation,
        &invocation.call.parameters,
    )?;
    let result = document::admit_result(&mut result_references, &operation, &invocation.result)?;

    Ok(InvocationAdmission {
        documents,
        pre: pre_admitted,
        post: post_admitted,
        self_reference,
        invocation: selected.clone(),
        self_kind: SelfKind::of(state_machine),
        receiver_pre,
        receiver_post,
        operation,
        parameters,
        result,
    })
}

/// What a pre-call admission reads about the selected clause, whether the
/// precondition is selected by `PreCall` or by `Invocation`, or a state
/// machine's invariant by `Context` on side `pre`.
struct PreCallContext<'a> {
    views: &'a [ModelView],
    types: &'a TypeEnvironment,
    context_view: &'a ModelView,
    context_name: &'a str,
    clause_context: EffectiveId,
    state_machine: bool,
    clause: quire_exact::NodeKey,
    operation: OperationSource<'a>,
    /// Whether this is the invariant of a state machine on side `pre`: the
    /// invocation's `parameters` are neither read nor checked.
    as_context: bool,
    ancestor_steps: u64,
    /// The populations the clause names at `pre` (FR-391), required with
    /// the reference parameters'.
    named: &'a [String],
}

/// FR-106 check 5: an invocation's `context` and `operation` against the
/// clause's. For an object receiver they must be the clause's. For a state
/// machine's clause the `context` must be the clause's machine or a
/// specialization of it, and the `operation` the clause's, or for an
/// invariant any effective operation of the machine the `context` names.
/// Returns the operation the invocation names.
fn check_operation(
    call: &document::InvocationCall,
    receiver: &Receiver<'_>,
    source: OperationSource<'_>,
) -> Result<OperationDeclaration, AdmissionFailure> {
    let wrong_invocation = || {
        refuse(AdmissionRecord::new(
            Code::WrongSnapshot,
            "wrong-invocation",
        ))
    };
    if !receiver.state_machine {
        let OperationSource::Clause(operation) = source else {
            return Err(fault("context-selection-of-an-object-receiver"));
        };
        if call.context != receiver.context_name || call.operation != operation.declaration.name() {
            return Err(wrong_invocation());
        }
        return Ok(operation.declaration.clone());
    }
    let invoked = document::find_declaration(receiver.views, &call.context)
        .and_then(|(view, key)| view.view.type_identities().get(&key).copied())
        .filter(|invoked| receiver.types.conforms(*invoked, receiver.context))
        .ok_or_else(wrong_invocation)?;
    match source {
        OperationSource::Clause(operation) if call.operation == operation.declaration.name() => {
            Ok(operation.declaration.clone())
        }
        OperationSource::Clause(_) => Err(wrong_invocation()),
        OperationSource::Any(table) => {
            match table.resolve(receiver.types, invoked, &call.operation) {
                OperationLookup::Declared { operation, .. } => Ok(operation.clone()),
                OperationLookup::Missing | OperationLookup::Ambiguous(_) => Err(wrong_invocation()),
            }
        }
    }
}

/// One pre-call observation's documents, already read by check 1: the
/// call half of the invocation that carried it (`None` for a `PreCall`
/// selection) and that invocation's read usage, its pre snapshot, the self
/// object and the parameters.
struct PreCallInput<'a> {
    call: Option<&'a document::InvocationCall>,
    receiver_invocation: Option<&'a DocumentRef>,
    /// The selected invocation's own identity, when one is selected.
    invocation: Option<&'a DocumentRef>,
    invocation_usage: AdmissionUsage,
    pre: document::ReadDocument,
    self_object: Option<&'a SelectedObject>,
    parameters: &'a [(String, SnapshotValue)],
}

/// A precondition selected by `Invocation` (FR-106 "Behavior"): check 1
/// reads the invocation's call half and then its `pre` snapshot, and
/// nothing of its post side -- not its `post` snapshot, `result`,
/// `created` or `deleted` -- and check 11 does not run. The rest is
/// [`admit_pre_call`], the one path a `PreCall` selection takes too, so
/// both admit the same observation. A state machine's invariant on side
/// `pre` reads the same documents and, binding no parameter, not the
/// invocation's `parameters` either.
fn admit_pre_call_invocation(
    context: &PreCallContext<'_>,
    provisions: &Provisions<'_>,
    selected: &DocumentRef,
    limits: ObservationLimits,
) -> Result<AdmittedObservations, AdmissionFailure> {
    let invocation_read = read_document(
        if context.as_context {
            DocumentKind::InvocationContext
        } else {
            DocumentKind::InvocationCall
        },
        provisions.invocations,
        selected,
        limits,
    )?;
    let call = invocation_read
        .as_invocation_call()
        .ok_or_else(|| fault("expected-invocation-document"))?;
    let pre_read = read_document(
        DocumentKind::Snapshot,
        provisions.snapshots,
        &call.pre,
        limits,
    )?;
    admit_pre_call(
        context,
        PreCallInput {
            call: Some(call),
            receiver_invocation: Some(selected),
            invocation: Some(&invocation_read.identity),
            invocation_usage: invocation_read.usage,
            pre: pre_read,
            self_object: call.self_object.as_ref(),
            parameters: &call.parameters,
        },
    )
}

/// FR-106 checks 3 to 10 over one pre-call observation, read by check 1:
/// the pre snapshot, the self object and the parameters, and for an
/// invocation its own model (check 4) and operation (check 5). Check 11
/// compares a pre and a post snapshot; a pre-call observation has no post
/// side, so it does not run.
fn admit_pre_call(
    context: &PreCallContext<'_>,
    input: PreCallInput<'_>,
) -> Result<AdmittedObservations, AdmissionFailure> {
    let call = input.call;
    let snapshot = input
        .pre
        .as_snapshot()
        .ok_or_else(|| fault("expected-snapshot-document"))?;

    // Check 3: observation role.
    if snapshot.observation != document::ObservationRole::Pre {
        return Err(refuse(AdmissionRecord::new(
            Code::WrongSnapshot,
            "wrong-observation",
        )));
    }

    // Check 4: model, for each document in the order it was read.
    if let Some(call) = call {
        check_model(context.context_view, &call.model)?;
    }
    check_model(context.context_view, &snapshot.model)?;

    // Check 5: operation.
    let operation = match (call, context.operation) {
        (Some(call), source) => check_operation(
            call,
            &Receiver {
                views: context.views,
                types: context.types,
                context_name: context.context_name,
                context: context.clause_context,
                state_machine: context.state_machine,
            },
            source,
        )?,
        (None, OperationSource::Clause(operation)) => operation.declaration.clone(),
        (None, OperationSource::Any(_)) => return Err(fault("context-selection-names-no-call")),
    };
    if let Some(invocation) = input.invocation {
        document::check_snapshot_invocation(snapshot, &input.pre.identity, invocation)?;
    }

    // Checks 6 to 8, with every declared population a reference parameter
    // names required alongside `self`'s.
    let state_machine = context.state_machine;
    let receiver_identity = input
        .receiver_invocation
        .cloned()
        .unwrap_or_else(|| input.pre.identity.clone());
    let mut values = document::admit_population_values(
        context.views,
        context.types,
        snapshot,
        &input.pre.identity,
        state_machine.then_some(context.clause_context),
        PostStateRange::Refuse,
        context.ancestor_steps,
    )?;
    let mut required = if context.as_context {
        Vec::new()
    } else {
        document::parameter_populations(context.views, &operation, input.parameters)
    };
    required.extend(values.read_context_populations());
    required.extend(context.named.iter().cloned());
    document::check_population_completeness(
        &snapshot.populations,
        &values.completeness,
        input
            .self_object
            .and_then(|object| (!state_machine).then_some(object.population.as_str())),
        &required,
        &values.extents,
    )?;
    document::check_population_closure(
        &snapshot.populations,
        &values.completeness,
        &values.keys_by_population,
        &values.extents,
    )?;
    document::check_context_closure(&[&values])?;

    // Check 9: self and context.
    let (admitted, self_reference, receiver_pre) = if state_machine {
        if input.self_object.is_some() {
            return Err(wrong_self_kind());
        }
        let reference = values.attach_read_context(
            context.context_name,
            &input.pre.identity,
            &receiver_identity,
        )?;
        let admitted = document::finish_populations(context.types, values)?;
        (admitted, reference.clone(), Some(reference))
    } else {
        let admitted = document::finish_populations(context.types, values)?;
        let self_object = input.self_object.ok_or_else(wrong_self_kind)?;
        let reference = document::resolve_self(
            context.views,
            context.types,
            context.context_view,
            context.context_name,
            &admitted.environment,
            self_object,
        )?;
        (admitted, reference, None)
    };

    // Check 10: parameters, resolved against the pre snapshot. A pre-call
    // observation carries no result, and a state machine's invariant binds
    // no parameter.
    let parameters = if context.as_context {
        Vec::new()
    } else {
        let mut references =
            document::References::over(context.views, context.types, &snapshot.populations);
        document::admit_parameters(&mut references, &operation, input.parameters)?
    };

    let usage = input.invocation_usage.merged_with(input.pre.usage);
    let observation = Observation {
        identity: input.pre.identity,
        environment: admitted.environment,
        populations: admitted.completeness,
        out_of_range: admitted.out_of_range,
    };
    let (current, pre) = if context.as_context {
        (Some(observation), None)
    } else {
        (None, Some(observation))
    };
    Ok(AdmittedObservations {
        usage,
        clause: context.clause,
        current,
        pre,
        post: None,
        self_object: self_reference,
        invocation: input.receiver_invocation.cloned(),
        self_kind: SelfKind::of(state_machine),
        receiver_pre,
        receiver_post: None,
        parameters,
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    })
}

/// A postcondition's invocation, or a state machine's invariant on side
/// `post` (`as_context`): FR-106 checks 1 and 3 to 11. A `Context`
/// selection's observation is the post snapshot alone.
#[allow(clippy::too_many_arguments)]
fn admit_operation(
    views: &[ModelView],
    types: &TypeEnvironment,
    context_view: &ModelView,
    context_name: &str,
    clause_identity: quire_exact::NodeKey,
    clause_context: EffectiveId,
    source: OperationSource<'_>,
    as_context: bool,
    named: &NamedReads,
    provisions: &Provisions<'_>,
    selected: &DocumentRef,
    limits: ObservationLimits,
    model_limits: ModelNormalizationLimits,
) -> Result<AdmittedObservations, AdmissionFailure> {
    let admitted = admit_invocation_documents(
        views,
        types,
        context_view,
        context_name,
        clause_context,
        source,
        true,
        named,
        provisions,
        selected,
        limits,
        model_limits.ancestor_steps,
    )?;
    let invocation = admitted.documents.invocation()?;

    // Check 11: frame and delta.
    let frame_context = frame::FrameContext {
        views,
        types,
        context_view,
        limits: model_limits,
        effect: admitted.operation.effect(),
    };
    let binding = frame_binding(invocation);
    let (created, deleted) = frame::enforce(
        &frame_context,
        &binding,
        frame_snapshot(admitted.documents.pre()?),
        frame_snapshot(admitted.documents.post()?),
        &invocation.created,
        &invocation.deleted,
    )?;
    // Check 11.6: the pre and the post context of a state machine's clause.
    if admitted.self_kind == SelfKind::Context {
        frame::enforce_contexts(
            &frame_context,
            clause_context,
            admitted.documents.pre()?,
            admitted.documents.post()?,
        )?;
    }

    let usage = admitted.usage();
    let pre = Observation {
        identity: admitted.documents.pre.identity,
        environment: admitted.pre.environment,
        populations: admitted.pre.completeness,
        out_of_range: admitted.pre.out_of_range,
    };
    let post = Observation {
        identity: admitted.documents.post.identity,
        environment: admitted.post.environment,
        populations: admitted.post.completeness,
        out_of_range: admitted.post.out_of_range,
    };
    if as_context {
        return Ok(AdmittedObservations {
            usage,
            clause: clause_identity,
            current: Some(post),
            pre: None,
            post: None,
            self_object: admitted.self_reference,
            invocation: Some(admitted.invocation),
            self_kind: admitted.self_kind,
            receiver_pre: None,
            receiver_post: admitted.receiver_post,
            parameters: Vec::new(),
            result: None,
            created: Vec::new(),
            deleted: Vec::new(),
        });
    }
    Ok(AdmittedObservations {
        usage,
        clause: clause_identity,
        current: None,
        pre: Some(pre),
        post: Some(post),
        self_object: admitted.self_reference,
        invocation: Some(admitted.invocation),
        self_kind: admitted.self_kind,
        receiver_pre: admitted.receiver_pre,
        receiver_post: admitted.receiver_post,
        parameters: admitted.parameters,
        result: admitted.result,
        created,
        deleted,
    })
}

// ---------------------------------------------------------------------------
// FR-115: an operation frame run over one invocation
// ---------------------------------------------------------------------------

/// FR-115: the selected operation a `Frame` run admits its invocation
/// against, read by the caller off its own checked graph
/// (`check::CheckedGraph::operation_frame`), as [`ClauseFacts`] is.
pub struct FrameFacts {
    /// The context object type `T` of `M::T::op`.
    pub context: EffectiveId,
    /// The operation's `state`/`frame` node identity (FR-105).
    pub frame: quire_exact::NodeKey,
    /// The operation, resolved as FR-104 resolves a clause's.
    pub operation: OperationFacts,
}

/// FR-115: one invocation admitted by FR-106's checks 1 and 3 to 10 for a
/// `Frame` run, holding what check 11 reads. Check 11 itself runs at S6a,
/// through [`Self::check_frame`], in the `ProtocolClause` `evaluate` arm.
pub struct AdmittedInvocation<'t> {
    /// FR-109 Outputs' admission usage over the three documents.
    pub usage: AdmissionUsage,
    /// The frame node identity the invocation was admitted for (the S6a
    /// arm's mismatch check, as [`AdmittedObservations::clause`] is).
    pub frame: quire_exact::NodeKey,
    /// The invocation document's identity and digest.
    pub invocation: DocumentRef,
    /// The pre snapshot's identity and digest.
    pub pre: DocumentRef,
    /// The post snapshot's identity and digest.
    pub post: DocumentRef,
    views: Vec<ModelView>,
    types: &'t TypeEnvironment,
    context: EffectiveId,
    model_limits: ModelNormalizationLimits,
    documents: InvocationDocuments,
}

impl std::fmt::Debug for AdmittedInvocation<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdmittedInvocation")
            .field("usage", &self.usage)
            .field("frame", &self.frame)
            .field("invocation", &self.invocation)
            .field("pre", &self.pre)
            .field("post", &self.post)
            .finish_non_exhaustive()
    }
}

/// `snapshot` as check 11 reads it.
fn frame_snapshot(snapshot: &document::SnapshotDocument) -> frame::FrameSnapshot<'_> {
    frame::FrameSnapshot {
        populations: &snapshot.populations,
        relationships: &snapshot.relationships,
    }
}

/// The receiver and the reference arguments of `invocation`, which the
/// frame's receiver-scoped and argument-scoped entries ground in (FR-013).
fn frame_binding(invocation: &document::InvocationDocument) -> frame::FrameBinding<'_> {
    let call = &invocation.call;
    frame::FrameBinding {
        receiver: call.self_object.as_ref(),
        arguments: call
            .parameters
            .iter()
            .filter_map(|(name, value)| match value.form() {
                RawForm::Reference(object) => Some((name.clone(), object)),
                _ => None,
            })
            .collect(),
    }
}

/// FR-115: check 11's result as the thing under test.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameVerdict {
    /// Nothing changed outside the frame, and the declared delta agrees.
    Holds,
    /// Check 11's steps 1 to 3 found a change outside the frame.
    Violation(Box<FrameWitness>),
    /// The frame cannot be evaluated over the invocation: check 11's step 4
    /// found a declared delta that disagrees
    /// (`population_delta_mismatch`/`delta-disagreement`), the
    /// conformance walk reached its ceiling, or a `frame.write` or
    /// `frame.redefines-link` charge was denied (`resource_exhausted`/
    /// `insufficient-next-charge`).
    Refused(AdmissionRecord),
}

/// FR-115: the evaluated frame witness, the counterexample a `Frame` run's
/// violation carries (ADR-012 §12.2 Evaluate row).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameWitness {
    /// The catalog code: `frame_violation`.
    pub code: Code,
    /// The catalog cause: `unauthorized-change`.
    pub cause: &'static str,
    /// The invocation.
    pub invocation: DocumentRef,
    /// The pre selection.
    pub pre: DocumentRef,
    /// The post selection.
    pub post: DocumentRef,
    /// The population the change is in.
    pub population: String,
    /// The change, with the frame permission it was checked against.
    pub change: FrameChange,
}

/// FR-115: one change outside the frame, with the frame permission it was
/// checked against.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameChange {
    /// A surviving object's field changed, and reaches no `modifies` entry.
    FieldWrite {
        /// The object's key.
        object: String,
        /// The field's declared name.
        field: String,
        /// The frame's `modifies`.
        modifies: Vec<DeclarationKey>,
    },
    /// An object was created whose type conforms to no `creates` entry.
    Created {
        /// The created object's key.
        object: String,
        /// Its most-specific type.
        type_name: DeclarationKey,
        /// The frame's `creates`.
        creates: Vec<DeclarationKey>,
    },
    /// An object was deleted whose type conforms to no `deletes` entry.
    Deleted {
        /// The deleted object's key.
        object: String,
        /// Its most-specific type.
        type_name: DeclarationKey,
        /// The frame's `deletes`.
        deletes: Vec<DeclarationKey>,
    },
    /// An object's most-specific type changed, which no frame permission
    /// authorizes (FR-151's grants are `modifies`, `creates` and `deletes`).
    Retyped {
        /// The object's key.
        object: String,
        /// Its pre most-specific type.
        pre_type: DeclarationKey,
        /// Its post most-specific type.
        post_type: DeclarationKey,
    },
    /// A relationship instance was added or removed, and the frame's
    /// `modifies` does not name the relationship (FR-115 step 5).
    Link {
        /// The relationship node's key.
        relationship: DeclarationKey,
        /// Whether the instance is in the post extent only or the pre only.
        difference: LinkDifference,
        /// The instance's source end.
        source: SelectedObject,
        /// The instance's target end.
        target: SelectedObject,
        /// The frame's checked `modifies`, in declared order.
        modifies: Vec<DeclarationKey>,
    },
}

/// FR-115: which extent a changed relationship instance is in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkDifference {
    /// The instance is in the post extent and not in the pre extent.
    Added,
    /// The instance is in the pre extent and not in the post extent.
    Removed,
}

impl AdmittedInvocation<'_> {
    /// FR-106 check 11 over this invocation with `effect`, the frame the
    /// caller resolved from the compiled package (QSpec FR-013-AC-3: the
    /// request carries no frame), in check 11's order, stopping at the
    /// first finding.
    pub fn check_frame(&self, effect: &OperationEffect) -> Result<FrameVerdict, InternalFault> {
        let as_fault = |failure: AdmissionFailure| match failure {
            AdmissionFailure::Fault(fault) => fault,
            AdmissionFailure::Refused(_) | AdmissionFailure::Incomplete(_) => {
                InternalFault::new("observation-admission", "admitted-invocation-unreadable")
            }
        };
        let context_view = view_of(&self.views, self.context).ok_or_else(|| {
            InternalFault::new("observation-admission", "unresolved-context-type")
        })?;
        let invocation = self.documents.invocation().map_err(as_fault)?;
        let frame_context = frame::FrameContext {
            views: &self.views,
            types: self.types,
            context_view,
            limits: self.model_limits,
            effect,
        };
        let binding = frame_binding(invocation);
        frame::verdict(
            &frame_context,
            &binding,
            &frame::WitnessDocuments {
                invocation: &self.invocation,
                pre: &self.pre,
                post: &self.post,
            },
            frame_snapshot(self.documents.pre().map_err(as_fault)?),
            frame_snapshot(self.documents.post().map_err(as_fault)?),
            &invocation.created,
            &invocation.deleted,
        )
    }
}

/// FR-115: admit the invocation `selected` for a `Frame` run of `frame`'s
/// operation, by FR-106's checks 1 and 3 to 10. Check 4 compares each
/// document's `model` to the model selection of the view `frame.context`
/// resolves in (the alias `M` of `M::T::op`), and check 5 the invocation's
/// `context` and `operation` to the selected ones. `self` is required in
/// the pre snapshot only, since the operation may delete it.
#[allow(clippy::too_many_arguments)]
pub fn admit_frame_invocation<'t>(
    model_selections: &[DomainPackageRef],
    types: &'t TypeEnvironment,
    frame: &FrameFacts,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    model_limits: ModelNormalizationLimits,
    provisions: &Provisions<'_>,
    selected: &DocumentRef,
    limits: ObservationLimits,
) -> Result<AdmittedInvocation<'t>, AdmissionFailure> {
    let views = model_views(model_selections, packages, model_limits)?;
    let context_view =
        view_of(&views, frame.context).ok_or_else(|| fault("unresolved-context-type"))?;
    let context_name = context_view
        .type_name(frame.context)
        .ok_or_else(|| fault("unresolved-context-name"))?
        .to_owned();
    let admitted = admit_invocation_documents(
        &views,
        types,
        context_view,
        &context_name,
        frame.context,
        OperationSource::Clause(&frame.operation),
        false,
        &NamedReads::default(),
        provisions,
        selected,
        limits,
        model_limits.ancestor_steps,
    )?;
    Ok(AdmittedInvocation {
        usage: admitted.usage(),
        frame: frame.frame,
        invocation: admitted.documents.invocation.identity.clone(),
        pre: admitted.documents.pre.identity.clone(),
        post: admitted.documents.post.identity.clone(),
        views,
        types,
        context: frame.context,
        model_limits,
        documents: admitted.documents,
    })
}

/// `type_identity`'s own [`UniverseId`], the same one
/// `crate::model::normalize` assigned its connected component when it
/// built `views` (`EffectiveView::object_universe_of`) -- not an
/// approximation of it. FR-106 admits objects into an
/// [`ObjectEnvironment`], which identifies objects by `(universe, type,
/// key)`; admission and `qsl_replay::spine::clause` (FR-109's `Function`
/// selection, which resolves an object argument's population the same way
/// admission does) must agree with normalization exactly, or a reference
/// to the same object admitted through a different path would compare
/// unequal (a spurious `foreign_reference`/`DanglingReference`).
/// `pub(crate)`: `qsl_replay::spine::clause` resolves an object argument
/// the same way.
fn population_universe(
    views: &[ModelView],
    type_identity: EffectiveId,
) -> Result<UniverseId, AdmissionFailure> {
    let view = view_of(views, type_identity).ok_or_else(|| fault("object-type-unresolved"))?;
    let key = view
        .declaration_key(type_identity)
        .ok_or_else(|| fault("object-type-unresolved"))?;
    view.view
        .object_universe_of(key)
        .map(|universe| universe.identity())
        .ok_or_else(|| fault("object-universe-unresolved"))
}

mod helpers {
    use super::*;

    /// An empty `key` is untrusted input, not an internal invariant (SR-750
    /// FND-004 round 2): FR-106 settles it at admission, `invalid_runtime_
    /// input`/`invalid-value` naming `key`, the same cause check 6.5 uses
    /// for any other malformed value.
    pub(crate) fn object_reference(
        views: &[ModelView],
        type_identity: EffectiveId,
        key: &str,
    ) -> Result<ObjectReference, AdmissionFailure> {
        let object = ObjectId::new(key.to_owned()).map_err(|_| {
            refuse(
                admission_record(Code::InvalidRuntimeInput, "invalid-value").with("field", "key"),
            )
        })?;
        let universe = population_universe(views, type_identity)?;
        Ok(ObjectReference::new(universe, type_identity, object))
    }

    pub(crate) fn admission_record(code: Code, cause: &'static str) -> AdmissionRecord {
        AdmissionRecord::new(code, cause)
    }
}
