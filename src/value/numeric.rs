// SPDX-License-Identifier: AGPL-3.0-or-later
//! Metered exact numeric kernels: `Integer`/`Int[..]` arithmetic,
//! `Rational[..]` arithmetic, numeric ordering and Boolean connectives, in the
//! `quire.value.accounting/v1` charge order.
//!
//! Every size amount is derived before the value it measures is retained; no
//! power of ten is allocated to measure an aligned decimal coefficient.

use std::cmp::Ordering;

use super::accounting::{Charge, ChargePoint, LimitKind, Meter};
use super::decimal::{sbits, sdigits, Decimal};
use super::integer::{Integer, IntegerInterval};
use super::outcome::{Outcome, Refusal, Stop, Undefined};
use super::rational::{Rational, RationalDomain};

/// A binary `+`, `-`, `*` or `/` before its operand values exist.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum ArithmeticOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

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
            meter.charge(
                Charge::new(ChargePoint::OrderingOperands)
                    .size(
                        LimitKind::IntegerBits,
                        left.magnitude_bits().max(right.magnitude_bits()),
                    )
                    .size(occurrences, 2),
            )?;
            meter.charge(
                Charge::new(ChargePoint::OrderingArithmetic)
                    .exact_size(LimitKind::IntegerBits, integer_ordering_bits(left, right)),
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
            meter.charge(
                Charge::new(ChargePoint::OrderingArithmetic)
                    .exact_size(LimitKind::IntegerBits, rational_ordering_bits(left, right)),
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
                        sbits(left_coefficient, left_shift)
                            .max(sbits(right_coefficient, right_shift)),
                    )
                    .exact_size(
                        LimitKind::DecimalDigits,
                        sdigits(left_coefficient, left_shift)
                            .max(sdigits(right_coefficient, right_shift)),
                    ),
            )?;
            left.compare(right)
        }
    };
    meter.charge(Charge::new(ChargePoint::OrderingResultRetain).results(1))?;
    Ok(operator.holds(ordering))
}

// Arithmetic charge amounts: one function per charge point, so an amount rule
// changes in exactly one place. Every amount derives from operand sizes.

/// `bits(n)` as an unbounded amount.
fn bits(value: &Integer) -> Integer {
    Integer::from(value.magnitude_bits())
}

/// The `integer_bits` amount of `ordering.arithmetic` for integers.
fn integer_ordering_bits(left: &Integer, right: &Integer) -> Integer {
    bits(left).max(bits(right))
}

/// The `integer_bits` amount of `ordering.arithmetic` for `a/b` and `c/d`:
/// `max(bits(a)+bits(d), bits(c)+bits(b))`.
fn rational_ordering_bits(left: &Rational, right: &Rational) -> Integer {
    cross_bits(left, right).max(cross_bits(right, left))
}

/// `bits(a)+bits(d)` for `a/b` and `c/d`.
fn cross_bits(left: &Rational, right: &Rational) -> Integer {
    bits(left.numerator()).add(&bits(right.denominator()))
}

/// The `integer_bits` amount of `integer-arithmetic.arithmetic`:
/// `bits(a)+bits(b)` for `*`, `max(bits(a),bits(b))+1` for `+` and `-`, and
/// `bits(a)` for unary `-`.
fn integer_arithmetic_bits(operation: IntegerArithmetic<'_>) -> Integer {
    match operation {
        IntegerArithmetic::Add(left, right) | IntegerArithmetic::Subtract(left, right) => {
            bits(left).max(bits(right)).add(&Integer::one())
        }
        IntegerArithmetic::Multiply(left, right) => bits(left).add(&bits(right)),
        IntegerArithmetic::Negate(operand) => bits(operand),
    }
}

/// The `integer_bits` amount of `rational-arithmetic.arithmetic` for `a/b`
/// and `c/d`: `max(N,D)`, with `N` and `D` bounding the unreduced parts, and
/// `max(bits(a),bits(b))` for unary `-`. `unit.rational-arithmetic` reuses it.
pub(crate) fn rational_arithmetic_bits(operation: RationalArithmetic<'_>) -> Integer {
    let (numerator, denominator) = match operation {
        RationalArithmetic::Multiply(left, right) => (
            bits(left.numerator()).add(&bits(right.numerator())),
            bits(left.denominator()).add(&bits(right.denominator())),
        ),
        RationalArithmetic::Divide(left, right) => (
            cross_bits(left, right),
            bits(left.denominator()).add(&bits(right.numerator())),
        ),
        RationalArithmetic::Add(left, right) | RationalArithmetic::Subtract(left, right) => (
            rational_ordering_bits(left, right).add(&Integer::one()),
            bits(left.denominator()).add(&bits(right.denominator())),
        ),
        RationalArithmetic::Negate(operand) => {
            (bits(operand.numerator()), bits(operand.denominator()))
        }
    };
    numerator.max(denominator)
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
    meter.charge(
        Charge::new(ChargePoint::IntegerArithmeticArithmetic)
            .exact_size(LimitKind::IntegerBits, integer_arithmetic_bits(operation)),
    )?;
    let result = match operation {
        IntegerArithmetic::Add(left, right) => left.add(right),
        IntegerArithmetic::Subtract(left, right) => left.sub(right),
        IntegerArithmetic::Negate(operand) => operand.neg(),
        IntegerArithmetic::Multiply(left, right) => left.mul(right),
    };
    if bound.is_some_and(|bound| !bound.contains(&result)) {
        return Err(Stop::Refused(Refusal::IntegerOutOfDomain));
    }
    meter.charge(Charge::new(ChargePoint::IntegerArithmeticResultRetain).results(1))?;
    Ok(result)
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
/// divisor as undefined, `rational-arithmetic.arithmetic` from the operands,
/// `rational-arithmetic.normalize` on the unreduced intermediate, the
/// uncharged membership of
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
    meter.charge(
        Charge::new(ChargePoint::RationalArithmeticArithmetic)
            .exact_size(LimitKind::IntegerBits, rational_arithmetic_bits(operation)),
    )?;
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
    meter.charge(Charge::new(ChargePoint::RationalArithmeticNormalize).size(
        LimitKind::IntegerBits,
        numerator.magnitude_bits().max(denominator.magnitude_bits()),
    ))?;
    let result = Rational::new(numerator, denominator)
        .map_err(|_| Stop::Undefined(Undefined::DivisionByZero))?;
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
