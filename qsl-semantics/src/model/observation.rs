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
use qsl_foundation::diagnostic::InternalFault;
use quire_exact::{EffectiveId, ObjectId, ObjectReference, UniverseId, Value};

use crate::model::accounting::ModelNormalizationLimits;
use crate::model::domain_package::{DomainPackage, DomainPackageRef};
use crate::model::intake::{admit_selections, read_records};
use crate::model::key::{hex, DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use crate::model::normalize::{normalize, EffectiveView, NormalizeOutcome};
use crate::model::object_environment::ObjectEnvironment;
use crate::value::declaration::{OperationDeclaration, TypeEnvironment};

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

/// FR-106 Inputs: the four document-shape ceilings admission enforces
/// before any semantic check (check 1.2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationLimits {
    /// The document byte ceiling (default 1 MiB).
    pub document_bytes: u64,
    /// The JSON nesting-depth ceiling (default 64).
    pub nesting_depth: u32,
    /// The objects-per-document ceiling (default 10,000).
    pub objects_per_document: u64,
    /// The values-per-document ceiling (default 100,000).
    pub values_per_document: u64,
}

impl Default for ObservationLimits {
    fn default() -> Self {
        Self {
            document_bytes: 1_048_576,
            nesting_depth: 64,
            objects_per_document: 10_000,
            values_per_document: 100_000,
        }
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
    /// A precondition's or postcondition's invocation document.
    Invocation {
        /// The invocation document.
        invocation: DocumentRef,
    },
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
    /// The catalog code, e.g. `"invalid_runtime_input"`.
    pub code: &'static str,
    /// The catalog cause, e.g. `"wrong-role-mapping"`.
    pub cause: &'static str,
    /// The named input path components this record carries, keyed by name
    /// (`document`, `population`, `object`, `field`, `label`, ...).
    pub fields: BTreeMap<&'static str, String>,
}

impl AdmissionRecord {
    fn new(code: &'static str, cause: &'static str) -> Self {
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
    #[error("admission refused: {}/{}", .0.code, .0.cause)]
    Refused(AdmissionRecord),
    /// Missing evidence, or an unresolved object/subtype closure: no
    /// `AdmittedObservations`, distinct from a refusal (FR-106 checks 1.1,
    /// 7).
    #[error("admission incomplete: {}/{}", .0.code, .0.cause)]
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
}

/// FR-106 Outputs: one typed observation set admitted for S6a.
#[derive(Clone, Debug)]
pub struct AdmittedObservations {
    /// The node identity of the state clause these observations were
    /// admitted for (FR-107's `ObservationsMismatch` check).
    pub clause: quire_exact::NodeKey,
    /// The current observation (invariant only).
    pub current: Option<Observation>,
    /// The pre observation (precondition/postcondition only).
    pub pre: Option<Observation>,
    /// The post observation (postcondition only, present alongside `pre`
    /// for a precondition too, since FR-106 admits both invocation
    /// snapshots).
    pub post: Option<Observation>,
    /// The selected self object's reference.
    pub self_object: ObjectReference,
    /// The invocation's parameter values, by declared name (empty for an
    /// invariant).
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

#[derive(Clone, Debug)]
enum RawValue {
    Boolean(bool),
    Integer(String),
    Absent,
    Present(Box<RawValue>),
    Reference(RawRef),
    Sequence(Vec<RawValue>),
}

#[derive(Clone, Debug)]
struct RawRef {
    population: String,
    key: String,
}

/// Read one FR-106 value form from `json`, or `None` when it is not one of
/// the six tagged shapes: a wire-reading edge (ADR-012 §9), converting the
/// value's tag to a closed [`RawValue`] variant once, here.
#[qsl_attrs::string_edge]
fn read_raw_value(json: &serde_json::Value) -> Option<RawValue> {
    let object = json.as_object()?;
    if object.len() != 1 {
        return None;
    }
    let (tag, payload) = object.iter().next()?;
    match tag.as_str() {
        "boolean" => Some(RawValue::Boolean(payload.as_bool()?)),
        "integer" => Some(RawValue::Integer(payload.as_str()?.to_owned())),
        // FR-106 line 107 spells this tag's payload `{}`, not any value
        // (SR-750 FND-013): an object with any member, or a non-object
        // payload, is not this shape at all.
        "absent" => {
            if payload.as_object().is_some_and(serde_json::Map::is_empty) {
                Some(RawValue::Absent)
            } else {
                None
            }
        }
        "present" => Some(RawValue::Present(Box::new(read_raw_value(payload)?))),
        "reference" => {
            let population = payload.get("population")?.as_str()?.to_owned();
            let key = payload.get("key")?.as_str()?.to_owned();
            Some(RawValue::Reference(RawRef { population, key }))
        }
        "sequence" => {
            let items = payload.as_array()?;
            let mut sequence = Vec::with_capacity(items.len());
            for item in items {
                sequence.push(read_raw_value(item)?);
            }
            Some(RawValue::Sequence(sequence))
        }
        _ => None,
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
    limits: ModelNormalizationLimits,
) -> Result<Vec<ModelView>, AdmissionFailure> {
    let admitted = admit_selections(model_selections, SHA256_JCS_DIGEST_DOMAIN, packages)
        .map_err(|_| fault("model-reconsistent-admission"))?;
    let mut views = Vec::with_capacity(admitted.len());
    for (package_ref, document) in admitted {
        let records = read_records(&package_ref.identity, &document)
            .map_err(|_| fault("model-reconsistent-records"))?;
        let view = match normalize(&DomainPackage::new(package_ref, records), limits) {
            NormalizeOutcome::Completed(view) => view,
            NormalizeOutcome::Refused(_) | NormalizeOutcome::Incomplete(_) => {
                return Err(fault("model-reconsistent-normalize"))
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

/// FR-106's digest-first rule, checked against `expected`: bytes that parse
/// as JSON are digested over their RFC 8785 canonical encoding, through
/// `quire-canonical` directly -- the one sanctioned encoder (ADR-013 §2);
/// bytes that do not parse are digested raw. The raw fallback itself is
/// `model::intake::check_package_digest` (widened, QSL-278, to serve this
/// second caller) -- never an ad hoc `ByteDigest::of` here, and never a
/// second call site of `model::key::raw_bytes_digest` outside that already-
/// exempt function (ADR-013 §2 O-05: one RFC 8785 encoder, one raw-fallback
/// call site). A value that parses as JSON but has no RFC 8785 encoding
/// (e.g. a non-finite float) refuses `stale_dependency`/`byte-digest-
/// mismatch` directly: admission cannot verify it against `expected` either
/// way, so this is that same outcome, not a silently substituted raw-bytes
/// fallback (the previous behavior).
fn check_document_digest(bytes: &[u8], expected: [u8; 32]) -> Result<(), AdmissionFailure> {
    let mismatch = || {
        refuse(AdmissionRecord::new(
            "stale_dependency",
            "byte-digest-mismatch",
        ))
    };
    let parsed_digest = match serde_json::from_slice::<serde_json::Value>(bytes) {
        Ok(value) => {
            let limits = quire_canonical::Limits::new(u64::MAX, quire_canonical::Limits::MAX_DEPTH)
                .map_err(|_| fault("canonical-limits-invalid"))?;
            let digest = quire_canonical::sha256(&value, limits)
                .map(|digest| *digest.as_bytes())
                .map_err(|_| mismatch())?;
            Some(digest)
        }
        Err(_) => None,
    };
    crate::model::intake::check_package_digest(expected, bytes, parsed_digest)
        .map_err(|_| mismatch())
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
) -> Result<ObjectEnvironment, AdmissionFailure> {
    let views = model_views(model_selections, packages, model_limits)?;
    let read = read_document(DocumentKind::Snapshot, provision, selected, limits)?;
    let snapshot = read
        .as_snapshot()
        .ok_or_else(|| fault("expected-snapshot-document"))?;
    if snapshot.observation != document::ObservationRole::Current {
        return Err(refuse(AdmissionRecord::new(
            "wrong_snapshot",
            "wrong-observation",
        )));
    }
    check_model_any(&views, &snapshot.model)?;
    let admitted = document::admit_populations(&views, types, &snapshot.populations, None, limits)?;
    Ok(admitted.environment)
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
    // Check 2: selection form.
    match (clause.kind, &selection.input) {
        (StateClauseKind::Invariant, ClauseSelectionInput::Current { .. })
        | (
            StateClauseKind::Precondition | StateClauseKind::Postcondition,
            ClauseSelectionInput::Invocation { .. },
        ) => {}
        _ => {
            return Err(refuse(AdmissionRecord::new(
                "wrong_snapshot",
                "wrong-observation",
            )))
        }
    }

    let views = model_views(model_selections, packages, model_limits)?;
    let context_view =
        view_of(&views, clause.context).ok_or_else(|| fault("unresolved-context-type"))?;
    let context_name = context_view
        .type_name(clause.context)
        .ok_or_else(|| fault("unresolved-context-name"))?
        .to_owned();

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
        ),
        ClauseSelectionInput::Invocation { invocation } => admit_operation(
            &views,
            types,
            context_view,
            &context_name,
            clause.identity,
            clause.kind,
            clause.operation.as_ref(),
            provisions,
            invocation,
            limits,
        ),
    }
}

// The remainder of this module (document reading, the eleven checks, and
// the frame/delta check) is implemented incrementally; see
// `admit_invariant`/`admit_operation` below.

mod document;
mod frame;
mod ordered_json;

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
            "wrong_snapshot",
            "wrong-observation",
        )));
    }
    let document_anchor = snapshot
        .anchor
        .as_ref()
        .ok_or_else(|| refuse(AdmissionRecord::new("wrong_snapshot", "wrong-observation")))?;
    // `AnchorKind` derives `PartialEq`; compared directly (SR-750
    // FND-011), never through `as_str()` string equality.
    if document_anchor.kind != anchor.kind || document_anchor.name != anchor.name {
        return Err(refuse(
            AdmissionRecord::new("wrong_snapshot", "wrong-anchor")
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
        limits,
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
        clause,
        current: Some(Observation {
            identity: read.identity,
            environment: environment.environment,
            populations: environment.completeness,
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
            "invalid_model_binding",
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
            "invalid_model_binding",
            "wrong-model-selection",
        )))
    }
}

#[allow(clippy::too_many_arguments)]
fn admit_operation(
    views: &[ModelView],
    types: &TypeEnvironment,
    context_view: &ModelView,
    context_name: &str,
    clause_identity: quire_exact::NodeKey,
    kind: StateClauseKind,
    operation: Option<&OperationFacts>,
    provisions: &Provisions<'_>,
    selected: &DocumentRef,
    limits: ObservationLimits,
) -> Result<AdmittedObservations, AdmissionFailure> {
    // Check 1: read every selected document -- the invocation, then its
    // pre and post snapshots, in that order -- before any of checks 2 to 5
    // read any of their content (SR-750 FND-005: this must not interleave
    // with check 4/5, which is what admission did before this fix).
    let read = read_document(
        DocumentKind::Invocation,
        provisions.invocations,
        selected,
        limits,
    )?;
    let invocation = read
        .as_invocation()
        .ok_or_else(|| fault("expected-invocation-document"))?;
    let pre_read = read_document(
        DocumentKind::Snapshot,
        provisions.snapshots,
        &invocation.pre,
        limits,
    )?;
    let post_read = read_document(
        DocumentKind::Snapshot,
        provisions.snapshots,
        &invocation.post,
        limits,
    )?;
    let pre_snapshot = pre_read
        .as_snapshot()
        .ok_or_else(|| fault("expected-snapshot-document"))?;
    let post_snapshot = post_read
        .as_snapshot()
        .ok_or_else(|| fault("expected-snapshot-document"))?;

    // Check 3: observation role (pre/post's own anchor is not applicable
    // to an invocation's snapshots).
    if pre_snapshot.observation != document::ObservationRole::Pre
        || post_snapshot.observation != document::ObservationRole::Post
    {
        return Err(refuse(AdmissionRecord::new(
            "wrong_snapshot",
            "wrong-observation",
        )));
    }

    // Check 4: model, for each document in the order it was read.
    check_model(context_view, &invocation.model)?;
    check_model(context_view, &pre_snapshot.model)?;
    check_model(context_view, &post_snapshot.model)?;

    // Check 5: operation.
    let operation = operation.ok_or_else(|| fault("clause-declares-no-operation"))?;
    if invocation.context != context_name || invocation.operation != operation.declaration.name() {
        return Err(refuse(AdmissionRecord::new(
            "wrong_snapshot",
            "wrong-invocation",
        )));
    }

    // Checks 6 to 8: pre's own check 6, then post's, then pre's check 7,
    // then post's, then pre's check 8, then post's -- never fully
    // finishing one snapshot's checks 6 to 8 before starting the other's
    // check 6 (SR-750 FND-005).
    let self_object = &invocation.self_object;
    // FR-106 check 9: `self` is required in the current snapshot and an
    // invocation's pre snapshot always, but in the post snapshot only for
    // a postcondition (SR-750 FND-006) -- a precondition of an operation
    // that deletes `self` must not wrongly refuse `wrong-role-mapping`
    // over `self`'s absence from post.
    let post_self_population = match kind {
        StateClauseKind::Postcondition => Some(self_object.population.as_str()),
        StateClauseKind::Invariant | StateClauseKind::Precondition => None,
    };
    let pre_values =
        document::admit_population_values(views, types, &pre_snapshot.populations, limits)?;
    let post_values =
        document::admit_population_values(views, types, &post_snapshot.populations, limits)?;
    document::check_population_completeness(
        &pre_snapshot.populations,
        &pre_values.completeness,
        Some(&self_object.population),
    )?;
    document::check_population_completeness(
        &post_snapshot.populations,
        &post_values.completeness,
        post_self_population,
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
    if kind == StateClauseKind::Postcondition {
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
    let (parameters, result) = document::admit_parameters_and_result(
        views,
        &operation.declaration,
        &invocation.parameters,
        &invocation.result,
    )?;

    // Check 11: frame and delta.
    let (created, deleted) = frame::enforce(
        views,
        operation.declaration.effect(),
        &pre_snapshot.populations,
        &post_snapshot.populations,
        &invocation.created,
        &invocation.deleted,
    )?;

    Ok(AdmittedObservations {
        clause: clause_identity,
        current: None,
        pre: Some(Observation {
            identity: pre_read.identity,
            environment: pre_admitted.environment,
            populations: pre_admitted.completeness,
        }),
        post: Some(Observation {
            identity: post_read.identity,
            environment: post_admitted.environment,
            populations: post_admitted.completeness,
        }),
        self_object: self_reference,
        parameters,
        result,
        created,
        deleted,
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
            refuse(admission_record("invalid_runtime_input", "invalid-value").with("field", "key"))
        })?;
        let universe = population_universe(views, type_identity)?;
        Ok(ObjectReference::new(universe, type_identity, object))
    }

    pub(crate) fn admission_record(code: &'static str, cause: &'static str) -> AdmissionRecord {
        AdmissionRecord::new(code, cause)
    }
}
