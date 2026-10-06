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
    CatalogCoded, Code, InternalFault, LimitExceeded, ALLOCATION_FAILED,
};
use qsl_foundation::{IntakeLimits, Setting};
use quire_exact::{EffectiveId, Identifier, ObjectId, ObjectReference, UniverseId, Value};

use crate::model::accounting::ModelNormalizationLimits;
use crate::model::domain_package::{DomainPackage, DomainPackageRef, OperationEffect};
use crate::model::intake::{admit_selections, read_records, DigestMismatch};
use crate::model::key::{hex, DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use crate::model::normalize::{normalize, EffectiveView, NormalizeOutcome};
use crate::model::object_environment::ObjectEnvironment;
use crate::model::operation::OperationDeclaration;
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
        /// The selected self object.
        self_object: SelectedObject,
    },
    /// A precondition's pre-call observation: the operation's pre state,
    /// the self object and the parameter values, before the operation
    /// runs. It has no post snapshot, result or delta.
    PreCall {
        /// The pre snapshot document.
        snapshot: DocumentRef,
        /// The selected self object.
        self_object: SelectedObject,
        /// Each declared parameter's value, by parameter name.
        parameters: BTreeMap<Identifier, SnapshotValue>,
    },
    /// A precondition's or postcondition's invocation document.
    Invocation {
        /// The invocation document.
        invocation: DocumentRef,
    },
}

impl ClauseSelectionInput {
    /// Which of the three observation forms this input is.
    pub fn form(&self) -> ObservationForm {
        match self {
            Self::Current { .. } => ObservationForm::Current,
            Self::PreCall { .. } => ObservationForm::PreCall,
            Self::Invocation { .. } => ObservationForm::Invocation,
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
}

impl std::fmt::Display for ObservationForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Current => "current",
            Self::PreCall => "pre-call",
            Self::Invocation => "invocation",
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
}

impl AdmissionRecord {
    fn new(code: Code, cause: &'static str) -> Self {
        Self {
            code,
            cause,
            fields: BTreeMap::new(),
        }
    }

    #[must_use]
    fn with(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.fields.insert(key, value.into());
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

/// FR-106 Outputs: one typed observation set admitted for S6a.
#[derive(Clone, Debug)]
pub struct AdmittedObservations {
    /// FR-109 Outputs' admission usage, summed over every document this
    /// call actually read (`current` alone, or `pre` and/or `post`).
    pub usage: AdmissionUsage,
    /// The node identity of the state clause these observations were
    /// admitted for (FR-107's `ObservationsMismatch` check).
    pub clause: quire_exact::NodeKey,
    /// The current observation (invariant only).
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
    Absent,
    Present(usize),
    Reference(SelectedObject),
    Sequence(Vec<usize>),
}

/// The form of one value of a [`SnapshotValue`], with nested values as
/// handles.
#[derive(Clone, Copy, Debug)]
pub enum RawForm<'v> {
    /// `{"boolean": true|false}`.
    Boolean(bool),
    /// `{"integer": "<decimal>"}`.
    Integer(&'v str),
    /// `{"absent": {}}`.
    Absent,
    /// `{"present": <value>}`.
    Present(RawValue<'v>),
    /// `{"reference": {"population": ..., "key": ...}}`.
    Reference(&'v SelectedObject),
    /// `{"sequence": [<value>, ...]}`, its items in order.
    Sequence(RawItems<'v>),
}

/// One value inside a [`SnapshotValue`].
#[derive(Clone, Copy, Debug)]
pub struct RawValue<'v> {
    arena: &'v SnapshotValue,
    index: usize,
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

    /// The root value's form.
    #[must_use]
    pub fn form(&self) -> RawForm<'_> {
        self.at(self.root)
    }

    fn at(&self, index: usize) -> RawForm<'_> {
        match &self.nodes[index] {
            RawNode::Boolean(value) => RawForm::Boolean(*value),
            RawNode::Integer(spelling) => RawForm::Integer(spelling),
            RawNode::Absent => RawForm::Absent,
            RawNode::Present(inner) => RawForm::Present(RawValue {
                arena: self,
                index: *inner,
            }),
            RawNode::Reference(reference) => RawForm::Reference(reference),
            RawNode::Sequence(items) => RawForm::Sequence(RawItems { arena: self, items }),
        }
    }
}

/// Structural equality, compared on an explicit heap stack so a value of any
/// depth compares in constant native stack, whatever order its arena was
/// built in.
impl PartialEq for SnapshotValue {
    fn eq(&self, other: &Self) -> bool {
        let mut work = vec![(self.form(), other.form())];
        while let Some(pair) = work.pop() {
            match pair {
                (RawForm::Boolean(left), RawForm::Boolean(right)) if left == right => {}
                (RawForm::Integer(left), RawForm::Integer(right)) if left == right => {}
                (RawForm::Absent, RawForm::Absent) => {}
                (RawForm::Reference(left), RawForm::Reference(right)) if left == right => {}
                (RawForm::Present(left), RawForm::Present(right)) => {
                    work.push((left.form(), right.form()));
                }
                (RawForm::Sequence(left), RawForm::Sequence(right))
                    if left.iter().len() == right.iter().len() =>
                {
                    work.extend(
                        left.iter()
                            .zip(right.iter())
                            .map(|(left, right)| (left.form(), right.form())),
                    );
                }
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
            other @ (Self::Boolean(_) | Self::Integer(_) | Self::Absent | Self::Reference(_)) => {
                other
            }
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
struct ModelView {
    view: EffectiveView,
    by_effective_id: BTreeMap<EffectiveId, DeclarationKey>,
}

impl ModelView {
    fn type_name(&self, effective: EffectiveId) -> Option<&str> {
        self.by_effective_id
            .get(&effective)
            .map(|key| key.node.as_str())
    }

    fn declaration_key(&self, effective: EffectiveId) -> Option<&DeclarationKey> {
        self.by_effective_id.get(&effective)
    }
}

/// Re-admit and re-normalize every domain package `model_selections`
/// names (the caller's own `CheckedGraph::model_selections()`), against the
/// same package input a caller's `compile` already used. See the module
/// doc's design note.
fn model_views(
    model_selections: &[DomainPackageRef],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    intake: IntakeLimits,
    limits: ModelNormalizationLimits,
) -> Result<Vec<ModelView>, AdmissionFailure> {
    let admitted = admit_selections(model_selections, SHA256_JCS_DIGEST_DOMAIN, packages, intake)
        .map_err(|_| fault("model-reconsistent-admission"))?;
    let mut views = Vec::with_capacity(admitted.len());
    for (package_ref, document) in admitted {
        let records = read_records(&package_ref.identity, &document)
            .map_err(|_| fault("model-reconsistent-records"))?;
        let view = match normalize(&DomainPackage::new(package_ref, records), limits) {
            NormalizeOutcome::Completed(view) => view,
            // A reached ceiling is the stage's limit outcome naming its
            // setting; any other refusal is a broken invariant, since the
            // same packages already normalized at compile time.
            NormalizeOutcome::Incomplete(incomplete) => {
                return Err(limit_refusal(&incomplete.limit_exceeded()))
            }
            NormalizeOutcome::Refused(refusals) => {
                return Err(refusals
                    .iter()
                    .find_map(|refusal| refusal.cause.limit_exceeded())
                    .map_or_else(
                        || fault("model-reconsistent-normalize"),
                        |limit| limit_refusal(&limit),
                    ))
            }
        };
        let by_effective_id = view
            .type_identities()
            .iter()
            .map(|(key, effective)| (*effective, key.clone()))
            .collect();
        views.push(ModelView {
            view,
            by_effective_id,
        });
    }
    Ok(views)
}

/// The one [`ModelView`] whose effective view names `effective`, and the
/// [`DeclarationKey`] it resolves to. `None` when no re-derived view names
/// it -- a broken invariant, since `effective` came from the same checked
/// package these views were re-derived from.
fn view_of(views: &[ModelView], effective: EffectiveId) -> Option<&ModelView> {
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
#[allow(clippy::too_many_arguments)]
pub fn admit_current_snapshot(
    model_selections: &[DomainPackageRef],
    types: &TypeEnvironment,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    intake: IntakeLimits,
    model_limits: ModelNormalizationLimits,
    provision: &BTreeMap<[u8; 32], Vec<u8>>,
    selected: &DocumentRef,
    limits: ObservationLimits,
) -> Result<(ObjectEnvironment, AdmissionUsage), AdmissionFailure> {
    let views = model_views(model_selections, packages, intake, model_limits)?;
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
    let admitted = document::admit_populations(
        &views,
        types,
        &snapshot.populations,
        None,
        model_limits.ancestor_steps,
    )?;
    Ok((admitted.environment, read.usage))
}

/// FR-109's `Function` selection: `type_identity`'s own [`UniverseId`],
/// re-deriving `views` the same way [`admit_current_snapshot`] does (the
/// module doc's design note), for the caller to resolve an object
/// argument's population into the exact same universe admission itself
/// would assign that object.
pub fn population_universe_for(
    model_selections: &[DomainPackageRef],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    intake: IntakeLimits,
    model_limits: ModelNormalizationLimits,
    type_identity: EffectiveId,
) -> Result<UniverseId, AdmissionFailure> {
    let views = model_views(model_selections, packages, intake, model_limits)?;
    population_universe(&views, type_identity)
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
    intake: IntakeLimits,
    model_limits: ModelNormalizationLimits,
    provisions: &Provisions<'_>,
    selection: &ClauseSelection,
    limits: ObservationLimits,
) -> Result<AdmittedObservations, AdmissionFailure> {
    // Check 2: selection form.
    match (clause.kind, &selection.input) {
        (StateClauseKind::Invariant, ClauseSelectionInput::Current { .. })
        | (StateClauseKind::Precondition, ClauseSelectionInput::PreCall { .. })
        | (
            StateClauseKind::Precondition | StateClauseKind::Postcondition,
            ClauseSelectionInput::Invocation { .. },
        ) => {}
        (
            StateClauseKind::Precondition | StateClauseKind::Postcondition,
            ClauseSelectionInput::Current { .. },
        )
        | (
            StateClauseKind::Invariant | StateClauseKind::Postcondition,
            ClauseSelectionInput::PreCall { .. },
        )
        | (StateClauseKind::Invariant, ClauseSelectionInput::Invocation { .. }) => {
            return Err(refuse(AdmissionRecord::new(
                Code::WrongSnapshot,
                "wrong-observation",
            )))
        }
    }

    let views = model_views(model_selections, packages, intake, model_limits)?;
    let context_view =
        view_of(&views, clause.context).ok_or_else(|| fault("unresolved-context-type"))?;
    let context_name = context_view
        .type_name(clause.context)
        .ok_or_else(|| fault("unresolved-context-name"))?
        .to_owned();

    // A precondition's pre-call observation, selected by `PreCall` or by
    // `Invocation`, is admitted against this one context.
    let pre_call_context = || -> Result<PreCallContext<'_>, AdmissionFailure> {
        Ok(PreCallContext {
            views: &views,
            types,
            context_view,
            context_name: &context_name,
            clause: clause.identity,
            operation: clause
                .operation
                .as_ref()
                .ok_or_else(|| fault("clause-declares-no-operation"))?,
            ancestor_steps: model_limits.ancestor_steps,
        })
    };

    match &selection.input {
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
            self_object,
            limits,
            model_limits.ancestor_steps,
        ),
        ClauseSelectionInput::PreCall {
            snapshot,
            self_object,
            parameters,
        } => {
            let context = pre_call_context()?;
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
                    invocation_usage: AdmissionUsage::default(),
                    pre: read,
                    self_object,
                    parameters: &parameters,
                },
            )
        }
        ClauseSelectionInput::Invocation { invocation } => match clause.kind {
            StateClauseKind::Precondition => {
                admit_pre_call_invocation(&pre_call_context()?, provisions, invocation, limits)
            }
            StateClauseKind::Postcondition => admit_operation(
                &views,
                types,
                context_view,
                &context_name,
                clause.identity,
                clause.operation.as_ref(),
                provisions,
                invocation,
                limits,
                model_limits.ancestor_steps,
            ),
            // Check 2 above refused an invariant's `Invocation` selection.
            StateClauseKind::Invariant => Err(fault("invariant-selected-by-invocation")),
        },
    }
}

// The remainder of this module (document reading, the eleven checks, and
// the frame/delta check) is implemented incrementally; see
// `admit_invariant`/`admit_operation` below.

mod document;
mod frame;
mod tree;

use document::{read_document, DocumentKind};

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
        &snapshot.populations,
        Some(&self_object.population),
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
    parameters: Vec<(String, Value)>,
    result: Option<Value>,
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

/// FR-106 checks 1 and 3 to 10 over the invocation `selected` of
/// `operation` on `context_view`'s type `context_name`. `self` is required
/// in the post snapshot only when `post_self` (a postcondition, SR-750
/// FND-006).
#[allow(clippy::too_many_arguments)]
fn admit_invocation_documents(
    views: &[ModelView],
    types: &TypeEnvironment,
    context_view: &ModelView,
    context_name: &str,
    operation: &OperationFacts,
    post_self: bool,
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

    // Check 5: operation.
    check_operation(&invocation.call, context_name, operation)?;

    // Checks 6 to 8: pre's own check 6, then post's, then pre's check 7,
    // then post's, then pre's check 8, then post's -- never fully
    // finishing one snapshot's checks 6 to 8 before starting the other's
    // check 6 (SR-750 FND-005).
    let self_object = &invocation.call.self_object;
    // FR-106 check 9: `self` is required in the post snapshot only for a
    // postcondition (SR-750 FND-006), never for a `Frame` run, whose
    // operation may delete it.
    let post_self_population = post_self.then_some(self_object.population.as_str());
    let pre_values = document::admit_population_values(
        views,
        types,
        &pre_snapshot.populations,
        PostStateRange::Refuse,
        ancestor_steps,
    )?;
    let post_values = document::admit_population_values(
        views,
        types,
        &post_snapshot.populations,
        // Only a postcondition reads the post snapshot as the
        // subject's output; a frame run refuses as for any input.
        if post_self {
            limits.post_state
        } else {
            PostStateRange::Refuse
        },
        ancestor_steps,
    )?;
    document::check_population_completeness(
        &pre_snapshot.populations,
        &pre_values.completeness,
        Some(&self_object.population),
        &document::parameter_populations(
            views,
            &operation.declaration,
            &invocation.call.parameters,
        ),
    )?;
    document::check_population_completeness(
        &post_snapshot.populations,
        &post_values.completeness,
        post_self_population,
        &[],
    )?;
    document::check_population_closure(
        &pre_snapshot.populations,
        &pre_values.completeness,
        &pre_values.keys_by_population,
    )?;
    document::check_population_closure(
        &post_snapshot.populations,
        &post_values.completeness,
        &post_values.keys_by_population,
    )?;
    let pre_admitted = document::finish_populations(types, pre_values)?;
    let post_admitted = document::finish_populations(types, post_values)?;

    let self_reference = document::resolve_self(
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
        &operation.declaration,
        &invocation.call.parameters,
    )?;
    let result = document::admit_result(
        &mut result_references,
        &operation.declaration,
        &invocation.result,
    )?;

    Ok(InvocationAdmission {
        documents,
        pre: pre_admitted,
        post: post_admitted,
        self_reference,
        parameters,
        result,
    })
}

/// What a pre-call admission reads about the selected precondition,
/// whether the precondition is selected by `PreCall` or by `Invocation`.
struct PreCallContext<'a> {
    views: &'a [ModelView],
    types: &'a TypeEnvironment,
    context_view: &'a ModelView,
    context_name: &'a str,
    clause: quire_exact::NodeKey,
    operation: &'a OperationFacts,
    ancestor_steps: u64,
}

/// FR-106 check 5: an invocation's `context` and `operation` must be the
/// selected clause's.
fn check_operation(
    call: &document::InvocationCall,
    context_name: &str,
    operation: &OperationFacts,
) -> Result<(), AdmissionFailure> {
    if call.context != context_name || call.operation != operation.declaration.name() {
        return Err(refuse(AdmissionRecord::new(
            Code::WrongSnapshot,
            "wrong-invocation",
        )));
    }
    Ok(())
}

/// One pre-call observation's documents, already read by check 1: the
/// call half of the invocation that carried it (`None` for a `PreCall`
/// selection) and that invocation's read usage, its pre snapshot, the self
/// object and the parameters.
struct PreCallInput<'a> {
    call: Option<&'a document::InvocationCall>,
    invocation_usage: AdmissionUsage,
    pre: document::ReadDocument,
    self_object: &'a SelectedObject,
    parameters: &'a [(String, SnapshotValue)],
}

/// A precondition selected by `Invocation` (FR-106 "Behavior"): check 1
/// reads the invocation's call half and then its `pre` snapshot, and
/// nothing of its post side -- not its `post` snapshot, `result`,
/// `created` or `deleted` -- and check 11 does not run. The rest is
/// [`admit_pre_call`], the one path a `PreCall` selection takes too, so
/// both admit the same observation.
fn admit_pre_call_invocation(
    context: &PreCallContext<'_>,
    provisions: &Provisions<'_>,
    selected: &DocumentRef,
    limits: ObservationLimits,
) -> Result<AdmittedObservations, AdmissionFailure> {
    let invocation_read = read_document(
        DocumentKind::InvocationCall,
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
            invocation_usage: invocation_read.usage,
            pre: pre_read,
            self_object: &call.self_object,
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
    if let Some(call) = call {
        check_operation(call, context.context_name, context.operation)?;
    }

    // Checks 6 to 8, with every declared population a reference parameter
    // names required alongside `self`'s.
    let values = document::admit_population_values(
        context.views,
        context.types,
        &snapshot.populations,
        PostStateRange::Refuse,
        context.ancestor_steps,
    )?;
    document::check_population_completeness(
        &snapshot.populations,
        &values.completeness,
        Some(&input.self_object.population),
        &document::parameter_populations(
            context.views,
            &context.operation.declaration,
            input.parameters,
        ),
    )?;
    document::check_population_closure(
        &snapshot.populations,
        &values.completeness,
        &values.keys_by_population,
    )?;
    let admitted = document::finish_populations(context.types, values)?;

    // Check 9: self.
    let self_reference = document::resolve_self(
        context.views,
        context.types,
        context.context_view,
        context.context_name,
        &admitted.environment,
        input.self_object,
    )?;

    // Check 10: parameters, resolved against the pre snapshot. A pre-call
    // observation carries no result.
    let mut references =
        document::References::over(context.views, context.types, &snapshot.populations);
    let parameters = document::admit_parameters(
        &mut references,
        &context.operation.declaration,
        input.parameters,
    )?;

    let usage = input.invocation_usage.merged_with(input.pre.usage);
    Ok(AdmittedObservations {
        usage,
        clause: context.clause,
        current: None,
        pre: Some(Observation {
            identity: input.pre.identity,
            environment: admitted.environment,
            populations: admitted.completeness,
            out_of_range: admitted.out_of_range,
        }),
        post: None,
        self_object: self_reference,
        parameters,
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    })
}

#[allow(clippy::too_many_arguments)]
fn admit_operation(
    views: &[ModelView],
    types: &TypeEnvironment,
    context_view: &ModelView,
    context_name: &str,
    clause_identity: quire_exact::NodeKey,
    operation: Option<&OperationFacts>,
    provisions: &Provisions<'_>,
    selected: &DocumentRef,
    limits: ObservationLimits,
    ancestor_steps: u64,
) -> Result<AdmittedObservations, AdmissionFailure> {
    let operation = operation.ok_or_else(|| fault("clause-declares-no-operation"))?;
    let admitted = admit_invocation_documents(
        views,
        types,
        context_view,
        context_name,
        operation,
        true,
        provisions,
        selected,
        limits,
        ancestor_steps,
    )?;
    let invocation = admitted.documents.invocation()?;

    // Check 11: frame and delta.
    let frame_context = frame::FrameContext {
        views,
        types,
        context_view,
        ancestor_steps,
        effect: operation.declaration.effect(),
    };
    let (created, deleted) = frame::enforce(
        &frame_context,
        &admitted.documents.pre()?.populations,
        &admitted.documents.post()?.populations,
        &invocation.created,
        &invocation.deleted,
    )?;

    let usage = admitted.usage();
    Ok(AdmittedObservations {
        usage,
        clause: clause_identity,
        current: None,
        pre: Some(Observation {
            identity: admitted.documents.pre.identity,
            environment: admitted.pre.environment,
            populations: admitted.pre.completeness,
            out_of_range: admitted.pre.out_of_range,
        }),
        post: Some(Observation {
            identity: admitted.documents.post.identity,
            environment: admitted.post.environment,
            populations: admitted.post.completeness,
            out_of_range: admitted.post.out_of_range,
        }),
        self_object: admitted.self_reference,
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
    ancestor_steps: u64,
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

/// FR-115: check 11's result as the thing under test.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameVerdict {
    /// Nothing changed outside the frame, and the declared delta agrees.
    Holds,
    /// Check 11's steps 1 to 3 found a change outside the frame.
    Violation(Box<FrameWitness>),
    /// The frame cannot be evaluated over the invocation: check 11's step 4
    /// found a declared delta that disagrees
    /// (`population_delta_mismatch`/`delta-disagreement`), or the
    /// conformance walk reached its ceiling.
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
            ancestor_steps: self.ancestor_steps,
            effect,
        };
        frame::verdict(
            &frame_context,
            &frame::WitnessDocuments {
                invocation: &self.invocation,
                pre: &self.pre,
                post: &self.post,
            },
            &self.documents.pre().map_err(as_fault)?.populations,
            &self.documents.post().map_err(as_fault)?.populations,
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
    intake: IntakeLimits,
    model_limits: ModelNormalizationLimits,
    provisions: &Provisions<'_>,
    selected: &DocumentRef,
    limits: ObservationLimits,
) -> Result<AdmittedInvocation<'t>, AdmissionFailure> {
    let views = model_views(model_selections, packages, intake, model_limits)?;
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
        &frame.operation,
        false,
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
        ancestor_steps: model_limits.ancestor_steps,
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
