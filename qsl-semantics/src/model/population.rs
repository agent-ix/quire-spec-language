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
//! [`CardinalityBound`] `[0, N]`, reusing `quire_exact`'s own FR-144 kernel
//! bound type rather than a plain comparison) and [`lookup`] returns a
//! [`TypedReference`] (the queried type `T` alongside the realized key).
//! Neither wraps its members as `crate::value::Value::Reference`
//! (`ObjectReference`): that type's identity components
//! (kernel [`quire_exact::UniverseId`]/[`quire_exact::ObjectId`],
//! `quire_semantic_value::object_closure`) belong to FR-143's own closed object
//! closure, so this rung's typed wrappers stay in `crate::model`'s own
//! identity domain rather than constructing that type themselves.
//! `crate::value::model_query` is the documented canonical encoding this
//! module's own module docs once called for: FR-143 defines a reference's
//! `universe` and most-specific `type` as literally this crate's own
//! `quire.model.object-universe/v2` and
//! `quire.model.effective-declaration/v2` digests, so that bridge carries
//! the type component as the same `EffectiveId` (ADR-013 O-05) and the
//! universe as its own bytes in the kernel `UniverseId`, never a re-hash.
//!
//! # Binding/domain package correspondence
//!
//! A [`PopulationBinding`]'s fields are private; [`admit_binding`] is its
//! only constructor, and it stores the admitted `DomainPackage` itself -- the
//! effective view's own package, shared through the same `Arc` rather than
//! copied -- alongside the universe of the binding's own declared
//! type's connected component (ADR-013 §8 OQ-E) -- never a single universe
//! for the whole domain package.
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
//! population (`qsl-eval/tests/it/model_reference_queries.rs`'s `l07_pre_*` tests). It
//! then runs two distinct checks over the two admitted bindings, both by
//! `enforce_frame`:
//!
//! - the FR-151 operation frame (`quire.model.conformance.effect/v1`):
//!   every created and deleted object identity's most-specific type must
//!   conform to a declared `creates`/`deletes` grant, and every changed
//!   field on a surviving object must be a declared `modifies` member
//!   (directly, or reaching one through a member's own `redefines` property
//!   — FR-151's own effect-inclusion rule, each changed field charged as
//!   `frame.write` and each link followed as `frame.redefines-link`). A field's declared collection kind
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
//! (`qsl-eval`'s `value::expression::evaluate`'s `Anchor`/`Task::RestoreAnchor`) is
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
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline for typed model-admission failures"
)]

use quire_canonical::FixedShape;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use crate::model::dispatch::GeneralizationClosure;
use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, FieldMemberRecord,
    OperationEffect,
};
use crate::model::index::ModelIndex;
use crate::model::key::EffectiveId;
use crate::model::normalize::{
    EffectiveView, ModelRefusal, ModelRefusalCause, OfferedSelection, TypeCatalog, ViewPopulation,
};
use qsl_foundation::absence::AbsenceMode;
use qsl_foundation::diagnostic::Code;
use quire_checked::key::DeclarationKey;
use serde::Serialize;

use quire_exact::{
    length_amount, CardinalityBound, Charge as ScalarCharge, ChargePoint as ScalarChargePoint,
    Incomplete as ScalarIncomplete, Integer, LimitKind as ScalarLimitKind, Meter as ScalarMeter,
    UniverseId,
};

/// The kernel's structural `PopulationId`: the population declaration's node
/// identity ([`crate::model::key::population_node_key`]). Re-exported at this
/// path so callers reach it as `crate::model::population::PopulationId`,
/// mirroring [`crate::model::key::EffectiveId`]'s re-export of its own
/// kernel identity type.
pub use quire_exact::PopulationId;

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
    /// Ceiling on the `supertypes` edges one member-type conformance walk
    /// follows, with the same meaning as
    /// [`crate::model::accounting::ModelNormalizationLimits::ancestor_steps`].
    /// Charged by every walk that follows `supertypes` edges: the coverage
    /// walk of admission, subsetting applicability, the frame check, and the
    /// member walk of [`all_instances`] and the key walk of [`lookup`]. It is
    /// recorded on the admitted [`PopulationBinding`], so the later walks run
    /// under the same caller-configured bound. Reaching it refuses
    /// `ModelRefusalCause::AncestorSteps` naming this bound.
    pub ancestor_steps: u64,
}

/// NFR-012's finite default ceilings, which a caller may raise or
/// lower field by field. `spec/non-functional/NFR-012-*.md` states each
/// value and its derivation.
impl Default for PopulationAdmissionLimits {
    fn default() -> Self {
        Self {
            population_members: 100_000,
            work_units: 16_777_216,
            ancestor_steps: quire_semantic_value::declaration::DEFAULT_ANCESTOR_STEPS,
        }
    }
}

impl PopulationAdmissionLimits {
    /// These limits with `population_members` set (`admission.population_members`).
    #[must_use]
    pub const fn with_population_members(mut self, bound: u64) -> Self {
        self.population_members = bound;
        self
    }

    /// These limits with `work_units` set (`admission.work_units`).
    #[must_use]
    pub const fn with_work_units(mut self, bound: u64) -> Self {
        self.work_units = bound;
        self
    }

    /// These limits with `ancestor_steps` set (`admission.ancestor_steps`).
    #[must_use]
    pub const fn with_ancestor_steps(mut self, bound: u64) -> Self {
        self.ancestor_steps = bound;
        self
    }
}

/// FR-255: the one mapping from each field to its setting.
impl qsl_foundation::SettingLimits for PopulationAdmissionLimits {
    fn bounds(&self) -> Vec<(qsl_foundation::Setting, u64)> {
        use qsl_foundation::Setting;
        vec![
            (Setting::AdmissionPopulationMembers, self.population_members),
            (Setting::AdmissionWorkUnits, self.work_units),
            (Setting::AdmissionAncestorSteps, self.ancestor_steps),
        ]
    }

    fn set_bound(&mut self, setting: qsl_foundation::Setting, bound: u64) -> bool {
        use qsl_foundation::Setting;
        match setting {
            Setting::AdmissionPopulationMembers => self.population_members = bound,
            Setting::AdmissionWorkUnits => self.work_units = bound,
            Setting::AdmissionAncestorSteps => self.ancestor_steps = bound,
            _ => return false,
        }
        true
    }
}

impl PopulationAdmissionLimits {
    /// A limit set large enough that no charge in this module is denied.
    /// Test-only: production callers start from [`Self::default`].
    #[cfg(any(test, feature = "test-support"))]
    pub const UNLIMITED: Self = Self {
        population_members: u64::MAX,
        work_units: u64::MAX,
        ancestor_steps: u64::MAX,
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

    /// The setting that raises this counter (FR-255).
    pub const fn setting(self) -> qsl_foundation::Setting {
        match self {
            Self::PopulationMembers => qsl_foundation::Setting::AdmissionPopulationMembers,
            Self::WorkUnits => qsl_foundation::Setting::AdmissionWorkUnits,
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
    /// `frame.write` (FR-013): one per changed field, before the write is
    /// tested against the frame's `modifies` entries.
    FrameWrite,
    /// `frame.redefines-link` (FR-013): one before each `redefines` link
    /// followed from a written member.
    FrameRedefinesLink,
}

impl AdmissionChargePoint {
    /// Every named point this schedule defines.
    pub const ALL: [Self; 4] = [
        Self::BindingMember,
        Self::BindingSubsetValue,
        Self::FrameWrite,
        Self::FrameRedefinesLink,
    ];

    /// Normative identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BindingMember => "binding.member",
            Self::BindingSubsetValue => "binding.subset-value",
            Self::FrameWrite => "frame.write",
            Self::FrameRedefinesLink => "frame.redefines-link",
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

impl AdmissionIncomplete {
    /// This stop as the caller-budget denial FR-255 Behavior 1 describes
    /// (`resource_exhausted`/`insufficient-next-charge`).
    pub fn denial(&self) -> qsl_foundation::diagnostic::BudgetDenial {
        qsl_foundation::diagnostic::BudgetDenial::charge(
            self.limit_kind.setting(),
            self.limit,
            qsl_foundation::diagnostic::DeniedCharge {
                owner: ADMISSION_OWNER,
                limit_kind: self.limit_kind.as_str(),
                charge_point: self.charge_point.as_str(),
                consumed: self.consumed,
                next_charge: self.next_charge,
            },
        )
    }
}

/// The owner of a `PopulationAdmissionLimitsV1` charge.
pub const ADMISSION_OWNER: qsl_foundation::diagnostic::BudgetOwner =
    qsl_foundation::diagnostic::BudgetOwner {
        stage: "population-admission",
        accounting_contract: "PopulationAdmissionLimitsV1",
    };

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
///
/// **A count, not a log.** A production meter holds only
/// fixed-size state: the limits, the two counters and the number of admitted
/// charges, and owns no heap memory; the const assertion below this type
/// holds that in every production build. The ordered charge log
/// ([`AdmissionMeter::admitted_charges`]) exists only under the
/// `test-support` feature, which only a dev-dependency may enable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionMeter {
    limits: PopulationAdmissionLimits,
    consumed: [u64; 2],
    admissions: u64,
    #[cfg(feature = "test-support")]
    admitted: Vec<AdmissionChargePoint>,
}

// A production `AdmissionMeter` owns no heap memory (see
// `crate::model::accounting::Meter`'s identical assertion).
#[cfg(not(feature = "test-support"))]
const _: () = assert!(!std::mem::needs_drop::<AdmissionMeter>());

impl AdmissionMeter {
    /// A fresh meter with nothing consumed.
    pub fn new(limits: PopulationAdmissionLimits) -> Self {
        Self {
            limits,
            consumed: [0; 2],
            admissions: 0,
            #[cfg(feature = "test-support")]
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

    /// How many charges this meter has admitted.
    pub fn admission_count(&self) -> u64 {
        self.admissions
    }

    /// Every admitted charge point in admission order. Test-only: a
    /// production meter keeps [`Self::admission_count`], not a log.
    #[cfg(feature = "test-support")]
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
        self.admissions = self.admissions.saturating_add(1);
        #[cfg(feature = "test-support")]
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
    /// (`quire.model.object-universe/v2` identity, ADR-013 §8 OQ-C ruling:
    /// a `UniverseId`, not an `EffectiveId`).
    pub universe: UniverseId,
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
/// resolves through the effective view by key.
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
/// type -- decides which: `universe` is compared against `binding`'s own
/// structurally; `object` is raw identity bytes, and [`lookup`]
/// only treats them as a candidate member identity when they are valid
/// UTF-8 -- every real population member's own object identity is a JSON
/// string (`PopulationDocument`'s member records), so bytes that are not
/// valid UTF-8 can never equal one. This type carries no classification of
/// its own (no public "this identity is malformed"
/// constructor) precisely so a caller cannot assert a false classification
/// for well-formed bytes and force a present member to read as absent, or
/// the reverse. `type_identity` is always well-formed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LookupKey {
    /// `r`'s own static type `S`.
    pub static_type: DeclarationKey,
    /// `r`'s realized reference's universe, exactly as supplied.
    pub universe: UniverseId,
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
/// against a domain package's `modelIdentity` or object closure. The declared
/// member types are retained so [`all_instances`] can establish subtype
/// closure for each queried type — and, because the admitted domain package
/// itself lives here, cannot later evaluate
/// [`all_instances`]/[`lookup`] against any domain package other than the one that
/// was checked (see the module docs' "Binding/domain package correspondence").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationBinding {
    /// The population declaration this binding was admitted from. Retained
    /// so callers can report the offered declaration when its model selection
    /// does not correspond to the selected model.
    population_key: DeclarationKey,
    /// This binding's own object universe, computed once at admission
    /// (ADR-013 §8 OQ-C ruling: a `UniverseId`, not an `EffectiveId`).
    universe: UniverseId,
    /// Every admitted member, ascending by [`ReferenceKey`].
    members: BTreeMap<ReferenceKey, DeclarationKey>,
    /// The population declaration's member types, retained to establish
    /// whether each effective subtype queried by `allInstances<T>` is
    /// represented in the closed population.
    declared_member_types: BTreeSet<DeclarationKey>,
    /// The binding's declared maximum, or `None` for a binding with no
    /// declared maximum (`allInstances` is then `operator-ineligible`).
    declared_maximum: Option<u64>,
    /// The domain package this binding was admitted against, together with
    /// the shared [`ModelIndex`] the admitting view's normalization built
    /// over it. `Arc`, so cloning a binding never re-clones the
    /// package's own records. Every conformance decision
    /// ([`all_instances`], [`lookup`] and the invocation frame check) reads
    /// its ancestry, which is computed once per type and kept, never walked
    /// again per call. `value-accounting.md`'s "Model and graph evaluation"
    /// paragraph places that decision outside any charge ("...selects the
    /// member, without a charge, exactly when that type conforms to `T`").
    index: Arc<ModelIndex>,
    /// The caller's [`PopulationAdmissionLimits::ancestor_steps`] this
    /// binding was admitted under: every conformance decision over
    /// [`Self::index`] uses it as given.
    ancestor_steps: u64,
    /// Every declared object type of `domain_package`'s effective view,
    /// `DeclarationKey` to its FR-150-derived [`EffectiveId`] and back,
    /// shared with the admitting view. The FR-143 reference-identity bridge
    /// (`crate::value::model_query`) reads the reverse direction to
    /// translate a checked `Reference<T>`'s `T` (its `EffectiveId`, ADR-013
    /// O-05) back into the `DeclarationKey` [`all_instances`]/[`lookup`]
    /// take, for every declared type, not only ones a current member happens
    /// to name, with no per-query rebuild.
    type_catalog: Arc<TypeCatalog>,
    /// FR-153's invocation pre population, attached only by
    /// [`admit_invocation`]: `pre(allInstances(p))`/`pre(lookup(p, r) absent
    /// m)` read this binding instead of `self` underneath a `pre(..)`
    /// anchor. `None` for a binding admitted directly by [`admit_binding`]
    /// (no invocation, so no pre state to anchor to). One level of `Arc`
    /// indirection, never chained: the pre binding [`admit_invocation`]
    /// attaches here is itself always admitted with `pre_anchor: None`.
    pre_anchor: Option<Arc<PopulationBinding>>,
    /// This binding's `PopulationId` (FR-089-AC-1): the structural node
    /// identity of the population declaration `population_key` names. The evaluator
    /// carries this identity alone as `Value::Population`, resolving it
    /// back to this binding through the caller's own recorded
    /// correspondence (`crate::model::object_environment::ObjectEnvironment::resolve_population`).
    population_id: PopulationId,
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
        &self.index.package().model_selection
    }

    /// The `DomainPackageRef` identity this binding was admitted against.
    pub fn model_identity(&self) -> &str {
        &self.index.package().model_selection.identity
    }

    /// The population declaration this binding was admitted from.
    pub fn population_key(&self) -> &DeclarationKey {
        &self.population_key
    }

    /// This binding's own object universe.
    pub fn universe(&self) -> &UniverseId {
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

    /// This binding's minted `PopulationId` (FR-089-AC-1); see the field's
    /// own doc comment.
    pub fn population_id(&self) -> PopulationId {
        self.population_id.clone()
    }

    /// Every declared object type of the admitted effective view,
    /// `DeclarationKey` to its FR-150-derived [`EffectiveId`]; see the field's
    /// own doc comment.
    pub fn type_catalog(&self) -> &BTreeMap<DeclarationKey, EffectiveId> {
        self.type_catalog.by_key()
    }

    /// The declared type whose FR-150-derived [`EffectiveId`] is
    /// `effective_type`: [`Self::type_catalog`] read the other way, from the
    /// reverse index built once with it.
    pub fn type_key_of(&self, effective_type: &EffectiveId) -> Option<&DeclarationKey> {
        self.type_catalog.key_of(effective_type)
    }
}

/// The outcome of one [`admit_binding`] attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmissionOutcome {
    /// Every member was charged and admitted.
    Admitted(PopulationBinding),
    /// A real model defect refused
    /// admission outright; no binding.
    Refused(ModelRefusal),
    /// Object or subtype closure is not established. Not a refusal
    /// (`Code::IncompletePopulation`); no charge and no binding.
    UnknownClosure(ModelRefusal),
    /// A `PopulationAdmissionLimitsV1` counter was exhausted; no binding.
    Incomplete(AdmissionIncomplete),
}

/// Admits `document` against `view`/`population_key` into a
/// [`PopulationBinding`], per FR-153's "Environment key and closure". Decides,
/// without a charge and in order, the `modelIdentity` check,
/// `population_key`'s own resolution against the view's domain package,
/// object closure (the resolved
/// [`crate::model::domain_package::PopulationRecord::extent`]) and subtype
/// closure; then charges
/// `binding.member` for each member record in document order, deciding
/// foreign-type and then duplicate-collapse/conflicting-identity after each
/// charge. Stops at the first refusal or denied charge.
///
/// `view` is FR-150's own already-normalized, already-charged effective view,
/// computed by the caller under `ModelNormalizationLimitsV1` before this
/// call. It carries the exact domain package it was normalized from
/// ([`EffectiveView::domain_package`]), and admission reads that package,
/// its declarations and its object universes from the view alone:
/// there is no second, caller-supplied package that could disagree with the
/// view, and admission does no normalization work of its own. A package over
/// the caller's normalization limits never yields a view to admit against.
///
/// `population_key` is a key, not a caller-supplied
/// [`crate::model::domain_package::PopulationRecord`]:
/// FR-153's "Its declaration key must belong to the binding's ModelSelection"
/// means the population's declared extent is a fact of the domain package
/// itself, never a value the caller states independently of it. This
/// function resolves the record through the view and refuses
/// `foreign_reference`/`foreign-model-selection` when no `Population` record
/// there carries that key, so a caller cannot claim an extent, or select some
/// other package's population declaration, that the package did not itself
/// declare.
pub fn admit_binding(
    view: &EffectiveView,
    document: &PopulationDocument,
    population_key: &DeclarationKey,
    subtype_closure: GeneralizationClosure,
    declared_maximum: Option<u64>,
    meter: &mut AdmissionMeter,
) -> AdmissionOutcome {
    admit_binding_as(
        InvocationContext {
            view,
            population: population_key,
            subtype_closure,
            declared_maximum,
        },
        document,
        meter,
    )
}

/// [`admit_binding`]'s own admission logic, which [`admit_invocation`] calls
/// directly (bypassing the public [`admit_binding`]) for its two constituent
/// bindings. Takes
/// [`InvocationContext`] rather than its constituent fields: that type
/// already exists to bundle exactly this admission surface for
/// [`admit_invocation`]'s own two calls.
fn admit_binding_as(
    context: InvocationContext<'_>,
    document: &PopulationDocument,
    meter: &mut AdmissionMeter,
) -> AdmissionOutcome {
    let InvocationContext {
        view,
        population: population_key,
        subtype_closure,
        declared_maximum,
    } = context;
    let domain_package = view.domain_package();
    let ancestor_steps = meter.limits().ancestor_steps;
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
    // ADR-013 §8 OQ-E: the view resolves the population together with its
    // own universe -- its first declared member type's connected component,
    // never the whole domain package's, or the first universe when it
    // declares no member type (FR-153 never requires one).
    let Some(ViewPopulation {
        record: population,
        universe,
    }) = view.population(population_key)
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
    let universe = match universe.identity() {
        Ok(universe) => universe,
        Err(error) => return AdmissionOutcome::Refused(universe_identity_refusal(&error)),
    };

    // Indexed once, not re-scanned per member: `view.declarations()` and
    // `admitted` would otherwise each be linearly searched per member,
    // making admission O(n^2) against the O(n) `binding.member` charges it
    // records.
    let type_lookup = view.type_identities();

    // The view's shared index: conformance and the D05 abstract-instance
    // check below are lookups, never a rescan of `domain_package.records`.
    let index = view.model_index();

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
            match index.conforms(
                &member.type_identity,
                declared,
                ancestor_steps,
                qsl_foundation::Setting::AdmissionAncestorSteps,
            ) {
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
        if index.is_abstract(&member.type_identity) {
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
            universe: universe.clone(),
            type_identity: effective_type.clone(),
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
                    existing_type: existing.type_identity.clone(),
                    declared_type: key.type_identity.clone(),
                },
                detail: format!(
                    "object {} is declared with conflicting types {:?} and {:?}",
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
                match index.conforms(
                    original_type,
                    edge.owner,
                    ancestor_steps,
                    qsl_foundation::Setting::AdmissionAncestorSteps,
                ) {
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

    let population_id = match crate::model::key::population_node_key(population_key) {
        Ok(node) => PopulationId::new(node),
        Err(error) => return AdmissionOutcome::Refused(node_key_refusal(&error)),
    };
    AdmissionOutcome::Admitted(PopulationBinding {
        population_key: population_key.clone(),
        universe: universe.clone(),
        members: admitted,
        declared_member_types: population.member_types.iter().cloned().collect(),
        declared_maximum,
        index: Arc::clone(view.shared_model_index()),
        ancestor_steps,
        type_catalog: Arc::clone(view.shared_type_catalog()),
        pre_anchor: None,
        population_id,
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
/// than separate parameters so `admit_invocation` stays within this crate's
/// argument-count convention.
#[derive(Clone, Copy)]
pub struct InvocationContext<'a> {
    /// The already-normalized, already-charged effective view both instants
    /// are admitted against; it carries its own domain package.
    pub view: &'a EffectiveView,
    /// The population role's own declaration key, resolved against the
    /// view's domain package by each [`admit_binding`] call
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
/// against the same `view`/`population`/`subtype_closure`/
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
/// Deciding created/deleted-identity and delta agreement is, like the
/// object/subtype-closure and `modelIdentity` checks [`admit_binding`]
/// already makes, a structural decision over already-charged data. A field
/// write is not: FR-013 charges one `frame.write` for each changed field and
/// one `frame.redefines-link` before each `redefines` link followed from it,
/// to a frame meter of its own (see [`admit_invocation_bound`]).
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
    let mut frame_meter = AdmissionMeter::new(*post_meter.limits());
    admit_invocation_bound(
        context,
        pre_document,
        post_document,
        declared,
        &InvocationBinding::default(),
        pre_meter,
        post_meter,
        &mut frame_meter,
    )
}

/// Who the invocation's receiver and reference arguments are, for the
/// frame's receiver-scoped and argument-scoped entries (FR-013): `self`
/// and `deletes p` ground in these objects, and a frame that names neither
/// reads none of it.
#[derive(Clone, Copy, Debug, Default)]
pub struct InvocationBinding<'a> {
    /// The invocation's receiver, by object identity in the admitted
    /// population.
    pub receiver: Option<&'a str>,
    /// Each reference parameter's bound argument, as `(parameter name,
    /// object identity)`.
    pub arguments: &'a [(String, String)],
}

/// [`admit_invocation`] with the invocation's receiver and reference
/// arguments: a `modifies self.f` entry grants the write to `f` on
/// `binding.receiver` only, `deletes self` the deletion of the receiver
/// only, and `deletes p` the deletion of the argument bound to `p` only.
///
/// The frame check charges `frame_meter`, a meter of its own beside
/// `pre_meter` and `post_meter`, so its `frame.write` and
/// `frame.redefines-link` charges are counted on their own and never against
/// the members the two admissions charged. A denied charge ends the check
/// with [`AdmissionOutcome::Incomplete`] and no frame verdict.
#[allow(clippy::too_many_arguments)]
pub fn admit_invocation_bound(
    context: InvocationContext<'_>,
    pre_document: &PopulationDocument,
    post_document: &PopulationDocument,
    declared: &InvocationDelta<'_>,
    binding: &InvocationBinding<'_>,
    pre_meter: &mut AdmissionMeter,
    post_meter: &mut AdmissionMeter,
    frame_meter: &mut AdmissionMeter,
) -> AdmissionOutcome {
    let pre = match admit_binding_as(context, pre_document, pre_meter) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => return other,
    };
    let post = match admit_binding_as(context, post_document, post_meter) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => return other,
    };
    match enforce_frame(
        &pre,
        pre_document,
        post_document,
        declared,
        binding,
        frame_meter,
    ) {
        Ok(()) => AdmissionOutcome::Admitted(post.with_pre_anchor(pre)),
        Err(FrameStop::Refused(refusal)) => AdmissionOutcome::Refused(refusal),
        Err(FrameStop::Denied(incomplete)) => AdmissionOutcome::Incomplete(incomplete),
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
/// the binding's package -- the strict, pre-existing comparison -- since an absent record
/// gives this function no positive basis to relax it.
fn field_ordered(index: &ModelIndex, field: &DeclarationKey) -> bool {
    index
        .field(field)
        .is_none_or(|member| member.multiplicity.ordered)
}

/// Whether `pre_values`/`post_values` (`field`'s declared values on the same
/// member, pre and post) are the same collection, per `field`'s own declared
/// collection kind: exact sequence equality when ordered, multiset equality
/// (order-insensitive, duplicate-count-sensitive) otherwise. A `Set` field's
/// values are already unique by construction, so multiset equality reduces
/// to set equality for it without a separate case.
fn field_values_equal(
    index: &ModelIndex,
    field: &DeclarationKey,
    pre: &[String],
    post: &[String],
) -> bool {
    if field_ordered(index, field) {
        return pre == post;
    }
    let mut pre = pre.to_vec();
    let mut post = post.to_vec();
    pre.sort();
    post.sort();
    pre == post
}

/// One object as [`decide_frame`] reads it: its declared identity, its
/// most-specific type, and its declared field values keyed by the field's
/// own [`DeclarationKey`]. `V` is the caller's own field-value form (a
/// runtime population document's reference identities here, a wire value at
/// FR-106 admission); [`decide_frame`] only ever compares two `V`s through
/// the caller's `values_equal`.
pub struct FrameObject<'a, V> {
    /// The object's declared identity.
    pub object: &'a str,
    /// The object's most-specific type.
    pub type_identity: DeclarationKey,
    /// The object's declared field values.
    pub fields: BTreeMap<DeclarationKey, V>,
}

/// [`decide_frame`]'s inputs other than the two object lists: the model the
/// types and fields resolve in, the operation's frame, and the invocation's
/// declared delta for this population.
pub struct FrameDecision<'e> {
    /// The model index types and fields resolve in.
    pub index: &'e ModelIndex,
    /// The conformance walk's `ancestor_steps` ceiling.
    pub ancestor_steps: u64,
    /// The operation's authored frame.
    pub effect: &'e OperationEffect,
    /// The invocation's own declared created identities.
    pub declared_created: &'e [String],
    /// The invocation's own declared deleted identities.
    pub declared_deleted: &'e [String],
    /// The identity, in this population, of the invocation's receiver, when
    /// the receiver is in this population. A receiver-scoped entry grants
    /// nothing without one.
    pub receiver: Option<&'e str>,
    /// The identity, in this population, of the object each reference
    /// parameter is bound to, as `(parameter name, object)`.
    pub arguments: &'e [(String, String)],
}

/// One charge FR-013 makes while it decides a field write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameCharge {
    /// `frame.write`.
    Write,
    /// `frame.redefines-link`.
    RedefinesLink,
}

/// Why [`decide_frame`] stopped: a finding about the invocation, or a
/// denied charge (`E` is the caller's own incomplete record).
#[derive(Debug)]
pub enum FrameStop<E> {
    /// A refusal or violation.
    Refused(ModelRefusal),
    /// The first charge the caller's meter denied.
    Denied(E),
}

impl<E> From<ModelRefusal> for FrameStop<E> {
    fn from(refusal: ModelRefusal) -> Self {
        Self::Refused(refusal)
    }
}

/// [`decide_frame`]'s result: the computed created and deleted identities.
#[derive(Debug, Default)]
pub struct FrameDelta {
    /// Identities in post and not in pre.
    pub created: BTreeSet<String>,
    /// Identities in pre and not in post.
    pub deleted: BTreeSet<String>,
}

/// Whether `type_identity` conforms to some grant in `grants`.
fn granted(
    frame: &FrameDecision<'_>,
    grants: &[DeclarationKey],
    type_identity: &DeclarationKey,
) -> Result<bool, ModelRefusal> {
    for grant in grants {
        if frame.index.conforms(
            type_identity,
            grant,
            frame.ancestor_steps,
            qsl_foundation::Setting::AdmissionAncestorSteps,
        )? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether `frame` permits deleting `object`, of most-specific type
/// `type_identity`: its type conforms to a type-wide `deletes` entry, or it
/// is the receiver under `deletes self`, or it is the argument bound to `p`
/// under `deletes p`.
fn delete_granted(
    frame: &FrameDecision<'_>,
    object: &str,
    type_identity: &DeclarationKey,
) -> Result<bool, ModelRefusal> {
    let effect = frame.effect;
    if effect.deletes_receiver && frame.receiver == Some(object) {
        return Ok(true);
    }
    if frame
        .arguments
        .iter()
        .any(|(name, bound)| bound == object && effect.deletes_arguments.contains(name))
    {
        return Ok(true);
    }
    granted(frame, &effect.deletes, type_identity)
}

/// The one frame and delta decision (FR-046/FR-151, FR-106 check 11) over
/// one population's `pre` and `post` objects, both in document order. Both
/// `enforce_frame` and FR-106 admission (`model::observation::frame`)
/// call it, each building [`FrameObject`]s from its own input.
///
/// The first violation, in this order, is returned:
///
/// 1. `post`'s objects in ascending identity order: an object absent from
///    `pre` whose type conforms to no `creates` grant
///    ([`ModelRefusalCause::FrameCreateOutsideGrant`]), or a surviving
///    object whose most-specific type differs
///    ([`ModelRefusalCause::FrameTypeChanged`]).
/// 2. `pre`'s objects absent from `post`, ascending identity order, whose
///    type conforms to no `deletes` grant
///    ([`ModelRefusalCause::FrameDeleteOutsideGrant`]).
/// 3. surviving objects in ascending identity order, fields in ascending
///    [`DeclarationKey`] order: a changed field (`values_equal` false) that
///    reaches no `modifies` entry through its `redefines` chain
///    ([`ModelRefusalCause::FrameFieldWriteOutsideGrant`]). An every-object
///    entry grants the write on any object, a receiver-scoped entry only on
///    `frame.receiver`. Each changed field charges `charge(Write)` first,
///    then `charge(RedefinesLink)` before each `redefines` link followed
///    from it, stopping at the first member an entry names; the first
///    charge `charge` denies ends the decision as [`FrameStop::Denied`]
///    with no verdict.
/// 4. the declared delta against the computed one (`check_declared_delta`).
///
/// A deletion (step 2) is granted by a type-wide entry the object's type
/// conforms to, by `deletes self` for the receiver, or by `deletes p` for
/// the argument bound to `p`.
///
/// Created/deleted classification is by *object identity* alone, never by
/// type: an object present under the same identity in both `pre` and `post`
/// with a different most-specific type is a retype, and no frame authorizes
/// one -- FR-151's effect vocabulary grants `modifies`, `creates` and
/// `deletes` only, and FR-143 treats the most-specific type as part of a
/// reference's identity, never a mutable attribute.
pub fn decide_frame<V, E>(
    frame: &FrameDecision<'_>,
    pre: &[FrameObject<'_, V>],
    post: &[FrameObject<'_, V>],
    values_equal: impl Fn(&DeclarationKey, Option<&V>, Option<&V>) -> bool,
    charge: &mut impl FnMut(FrameCharge) -> Result<(), E>,
) -> Result<FrameDelta, FrameStop<E>> {
    let pre_by_object: BTreeMap<&str, &FrameObject<'_, V>> =
        pre.iter().map(|object| (object.object, object)).collect();
    let post_by_object: BTreeMap<&str, &FrameObject<'_, V>> =
        post.iter().map(|object| (object.object, object)).collect();
    let mut delta = FrameDelta::default();

    for (object, post_object) in &post_by_object {
        let post_type = &post_object.type_identity;
        let Some(pre_object) = pre_by_object.get(object) else {
            if !granted(frame, &frame.effect.creates, post_type)? {
                return Err(frame_violation(
                    ModelRefusalCause::FrameCreateOutsideGrant {
                        object: (*object).to_owned(),
                        type_name: post_type.clone(),
                    },
                    format!(
                        "invocation creates object {object} of type {}, outside the operation's \
                         declared creates frame",
                        post_type.node
                    ),
                )
                .into());
            }
            delta.created.insert((*object).to_owned());
            continue;
        };
        let pre_type = &pre_object.type_identity;
        if pre_type != post_type {
            return Err(frame_violation(
                ModelRefusalCause::FrameTypeChanged {
                    object: (*object).to_owned(),
                    pre_type: pre_type.clone(),
                    post_type: post_type.clone(),
                },
                format!(
                    "invocation changes object {object}'s most-specific type from {} to {} \
                     between pre and post, which no operation frame may authorize (FR-151's \
                     effect grants cover modifies/creates/deletes only)",
                    pre_type.node, post_type.node
                ),
            )
            .into());
        }
    }

    for (object, pre_object) in &pre_by_object {
        if post_by_object.contains_key(object) {
            continue; // Survivor or retype; already decided above.
        }
        let pre_type = &pre_object.type_identity;
        if !delete_granted(frame, object, pre_type)? {
            return Err(frame_violation(
                ModelRefusalCause::FrameDeleteOutsideGrant {
                    object: (*object).to_owned(),
                    type_name: pre_type.clone(),
                },
                format!(
                    "invocation deletes object {object} of type {}, outside the operation's \
                     declared deletes frame",
                    pre_type.node
                ),
            )
            .into());
        }
        delta.deleted.insert((*object).to_owned());
    }

    for pre_object in pre_by_object.values() {
        let Some(post_object) = post_by_object.get(pre_object.object) else {
            continue; // Deleted; already decided above.
        };
        let is_receiver = frame.receiver == Some(pre_object.object);
        let fields: BTreeSet<&DeclarationKey> = pre_object
            .fields
            .keys()
            .chain(post_object.fields.keys())
            .collect();
        for field in fields {
            if values_equal(
                field,
                pre_object.fields.get(field),
                post_object.fields.get(field),
            ) {
                continue;
            }
            charge(FrameCharge::Write).map_err(FrameStop::Denied)?;
            let covered = frame
                .index
                .redefinition_walk(
                    field,
                    |candidate| {
                        frame.effect.modifies.contains(candidate)
                            || (is_receiver && frame.effect.receiver_modifies.contains(candidate))
                    },
                    || charge(FrameCharge::RedefinesLink),
                )
                .map_err(FrameStop::Denied)?;
            if covered {
                continue;
            }
            return Err(frame_violation(
                ModelRefusalCause::FrameFieldWriteOutsideGrant {
                    object: pre_object.object.to_owned(),
                    field: field.clone(),
                },
                format!(
                    "invocation changes object {}'s field {}, outside the operation's declared \
                     modifies frame",
                    pre_object.object, field.node
                ),
            )
            .into());
        }
    }

    check_declared_delta(&delta, frame.declared_created, frame.declared_deleted)?;
    Ok(delta)
}

/// Enforces `declared.effect`'s frame and delta over `pre_document`/
/// `post_document` (the documents `pre`/`post` were admitted from, read for
/// their `field_values`, which [`PopulationBinding`] does not retain),
/// through [`decide_frame`].
fn enforce_frame(
    pre: &PopulationBinding,
    pre_document: &PopulationDocument,
    post_document: &PopulationDocument,
    declared: &InvocationDelta<'_>,
    binding: &InvocationBinding<'_>,
    meter: &mut AdmissionMeter,
) -> Result<(), FrameStop<AdmissionIncomplete>> {
    let index = &*pre.index;
    let frame = FrameDecision {
        index,
        ancestor_steps: pre.ancestor_steps,
        effect: declared.effect,
        declared_created: declared.declared_created,
        declared_deleted: declared.declared_deleted,
        receiver: binding.receiver,
        arguments: binding.arguments,
    };
    decide_frame(
        &frame,
        &frame_objects(pre_document),
        &frame_objects(post_document),
        |field, pre_values, post_values| {
            field_values_equal(
                index,
                field,
                pre_values.copied().unwrap_or(&[]),
                post_values.copied().unwrap_or(&[]),
            )
        },
        &mut |charge| {
            let point = match charge {
                FrameCharge::Write => AdmissionChargePoint::FrameWrite,
                FrameCharge::RedefinesLink => AdmissionChargePoint::FrameRedefinesLink,
            };
            meter.charge(AdmissionCharge::new(point))
        },
    )
    .map(|_| ())
}

/// `document`'s members as [`decide_frame`] reads them, in document order.
fn frame_objects(document: &PopulationDocument) -> Vec<FrameObject<'_, &[String]>> {
    document
        .members
        .iter()
        .map(|member| FrameObject {
            object: member.object.as_str(),
            type_identity: member.type_identity.clone(),
            fields: member
                .field_values
                .iter()
                .map(|entry| (entry.field.clone(), entry.values.as_slice()))
                .collect(),
        })
        .collect()
}

/// FR-046's "caller-supplied lists must match": decides the declared
/// created/deleted identity lists internally consistent (no duplicate within
/// either list, no identity in both), then compares them against the
/// complete computed sets [`decide_frame`] derived. Any disagreement is
/// `Code::PopulationDeltaMismatch`/cause `delta-disagreement` -- mirroring
/// the native runtime's own `declared_deltas`/`inspect_frames` (see
/// `admit_invocation`'s own "Native duplication" doc).
fn check_declared_delta(
    computed: &FrameDelta,
    declared_created_list: &[String],
    declared_deleted_list: &[String],
) -> Result<(), ModelRefusal> {
    let computed_created = &computed.created;
    let computed_deleted = &computed.deleted;
    let mut declared_created = BTreeSet::new();
    let mut declared_deleted = BTreeSet::new();
    for (declared_list, identities) in [
        (declared_created_list, &mut declared_created),
        (declared_deleted_list, &mut declared_deleted),
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
    bound: Option<CardinalityBound>,
    members: BTreeSet<ReferenceKey>,
}

impl ReferenceSet {
    /// The queried type `T`.
    pub fn element_type(&self) -> &DeclarationKey {
        &self.element_type
    }

    /// The declared bound `[0, N]`; `None` for a binding with no declared
    /// maximum, which is unbounded.
    pub fn bound(&self) -> Option<CardinalityBound> {
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
pub struct SubtypeClosureIncomplete {
    /// The type queried by `allInstances<T>`.
    pub queried_type: DeclarationKey,
    /// The queried type, which conforms to no declared population member
    /// type and therefore marks the missing subtype coverage.
    pub missing_types: Vec<DeclarationKey>,
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
    /// The binding is object-closed, but no declared member type covers the
    /// queried type by conformance; no collection exists.
    IncompleteSubtypeClosure(SubtypeClosureIncomplete),
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
    // A binding with no declared maximum is unbounded (QSpec FR-153-AC-9,
    // ADR-014 §2): its selection has no cardinality to exceed.
    let declared_maximum = binding.declared_maximum();
    if !binding.index.is_object_type(t) {
        return AllInstancesOutcome::Refused(ModelRefusal {
            code: Code::IllTyped,
            cause: ModelRefusalCause::TypeMismatch,
            detail: format!("{} is not a model object type", t.node),
        });
    }

    let mut covered = false;
    for declared_member_type in &binding.declared_member_types {
        match binding.index.conforms(
            t,
            declared_member_type,
            binding.ancestor_steps,
            qsl_foundation::Setting::AdmissionAncestorSteps,
        ) {
            Ok(true) => {
                covered = true;
                break;
            }
            Ok(false) => {}
            Err(refusal) => return AllInstancesOutcome::Refused(refusal),
        }
    }
    if !covered {
        return AllInstancesOutcome::IncompleteSubtypeClosure(SubtypeClosureIncomplete {
            queried_type: t.clone(),
            missing_types: vec![t.clone()],
        });
    }

    let mut selected: BTreeSet<ReferenceKey> = BTreeSet::new();
    for (key, original_type) in binding.members() {
        if let Err(incomplete) = meter.charge(ScalarCharge::new(ScalarChargePoint::PopulationVisit))
        {
            return AllInstancesOutcome::Incomplete(incomplete);
        }
        match binding.index.conforms(
            original_type,
            t,
            binding.ancestor_steps,
            qsl_foundation::Setting::AdmissionAncestorSteps,
        ) {
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
    if let Some(declared_maximum) = declared_maximum.filter(|maximum| n > *maximum) {
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
    // collection-result-retain fold (the kernel's `quire_exact::form`)
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

    let bound = declared_maximum.map(CardinalityBound::at_most);
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
/// shape ([`quire_exact::Outcome`]) with this operation's own payload
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
/// universe other than `binding`'s own. `actual` is the universe's root type
/// node as supplied.
fn foreign_universe(binding: &PopulationBinding, actual: &UniverseId) -> ModelRefusal {
    ModelRefusal {
        code: Code::ForeignReference,
        cause: ModelRefusalCause::ForeignUniverse {
            actual: actual.root_type().clone(),
            expected: binding.universe().clone(),
        },
        detail: format!(
            "reference key names universe {}, not the binding's {}",
            actual.root_type(),
            binding.universe().root_type()
        ),
    }
}

/// The refusal for a node identity that cannot be carried structurally.
fn node_key_refusal(error: &quire_exact::InvalidNodeKey) -> ModelRefusal {
    crate::model::key::node_key_refusal(error)
}

/// The refusal for a universe that has no structural identity.
fn universe_identity_refusal(error: &crate::model::normalize::UniverseIdentityError) -> ModelRefusal {
    crate::model::key::universe_identity_refusal(error)
}

/// `lookup<T>(p, r) absent m`: `r`'s presence in `binding`, per `mode`, in
/// one order for a well-formed and a malformed reference alike --
/// `ModelIndex::conforms(S, T)`, then the `lookup.key` charge, then the universe
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
    match binding.index.conforms(
        &r.static_type,
        t,
        binding.ancestor_steps,
        qsl_foundation::Setting::AdmissionAncestorSteps,
    ) {
        Ok(true) => {}
        Ok(false) => {
            return LookupOutcome::Refused(ModelRefusal::type_mismatch(
                r.static_type.node.clone(),
                t.node.clone(),
            ));
        }
        Err(refusal) => return LookupOutcome::Refused(refusal),
    }

    if let Err(incomplete) = meter.charge(
        ScalarCharge::new(ScalarChargePoint::LookupKey).size(ScalarLimitKind::ValueOccurrences, 1),
    ) {
        return LookupOutcome::Incomplete(incomplete);
    }

    if &r.universe != binding.universe() {
        return LookupOutcome::Refused(foreign_universe(binding, &r.universe));
    }

    // `lookup` alone decides whether `r.object` is a candidate member
    // identity (valid UTF-8, see `LookupKey`'s own doc comment) or can never
    // name one at all -- never a caller-asserted classification.
    let present = match std::str::from_utf8(&r.object) {
        Ok(object) => {
            let key = ReferenceKey {
                universe: binding.universe().clone(),
                type_identity: r.type_identity.clone(),
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
        AbsenceMode::Refused => LookupOutcome::Refused(ModelRefusal::absent_key(
            format!("{:?}", binding.population_id()),
            r.object.clone(),
        )),
        AbsenceMode::Empty => {
            if let Err(incomplete) = charge_result_retain(meter, 1) {
                return LookupOutcome::Incomplete(incomplete);
            }
            LookupOutcome::Completed(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;
    use crate::model::accounting::ModelNormalizationLimits;
    use crate::model::domain_package::{ObjectTypeRecord, PopulationRecord};
    use crate::model::normalize::{build_calls, normalize, NormalizeOutcome};

    /// A production `AdmissionMeter` and `AdmissionCharge`
    /// have no drop glue, so the meter holds no per-charge state however many
    /// charges it admits, and its admission count counts every one. Built only
    /// without `test-support`, as `accounting`'s identical test.
    #[cfg(not(feature = "test-support"))]
    #[trace("TC-434", "NFR-012")]
    #[test]
    fn a_production_admission_meter_owns_no_heap_memory_and_counts_every_charge() {
        assert!(!std::mem::needs_drop::<AdmissionMeter>());
        assert!(!std::mem::needs_drop::<AdmissionCharge>());
        let mut meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
        let mut admitted = 0_u64;
        for target in [1_u64, 1_000, 100_000] {
            while admitted < target {
                admitted += 1;
                meter
                    .charge(
                        AdmissionCharge::new(AdmissionChargePoint::BindingMember).size(admitted),
                    )
                    .expect("an unlimited meter admits every charge");
            }
            assert_eq!(meter.admission_count(), target);
            assert_eq!(meter.consumed(AdmissionLimitKind::WorkUnits), target);
        }
    }

    /// The default admission limits are NFR-012's finite ceilings.
    #[trace("TC-434", "NFR-012")]
    #[test]
    fn the_default_admission_limits_are_finite() {
        assert_eq!(
            PopulationAdmissionLimits::default(),
            PopulationAdmissionLimits {
                population_members: 100_000,
                work_units: 16_777_216,
                ancestor_steps: 16_777_216,
            }
        );
    }

    fn object_type(identity: &str, supertypes: &[&str]) -> DomainPackageRecord {
        DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key: DeclarationKey::fixture(identity),
            interface_features: None,
            abstract_type: false,
            supertypes: supertypes
                .iter()
                .map(|key| DeclarationKey::fixture(*key))
                .collect(),
        })
    }

    /// `model.A`, `model.B -> model.A` and a closed population over both.
    fn domain_package() -> DomainPackage {
        DomainPackage::new(
            DomainPackageRef::fixture("bundle.qsl204"),
            vec![
                object_type("model.A", &[]),
                object_type("model.B", &["model.A"]),
                DomainPackageRecord::Population(PopulationRecord {
                    key: DeclarationKey::fixture("model.pop"),
                    member_types: vec![
                        DeclarationKey::fixture("model.A"),
                        DeclarationKey::fixture("model.B"),
                    ],
                    extent: Extent::Closed,
                    symmetric: false,
                }),
            ],
        )
    }

    fn document() -> PopulationDocument {
        PopulationDocument {
            model_identity: "test/orders".to_owned(),
            members: vec![
                PopulationMember {
                    object: "a1".to_owned(),
                    type_identity: DeclarationKey::fixture("model.A"),
                    field_values: Vec::new(),
                },
                PopulationMember {
                    object: "b1".to_owned(),
                    type_identity: DeclarationKey::fixture("model.B"),
                    field_values: Vec::new(),
                },
            ],
        }
    }

    /// Admission reads its object universe from the caller's
    /// already-metered effective view and never normalizes the domain
    /// package itself -- neither a direct [`admit_binding`] nor an
    /// [`admit_invocation`], which admits a pre and a post document. The
    /// counter is `normalize::build`'s own per-thread call count, so any
    /// reintroduced normalization on the admission path turns this red.
    ///
    /// Mutation used: restoring the earlier `build`-backed universe
    /// lookup in `admit_binding_as` made `admit_binding` count one build
    /// and `admit_invocation` two.
    #[test]
    #[trace("QSpec-TC-198", "QSpec-FR-153-AC-1")]
    fn admission_never_normalizes_the_domain_package() {
        let domain_package = domain_package();
        let before_normalize = build_calls();
        let view = match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
            NormalizeOutcome::Completed(view) => view,
            other => panic!("expected a completed view, got {other:?}"),
        };
        assert_eq!(
            build_calls() - before_normalize,
            1,
            "normalize builds exactly once"
        );
        let population = DeclarationKey::fixture("model.pop");
        let context = InvocationContext {
            view: &view,
            population: &population,
            subtype_closure: GeneralizationClosure::Closed,
            declared_maximum: None,
        };

        let before_binding = build_calls();
        let binding = admit_binding(
            &view,
            &document(),
            &population,
            GeneralizationClosure::Closed,
            None,
            &mut AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED),
        );
        let AdmissionOutcome::Admitted(binding) = binding else {
            panic!("expected an admitted binding, got {binding:?}");
        };
        assert_eq!(build_calls(), before_binding, "admit_binding never builds");
        assert_eq!(
            *binding.universe(),
            view.object_universe().identity(),
            "the binding's universe is the view's own"
        );

        let before_invocation = build_calls();
        let effect = OperationEffect::default();
        let invocation = admit_invocation(
            context,
            &document(),
            &document(),
            &InvocationDelta {
                effect: &effect,
                declared_created: &[],
                declared_deleted: &[],
            },
            &mut AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED),
            &mut AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED),
        );
        assert!(
            matches!(invocation, AdmissionOutcome::Admitted(_)),
            "expected an admitted invocation, got {invocation:?}"
        );
        assert_eq!(
            build_calls(),
            before_invocation,
            "admit_invocation never builds, for either instant"
        );
    }
}
