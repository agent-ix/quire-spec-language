// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-153: query explicit closed model environments (`allInstances<T>(p)`,
//! `lookup<T>(p, r) absent m`) over an explicit population binding admitted
//! from one FCD FR-121 population document.
//!
//! This module builds the object population [`crate::model::systems`]'s own
//! module docs name as out of its scope ("nothing in `crate::model` builds an
//! object population yet; that is FR-153's own territory"). It holds no
//! ambient registry: every query is answered from a caller-constructed
//! [`PopulationDocument`] admitted into a [`PopulationBinding`] by
//! [`admit_binding`], never from a process-global lookup, and resolution is
//! always by declared identity (a [`DeclarationKey`]/[`EffectiveId`]), never by
//! display name.
//!
//! # Two independent meters
//!
//! Binding admission (`modelIdentity`, object/subtype closure,
//! `binding.member`, `binding.subset-value`) is metered by this module's own
//! [`PopulationAdmissionLimits`]/[`AdmissionMeter`], independent of
//! `ScalarLimitsV1` and `ModelNormalizationLimitsV1`
//! (`value-accounting.md`, "Population admission limits"). Once admitted,
//! [`all_instances`] and [`lookup`] evaluate against `quire.value.accounting/v1`
//! (`crate::value`'s `Meter`/`ChargePoint`), charging the `lookup.*`,
//! `population.visit` and `collection.*` schedules that module already owns.
//!
//! # `binding.subset-value`
//!
//! `value-accounting.md` charges `binding.subset-value` once per admitted
//! member's subsetting-feature value, in canonical reference-key order,
//! then each subsetting field reaching that member's most-specific type
//! ascending by declaration key, then each value of the subsetting feature
//! in its declared order; `work_units += max(1, n)` where `n` is the number
//! of values of the *subsetted* feature the check scans, and a value absent
//! from the subsetted feature's values refuses `invalid_runtime_input`/
//! `subsetting-violation`. Unlike a real FR-146 postcondition clause (which
//! `crate::model::conformance`'s own refinement-obligation check derives via
//! that crate's fact machinery, see its module docs), a population
//! document's field values are not an expression to evaluate at all — they
//! are declared runtime data (FCD FR-121), exactly like [`PopulationMember`]'s
//! existing `object`/`type_identity` — so [`PopulationMember::field_values`]
//! carries them directly and this check needs no expression evaluator.
//!
//! # Typed, bounded Outputs
//!
//! FR-153's own Outputs clause promises "a typed reference, option, or
//! bounded collection", never a bare, unbounded container. [`all_instances`]
//! returns a [`ReferenceSet`] (its element type `T` plus a
//! [`CardinalityBound`] `[0, N]`, reusing `crate::value`'s own FR-144 bound
//! type rather than a plain comparison) and [`lookup`] returns a
//! [`TypedReference`] (the queried type `T` alongside the realized key).
//! Neither wraps its members as `crate::value::Value::Reference`
//! (`ObjectReference`): that type's identity components
//! (`NodeKey`/`UniverseIdentity`/`ObjectIdentity`, `crate::value::reference`)
//! belong to FR-143's own closed object environment, so this rung's typed
//! wrappers stay in `crate::model`'s own identity domain rather than
//! constructing that type themselves. `crate::value::model_query` is the
//! documented canonical encoding this module's own module docs once called
//! for: FR-143 defines a reference's `universe` and most-specific `type` as
//! literally this crate's own `quire.model.object-universe/v1` and
//! `quire.model.effective-declaration/v1` digests, so that bridge is a
//! direct byte transfer between `EffectiveId`/`DeclarationKey` and
//! `NodeKey`/`UniverseIdentity`, never a re-hash.
//!
//! # Binding/domain package correspondence
//!
//! A [`PopulationBinding`]'s fields are private; [`admit_binding`] is its
//! only constructor, and it stores the admitted `DomainPackage` itself (as an
//! `Arc`, cheap to clone) alongside the universe that domain package normalized to.
//! [`all_instances`] and [`lookup`] read that bound domain package — neither takes a
//! separate `domain_package` argument — so there is no mismatched-domain-package class to
//! guard against at query time: a query can only ever be evaluated against
//! the exact domain package [`admit_binding`] admitted, by construction, not by a
//! runtime comparison.
//!
//! # Invocation admission
//!
//! [`admit_invocation`] admits one operation invocation's pre and post
//! [`PopulationDocument`]s (via two independent [`admit_binding`] calls,
//! each metered by its own [`AdmissionMeter`]) and attaches the pre binding
//! to the post binding as its FR-153 [`PopulationBinding::pre_anchor`], so a
//! `pre(..)`-anchored `allInstances`/`lookup` reads exactly the admitted pre
//! population (`tests/model_reference_queries.rs`'s `l07_pre_*` tests). It
//! then runs two distinct checks over the two admitted bindings, both by
//! `enforce_frame`:
//!
//! - the FR-151 operation frame (`quire.model.conformance.effect/v1`):
//!   every created and deleted object identity's most-specific type must
//!   conform to a declared `creates`/`deletes` grant, and every changed
//!   field on a surviving object must be a declared `modifies` member
//!   (directly, or reaching one through a member's own `redefines` property
//!   — FR-151's own effect-inclusion rule). A field's declared collection kind
//!   (`Multiplicity::ordered`) decides whether its pre/post values compare
//!   by exact sequence or by order-insensitive multiset, so re-serializing
//!   an unordered field in a different order is never itself a write. An
//!   object whose most-specific type differs between pre and post is
//!   refused outright — FR-151's effect vocabulary has no "retype" grant,
//!   and FR-143 binds the most-specific type into an object reference's own
//!   identity, so a type change is never silently reinterpreted as an
//!   unrelated delete-plus-create;
//! - FR-046's own delta agreement: the invocation's caller-supplied
//!   created/deleted identity lists (declared once per invocation, outside
//!   any operation's own effect) must be internally consistent (no
//!   duplicate, no identity declared both created and deleted) and must
//!   equal the complete created/deleted sets `enforce_frame` computed from
//!   the two documents.
//!
//! A frame violation is `Refused` with `Code::FrameViolation`/cause
//! `unauthorized-change`; a delta disagreement is `Refused` with
//! `Code::PopulationDeltaMismatch`/cause `delta-disagreement` — both the
//! catalogued `native-diagnostics.md` causes for their codes, and the exact
//! causes the native runtime's own `src/runtime/validation/frames.rs` uses
//! for the equivalent violations over its own budget-metered types (see
//! [`admit_invocation`]'s own "Native duplication" doc for why that module's
//! decision logic is mirrored here rather than called into directly).
//!
//! Switching a `pre(..)` expression's evaluation anchor between pre and post
//! (`crate::value::expression::evaluate`'s `Anchor`/`Task::RestoreAnchor`) is
//! not itself a charge: `value-accounting.md`'s "Model and graph evaluation"
//! paragraph charges only static resolution, conformance, closure and
//! foreign checks, and binding admission; selecting which already-admitted
//! binding a query reads is a structural dispatch over that already-charged
//! data, not a new evaluation step.
#![allow(
    clippy::large_enum_variant,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline"
)]
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use crate::absence::AbsenceMode;
use crate::diagnostic::Code;
use crate::model::conformance::{generals_by_specific, type_conforms};
use crate::model::dispatch::GeneralizationClosure;
use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, FieldMemberRecord,
    OperationEffect,
};
use crate::model::key::{hex, DeclarationKey, EffectiveId};
use crate::model::normalize::{
    object_universe, EffectiveView, ModelRefusal, ModelRefusalCause, OfferedSelection,
};
use crate::value::CardinalityBound;
use quire_exact::{
    length_amount, Charge as ScalarCharge, ChargePoint as ScalarChargePoint,
    Incomplete as ScalarIncomplete, Integer, LimitKind as ScalarLimitKind, Meter as ScalarMeter,
};

// ---------------------------------------------------------------------------
// PopulationAdmissionLimitsV1
// ---------------------------------------------------------------------------

/// `PopulationAdmissionLimitsV1`: exact `u64` limits for FR-153 binding
/// admission, independent of `ScalarLimitsV1` and `ModelNormalizationLimitsV1`.
/// Every member is required; zero is a real limit and never means unlimited.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PopulationAdmissionLimits {
    /// High-water count of population-document member records charged.
    pub population_members: u64,
    /// Cumulative admission work units.
    pub work_units: u64,
}

impl PopulationAdmissionLimits {
    /// A limit set large enough that no charge in this module is denied.
    pub const UNLIMITED: Self = Self {
        population_members: u64::MAX,
        work_units: u64::MAX,
    };
}

/// One counter of [`PopulationAdmissionLimits`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AdmissionLimitKind {
    /// `population_members`.
    PopulationMembers,
    /// `work_units`.
    WorkUnits,
}

impl AdmissionLimitKind {
    /// Every counter, in `PopulationAdmissionLimitsV1` field order.
    pub const ALL: [Self; 2] = [Self::PopulationMembers, Self::WorkUnits];

    /// Normative member name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PopulationMembers => "population_members",
            Self::WorkUnits => "work_units",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::PopulationMembers => 0,
            Self::WorkUnits => 1,
        }
    }

    fn limit(self, limits: &PopulationAdmissionLimits) -> u64 {
        match self {
            Self::PopulationMembers => limits.population_members,
            Self::WorkUnits => limits.work_units,
        }
    }
}

/// A normative named FR-153 binding-admission charge point.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AdmissionChargePoint {
    /// `binding.member`.
    BindingMember,
    /// `binding.subset-value`; see the module docs.
    BindingSubsetValue,
}

impl AdmissionChargePoint {
    /// Every named point this schedule defines.
    pub const ALL: [Self; 2] = [Self::BindingMember, Self::BindingSubsetValue];

    /// Normative identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BindingMember => "binding.member",
            Self::BindingSubsetValue => "binding.subset-value",
        }
    }
}

/// The exact incomplete record: `incomplete { limit_kind, limit, consumed,
/// next_charge, charge_point }`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AdmissionIncomplete {
    /// First unavailable counter in `PopulationAdmissionLimitsV1` field order.
    pub limit_kind: AdmissionLimitKind,
    /// That counter's configured limit.
    pub limit: u64,
    /// That counter's consumed value before the denied charge.
    pub consumed: u64,
    /// The denied amount.
    pub next_charge: u64,
    /// The named point whose charge was denied.
    pub charge_point: AdmissionChargePoint,
}

struct AdmissionCharge {
    point: AdmissionChargePoint,
    size: Option<u64>,
    work_units: u64,
}

impl AdmissionCharge {
    fn new(point: AdmissionChargePoint) -> Self {
        Self {
            point,
            size: None,
            work_units: 1,
        }
    }

    fn size(mut self, amount: u64) -> Self {
        self.size = Some(amount);
        self
    }

    /// Overrides the default one work unit, for a charge whose cost is not
    /// flat (`binding.subset-value`'s own `max(1, n)`).
    fn work(mut self, amount: u64) -> Self {
        self.work_units = amount;
        self
    }
}

/// A per-binding-admission scalar meter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionMeter {
    limits: PopulationAdmissionLimits,
    consumed: [u64; 2],
    admitted: Vec<AdmissionChargePoint>,
}

impl AdmissionMeter {
    /// A fresh meter with nothing consumed.
    pub fn new(limits: PopulationAdmissionLimits) -> Self {
        Self {
            limits,
            consumed: [0; 2],
            admitted: Vec::new(),
        }
    }

    /// The configured limits.
    pub fn limits(&self) -> &PopulationAdmissionLimits {
        &self.limits
    }

    /// Consumed value of one counter.
    pub fn consumed(&self, kind: AdmissionLimitKind) -> u64 {
        self.consumed[kind.index()]
    }

    /// Every admitted charge point in admission order.
    pub fn admitted_charges(&self) -> &[AdmissionChargePoint] {
        &self.admitted
    }

    fn incomplete(
        &self,
        kind: AdmissionLimitKind,
        next: u64,
        point: AdmissionChargePoint,
    ) -> AdmissionIncomplete {
        AdmissionIncomplete {
            limit_kind: kind,
            limit: kind.limit(&self.limits),
            consumed: self.consumed(kind),
            next_charge: next,
            charge_point: point,
        }
    }

    /// Atomically admit `charge` or return the first unavailable counter in
    /// `PopulationAdmissionLimitsV1` field order. Every counter's resulting
    /// value is computed before any counter is mutated.
    fn charge(&mut self, charge: AdmissionCharge) -> Result<(), AdmissionIncomplete> {
        let point = charge.point;
        if let Some(amount) = charge.size {
            let kind = AdmissionLimitKind::PopulationMembers;
            if amount > kind.limit(&self.limits) {
                return Err(self.incomplete(kind, amount, point));
            }
        }
        let work_total = match self
            .consumed(AdmissionLimitKind::WorkUnits)
            .checked_add(charge.work_units)
        {
            Some(total) if total <= self.limits.work_units => total,
            _ => {
                return Err(self.incomplete(
                    AdmissionLimitKind::WorkUnits,
                    charge.work_units,
                    point,
                ))
            }
        };
        if let Some(amount) = charge.size {
            let slot = &mut self.consumed[AdmissionLimitKind::PopulationMembers.index()];
            *slot = (*slot).max(amount);
        }
        self.consumed[AdmissionLimitKind::WorkUnits.index()] = work_total;
        self.admitted.push(point);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Reference keys and population documents
// ---------------------------------------------------------------------------

/// A closed-environment `Reference<T>` value and FR-153's member key: the
/// FR-204 triple `(universe, effective type identity of the most-specific
/// type, object identity)`. Field declaration order gives canonical
/// reference-key order under `derive(Ord)` exactly: universe, then type, then
/// object bytes with a proper prefix first (TC-198 L09). Each identity
/// component's domain string is constant per component, so comparing raw
/// digest bytes (as [`EffectiveId`]'s own derived `Ord` already does) is
/// equivalent to comparing the domain-prefixed FR-204 key bytes L09 states.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ReferenceKey {
    /// The object universe this reference is a member of
    /// (`quire.model.object-universe/v1` identity).
    pub universe: EffectiveId,
    /// The effective identity of the referenced object's most-specific type.
    pub type_identity: EffectiveId,
    /// The object's own declared identity.
    pub object: String,
}

/// One reference-valued field's runtime values on a population member: the
/// declared identities of the other population members it names, in the
/// document's declared order. `binding.subset-value` (see the module docs)
/// is this rung's only consumer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberFieldValues {
    /// The field's own original producer key.
    pub field: DeclarationKey,
    /// The named objects' own declared identities, in declared order.
    pub values: Vec<String>,
}

/// One member of an FCD FR-121 population document:
/// `{object, type_identity, field_values}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationMember {
    /// The object's own declared identity.
    pub object: String,
    /// The member's declared most-specific type, as its original producer key.
    pub type_identity: DeclarationKey,
    /// This member's declared reference-valued field values, for FR-151's
    /// runtime subsetting check. Empty for a member that declares no
    /// subsetting-relevant field.
    pub field_values: Vec<MemberFieldValues>,
}

/// An FCD FR-121 population document: `{model_identity, members}`. The
/// population's extent (FR-153, FR-208:50) is not part of this runtime
/// document; it is declared once, statically, on the domain package's own
/// [`crate::model::domain_package::PopulationRecord`], which [`admit_binding`]
/// resolves from `domain_package.records` by key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationDocument {
    /// The document's own declared `modelIdentity`, which must name the
    /// binding's `DomainPackageRef` identity.
    pub model_identity: String,
    /// Member records, in document order.
    pub members: Vec<PopulationMember>,
}

/// A `lookup<T>(p, r) absent m` key parameter: `r: Reference<S>`'s own
/// realized identity triple, read directly off `r`'s checked value, exactly
/// as supplied. `r` is supplied by an untrusted producer, so not every
/// component is necessarily well-formed, and [`lookup`] itself -- never this
/// type -- decides which: `universe` is compared against `binding`'s own by
/// raw bytes (a length other than 32 can never equal a real universe, so
/// [`lookup`]'s own byte comparison already decides it correctly with no
/// separate malformed case); `object` is raw identity bytes, and [`lookup`]
/// only treats them as a candidate member identity when they are valid
/// UTF-8 -- every real population member's own object identity is a JSON
/// string (`PopulationDocument`'s member records), so bytes that are not
/// valid UTF-8 can never equal one. This type carries no classification of
/// its own (no public "this identity is malformed"
/// constructor) precisely so a caller cannot assert a false classification
/// for well-formed bytes and force a present member to read as absent, or
/// the reverse. `type_identity` is always well-formed
/// (`crate::value::node::NodeKey` is a fixed 32-byte digest already, so
/// there is nothing to bridge).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LookupKey {
    /// `r`'s own static type `S`.
    pub static_type: DeclarationKey,
    /// `r`'s realized reference's universe, exactly as supplied.
    pub universe: Vec<u8>,
    /// `r`'s realized reference's most-specific type.
    pub type_identity: EffectiveId,
    /// `r`'s realized reference's own declared object identity, exactly as
    /// supplied -- well-formed (valid UTF-8) or not. [`lookup`] alone decides
    /// which.
    pub object: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Binding admission
// ---------------------------------------------------------------------------

/// The declared values of `member`'s `field`, or an empty slice when
/// `member` declares no values for it. Shared by [`admit_binding`]'s
/// `binding.subset-value` check and [`enforce_frame`]'s field-write check —
/// both read exactly this same `PopulationMember.field_values` shape.
fn values_of<'a>(member: &'a PopulationMember, field: &DeclarationKey) -> &'a [String] {
    member
        .field_values
        .iter()
        .find(|entry| &entry.field == field)
        .map_or(&[][..], |entry| entry.values.as_slice())
}

/// One admitted population binding: every admitted member, keyed by
/// [`ReferenceKey`] (so iteration is already canonical reference-key order),
/// with each member's original most-specific type retained for conformance
/// checking.
///
/// Every field is private; [`admit_binding`] is this type's only
/// constructor, so a caller cannot assemble a binding that was never checked
/// against a domain package's `modelIdentity`, object closure or subtype closure —
/// and, because the admitted domain package itself lives here, cannot later evaluate
/// [`all_instances`]/[`lookup`] against any domain package other than the one that
/// was checked (see the module docs' "Binding/domain package correspondence").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationBinding {
    /// The domain package this binding was admitted against. `Arc`, so cloning a
    /// binding never re-clones the domain package's own records.
    domain_package: Arc<DomainPackage>,
    /// This binding's own object universe, computed once at admission.
    universe: EffectiveId,
    /// Every admitted member, ascending by [`ReferenceKey`].
    members: BTreeMap<ReferenceKey, DeclarationKey>,
    /// The binding's declared maximum, or `None` for a binding with no
    /// declared maximum (`allInstances` is then `operator-ineligible`).
    declared_maximum: Option<u64>,
    /// `domain_package`'s declared `supertypes[]` generals, indexed by
    /// specific, computed once here rather than by [`all_instances`]/[`lookup`]
    /// on every call.
    /// `value-accounting.md`'s "Model and graph evaluation" paragraph
    /// already places the type-conformance decision this index serves
    /// outside any charge ("...selects the member, without a charge, exactly
    /// when that type conforms to `T`"), so this is a one-time efficiency
    /// fix, not a new charge: the same "compute an index once at admission
    /// instead of once per lookup" move [`admit_binding`] already makes for
    /// `type_lookup`/`by_object` below.
    generals: HashMap<DeclarationKey, Vec<DeclarationKey>>,
    /// Every declared object type of `domain_package`'s effective view, `DeclarationKey`
    /// to its FR-150-derived [`EffectiveId`]. Computed once here from
    /// [`admit_binding`]'s own `type_lookup` (identical to it, retained
    /// rather than discarded): the FR-143 reference-identity bridge
    /// (`crate::value::model_query`) needs this exact
    /// correspondence to translate a checked `Reference<T>`'s `T`
    /// (a `crate::value::NodeKey`, the same 32 bytes as an `EffectiveId`)
    /// back into the `DeclarationKey` [`all_instances`]/[`lookup`] take, for
    /// every declared type, not only ones a current member happens to name.
    type_catalog: BTreeMap<DeclarationKey, EffectiveId>,
    /// FR-153's invocation pre population, attached only by
    /// [`admit_invocation`]: `pre(allInstances(p))`/`pre(lookup(p, r) absent
    /// m)` read this binding instead of `self` underneath a `pre(..)`
    /// anchor. `None` for a binding admitted directly by [`admit_binding`]
    /// (no invocation, so no pre state to anchor to). One level of `Arc`
    /// indirection, never chained: the pre binding [`admit_invocation`]
    /// attaches here is itself always admitted with `pre_anchor: None`.
    pre_anchor: Option<Arc<PopulationBinding>>,
}

impl PopulationBinding {
    /// Attaches `pre` as this binding's FR-153 invocation pre population,
    /// consuming both. Only [`admit_invocation`] calls this: attaching an
    /// arbitrary binding here would let a caller assemble a `pre`/`post`
    /// pair that was never checked against one invocation's frame.
    fn with_pre_anchor(mut self, pre: PopulationBinding) -> Self {
        self.pre_anchor = Some(Arc::new(pre));
        self
    }

    /// This binding's attached FR-153 invocation pre population, when one
    /// was attached by [`admit_invocation`].
    pub fn pre_anchor(&self) -> Option<&PopulationBinding> {
        self.pre_anchor.as_deref()
    }
    /// The full `DomainPackageRef` header this binding was admitted against.
    pub fn model_selection(&self) -> &DomainPackageRef {
        &self.domain_package.model_selection
    }

    /// The `DomainPackageRef` identity this binding was admitted against.
    pub fn model_identity(&self) -> &str {
        &self.domain_package.model_selection.identity
    }

    /// This binding's own object universe.
    pub fn universe(&self) -> &EffectiveId {
        &self.universe
    }

    /// Every admitted member, ascending by [`ReferenceKey`].
    pub fn members(&self) -> &BTreeMap<ReferenceKey, DeclarationKey> {
        &self.members
    }

    /// The binding's declared maximum, or `None` for a binding with no
    /// declared maximum (`allInstances` is then `operator-ineligible`).
    pub fn declared_maximum(&self) -> Option<u64> {
        self.declared_maximum
    }

    /// Every declared object type of the admitted effective view,
    /// `DeclarationKey` to its FR-150-derived [`EffectiveId`]; see the field's
    /// own doc comment.
    pub fn type_catalog(&self) -> &BTreeMap<DeclarationKey, EffectiveId> {
        &self.type_catalog
    }
}

/// The outcome of one [`admit_binding`] attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmissionOutcome {
    /// Every member was charged and admitted.
    Admitted(PopulationBinding),
    /// A real defect refused admission outright; no binding.
    Refused(ModelRefusal),
    /// Object or subtype closure is not established. Not a refusal
    /// (`Code::IncompletePopulation`); no charge and no binding.
    UnknownClosure(ModelRefusal),
    /// A `PopulationAdmissionLimitsV1` counter was exhausted; no binding.
    Incomplete(AdmissionIncomplete),
}

/// Admits `document` against `domain_package`/`view`/`population_key` into a
/// [`PopulationBinding`], per FR-153's "Environment key and closure". Decides,
/// without a charge and in order, `view`'s correspondence to `domain_package`,
/// the `modelIdentity` check, `population_key`'s own resolution against
/// `domain_package.records`, object closure (the resolved
/// [`crate::model::domain_package::PopulationRecord::extent`]) and subtype
/// closure; then charges
/// `binding.member` for each member record in document order, deciding
/// foreign-type and then duplicate-collapse/conflicting-identity after each
/// charge. Stops at the first refusal or denied charge.
///
/// `view` and `domain_package` are supplied separately (`view` is FR-150's own
/// already-normalized, already-charged effective view of `domain_package`, computed
/// by the caller under `ModelNormalizationLimitsV1` before this call), so
/// nothing before this check ensures the two actually correspond: a caller
/// could pass a `view` normalized from a different domain package than the one named
/// here. Re-normalizing `domain_package` here to check would duplicate the caller's
/// own already-charged normalization work under the wrong meter (this
/// module's own "Two independent meters" docs), so this compares the two
/// values' own `model_selection` headers instead — the same closed-catalog
/// `foreign_reference`/`foreign-model-selection` cause the `modelIdentity`
/// check below already uses for a DomainPackageRef-key mismatch, and the same
/// boundary this crate already trusts at that check (`identity`
/// without content verification) — comparing the *full* header rather than
/// only `identity` so a version-only divergence is caught too.
///
/// `population_key` is a key, not a caller-supplied
/// [`crate::model::domain_package::PopulationRecord`]:
/// FR-153's "Its declaration key must belong to the binding's ModelSelection"
/// means the population's declared extent is a fact of `domain_package`
/// itself, never a value the caller states independently of it. This
/// function resolves the record from `domain_package.records` and refuses
/// `foreign_reference`/`foreign-model-selection` when no `Population` record
/// there carries that key, so a caller cannot claim an extent, or select some
/// other package's population declaration, that `domain_package` did not
/// itself declare.
pub fn admit_binding(
    domain_package: &DomainPackage,
    view: &EffectiveView,
    document: &PopulationDocument,
    population_key: &DeclarationKey,
    subtype_closure: GeneralizationClosure,
    declared_maximum: Option<u64>,
    meter: &mut AdmissionMeter,
) -> AdmissionOutcome {
    if *view.model_selection() != domain_package.model_selection {
        return AdmissionOutcome::Refused(ModelRefusal {
            code: Code::ForeignReference,
            cause: ModelRefusalCause::ForeignModelSelection {
                actual: OfferedSelection::View(view.model_selection().clone()),
                expected: domain_package.model_selection.clone(),
            },
            detail: format!(
                "effective view was normalized under model selection {}, not the admitting domain package's {}",
                view.model_selection().identity, domain_package.model_selection.identity
            ),
        });
    }
    if document.model_identity != domain_package.model_selection.identity {
        return AdmissionOutcome::Refused(ModelRefusal {
            code: Code::ForeignReference,
            cause: ModelRefusalCause::ForeignModelSelection {
                actual: OfferedSelection::Document(document.model_identity.clone()),
                expected: domain_package.model_selection.clone(),
            },
            detail: format!(
                "population document names modelIdentity {}, not the binding's {}",
                document.model_identity, domain_package.model_selection.identity
            ),
        });
    }
    let Some(population) = domain_package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::Population(population) if population.key == *population_key => {
                Some(population)
            }
            _ => None,
        })
    else {
        return AdmissionOutcome::Refused(ModelRefusal {
            code: Code::ForeignReference,
            cause: ModelRefusalCause::ForeignModelSelection {
                actual: OfferedSelection::Population(population_key.clone()),
                expected: domain_package.model_selection.clone(),
            },
            detail: format!(
                "population key {} names no Population declaration of domain package {}",
                population_key.node, domain_package.model_selection.identity
            ),
        });
    };
    if population.extent != Extent::Closed {
        return AdmissionOutcome::UnknownClosure(ModelRefusal {
            code: Code::IncompletePopulation,
            cause: ModelRefusalCause::IncompleteScope {
                selection: domain_package.model_selection.identity.clone(),
            },
            detail: format!(
                "population {} for {} does not declare extent: closed",
                population.key.node, domain_package.model_selection.identity
            ),
        });
    }
    if matches!(subtype_closure, GeneralizationClosure::Open) {
        return AdmissionOutcome::UnknownClosure(ModelRefusal {
            code: Code::IncompletePopulation,
            cause: ModelRefusalCause::UnclosedSubtypes {
                selection: domain_package.model_selection.identity.clone(),
                type_name: document
                    .members
                    .first()
                    .map(|member| member.type_identity.clone()),
            },
            detail: format!(
                "model selection {} naming {} does not have a closed generalization graph",
                domain_package.model_selection.identity,
                document
                    .members
                    .first()
                    .map(|member| member.type_identity.node.as_str())
                    .unwrap_or("<no members>")
            ),
        });
    }

    let universe = match object_universe(domain_package) {
        Ok(universe) => universe.identity(),
        // `object_universe` now returns its full charge-ordered refusal
        // bundle (L2 finding, PR #228 review); this FR-153 admission path
        // stays single-refusal (`AdmissionOutcome::Refused`'s own shape,
        // unchanged here), so only the first is surfaced, exactly as before
        // this fix. `Refusals::into_first` reads it directly (M2 finding, PR
        // #228 round 2 review): `Refusals` is non-empty by construction, so
        // there is no empty case left to `.expect()` past.
        Err(refusals) => return AdmissionOutcome::Refused(refusals.into_first()),
    };

    // Indexed once, not re-scanned per member: `view.declarations()` and
    // `admitted` would otherwise each be linearly searched per member,
    // making admission O(n^2) against the O(n) `binding.member` charges it
    // records.
    let type_lookup: BTreeMap<DeclarationKey, EffectiveId> = view
        .declarations()
        .iter()
        .filter(|entry| entry.preimage.owner_effective_type.is_none())
        .map(|entry| (entry.preimage.original.clone(), entry.effective_id))
        .collect();

    // Computed once here rather than once per `all_instances`/`lookup` call;
    // see the `generals` field's own doc comment.
    let generals = generals_by_specific(domain_package);

    // D05 (`model-complete.md:156`): indexed once, alongside `type_lookup`
    // above, so the abstract-instance check below is a lookup rather than a
    // rescan of `domain_package.records` per member.
    let abstract_types: BTreeMap<DeclarationKey, bool> = domain_package
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::ObjectType(object) => {
                Some((object.key.clone(), object.abstract_type))
            }
            DomainPackageRecord::FieldMember(_)
            | DomainPackageRecord::ScalarType(_)
            | DomainPackageRecord::OperationMember(_)
            | DomainPackageRecord::Component(_)
            | DomainPackageRecord::Endpoint(_)
            | DomainPackageRecord::Relationship(_)
            | DomainPackageRecord::Allocation(_)
            | DomainPackageRecord::Population(_) => None,
        })
        .collect();

    let mut admitted: BTreeMap<ReferenceKey, DeclarationKey> = BTreeMap::new();
    let mut by_object: BTreeMap<String, ReferenceKey> = BTreeMap::new();
    for (position, member) in document.members.iter().enumerate() {
        let k = length_amount(position + 1);
        if let Err(incomplete) =
            meter.charge(AdmissionCharge::new(AdmissionChargePoint::BindingMember).size(k))
        {
            return AdmissionOutcome::Incomplete(incomplete);
        }

        let Some(effective_type) = type_lookup.get(&member.type_identity) else {
            return AdmissionOutcome::Refused(ModelRefusal {
                code: Code::ForeignReference,
                cause: ModelRefusalCause::ForeignType {
                    member: member.object.clone(),
                    type_name: member.type_identity.clone(),
                },
                detail: format!(
                    "member {} names type {}, absent from the effective view",
                    member.object, member.type_identity.node
                ),
            });
        };

        // FR-153:57/:72: a member type not covered by the population's own
        // declared `member_types` is foreign, even when it exists elsewhere
        // in the domain package. "Covered" is the member type itself, or a
        // type conforming to a declared member type.
        let mut covered = false;
        for declared in &population.member_types {
            match type_conforms(&generals, &member.type_identity, declared) {
                Ok(true) => {
                    covered = true;
                    break;
                }
                Ok(false) => {}
                Err(refusal) => return AdmissionOutcome::Refused(refusal),
            }
        }
        if !covered {
            return AdmissionOutcome::Refused(ModelRefusal {
                code: Code::ForeignReference,
                cause: ModelRefusalCause::ForeignType {
                    member: member.object.clone(),
                    type_name: member.type_identity.clone(),
                },
                detail: format!(
                    "member {} names type {}, not covered by population {}'s member types",
                    member.object, member.type_identity.node, population.key.node
                ),
            });
        }

        // D05 (`model-complete.md:156`): a member whose most-specific type
        // is abstract has no direct instances.
        if abstract_types
            .get(&member.type_identity)
            .copied()
            .unwrap_or(false)
        {
            return AdmissionOutcome::Refused(ModelRefusal {
                code: Code::InvalidRuntimeInput,
                cause: ModelRefusalCause::AbstractInstance {
                    member: member.object.clone(),
                    abstract_type: member.type_identity.clone(),
                },
                detail: format!(
                    "member {} names abstract type {}, which has no direct instances",
                    member.object, member.type_identity.node
                ),
            });
        }

        let key = ReferenceKey {
            universe,
            type_identity: *effective_type,
            object: member.object.clone(),
        };
        if let Some(existing) = by_object.get(&key.object) {
            if *existing == key {
                continue; // exact duplicate: equal key, equal record content, collapses.
            }
            return AdmissionOutcome::Refused(ModelRefusal {
                code: Code::InvalidRuntimeInput,
                cause: ModelRefusalCause::ConflictingIdentity {
                    object: key.object.clone(),
                    existing_type: existing.type_identity,
                    declared_type: key.type_identity,
                },
                detail: format!(
                    "object {} is declared with conflicting types {} and {}",
                    key.object, existing.type_identity, key.type_identity
                ),
            });
        }
        by_object.insert(key.object.clone(), key.clone());
        admitted.insert(key, member.type_identity.clone());
    }

    // `binding.subset-value`: once per admitted member's subsetting-feature
    // value, in canonical reference-key order (`admitted` iterates in that
    // order already), then each subsetting field reaching that member's
    // most-specific type ascending by declaration key, then each value of
    // the subsetting feature in its declared order (see the module docs).

    /// The first field named by more than one of `member`'s own
    /// `field_values` entries, or `None` when every named field is unique.
    /// [`values_of`]'s `find` keeps only the first matching entry, so an
    /// unrejected duplicate would silently discard the others rather than
    /// refusing.
    fn duplicate_field(member: &PopulationMember) -> Option<&DeclarationKey> {
        let mut seen: Vec<&DeclarationKey> = Vec::new();
        for entry in &member.field_values {
            if seen.iter().any(|existing| **existing == entry.field) {
                return Some(&entry.field);
            }
            seen.push(&entry.field);
        }
        None
    }

    /// One subsetting edge derived from a field member's own inline
    /// `subsets[]` property (`model-complete.md`:161): `owner` declares
    /// `subsetting`, whose runtime values must be a subset of `subsetted`'s.
    struct SubsettingEdge<'a> {
        owner: &'a DeclarationKey,
        subsetting: &'a DeclarationKey,
        subsetted: &'a DeclarationKey,
    }
    let subsetting_edges: Vec<SubsettingEdge<'_>> = domain_package
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::FieldMember(field) if !field.subsets.is_empty() => Some(field),
            _ => None,
        })
        .flat_map(|field: &FieldMemberRecord| {
            field.subsets.iter().map(move |subsetted| SubsettingEdge {
                owner: &field.owner,
                subsetting: &field.key,
                subsetted,
            })
        })
        .collect();
    if !subsetting_edges.is_empty() {
        let member_by_object: HashMap<&str, &PopulationMember> = document
            .members
            .iter()
            .map(|member| (member.object.as_str(), member))
            .collect();

        for (key, original_type) in &admitted {
            let Some(&member) = member_by_object.get(key.object.as_str()) else {
                continue;
            };
            if let Some(field) = duplicate_field(member) {
                return AdmissionOutcome::Refused(ModelRefusal {
                    code: Code::InvalidRuntimeInput,
                    // FR-272's `invalid_runtime_input` cause list is
                    // closed; there is no dedicated duplicate-field
                    // variant, so this is the catalogued `duplicate-member`.
                    cause: ModelRefusalCause::DuplicateMember {
                        object: key.object.clone(),
                        field: field.clone(),
                    },
                    detail: format!(
                        "object {} declares field {} more than once in its field_values",
                        key.object, field.node
                    ),
                });
            }
            let mut applicable: Vec<&SubsettingEdge<'_>> = Vec::new();
            for edge in &subsetting_edges {
                match type_conforms(&generals, original_type, edge.owner) {
                    Ok(true) => applicable.push(edge),
                    Ok(false) => {}
                    Err(refusal) => return AdmissionOutcome::Refused(refusal),
                }
            }
            applicable.sort_by(|a, b| a.subsetting.cmp(b.subsetting));

            for edge in applicable {
                let subsetting_values = values_of(member, edge.subsetting);
                let subsetted_values = values_of(member, edge.subsetted);
                for value in subsetting_values {
                    let n = length_amount(subsetted_values.len());
                    if let Err(incomplete) = meter.charge(
                        AdmissionCharge::new(AdmissionChargePoint::BindingSubsetValue)
                            .work(n.max(1)),
                    ) {
                        return AdmissionOutcome::Incomplete(incomplete);
                    }
                    if !subsetted_values.iter().any(|other| other == value) {
                        return AdmissionOutcome::Refused(ModelRefusal {
                            code: Code::InvalidRuntimeInput,
                            cause: ModelRefusalCause::SubsettingViolation {
                                object: key.object.clone(),
                                subsetting: edge.subsetting.clone(),
                                subsetted: edge.subsetted.clone(),
                            },
                            detail: format!(
                                "object {}'s {} names {value}, not among its {} values \
                                 (subsetting field {})",
                                key.object,
                                edge.subsetting.node,
                                edge.subsetted.node,
                                edge.subsetting.node
                            ),
                        });
                    }
                }
            }
        }
    }

    AdmissionOutcome::Admitted(PopulationBinding {
        domain_package: Arc::new(domain_package.clone()),
        universe,
        members: admitted,
        declared_maximum,
        generals,
        type_catalog: type_lookup,
        pre_anchor: None,
    })
}

// ---------------------------------------------------------------------------
// Invocation admission: EXPR-020/021/022's created/deleted identities and
// operation frame
// ---------------------------------------------------------------------------

/// [`admit_invocation`]'s shared admission context for its pre and post
/// bindings: the same domain package, effective view, population declaration,
/// subtype closure and declared maximum -- the same population role,
/// admitted at the invocation's two instants. Grouped into one type rather
/// than five parameters so `admit_invocation` stays within this crate's
/// argument-count convention.
#[derive(Clone, Copy)]
pub struct InvocationContext<'a> {
    /// The domain package both instants are admitted against.
    pub domain_package: &'a DomainPackage,
    /// The domain package's already-normalized, already-charged effective view.
    pub view: &'a EffectiveView,
    /// The population role's own declaration key, resolved against
    /// `domain_package.records` by each [`admit_binding`] call
    /// ([`admit_binding`]'s own `population_key` doc).
    pub population: &'a DeclarationKey,
    /// The model selection's subtype closure.
    pub subtype_closure: GeneralizationClosure,
    /// The population role's declared maximum, or `None`.
    pub declared_maximum: Option<u64>,
}

/// The operation's own declared frame and recorded population delta for one
/// invocation. `effect` is the operation's authored frame
/// (`quire.model.conformance.effect/v1`), enforced by `enforce_frame`.
/// `declared_created`/`declared_deleted` are the invocation's own recorded
/// created/deleted object identities for this population role -- FR-046's
/// "caller-supplied lists must match" (the native runtime's own
/// `declared_deltas`/`inspect_frames`, `src/runtime/validation/frames.rs`),
/// applied here at the model-population layer. Object identity only: this
/// call's own universe is implicit (both admitted bindings share it), and
/// per `enforce_frame`'s own docs, identity for delta purposes is
/// `(universe, object)`, never the most-specific type.
#[derive(Clone, Copy)]
pub struct InvocationDelta<'a> {
    /// The operation's authored frame.
    pub effect: &'a OperationEffect,
    /// The invocation's own recorded created-object identities.
    pub declared_created: &'a [String],
    /// The invocation's own recorded deleted-object identities.
    pub declared_deleted: &'a [String],
}

/// Admits one operation invocation's pre and post [`PopulationDocument`]s
/// against the same `domain_package`/`view`/`population`/`subtype_closure`/
/// `declared_maximum` (the same population role, at the invocation's two
/// instants), then
/// enforces `declared.effect`'s FR-151 frame
/// (`quire.model.conformance.effect/v1`) against the two bindings' created
/// and deleted identities and their surviving members' declared field
/// values, and separately compares those same computed created/deleted sets
/// against `declared.declared_created`/`declared_deleted` (FR-046's
/// caller-supplied delta).
///
/// Either admission failing (`Refused`/`UnknownClosure`/`Incomplete`) is
/// returned as-is; a frame violation is `Refused` with
/// `Code::FrameViolation`/cause `unauthorized-change`, and a delta
/// disagreement (a duplicate or intersecting declared identity, or a
/// declared set that differs from the complete computed one) is `Refused`
/// with `Code::PopulationDeltaMismatch`/cause `delta-disagreement` -- both
/// the catalogued `native-diagnostics.md` causes for their codes, and both
/// the exact causes the native runtime's own `src/runtime/validation/
/// frames.rs` already uses for the equivalent violations (see this
/// function's own "Native duplication" paragraph for why that module's own
/// code is not called directly). On success, the returned binding is the
/// post binding with `pre` attached as its [`PopulationBinding::pre_anchor`],
/// so `allInstances`/`lookup` underneath a `pre(..)` anchor read exactly the
/// admitted pre population.
///
/// `pre_meter`/`post_meter` are two independent [`AdmissionMeter`]s, one per
/// admitted binding: this module's own "Two independent meters" docs already
/// establish that admission and evaluation are metered separately, and
/// nothing in FR-153's Inputs clause defines a single combined budget for
/// admitting two population instants of one invocation, so charging each
/// against its own meter (rather than inventing an unspecified combined
/// schedule) is the conservative reading.
///
/// Deciding created/deleted-identity, field-write frame membership and delta
/// agreement is, like the object/subtype-closure and `modelIdentity` checks
/// [`admit_binding`] already makes, a structural admission decision over
/// already-charged data, not itself a new charge point: neither FR-151's nor
/// FR-153's model module defines one, and the two admitted bindings' own
/// `binding.member`/`binding.subset-value` charges already measured the data
/// this check reads.
///
/// # Native duplication
///
/// `src/runtime/validation/frames.rs`'s `inspect_frames`/`object_frame`/
/// `declared_deltas` decide the identical two violations
/// (`FrameViolation`/`PopulationDeltaMismatch`) this function does, over the
/// native runtime's own budget-metered `Validator`, `ValueId` value arena and
/// `ObjectIdentity`/`FieldBinding` snapshot types. Those types are foreign to
/// `crate::model`: `Validator` is `impl`-private to that module and tightly
/// coupled to its own `budget`/arena plumbing, so there is no call this
/// function could make into it, only a parallel decision over this crate's
/// own `PopulationDocument`/`PopulationMember`/`AdmissionMeter` types. What is
/// shared, deliberately, is every cause: this function reuses
/// `Code::FrameViolation`/`"unauthorized-change"` and
/// `Code::PopulationDeltaMismatch`/`"delta-disagreement"` for the same
/// violations frames.rs names them for, and applies the equivalent decision
/// (declared-identity duplicate, declared-set intersection, computed-vs-
/// declared mismatch; created/deleted-identity and field-write frame
/// coverage) rather than a differently shaped one.
pub fn admit_invocation(
    context: InvocationContext<'_>,
    pre_document: &PopulationDocument,
    post_document: &PopulationDocument,
    declared: &InvocationDelta<'_>,
    pre_meter: &mut AdmissionMeter,
    post_meter: &mut AdmissionMeter,
) -> AdmissionOutcome {
    let pre = match admit_binding(
        context.domain_package,
        context.view,
        pre_document,
        context.population,
        context.subtype_closure,
        context.declared_maximum,
        pre_meter,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => return other,
    };
    let post = match admit_binding(
        context.domain_package,
        context.view,
        post_document,
        context.population,
        context.subtype_closure,
        context.declared_maximum,
        post_meter,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => return other,
    };
    match enforce_frame(&pre, &post, pre_document, post_document, declared) {
        Ok(()) => AdmissionOutcome::Admitted(post.with_pre_anchor(pre)),
        Err(refusal) => AdmissionOutcome::Refused(refusal),
    }
}

/// One [`ModelRefusal`]/`Code::FrameViolation` for `detail`/`cause`, `cause`
/// one of [`ModelRefusalCause`]'s four `unauthorized-change` variants (the
/// catalogued cause tag for this code; see `admit_invocation`'s own docs).
fn frame_violation(cause: ModelRefusalCause, detail: String) -> ModelRefusal {
    ModelRefusal {
        code: Code::FrameViolation,
        cause,
        detail,
    }
}

/// One [`ModelRefusal`]/`Code::PopulationDeltaMismatch` for `detail`/`cause`,
/// `cause` one of [`ModelRefusalCause`]'s three `delta-disagreement`
/// variants (the catalogued cause tag for this code; see
/// `admit_invocation`'s own docs).
fn delta_mismatch(cause: ModelRefusalCause, detail: String) -> ModelRefusal {
    ModelRefusal {
        code: Code::PopulationDeltaMismatch,
        cause,
        detail,
    }
}

/// Whether `field`'s declared multiplicity is `ordered` (`Sequence`/
/// `OrderedSet`); `Bag`/`Set` fields compare by value multiset, ignoring
/// declared order, so a producer that re-serializes an unordered field in a
/// different order between pre and post is not a frame violation. Defaults
/// to `true` (order-sensitive) when `field` has no `FieldMemberRecord` in
/// `domain_package` -- the strict, pre-existing comparison -- since an absent record
/// gives this function no positive basis to relax it.
fn field_ordered(domain_package: &DomainPackage, field: &DeclarationKey) -> bool {
    domain_package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::FieldMember(member) if member.key == *field => {
                Some(member.multiplicity.ordered)
            }
            _ => None,
        })
        .unwrap_or(true)
}

/// Whether `pre_values`/`post_values` (`field`'s declared values on the same
/// member, pre and post) are the same collection, per `field`'s own declared
/// collection kind: exact sequence equality when ordered, multiset equality
/// (order-insensitive, duplicate-count-sensitive) otherwise. A `Set` field's
/// values are already unique by construction, so multiset equality reduces
/// to set equality for it without a separate case.
fn field_values_equal(
    domain_package: &DomainPackage,
    field: &DeclarationKey,
    pre: &[String],
    post: &[String],
) -> bool {
    if field_ordered(domain_package, field) {
        return pre == post;
    }
    let mut pre = pre.to_vec();
    let mut post = post.to_vec();
    pre.sort();
    post.sort();
    pre == post
}

/// Walks `field`'s redefinition chain (one member's own inline `redefines`
/// hop at a time — `model-complete.md`:162), returning `true` as soon as
/// `admits` accepts `field` itself or some ancestor it reaches, `false` once
/// the chain ends with no accepted link. model-complete.md:56: the
/// redefining feature replaces "the *one* inherited redefined feature", so a
/// chain -- `C.x` redefines `B.x`, `B.x` redefines `A.x`, with no direct
/// `C.x -> A.x` edge -- is legal and normal, not an edge case; a single
/// hop only ever reaches an immediate redefinition target, never a
/// grandparent one. `redefinitionClosure: closed` (model-complete.md:64)
/// means every redefinition edge in the model is *listed* here, not that
/// the chain is pre-flattened into direct edges to every ancestor -- this
/// walk is what actually flattens it, at each call site that needs to know.
///
/// Bounded by `records.len()` hops (an acyclic chain can never visit more
/// distinct fields than there are records at all) and refuses -- stops and
/// returns `false`, never loops -- past that bound, so a malformed domain package
/// with a redefinition cycle cannot hang this walk.
///
/// Shared by [`field_write_covered`] here and by
/// `crate::model::conformance`'s effect-escape check
/// (`check_operation_redefinition`'s "Effect" axis). Taking a
/// [`DomainPackageRecord`] slice rather than a whole [`DomainPackage`] lets either call
/// site pass its own already-available `&domain_package.records`. Equality is
/// `DeclarationKey`'s derived `PartialEq` (`package`, `node`, both) at both
/// call sites -- a write naming a field in one package does not reach a
/// grant for the same node in a different package. Pinned by
/// `enforce_frame_refuses_a_field_write_at_a_package_the_declared_grant_does_not_name`
/// here (`tests/model_population.rs`) and by
/// `r10_operation_redefinition_effect_axis_refuses_a_write_at_a_package_the_grant_does_not_name`
/// at the `conformance` call site (`tests/model_conformance.rs`).
pub(super) fn redefinition_reaches(
    records: &[DomainPackageRecord],
    field: &DeclarationKey,
    admits: impl Fn(&DeclarationKey) -> bool,
) -> bool {
    let mut current = field.clone();
    let bound = records.len();
    for _ in 0..=bound {
        if admits(&current) {
            return true;
        }
        let Some(redefined) = records.iter().find_map(|record| match record {
            DomainPackageRecord::FieldMember(member) if member.key == current => {
                member.redefines.clone()
            }
            DomainPackageRecord::OperationMember(member) if member.key == current => {
                member.redefines.clone()
            }
            _ => None,
        }) else {
            return false;
        };
        current = redefined;
    }
    false // Cycle: exceeded the maximum possible acyclic chain length.
}

/// Whether `field` (the field a runtime population document names on some
/// member) is covered by `effect.modifies`, directly or because it reaches
/// one through a chain of members' own `redefines` properties -- FR-151's
/// own effect-inclusion rule (`quire.model.conformance.effect/v1`), applied here to one
/// operation's own declared writes rather than to a redefining operation's
/// writes against its redefined ancestor's. Walks the full chain
/// ([`redefinition_reaches`]), not just one hop: `modifies: [model.A.x]`
/// covers a write to `model.C.x` through `model.C.x -> model.B.x -> model.A.x`.
fn field_write_covered(
    domain_package: &DomainPackage,
    effect: &OperationEffect,
    field: &DeclarationKey,
) -> bool {
    redefinition_reaches(&domain_package.records, field, |candidate| {
        effect.modifies.contains(candidate)
    })
}

/// Enforces `declared.effect`'s frame against `pre`/`post`'s created and
/// deleted identities and their surviving members' declared field values,
/// then compares the computed created/deleted sets against
/// `declared.declared_created`/`declared_deleted` (FR-046's caller-supplied
/// delta). `pre`, `post` are the two admitted bindings; `pre_document`,
/// `post_document` are the raw documents they were admitted from (needed
/// here only for their `field_values`, which [`PopulationBinding`] does not
/// retain).
///
/// Created/deleted classification is by *object identity* (the declared
/// object identity alone), never by [`ReferenceKey`] (which also carries the
/// most-specific type): an object present under the same identity in both
/// `pre` and `post` but with a *different* most-specific type is a type
/// change, decided before either classification runs, immediately below.
/// FR-151's effect vocabulary grants exactly three kinds of authorized
/// change -- `modifies`, `creates`, `deletes` -- and none of them is a
/// retype; FR-143's own "Object references" clause additionally treats the
/// most-specific type as part of a reference's snapshot-supplied identity
/// triple, alongside the universe and declared object identity, never a
/// mutable per-object attribute. A type change therefore has no
/// authorization path under any declared frame and always refuses
/// `FrameViolation`/`unauthorized-change`, regardless of whether `creates`/
/// `deletes` grants happen to cover both the pre and post type -- silently
/// admitting it as an unrelated delete-plus-create would hide exactly the
/// violation this check exists to catch.
fn enforce_frame(
    pre: &PopulationBinding,
    post: &PopulationBinding,
    pre_document: &PopulationDocument,
    post_document: &PopulationDocument,
    declared: &InvocationDelta<'_>,
) -> Result<(), ModelRefusal> {
    let domain_package = &pre.domain_package;
    let pre_by_object: BTreeMap<&str, (&ReferenceKey, &DeclarationKey)> = pre
        .members()
        .iter()
        .map(|(key, type_identity)| (key.object.as_str(), (key, type_identity)))
        .collect();
    let post_by_object: BTreeMap<&str, (&ReferenceKey, &DeclarationKey)> = post
        .members()
        .iter()
        .map(|(key, type_identity)| (key.object.as_str(), (key, type_identity)))
        .collect();

    let mut computed_created: BTreeSet<String> = BTreeSet::new();
    let mut computed_deleted: BTreeSet<String> = BTreeSet::new();

    for (object, (_, post_type)) in &post_by_object {
        let Some((_, pre_type)) = pre_by_object.get(object) else {
            // Created: absent from `pre`. Its most-specific type must
            // conform to a declared `creates` grant.
            let mut allowed = false;
            for grant in &declared.effect.creates {
                if type_conforms(&post.generals, post_type, grant)? {
                    allowed = true;
                    break;
                }
            }
            if !allowed {
                return Err(frame_violation(
                    ModelRefusalCause::FrameCreateOutsideGrant {
                        object: (*object).to_owned(),
                        type_name: (*post_type).clone(),
                    },
                    format!(
                        "invocation creates object {object} of type {}, outside the operation's \
                         declared creates frame",
                        post_type.node
                    ),
                ));
            }
            computed_created.insert((*object).to_owned());
            continue;
        };
        if *pre_type != *post_type {
            return Err(frame_violation(
                ModelRefusalCause::FrameTypeChanged {
                    object: (*object).to_owned(),
                    pre_type: (*pre_type).clone(),
                    post_type: (*post_type).clone(),
                },
                format!(
                    "invocation changes object {object}'s most-specific type from {} to {} \
                     between pre and post, which no operation frame may authorize (FR-151's \
                     effect grants cover modifies/creates/deletes only)",
                    pre_type.node, post_type.node
                ),
            ));
        }
    }

    for (object, (_, pre_type)) in &pre_by_object {
        if post_by_object.contains_key(object) {
            continue; // Survivor or type change; already decided above.
        }
        // Deleted: absent from `post`. Its pre most-specific type must
        // conform to a declared `deletes` grant.
        let mut allowed = false;
        for grant in &declared.effect.deletes {
            if type_conforms(&pre.generals, pre_type, grant)? {
                allowed = true;
                break;
            }
        }
        if !allowed {
            return Err(frame_violation(
                ModelRefusalCause::FrameDeleteOutsideGrant {
                    object: (*object).to_owned(),
                    type_name: (*pre_type).clone(),
                },
                format!(
                    "invocation deletes object {object} of type {}, outside the operation's \
                     declared deletes frame",
                    pre_type.node
                ),
            ));
        }
        computed_deleted.insert((*object).to_owned());
    }

    // Field writes: members present in both documents under the same
    // identity and the same most-specific type (survivors; a type change
    // already refused above).
    let post_members_by_object: HashMap<&str, &PopulationMember> = post_document
        .members
        .iter()
        .map(|member| (member.object.as_str(), member))
        .collect();
    for pre_member in &pre_document.members {
        let Some(&post_member) = post_members_by_object.get(pre_member.object.as_str()) else {
            continue; // Deleted; already decided above.
        };
        let mut fields: BTreeSet<&DeclarationKey> = BTreeSet::new();
        fields.extend(pre_member.field_values.iter().map(|entry| &entry.field));
        fields.extend(post_member.field_values.iter().map(|entry| &entry.field));
        for field in fields {
            let pre_values = values_of(pre_member, field);
            let post_values = values_of(post_member, field);
            if field_values_equal(domain_package, field, pre_values, post_values) {
                continue;
            }
            if field_write_covered(domain_package, declared.effect, field) {
                continue;
            }
            return Err(frame_violation(
                ModelRefusalCause::FrameFieldWriteOutsideGrant {
                    object: pre_member.object.clone(),
                    field: field.clone(),
                },
                format!(
                    "invocation changes object {}'s field {}, outside the operation's declared \
                     modifies frame",
                    pre_member.object, field.node
                ),
            ));
        }
    }

    check_declared_delta(&computed_created, &computed_deleted, declared)
}

/// FR-046's "caller-supplied lists must match": decides `declared`'s own
/// created/deleted identity lists internally consistent (no duplicate within
/// either list, no identity in both), then compares them against the
/// complete computed sets [`enforce_frame`] derived from `pre`/`post`. Any
/// disagreement is `Code::PopulationDeltaMismatch`/cause
/// `delta-disagreement` -- mirroring the native runtime's own
/// `declared_deltas`/`inspect_frames` (see `admit_invocation`'s own
/// "Native duplication" doc).
fn check_declared_delta(
    computed_created: &BTreeSet<String>,
    computed_deleted: &BTreeSet<String>,
    declared: &InvocationDelta<'_>,
) -> Result<(), ModelRefusal> {
    let mut declared_created = BTreeSet::new();
    let mut declared_deleted = BTreeSet::new();
    for (declared_list, identities) in [
        (declared.declared_created, &mut declared_created),
        (declared.declared_deleted, &mut declared_deleted),
    ] {
        for identity in declared_list {
            if !identities.insert(identity.clone()) {
                return Err(delta_mismatch(
                    ModelRefusalCause::DuplicateDeclaredIdentity {
                        identity: identity.clone(),
                    },
                    format!("invocation declares {identity} more than once in the same delta list"),
                ));
            }
        }
    }
    if let Some(overlap) = declared_created.intersection(&declared_deleted).next() {
        return Err(delta_mismatch(
            ModelRefusalCause::DeclaredCreateDeleteOverlap {
                identity: overlap.clone(),
            },
            format!("invocation declares {overlap} as both created and deleted"),
        ));
    }
    if declared_created != *computed_created || declared_deleted != *computed_deleted {
        return Err(delta_mismatch(
            ModelRefusalCause::DeclaredDeltaMismatch {
                declared_created: declared_created.clone(),
                declared_deleted: declared_deleted.clone(),
                computed_created: computed_created.clone(),
                computed_deleted: computed_deleted.clone(),
            },
            format!(
                "declared population delta (created {declared_created:?}, deleted \
                 {declared_deleted:?}) differs from the complete pre/post populations (created \
                 {computed_created:?}, deleted {computed_deleted:?})"
            ),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// allInstances
// ---------------------------------------------------------------------------

/// `allInstances<T>(p)`'s own typed, bounded Outputs form:
/// `Set<Reference<T>>[0,N]`, never a bare unbounded collection. `bound` is a
/// real [`CardinalityBound`] (`[0, N]`, `N` the binding's own declared
/// maximum), reusing `crate::value`'s FR-144 bound type rather than a plain
/// comparison; see the module docs for why members stay [`ReferenceKey`]s
/// rather than `crate::value::Value::Reference`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceSet {
    element_type: DeclarationKey,
    bound: CardinalityBound,
    members: BTreeSet<ReferenceKey>,
}

impl ReferenceSet {
    /// The queried type `T`.
    pub fn element_type(&self) -> &DeclarationKey {
        &self.element_type
    }

    /// The declared bound `[0, N]`.
    pub fn bound(&self) -> CardinalityBound {
        self.bound
    }

    /// Every selected member, ascending by [`ReferenceKey`].
    pub fn members(&self) -> &BTreeSet<ReferenceKey> {
        &self.members
    }

    /// The selected count.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Whether no member was selected.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}

/// The outcome of one [`all_instances`] evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AllInstancesOutcome {
    /// Every and only member whose most-specific type conforms to `T`, once
    /// by reference key, in canonical reference-key order.
    Completed(ReferenceSet),
    /// A real defect refused the query outright; no collection.
    Refused(ModelRefusal),
    /// A `ScalarLimitsV1` counter was exhausted; no collection.
    Incomplete(ScalarIncomplete),
}

fn is_object_type(domain_package: &DomainPackage, key: &DeclarationKey) -> bool {
    domain_package.records.iter().any(
        |record| matches!(record, DomainPackageRecord::ObjectType(object) if &object.key == key),
    )
}

/// `allInstances<T>(p)`: every member of `binding` whose most-specific type
/// conforms to `t`, once by reference key, in canonical reference-key order.
/// `binding.members()` already iterates in that order. Evaluated entirely
/// against `binding`'s own admitted domain package — there is no separate `domain_package`
/// argument to mismatch (see the module docs' "Binding/domain package correspondence").
pub fn all_instances(
    binding: &PopulationBinding,
    t: &DeclarationKey,
    meter: &mut ScalarMeter,
) -> AllInstancesOutcome {
    let domain_package = &*binding.domain_package;
    let Some(declared_maximum) = binding.declared_maximum() else {
        return AllInstancesOutcome::Refused(ModelRefusal {
            code: Code::IllTyped,
            cause: ModelRefusalCause::OperatorIneligible,
            detail: "population binding has no declared maximum".to_owned(),
        });
    };
    if !is_object_type(domain_package, t) {
        return AllInstancesOutcome::Refused(ModelRefusal {
            code: Code::IllTyped,
            cause: ModelRefusalCause::TypeMismatch,
            detail: format!("{} is not a model object type", t.node),
        });
    }

    let mut selected: BTreeSet<ReferenceKey> = BTreeSet::new();
    for (key, original_type) in binding.members() {
        if let Err(incomplete) = meter.charge(ScalarCharge::new(ScalarChargePoint::PopulationVisit))
        {
            return AllInstancesOutcome::Incomplete(incomplete);
        }
        match type_conforms(&binding.generals, original_type, t) {
            Ok(true) => {
                selected.insert(key.clone());
            }
            Ok(false) => {}
            Err(refusal) => return AllInstancesOutcome::Refused(refusal),
        }
    }

    let n = length_amount(selected.len());
    if let Err(incomplete) = meter.charge(
        ScalarCharge::new(ScalarChargePoint::CollectionBound)
            .size(ScalarLimitKind::ValueOccurrences, n),
    ) {
        return AllInstancesOutcome::Incomplete(incomplete);
    }
    if n > declared_maximum {
        return AllInstancesOutcome::Refused(ModelRefusal {
            code: Code::CardinalityOutOfBound,
            cause: ModelRefusalCause::AboveMaximum {
                selected: selected.len(),
                maximum: usize::try_from(declared_maximum).unwrap_or(usize::MAX),
            },
            detail: format!(
                "selected count {n} is above the declared maximum [0,{declared_maximum}]"
            ),
        });
    }

    // Each selected member is a scalar reference (occ() = 1); the FR-144
    // collection-result-retain fold (`src/value/collection.rs::bound_and_retain`)
    // starts from `Integer::one()`, so a set of `n` references retains
    // `n + 1` value/result units — verified against TC-198 L01's exact
    // `value_occurrences 4` / `result_units += 4` for a 3-element selection.
    let occ = Integer::from(n).add(&Integer::one());
    if let Err(incomplete) = meter.charge(
        ScalarCharge::new(ScalarChargePoint::CollectionResultRetain)
            .exact_size(ScalarLimitKind::ValueOccurrences, occ.clone())
            .exact_results(occ),
    ) {
        return AllInstancesOutcome::Incomplete(incomplete);
    }

    let bound =
        CardinalityBound::new(0, declared_maximum).expect("0 is always <= a declared u64 maximum");
    AllInstancesOutcome::Completed(ReferenceSet {
        element_type: t.clone(),
        bound,
        members: selected,
    })
}

// ---------------------------------------------------------------------------
// lookup
// ---------------------------------------------------------------------------

/// `lookup<T>(p, r)`'s own typed Outputs form: the realized [`ReferenceKey`]
/// paired with the queried type `T`, never a bare [`ReferenceKey`] alone
/// (which carries only the referenced object's own most-specific type, not
/// the query's declared static type `T`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedReference {
    queried_type: DeclarationKey,
    key: ReferenceKey,
}

impl TypedReference {
    /// A reference to `key`, typed as `queried_type` (`T`).
    pub fn new(queried_type: DeclarationKey, key: ReferenceKey) -> Self {
        Self { queried_type, key }
    }

    /// The queried type `T`.
    pub fn queried_type(&self) -> &DeclarationKey {
        &self.queried_type
    }

    /// The realized reference key.
    pub fn key(&self) -> &ReferenceKey {
        &self.key
    }
}

/// The outcome of one [`lookup`] evaluation, mirroring AD-005's four-way
/// shape (`crate::value::outcome::Outcome`) with this operation's own payload
/// and refusal types. `Completed(Some(_))` is `r`'s realized reference,
/// typed by the queried `T`, when present, for every mode alike;
/// `Completed(None)` only ever occurs under [`AbsenceMode::Empty`] (the other
/// two modes route absence to
/// [`LookupOutcome::Undefined`]/[`LookupOutcome::Refused`] instead, per
/// FR-153's own table, never to a `None` payload).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LookupOutcome {
    /// A completed value: present, or (`empty` mode only) absent-as-`none`.
    Completed(Option<TypedReference>),
    /// `absent undefined`'s catalogued `absent-key` reason: the operation has
    /// no mathematical value.
    Undefined,
    /// A real defect refused the query outright; no value.
    Refused(ModelRefusal),
    /// A `ScalarLimitsV1` counter was exhausted; no value.
    Incomplete(ScalarIncomplete),
}

fn charge_result_retain(meter: &mut ScalarMeter, occ: u64) -> Result<(), ScalarIncomplete> {
    meter.charge(
        ScalarCharge::new(ScalarChargePoint::LookupResultRetain)
            .size(ScalarLimitKind::ValueOccurrences, occ)
            .results(occ),
    )
}

/// One [`ModelRefusal`]/`Code::ForeignReference` for a reference key naming a
/// universe other than `binding`'s own. `actual` is the raw universe bytes
/// exactly as supplied -- not always a well-formed 32-byte identity (see
/// [`ModelRefusalCause::ForeignUniverse`]'s own doc comment).
fn foreign_universe(binding: &PopulationBinding, actual: &[u8]) -> ModelRefusal {
    ModelRefusal {
        code: Code::ForeignReference,
        cause: ModelRefusalCause::ForeignUniverse {
            actual: actual.to_vec(),
            expected: *binding.universe(),
        },
        detail: format!(
            "reference key names universe {}, not the binding's {}",
            hex(actual),
            binding.universe()
        ),
    }
}

/// `lookup<T>(p, r) absent m`: `r`'s presence in `binding`, per `mode`, in
/// one order for a well-formed and a malformed reference alike --
/// `type_conforms(S, T)`, then the `lookup.key` charge, then the universe
/// check, then membership or absence (see [`LookupKey`]'s own doc comment
/// for how `r`'s own components carry a malformed universe or object
/// identity). Charges `lookup.key` once per call; a present result then
/// charges `lookup.result-retain` with `value_occurrences`/`result_units` = 1
/// for a bare `Reference<T>` (`undefined`/`refused` modes) or = 2 for an
/// `Option<Reference<T>>` (`empty` mode, the wrapper plus the reference); an
/// absent `empty`-mode result still retains 1 (the `none` wrapper alone),
/// while an absent `undefined`/`refused`-mode result retains nothing at all
/// — verified against TC-198 L03's three absence-mode vectors. Evaluated
/// entirely against `binding`'s own admitted domain package — there is no separate
/// `domain_package` argument to mismatch (see the module docs' "Binding/domain package
/// correspondence").
pub fn lookup(
    binding: &PopulationBinding,
    t: &DeclarationKey,
    r: &LookupKey,
    mode: AbsenceMode,
    meter: &mut ScalarMeter,
) -> LookupOutcome {
    match type_conforms(&binding.generals, &r.static_type, t) {
        Ok(true) => {}
        Ok(false) => {
            return LookupOutcome::Refused(ModelRefusal {
                code: Code::IllTyped,
                cause: ModelRefusalCause::TypeMismatch,
                detail: format!("{} does not conform to {}", r.static_type.node, t.node),
            })
        }
        Err(refusal) => return LookupOutcome::Refused(refusal),
    }

    if let Err(incomplete) = meter.charge(
        ScalarCharge::new(ScalarChargePoint::LookupKey).size(ScalarLimitKind::ValueOccurrences, 1),
    ) {
        return LookupOutcome::Incomplete(incomplete);
    }

    if r.universe.as_slice() != binding.universe().as_bytes().as_slice() {
        return LookupOutcome::Refused(foreign_universe(binding, &r.universe));
    }

    // `lookup` alone decides whether `r.object` is a candidate member
    // identity (valid UTF-8, see `LookupKey`'s own doc comment) or can never
    // name one at all -- never a caller-asserted classification.
    let present = match std::str::from_utf8(&r.object) {
        Ok(object) => {
            let key = ReferenceKey {
                universe: *binding.universe(),
                type_identity: r.type_identity,
                object: object.to_owned(),
            };
            binding.members().contains_key(&key).then_some(key)
        }
        Err(_) => None,
    };

    if let Some(key) = present {
        let occ = if matches!(mode, AbsenceMode::Empty) {
            2
        } else {
            1
        };
        if let Err(incomplete) = charge_result_retain(meter, occ) {
            return LookupOutcome::Incomplete(incomplete);
        }
        return LookupOutcome::Completed(Some(TypedReference::new(t.clone(), key)));
    }

    match mode {
        AbsenceMode::Undefined => LookupOutcome::Undefined,
        AbsenceMode::Refused => {
            let display = match std::str::from_utf8(&r.object) {
                Ok(object) => object.to_owned(),
                // Distinguishable from a well-formed identity string, which
                // this format can never itself produce: no JSON string
                // (`PopulationDocument`'s own member identity shape)
                // contains "(not UTF-8)" as a literal suffix of a bare hex
                // run, since a JSON string is already valid UTF-8 by
                // construction.
                Err(_) => format!("identity bytes 0x{} (not UTF-8)", hex(&r.object)),
            };
            LookupOutcome::Refused(ModelRefusal {
                code: Code::InvalidRuntimeInput,
                cause: ModelRefusalCause::AbsentKey {
                    key: r.object.clone(),
                },
                detail: format!("{display} is not a member of the bound population"),
            })
        }
        AbsenceMode::Empty => {
            if let Err(incomplete) = charge_result_retain(meter, 1) {
                return LookupOutcome::Incomplete(incomplete);
            }
            LookupOutcome::Completed(None)
        }
    }
}
