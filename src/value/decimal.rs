// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-140 exact decimals: coefficient/scale values, the six rounding spellings,
//! canonical-rational loss records and metered evaluation.
//!
//! Every intermediate is an exact integer or reduced rational; no binary
//! floating-point value exists anywhere on this path.

use std::cmp::Ordering;

use super::accounting::{Charge, ChargePoint, LimitKind, Meter};
use super::comparison::{IllTyped, IllTypedCause};
use super::integer::Integer;
use super::outcome::{Outcome, Refusal, Stop, Undefined};
use super::rational::Rational;

/// An authored or computed `(coefficient, scale)` pair denoting
/// `coefficient × 10^-scale`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DecimalRepresentation {
    coefficient: Integer,
    scale: u32,
}

impl DecimalRepresentation {
    /// The exact pair.
    pub fn new(coefficient: Integer, scale: u32) -> Self {
        Self { coefficient, scale }
    }

    /// Signed coefficient.
    pub fn coefficient(&self) -> &Integer {
        &self.coefficient
    }

    /// Nonnegative scale.
    pub fn scale(&self) -> u32 {
        self.scale
    }

    /// Remove trailing decimal zeros while `scale > 0`; zero becomes `(0, 0)`.
    pub fn normalized(&self) -> Self {
        if self.coefficient.is_zero() {
            return Self::new(Integer::zero(), 0);
        }
        let ten = Integer::from(10_i64);
        let mut coefficient = self.coefficient.clone();
        let mut scale = self.scale;
        while scale > 0 {
            let (quotient, remainder) = coefficient.div_rem_truncating(&ten);
            if !remainder.is_zero() {
                break;
            }
            coefficient = quotient;
            scale -= 1;
        }
        Self::new(coefficient, scale)
    }

    /// The exact mathematical value.
    pub fn to_rational(&self) -> Rational {
        Rational::from_integer(self.coefficient.clone())
            .divided_by_power_of_ten(u64::from(self.scale))
    }
}

/// A decimal value retaining its pre-normalized representation provenance.
///
/// Equality and ordering are mathematical and use [`Decimal::normalized`];
/// the type deliberately has no structural `PartialEq`.
#[derive(Clone, Debug)]
pub struct Decimal {
    representation: DecimalRepresentation,
    normalized: DecimalRepresentation,
}

impl Decimal {
    /// A decimal with this exact representation.
    pub fn new(coefficient: Integer, scale: u32) -> Self {
        Self::from_representation(DecimalRepresentation::new(coefficient, scale))
    }

    /// A decimal retaining `representation` as provenance.
    pub fn from_representation(representation: DecimalRepresentation) -> Self {
        let normalized = representation.normalized();
        Self {
            representation,
            normalized,
        }
    }

    /// Pre-normalized provenance.
    pub fn representation(&self) -> &DecimalRepresentation {
        &self.representation
    }

    /// Canonical mathematical representation.
    pub fn normalized(&self) -> &DecimalRepresentation {
        &self.normalized
    }

    /// Mathematical ordering. This is the unmetered scalar primitive consumed
    /// by the metered equality matrix; it is not itself an evaluator result.
    pub fn compare(&self, other: &Self) -> Ordering {
        self.normalized
            .to_rational()
            .cmp(&other.normalized.to_rational())
    }

    /// Mathematical equality over normalized values.
    pub fn numerically_equal(&self, other: &Self) -> bool {
        self.normalized == other.normalized
    }
}

/// A rounding spelling; an omitted spelling is strict [`RoundingMode::Exact`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RoundingMode {
    /// Refuse any discarded nonzero digit.
    #[default]
    Exact,
    /// Round toward zero.
    TowardZero,
    /// Round toward positive infinity.
    TowardPositive,
    /// Round toward negative infinity.
    TowardNegative,
    /// Nearest; ties choose the even coefficient.
    NearestEven,
    /// Nearest; ties choose the greater absolute coefficient.
    NearestAway,
}

impl RoundingMode {
    /// Every spelling in grammar order.
    pub const ALL: [Self; 6] = [
        Self::Exact,
        Self::TowardZero,
        Self::TowardPositive,
        Self::TowardNegative,
        Self::NearestEven,
        Self::NearestAway,
    ];

    /// Source spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::TowardZero => "toward-zero",
            Self::TowardPositive => "toward-positive",
            Self::TowardNegative => "toward-negative",
            Self::NearestEven => "nearest-even",
            Self::NearestAway => "nearest-away",
        }
    }

    /// Resolve a source spelling.
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.as_str() == code)
    }
}

/// `DecimalLoss { exact_numerator, exact_denominator, rounded_coefficient,
/// rounded_scale, mode }`. The exact value is a canonical rational.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DecimalLoss {
    exact: Rational,
    rounded: DecimalRepresentation,
    mode: RoundingMode,
}

impl DecimalLoss {
    /// Reduced exact numerator.
    pub fn exact_numerator(&self) -> &Integer {
        self.exact.numerator()
    }

    /// Reduced positive exact denominator.
    pub fn exact_denominator(&self) -> &Integer {
        self.exact.denominator()
    }

    /// The exact mathematical value.
    pub fn exact(&self) -> &Rational {
        &self.exact
    }

    /// Pre-normalized rounded coefficient.
    pub fn rounded_coefficient(&self) -> &Integer {
        self.rounded.coefficient()
    }

    /// Pre-normalized rounded scale.
    pub fn rounded_scale(&self) -> u32 {
        self.rounded.scale()
    }

    /// The selected non-`exact` mode.
    pub fn mode(&self) -> RoundingMode {
        self.mode
    }
}

/// A well-formed `Decimal[lo, hi; smin, smax; mode]` type: inclusive
/// membership coefficient and scale bounds plus the rounding spelling. Its
/// target scale is `smax`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecimalType {
    lower: Integer,
    upper: Integer,
    min_scale: u32,
    max_scale: u32,
    rounding: RoundingMode,
}

impl DecimalType {
    /// Type-check a declaration; `lo > hi`, `smin > smax` or
    /// `smax > u32::MAX` is `ill_typed`.
    pub fn new(
        lower: Integer,
        upper: Integer,
        min_scale: u64,
        max_scale: u64,
        rounding: RoundingMode,
    ) -> Result<Self, IllTyped> {
        let malformed = IllTyped {
            cause: IllTypedCause::MalformedDecimalType,
        };
        let max_scale = u32::try_from(max_scale).map_err(|_| malformed)?;
        let min_scale = u32::try_from(min_scale).map_err(|_| malformed)?;
        if lower > upper || min_scale > max_scale {
            return Err(malformed);
        }
        Ok(Self {
            lower,
            upper,
            min_scale,
            max_scale,
            rounding,
        })
    }

    /// Inclusive lower membership coefficient `lo`.
    pub fn lower(&self) -> &Integer {
        &self.lower
    }

    /// Inclusive upper membership coefficient `hi`.
    pub fn upper(&self) -> &Integer {
        &self.upper
    }

    /// Inclusive minimum membership scale `smin`.
    pub fn min_scale(&self) -> u32 {
        self.min_scale
    }

    /// Inclusive maximum membership scale `smax`, also the target scale.
    pub fn max_scale(&self) -> u32 {
        self.max_scale
    }

    /// Selected rounding spelling.
    pub fn rounding(&self) -> RoundingMode {
        self.rounding
    }

    /// FR-140 value-only membership. With normalized (`c`, `s`), the value is a
    /// member exactly when `s* = max(s, smin) <= smax` and
    /// `lo <= c × 10^(s* - s) <= hi`. The lifted coefficient is never
    /// materialized and no charge is made.
    pub fn contains(&self, value: &Decimal) -> bool {
        let normalized = value.normalized();
        let scale = normalized.scale().max(self.min_scale);
        if scale > self.max_scale {
            return false;
        }
        let shift = u64::from(scale - normalized.scale());
        let coefficient = normalized.coefficient();
        compare_shifted(coefficient, shift, &self.lower).is_ge()
            && compare_shifted(coefficient, shift, &self.upper).is_le()
    }
}

/// Compare `value × 10^shift` with `bound` without materializing the power.
fn compare_shifted(value: &Integer, shift: u64, bound: &Integer) -> Ordering {
    let sign = |integer: &Integer| {
        if integer.is_zero() {
            Ordering::Equal
        } else if integer.is_negative() {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    };
    match sign(value).cmp(&sign(bound)) {
        Ordering::Equal if value.is_zero() => Ordering::Equal,
        Ordering::Equal => {
            let magnitude = shifted_digits(value, shift)
                .cmp(&bound.decimal_digits())
                .then_with(|| {
                    // Equal digit counts bound `shift` by the materialized bound.
                    value
                        .abs()
                        .mul(&Integer::power_of_ten(shift))
                        .cmp(&bound.abs())
                });
            if value.is_negative() {
                magnitude.reverse()
            } else {
                magnitude
            }
        }
        unequal => unequal,
    }
}

/// One FR-140 decimal operation.
#[derive(Clone, Copy, Debug)]
pub enum DecimalOperation<'a> {
    /// `a + b`.
    Add(&'a Decimal, &'a Decimal),
    /// `a - b`.
    Subtract(&'a Decimal, &'a Decimal),
    /// `a * b`.
    Multiply(&'a Decimal, &'a Decimal),
    /// `-a`.
    Negate(&'a Decimal),
    /// `a / b`.
    Divide(&'a Decimal, &'a Decimal),
    /// Explicit conversion of `a` into the target type.
    Round(&'a Decimal),
}

/// A completed decimal result.
#[derive(Clone, Debug)]
pub struct DecimalResult {
    value: Decimal,
    loss: Option<DecimalLoss>,
}

/// Result identity is the exact retained representation and loss record.
impl PartialEq for DecimalResult {
    fn eq(&self, other: &Self) -> bool {
        self.value.representation == other.value.representation && self.loss == other.loss
    }
}

impl Eq for DecimalResult {}

impl DecimalResult {
    /// The result, retaining its representation (`v × 10^T`, `T`).
    pub fn value(&self) -> &Decimal {
        &self.value
    }

    /// The loss record when a rounding step occurred.
    pub fn loss(&self) -> Option<&DecimalLoss> {
        self.loss.as_ref()
    }
}

/// Evaluate `operation` into `target` under `quire.value.accounting/v1`.
pub fn evaluate_decimal(
    operation: DecimalOperation<'_>,
    target: &DecimalType,
    meter: &mut Meter,
) -> Outcome<DecimalResult> {
    Outcome::from_stop(evaluate(operation, target, meter))
}

/// One retained coefficient multiplied by `10^shift`.
type Shifted<'a> = (&'a Integer, u64);

/// The retained-representation plan of an operation.
enum Plan<'a> {
    /// `left ± right` or `left × right` at `scale`.
    Combine {
        combine: Combine,
        left: Shifted<'a>,
        right: Shifted<'a>,
        scale: u64,
    },
    /// `N/D` already in units of `10^-T`.
    Divide {
        numerator: Shifted<'a>,
        denominator: Shifted<'a>,
    },
    /// Negation or conversion of one coefficient at `scale`.
    Unary {
        negate: bool,
        operand: &'a Integer,
        scale: u64,
    },
}

#[derive(Clone, Copy)]
enum Combine {
    Add,
    Subtract,
    Multiply,
}

fn expanded_side(sides: [Shifted<'_>; 2]) -> (u64, Option<Shifted<'_>>) {
    let side = sides.into_iter().find(|(_, shift)| *shift > 0);
    (side.map_or(0, |(_, shift)| shift), side)
}

fn retained_parts(value: &Decimal) -> Shifted<'_> {
    let representation = &value.representation;
    (
        representation.coefficient(),
        u64::from(representation.scale()),
    )
}

impl<'a> Plan<'a> {
    /// The `decimal.scale-expansion` shift and the expanded coefficient, if any.
    fn expansion(&self) -> (u64, Option<Shifted<'a>>) {
        let expanded = expanded_side;
        match self {
            Self::Combine {
                combine: Combine::Multiply,
                ..
            }
            | Self::Unary { .. } => (0, None),
            Self::Combine { left, right, .. } => expanded([*left, *right]),
            Self::Divide {
                numerator,
                denominator,
            } => expanded([*numerator, *denominator]),
        }
    }
}

fn plan(operation: DecimalOperation<'_>, target_scale: u64) -> Plan<'_> {
    let parts = retained_parts;
    match operation {
        DecimalOperation::Add(a, b) | DecimalOperation::Subtract(a, b) => {
            let ((ca, sa), (cb, sb)) = (parts(a), parts(b));
            let scale = sa.max(sb);
            let combine = if matches!(operation, DecimalOperation::Add(..)) {
                Combine::Add
            } else {
                Combine::Subtract
            };
            Plan::Combine {
                combine,
                left: (ca, scale - sa),
                right: (cb, scale - sb),
                scale,
            }
        }
        DecimalOperation::Multiply(a, b) => {
            let ((ca, sa), (cb, sb)) = (parts(a), parts(b));
            Plan::Combine {
                combine: Combine::Multiply,
                left: (ca, 0),
                right: (cb, 0),
                scale: sa + sb,
            }
        }
        DecimalOperation::Divide(a, b) => {
            // N = ca × 10^max(0, T + sb - sa), D = cb × 10^max(0, sa - sb - T).
            let ((ca, sa), (cb, sb)) = (parts(a), parts(b));
            let up = target_scale + sb;
            Plan::Divide {
                numerator: (ca, up.saturating_sub(sa)),
                denominator: (cb, sa.saturating_sub(up)),
            }
        }
        DecimalOperation::Negate(a) | DecimalOperation::Round(a) => {
            let (coefficient, scale) = parts(a);
            Plan::Unary {
                negate: matches!(operation, DecimalOperation::Negate(_)),
                operand: coefficient,
                scale,
            }
        }
    }
}

fn evaluate(
    operation: DecimalOperation<'_>,
    target: &DecimalType,
    meter: &mut Meter,
) -> Result<DecimalResult, Stop> {
    let inputs = operands(operation);
    let count = u64::try_from(inputs.len()).unwrap_or(u64::MAX);
    meter.charge(
        Charge::new(ChargePoint::DecimalOperands)
            .size(
                LimitKind::IntegerBits,
                max_of(
                    inputs
                        .iter()
                        .map(|value| value.coefficient().magnitude_bits()),
                ),
            )
            .size(
                LimitKind::DecimalDigits,
                max_of(
                    inputs
                        .iter()
                        .map(|value| value.coefficient().decimal_digits()),
                ),
            )
            .size(LimitKind::ValueOccurrences, count),
    )?;
    reject_zero_divisor(operation)?;

    let target_scale = u64::from(target.max_scale());
    let plan = plan(operation, target_scale);
    let (shift, expanded) = plan.expansion();
    let mut expansion =
        Charge::new(ChargePoint::DecimalScaleExpansion).size(LimitKind::ScaleExpansion, shift);
    if let Some((coefficient, shift)) = expanded {
        expansion = expansion
            .exact_size(LimitKind::IntegerBits, shifted_bits(coefficient, shift))
            .size(LimitKind::DecimalDigits, shifted_digits(coefficient, shift));
    }
    meter.charge(expansion)?;

    // The exact intermediate `numerator/denominator` in units of
    // `10^-working_scale`.
    let (intermediate, working_scale) = match plan {
        Plan::Combine {
            combine,
            left,
            right,
            scale,
        } => {
            let (a, b) = (expand_one(left), expand_one(right));
            let value = match combine {
                Combine::Add => a.add(&b),
                Combine::Subtract => a.sub(&b),
                Combine::Multiply => a.mul(&b),
            };
            (Rational::from_integer(value), scale)
        }
        Plan::Divide {
            numerator,
            denominator,
        } => (
            Rational::new(expand_one(numerator), expand_one(denominator))
                .map_err(|_| Stop::Undefined(Undefined::DivisionByZero))?,
            target_scale,
        ),
        Plan::Unary {
            negate,
            operand,
            scale,
        } => {
            let value = if negate {
                operand.neg()
            } else {
                operand.clone()
            };
            (Rational::from_integer(value), scale)
        }
    };
    meter.charge(
        Charge::new(ChargePoint::DecimalArithmetic)
            .size(LimitKind::IntegerBits, intermediate.max_part_bits())
            .size(
                LimitKind::DecimalDigits,
                intermediate
                    .numerator()
                    .decimal_digits()
                    .max(intermediate.denominator().decimal_digits()),
            ),
    )?;

    let placed = match in_target_units(&intermediate, working_scale, target.max_scale()) {
        TargetUnits::Exact { coefficient, scale } => Placed {
            coefficient,
            scale,
            loss: None,
        },
        TargetUnits::RoundingStep(units) => {
            let mode = target.rounding();
            let rounded = round(&units, mode).ok_or(Stop::Refused(Refusal::InexactDecimal))?;
            meter.charge(
                Charge::new(ChargePoint::DecimalRounding)
                    .size(LimitKind::IntegerBits, rounded.magnitude_bits())
                    .size(LimitKind::DecimalDigits, rounded.decimal_digits()),
            )?;
            Placed {
                loss: Some(DecimalLoss {
                    exact: intermediate.divided_by_power_of_ten(working_scale),
                    rounded: DecimalRepresentation::new(rounded.clone(), target.max_scale()),
                    mode,
                }),
                coefficient: rounded,
                scale: target.max_scale(),
            }
        }
    };
    placed.check_membership(target).map_err(Stop::Refused)?;
    let (bits, digits) = placed.retained_sizes(target);
    meter.charge(
        Charge::new(ChargePoint::DecimalResultRetain)
            .exact_size(LimitKind::IntegerBits, bits)
            .size(LimitKind::DecimalDigits, digits)
            .size(LimitKind::ValueOccurrences, 1)
            .results(1),
    )?;
    Ok(placed.retain(target))
}

/// An exact rational placed at a target's maximum scale `T`, before target
/// membership and retention.
pub(crate) struct Placed {
    coefficient: Integer,
    scale: u32,
    loss: Option<DecimalLoss>,
}

impl DecimalType {
    /// Place `value` at `T` under the target rounding mode; strict `exact`
    /// refuses a nonzero discarded digit.
    pub(crate) fn place(&self, value: &Rational) -> Result<Placed, Refusal> {
        let (numerator, denominator) = (value.numerator(), value.denominator());
        let terminating = terminating_scale(denominator)
            .and_then(|scale| u32::try_from(scale).ok())
            .filter(|scale| *scale <= self.max_scale);
        match terminating {
            Some(scale) => {
                // `n/d = n × (10^k / d) × 10^-k`; `10^k` has at most `k` more
                // bits than `d` has twos and fives.
                let factor = Integer::power_of_ten(u64::from(scale)).exact_div(denominator);
                Ok(Placed {
                    coefficient: numerator.mul(&factor),
                    scale,
                    loss: None,
                })
            }
            None => {
                let units = Rational::from_integer(
                    numerator.mul(&Integer::power_of_ten(u64::from(self.max_scale))),
                )
                // A reduced denominator is positive, so the division exists.
                .div(&Rational::from_integer(denominator.clone()))
                .ok_or(Refusal::InexactDecimal)?;
                let rounded = round(&units, self.rounding).ok_or(Refusal::InexactDecimal)?;
                Ok(Placed {
                    loss: Some(DecimalLoss {
                        exact: value.clone(),
                        rounded: DecimalRepresentation::new(rounded.clone(), self.max_scale),
                        mode: self.rounding,
                    }),
                    coefficient: rounded,
                    scale: self.max_scale,
                })
            }
        }
    }
}

/// The least `k` with `denominator | 10^k`, or `None` when the positive
/// `denominator` has a prime factor other than two or five.
fn terminating_scale(denominator: &Integer) -> Option<u64> {
    let mut remaining = denominator.clone();
    let mut counts = [0_u64; 2];
    for (count, prime) in counts.iter_mut().zip([2_i64, 5]) {
        let prime = Integer::from(prime);
        loop {
            let (quotient, remainder) = remaining.div_mod_floor(&prime);
            if !remainder.is_zero() {
                break;
            }
            remaining = quotient;
            *count += 1;
        }
    }
    (remaining == Integer::one()).then(|| counts[0].max(counts[1]))
}

impl Placed {
    /// `(integer_bits, decimal_digits)` of the retained coefficient `v × 10^T`.
    pub(crate) fn retained_sizes(&self, target: &DecimalType) -> (Integer, u64) {
        let lift = u64::from(target.max_scale) - u64::from(self.scale);
        (
            shifted_bits(&self.coefficient, lift),
            shifted_digits(&self.coefficient, lift),
        )
    }

    /// Refuse a value outside the target's declared membership.
    pub(crate) fn check_membership(&self, target: &DecimalType) -> Result<(), Refusal> {
        if target.contains(&Decimal::new(self.coefficient.clone(), self.scale)) {
            Ok(())
        } else {
            Err(Refusal::DecimalOutOfDomain)
        }
    }

    /// The completed result retaining `(v × 10^T, T)`.
    pub(crate) fn retain(self, target: &DecimalType) -> DecimalResult {
        let lift = u64::from(target.max_scale) - u64::from(self.scale);
        DecimalResult {
            value: Decimal::new(expand_one((&self.coefficient, lift)), target.max_scale),
            loss: self.loss,
        }
    }
}

fn reject_zero_divisor(operation: DecimalOperation<'_>) -> Result<(), Stop> {
    if let DecimalOperation::Divide(_, divisor) = operation {
        if divisor.normalized().coefficient().is_zero() {
            return Err(Stop::Undefined(Undefined::DivisionByZero));
        }
    }
    Ok(())
}

/// Whether the exact result is an integer multiple of `10^-T`.
enum TargetUnits {
    /// The exact value `coefficient × 10^-scale` with `scale <= T`.
    Exact { coefficient: Integer, scale: u32 },
    /// A rounding step over this non-integer value in units of `10^-T`.
    RoundingStep(Rational),
}

/// `intermediate` is in units of `10^-working_scale`; a division intermediate
/// is already in target units.
fn in_target_units(intermediate: &Rational, working_scale: u64, target_scale: u32) -> TargetUnits {
    let (units, scale) = match u32::try_from(working_scale) {
        Ok(scale) if scale <= target_scale => (intermediate.clone(), scale),
        _ => (
            intermediate.divided_by_power_of_ten(working_scale - u64::from(target_scale)),
            target_scale,
        ),
    };
    if units.is_integer() {
        TargetUnits::Exact {
            coefficient: units.numerator().clone(),
            scale,
        }
    } else {
        TargetUnits::RoundingStep(units)
    }
}

fn expand_one((coefficient, shift): Shifted<'_>) -> Integer {
    if shift == 0 {
        coefficient.clone()
    } else {
        coefficient.mul(&Integer::power_of_ten(shift))
    }
}

/// `bits(c × 10^shift)`, derived without allocating the power of ten.
fn shifted_bits(value: &Integer, shift: u64) -> Integer {
    Integer::power_product_bits(value, &Integer::from(10_i64), &Integer::from(shift))
}

/// `digits(c × 10^shift)`, derived without allocating the power of ten.
fn shifted_digits(value: &Integer, shift: u64) -> u64 {
    if value.is_zero() {
        1
    } else {
        value.decimal_digits().saturating_add(shift)
    }
}

/// Round a non-integer reduced rational to an integer, or `None` for `exact`.
fn round(value: &Rational, mode: RoundingMode) -> Option<Integer> {
    let (floor, remainder) = value.numerator().div_mod_floor(value.denominator());
    let ceiling = floor.add(&Integer::one());
    let negative = value.numerator().is_negative();
    let nearest = || match remainder.add(&remainder).cmp(value.denominator()) {
        Ordering::Less => Some(floor.clone()),
        Ordering::Greater => Some(ceiling.clone()),
        Ordering::Equal => None,
    };
    match mode {
        RoundingMode::Exact => None,
        RoundingMode::TowardZero => Some(if negative { ceiling } else { floor }),
        RoundingMode::TowardPositive => Some(ceiling),
        RoundingMode::TowardNegative => Some(floor),
        RoundingMode::NearestEven => Some(nearest().unwrap_or_else(|| {
            if floor.is_even() {
                floor.clone()
            } else {
                ceiling.clone()
            }
        })),
        RoundingMode::NearestAway => Some(nearest().unwrap_or_else(|| {
            if negative {
                floor.clone()
            } else {
                ceiling.clone()
            }
        })),
    }
}

fn operands(operation: DecimalOperation<'_>) -> Vec<&DecimalRepresentation> {
    match operation {
        DecimalOperation::Add(a, b)
        | DecimalOperation::Subtract(a, b)
        | DecimalOperation::Multiply(a, b)
        | DecimalOperation::Divide(a, b) => vec![a.representation(), b.representation()],
        DecimalOperation::Negate(a) | DecimalOperation::Round(a) => vec![a.representation()],
    }
}

fn max_of(values: impl Iterator<Item = u64>) -> u64 {
    values.max().unwrap_or(0)
}
