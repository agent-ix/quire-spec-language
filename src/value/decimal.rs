// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-140 exact decimals: coefficient/scale values, the six rounding spellings,
//! canonical-rational loss records and metered evaluation.
//!
//! Every intermediate is an exact integer or reduced rational; no binary
//! floating-point value exists anywhere on this path.

use std::cmp::Ordering;

use super::accounting::{Charge, ChargePoint, LimitKind, Meter};
use super::integer::{Integer, IntegerInterval};
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

/// The finite target of a decimal operation: the scale results are rounded to,
/// the rounding spelling and an optional inclusive normalized-coefficient
/// domain.
// SPEC-GAP(1): `shared-grammar.md:275` gives `Decimal[lo,hi; uint,uint; mode]`
// but does not define the second `uint` pair. This target models an explicit
// result scale plus a coefficient interval; `admits_coefficient` is the single
// domain decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecimalTarget {
    scale: u32,
    rounding: RoundingMode,
    coefficient: Option<IntegerInterval>,
}

impl DecimalTarget {
    /// A target with an unbounded coefficient.
    pub fn new(scale: u32, rounding: RoundingMode) -> Self {
        Self {
            scale,
            rounding,
            coefficient: None,
        }
    }

    /// Bound the normalized coefficient to `domain`.
    pub fn with_coefficient_domain(mut self, domain: IntegerInterval) -> Self {
        self.coefficient = Some(domain);
        self
    }

    /// Maximum result scale.
    pub fn scale(&self) -> u32 {
        self.scale
    }

    /// Selected rounding spelling.
    pub fn rounding(&self) -> RoundingMode {
        self.rounding
    }

    /// Inclusive normalized-coefficient domain, if bounded.
    pub fn coefficient_domain(&self) -> Option<&IntegerInterval> {
        self.coefficient.as_ref()
    }

    /// SPEC-GAP(1): domain membership is decided on the normalized
    /// coefficient only.
    fn admits_coefficient(&self, value: &Decimal) -> bool {
        self.coefficient
            .as_ref()
            .is_none_or(|domain| domain.contains(value.normalized().coefficient()))
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
    /// Admit `a` into the target, rounding if required.
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
    /// The result, retaining its pre-normalized rounded representation.
    pub fn value(&self) -> &Decimal {
        &self.value
    }

    /// The loss record when a nonzero digit was discarded.
    pub fn loss(&self) -> Option<&DecimalLoss> {
        self.loss.as_ref()
    }
}

/// Evaluate `operation` into `target` under `quire.value.accounting/v1`.
pub fn evaluate_decimal(
    operation: DecimalOperation<'_>,
    target: &DecimalTarget,
    meter: &mut Meter,
) -> Outcome<DecimalResult> {
    Outcome::from_stop(evaluate(operation, target, meter))
}

/// How the exact intermediate combines its expanded coefficients.
#[derive(Clone, Copy)]
enum Combine {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// One expanded coefficient: the operand coefficient and its left shift.
type Shifted<'a> = (&'a Integer, u64);

/// The shape of an operation before any power of ten is allocated.
enum Plan<'a> {
    Binary {
        combine: Combine,
        left: Shifted<'a>,
        right: Shifted<'a>,
    },
    Unary {
        negate: bool,
        operand: &'a Integer,
    },
}

/// The exact intermediate `numerator/denominator` in coefficient units of
/// `scale`.
struct Working {
    numerator: Integer,
    denominator: Integer,
    scale: u32,
}

fn evaluate(
    operation: DecimalOperation<'_>,
    target: &DecimalTarget,
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
    // SPEC-GAP(5): the definition does not place the zero-divisor decision
    // among the named charges; it is taken after `decimal.operands`.
    reject_zero_divisor(operation)?;

    let working = expand(operation, target.scale(), meter)?;
    let exact = Rational::new(working.numerator, working.denominator)
        .map_err(|_| Stop::Undefined(Undefined::DivisionByZero))?;
    meter.charge(
        Charge::new(ChargePoint::DecimalArithmetic)
            .size(LimitKind::IntegerBits, exact.max_part_bits())
            .size(
                LimitKind::DecimalDigits,
                exact
                    .numerator()
                    .decimal_digits()
                    .max(exact.denominator().decimal_digits()),
            ),
    )?;

    let (coefficient, loss) = if !rounding_step_occurs(&exact) {
        (exact.numerator().clone(), None)
    } else {
        let mode = target.rounding();
        let rounded = round(&exact, mode).ok_or(Stop::Refused(Refusal::InexactDecimal))?;
        meter.charge(
            Charge::new(ChargePoint::DecimalRounding)
                .size(LimitKind::IntegerBits, rounded.magnitude_bits())
                .size(LimitKind::DecimalDigits, rounded.decimal_digits()),
        )?;
        let loss = DecimalLoss {
            exact: exact.divided_by_power_of_ten(u64::from(working.scale)),
            rounded: DecimalRepresentation::new(rounded.clone(), working.scale),
            mode,
        };
        (rounded, Some(loss))
    };

    let value = Decimal::new(coefficient, working.scale);
    if !target.admits_coefficient(&value) {
        return Err(Stop::Refused(Refusal::DecimalOutOfDomain));
    }
    let retained = value.representation().coefficient();
    meter.charge(
        Charge::new(ChargePoint::DecimalResultRetain)
            .size(LimitKind::IntegerBits, retained.magnitude_bits())
            .size(LimitKind::DecimalDigits, retained.decimal_digits())
            .size(LimitKind::ValueOccurrences, 1)
            .results(1),
    )?;
    Ok(DecimalResult { value, loss })
}

fn reject_zero_divisor(operation: DecimalOperation<'_>) -> Result<(), Stop> {
    if let DecimalOperation::Divide(_, divisor) = operation {
        if divisor.normalized().coefficient().is_zero() {
            return Err(Stop::Undefined(Undefined::DivisionByZero));
        }
    }
    Ok(())
}

// SPEC-GAP(5): "omitted when no rounding step occurs" does not say whether a
// target-scale admission that discards only zero digits is a rounding step.
// A step occurs only when the exact intermediate is not an integer in target
// units, i.e. a nonzero digit would be discarded.
fn rounding_step_occurs(exact: &Rational) -> bool {
    !exact.is_integer()
}

/// Plan the operation and its working scale without allocating a power of ten.
fn plan(operation: DecimalOperation<'_>, target_scale: u64) -> (Plan<'_>, u64) {
    let binary = |combine, left, right| Plan::Binary {
        combine,
        left,
        right,
    };
    match operation {
        DecimalOperation::Add(a, b) | DecimalOperation::Subtract(a, b) => {
            let (a, b) = (a.representation(), b.representation());
            let w = u64::from(a.scale().max(b.scale()));
            let combine = if matches!(operation, DecimalOperation::Add(..)) {
                Combine::Add
            } else {
                Combine::Subtract
            };
            (
                binary(
                    combine,
                    (a.coefficient(), w - u64::from(a.scale())),
                    (b.coefficient(), w - u64::from(b.scale())),
                ),
                w,
            )
        }
        DecimalOperation::Multiply(a, b) => {
            let (a, b) = (a.representation(), b.representation());
            (
                binary(
                    Combine::Multiply,
                    (a.coefficient(), 0),
                    (b.coefficient(), 0),
                ),
                u64::from(a.scale()) + u64::from(b.scale()),
            )
        }
        DecimalOperation::Divide(a, b) => {
            // a/b in target units: c_a * 10^(t + s_b - s_a) / c_b.
            let (a, b) = (a.representation(), b.representation());
            let up = target_scale + u64::from(b.scale());
            let down = u64::from(a.scale());
            (
                binary(
                    Combine::Divide,
                    (a.coefficient(), up.saturating_sub(down)),
                    (b.coefficient(), down.saturating_sub(up)),
                ),
                target_scale,
            )
        }
        DecimalOperation::Negate(a) => (
            Plan::Unary {
                negate: true,
                operand: a.representation().coefficient(),
            },
            u64::from(a.representation().scale()),
        ),
        DecimalOperation::Round(a) => (
            Plan::Unary {
                negate: false,
                operand: a.representation().coefficient(),
            },
            u64::from(a.representation().scale()),
        ),
    }
}

fn expand_one((coefficient, shift): Shifted<'_>) -> Integer {
    coefficient.mul(&Integer::power_of_ten(shift))
}

/// Align, combine and (for working scales above the target) express the exact
/// result in target coefficient units. `decimal.scale-expansion` is checked
/// before any power of ten is allocated and charged before combination.
fn expand(
    operation: DecimalOperation<'_>,
    target_scale: u32,
    meter: &mut Meter,
) -> Result<Working, Stop> {
    let t = u64::from(target_scale);
    let (plan, working_scale) = plan(operation, t);
    let downward = working_scale.saturating_sub(t);
    let shifted: Vec<Shifted<'_>> = match &plan {
        Plan::Binary { left, right, .. } => vec![*left, *right],
        Plan::Unary { operand, .. } => vec![(*operand, 0)],
    };
    let shift = max_of(shifted.iter().map(|(_, left)| *left)).max(downward);
    let digits = max_of(
        shifted
            .iter()
            .map(|(coefficient, left)| digits_after_shift(coefficient, *left)),
    );
    meter.precheck(
        ChargePoint::DecimalScaleExpansion,
        &[(LimitKind::ScaleExpansion, shift)],
    )?;
    let (numerator, denominator) = match plan {
        Plan::Binary {
            combine,
            left,
            right,
        } => {
            let (a, b) = (expand_one(left), expand_one(right));
            let bits = a.magnitude_bits().max(b.magnitude_bits());
            meter.charge(expansion_charge(shift, bits, digits))?;
            match combine {
                Combine::Add => (a.add(&b), Integer::one()),
                Combine::Subtract => (a.sub(&b), Integer::one()),
                Combine::Multiply => (a.mul(&b), Integer::one()),
                Combine::Divide => (a, b),
            }
        }
        Plan::Unary { negate, operand } => {
            let bits = operand.magnitude_bits();
            meter.charge(expansion_charge(shift, bits, digits))?;
            let value = if negate {
                operand.neg()
            } else {
                operand.clone()
            };
            (value, Integer::one())
        }
    };
    let (denominator, scale) = if downward > 0 {
        (
            denominator.mul(&Integer::power_of_ten(downward)),
            target_scale,
        )
    } else {
        // Without a downward shift the working scale is at most the target.
        (
            denominator,
            u32::try_from(working_scale).unwrap_or(target_scale),
        )
    };
    Ok(Working {
        numerator,
        denominator,
        scale,
    })
}

fn expansion_charge(shift: u64, bits: u64, digits: u64) -> Charge {
    Charge::new(ChargePoint::DecimalScaleExpansion)
        .size(LimitKind::ScaleExpansion, shift)
        .size(LimitKind::IntegerBits, bits)
        .size(LimitKind::DecimalDigits, digits)
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

/// `digits(c × 10^shift)`, derived without allocating the power of ten.
fn digits_after_shift(value: &Integer, shift: u64) -> u64 {
    if value.is_zero() {
        1
    } else {
        value.decimal_digits().saturating_add(shift)
    }
}
