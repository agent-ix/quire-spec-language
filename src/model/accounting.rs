// SPDX-License-Identifier: AGPL-3.0-or-later
//! `ModelNormalizationLimitsV1`: exact `u64` limits, named ordered charge
//! points and the exact incomplete record for FR-150 normalization.
//!
//! Mirrors `crate::value::accounting`'s `Meter`/`ChargePoint`/`Incomplete`
//! shape (same atomic-first-denial semantics), over this rung's own,
//! independent counter set — `ScalarLimitsV1` is A03's complete-value meter
//! and is not reused here, per `value-accounting.md`: "These counters are
//! independent of `ScalarLimitsV1`."

/// `ModelNormalizationLimitsV1`. Every member is required; zero is a real
/// limit and never means unlimited.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ModelNormalizationLimits {
    /// High-water count of decoded producer records.
    pub producer_records: u64,
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
}

impl ModelNormalizationLimits {
    /// A limit set large enough that no charge in this rung is denied.
    pub const UNLIMITED: Self = Self {
        producer_records: u64::MAX,
        derivation_facts: u64::MAX,
        effective_declarations: u64::MAX,
        dispatch_candidates: u64::MAX,
        hashed_bytes: u64::MAX,
        work_units: u64::MAX,
    };
}

/// One counter of [`ModelNormalizationLimits`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LimitKind {
    /// `producer_records`.
    ProducerRecords,
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
}

impl LimitKind {
    /// Every counter, in `ModelNormalizationLimitsV1` field order.
    pub const ALL: [Self; 6] = [
        Self::ProducerRecords,
        Self::DerivationFacts,
        Self::EffectiveDeclarations,
        Self::DispatchCandidates,
        Self::HashedBytes,
        Self::WorkUnits,
    ];

    /// Normative member name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProducerRecords => "producer_records",
            Self::DerivationFacts => "derivation_facts",
            Self::EffectiveDeclarations => "effective_declarations",
            Self::DispatchCandidates => "dispatch_candidates",
            Self::HashedBytes => "hashed_bytes",
            Self::WorkUnits => "work_units",
        }
    }

    /// Whether this counter is cumulative rather than a high-water size.
    pub fn is_cumulative(self) -> bool {
        matches!(self, Self::HashedBytes | Self::WorkUnits)
    }

    fn index(self) -> usize {
        match self {
            Self::ProducerRecords => 0,
            Self::DerivationFacts => 1,
            Self::EffectiveDeclarations => 2,
            Self::DispatchCandidates => 3,
            Self::HashedBytes => 4,
            Self::WorkUnits => 5,
        }
    }

    fn limit(self, limits: &ModelNormalizationLimits) -> u64 {
        match self {
            Self::ProducerRecords => limits.producer_records,
            Self::DerivationFacts => limits.derivation_facts,
            Self::EffectiveDeclarations => limits.effective_declarations,
            Self::DispatchCandidates => limits.dispatch_candidates,
            Self::HashedBytes => limits.hashed_bytes,
            Self::WorkUnits => limits.work_units,
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
    pub const ALL: [Self; 14] = [
        Self::NormalizeRecord,
        Self::NormalizeFact,
        Self::NormalizeCycleCheck,
        Self::NormalizeRedefinitionCheck,
        Self::NormalizeConflictCheck,
        Self::NormalizeDeclaration,
        Self::NormalizeHash,
        Self::ConformanceAxis,
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

/// One exact `{ counter: amount }` charge vector.
pub(super) struct Charge {
    point: ChargePoint,
    sizes: Vec<(LimitKind, u64)>,
    work_units: u64,
}

impl Charge {
    pub(super) fn new(point: ChargePoint) -> Self {
        Self {
            point,
            sizes: Vec::new(),
            work_units: 1,
        }
    }

    pub(super) fn size(mut self, kind: LimitKind, amount: u64) -> Self {
        self.sizes.push((kind, amount));
        self
    }

    /// A `work_units` addition other than the default one.
    pub(super) fn work(mut self, amount: u64) -> Self {
        self.work_units = amount;
        self
    }
}

/// A per-normalization-run scalar meter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Meter {
    limits: ModelNormalizationLimits,
    consumed: [u64; 6],
    admitted: Vec<ChargePoint>,
}

impl Meter {
    /// A fresh meter with nothing consumed.
    pub fn new(limits: ModelNormalizationLimits) -> Self {
        Self {
            limits,
            consumed: [0; 6],
            admitted: Vec::new(),
        }
    }

    /// The configured limits.
    pub fn limits(&self) -> &ModelNormalizationLimits {
        &self.limits
    }

    /// Consumed value of one counter.
    pub fn consumed(&self, kind: LimitKind) -> u64 {
        self.consumed[kind.index()]
    }

    /// Every admitted charge point in admission order.
    pub fn admitted_charges(&self) -> &[ChargePoint] {
        &self.admitted
    }

    fn incomplete(&self, kind: LimitKind, next: u64, point: ChargePoint) -> Incomplete {
        Incomplete {
            limit_kind: kind,
            limit: kind.limit(&self.limits),
            consumed: self.consumed(kind),
            next_charge: next,
            charge_point: point,
        }
    }

    /// Atomically admit `charge` or return the first unavailable counter in
    /// `ModelNormalizationLimitsV1` field order. Every counter's resulting
    /// value is computed before any counter is mutated, so a denial leaves
    /// every counter exactly as it was.
    pub(super) fn charge(&mut self, mut charge: Charge) -> Result<(), Incomplete> {
        let point = charge.point;
        charge.sizes.sort_by_key(|(kind, _)| kind.index());
        let mut results = Vec::with_capacity(charge.sizes.len());
        for (kind, amount) in &charge.sizes {
            let candidate = if kind.is_cumulative() {
                self.consumed(*kind).checked_add(*amount)
            } else {
                Some((*amount).max(self.consumed(*kind)))
            };
            match candidate {
                Some(value) if value <= kind.limit(&self.limits) => results.push((*kind, value)),
                _ => return Err(self.incomplete(*kind, *amount, point)),
            }
        }
        let work_total = match self
            .consumed(LimitKind::WorkUnits)
            .checked_add(charge.work_units)
        {
            Some(total) if total <= self.limits.work_units => total,
            _ => return Err(self.incomplete(LimitKind::WorkUnits, charge.work_units, point)),
        };
        for (kind, value) in results {
            self.consumed[kind.index()] = value;
        }
        self.consumed[LimitKind::WorkUnits.index()] = work_total;
        self.admitted.push(point);
        Ok(())
    }
}
