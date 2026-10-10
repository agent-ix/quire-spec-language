// SPDX-License-Identifier: AGPL-3.0-or-later
//! `ModelNormalizationLimitsV1`: exact `u64` limits, named ordered charge
//! points and the exact incomplete record for FR-150 normalization.
//!
//! Mirrors `crate::value::accounting`'s `Meter`/`ChargePoint`/`Incomplete`
//! shape (same atomic-first-denial semantics), over this rung's own,
//! independent counter set — `ScalarLimitsV1` is A03's complete-value meter
//! and is not reused here, per `value-accounting.md`: "These counters are
//! independent of `ScalarLimitsV1`."

use quire_exact::Cancel;

/// `ModelNormalizationLimitsV1`. Every member is required; zero is a real
/// limit and never means unlimited.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ModelNormalizationLimits {
    /// High-water count of decoded declaration records.
    pub declaration_records: u64,
    /// High-water count of derivation facts.
    pub derivation_facts: u64,
    /// High-water count of effective declarations.
    pub effective_declarations: u64,
    /// High-water count of dispatch candidates.
    pub dispatch_candidates: u64,
    /// Cumulative hashed preimage bytes.
    pub hashed_bytes: u64,
    /// Cumulative work units.
    pub work_units: u64,
    /// Ceiling on the `supertypes` edges one walk follows, charged before
    /// each edge is followed: one conformance walk (FR-082), or one type's
    /// ancestor-path enumeration in normalization. Under multiple
    /// supertypes it counts the walk's closure, never a chain depth, and
    /// each walk starts a fresh count. A target or ancestor `n` edges up a
    /// chain is reached at `ancestor_steps == n`. Applies to normalization
    /// and to every conformance, dispatch-applicability, dominance and
    /// systems-flow check that owns a `Meter`. Reaching it refuses
    /// `ModelRefusalCause::AncestorSteps` naming this bound.
    pub ancestor_steps: u64,
    /// Ceiling on the `redefines` edges one dispatch-family link follows,
    /// charged before each edge is followed and counted over all the link's
    /// walks together (FR-083): the family enumeration of
    /// `dispatch::link_dispatch` and the effective-precondition walks of
    /// `check::checked_dispatch_operation`, each distinct edge once. It
    /// counts edges, never a chain depth: a linear chain of `n`
    /// redefinitions is admitted at `family_steps == n`. Reaching it
    /// refuses `ModelRefusalCause::FamilySteps` naming this bound.
    pub family_steps: u64,
    /// The byte limit one domain package document is read under
    /// (`intake.input_bytes`, FR-260): it rides here so every stage that
    /// admits a package carries it with the other model limits.
    pub intake: qsl_foundation::IntakeLimits,
}

/// NFR-012's finite default ceilings, which a caller may raise or
/// lower field by field. `spec/non-functional/NFR-012-*.md` states each
/// value and its derivation.
impl Default for ModelNormalizationLimits {
    fn default() -> Self {
        Self {
            declaration_records: 100_000,
            derivation_facts: 1_600_000,
            effective_declarations: 1_600_000,
            dispatch_candidates: 1_600_000,
            hashed_bytes: 268_435_456,
            work_units: 16_777_216,
            ancestor_steps: quire_semantic_value::declaration::DEFAULT_ANCESTOR_STEPS,
            family_steps: 16_777_216,
            intake: qsl_foundation::IntakeLimits::default(),
        }
    }
}

impl ModelNormalizationLimits {
    /// A limit set large enough that no charge in this rung is denied.
    /// Test-only: production callers start from [`Self::default`].
    #[cfg(any(test, feature = "test-support"))]
    pub const UNLIMITED: Self = Self {
        declaration_records: u64::MAX,
        derivation_facts: u64::MAX,
        effective_declarations: u64::MAX,
        dispatch_candidates: u64::MAX,
        hashed_bytes: u64::MAX,
        work_units: u64::MAX,
        ancestor_steps: u64::MAX,
        family_steps: u64::MAX,
        intake: qsl_foundation::IntakeLimits {
            input_bytes: u64::MAX,
        },
    };
}

impl ModelNormalizationLimits {
    /// These limits with `declaration_records` set (`model.declaration_records`).
    #[must_use]
    pub const fn with_declaration_records(mut self, bound: u64) -> Self {
        self.declaration_records = bound;
        self
    }

    /// These limits with `derivation_facts` set (`model.derivation_facts`).
    #[must_use]
    pub const fn with_derivation_facts(mut self, bound: u64) -> Self {
        self.derivation_facts = bound;
        self
    }

    /// These limits with `effective_declarations` set
    /// (`model.effective_declarations`).
    #[must_use]
    pub const fn with_effective_declarations(mut self, bound: u64) -> Self {
        self.effective_declarations = bound;
        self
    }

    /// These limits with `dispatch_candidates` set
    /// (`model.dispatch_candidates`).
    #[must_use]
    pub const fn with_dispatch_candidates(mut self, bound: u64) -> Self {
        self.dispatch_candidates = bound;
        self
    }

    /// These limits with `hashed_bytes` set (`model.hashed_bytes`).
    #[must_use]
    pub const fn with_hashed_bytes(mut self, bound: u64) -> Self {
        self.hashed_bytes = bound;
        self
    }

    /// These limits with `work_units` set (`model.work_units`).
    #[must_use]
    pub const fn with_work_units(mut self, bound: u64) -> Self {
        self.work_units = bound;
        self
    }

    /// These limits with `ancestor_steps` set (`model.ancestor_steps`).
    #[must_use]
    pub const fn with_ancestor_steps(mut self, bound: u64) -> Self {
        self.ancestor_steps = bound;
        self
    }

    /// These limits with `family_steps` set (`model.family_steps`).
    #[must_use]
    pub const fn with_family_steps(mut self, bound: u64) -> Self {
        self.family_steps = bound;
        self
    }
}

impl LimitKind {
    /// The setting that raises this counter (FR-255).
    pub const fn setting(self) -> qsl_foundation::Setting {
        use qsl_foundation::Setting;
        match self {
            Self::DeclarationRecords => Setting::ModelDeclarationRecords,
            Self::DerivationFacts => Setting::ModelDerivationFacts,
            Self::EffectiveDeclarations => Setting::ModelEffectiveDeclarations,
            Self::DispatchCandidates => Setting::ModelDispatchCandidates,
            Self::HashedBytes => Setting::ModelHashedBytes,
            Self::WorkUnits => Setting::ModelWorkUnits,
            Self::AncestorSteps => Setting::ModelAncestorSteps,
        }
    }
}

/// FR-255: the one mapping from each field to its setting.
impl qsl_foundation::SettingLimits for ModelNormalizationLimits {
    fn bounds(&self) -> Vec<(qsl_foundation::Setting, u64)> {
        use qsl_foundation::Setting;
        vec![
            (Setting::ModelDeclarationRecords, self.declaration_records),
            (Setting::ModelDerivationFacts, self.derivation_facts),
            (
                Setting::ModelEffectiveDeclarations,
                self.effective_declarations,
            ),
            (Setting::ModelDispatchCandidates, self.dispatch_candidates),
            (Setting::ModelHashedBytes, self.hashed_bytes),
            (Setting::ModelWorkUnits, self.work_units),
            (Setting::ModelAncestorSteps, self.ancestor_steps),
            (Setting::ModelFamilySteps, self.family_steps),
            (Setting::IntakeInputBytes, self.intake.input_bytes),
        ]
    }

    fn set_bound(&mut self, setting: qsl_foundation::Setting, bound: u64) -> bool {
        use qsl_foundation::Setting;
        match setting {
            Setting::ModelDeclarationRecords => self.declaration_records = bound,
            Setting::ModelDerivationFacts => self.derivation_facts = bound,
            Setting::ModelEffectiveDeclarations => self.effective_declarations = bound,
            Setting::ModelDispatchCandidates => self.dispatch_candidates = bound,
            Setting::ModelHashedBytes => self.hashed_bytes = bound,
            Setting::ModelWorkUnits => self.work_units = bound,
            Setting::ModelAncestorSteps => self.ancestor_steps = bound,
            Setting::ModelFamilySteps => self.family_steps = bound,
            Setting::IntakeInputBytes => self.intake.input_bytes = bound,
            _ => return false,
        }
        true
    }
}

/// One counter of [`ModelNormalizationLimits`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LimitKind {
    /// `declaration_records`.
    DeclarationRecords,
    /// `derivation_facts`.
    DerivationFacts,
    /// `effective_declarations`.
    EffectiveDeclarations,
    /// `dispatch_candidates`.
    DispatchCandidates,
    /// `hashed_bytes`.
    HashedBytes,
    /// `work_units`.
    WorkUnits,
    /// Per-walk `ancestor_steps`; this is not a retained meter counter.
    AncestorSteps,
}

impl LimitKind {
    /// Every counter, in `ModelNormalizationLimitsV1` field order.
    pub const ALL: [Self; 6] = [
        Self::DeclarationRecords,
        Self::DerivationFacts,
        Self::EffectiveDeclarations,
        Self::DispatchCandidates,
        Self::HashedBytes,
        Self::WorkUnits,
    ];

    /// Normative member name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeclarationRecords => "declaration_records",
            Self::DerivationFacts => "derivation_facts",
            Self::EffectiveDeclarations => "effective_declarations",
            Self::DispatchCandidates => "dispatch_candidates",
            Self::HashedBytes => "hashed_bytes",
            Self::WorkUnits => "work_units",
            Self::AncestorSteps => "ancestor_steps",
        }
    }

    /// Whether this counter is cumulative rather than a high-water size.
    pub fn is_cumulative(self) -> bool {
        matches!(self, Self::HashedBytes | Self::WorkUnits)
    }

    fn index(self) -> Option<usize> {
        match self {
            Self::DeclarationRecords => Some(0),
            Self::DerivationFacts => Some(1),
            Self::EffectiveDeclarations => Some(2),
            Self::DispatchCandidates => Some(3),
            Self::HashedBytes => Some(4),
            Self::WorkUnits => Some(5),
            Self::AncestorSteps => None,
        }
    }

    fn limit(self, limits: &ModelNormalizationLimits) -> u64 {
        match self {
            Self::DeclarationRecords => limits.declaration_records,
            Self::DerivationFacts => limits.derivation_facts,
            Self::EffectiveDeclarations => limits.effective_declarations,
            Self::DispatchCandidates => limits.dispatch_candidates,
            Self::HashedBytes => limits.hashed_bytes,
            Self::WorkUnits => limits.work_units,
            Self::AncestorSteps => limits.ancestor_steps,
        }
    }
}

/// A normative named model-normalization charge point.
///
/// This rung charges the phase-1/2/3/5 points below, plus
/// phase 4's `normalize.redefinition-check`/`normalize.conflict-check`
/// (`quire.model.normalize.redefine/v1`, TC-195 N06); see
/// `crate::model::normalize` module docs for that pass's exact scope. FR-151
/// adds `conformance.axis` (`crate::model::conformance`) and
/// `dispatch.subtype`/`dispatch.candidate`/`dispatch.dominance`
/// (`crate::model::dispatch`); see those modules' docs for their scope —
/// notably, dispatch's own work-unit costing is this crate's own choice
/// where FR-151's exact prose figures depend on authored operation bodies
/// this rung does not model. FR-152 adds `systems.kind`,
/// `systems.connection-condition` and `systems.allocation`
/// (`crate::model::systems`); its own module docs record that this rung
/// does not implement `systems.resolve`/`model.navigate` (static/runtime
/// navigation), which need a qualified-name binder and an object
/// population this crate does not build.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ChargePoint {
    /// `normalize.record`.
    NormalizeRecord,
    /// `normalize.fact`.
    NormalizeFact,
    /// `normalize.cycle-check`.
    NormalizeCycleCheck,
    /// `normalize.redefinition-check`.
    NormalizeRedefinitionCheck,
    /// `normalize.conflict-check`.
    NormalizeConflictCheck,
    /// `normalize.declaration`.
    NormalizeDeclaration,
    /// `normalize.hash`.
    NormalizeHash,
    /// `conformance.axis`.
    ConformanceAxis,
    /// Availability of the next edge in one ancestor walk.
    ModelAncestorEdge,
    /// `dispatch.subtype`.
    DispatchSubtype,
    /// `dispatch.candidate`.
    DispatchCandidate,
    /// `dispatch.dominance`.
    DispatchDominance,
    /// `systems.kind`.
    SystemsKind,
    /// `systems.connection-condition`.
    SystemsConnectionCondition,
    /// `systems.allocation`.
    SystemsAllocation,
}

impl ChargePoint {
    /// Every named point this rung charges, in first-use order.
    pub const ALL: [Self; 15] = [
        Self::NormalizeRecord,
        Self::NormalizeFact,
        Self::NormalizeCycleCheck,
        Self::NormalizeRedefinitionCheck,
        Self::NormalizeConflictCheck,
        Self::NormalizeDeclaration,
        Self::NormalizeHash,
        Self::ConformanceAxis,
        Self::ModelAncestorEdge,
        Self::DispatchSubtype,
        Self::DispatchCandidate,
        Self::DispatchDominance,
        Self::SystemsKind,
        Self::SystemsConnectionCondition,
        Self::SystemsAllocation,
    ];

    /// Normative identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NormalizeRecord => "normalize.record",
            Self::NormalizeFact => "normalize.fact",
            Self::NormalizeCycleCheck => "normalize.cycle-check",
            Self::NormalizeRedefinitionCheck => "normalize.redefinition-check",
            Self::NormalizeConflictCheck => "normalize.conflict-check",
            Self::NormalizeDeclaration => "normalize.declaration",
            Self::NormalizeHash => "normalize.hash",
            Self::ConformanceAxis => "conformance.axis",
            Self::ModelAncestorEdge => "model.ancestor-edge",
            Self::DispatchSubtype => "dispatch.subtype",
            Self::DispatchCandidate => "dispatch.candidate",
            Self::DispatchDominance => "dispatch.dominance",
            Self::SystemsKind => "systems.kind",
            Self::SystemsConnectionCondition => "systems.connection-condition",
            Self::SystemsAllocation => "systems.allocation",
        }
    }
}

/// The exact incomplete record: `incomplete { limit_kind, limit, consumed,
/// next_charge, charge_point }`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Incomplete {
    /// First unavailable counter in `ModelNormalizationLimitsV1` field order.
    pub limit_kind: LimitKind,
    /// That counter's configured limit.
    pub limit: u64,
    /// That counter's consumed value before the denied charge.
    pub consumed: u64,
    /// The denied amount.
    pub next_charge: u64,
    /// The named point whose charge was denied.
    pub charge_point: ChargePoint,
}

impl Incomplete {
    /// This stop as the stage limit outcome FR-255 Behavior 1 describes: the
    /// counter's setting, its bound and the total the denied charge would
    /// have reached.
    pub fn limit_exceeded(&self) -> qsl_foundation::diagnostic::LimitExceeded {
        qsl_foundation::diagnostic::LimitExceeded::new(
            self.limit_kind.setting(),
            self.limit,
            u128::from(self.consumed) + u128::from(self.next_charge),
        )
    }
}

/// One exact `{ counter: amount }` charge vector: `work_units` plus at most
/// one sized counter, since no model charge point sizes more than one.
/// Fixed-size, so building and admitting a charge allocates nothing.
pub(super) struct Charge {
    point: ChargePoint,
    size: Option<(LimitKind, u64)>,
    work_units: u64,
}

impl Charge {
    pub(super) fn new(point: ChargePoint) -> Self {
        Self {
            point,
            size: None,
            work_units: 1,
        }
    }

    /// The one counter this charge sizes, and the amount.
    pub(super) fn size(mut self, kind: LimitKind, amount: u64) -> Self {
        self.size = Some((kind, amount));
        self
    }

    /// A `work_units` addition other than the default one.
    pub(super) fn work(mut self, amount: u64) -> Self {
        self.work_units = amount;
        self
    }
}

/// A per-normalization-run scalar meter.
///
/// **A count, not a log, as for the kernel meter.** A
/// production meter holds only fixed-size counters: the limits, the six
/// consumed values and the number of admitted charges. The counters own no
/// heap memory, so the meter that bounds normalization's work does not
/// itself grow with that work; the const assertion below holds that in every
/// production build. The meter also shares the caller's
/// [`Cancel`] handle, which is a reference count and not per-charge state.
/// The ordered charge log ([`Meter::admitted_charges`]) exists only under
/// the `test-support` feature, which only a dev-dependency may enable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Meter {
    counters: Counters,
    cancel: Option<Cancel>,
    #[cfg(feature = "test-support")]
    admitted: Vec<ChargePoint>,
}

/// The fixed-size state a [`Meter`] charges against.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Counters {
    limits: ModelNormalizationLimits,
    consumed: [u64; 6],
    admissions: u64,
}

// The counters own no heap memory. A type with no drop glue holds no
// `Vec`, `Box` or `String`, so a heap-owning field added to it fails the
// build rather than a test.
const _: () = assert!(!std::mem::needs_drop::<Counters>());

impl Meter {
    /// A fresh meter with nothing consumed.
    pub fn new(limits: ModelNormalizationLimits) -> Self {
        Self {
            counters: Counters {
                limits,
                consumed: [0; 6],
                admissions: 0,
            },
            cancel: None,
            #[cfg(feature = "test-support")]
            admitted: Vec::new(),
        }
    }

    /// This meter, polling `cancel` at every charge (FR-276): a cancelled
    /// handle denies the charge as an exhausted work budget does.
    #[must_use]
    pub fn with_cancel(mut self, cancel: Cancel) -> Self {
        self.cancel = Some(cancel);
        self
    }

    /// The configured limits.
    pub fn limits(&self) -> &ModelNormalizationLimits {
        &self.counters.limits
    }

    /// Consumed value of one retained counter. `AncestorSteps` belongs to
    /// a walk, so the operation meter retains no consumption for it.
    pub fn consumed(&self, kind: LimitKind) -> u64 {
        kind.index().map_or(0, |index| self.counters.consumed[index])
    }

    /// How many charges this meter has admitted.
    pub fn admission_count(&self) -> u64 {
        self.counters.admissions
    }

    /// Every admitted charge point in admission order. Test-only: a
    /// production meter keeps [`Self::admission_count`], not a log.
    #[cfg(feature = "test-support")]
    pub fn admitted_charges(&self) -> &[ChargePoint] {
        &self.admitted
    }

    fn incomplete(&self, kind: LimitKind, next: u64, point: ChargePoint) -> Incomplete {
        Incomplete {
            limit_kind: kind,
            limit: kind.limit(&self.counters.limits),
            consumed: self.consumed(kind),
            next_charge: next,
            charge_point: point,
        }
    }

    /// Atomically admit `charge` or return the first unavailable counter in
    /// `ModelNormalizationLimitsV1` field order. Every counter's resulting
    /// value is computed before any counter is mutated, so a denial leaves
    /// every counter exactly as it was.
    pub(super) fn charge(&mut self, charge: Charge) -> Result<(), Incomplete> {
        let point = charge.point;
        if self.cancel.as_ref().is_some_and(Cancel::poll) {
            return Err(self.incomplete(LimitKind::WorkUnits, charge.work_units, point));
        }
        let sized = match charge.size {
            None => None,
            Some((kind, amount)) => {
                let candidate = if kind.is_cumulative() {
                    self.consumed(kind).checked_add(amount)
                } else {
                    Some(amount.max(self.consumed(kind)))
                };
                match candidate {
                    Some(value) if value <= kind.limit(&self.counters.limits) => {
                        Some((kind, value))
                    }
                    _ => return Err(self.incomplete(kind, amount, point)),
                }
            }
        };
        let work_total = match self
            .consumed(LimitKind::WorkUnits)
            .checked_add(charge.work_units)
        {
            Some(total) if total <= self.counters.limits.work_units => total,
            _ => return Err(self.incomplete(LimitKind::WorkUnits, charge.work_units, point)),
        };
        if let Some((kind, value)) = sized {
            if let Some(index) = kind.index() {
                self.counters.consumed[index] = value;
            }
        }
        self.counters.consumed[5] = work_total;
        self.counters.admissions = self.counters.admissions.saturating_add(1);
        #[cfg(feature = "test-support")]
        self.admitted.push(point);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// A production `Meter` and `Charge` have no drop glue, so
    /// they own no `Vec`, `Box` or `String`: the meter holds no per-charge
    /// state however many charges it admits, and its admission count still
    /// counts every one. Built only without `test-support` (`cargo test -p
    /// qsl-semantics`, which `make ci` runs): with it, the meter keeps its
    /// ordered charge log, which allocates.
    #[cfg(not(feature = "test-support"))]
    #[trace("TC-434", "NFR-012")]
    #[test]
    fn a_production_meter_owns_no_heap_memory_and_counts_every_charge() {
        assert!(!std::mem::needs_drop::<Counters>());
        assert!(!std::mem::needs_drop::<Charge>());
        let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
        let mut admitted = 0_u64;
        for target in [1_u64, 1_000, 100_000] {
            while admitted < target {
                admitted += 1;
                meter
                    .charge(
                        Charge::new(ChargePoint::NormalizeFact)
                            .size(LimitKind::DerivationFacts, admitted),
                    )
                    .expect("an unlimited meter admits every charge");
            }
            assert_eq!(meter.admission_count(), target);
            assert_eq!(meter.consumed(LimitKind::DerivationFacts), target);
        }
    }

    /// A denied charge leaves every counter and the admission count as they
    /// were.
    #[trace("TC-434", "NFR-012")]
    #[test]
    fn a_denied_charge_changes_nothing() {
        let mut meter = Meter::new(ModelNormalizationLimits {
            hashed_bytes: 10,
            ..ModelNormalizationLimits::UNLIMITED
        });
        meter
            .charge(Charge::new(ChargePoint::NormalizeHash).size(LimitKind::HashedBytes, 6))
            .expect("6 of 10 bytes is admitted");
        let denied = meter
            .charge(Charge::new(ChargePoint::NormalizeHash).size(LimitKind::HashedBytes, 5))
            .expect_err("11 of 10 bytes is denied");
        assert_eq!(
            denied,
            Incomplete {
                limit_kind: LimitKind::HashedBytes,
                limit: 10,
                consumed: 6,
                next_charge: 5,
                charge_point: ChargePoint::NormalizeHash,
            }
        );
        assert_eq!(meter.consumed(LimitKind::HashedBytes), 6);
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 1);
        assert_eq!(meter.admission_count(), 1);
    }
}
