// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142 quantities: exact arithmetic under normalized dimensions, explicit
//! affine conversion through the canonical root and the named `unit.*`
//! charges of `quire.value.accounting/v1`.

use std::cmp::Ordering;

use super::comparison::{ComparisonOperator, IllTyped, IllTypedCause};
use super::decimal::{
    sbits, sdigits, DecimalLoss, DecimalResult, DecimalType, Placed, RoundingMode,
};
use super::numeric::{rational_arithmetic_bits, RationalArithmetic};
use super::outcome::{Outcome, Refusal, Stop, Undefined};
use super::rational::{
    rational_add, rational_div, rational_mul, rational_neg, rational_pow, rational_sub, Rational,
};
use super::unit::{CompoundUnit, Dimension, Unit, UnitEdge};
use quire_exact::{
    BoundedInteger, Charge, ChargePoint, Integer, IntegerInterval, LimitKind, Meter,
};

/// The unit of a quantity: an admitted declared unit or a compound unit
/// produced by multiplication, division or power.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum QuantityUnit {
    /// An admitted I04 declared unit.
    Declared(Box<Unit>),
    /// An evaluator-owned `quire.value.compound-unit/v1` unit over canonical
    /// roots.
    Compound(CompoundUnit),
}

impl QuantityUnit {
    /// The normalized dimension map.
    pub fn dimension(&self) -> &Dimension {
        match self {
            Self::Declared(unit) => unit.dimension(),
            Self::Compound(unit) => unit.dimension(),
        }
    }

    /// Whether an operation requiring equal dimensions admits `self` and
    /// `other`: their normalized base-dimension maps are equal.
    pub(crate) fn has_dimension_of(&self, other: &Self) -> bool {
        self.dimension() == other.dimension()
    }

    /// Whether direct conversion from `self` into `target` is admitted. Two
    /// declared units need one dimension node, so equal base-dimension maps
    /// under distinct nodes (torque and energy) are not enough; a compound
    /// side compares base-dimension maps.
    pub(crate) fn converts_to(&self, target: &Self) -> bool {
        match (self, target) {
            (Self::Declared(source), Self::Declared(target)) => {
                source.dimension_node() == target.dimension_node()
            }
            _ => self.has_dimension_of(target),
        }
    }

    fn is_affine(&self) -> bool {
        matches!(self, Self::Declared(unit) if unit.is_affine())
    }

    /// Edges from this unit to its canonical root, in source-to-root order.
    fn path(&self) -> &[UnitEdge] {
        match self {
            Self::Declared(unit) => unit.path(),
            Self::Compound(_) => &[],
        }
    }

    /// The compound unit of this unit's canonical root.
    fn canonical_compound(&self) -> CompoundUnit {
        match self {
            Self::Declared(unit) => CompoundUnit::of_root(unit),
            Self::Compound(unit) => unit.clone(),
        }
    }
}

/// An exact rational value in a unit.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Quantity {
    value: Rational,
    unit: QuantityUnit,
}

impl Quantity {
    /// A quantity of exactly `value` in `unit`.
    pub fn new(value: Rational, unit: QuantityUnit) -> Self {
        Self { value, unit }
    }

    /// The exact value.
    pub fn value(&self) -> &Rational {
        &self.value
    }

    /// The unit.
    pub fn unit(&self) -> &QuantityUnit {
        &self.unit
    }
}

/// A quantity operation.
#[derive(Clone, Copy, Debug)]
pub enum QuantityOperation<'a> {
    /// `a + b` in one common unit.
    Add(&'a Quantity, &'a Quantity),
    /// `a - b` in one common unit.
    Subtract(&'a Quantity, &'a Quantity),
    /// `a * b` under the canonical compound unit.
    Multiply(&'a Quantity, &'a Quantity),
    /// `a / b` under the canonical compound unit.
    Divide(&'a Quantity, &'a Quantity),
    /// `a ^ n` for a mathematical integer `n`.
    Power(&'a Quantity, &'a Integer),
}

/// The value representation of an explicit conversion target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuantityTarget {
    /// An unbounded exact rational; conversion reports no loss.
    Exact,
    /// An FR-140 decimal type with its rounding and membership.
    Decimal(DecimalType),
    /// An integer domain, placed as a decimal target of scale zero under
    /// `rounding` and then admitted by integer-domain membership.
    Integer {
        /// The inclusive integer domain.
        domain: IntegerInterval,
        /// The rounding spelling of the scale-zero placement.
        rounding: RoundingMode,
    },
}

/// A converted value in its target representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConvertedValue {
    /// The exact converted rational.
    Exact(Rational),
    /// The FR-140 decimal result with any loss record.
    Decimal(DecimalResult),
    /// The admitted integer with any scale-zero loss record.
    Integer {
        /// The integer-domain member.
        value: BoundedInteger,
        /// The loss record when a rounding step occurred.
        loss: Option<DecimalLoss>,
    },
}

/// A completed explicit conversion with its provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Conversion {
    source: Quantity,
    canonical: Rational,
    value: ConvertedValue,
    unit: QuantityUnit,
}

impl Conversion {
    /// The original quantity.
    pub fn source(&self) -> &Quantity {
        &self.source
    }

    /// The exact value in the canonical root unit.
    pub fn canonical(&self) -> &Rational {
        &self.canonical
    }

    /// The converted value.
    pub fn value(&self) -> &ConvertedValue {
        &self.value
    }

    /// The target unit.
    pub fn unit(&self) -> &QuantityUnit {
        &self.unit
    }
}

/// Evaluate a quantity operation under `quire.value.accounting/v1`.
/// Incompatible dimensions, affine-unit arithmetic and distinct units are
/// ill-typed and consume nothing.
pub fn evaluate_quantity(
    operation: QuantityOperation<'_>,
    meter: &mut Meter,
) -> Result<Outcome<Quantity>, IllTyped> {
    type_check(operation)?;
    Ok(Outcome::from_stop(evaluate(operation, meter)))
}

/// Explicitly convert `source` into `unit` with the `target` representation.
/// Incompatible dimensions are ill-typed and consume nothing.
pub fn convert_quantity(
    source: &Quantity,
    unit: &QuantityUnit,
    target: &QuantityTarget,
    meter: &mut Meter,
) -> Result<Outcome<Conversion>, IllTyped> {
    if !source.unit.converts_to(unit) {
        return Err(ill_typed(IllTypedCause::IncompatibleDimensions));
    }
    let target = match target {
        QuantityTarget::Exact => Target::Exact,
        QuantityTarget::Decimal(decimal) => Target::Decimal(decimal.clone()),
        QuantityTarget::Integer { domain, rounding } => Target::Integer {
            placement: DecimalType::new(
                domain.lower().clone(),
                domain.upper().clone(),
                0,
                0,
                *rounding,
            )?,
            domain,
        },
    };
    Ok(Outcome::from_stop(convert(source, unit, &target, meter)))
}

/// Compare two quantities of the identical unit under
/// `quire.value.accounting/v1` by their exact values mapped to the canonical
/// root, so affine units and negative composed scales compare by root value.
/// Operands of different dimensions or units are ill-typed and consume
/// nothing.
pub fn compare_quantity(
    operator: ComparisonOperator,
    left: &Quantity,
    right: &Quantity,
    meter: &mut Meter,
) -> Result<Outcome<bool>, IllTyped> {
    check_comparable(&left.unit, &right.unit)?;
    Ok(Outcome::from_stop(
        compare(left, right, meter).map(|ordering| operator.holds(ordering)),
    ))
}

fn ill_typed(cause: IllTypedCause) -> IllTyped {
    IllTyped { cause }
}

/// One `unit.identity-read` per operand, each sized over the operands read so
/// far.
fn read_identities(operands: &[&Quantity], meter: &mut Meter) -> Result<(), Stop> {
    let mut bits = 0;
    for (read, operand) in (1_u64..).zip(operands) {
        bits = operand.value.max_part_bits().max(bits);
        meter.charge(
            Charge::new(ChargePoint::UnitIdentityRead)
                .size(LimitKind::ValueOccurrences, read)
                .size(LimitKind::IntegerBits, bits),
        )?;
    }
    Ok(())
}

/// One scheduled exact rational event, charged from its operands before the
/// result is computed.
fn rational_event(operation: RationalArithmetic<'_>, meter: &mut Meter) -> Result<Rational, Stop> {
    meter.charge(
        Charge::new(ChargePoint::UnitRationalArithmetic)
            .exact_size(LimitKind::IntegerBits, rational_arithmetic_bits(operation)),
    )?;
    Ok(match operation {
        RationalArithmetic::Add(left, right) => rational_add(left, right),
        RationalArithmetic::Subtract(left, right) => rational_sub(left, right),
        RationalArithmetic::Multiply(left, right) => rational_mul(left, right),
        RationalArithmetic::Divide(left, right) => {
            rational_div(left, right).ok_or(Stop::Undefined(Undefined::DivisionByZero))?
        }
        RationalArithmetic::Negate(operand) => rational_neg(operand),
    })
}

/// The direction an edge is traversed.
#[derive(Clone, Copy)]
enum Direction {
    /// `scale × v + offset`: multiply then add.
    Forward,
    /// `(v - offset) / scale`: subtract then divide.
    Reverse,
}

/// Charge one `unit.edge` per traversed edge, numbered across every root path
/// of the operation; every edge is charged before any rational event.
fn charge_edges(count: usize, meter: &mut Meter) -> Result<(), Stop> {
    for edge in (1_u64..).take(count) {
        meter.charge(Charge::new(ChargePoint::UnitEdge).size(LimitKind::UnitEdges, edge))?;
    }
    Ok(())
}

/// Forward edges from a unit to its canonical root.
fn to_root(unit: &QuantityUnit) -> Vec<(&UnitEdge, Direction)> {
    unit.path()
        .iter()
        .map(|edge| (edge, Direction::Forward))
        .collect()
}

/// `unit.target-domain` with no size amount for an unbounded exact rational
/// result, whose size its last rational event or identity read already
/// charged.
fn charge_rational_target(meter: &mut Meter) -> Result<(), Stop> {
    meter
        .charge(Charge::new(ChargePoint::UnitTargetDomain))
        .map_err(Stop::from)
}

/// `unit.result-retain` of one quantity: `occ(result) = 1`.
fn charge_retain(meter: &mut Meter) -> Result<(), Stop> {
    meter
        .charge(
            Charge::new(ChargePoint::UnitResultRetain)
                .size(LimitKind::ValueOccurrences, 1)
                .results(1),
        )
        .map_err(Stop::from)
}

/// The type-time refusals of an operation, in cause order: incompatible
/// dimensions, affine-unit arithmetic, distinct units.
fn type_check(operation: QuantityOperation<'_>) -> Result<(), IllTyped> {
    let (unit_operation, a, b) = match operation {
        QuantityOperation::Add(a, b) => (UnitOperation::Add, a, b),
        QuantityOperation::Subtract(a, b) => (UnitOperation::Subtract, a, b),
        QuantityOperation::Multiply(a, b) => (UnitOperation::Multiply, a, b),
        QuantityOperation::Divide(a, b) => (UnitOperation::Divide, a, b),
        QuantityOperation::Power(a, _) => {
            if a.unit.is_affine() {
                return Err(ill_typed(IllTypedCause::AffineUnitArithmetic));
            }
            return Ok(());
        }
    };
    result_unit(unit_operation, &a.unit, &b.unit).map(|_| ())
}

/// A binary quantity operation, before its operand values exist.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnitOperation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// The static result unit of `left op right`, or its type-time refusal in
/// cause order: incompatible dimensions, affine-unit arithmetic, distinct
/// units. Addition and subtraction keep the identical unit; multiplication
/// and division form the canonical compound unit.
pub(crate) fn result_unit(
    operation: UnitOperation,
    left: &QuantityUnit,
    right: &QuantityUnit,
) -> Result<QuantityUnit, IllTyped> {
    match operation {
        UnitOperation::Add | UnitOperation::Subtract => {
            if !left.has_dimension_of(right) {
                return Err(ill_typed(IllTypedCause::IncompatibleDimensions));
            }
            if left.is_affine() || right.is_affine() {
                return Err(ill_typed(IllTypedCause::AffineUnitArithmetic));
            }
            if left != right {
                return Err(ill_typed(IllTypedCause::DistinctUnits));
            }
            Ok(left.clone())
        }
        UnitOperation::Multiply | UnitOperation::Divide => {
            if left.is_affine() || right.is_affine() {
                return Err(ill_typed(IllTypedCause::AffineUnitArithmetic));
            }
            let (left, right) = (left.canonical_compound(), right.canonical_compound());
            Ok(QuantityUnit::Compound(
                if operation == UnitOperation::Multiply {
                    left.multiply(&right)
                } else {
                    left.divide(&right)
                },
            ))
        }
    }
}

/// Whether quantities in `left` and `right` may be compared: equal dimensions,
/// then the identical unit.
pub(crate) fn check_comparable(left: &QuantityUnit, right: &QuantityUnit) -> Result<(), IllTyped> {
    if !left.has_dimension_of(right) {
        return Err(ill_typed(IllTypedCause::IncompatibleDimensions));
    }
    if left != right {
        return Err(ill_typed(IllTypedCause::DistinctUnits));
    }
    Ok(())
}

/// The uncharged runtime conditions after the identity reads, first match
/// wins: a zero divisor, then a zero base under a negative exponent.
fn check_undefined(operation: QuantityOperation<'_>) -> Result<(), Stop> {
    let undefined = match operation {
        QuantityOperation::Divide(_, divisor) => divisor.value.is_zero(),
        QuantityOperation::Power(base, exponent) => base.value.is_zero() && exponent.is_negative(),
        QuantityOperation::Add(..)
        | QuantityOperation::Subtract(..)
        | QuantityOperation::Multiply(..) => false,
    };
    if undefined {
        return Err(Stop::Undefined(Undefined::DivisionByZero));
    }
    Ok(())
}

fn evaluate(operation: QuantityOperation<'_>, meter: &mut Meter) -> Result<Quantity, Stop> {
    let operands = match operation {
        QuantityOperation::Add(a, b)
        | QuantityOperation::Subtract(a, b)
        | QuantityOperation::Multiply(a, b)
        | QuantityOperation::Divide(a, b) => vec![a, b],
        QuantityOperation::Power(a, _) => vec![a],
    };
    read_identities(&operands, meter)?;
    check_undefined(operation)?;
    let result = match operation {
        QuantityOperation::Add(a, b) => Quantity {
            value: rational_event(RationalArithmetic::Add(&a.value, &b.value), meter)?,
            unit: a.unit.clone(),
        },
        QuantityOperation::Subtract(a, b) => Quantity {
            value: rational_event(RationalArithmetic::Subtract(&a.value, &b.value), meter)?,
            unit: a.unit.clone(),
        },
        QuantityOperation::Multiply(a, b) | QuantityOperation::Divide(a, b) => {
            let (left_edges, right_edges) = (to_root(&a.unit), to_root(&b.unit));
            charge_edges(left_edges.len() + right_edges.len(), meter)?;
            let left = events(&a.value, &left_edges, meter)?;
            let right = events(&b.value, &right_edges, meter)?;
            let (left_unit, right_unit) =
                (a.unit.canonical_compound(), b.unit.canonical_compound());
            if matches!(operation, QuantityOperation::Multiply(..)) {
                Quantity {
                    value: rational_event(RationalArithmetic::Multiply(&left, &right), meter)?,
                    unit: QuantityUnit::Compound(left_unit.multiply(&right_unit)),
                }
            } else {
                Quantity {
                    value: rational_event(RationalArithmetic::Divide(&left, &right), meter)?,
                    unit: QuantityUnit::Compound(left_unit.divide(&right_unit)),
                }
            }
        }
        QuantityOperation::Power(a, exponent) => {
            let edges = to_root(&a.unit);
            charge_edges(edges.len(), meter)?;
            let base = events(&a.value, &edges, meter)?;
            Quantity {
                value: power(&base, exponent, meter)?
                    .ok_or(Stop::Undefined(Undefined::DivisionByZero))?,
                unit: QuantityUnit::Compound(a.unit.canonical_compound().power(exponent)),
            }
        }
    };
    charge_rational_target(meter)?;
    charge_retain(meter)?;
    Ok(result)
}

/// The two rational events of each edge, in traversal order.
fn events(
    value: &Rational,
    edges: &[(&UnitEdge, Direction)],
    meter: &mut Meter,
) -> Result<Rational, Stop> {
    let mut current = value.clone();
    for (edge, direction) in edges {
        current = match direction {
            Direction::Forward => {
                let scaled =
                    rational_event(RationalArithmetic::Multiply(&current, edge.scale()), meter)?;
                rational_event(RationalArithmetic::Add(&scaled, edge.offset()), meter)?
            }
            Direction::Reverse => {
                let shifted =
                    rational_event(RationalArithmetic::Subtract(&current, edge.offset()), meter)?;
                // Admitted scales are nonzero.
                rational_event(RationalArithmetic::Divide(&shifted, edge.scale()), meter)?
            }
        };
    }
    Ok(current)
}

/// Charge the powered result's exact size before computing it. `0^0` is one;
/// `None` is a zero base under a negative exponent, which `check_undefined`
/// has already refused.
fn power(base: &Rational, exponent: &Integer, meter: &mut Meter) -> Result<Option<Rational>, Stop> {
    // `max(1, |n| × maxparts(x))` bounds both parts of `x^n`.
    let bits = exponent
        .abs()
        .mul(&Integer::from(base.max_part_bits()))
        .max(Integer::one());
    meter.charge(
        Charge::new(ChargePoint::UnitRationalArithmetic).exact_size(LimitKind::IntegerBits, bits),
    )?;
    Ok(rational_pow(base, exponent))
}

/// A conversion target resolved at type-check time.
enum Target<'a> {
    Exact,
    Decimal(DecimalType),
    Integer {
        placement: DecimalType,
        domain: &'a IntegerInterval,
    },
}

/// `source → root → target` in full, with no shortcut and no operation event.
fn convert(
    source: &Quantity,
    unit: &QuantityUnit,
    target: &Target<'_>,
    meter: &mut Meter,
) -> Result<Conversion, Stop> {
    read_identities(&[source], meter)?;
    let source_edges = to_root(&source.unit);
    let target_edges: Vec<_> = unit
        .path()
        .iter()
        .rev()
        .map(|edge| (edge, Direction::Reverse))
        .collect();
    charge_edges(source_edges.len() + target_edges.len(), meter)?;
    let canonical = events(&source.value, &source_edges, meter)?;
    let exact = events(&canonical, &target_edges, meter)?;
    let value = match target {
        Target::Exact => {
            charge_rational_target(meter)?;
            charge_retain(meter)?;
            ConvertedValue::Exact(exact)
        }
        Target::Decimal(decimal) => {
            let placed = place(&exact, decimal, Retained::Decimal, meter)?;
            placed.check_membership(decimal).map_err(Stop::Refused)?;
            charge_retain(meter)?;
            ConvertedValue::Decimal(placed.retain(decimal))
        }
        Target::Integer { placement, domain } => {
            let (coefficient, loss) =
                place(&exact, placement, Retained::Integer, meter)?.into_integer();
            let value = domain
                .admit(coefficient)
                .map_err(|_| Stop::Refused(Refusal::IntegerOutOfDomain))?;
            charge_retain(meter)?;
            ConvertedValue::Integer { value, loss }
        }
    };
    Ok(Conversion {
        source: source.clone(),
        canonical,
        value,
        unit: unit.clone(),
    })
}

/// The representation a placed coefficient is retained as, which selects the
/// size amounts of its `unit.target-domain` charge.
#[derive(Clone, Copy)]
enum Retained {
    /// A decimal: `integer_bits` and `decimal_digits` of the coefficient.
    Decimal,
    /// An integer: `integer_bits` of the rounded integer only.
    Integer,
}

/// Place `exact` at the decimal target's scale and charge
/// `unit.target-domain` with the analytically sized retained coefficient
/// before materializing it. Strict `exact` refuses before the charge.
fn place(
    exact: &Rational,
    decimal: &DecimalType,
    retained: Retained,
    meter: &mut Meter,
) -> Result<Placed, Stop> {
    let placement = decimal.placement(exact).map_err(Stop::Refused)?;
    // Sized from the reduced exact numerator `a`, before placement
    // materializes any coefficient.
    let numerator = exact.numerator();
    let charge = Charge::new(ChargePoint::UnitTargetDomain);
    meter.charge(match retained {
        Retained::Decimal => {
            let scale = u64::from(decimal.max_scale());
            charge
                .exact_size(LimitKind::IntegerBits, sbits(numerator, scale))
                .exact_size(LimitKind::DecimalDigits, sdigits(numerator, scale))
        }
        Retained::Integer => charge.size(LimitKind::IntegerBits, numerator.magnitude_bits()),
    })?;
    placement.materialize(decimal).map_err(Stop::Refused)
}

/// The top-level equality and ordering schedule: both root paths, left then
/// right, with no operation event and no `unit.target-domain`.
fn compare(left: &Quantity, right: &Quantity, meter: &mut Meter) -> Result<Ordering, Stop> {
    read_identities(&[left, right], meter)?;
    let (left_edges, right_edges) = (to_root(&left.unit), to_root(&right.unit));
    charge_edges(left_edges.len() + right_edges.len(), meter)?;
    let left = events(&left.value, &left_edges, meter)?;
    let right = events(&right.value, &right_edges, meter)?;
    charge_retain(meter)?;
    Ok(left.cmp(&right))
}
