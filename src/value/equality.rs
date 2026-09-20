// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FR-149 complete typed equality matrix.
//!
//! [`TypeEnvironment::check_equality`] is the static stage: it admits each
//! `convert<T>(e)` operand only through the closed equality-conversion table,
//! refuses `=` on any IEEE-bearing type as `operator-ineligible`, and selects
//! one schedule from the common type. [`CheckedEquality::evaluate`] then runs
//! any conversion charges in operand order and the selected schedule: FR-141
//! text, FR-141 enum or FR-142 quantity comparison for those top-level types,
//! and otherwise `equality.plan-form`, `equality.plan`, one `equality.pair`
//! per planned occurrence-path pair and `equality.result-retain`.
//!
//! Plan formation is iterative, so value depth never reaches the host stack,
//! and it walks the occurrence tree, so DAG sharing never changes the plan.

use super::accounting::{Charge, ChargePoint, LimitKind, Meter};
use super::comparison::{ComparisonOperator, IllTyped, IllTypedCause};
use super::composite::{FieldValue, TypeEnvironment, Value, ValueType};
use super::decimal::{
    compare_shifted, evaluate_decimal, power_of_ten_bits, sbits, sdigits, Decimal,
    DecimalOperation, DecimalType,
};
use super::enumeration::compare_enum;
use super::integer::Integer;
use super::outcome::{Outcome, Refusal, Stop};
use super::quantity::{
    compare_quantity, convert_quantity, ConvertedValue, Quantity, QuantityTarget,
};
use super::rational::Rational;
use super::text::compare_text;

/// The grammar's equality operators.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EqualityOperator {
    /// `=`.
    Equal,
    /// `!=`: the same schedule, retaining the negated Boolean.
    NotEqual,
}

impl EqualityOperator {
    fn comparison(self) -> ComparisonOperator {
        match self {
            Self::Equal => ComparisonOperator::Equal,
            Self::NotEqual => ComparisonOperator::NotEqual,
        }
    }
}

/// The static type of one equality operand.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EqualityOperand {
    source: ValueType,
    target: Option<ValueType>,
}

impl EqualityOperand {
    /// An operand `e` of static type `source`.
    pub fn typed(source: ValueType) -> Self {
        Self {
            source,
            target: None,
        }
    }

    /// An operand `convert<target>(e)` for `e` of static type `source`.
    pub fn converted(source: ValueType, target: ValueType) -> Self {
        Self {
            source,
            target: Some(target),
        }
    }

    /// The comparison type.
    fn comparison_type(&self) -> &ValueType {
        self.target.as_ref().unwrap_or(&self.source)
    }
}

/// The schedule FR-149 selects from the common type.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EqualitySchedule {
    /// A top-level text pair: the FR-141 text schedule.
    Text,
    /// A top-level enumeration pair: `enum.*`.
    Enum,
    /// A top-level quantity pair: the FR-142 comparison schedule.
    Quantity,
    /// Every other common type: the occurrence-pair plan.
    Plan,
}

/// A type-checked equality expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedEquality {
    operator: EqualityOperator,
    left: EqualityOperand,
    right: EqualityOperand,
    schedule: EqualitySchedule,
}

impl TypeEnvironment {
    /// Type-check `left op right`. Every refusal is made before any charge.
    pub fn check_equality(
        &self,
        operator: EqualityOperator,
        left: EqualityOperand,
        right: EqualityOperand,
    ) -> Result<CheckedEquality, IllTyped> {
        let ill_typed = |cause| Err(IllTyped { cause });
        for operand in [&left, &right] {
            if let Some(target) = &operand.target {
                if !admits_equality_conversion(&operand.source, target) {
                    return ill_typed(IllTypedCause::TypeMismatch);
                }
            }
        }
        let (left_type, right_type) = (left.comparison_type(), right.comparison_type());
        if self.contains_ieee(left_type) || self.contains_ieee(right_type) {
            return ill_typed(IllTypedCause::OperatorIneligible);
        }
        let schedule = match (left_type, right_type) {
            (ValueType::Text(l), ValueType::Text(r)) if l.profile() != r.profile() => {
                return ill_typed(IllTypedCause::DistinctTextProfiles)
            }
            (ValueType::Text(_), ValueType::Text(_)) => EqualitySchedule::Text,
            (ValueType::Enum(l), ValueType::Enum(r)) if l != r => {
                return ill_typed(IllTypedCause::DistinctEnumDeclarations)
            }
            (ValueType::Enum(_), ValueType::Enum(_)) => EqualitySchedule::Enum,
            (ValueType::Quantity(l), ValueType::Quantity(r)) if !l.has_dimension_of(r) => {
                return ill_typed(IllTypedCause::IncompatibleDimensions)
            }
            (ValueType::Quantity(l), ValueType::Quantity(r)) if l != r => {
                return ill_typed(IllTypedCause::DistinctUnits)
            }
            (ValueType::Quantity(_), ValueType::Quantity(_)) => EqualitySchedule::Quantity,
            // FR-153 names a population binding only as the direct operand of
            // `allInstances`/`lookup`, never as an equality operand: refuse it
            // here rather than falling into the `l == r` plan schedule below,
            // which would otherwise accept `p == p` and only fail at
            // evaluation (`plan_pairs`'s own checked-invariant catch-all).
            (ValueType::Population(_), _) | (_, ValueType::Population(_)) => {
                return ill_typed(IllTypedCause::OperatorIneligible)
            }
            // FR-149/TC-198 L08: a `Reference<A>` and a `Reference<B>` may
            // denote the same real object even when `A != B`, as long as one
            // conforms to the other in `self`'s own admitted generalization
            // graph (H1, #204 round 1) -- the same upcast relation
            // `Self::conforms` already grants dispatch call arguments
            // (`crate::value::expression::check::Typer::check_dispatch_argument`).
            // Evaluation compares the completed `ObjectReference`s' full
            // identity regardless of either operand's static type
            // (`plan_pairs`'s `Value::Reference` arm), so this only widens
            // which pairs reach that comparison, never how it decides.
            (ValueType::Reference(l), ValueType::Reference(r))
                if l != r && (self.conforms(*l, *r) || self.conforms(*r, *l)) =>
            {
                EqualitySchedule::Plan
            }
            (l, r) if l == r => EqualitySchedule::Plan,
            _ => return ill_typed(IllTypedCause::TypeMismatch),
        };
        Ok(CheckedEquality {
            operator,
            left,
            right,
            schedule,
        })
    }
}

impl CheckedEquality {
    /// The selected schedule.
    pub fn schedule(&self) -> EqualitySchedule {
        self.schedule
    }

    /// Evaluate over the two completed operand values, left conversion first.
    /// Neither source value is changed.
    pub fn evaluate(&self, left: &Value, right: &Value, meter: &mut Meter) -> Outcome<bool> {
        Outcome::from_stop(self.run(left, right, meter))
    }

    fn run(&self, left: &Value, right: &Value, meter: &mut Meter) -> Result<bool, Stop> {
        let left = operand_value(&self.left, left, meter)?;
        let right = operand_value(&self.right, right, meter)?;
        let operator = self.operator.comparison();
        let scheduled = match (self.schedule, &left, &right) {
            (EqualitySchedule::Text, Value::Text(l), Value::Text(r)) => {
                compare_text(operator, l, r, meter)
            }
            (EqualitySchedule::Enum, Value::Enum(l), Value::Enum(r)) => {
                compare_enum(operator, l, r, meter)
            }
            (EqualitySchedule::Quantity, Value::Quantity(l), Value::Quantity(r)) => {
                compare_quantity(operator, l, r, meter)
            }
            (EqualitySchedule::Plan, _, _) => {
                let equal = planned_equality(&left, &right, meter)?;
                return Ok(equal == (self.operator == EqualityOperator::Equal));
            }
            (
                EqualitySchedule::Text | EqualitySchedule::Enum | EqualitySchedule::Quantity,
                _,
                _,
            ) => return Err(invariant()),
        };
        scheduled.map_err(|_| invariant())?.into_stop()
    }
}

fn invariant() -> Stop {
    Stop::Refused(Refusal::CheckedInvariant)
}

/// The complete occurrence-pair plan of one planned equality.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct EqualityPlan {
    pair_events: Integer,
}

impl EqualityPlan {
    /// The exact number of planned `equality.pair` events.
    pub fn pair_events(&self) -> &Integer {
        &self.pair_events
    }
}

/// Form the plan of two completed operands of one type, without charge. A
/// reference pair of different universes refuses with `foreign_reference`.
pub fn plan_equality(left: &Value, right: &Value) -> Result<EqualityPlan, Refusal> {
    plan_pairs(left, right).map(|plan| EqualityPlan {
        pair_events: plan.pairs,
    })
}

/// The equality schedule over completed operands of one type.
fn planned_equality(left: &Value, right: &Value, meter: &mut Meter) -> Result<bool, Stop> {
    let (left_occ, right_occ) = (left.occ(), right.occ());
    meter.charge(
        Charge::new(ChargePoint::EqualityPlanForm)
            .exact_size(
                LimitKind::ValueOccurrences,
                left_occ.clone().max(right_occ.clone()),
            )
            .work(left_occ.add(&right_occ)),
    )?;
    let plan = plan_pairs(left, right).map_err(Stop::Refused)?;
    meter.charge_plan(&plan.pairs)?;
    let mut remaining = plan.pairs;
    while !remaining.is_zero() {
        meter.charge(Charge::new(ChargePoint::EqualityPair))?;
        remaining = remaining.sub(&Integer::one());
    }
    meter.charge(Charge::new(ChargePoint::EqualityResultRetain).results(1))?;
    Ok(plan.equal)
}

/// A formed plan: its pair count and the relation's Boolean.
pub(crate) struct PlannedPairs {
    pub(crate) pairs: Integer,
    pub(crate) equal: bool,
}

/// One node of the occurrence-pair tree still to be formed.
enum Pair<'a> {
    Values(&'a Value, &'a Value),
    Slots(&'a FieldValue, &'a FieldValue),
}

/// Walk the FR-149 occurrence-pair tree of two values of one type. Operands
/// that are not of one type, which a checked program never produces, refuse
/// with the checked invariant.
pub(crate) fn plan_pairs(left: &Value, right: &Value) -> Result<PlannedPairs, Refusal> {
    let mut pairs = Integer::zero();
    let mut equal = true;
    let mut pending = vec![Pair::Values(left, right)];
    while let Some(pair) = pending.pop() {
        pairs = pairs.add(&Integer::one());
        let (left, right) = match pair {
            Pair::Slots(FieldValue::Present(left), FieldValue::Present(right)) => (left, right),
            Pair::Slots(FieldValue::Absent, FieldValue::Absent)
            | Pair::Slots(FieldValue::Null, FieldValue::Null) => continue,
            Pair::Slots(FieldValue::Present(_) | FieldValue::Absent | FieldValue::Null, _) => {
                equal = false;
                continue;
            }
            Pair::Values(left, right) => (left, right),
        };
        let leaf = match (left, right) {
            (Value::Boolean(l), Value::Boolean(r)) => l == r,
            (Value::Integer(l), Value::Integer(r)) => l == r,
            (Value::Rational(l), Value::Rational(r)) => l == r,
            (Value::Decimal(l), Value::Decimal(r)) => l.numerically_equal(r),
            (Value::Quantity(l), Value::Quantity(r)) if l.unit() == r.unit() => {
                l.value() == r.value()
            }
            (Value::Text(l), Value::Text(r))
                if l.text_type().profile() == r.text_type().profile() =>
            {
                l.retained() == r.retained()
            }
            (Value::Enum(l), Value::Enum(r)) if l.declaration() == r.declaration() => {
                l.member() == r.member()
            }
            (Value::Reference(l), Value::Reference(r)) => {
                if l.universe() != r.universe() {
                    return Err(Refusal::ForeignReference);
                }
                l == r
            }
            (Value::Option(l), Value::Option(r)) => match (l.payload(), r.payload()) {
                (Some(l), Some(r)) => {
                    pending.push(Pair::Values(l, r));
                    continue;
                }
                (l, r) => l.is_some() == r.is_some(),
            },
            (Value::Composite(l), Value::Composite(r))
                if l.declaration() == r.declaration() && l.slots().len() == r.slots().len() =>
            {
                let slots = l.slots().iter().zip(r.slots()).rev();
                pending.extend(slots.map(|(l, r)| Pair::Slots(l, r)));
                continue;
            }
            (Value::Collection(l), Value::Collection(r)) => {
                let kind = l.collection_type().kind();
                if kind != r.collection_type().kind() {
                    return Err(Refusal::CheckedInvariant);
                }
                let (l, r) = (l.elements(), r.elements());
                // A set's member count and a bag's occurrence count are the
                // stored lengths; the other kinds compare lengths directly.
                if l.len() != r.len() {
                    false
                } else {
                    let ranks = l.iter().zip(r).rev();
                    pending.extend(ranks.map(|(l, r)| Pair::Values(l, r)));
                    continue;
                }
            }
            (
                Value::Boolean(_)
                | Value::Integer(_)
                | Value::Rational(_)
                | Value::Decimal(_)
                | Value::Float(_)
                | Value::Quantity(_)
                | Value::Text(_)
                | Value::Enum(_)
                | Value::Reference(_)
                | Value::Option(_)
                | Value::Composite(_)
                | Value::Collection(_)
                | Value::Population(_),
                _,
            ) => return Err(Refusal::CheckedInvariant),
        };
        equal = equal && leaf;
    }
    Ok(PlannedPairs { pairs, equal })
}

/// Whether (`source`, `target`) is a row of the closed FR-149
/// equality-conversion table, decided from declared bounds alone.
pub fn admits_equality_conversion(source: &ValueType, target: &ValueType) -> bool {
    match (source, target) {
        (source, target) if source == target => true,
        (ValueType::Int(interval), target) => {
            integer_source_admits(interval.lower(), interval.upper(), target)
        }
        (ValueType::Rational(from), ValueType::Rational(to)) => {
            to.numerator().lower() <= from.numerator().lower()
                && from.numerator().upper() <= to.numerator().upper()
                && to.denominator().lower() <= from.denominator().lower()
                && from.denominator().upper() <= to.denominator().upper()
        }
        (
            ValueType::Rational(from),
            target @ (ValueType::Integer | ValueType::Int(_) | ValueType::Decimal(_)),
        ) if from.denominator().upper() == &Integer::one() => {
            integer_source_admits(from.numerator().lower(), from.numerator().upper(), target)
        }
        (ValueType::Decimal(from), ValueType::Rational(to)) => {
            let zero = Integer::zero();
            to.numerator().lower() <= from.lower().min(&zero)
                && from.upper().max(&zero) <= to.numerator().upper()
                && to.denominator().lower() <= &Integer::one()
                && compare_shifted(
                    &Integer::one(),
                    u64::from(from.max_scale()),
                    to.denominator().upper(),
                )
                .is_le()
        }
        (ValueType::Decimal(from), ValueType::Decimal(to)) => {
            let zero = Integer::zero();
            to.min_scale() <= from.min_scale()
                && from.max_scale() <= to.max_scale()
                && if to.min_scale() == from.min_scale() {
                    to.lower() <= from.lower() && from.upper() <= to.upper()
                } else {
                    to.lower() <= from.lower().min(&zero) && from.upper().max(&zero) <= to.upper()
                }
        }
        (ValueType::Decimal(from), target @ (ValueType::Integer | ValueType::Int(_)))
            if from.max_scale() == 0 =>
        {
            integer_source_admits(from.lower(), from.upper(), target)
        }
        (ValueType::Quantity(from), ValueType::Quantity(to)) => from.converts_to(to),
        _ => false,
    }
}

/// The rows for a source `Int[lo, hi]`.
fn integer_source_admits(lower: &Integer, upper: &Integer, target: &ValueType) -> bool {
    match target {
        ValueType::Integer => true,
        ValueType::Int(to) => to.lower() <= lower && upper <= to.upper(),
        ValueType::Rational(to) => {
            let one = Integer::one();
            to.numerator().lower() <= lower
                && upper <= to.numerator().upper()
                && to.denominator().lower() <= &one
                && &one <= to.denominator().upper()
        }
        ValueType::Decimal(to) => {
            let shift = u64::from(to.min_scale());
            compare_shifted(lower, shift, to.lower()).is_ge()
                && compare_shifted(upper, shift, to.upper()).is_le()
        }
        ValueType::Boolean
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::Text(_)
        | ValueType::Enum(_)
        | ValueType::Option(_)
        | ValueType::Composite(_)
        | ValueType::Collection(_)
        | ValueType::Reference(_)
        | ValueType::Population(_) => false,
    }
}

/// The comparison value of one operand after its admitted conversion.
pub(crate) fn operand_value(
    operand: &EqualityOperand,
    value: &Value,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    // A `Reference<T>` operand's own runtime value carries its real
    // most-specific object type, not `T` (#164 item 1; the identical
    // soundness chain `OptionValue::from_admitted`'s own doc comment states
    // for `lookup`'s upcast case): `Self::check_equality` already proved `T`
    // conforms to (or equals) the peer operand's own declared type before
    // this ever runs, so `ValueType::Reference(T)::admits`'s exact-key
    // structural check is both redundant for an equal pair and wrong for a
    // genuine upcast pair -- skip it here and let `plan_pairs`'s own
    // `Value::Reference` arm decide identity. `Reference` is exhaustively
    // named as the one exception, not wildcarded past: object types are the
    // only nominal type in this crate with a declared supertype/conformance
    // relation (`ObjectTypeDeclaration::with_supertypes`), so every other
    // variant's declared type and runtime type can never diverge -- each is
    // structural (collection kind/element/bound) or an exact declaration-key
    // match (composite, enum). If a future variant gains its own nominal
    // subtyping, this match stops compiling instead of silently admitting an
    // unvetted upcast through the wrong arm.
    let admits_by_conformance_not_structure = match &operand.source {
        ValueType::Reference(_) => true,
        ValueType::Boolean
        | ValueType::Integer
        | ValueType::Int(_)
        | ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::Text(_)
        | ValueType::Enum(_)
        | ValueType::Option(_)
        | ValueType::Composite(_)
        | ValueType::Collection(_)
        | ValueType::Population(_) => false,
    };
    if !admits_by_conformance_not_structure && !operand.source.admits(value) {
        return Err(invariant());
    }
    let Some(target) = &operand.target else {
        return Ok(value.clone());
    };
    let converted = match (&operand.source, target, value) {
        (source, target, value) if source == target => value.clone(),
        (_, ValueType::Integer | ValueType::Int(_), Value::Integer(_)) => value.clone(),
        (_, ValueType::Rational(_), Value::Integer(integer)) => {
            Value::Rational(Rational::from_integer(integer.clone()))
        }
        (_, ValueType::Rational(_), Value::Rational(_)) => value.clone(),
        (_, ValueType::Integer | ValueType::Int(_), Value::Rational(rational))
            if rational.is_integer() =>
        {
            Value::Integer(rational.numerator().clone())
        }
        (_, ValueType::Decimal(to), Value::Integer(integer)) => {
            integer_to_decimal(integer, to, meter)?
        }
        (_, ValueType::Decimal(to), Value::Rational(rational)) if rational.is_integer() => {
            integer_to_decimal(rational.numerator(), to, meter)?
        }
        (_, ValueType::Rational(_), Value::Decimal(decimal)) => {
            decimal_to_rational(decimal, meter)?
        }
        (_, ValueType::Decimal(to), Value::Decimal(decimal)) => {
            let result =
                evaluate_decimal(DecimalOperation::Round(decimal), to, meter).into_stop()?;
            Value::Decimal(result.value().clone())
        }
        (_, ValueType::Integer | ValueType::Int(_), Value::Decimal(decimal)) => {
            let rational = decimal.normalized().to_rational();
            if !rational.is_integer() {
                return Err(invariant());
            }
            Value::Integer(rational.numerator().clone())
        }
        (_, ValueType::Quantity(unit), Value::Quantity(quantity)) => {
            let conversion = convert_quantity(quantity, unit, &QuantityTarget::Exact, meter)
                .map_err(|_| invariant())?
                .into_stop()?;
            match conversion.value() {
                ConvertedValue::Exact(exact) => {
                    Value::Quantity(Quantity::new(exact.clone(), unit.clone()))
                }
                ConvertedValue::Decimal(_) | ConvertedValue::Integer { .. } => {
                    return Err(invariant())
                }
            }
        }
        _ => return Err(invariant()),
    };
    if target.admits(&converted) {
        Ok(converted)
    } else {
        Err(invariant())
    }
}

/// `Int[..]` or `Rational[..;d,1]` integer `n` into `Decimal[c1,c2;s1,s2]`,
/// retained as `(n × 10^s1, s1)`.
fn integer_to_decimal(
    value: &Integer,
    target: &DecimalType,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    let scale = u64::from(target.min_scale());
    meter.charge(
        Charge::new(ChargePoint::DecimalOperands)
            .size(LimitKind::IntegerBits, value.magnitude_bits())
            .size(LimitKind::DecimalDigits, value.decimal_digits())
            .size(LimitKind::ValueOccurrences, 1),
    )?;
    let (bits, digits) = (sbits(value, scale), sdigits(value, scale));
    meter.charge(
        Charge::new(ChargePoint::DecimalScaleExpansion)
            .size(LimitKind::ScaleExpansion, scale)
            .exact_size(LimitKind::IntegerBits, bits.clone())
            .exact_size(LimitKind::DecimalDigits, digits.clone()),
    )?;
    meter.charge(
        Charge::new(ChargePoint::DecimalArithmetic)
            .exact_size(LimitKind::IntegerBits, bits.clone())
            .exact_size(LimitKind::DecimalDigits, digits.clone()),
    )?;
    // Retention upscales `n` at scale 0 by `k = s1`, sized before
    // `n × 10^s1` is materialized.
    meter.charge(
        Charge::new(ChargePoint::DecimalResultRetain)
            .size(LimitKind::ScaleExpansion, scale)
            .exact_size(LimitKind::IntegerBits, bits)
            .exact_size(LimitKind::DecimalDigits, digits)
            .size(LimitKind::ValueOccurrences, 1)
            .results(1),
    )?;
    let coefficient = value.mul(&Integer::power_of_ten(scale));
    Ok(Value::Decimal(Decimal::new(
        coefficient,
        target.min_scale(),
    )))
}

/// A `Decimal` `(c, s)` into an exact rational.
fn decimal_to_rational(value: &Decimal, meter: &mut Meter) -> Result<Value, Stop> {
    let representation = value.representation();
    let coefficient = representation.coefficient();
    let scale = u64::from(representation.scale());
    meter.charge(
        Charge::new(ChargePoint::DecimalOperands)
            .size(LimitKind::IntegerBits, coefficient.magnitude_bits())
            .size(LimitKind::DecimalDigits, coefficient.decimal_digits())
            .size(LimitKind::ValueOccurrences, 1),
    )?;
    meter.charge(
        Charge::new(ChargePoint::DecimalScaleExpansion)
            .size(LimitKind::ScaleExpansion, scale)
            .exact_size(LimitKind::IntegerBits, power_of_ten_bits(scale)),
    )?;
    meter.charge(
        Charge::new(ChargePoint::DecimalArithmetic)
            .exact_size(
                LimitKind::IntegerBits,
                Integer::from(coefficient.magnitude_bits()).max(power_of_ten_bits(scale)),
            )
            .exact_size(
                LimitKind::DecimalDigits,
                Integer::from(coefficient.decimal_digits())
                    .max(Integer::from(scale).add(&Integer::one())),
            ),
    )?;
    let rational = representation.to_rational();
    let maxparts = rational.max_part_bits();
    meter.charge(
        Charge::new(ChargePoint::DecimalResultRetain)
            .size(LimitKind::IntegerBits, maxparts)
            .size(LimitKind::ValueOccurrences, 1)
            .results(1),
    )?;
    Ok(Value::Rational(rational))
}
