// SPDX-License-Identifier: AGPL-3.0-or-later
//! Metered exact numeric kernels: `Integer`/`Int[..]` arithmetic,
//! `Rational[..]` arithmetic, numeric ordering and Boolean connectives, in the
//! `quire.value.accounting/v1` charge order.
//!
//! Every size amount is derived before the value it measures is retained; no
//! power of ten is allocated to measure an aligned decimal coefficient.

use std::cmp::Ordering;

use super::accounting::{Charge, ChargePoint, LimitKind, Meter};
use super::decimal::{shifted_bits, shifted_digits, Decimal};
use super::integer::{Integer, IntegerInterval};
use super::outcome::{Outcome, Refusal, Stop, Undefined};
use super::rational::{Rational, RationalDomain};

/// A numeric ordering operator. Equality has its own FR-149 schedule.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OrderingOperator {
    /// `<`.
    Less,
    /// `<=`.
    LessOrEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterOrEqual,
}

impl OrderingOperator {
    fn holds(self, ordering: Ordering) -> bool {
        match self {
            Self::Less => ordering.is_lt(),
            Self::LessOrEqual => ordering.is_le(),
            Self::Greater => ordering.is_gt(),
            Self::GreaterOrEqual => ordering.is_ge(),
        }
    }
}

/// The two operands of one numeric ordering, both of one exact kind.
#[derive(Clone, Copy, Debug)]
pub enum OrderedOperands<'a> {
    /// `Integer` or `Int[..]` operands.
    Integers(&'a Integer, &'a Integer),
    /// `Rational[..]` operands.
    Rationals(&'a Rational, &'a Rational),
    /// `Decimal[..]` operands in their retained representations.
    Decimals(&'a Decimal, &'a Decimal),
}

/// Order two exact numbers: `ordering.operands`, `ordering.arithmetic`, then
/// `ordering.result-retain`.
pub fn order_numbers(
    operator: OrderingOperator,
    operands: OrderedOperands<'_>,
    meter: &mut Meter,
) -> Outcome<bool> {
    Outcome::from_stop(order(operator, operands, meter))
}

fn order(
    operator: OrderingOperator,
    operands: OrderedOperands<'_>,
    meter: &mut Meter,
) -> Result<bool, Stop> {
    let occurrences = LimitKind::ValueOccurrences;
    let ordering = match operands {
        OrderedOperands::Integers(left, right) => {
            let bits = left.magnitude_bits().max(right.magnitude_bits());
            meter.charge(
                Charge::new(ChargePoint::OrderingOperands)
                    .size(LimitKind::IntegerBits, bits)
                    .size(occurrences, 2),
            )?;
            meter.charge(
                Charge::new(ChargePoint::OrderingArithmetic).size(LimitKind::IntegerBits, bits),
            )?;
            left.cmp(right)
        }
        OrderedOperands::Rationals(left, right) => {
            let bits = left.max_part_bits().max(right.max_part_bits());
            meter.charge(
                Charge::new(ChargePoint::OrderingOperands)
                    .size(LimitKind::IntegerBits, bits)
                    .size(occurrences, 2),
            )?;
            let cross = product_bits(left.numerator(), right.denominator())
                .max(product_bits(right.numerator(), left.denominator()));
            meter.charge(
                Charge::new(ChargePoint::OrderingArithmetic)
                    .exact_size(LimitKind::IntegerBits, cross),
            )?;
            left.cmp(right)
        }
        OrderedOperands::Decimals(left, right) => {
            let (left_repr, right_repr) = (left.representation(), right.representation());
            let (left_coefficient, right_coefficient) =
                (left_repr.coefficient(), right_repr.coefficient());
            meter.charge(
                Charge::new(ChargePoint::OrderingOperands)
                    .size(
                        LimitKind::IntegerBits,
                        left_coefficient
                            .magnitude_bits()
                            .max(right_coefficient.magnitude_bits()),
                    )
                    .size(
                        LimitKind::DecimalDigits,
                        left_coefficient
                            .decimal_digits()
                            .max(right_coefficient.decimal_digits()),
                    )
                    .size(occurrences, 2),
            )?;
            let (left_scale, right_scale) =
                (u64::from(left_repr.scale()), u64::from(right_repr.scale()));
            let aligned = left_scale.max(right_scale);
            let (left_shift, right_shift) = (aligned - left_scale, aligned - right_scale);
            meter.charge(
                Charge::new(ChargePoint::OrderingArithmetic)
                    .size(LimitKind::ScaleExpansion, left_shift.max(right_shift))
                    .exact_size(
                        LimitKind::IntegerBits,
                        shifted_bits(left_coefficient, left_shift)
                            .max(shifted_bits(right_coefficient, right_shift)),
                    )
                    .size(
                        LimitKind::DecimalDigits,
                        shifted_digits(left_coefficient, left_shift)
                            .max(shifted_digits(right_coefficient, right_shift)),
                    ),
            )?;
            left.compare(right)
        }
    };
    meter.charge(Charge::new(ChargePoint::OrderingResultRetain).results(1))?;
    Ok(operator.holds(ordering))
}

/// `bits(a × b)`, derived without materializing the product.
fn product_bits(left: &Integer, right: &Integer) -> Integer {
    Integer::power_product_bits(left, right, &Integer::one())
}

/// One `Integer` or `Int[..]` arithmetic operation.
#[derive(Clone, Copy, Debug)]
pub enum IntegerArithmetic<'a> {
    /// `a + b`.
    Add(&'a Integer, &'a Integer),
    /// `a - b`.
    Subtract(&'a Integer, &'a Integer),
    /// `a * b`.
    Multiply(&'a Integer, &'a Integer),
    /// `-a`.
    Negate(&'a Integer),
}

/// Evaluate integer arithmetic: `integer-arithmetic.operands`,
/// `integer-arithmetic.arithmetic`, the uncharged membership of an optional
/// FR-044 result bound, then `integer-arithmetic.result-retain`.
pub fn evaluate_integer_arithmetic(
    operation: IntegerArithmetic<'_>,
    bound: Option<&IntegerInterval>,
    meter: &mut Meter,
) -> Outcome<Integer> {
    Outcome::from_stop(integer_arithmetic(operation, bound, meter))
}

fn integer_arithmetic(
    operation: IntegerArithmetic<'_>,
    bound: Option<&IntegerInterval>,
    meter: &mut Meter,
) -> Result<Integer, Stop> {
    let (bits, count) = match operation {
        IntegerArithmetic::Add(left, right)
        | IntegerArithmetic::Subtract(left, right)
        | IntegerArithmetic::Multiply(left, right) => {
            (left.magnitude_bits().max(right.magnitude_bits()), 2)
        }
        IntegerArithmetic::Negate(operand) => (operand.magnitude_bits(), 1),
    };
    meter.charge(
        Charge::new(ChargePoint::IntegerArithmeticOperands)
            .size(LimitKind::IntegerBits, bits)
            .size(LimitKind::ValueOccurrences, count),
    )?;
    // A sum or difference is at most one bit wider than its admitted operands,
    // so it is computed to be measured; a product is measured analytically.
    let (result_bits, result) = match operation {
        IntegerArithmetic::Add(left, right) => measured(left.add(right)),
        IntegerArithmetic::Subtract(left, right) => measured(left.sub(right)),
        IntegerArithmetic::Negate(operand) => measured(operand.neg()),
        IntegerArithmetic::Multiply(left, right) => {
            (product_bits(left, right), Deferred::Product(left, right))
        }
    };
    meter.charge(
        Charge::new(ChargePoint::IntegerArithmeticArithmetic)
            .exact_size(LimitKind::IntegerBits, result_bits),
    )?;
    let result = match result {
        Deferred::Computed(value) => value,
        Deferred::Product(left, right) => left.mul(right),
    };
    if bound.is_some_and(|bound| !bound.contains(&result)) {
        return Err(Stop::Refused(Refusal::IntegerOutOfDomain));
    }
    meter.charge(Charge::new(ChargePoint::IntegerArithmeticResultRetain).results(1))?;
    Ok(result)
}

enum Deferred<'a> {
    Computed(Integer),
    Product(&'a Integer, &'a Integer),
}

fn measured<'a>(value: Integer) -> (Integer, Deferred<'a>) {
    (
        Integer::from(value.magnitude_bits()),
        Deferred::Computed(value),
    )
}

/// One `Rational[..]` arithmetic operation. An `Integer` or `Int[..]` `/`
/// producing `Rational[..]` takes each operand `n` as `n/1`.
#[derive(Clone, Copy, Debug)]
pub enum RationalArithmetic<'a> {
    /// `a/b + c/d`.
    Add(&'a Rational, &'a Rational),
    /// `a/b - c/d`.
    Subtract(&'a Rational, &'a Rational),
    /// `a/b * c/d`.
    Multiply(&'a Rational, &'a Rational),
    /// `(a/b) / (c/d)`.
    Divide(&'a Rational, &'a Rational),
    /// `-(a/b)`.
    Negate(&'a Rational),
}

/// Evaluate rational arithmetic: `rational-arithmetic.operands`, a zero
/// divisor as undefined, `rational-arithmetic.arithmetic` on the unreduced
/// intermediate, `rational-arithmetic.normalize`, the uncharged membership of
/// an optional FR-044 result domain, then `rational-arithmetic.result-retain`.
pub fn evaluate_rational_arithmetic(
    operation: RationalArithmetic<'_>,
    domain: Option<&RationalDomain>,
    meter: &mut Meter,
) -> Outcome<Rational> {
    Outcome::from_stop(rational_arithmetic(operation, domain, meter))
}

fn rational_arithmetic(
    operation: RationalArithmetic<'_>,
    domain: Option<&RationalDomain>,
    meter: &mut Meter,
) -> Result<Rational, Stop> {
    let (bits, count) = match operation {
        RationalArithmetic::Add(left, right)
        | RationalArithmetic::Subtract(left, right)
        | RationalArithmetic::Multiply(left, right)
        | RationalArithmetic::Divide(left, right) => {
            (left.max_part_bits().max(right.max_part_bits()), 2)
        }
        RationalArithmetic::Negate(operand) => (operand.max_part_bits(), 1),
    };
    meter.charge(
        Charge::new(ChargePoint::RationalArithmeticOperands)
            .size(LimitKind::IntegerBits, bits)
            .size(LimitKind::ValueOccurrences, count),
    )?;
    if let RationalArithmetic::Divide(_, divisor) = operation {
        if divisor.is_zero() {
            return Err(Stop::Undefined(Undefined::DivisionByZero));
        }
    }
    // Products of admitted operands are at most twice their width, so the
    // unreduced intermediate is formed to be measured.
    let (numerator, denominator) = match operation {
        RationalArithmetic::Add(left, right) => (
            left.numerator()
                .mul(right.denominator())
                .add(&right.numerator().mul(left.denominator())),
            left.denominator().mul(right.denominator()),
        ),
        RationalArithmetic::Subtract(left, right) => (
            left.numerator()
                .mul(right.denominator())
                .sub(&right.numerator().mul(left.denominator())),
            left.denominator().mul(right.denominator()),
        ),
        RationalArithmetic::Multiply(left, right) => (
            left.numerator().mul(right.numerator()),
            left.denominator().mul(right.denominator()),
        ),
        RationalArithmetic::Divide(left, right) => (
            left.numerator().mul(right.denominator()),
            left.denominator().mul(right.numerator()),
        ),
        RationalArithmetic::Negate(operand) => {
            (operand.numerator().neg(), operand.denominator().clone())
        }
    };
    meter.charge(Charge::new(ChargePoint::RationalArithmeticArithmetic).size(
        LimitKind::IntegerBits,
        numerator.magnitude_bits().max(denominator.magnitude_bits()),
    ))?;
    let result = Rational::new(numerator, denominator)
        .map_err(|_| Stop::Undefined(Undefined::DivisionByZero))?;
    meter.charge(
        Charge::new(ChargePoint::RationalArithmeticNormalize)
            .size(LimitKind::IntegerBits, result.max_part_bits()),
    )?;
    if domain.is_some_and(|domain| !domain.contains(&result)) {
        return Err(Stop::Refused(Refusal::RationalOutOfDomain));
    }
    meter.charge(Charge::new(ChargePoint::RationalArithmeticResultRetain).results(1))?;
    Ok(result)
}

/// One Boolean connective over decided operands.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BooleanConnective {
    /// `a and b`.
    And(bool, bool),
    /// `a or b`.
    Or(bool, bool),
    /// `a implies b`.
    Implies(bool, bool),
    /// `not a`.
    Not(bool),
}

/// Decide a connective, then charge `boolean.result-retain`.
pub fn evaluate_boolean(connective: BooleanConnective, meter: &mut Meter) -> Outcome<bool> {
    let result = match connective {
        BooleanConnective::And(left, right) => left && right,
        BooleanConnective::Or(left, right) => left || right,
        BooleanConnective::Implies(left, right) => !left || right,
        BooleanConnective::Not(operand) => !operand,
    };
    Outcome::from_stop(retain_boolean(result, meter).map_err(Stop::from))
}

/// Charge `boolean.result-retain` for a decided connective result.
pub(crate) fn retain_boolean(
    result: bool,
    meter: &mut Meter,
) -> Result<bool, super::accounting::Incomplete> {
    meter.charge(Charge::new(ChargePoint::BooleanResultRetain).results(1))?;
    Ok(result)
}
