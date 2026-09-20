// SPDX-License-Identifier: AGPL-3.0-or-later
//! O-13/T-6 bare quantity values: a magnitude in a unit, with no unit graph.
//!
//! ADR-013 T-6: "Quantity payload→magnitude+UnitId, with no reference to
//! quantity declarations." This is a **fresh, minimal design**, not a port
//! of QSL `value::quantity`: the original `QuantityUnit`/`Quantity` carry a
//! unit graph (dimension, composed conversion factors and offsets, edge
//! traversal for `convert_quantity`) so that quantities in *different but
//! compatible* units can be added, compared or converted. None of that graph
//! is a kernel type -- it is QSL's own unit declarations. The kernel
//! `Quantity` therefore only supports operations between quantities already
//! in the *same* unit; converting first to a common unit, when the units
//! differ but are compatible, is QSL's job, done above this module with the
//! unit graph it holds and the kernel isn't given.
//!
//! This is a real capability loss at the kernel boundary (no
//! cross-unit arithmetic, comparison or equality), not an oversight --
//! flagged for review alongside the identical Enum-ordering and
//! Reference-identity cuts in `crate::value`'s and `crate::key`'s module doc
//! comments.

use crate::accounting::{Charge, ChargePoint, Meter};
use crate::comparison::{ComparisonOperator, IllTyped, IllTypedCause};
use crate::identity::UnitId;
use crate::numeric::{evaluate_rational_arithmetic, RationalArithmetic};
use crate::outcome::Outcome;
use crate::rational::Rational;

/// A quantity value: a magnitude in exactly one unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Quantity {
    magnitude: Rational,
    unit: UnitId,
}

impl Quantity {
    /// A quantity of `magnitude` in `unit`.
    pub fn new(magnitude: Rational, unit: UnitId) -> Self {
        Self { magnitude, unit }
    }

    /// The magnitude.
    pub fn magnitude(&self) -> &Rational {
        &self.magnitude
    }

    /// The unit.
    pub fn unit(&self) -> UnitId {
        self.unit
    }
}

fn same_unit(left: &Quantity, right: &Quantity) -> Result<(), IllTyped> {
    if left.unit == right.unit {
        Ok(())
    } else {
        Err(IllTyped {
            cause: IllTypedCause::DistinctUnits,
        })
    }
}

/// Add or subtract two quantities in the same unit. Operands in different
/// units are ill-typed and consume nothing.
pub fn evaluate_quantity_arithmetic(
    operation: QuantityArithmetic<'_>,
    meter: &mut Meter,
) -> Result<Outcome<Quantity>, IllTyped> {
    let (left, right, add) = match operation {
        QuantityArithmetic::Add(left, right) => (left, right, true),
        QuantityArithmetic::Subtract(left, right) => (left, right, false),
    };
    same_unit(left, right)?;
    let arithmetic = if add {
        RationalArithmetic::Add(&left.magnitude, &right.magnitude)
    } else {
        RationalArithmetic::Subtract(&left.magnitude, &right.magnitude)
    };
    let outcome = evaluate_rational_arithmetic(arithmetic, None, meter);
    Ok(match outcome {
        Outcome::Completed(magnitude) => {
            match meter.charge(Charge::new(ChargePoint::UnitResultRetain).results(1)) {
                Ok(()) => Outcome::Completed(Quantity::new(magnitude, left.unit)),
                Err(incomplete) => Outcome::Incomplete(incomplete),
            }
        }
        Outcome::Undefined(reason) => Outcome::Undefined(reason),
        Outcome::Refused(reason) => Outcome::Refused(reason),
        Outcome::Incomplete(record) => Outcome::Incomplete(record),
    })
}

/// One quantity arithmetic operation.
#[derive(Clone, Copy, Debug)]
pub enum QuantityArithmetic<'a> {
    /// `a + b`.
    Add(&'a Quantity, &'a Quantity),
    /// `a - b`.
    Subtract(&'a Quantity, &'a Quantity),
}

/// Compare two quantities in the same unit. Operands in different units are
/// ill-typed and consume nothing.
pub fn compare_quantity(
    operator: ComparisonOperator,
    left: &Quantity,
    right: &Quantity,
) -> Result<bool, IllTyped> {
    same_unit(left, right)?;
    Ok(operator.holds(left.magnitude.cmp(&right.magnitude)))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;
    use crate::accounting::ScalarLimits;
    use crate::integer::Integer;

    fn generous_meter() -> Meter {
        Meter::new(ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        })
    }

    fn unit(byte: u8) -> UnitId {
        let mut bytes = [0_u8; 32];
        bytes[31] = byte;
        UnitId::from_digest(bytes)
    }

    /// TC-319: adding two quantities in the same unit completes with the
    /// summed magnitude in that unit.
    #[trace("TC-319")]
    #[test]
    fn tc_319_same_unit_addition_completes() {
        let metres = unit(1);
        let left = Quantity::new(Rational::from_integer(Integer::one()), metres);
        let right = Quantity::new(Rational::from_integer(Integer::one()), metres);
        let mut meter = generous_meter();
        let outcome =
            evaluate_quantity_arithmetic(QuantityArithmetic::Add(&left, &right), &mut meter)
                .expect("same unit");
        let sum = outcome.completed().expect("charges available");
        assert_eq!(sum.unit(), metres);
        assert_eq!(
            sum.magnitude(),
            &Rational::from_integer(Integer::from(2_u64))
        );
    }

    /// TC-320: comparing or adding quantities of different units is
    /// ill-typed with `DistinctUnits`, before any charge.
    #[trace("TC-320")]
    #[test]
    fn tc_320_distinct_units_are_ill_typed() {
        let metres = Quantity::new(Rational::from_integer(Integer::one()), unit(1));
        let seconds = Quantity::new(Rational::from_integer(Integer::one()), unit(2));
        let err = compare_quantity(ComparisonOperator::Equal, &metres, &seconds).unwrap_err();
        assert_eq!(err.cause, IllTypedCause::DistinctUnits);
    }
}
