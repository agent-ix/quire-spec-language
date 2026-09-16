// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-140 exact decimals: coefficient/scale values, the six rounding spellings,
//! canonical-rational loss records and metered evaluation.
//!
//! Every intermediate is an exact integer or reduced rational; no binary
//! floating-point value exists anywhere on this path.

use std::cmp::Ordering;

use super::accounting::{length_amount, Charge, ChargePoint, LimitKind, Meter};
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
    ///
    /// Signs decide first; equal signs compare the magnitudes aligned to the
    /// larger scale without materializing a power of ten larger than the
    /// operands (see [`compare_shifted`]).
    pub fn compare(&self, other: &Self) -> Ordering {
        let (left, right) = (&self.normalized, &other.normalized);
        let (left_scale, right_scale) = (u64::from(left.scale), u64::from(right.scale));
        // `l / 10^ls` against `r / 10^rs`, both sides multiplied by `10^max(ls, rs)`.
        if left_scale >= right_scale {
            compare_shifted(
                &right.coefficient,
                left_scale - right_scale,
                &left.coefficient,
            )
            .reverse()
        } else {
            compare_shifted(
                &left.coefficient,
                right_scale - left_scale,
                &right.coefficient,
            )
        }
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
    exact: ExactLossValue,
    rounded: DecimalRepresentation,
    mode: RoundingMode,
}

/// The reduced exact value `numerator / (cofactor × 2^twos × 5^fives)` with
/// `cofactor` positive and coprime to ten.
///
/// The factored denominator is canonical, so structural equality is value
/// equality. Evaluation never charges the power of ten a working scale adds
/// to the denominator, so it keeps that power as exponents; only the
/// consumer-side accessors materialize it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ExactLossValue {
    numerator: Integer,
    cofactor: Integer,
    twos: u64,
    fives: u64,
}

impl ExactLossValue {
    /// The canonical form of `value / 10^ten_exponent` for a reduced `value`.
    fn scaled(value: &Rational, ten_exponent: u64) -> Self {
        if value.is_zero() {
            return Self {
                numerator: Integer::zero(),
                cofactor: Integer::one(),
                twos: 0,
                fives: 0,
            };
        }
        // Cancel the common twos and fives of the numerator and `10^e`; a
        // reduced denominator shares no factor with the numerator.
        let (numerator, cancelled_twos) = value.numerator().split_factor_two(ten_exponent);
        let (numerator, cancelled_fives) = split_factor_five(&numerator, ten_exponent);
        let (cofactor, denominator_twos) = value.denominator().split_factor_two(u64::MAX);
        let (cofactor, denominator_fives) = split_factor_five(&cofactor, u64::MAX);
        // Each count is an in-memory bit length or a `u64` scale, so the sums
        // stay far below `u64::MAX`.
        Self {
            numerator,
            cofactor,
            twos: denominator_twos + (ten_exponent - cancelled_twos),
            fives: denominator_fives + (ten_exponent - cancelled_fives),
        }
    }
}

/// `(value / 5^k, k)` for the greatest `k <= limit` with `5^k | value`, for a
/// nonzero `value`, in `O(log k)` divisions.
///
/// The ascending pass divides out `5^1, 5^2, 5^4, ...` while each divides and
/// fits the limit, so it stops with the remaining multiplicity `m` and limit
/// `l` satisfying `min(m, l) < 2^(J+1)` for the last squared exponent `2^J`.
/// The descending pass then takes each `5^(2^j)`, `j = J..0`, exactly when
/// `2^j <= min(m, l)`, which strips the binary digits of `min(m, l)`.
fn split_factor_five(value: &Integer, limit: u64) -> (Integer, u64) {
    let mut remaining = value.clone();
    let mut count = 0_u64;
    // `(5^width, width)` with `width = 2^j`.
    let mut powers = vec![(Integer::from(5_i64), 1_u64)];
    loop {
        let (power, width) = powers.last().expect("the ladder starts with 5^1");
        if *width > limit - count {
            break;
        }
        let (quotient, remainder) = remaining.div_rem_truncating(power);
        if !remainder.is_zero() {
            break;
        }
        remaining = quotient;
        count += width;
        // `width <= multiplicity < bits(value)`, so doubling stays in `u64`.
        let next = (power.mul(power), width * 2);
        powers.push(next);
    }
    for (power, width) in powers.iter().rev() {
        if *width > limit - count {
            continue;
        }
        let (quotient, remainder) = remaining.div_rem_truncating(power);
        if remainder.is_zero() {
            remaining = quotient;
            count += width;
        }
    }
    (remaining, count)
}

impl DecimalLoss {
    /// Reduced exact numerator.
    pub fn exact_numerator(&self) -> &Integer {
        &self.exact.numerator
    }

    /// Reduced positive exact denominator. This unmetered accessor
    /// materializes the denominator, including any power of ten contributed by
    /// a large working scale.
    pub fn exact_denominator(&self) -> Integer {
        let fives = Integer::from(5_i64).pow(&Integer::from(self.exact.fives));
        self.exact
            .cofactor
            .mul(&fives)
            .shifted_left(self.exact.twos)
    }

    /// The exact mathematical value, materialized as for
    /// [`DecimalLoss::exact_denominator`].
    pub fn exact(&self) -> Rational {
        Rational::new(self.exact.numerator.clone(), self.exact_denominator())
            .expect("a loss denominator is a product of positive factors")
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
pub(crate) fn compare_shifted(value: &Integer, shift: u64, bound: &Integer) -> Ordering {
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
    let count = length_amount(inputs.len());
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

    let intermediate = match plan {
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
            Intermediate::Scaled { value, scale }
        }
        Plan::Divide {
            numerator,
            denominator,
        } => Intermediate::Quotient(
            Rational::new(expand_one(numerator), expand_one(denominator))
                .map_err(|_| Stop::Undefined(Undefined::DivisionByZero))?,
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
            Intermediate::Scaled { value, scale }
        }
    };
    let (bits, digits) = intermediate.sizes();
    meter.charge(
        Charge::new(ChargePoint::DecimalArithmetic)
            .size(LimitKind::IntegerBits, bits)
            .size(LimitKind::DecimalDigits, digits),
    )?;

    let placed = match in_target_units(&intermediate, target.max_scale()) {
        TargetUnits::Exact { coefficient, scale } => Placed {
            coefficient,
            scale,
            loss: None,
        },
        TargetUnits::RoundingStep(units) => {
            let mode = target.rounding();
            // Strict `exact` refuses here, before any charge or loss record.
            let rounded = round(&units, mode).ok_or(Stop::Refused(Refusal::InexactDecimal))?;
            meter.charge(
                Charge::new(ChargePoint::DecimalRounding)
                    .size(LimitKind::IntegerBits, rounded.magnitude_bits())
                    .size(LimitKind::DecimalDigits, rounded.decimal_digits()),
            )?;
            Placed {
                loss: Some(DecimalLoss {
                    exact: intermediate.exact_loss_value(target.max_scale()),
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
    /// Decide how `value` is placed at `T` under the target rounding mode and
    /// the exact retained sizes of that placement, without materializing a
    /// coefficient larger than the inputs. Strict `exact` refuses a nonzero
    /// discarded digit.
    pub(crate) fn placement(&self, value: &Rational) -> Result<Placement, Refusal> {
        let (numerator, denominator) = (value.numerator(), value.denominator());
        let target_scale = u64::from(self.max_scale);
        let terminating = terminating_scale(denominator)
            .and_then(|scale| u32::try_from(scale).ok())
            .filter(|scale| *scale <= self.max_scale);
        if let Some(scale) = terminating {
            // `n/d = n × (10^k / d) × 10^-k` with `k <= bits(d)`.
            let factor = Integer::power_of_ten(u64::from(scale)).exact_div(denominator);
            let placed = Placed {
                coefficient: numerator.mul(&factor),
                scale,
                loss: None,
            };
            let (bits, digits) = placed.retained_sizes(self);
            return Ok(Placement {
                bits,
                digits: Integer::from(digits),
                kind: PlacementKind::Placed(placed),
            });
        }
        // A reduced value that is not a multiple of `10^-T` always discards a
        // nonzero digit.
        if self.rounding == RoundingMode::Exact {
            return Err(Refusal::InexactDecimal);
        }
        if target_scale
            <= ANALYTIC_PLACEMENT_BITS_FACTOR.saturating_mul(denominator.magnitude_bits())
        {
            // `n × 10^T` has at most `bits(n) + 7 × bits(d) + 1` bits here.
            let placed = self.round_at_target(value)?;
            let (bits, digits) = placed.retained_sizes(self);
            return Ok(Placement {
                bits,
                digits: Integer::from(digits),
                kind: PlacementKind::Placed(placed),
            });
        }
        let (bits, digits) = rounded_sizes(&numerator.abs(), denominator, target_scale);
        Ok(Placement {
            bits,
            digits,
            kind: PlacementKind::Deferred(value.clone()),
        })
    }

    /// Round `value × 10^T` to an integer coefficient at `T`, recording the
    /// loss. The caller has charged or bounded its size.
    fn round_at_target(&self, value: &Rational) -> Result<Placed, Refusal> {
        let (numerator, denominator) = (value.numerator(), value.denominator());
        let units = Rational::from_integer(
            numerator.mul(&Integer::power_of_ten(u64::from(self.max_scale))),
        )
        // A reduced denominator is positive, so the division exists.
        .div(&Rational::from_integer(denominator.clone()))
        .ok_or(Refusal::InexactDecimal)?;
        let rounded = round(&units, self.rounding).ok_or(Refusal::InexactDecimal)?;
        Ok(Placed {
            loss: Some(DecimalLoss {
                exact: ExactLossValue::scaled(value, 0),
                rounded: DecimalRepresentation::new(rounded.clone(), self.max_scale),
                mode: self.rounding,
            }),
            coefficient: rounded,
            scale: self.max_scale,
        })
    }
}

/// A rounded placement is sized analytically once `T > 2 × bits(d)`; below
/// that, `n × 10^T` is bounded by the inputs and is materialized. The two
/// routes agree; the bound only avoids allocating a coefficient before its
/// charge.
const ANALYTIC_PLACEMENT_BITS_FACTOR: u64 = 2;

/// A decided placement at `T` with its exact retained `(integer_bits,
/// decimal_digits)`.
pub(crate) struct Placement {
    bits: Integer,
    digits: Integer,
    kind: PlacementKind,
}

enum PlacementKind {
    /// A coefficient already bounded by the inputs.
    Placed(Placed),
    /// A rounded coefficient whose materialization waits for its charge.
    Deferred(Rational),
}

impl Placement {
    /// Exact retained `(integer_bits, decimal_digits)` of `v × 10^T`.
    pub(crate) fn retained_sizes(&self) -> (&Integer, &Integer) {
        (&self.bits, &self.digits)
    }

    /// Materialize the placed coefficient after its sizes were charged.
    pub(crate) fn materialize(self, target: &DecimalType) -> Result<Placed, Refusal> {
        match self.kind {
            PlacementKind::Placed(placed) => Ok(placed),
            PlacementKind::Deferred(value) => target.round_at_target(&value),
        }
    }
}

/// Exact `(bits, digits)` of `round(a × 10^T / d)` for `a >= 1`, `d > 1`,
/// `d ∤ a × 10^T` and `T > 2 × bits(d)`, without the power.
///
/// Let `M = a × 10^T` and `q = floor(M / d)`; `r = M mod d` is nonzero, so the
/// rounded magnitude is `q` or `q + 1` and `M` never equals `d × 2^j` or
/// `d × 10^j`.
///
/// Digits: `q >= 10^j` exactly when `a × 10^(T-j) > d`, so with `t0` the least
/// integer where `a × 10^t0 > d`, `digits(q) = T - t0 + 1`. Bits: with
/// `B = bits(M)`, `q >= 2^j` holds for every `j < B - bits(d)` and for no
/// `j > B - bits(d)`, so `bits(q) = B - bits(d) + [M > d × 2^(B - bits(d))]`.
///
/// `q + 1` is neither a power of ten nor of two: `M + d - r = d × 10^k` or
/// `d × 2^k` with `k >= bits(q) - 1 > bits(d)` would give
/// `v2(d - r) >= min(T, k) >= bits(d)`, but `0 < d - r < d`. So `q + 1` has
/// the sizes of `q` and the rounding direction does not change them.
fn rounded_sizes(
    magnitude: &Integer,
    denominator: &Integer,
    target_scale: u64,
) -> (Integer, Integer) {
    let ten = Integer::from(10_i64);
    let least_scale = if magnitude > denominator {
        // `t0 = -k` for the greatest `k` with `a > d × 10^k`.
        let mut lifted = denominator.mul(&ten);
        let mut k = Integer::zero();
        while magnitude > &lifted {
            lifted = lifted.mul(&ten);
            k = k.add(&Integer::one());
        }
        k.neg()
    } else {
        let mut scaled = magnitude.mul(&ten);
        let mut t = Integer::one();
        while &scaled <= denominator {
            scaled = scaled.mul(&ten);
            t = t.add(&Integer::one());
        }
        t
    };
    let target_scale = Integer::from(target_scale);
    let digits = target_scale.add(&Integer::one()).sub(&least_scale);
    let power_bits = Integer::power_product_bits(magnitude, &ten, &target_scale);
    let candidate = power_bits.sub(&Integer::from(denominator.magnitude_bits()));
    let above =
        Integer::compare_power_product(magnitude, &ten, &target_scale, denominator, &candidate)
            .is_gt();
    let bits = if above {
        candidate.add(&Integer::one())
    } else {
        candidate
    };
    (bits, digits)
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

    /// The scale-zero coefficient and loss record of an integer target.
    pub(crate) fn into_integer(self) -> (Integer, Option<DecimalLoss>) {
        (self.coefficient, self.loss)
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

/// The exact `decimal.arithmetic` intermediate.
enum Intermediate {
    /// The integer `value` in units of `10^-scale`.
    Scaled { value: Integer, scale: u64 },
    /// The reduced FR-140 `N/D`, already in units of `10^-T`.
    Quotient(Rational),
}

impl Intermediate {
    /// `(maxparts, max(digits(numerator), digits(denominator)))`. A scaled
    /// integer's denominator `1` never exceeds its numerator's sizes.
    fn sizes(&self) -> (u64, u64) {
        match self {
            Self::Scaled { value, .. } => (value.magnitude_bits(), value.decimal_digits()),
            Self::Quotient(value) => (
                value.max_part_bits(),
                value
                    .numerator()
                    .decimal_digits()
                    .max(value.denominator().decimal_digits()),
            ),
        }
    }

    /// The loss record's exact value, keeping the working-scale power of ten
    /// factored.
    fn exact_loss_value(&self, target_scale: u32) -> ExactLossValue {
        match self {
            Self::Scaled { value, scale } => {
                ExactLossValue::scaled(&Rational::from_integer(value.clone()), *scale)
            }
            Self::Quotient(value) => ExactLossValue::scaled(value, u64::from(target_scale)),
        }
    }
}

/// Whether the exact result is an integer multiple of `10^-T`.
enum TargetUnits {
    /// The exact value `coefficient × 10^-scale` with `scale <= T`.
    Exact { coefficient: Integer, scale: u32 },
    /// A rounding step over a non-integer value in units of `10^-T`, or over a
    /// proxy that every rounding mode rounds to the same integer.
    RoundingStep(Rational),
}

/// Place `intermediate` into units of `10^-T` without materializing a power of
/// ten larger than the charged intermediate.
///
/// For a scaled `v` with `k = scale - T > 0`, `v` is divided by `10^k` only
/// when `k <= digits(v)`, where the power is at most `10 × |v|`. Otherwise
/// `0 < |v| < 10^(k-1)`, so `v / 10^k` is a non-integer strictly inside
/// `(-1/10, 1/10)`; `sign(v) / 10` lies in the same interval on the same side
/// of zero, and every rounding mode rounds both to the same integer.
fn in_target_units(intermediate: &Intermediate, target_scale: u32) -> TargetUnits {
    let (value, scale) = match intermediate {
        Intermediate::Quotient(quotient) if quotient.is_integer() => {
            return TargetUnits::Exact {
                coefficient: quotient.numerator().clone(),
                scale: target_scale,
            }
        }
        Intermediate::Quotient(quotient) => return TargetUnits::RoundingStep(quotient.clone()),
        Intermediate::Scaled { value, scale } => (value, *scale),
    };
    let excess = match u32::try_from(scale) {
        Ok(scale) if scale <= target_scale => {
            return TargetUnits::Exact {
                coefficient: value.clone(),
                scale,
            }
        }
        _ => scale - u64::from(target_scale),
    };
    if value.is_zero() {
        return TargetUnits::Exact {
            coefficient: Integer::zero(),
            scale: target_scale,
        };
    }
    if excess > value.decimal_digits() {
        let sign = if value.is_negative() { -1_i64 } else { 1 };
        let proxy = Rational::new(Integer::from(sign), Integer::from(10_i64))
            .expect("ten is a nonzero denominator");
        return TargetUnits::RoundingStep(proxy);
    }
    let power = Integer::power_of_ten(excess);
    let (quotient, remainder) = value.div_rem_truncating(&power);
    if remainder.is_zero() {
        TargetUnits::Exact {
            coefficient: quotient,
            scale: target_scale,
        }
    } else {
        TargetUnits::RoundingStep(
            Rational::new(value.clone(), power).expect("a power of ten is nonzero"),
        )
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
pub(crate) fn shifted_bits(value: &Integer, shift: u64) -> Integer {
    Integer::power_product_bits(value, &Integer::from(10_i64), &Integer::from(shift))
}

/// `digits(c × 10^shift)`, derived without allocating the power of ten.
pub(crate) fn shifted_digits(value: &Integer, shift: u64) -> u64 {
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
