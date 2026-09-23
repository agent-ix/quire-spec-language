// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142 quantities: exact arithmetic under normalized dimensions, explicit
//! affine conversion through the canonical root and the named `unit.*`
//! charges of `quire.value.accounting/v1`.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use super::stop::{outcome_from_stop, Stop};
use super::unit::{CompoundUnit, Dimension, Unit, UnitEdge, UnitGraph};
use quire_exact::{rational_arithmetic_bits, Rational, RationalArithmetic};
use quire_exact::{sbits, sdigits, DecimalLoss, DecimalResult, DecimalType, Placed, RoundingMode};
use quire_exact::{
    BoundedInteger, Charge, ChargePoint, Integer, IntegerInterval, LimitKind, Meter, Quantity,
    UnitId,
};
use quire_exact::{ComparisonOperator, IllTyped, IllTypedCause};
use quire_exact::{Outcome, Refusal, Undefined};

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
    /// The kernel [`UnitId`] a quantity in this unit carries (ADR-013 T-6,
    /// OQ-B): the declared unit's node key or the compound unit's digest.
    pub fn id(&self) -> UnitId {
        match self {
            Self::Declared(unit) => unit.id(),
            Self::Compound(unit) => unit.id(),
        }
    }

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

/// The unit graph over kernel [`UnitId`]s: each id's [`QuantityUnit`]. A
/// kernel [`Quantity`] carries only its unit's id, so every FR-142 operation
/// reads its operands through a table (ADR-013 T-6: the unit graph stays in
/// `semantic_value`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UnitTable(BTreeMap<UnitId, QuantityUnit>);

impl UnitTable {
    /// Every admitted unit of `graph`, by its declared-arm id.
    pub fn declared(graph: &UnitGraph) -> Self {
        graph
            .units()
            .map(|unit| QuantityUnit::Declared(Box::new(unit.clone())))
            .collect()
    }

    /// Record `unit` under its id and return the id.
    pub fn insert(&mut self, unit: QuantityUnit) -> UnitId {
        let id = unit.id();
        self.0.entry(id).or_insert(unit);
        id
    }

    /// The unit with this id.
    pub fn get(&self, id: UnitId) -> Option<&QuantityUnit> {
        self.0.get(&id)
    }

    /// `quantity` with its unit resolved, or `None` when this table has no
    /// unit with the quantity's id.
    pub fn resolve<'a>(&'a self, quantity: &'a Quantity) -> Option<UnitQuantity<'a>> {
        self.get(quantity.unit())
            .map(|unit| UnitQuantity { quantity, unit })
    }
}

impl FromIterator<QuantityUnit> for UnitTable {
    fn from_iter<I: IntoIterator<Item = QuantityUnit>>(units: I) -> Self {
        let mut table = Self::default();
        for unit in units {
            table.insert(unit);
        }
        table
    }
}

/// One stage's units: the package's [`UnitTable`], then every compound unit
/// the stage itself formed. Checking forms each static product and quotient
/// unit; evaluation forms each computed one, so a later operation can read a
/// compound operand the package never declared.
#[derive(Clone, Debug)]
pub(crate) struct UnitScope<'a> {
    package: &'a UnitTable,
    formed: UnitTable,
}

impl<'a> UnitScope<'a> {
    pub(crate) fn new(package: &'a UnitTable) -> Self {
        Self {
            package,
            formed: UnitTable::default(),
        }
    }

    pub(crate) fn get(&self, id: UnitId) -> Option<&QuantityUnit> {
        self.package.get(id).or_else(|| self.formed.get(id))
    }

    /// Record a unit this stage formed and return its id.
    pub(crate) fn form(&mut self, unit: QuantityUnit) -> UnitId {
        let id = unit.id();
        if self.package.get(id).is_none() {
            self.formed.insert(unit);
        }
        id
    }

    pub(crate) fn resolve<'q>(&'q self, quantity: &'q Quantity) -> Option<UnitQuantity<'q>> {
        self.package
            .resolve(quantity)
            .or_else(|| self.formed.resolve(quantity))
    }
}

/// A kernel [`Quantity`] read against the unit graph: its magnitude and its
/// unit. Built only by resolving the quantity's own [`UnitId`]
/// ([`UnitTable::resolve`]), so the unit is always the quantity's.
#[derive(Clone, Copy, Debug)]
pub struct UnitQuantity<'a> {
    quantity: &'a Quantity,
    unit: &'a QuantityUnit,
}

impl<'a> UnitQuantity<'a> {
    /// The kernel quantity.
    pub fn quantity(&self) -> &'a Quantity {
        self.quantity
    }

    /// The resolved unit.
    pub fn unit(&self) -> &'a QuantityUnit {
        self.unit
    }

    fn value(&self) -> &'a Rational {
        self.quantity.magnitude()
    }
}

/// A quantity operation over resolved operands.
#[derive(Clone, Copy, Debug)]
pub enum QuantityOperation<'a> {
    /// `a + b` in one common unit.
    Add(UnitQuantity<'a>, UnitQuantity<'a>),
    /// `a - b` in one common unit.
    Subtract(UnitQuantity<'a>, UnitQuantity<'a>),
    /// `a * b` under the canonical compound unit.
    Multiply(UnitQuantity<'a>, UnitQuantity<'a>),
    /// `a / b` under the canonical compound unit.
    Divide(UnitQuantity<'a>, UnitQuantity<'a>),
    /// `a ^ n` for a mathematical integer `n`.
    Power(UnitQuantity<'a>, &'a Integer),
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

/// Evaluate a quantity operation under `quire.value.accounting/v1`. The
/// result carries the id of its unit: the operands' identical unit for `+`
/// and `-`, otherwise the canonical compound unit. Incompatible dimensions,
/// affine-unit arithmetic and distinct units are ill-typed and consume
/// nothing.
pub fn evaluate_quantity(
    operation: QuantityOperation<'_>,
    meter: &mut Meter,
) -> Result<Outcome<Quantity>, IllTyped> {
    evaluate_quantity_unit(operation, meter).map(|(outcome, _)| outcome)
}

/// [`evaluate_quantity`] with the result's unit, formed once at type time
/// and carried by the result quantity's id, for a caller that records it.
pub(crate) fn evaluate_quantity_unit(
    operation: QuantityOperation<'_>,
    meter: &mut Meter,
) -> Result<(Outcome<Quantity>, QuantityUnit), IllTyped> {
    let unit = type_check(operation)?;
    let outcome = outcome_from_stop(evaluate(operation, unit.id(), meter));
    Ok((outcome, unit))
}

/// Explicitly convert `source` into `unit` with the `target` representation.
/// Incompatible dimensions are ill-typed and consume nothing.
pub fn convert_quantity(
    source: UnitQuantity<'_>,
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
    Ok(outcome_from_stop(convert(source, unit, &target, meter)))
}

/// Compare two quantities of the identical unit under
/// `quire.value.accounting/v1` by their exact values mapped to the canonical
/// root, so affine units and negative composed scales compare by root value.
/// Operands of different dimensions or units are ill-typed and consume
/// nothing.
pub fn compare_quantity(
    operator: ComparisonOperator,
    left: UnitQuantity<'_>,
    right: UnitQuantity<'_>,
    meter: &mut Meter,
) -> Result<Outcome<bool>, IllTyped> {
    check_comparable(left.unit, right.unit)?;
    Ok(outcome_from_stop(
        compare(left, right, meter).map(|ordering| operator.holds(ordering)),
    ))
}

fn ill_typed(cause: IllTypedCause) -> IllTyped {
    IllTyped { cause }
}

/// A kernel decimal-placement refusal as this evaluator's stop.
fn refused(refusal: Refusal) -> Stop {
    Stop::Refused(refusal)
}

/// One `unit.identity-read` per operand, each sized over the operands read so
/// far.
fn read_identities(operands: &[UnitQuantity<'_>], meter: &mut Meter) -> Result<(), Stop> {
    let mut bits = 0;
    for (read, operand) in (1_u64..).zip(operands) {
        bits = operand.value().max_part_bits().max(bits);
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
        RationalArithmetic::Add(left, right) => left.add(right),
        RationalArithmetic::Subtract(left, right) => left.sub(right),
        RationalArithmetic::Multiply(left, right) => left.mul(right),
        RationalArithmetic::Divide(left, right) => left
            .div(right)
            .ok_or(Stop::Undefined(Undefined::DivisionByZero))?,
        RationalArithmetic::Negate(operand) => operand.neg(),
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
/// dimensions, affine-unit arithmetic, distinct units; otherwise its result
/// unit.
fn type_check(operation: QuantityOperation<'_>) -> Result<QuantityUnit, IllTyped> {
    let (unit_operation, a, b) = match operation {
        QuantityOperation::Add(a, b) => (UnitOperation::Add, a, b),
        QuantityOperation::Subtract(a, b) => (UnitOperation::Subtract, a, b),
        QuantityOperation::Multiply(a, b) => (UnitOperation::Multiply, a, b),
        QuantityOperation::Divide(a, b) => (UnitOperation::Divide, a, b),
        QuantityOperation::Power(a, n) => {
            if a.unit.is_affine() {
                return Err(ill_typed(IllTypedCause::AffineUnitArithmetic));
            }
            return Ok(QuantityUnit::Compound(a.unit.canonical_compound().power(n)));
        }
    };
    result_unit(unit_operation, a.unit, b.unit)
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
        QuantityOperation::Divide(_, divisor) => divisor.value().is_zero(),
        QuantityOperation::Power(base, exponent) => {
            base.value().is_zero() && exponent.is_negative()
        }
        QuantityOperation::Add(..)
        | QuantityOperation::Subtract(..)
        | QuantityOperation::Multiply(..) => false,
    };
    if undefined {
        return Err(Stop::Undefined(Undefined::DivisionByZero));
    }
    Ok(())
}

/// Evaluate a type-checked operation whose result unit has id `unit`.
fn evaluate(
    operation: QuantityOperation<'_>,
    unit: UnitId,
    meter: &mut Meter,
) -> Result<Quantity, Stop> {
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
        QuantityOperation::Add(a, b) => Quantity::new(
            rational_event(RationalArithmetic::Add(a.value(), b.value()), meter)?,
            unit,
        ),
        QuantityOperation::Subtract(a, b) => Quantity::new(
            rational_event(RationalArithmetic::Subtract(a.value(), b.value()), meter)?,
            unit,
        ),
        QuantityOperation::Multiply(a, b) | QuantityOperation::Divide(a, b) => {
            let (left_edges, right_edges) = (to_root(a.unit), to_root(b.unit));
            charge_edges(left_edges.len() + right_edges.len(), meter)?;
            let left = events(a.value(), &left_edges, meter)?;
            let right = events(b.value(), &right_edges, meter)?;
            let value = if matches!(operation, QuantityOperation::Multiply(..)) {
                rational_event(RationalArithmetic::Multiply(&left, &right), meter)?
            } else {
                rational_event(RationalArithmetic::Divide(&left, &right), meter)?
            };
            Quantity::new(value, unit)
        }
        QuantityOperation::Power(a, exponent) => {
            let edges = to_root(a.unit);
            charge_edges(edges.len(), meter)?;
            let base = events(a.value(), &edges, meter)?;
            Quantity::new(
                power(&base, exponent, meter)?.ok_or(Stop::Undefined(Undefined::DivisionByZero))?,
                unit,
            )
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
    source: UnitQuantity<'_>,
    unit: &QuantityUnit,
    target: &Target<'_>,
    meter: &mut Meter,
) -> Result<Conversion, Stop> {
    read_identities(&[source], meter)?;
    let source_edges = to_root(source.unit);
    let target_edges: Vec<_> = unit
        .path()
        .iter()
        .rev()
        .map(|edge| (edge, Direction::Reverse))
        .collect();
    charge_edges(source_edges.len() + target_edges.len(), meter)?;
    let canonical = events(source.value(), &source_edges, meter)?;
    let exact = events(&canonical, &target_edges, meter)?;
    let value = match target {
        Target::Exact => {
            charge_rational_target(meter)?;
            charge_retain(meter)?;
            ConvertedValue::Exact(exact)
        }
        Target::Decimal(decimal) => {
            let admitted = place(&exact, decimal, Retained::Decimal, meter)?
                .admit()
                .map_err(refused)?;
            charge_retain(meter)?;
            ConvertedValue::Decimal(admitted.retain())
        }
        Target::Integer { placement, domain } => {
            let out_of_domain = || Stop::Refused(Refusal::IntegerOutOfDomain);
            // The scale-zero placement's membership is the integer domain.
            let (coefficient, loss) = place(&exact, placement, Retained::Integer, meter)?
                .admit()
                .map_err(|_| out_of_domain())?
                .into_integer()
                .ok_or(Stop::Refused(Refusal::CheckedInvariant))?;
            let value = domain.admit(coefficient).map_err(|_| out_of_domain())?;
            charge_retain(meter)?;
            ConvertedValue::Integer { value, loss }
        }
    };
    Ok(Conversion {
        source: source.quantity.clone(),
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
    let placement = decimal.placement(exact).map_err(refused)?;
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
    placement.materialize().map_err(refused)
}

/// The top-level equality and ordering schedule: both root paths, left then
/// right, with no operation event and no `unit.target-domain`.
fn compare(
    left: UnitQuantity<'_>,
    right: UnitQuantity<'_>,
    meter: &mut Meter,
) -> Result<Ordering, Stop> {
    read_identities(&[left, right], meter)?;
    let (left_edges, right_edges) = (to_root(left.unit), to_root(right.unit));
    charge_edges(left_edges.len() + right_edges.len(), meter)?;
    let left = events(left.value(), &left_edges, meter)?;
    let right = events(right.value(), &right_edges, meter)?;
    charge_retain(meter)?;
    Ok(left.cmp(&right))
}
