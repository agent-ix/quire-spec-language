// SPDX-License-Identifier: AGPL-3.0-or-later
//! `quire.value.accounting/v1`: `u64` limits, named ordered charge points and
//! the exact incomplete record.
//!
//! Semantic-size counters record a high-water maximum; `work_units` and
//! `result_units` are cumulative. Every charge is atomic: the first
//! unavailable counter, in the order written in its definition row, returns
//! [`Incomplete`] and nothing is consumed.

use serde::{Deserialize, Serialize};

use quire_exact::Integer;

/// A `usize` length or count as an accounting amount.
///
/// Every supported target has pointers of at most 64 bits, so an in-memory
/// length always fits `u64`; a wider target would violate that invariant.
pub(crate) fn length_amount(length: usize) -> u64 {
    u64::try_from(length)
        .expect("in-memory lengths fit u64 on targets with pointers of at most 64 bits")
}

/// `ScalarLimitsV1`. Every member is required; zero is a real limit.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScalarLimits {
    /// Maximum magnitude bit length.
    pub integer_bits: u64,
    /// Maximum base-ten digit count.
    pub decimal_digits: u64,
    /// Maximum decimal-place shift in one operation.
    pub scale_expansion: u64,
    /// Maximum UTF-8 input bytes.
    pub text_input_bytes: u64,
    /// Maximum decoded Unicode scalars.
    pub text_scalars: u64,
    /// Maximum normalized output scalars.
    pub normalized_scalars: u64,
    /// Maximum unit graph edges.
    pub unit_edges: u64,
    /// Maximum typed value occurrences.
    pub value_occurrences: u64,
    /// Maximum cumulative work units.
    pub work_units: u64,
    /// Maximum cumulative retained result units.
    pub result_units: u64,
}

/// One counter of [`ScalarLimits`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LimitKind {
    /// `integer_bits`.
    IntegerBits,
    /// `decimal_digits`.
    DecimalDigits,
    /// `scale_expansion`.
    ScaleExpansion,
    /// `text_input_bytes`.
    TextInputBytes,
    /// `text_scalars`.
    TextScalars,
    /// `normalized_scalars`.
    NormalizedScalars,
    /// `unit_edges`.
    UnitEdges,
    /// `value_occurrences`.
    ValueOccurrences,
    /// `work_units`.
    WorkUnits,
    /// `result_units`.
    ResultUnits,
}

impl LimitKind {
    /// Every counter in `ScalarLimitsV1` member order.
    pub const ALL: [Self; 10] = [
        Self::IntegerBits,
        Self::DecimalDigits,
        Self::ScaleExpansion,
        Self::TextInputBytes,
        Self::TextScalars,
        Self::NormalizedScalars,
        Self::UnitEdges,
        Self::ValueOccurrences,
        Self::WorkUnits,
        Self::ResultUnits,
    ];

    /// Normative member name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IntegerBits => "integer_bits",
            Self::DecimalDigits => "decimal_digits",
            Self::ScaleExpansion => "scale_expansion",
            Self::TextInputBytes => "text_input_bytes",
            Self::TextScalars => "text_scalars",
            Self::NormalizedScalars => "normalized_scalars",
            Self::UnitEdges => "unit_edges",
            Self::ValueOccurrences => "value_occurrences",
            Self::WorkUnits => "work_units",
            Self::ResultUnits => "result_units",
        }
    }

    /// Whether this counter is cumulative rather than a high-water size.
    pub fn is_cumulative(self) -> bool {
        matches!(self, Self::WorkUnits | Self::ResultUnits)
    }

    fn index(self) -> usize {
        match self {
            Self::IntegerBits => 0,
            Self::DecimalDigits => 1,
            Self::ScaleExpansion => 2,
            Self::TextInputBytes => 3,
            Self::TextScalars => 4,
            Self::NormalizedScalars => 5,
            Self::UnitEdges => 6,
            Self::ValueOccurrences => 7,
            Self::WorkUnits => 8,
            Self::ResultUnits => 9,
        }
    }

    fn limit(self, limits: &ScalarLimits) -> u64 {
        match self {
            Self::IntegerBits => limits.integer_bits,
            Self::DecimalDigits => limits.decimal_digits,
            Self::ScaleExpansion => limits.scale_expansion,
            Self::TextInputBytes => limits.text_input_bytes,
            Self::TextScalars => limits.text_scalars,
            Self::NormalizedScalars => limits.normalized_scalars,
            Self::UnitEdges => limits.unit_edges,
            Self::ValueOccurrences => limits.value_occurrences,
            Self::WorkUnits => limits.work_units,
            Self::ResultUnits => limits.result_units,
        }
    }
}

/// A normative named charge point.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ChargePoint {
    /// `decimal.operands`.
    DecimalOperands,
    /// `decimal.scale-expansion`.
    DecimalScaleExpansion,
    /// `decimal.arithmetic`.
    DecimalArithmetic,
    /// `decimal.rounding`.
    DecimalRounding,
    /// `decimal.result-retain`.
    DecimalResultRetain,
    /// `text.input-bytes`.
    TextInputBytes,
    /// `text.decode-scalars`.
    TextDecodeScalars,
    /// `text.normalize-input`.
    TextNormalizeInput,
    /// `text.normalize-output`.
    TextNormalizeOutput,
    /// `text.result-retain`.
    TextResultRetain,
    /// `enum.identity-read`.
    EnumIdentityRead,
    /// `enum.result-retain`.
    EnumResultRetain,
    /// `unit.identity-read`.
    UnitIdentityRead,
    /// `unit.edge`.
    UnitEdge,
    /// `unit.rational-arithmetic`.
    UnitRationalArithmetic,
    /// `unit.target-domain`.
    UnitTargetDomain,
    /// `unit.result-retain`.
    UnitResultRetain,
    /// `integer-division.operands`.
    IntegerDivisionOperands,
    /// `integer-division.arithmetic`.
    IntegerDivisionArithmetic,
    /// `integer-division.domain-pair`.
    IntegerDivisionDomainPair,
    /// `integer-division.result-pair`.
    IntegerDivisionResultPair,
    /// `integer-modulus.operands`.
    IntegerModulusOperands,
    /// `integer-modulus.arithmetic`.
    IntegerModulusArithmetic,
    /// `integer-modulus.domain`.
    IntegerModulusDomain,
    /// `integer-modulus.result-retain`.
    IntegerModulusResultRetain,
    /// `ieee.operands`.
    IeeeOperands,
    /// `ieee.exact-intermediate`.
    IeeeExactIntermediate,
    /// `ieee.round`.
    IeeeRound,
    /// `ieee.result-retain`.
    IeeeResultRetain,
    /// `equality.plan-form`.
    EqualityPlanForm,
    /// `equality.plan`.
    EqualityPlan,
    /// `equality.pair`.
    EqualityPair,
    /// `equality.result-retain`.
    EqualityResultRetain,
    /// `function.call`: one checked function call.
    FunctionCall,
    /// `collection.element`.
    CollectionElement,
    /// `collection.visit`.
    CollectionVisit,
    /// `collection.member-walk`.
    CollectionMemberWalk,
    /// `collection.member-test`.
    CollectionMemberTest,
    /// `collection.bound`.
    CollectionBound,
    /// `collection.result-retain`.
    CollectionResultRetain,
    /// `composite.result-retain`.
    CompositeResultRetain,
    /// `integer-arithmetic.operands`.
    IntegerArithmeticOperands,
    /// `integer-arithmetic.arithmetic`.
    IntegerArithmeticArithmetic,
    /// `integer-arithmetic.result-retain`.
    IntegerArithmeticResultRetain,
    /// `rational-arithmetic.operands`.
    RationalArithmeticOperands,
    /// `rational-arithmetic.arithmetic`.
    RationalArithmeticArithmetic,
    /// `rational-arithmetic.normalize`.
    RationalArithmeticNormalize,
    /// `rational-arithmetic.result-retain`.
    RationalArithmeticResultRetain,
    /// `ordering.operands`.
    OrderingOperands,
    /// `ordering.arithmetic`.
    OrderingArithmetic,
    /// `ordering.result-retain`.
    OrderingResultRetain,
    /// `boolean.result-retain`.
    BooleanResultRetain,
    /// `lookup.key`.
    LookupKey,
    /// `lookup.result-retain`.
    LookupResultRetain,
    /// `population.visit`.
    PopulationVisit,
    /// `dispatch.select`: one dispatched `receiver.member(args)` call
    /// (FR-151, TC-196 D06).
    DispatchSelect,
}

impl ChargePoint {
    /// Every named point, grouped by family in normative order.
    pub const ALL: [Self; 56] = [
        Self::DecimalOperands,
        Self::DecimalScaleExpansion,
        Self::DecimalArithmetic,
        Self::DecimalRounding,
        Self::DecimalResultRetain,
        Self::TextInputBytes,
        Self::TextDecodeScalars,
        Self::TextNormalizeInput,
        Self::TextNormalizeOutput,
        Self::TextResultRetain,
        Self::EnumIdentityRead,
        Self::EnumResultRetain,
        Self::UnitIdentityRead,
        Self::UnitEdge,
        Self::UnitRationalArithmetic,
        Self::UnitTargetDomain,
        Self::UnitResultRetain,
        Self::IntegerDivisionOperands,
        Self::IntegerDivisionArithmetic,
        Self::IntegerDivisionDomainPair,
        Self::IntegerDivisionResultPair,
        Self::IntegerModulusOperands,
        Self::IntegerModulusArithmetic,
        Self::IntegerModulusDomain,
        Self::IntegerModulusResultRetain,
        Self::IeeeOperands,
        Self::IeeeExactIntermediate,
        Self::IeeeRound,
        Self::IeeeResultRetain,
        Self::EqualityPlanForm,
        Self::EqualityPlan,
        Self::EqualityPair,
        Self::EqualityResultRetain,
        Self::FunctionCall,
        Self::CollectionElement,
        Self::CollectionVisit,
        Self::CollectionMemberWalk,
        Self::CollectionMemberTest,
        Self::CollectionBound,
        Self::CollectionResultRetain,
        Self::CompositeResultRetain,
        Self::IntegerArithmeticOperands,
        Self::IntegerArithmeticArithmetic,
        Self::IntegerArithmeticResultRetain,
        Self::RationalArithmeticOperands,
        Self::RationalArithmeticArithmetic,
        Self::RationalArithmeticNormalize,
        Self::RationalArithmeticResultRetain,
        Self::OrderingOperands,
        Self::OrderingArithmetic,
        Self::OrderingResultRetain,
        Self::BooleanResultRetain,
        Self::LookupKey,
        Self::LookupResultRetain,
        Self::PopulationVisit,
        Self::DispatchSelect,
    ];

    /// Normative identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DecimalOperands => "decimal.operands",
            Self::DecimalScaleExpansion => "decimal.scale-expansion",
            Self::DecimalArithmetic => "decimal.arithmetic",
            Self::DecimalRounding => "decimal.rounding",
            Self::DecimalResultRetain => "decimal.result-retain",
            Self::TextInputBytes => "text.input-bytes",
            Self::TextDecodeScalars => "text.decode-scalars",
            Self::TextNormalizeInput => "text.normalize-input",
            Self::TextNormalizeOutput => "text.normalize-output",
            Self::TextResultRetain => "text.result-retain",
            Self::EnumIdentityRead => "enum.identity-read",
            Self::EnumResultRetain => "enum.result-retain",
            Self::UnitIdentityRead => "unit.identity-read",
            Self::UnitEdge => "unit.edge",
            Self::UnitRationalArithmetic => "unit.rational-arithmetic",
            Self::UnitTargetDomain => "unit.target-domain",
            Self::UnitResultRetain => "unit.result-retain",
            Self::IntegerDivisionOperands => "integer-division.operands",
            Self::IntegerDivisionArithmetic => "integer-division.arithmetic",
            Self::IntegerDivisionDomainPair => "integer-division.domain-pair",
            Self::IntegerDivisionResultPair => "integer-division.result-pair",
            Self::IntegerModulusOperands => "integer-modulus.operands",
            Self::IntegerModulusArithmetic => "integer-modulus.arithmetic",
            Self::IntegerModulusDomain => "integer-modulus.domain",
            Self::IntegerModulusResultRetain => "integer-modulus.result-retain",
            Self::IeeeOperands => "ieee.operands",
            Self::IeeeExactIntermediate => "ieee.exact-intermediate",
            Self::IeeeRound => "ieee.round",
            Self::IeeeResultRetain => "ieee.result-retain",
            Self::EqualityPlanForm => "equality.plan-form",
            Self::EqualityPlan => "equality.plan",
            Self::EqualityPair => "equality.pair",
            Self::EqualityResultRetain => "equality.result-retain",
            Self::FunctionCall => "function.call",
            Self::CollectionElement => "collection.element",
            Self::CollectionVisit => "collection.visit",
            Self::CollectionMemberWalk => "collection.member-walk",
            Self::CollectionMemberTest => "collection.member-test",
            Self::CollectionBound => "collection.bound",
            Self::CollectionResultRetain => "collection.result-retain",
            Self::CompositeResultRetain => "composite.result-retain",
            Self::IntegerArithmeticOperands => "integer-arithmetic.operands",
            Self::IntegerArithmeticArithmetic => "integer-arithmetic.arithmetic",
            Self::IntegerArithmeticResultRetain => "integer-arithmetic.result-retain",
            Self::RationalArithmeticOperands => "rational-arithmetic.operands",
            Self::RationalArithmeticArithmetic => "rational-arithmetic.arithmetic",
            Self::RationalArithmeticNormalize => "rational-arithmetic.normalize",
            Self::RationalArithmeticResultRetain => "rational-arithmetic.result-retain",
            Self::OrderingOperands => "ordering.operands",
            Self::OrderingArithmetic => "ordering.arithmetic",
            Self::OrderingResultRetain => "ordering.result-retain",
            Self::BooleanResultRetain => "boolean.result-retain",
            Self::LookupKey => "lookup.key",
            Self::LookupResultRetain => "lookup.result-retain",
            Self::PopulationVisit => "population.visit",
            Self::DispatchSelect => "dispatch.select",
        }
    }

    /// Resolve a normative identifier.
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|point| point.as_str() == code)
    }
}

/// The exact incomplete record: `incomplete { limit_kind, limit, consumed,
/// next_charge, charge_point }`. It never carries a partial value or any other
/// member.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Incomplete {
    /// First unavailable counter in `ScalarLimitsV1` field order.
    pub limit_kind: LimitKind,
    /// That counter's configured limit.
    pub limit: u64,
    /// That counter's consumed value before the denied charge.
    pub consumed: u64,
    /// The denied amount (a size for high-water counters, an addition for
    /// cumulative counters). It is a mathematical integer because a unit
    /// integer power can require more than `u64::MAX`.
    pub next_charge: Integer,
    /// The named point whose charge was denied.
    pub charge_point: ChargePoint,
}

/// An NFR-071 fault-injection request: deny the `occurrence`th (1-based) charge
/// at `point`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct InjectedDenial {
    /// The named point to deny.
    pub point: ChargePoint,
    /// Which occurrence of that point to deny, starting at one.
    pub occurrence: u64,
}

/// One exact `{ counter: amount }` charge vector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Charge {
    point: ChargePoint,
    sizes: Vec<(LimitKind, Integer)>,
    work_units: Integer,
    result_units: Integer,
}

impl Charge {
    pub(crate) fn new(point: ChargePoint) -> Self {
        Self {
            point,
            sizes: Vec::new(),
            work_units: Integer::one(),
            result_units: Integer::zero(),
        }
    }

    pub(crate) fn size(self, kind: LimitKind, amount: u64) -> Self {
        self.exact_size(kind, Integer::from(amount))
    }

    /// A semantic-size amount that may exceed `u64::MAX`.
    pub(crate) fn exact_size(mut self, kind: LimitKind, amount: Integer) -> Self {
        self.sizes.push((kind, amount));
        self
    }

    /// A `work_units` addition other than the default one.
    pub(crate) fn work(mut self, amount: Integer) -> Self {
        self.work_units = amount;
        self
    }

    pub(crate) fn results(self, amount: u64) -> Self {
        self.exact_results(Integer::from(amount))
    }

    /// A `result_units` addition that may exceed `u64::MAX`.
    pub(crate) fn exact_results(mut self, amount: Integer) -> Self {
        self.result_units = amount;
        self
    }
}

/// A per-request scalar meter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Meter {
    limits: ScalarLimits,
    consumed: [u64; 10],
    occurrences: Vec<(ChargePoint, u64)>,
    denial: Option<InjectedDenial>,
    admitted: Vec<ChargePoint>,
}

impl Meter {
    /// A fresh meter with nothing consumed.
    pub fn new(limits: ScalarLimits) -> Self {
        Self {
            limits,
            consumed: [0; 10],
            occurrences: Vec::new(),
            denial: None,
            admitted: Vec::new(),
        }
    }

    /// Add the qualification seam that denies one exact named charge.
    pub fn with_injected_denial(mut self, denial: InjectedDenial) -> Self {
        self.denial = Some(denial);
        self
    }

    /// The configured limits.
    pub fn limits(&self) -> &ScalarLimits {
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

    fn occurrence(&self, point: ChargePoint) -> u64 {
        self.occurrences
            .iter()
            .find(|(seen, _)| *seen == point)
            .map_or(0, |(_, count)| *count)
    }

    fn incomplete(&self, kind: LimitKind, next: Integer, point: ChargePoint) -> Incomplete {
        Incomplete {
            limit_kind: kind,
            limit: kind.limit(&self.limits),
            consumed: self.consumed(kind),
            next_charge: next,
            charge_point: point,
        }
    }

    /// The qualification-seam record: as if the `work_units` limit were the
    /// work already consumed, so `limit = consumed = w`.
    fn check_injected(&self, point: ChargePoint, work_units: Integer) -> Result<(), Incomplete> {
        match self.denial {
            Some(denial)
                if denial.point == point
                    && self.occurrence(point).checked_add(1) == Some(denial.occurrence) =>
            {
                let consumed = self.consumed(LimitKind::WorkUnits);
                Err(Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: consumed,
                    consumed,
                    next_charge: work_units,
                    charge_point: point,
                })
            }
            _ => Ok(()),
        }
    }

    /// Atomically admit `charge` or return the first unavailable counter in
    /// `ScalarLimitsV1` field order.
    pub(crate) fn charge(&mut self, mut charge: Charge) -> Result<(), Incomplete> {
        let point = charge.point;
        self.check_injected(point, charge.work_units.clone())?;
        // Every semantic-size counter precedes `work_units` and `result_units`
        // in field order.
        charge.sizes.sort_by_key(|(kind, _)| kind.index());
        let mut sizes = Vec::with_capacity(charge.sizes.len());
        for (kind, amount) in charge.sizes {
            match amount.to_u64() {
                Some(amount) if amount <= kind.limit(&self.limits) => sizes.push((kind, amount)),
                _ => return Err(self.incomplete(kind, amount, point)),
            }
        }
        let cumulative = |kind: LimitKind, amount: Integer| {
            amount
                .to_u64()
                .and_then(|addition| self.consumed(kind).checked_add(addition))
                .filter(|total| *total <= kind.limit(&self.limits))
                .ok_or_else(|| self.incomplete(kind, amount, point))
        };
        let work = cumulative(LimitKind::WorkUnits, charge.work_units)?;
        let results = cumulative(LimitKind::ResultUnits, charge.result_units)?;
        for (kind, amount) in sizes {
            let slot = &mut self.consumed[kind.index()];
            *slot = (*slot).max(amount);
        }
        self.consumed[LimitKind::WorkUnits.index()] = work;
        self.consumed[LimitKind::ResultUnits.index()] = results;
        self.admit(point);
        Ok(())
    }

    fn admit(&mut self, point: ChargePoint) {
        match self.occurrences.iter_mut().find(|(seen, _)| *seen == point) {
            Some((_, count)) => *count = count.saturating_add(1),
            None => self.occurrences.push((point, 1)),
        }
        self.admitted.push(point);
    }

    /// The FR-149 `equality.plan` charge: size `value_occurrences` is the
    /// planned pair count; without changing any consumed counter it requires
    /// `pairs + 2` remaining work units and one remaining result unit, then
    /// consumes the plan's own work unit.
    pub(crate) fn charge_plan(&mut self, pairs: &Integer) -> Result<(), Incomplete> {
        let point = ChargePoint::EqualityPlan;
        let reservation = pairs.add(&Integer::from(2_u64));
        self.check_injected(point, reservation.clone())?;
        let kind = LimitKind::ValueOccurrences;
        let size = pairs
            .to_u64()
            .filter(|size| *size <= kind.limit(&self.limits))
            .ok_or_else(|| self.incomplete(kind, pairs.clone(), point))?;
        let remaining = |kind: LimitKind| {
            Integer::from(kind.limit(&self.limits)).sub(&Integer::from(self.consumed(kind)))
        };
        if reservation > remaining(LimitKind::WorkUnits) {
            return Err(self.incomplete(LimitKind::WorkUnits, reservation, point));
        }
        if remaining(LimitKind::ResultUnits) < Integer::one() {
            return Err(self.incomplete(LimitKind::ResultUnits, Integer::one(), point));
        }
        let slot = &mut self.consumed[kind.index()];
        *slot = (*slot).max(size);
        let work = &mut self.consumed[LimitKind::WorkUnits.index()];
        *work = work.saturating_add(1);
        self.admit(point);
        Ok(())
    }
}
