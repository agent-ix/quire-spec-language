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
//! always by declared identity (a [`ProducerKey`]/[`EffectiveId`]), never by
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
//! # Scope boundary: `binding.subset-value`
//!
//! `value-accounting.md` charges `binding.subset-value` for "each value of
//! the subsetted feature of that object", which requires evaluating real
//! object field values against an FR-151 subsetting record. `crate::model`
//! has no FR-146 expression evaluator and this rung's [`PopulationMember`]
//! carries no field values at all (only its declared most-specific type and
//! object identity) — the same boundary
//! [`crate::model::conformance::check_subsetting`]'s own module doc already
//! records for the static case. [`AdmissionChargePoint::BindingSubsetValue`]
//! is declared for schedule completeness, but [`admit_binding`] never charges
//! it: no FR-153 acceptance criterion and no TC-198 vector exercises
//! subsetting (F1/P1/P2/P3 declare no [`crate::model::bundle::SubsettingRecord`]).
//! Wiring it needs the same evaluator bridge tracked for QSL #147's
//! FR-151/FR-152 criteria, not this rung.
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
//! belong to FR-143's own closed object environment, and this crate defines
//! no mapping from `crate::model`'s `EffectiveId`/`ProducerKey` identities
//! into that byte space anywhere. Fabricating one here without a documented
//! canonical encoding would risk a worse defect than the untyped result it
//! replaces, so this rung's typed wrappers stay in `crate::model`'s own
//! identity domain.
//!
//! # Binding/bundle correspondence
//!
//! A [`PopulationBinding`]'s fields are private; [`admit_binding`] is its
//! only constructor, and it stores the admitted `Bundle` itself (as an
//! `Arc`, cheap to clone) alongside the universe that bundle normalized to.
//! [`all_instances`] and [`lookup`] read that bound bundle — neither takes a
//! separate `bundle` argument — so there is no mismatched-bundle class to
//! guard against at query time: a query can only ever be evaluated against
//! the exact bundle [`admit_binding`] admitted, by construction, not by a
//! runtime comparison.
//!
//! An earlier revision of this module took a separate `bundle: &Bundle`
//! argument on both query functions and refused when it didn't match the
//! binding's own recorded header
//! (`bundle.model_selection != binding.model_selection`). Review found that
//! check unsound: nothing in this crate verifies `ModelSelection.export.digest`
//! against a bundle's actual `records` anywhere (including at admission), and
//! the test-only `ModelSelection::fixture` constructor derives the digest from
//! the identity string alone — so a caller could present a bundle with the
//! binding's exact admitted header (identity, revision, digest) but different
//! `records`, and the header comparison would pass while the query silently
//! answered against the wrong content (e.g. `all_instances` dropping a member
//! whose only supporting generalization record the caller removed). FR-153's
//! own Inputs clause already describes the population binding as carrying
//! "the FR-150 effective view and ModelSelection" itself, with the query
//! supplying only the requested type (and, for `lookup`, a key and absence
//! mode) — never a second, independently suppliable bundle — so binding the
//! bundle removes the unsound check by removing the parameter that made it
//! necessary, rather than trying to make the comparison itself sound.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::diagnostic::Code;
use crate::model::bundle::{Bundle, BundleRecord, ModelSelection};
use crate::model::conformance::{generals_by_specific, type_conforms};
use crate::model::dispatch::GeneralizationClosure;
use crate::model::key::{EffectiveId, ProducerKey};
use crate::model::normalize::{object_universe, EffectiveView, ModelRefusal};
use crate::value::{
    length_amount, CardinalityBound, Charge as ScalarCharge, ChargePoint as ScalarChargePoint,
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
    /// `binding.subset-value`. Declared for schedule completeness; see the
    /// module docs — never charged by this rung.
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

    /// Whether this rung's [`admit_binding`] ever charges this point.
    /// `false` only for [`Self::BindingSubsetValue`] — declared here for
    /// schedule completeness, never charged (see the module docs' scope
    /// boundary). Exposed so the declared-but-uncharged state is checkable
    /// in code, not only in a doc comment.
    pub fn is_charged(self) -> bool {
        !matches!(self, Self::BindingSubsetValue)
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

/// One member of an FCD FR-121 population document: `{object, type_identity}`.
/// This rung carries no field values (see the module docs' scope boundary).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationMember {
    /// The object's own declared identity.
    pub object: String,
    /// The member's declared most-specific type, as its original producer key.
    pub type_identity: ProducerKey,
}

/// An FCD FR-121 population document: `{closed_world, model_identity, members}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationDocument {
    /// Whether the document declares `closedWorld: true`.
    pub closed_world: bool,
    /// The document's own declared `modelIdentity`, which must name the
    /// binding's `ModelSelection` export identity.
    pub model_identity: String,
    /// Member records, in document order.
    pub members: Vec<PopulationMember>,
}

/// A `lookup<T>(p, r) absent m` key parameter: `r: Reference<S>`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LookupKey {
    /// `r`'s own static type `S`.
    pub static_type: ProducerKey,
    /// `r`'s realized reference key.
    pub key: ReferenceKey,
}

/// `lookup`'s absence mode, part of the query and never inferred from a
/// result type (FR-153's own table).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbsenceMode {
    /// `absent undefined`.
    Undefined,
    /// `absent empty`.
    Empty,
    /// `absent refused`.
    Refused,
}

// ---------------------------------------------------------------------------
// Binding admission
// ---------------------------------------------------------------------------

/// One admitted population binding: every admitted member, keyed by
/// [`ReferenceKey`] (so iteration is already canonical reference-key order),
/// with each member's original most-specific type retained for conformance
/// checking.
///
/// Every field is private; [`admit_binding`] is this type's only
/// constructor, so a caller cannot assemble a binding that was never checked
/// against a bundle's `modelIdentity`, object closure or subtype closure —
/// and, because the admitted bundle itself lives here, cannot later evaluate
/// [`all_instances`]/[`lookup`] against any bundle other than the one that
/// was checked (see the module docs' "Binding/bundle correspondence").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationBinding {
    /// The bundle this binding was admitted against. `Arc`, so cloning a
    /// binding never re-clones the bundle's own records.
    bundle: Arc<Bundle>,
    /// This binding's own object universe, computed once at admission.
    universe: EffectiveId,
    /// Every admitted member, ascending by [`ReferenceKey`].
    members: BTreeMap<ReferenceKey, ProducerKey>,
    /// The binding's declared maximum, or `None` for a binding with no
    /// declared maximum (`allInstances` is then `operator-ineligible`).
    declared_maximum: Option<u64>,
}

impl PopulationBinding {
    /// The full `ModelSelection` header this binding was admitted against.
    pub fn model_selection(&self) -> &ModelSelection {
        &self.bundle.model_selection
    }

    /// The `ModelSelection` export identity this binding was admitted
    /// against.
    pub fn model_identity(&self) -> &str {
        &self.bundle.model_selection.export.identity
    }

    /// This binding's own object universe.
    pub fn universe(&self) -> &EffectiveId {
        &self.universe
    }

    /// Every admitted member, ascending by [`ReferenceKey`].
    pub fn members(&self) -> &BTreeMap<ReferenceKey, ProducerKey> {
        &self.members
    }

    /// The binding's declared maximum, or `None` for a binding with no
    /// declared maximum (`allInstances` is then `operator-ineligible`).
    pub fn declared_maximum(&self) -> Option<u64> {
        self.declared_maximum
    }
}

/// The outcome of one [`admit_binding`] attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmissionOutcome {
    /// Every member was charged and admitted. Boxed: `PopulationBinding` now
    /// carries the full `ModelSelection` header, not just an identity
    /// `String`, which otherwise makes this the dominant variant by size
    /// (`clippy::large_enum_variant`).
    Admitted(Box<PopulationBinding>),
    /// A real defect refused admission outright; no binding.
    Refused(ModelRefusal),
    /// Object or subtype closure is not established. Not a refusal
    /// (`Code::IncompletePopulation`); no charge and no binding.
    UnknownClosure(ModelRefusal),
    /// A `PopulationAdmissionLimitsV1` counter was exhausted; no binding.
    Incomplete(AdmissionIncomplete),
}

/// Admits `document` against `bundle`/`view` into a [`PopulationBinding`],
/// per FR-153's "Environment key and closure". Decides, without a charge and
/// in order, the `modelIdentity` check, object closure and subtype closure;
/// then charges `binding.member` for each member record in document order,
/// deciding foreign-type and then duplicate-collapse/conflicting-identity
/// after each charge. Stops at the first refusal or denied charge.
pub fn admit_binding(
    bundle: &Bundle,
    view: &EffectiveView,
    document: &PopulationDocument,
    subtype_closure: GeneralizationClosure,
    declared_maximum: Option<u64>,
    meter: &mut AdmissionMeter,
) -> AdmissionOutcome {
    if document.model_identity != bundle.model_selection.export.identity {
        return AdmissionOutcome::Refused(ModelRefusal {
            code: Code::ForeignReference,
            cause: "foreign-model-selection",
            detail: format!(
                "population document names modelIdentity {}, not the binding's {}",
                document.model_identity, bundle.model_selection.export.identity
            ),
        });
    }
    if !document.closed_world {
        return AdmissionOutcome::UnknownClosure(ModelRefusal {
            code: Code::IncompletePopulation,
            cause: "incomplete-scope",
            detail: format!(
                "population document for {} does not declare closedWorld: true",
                bundle.model_selection.export.identity
            ),
        });
    }
    if matches!(subtype_closure, GeneralizationClosure::Open) {
        return AdmissionOutcome::UnknownClosure(ModelRefusal {
            code: Code::IncompletePopulation,
            cause: "unclosed-subtypes",
            detail: format!(
                "model selection {} naming {} does not have a closed generalization graph",
                bundle.model_selection.export.identity,
                document
                    .members
                    .first()
                    .map(|member| member.type_identity.identity.as_str())
                    .unwrap_or("<no members>")
            ),
        });
    }

    let universe = match object_universe(bundle) {
        Ok(universe) => universe.identity(),
        Err(refusal) => return AdmissionOutcome::Refused(refusal),
    };

    // Indexed once, not re-scanned per member: `view.declarations` and
    // `admitted` would otherwise each be linearly searched per member,
    // making admission O(n^2) against the O(n) `binding.member` charges it
    // records.
    let type_lookup: BTreeMap<ProducerKey, EffectiveId> = view
        .declarations
        .iter()
        .filter(|entry| entry.preimage.owner_effective_type.is_none())
        .map(|entry| (entry.preimage.original.clone(), entry.effective_id.clone()))
        .collect();

    let mut admitted: BTreeMap<ReferenceKey, ProducerKey> = BTreeMap::new();
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
                cause: "foreign-type",
                detail: format!(
                    "member {} names type {}, absent from the effective view",
                    member.object, member.type_identity.identity
                ),
            });
        };

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
                cause: "conflicting-identity",
                detail: format!(
                    "object {} is declared with conflicting types {} and {}",
                    key.object,
                    existing.type_identity.hex(),
                    key.type_identity.hex()
                ),
            });
        }
        by_object.insert(key.object.clone(), key.clone());
        admitted.insert(key, member.type_identity.clone());
    }

    // `binding.subset-value` is deliberately not walked here; see the module
    // docs' scope boundary.

    AdmissionOutcome::Admitted(Box::new(PopulationBinding {
        bundle: Arc::new(bundle.clone()),
        universe,
        members: admitted,
        declared_maximum,
    }))
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
    element_type: ProducerKey,
    bound: CardinalityBound,
    members: BTreeSet<ReferenceKey>,
}

impl ReferenceSet {
    /// The queried type `T`.
    pub fn element_type(&self) -> &ProducerKey {
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

fn is_object_type(bundle: &Bundle, key: &ProducerKey) -> bool {
    bundle
        .records
        .iter()
        .any(|record| matches!(record, BundleRecord::ObjectType(object) if &object.key == key))
}

/// `allInstances<T>(p)`: every member of `binding` whose most-specific type
/// conforms to `t`, once by reference key, in canonical reference-key order.
/// `binding.members()` already iterates in that order. Evaluated entirely
/// against `binding`'s own admitted bundle — there is no separate `bundle`
/// argument to mismatch (see the module docs' "Binding/bundle correspondence").
pub fn all_instances(
    binding: &PopulationBinding,
    t: &ProducerKey,
    meter: &mut ScalarMeter,
) -> AllInstancesOutcome {
    let bundle = &*binding.bundle;
    let Some(declared_maximum) = binding.declared_maximum() else {
        return AllInstancesOutcome::Refused(ModelRefusal {
            code: Code::IllTyped,
            cause: "operator-ineligible",
            detail: "population binding has no declared maximum".to_owned(),
        });
    };
    if !is_object_type(bundle, t) {
        return AllInstancesOutcome::Refused(ModelRefusal {
            code: Code::IllTyped,
            cause: "type-mismatch",
            detail: format!("{} is not a model object type", t.identity),
        });
    }

    let generals = generals_by_specific(bundle);
    let mut selected: BTreeSet<ReferenceKey> = BTreeSet::new();
    for (key, original_type) in binding.members() {
        if let Err(incomplete) = meter.charge(ScalarCharge::new(ScalarChargePoint::PopulationVisit))
        {
            return AllInstancesOutcome::Incomplete(incomplete);
        }
        match type_conforms(&generals, original_type, t) {
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
            cause: "above-maximum",
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
    queried_type: ProducerKey,
    key: ReferenceKey,
}

impl TypedReference {
    /// A reference to `key`, typed as `queried_type` (`T`).
    pub fn new(queried_type: ProducerKey, key: ReferenceKey) -> Self {
        Self { queried_type, key }
    }

    /// The queried type `T`.
    pub fn queried_type(&self) -> &ProducerKey {
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

/// `lookup<T>(p, r) absent m`: `r`'s presence in `binding`, per `mode`.
/// Charges `lookup.key` once per call; a present result then charges
/// `lookup.result-retain` with `value_occurrences`/`result_units` = 1 for a
/// bare `Reference<T>` (`undefined`/`refused` modes) or = 2 for an
/// `Option<Reference<T>>` (`empty` mode, the wrapper plus the reference); an
/// absent `empty`-mode result still retains 1 (the `none` wrapper alone),
/// while an absent `undefined`/`refused`-mode result retains nothing at all
/// — verified against TC-198 L03's three absence-mode vectors. Evaluated
/// entirely against `binding`'s own admitted bundle — there is no separate
/// `bundle` argument to mismatch (see the module docs' "Binding/bundle
/// correspondence").
pub fn lookup(
    binding: &PopulationBinding,
    t: &ProducerKey,
    r: &LookupKey,
    mode: AbsenceMode,
    meter: &mut ScalarMeter,
) -> LookupOutcome {
    let generals = generals_by_specific(&binding.bundle);
    match type_conforms(&generals, &r.static_type, t) {
        Ok(true) => {}
        Ok(false) => {
            return LookupOutcome::Refused(ModelRefusal {
                code: Code::IllTyped,
                cause: "type-mismatch",
                detail: format!(
                    "{} does not conform to {}",
                    r.static_type.identity, t.identity
                ),
            })
        }
        Err(refusal) => return LookupOutcome::Refused(refusal),
    }

    if let Err(incomplete) = meter.charge(
        ScalarCharge::new(ScalarChargePoint::LookupKey).size(ScalarLimitKind::ValueOccurrences, 1),
    ) {
        return LookupOutcome::Incomplete(incomplete);
    }

    if r.key.universe != *binding.universe() {
        return LookupOutcome::Refused(ModelRefusal {
            code: Code::ForeignReference,
            cause: "foreign-universe",
            detail: format!(
                "reference key names universe {}, not the binding's {}",
                r.key.universe.hex(),
                binding.universe().hex()
            ),
        });
    }

    if binding.members().contains_key(&r.key) {
        let occ = if matches!(mode, AbsenceMode::Empty) {
            2
        } else {
            1
        };
        if let Err(incomplete) = charge_result_retain(meter, occ) {
            return LookupOutcome::Incomplete(incomplete);
        }
        return LookupOutcome::Completed(Some(TypedReference::new(t.clone(), r.key.clone())));
    }

    match mode {
        AbsenceMode::Undefined => LookupOutcome::Undefined,
        AbsenceMode::Refused => LookupOutcome::Refused(ModelRefusal {
            code: Code::InvalidRuntimeInput,
            cause: "absent-key",
            detail: format!("{} is not a member of the bound population", r.key.object),
        }),
        AbsenceMode::Empty => {
            if let Err(incomplete) = charge_result_retain(meter, 1) {
                return LookupOutcome::Incomplete(incomplete);
            }
            LookupOutcome::Completed(None)
        }
    }
}
