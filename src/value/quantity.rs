// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142 quantities: exact arithmetic under normalized dimensions, explicit
//! affine conversion through the canonical root and the named `unit.*`
//! charges of `quire.value.accounting/v1`.

use std::cmp::Ordering;

use super::accounting::{Charge, ChargePoint, LimitKind, Meter};
use super::comparison::{ComparisonOperator, IllTyped, IllTypedCause};
use super::decimal::{DecimalLoss, DecimalResult, DecimalType, Placed, RoundingMode};
use super::integer::{BoundedInteger, Integer, IntegerInterval};
use super::outcome::{Outcome, Refusal, Stop, Undefined};
use super::rational::Rational;
use super::unit::{CompoundUnit, Dimension, Unit, UnitEdge};

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

    /// Whether quantities in `self` and `other` share one dimension.
    // SPEC-GAP(14): FR-142 defines conversion only within one dimension
    // node's unit graph. Two declared units are compatible exactly when they
    // share one nominal dimension node; equal base-dimension maps are not
    // enough. Normalized base-dimension maps are compared only when a
    // compound unit is involved, which then composes through each side's
    // canonical root as if those roots were coherent.
    fn is_compatible(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Declared(left), Self::Declared(right)) => {
                left.dimension_node() == right.dimension_node()
            }
            _ => self.dimension() == other.dimension(),
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
    // SPEC-GAP(17): the integer-target ruling does not name a rounding
    // spelling. The target carries one, as a scale-zero decimal target does.
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
    if !source.unit.is_compatible(unit) {
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
    if !left.unit.is_compatible(&right.unit) {
        return Err(ill_typed(IllTypedCause::IncompatibleDimensions));
    }
    if left.unit != right.unit {
        return Err(ill_typed(IllTypedCause::DistinctUnits));
    }
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

/// One scheduled exact rational event.
fn rational_event(value: Rational, meter: &mut Meter) -> Result<Rational, Stop> {
    meter.charge(
        Charge::new(ChargePoint::UnitRationalArithmetic)
            .size(LimitKind::IntegerBits, value.max_part_bits()),
    )?;
    Ok(value)
}

/// The direction an edge is traversed.
#[derive(Clone, Copy)]
enum Direction {
    /// `scale × v + offset`: multiply then add.
    Forward,
    /// `(v - offset) / scale`: subtract then divide.
    Reverse,
}

/// Charge one `unit.edge` per traversed edge, numbered across every source
/// and target root path of the operation.
// SPEC-GAP(10): `value-accounting.md` orders the unit family as
// `unit.identity-read`, "one `unit.edge` before each source/target edge",
// `unit.rational-arithmetic`, without saying whether each edge's two events
// follow its own edge charge or all edge charges precede all events. Every
// edge of the operation is charged first, in table order, before any rational
// event.
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

/// `unit.target-domain` with zero amounts for an unbounded rational result.
// SPEC-GAP(11): the pinned `value-accounting.md` lists `unit.target-domain`
// for every unit operation and sizes it as "zero for an unbounded rational
// target", without saying whether quantity arithmetic has a target. Quantity
// arithmetic and exact conversion charge it with no size amount; the result's
// size is already charged by its final `unit.rational-arithmetic` event.
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
    match operation {
        QuantityOperation::Add(a, b) | QuantityOperation::Subtract(a, b) => {
            if !a.unit.is_compatible(&b.unit) {
                return Err(ill_typed(IllTypedCause::IncompatibleDimensions));
            }
            if a.unit.is_affine() || b.unit.is_affine() {
                return Err(ill_typed(IllTypedCause::AffineUnitArithmetic));
            }
            if a.unit != b.unit {
                return Err(ill_typed(IllTypedCause::DistinctUnits));
            }
        }
        QuantityOperation::Multiply(a, b) | QuantityOperation::Divide(a, b) => {
            if a.unit.is_affine() || b.unit.is_affine() {
                return Err(ill_typed(IllTypedCause::AffineUnitArithmetic));
            }
        }
        QuantityOperation::Power(a, _) => {
            if a.unit.is_affine() {
                return Err(ill_typed(IllTypedCause::AffineUnitArithmetic));
            }
        }
    }
    Ok(())
}

/// The uncharged runtime checks after the identity reads.
// SPEC-GAP(12): the pinned FR-142 does not place its checks against
// `unit.identity-read`. Only a zero divisor and a zero base under a negative
// exponent remain runtime checks; each is `undefined(division_by_zero)` after
// the operand identity reads and before any edge or rational event.
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
            value: rational_event(a.value.add(&b.value), meter)?,
            unit: a.unit.clone(),
        },
        QuantityOperation::Subtract(a, b) => Quantity {
            value: rational_event(a.value.sub(&b.value), meter)?,
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
                    value: rational_event(left.mul(&right), meter)?,
                    unit: QuantityUnit::Compound(left_unit.multiply(&right_unit)),
                }
            } else {
                let quotient = left
                    .div(&right)
                    .ok_or(Stop::Undefined(Undefined::DivisionByZero))?;
                Quantity {
                    value: rational_event(quotient, meter)?,
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
                let scaled = rational_event(current.mul(edge.scale()), meter)?;
                rational_event(scaled.add(edge.offset()), meter)?
            }
            Direction::Reverse => {
                let shifted = rational_event(current.sub(edge.offset()), meter)?;
                // Admitted scales are nonzero.
                let divided = shifted
                    .div(edge.scale())
                    .ok_or(Stop::Undefined(Undefined::DivisionByZero))?;
                rational_event(divided, meter)?
            }
        };
    }
    Ok(current)
}

/// Charge the powered result's exact size before computing it. `None` is a
/// zero base under a negative exponent.
// SPEC-GAP(13): FR-142 does not define integer power at a zero base. `0^0`
// is the exact value one, as `Rational::pow` computes; a zero base under a
// negative exponent is `undefined(division_by_zero)`, decided in
// `check_undefined` before any edge or event.
fn power(base: &Rational, exponent: &Integer, meter: &mut Meter) -> Result<Option<Rational>, Stop> {
    // For reduced `n/d`, `(n/d)^e` is reduced with parts `|n|^|e|` and
    // `d^|e|` (swapped for a negative exponent), so its `maxparts` is
    // `bits(max(|n|, d)^|e|)`.
    let largest = base.numerator().abs().max(base.denominator().clone());
    let bits = Integer::power_product_bits(&Integer::one(), &largest, &exponent.abs());
    meter.charge(
        Charge::new(ChargePoint::UnitRationalArithmetic).exact_size(LimitKind::IntegerBits, bits),
    )?;
    Ok(base.pow(exponent))
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
            let placed = place(&exact, decimal, meter)?;
            placed.check_membership(decimal).map_err(Stop::Refused)?;
            charge_retain(meter)?;
            ConvertedValue::Decimal(placed.retain(decimal))
        }
        Target::Integer { placement, domain } => {
            let (coefficient, loss) = place(&exact, placement, meter)?.into_integer();
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

/// Place `exact` at the decimal target's scale and charge
/// `unit.target-domain` with the retained sizes before materializing it.
// SPEC-GAP(15): the pinned unit family does not place FR-140 rounding against
// `unit.target-domain`. Strict `exact` refuses before the charge, which is
// sized analytically on the retained coefficient `v × 10^T`; the coefficient
// is materialized after it, and membership refuses after it and before
// `unit.result-retain`.
fn place(exact: &Rational, decimal: &DecimalType, meter: &mut Meter) -> Result<Placed, Stop> {
    let placement = decimal.placement(exact).map_err(Stop::Refused)?;
    let (bits, digits) = placement.retained_sizes();
    meter.charge(
        Charge::new(ChargePoint::UnitTargetDomain)
            .exact_size(LimitKind::IntegerBits, bits.clone())
            .exact_size(LimitKind::DecimalDigits, digits.clone()),
    )?;
    placement.materialize(decimal).map_err(Stop::Refused)
}

// SPEC-GAP(16): FR-142 and the pinned `value-accounting.md` give no quantity
// comparison schedule. Equality and ordering share one: two
// `unit.identity-read`s, one `unit.edge` per edge of the left then the right
// root path, the two `unit.rational-arithmetic` events of each edge in that
// order, no operation event and no `unit.target-domain`, then
// `unit.result-retain`.
fn compare(left: &Quantity, right: &Quantity, meter: &mut Meter) -> Result<Ordering, Stop> {
    read_identities(&[left, right], meter)?;
    let (left_edges, right_edges) = (to_root(&left.unit), to_root(&right.unit));
    charge_edges(left_edges.len() + right_edges.len(), meter)?;
    let left = events(&left.value, &left_edges, meter)?;
    let right = events(&right.value, &right_edges, meter)?;
    charge_retain(meter)?;
    Ok(left.cmp(&right))
}
