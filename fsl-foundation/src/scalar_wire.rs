// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical decimal integers and structural unit descriptors shared by keys and proofs.

use alloc::string::ToString;
use filament_exact::{
    Integer, IntegerDecimalBudget, IntegerDecimalBudgetError, IntegerDecimalError, UnitId,
};
use quire_canonical::{Error, Sink, Writer};

/// Write the canonical decimal-string integer under a caller's admitted capacity.
///
/// # Errors
/// The caller's decimal budget, byte bound or allocation may refuse conversion.
pub fn integer<S: Sink + ?Sized>(
    value: &Integer,
    max_bytes: u64,
    budget: Option<&mut dyn IntegerDecimalBudget>,
    writer: &mut Writer<'_, S>,
) -> Result<(), Error> {
    let limit = usize::try_from(max_bytes).unwrap_or(usize::MAX);
    let rendered = match budget {
        Some(budget) => value.try_to_decimal_string_with_budget(limit, budget),
        None => value
            .try_to_decimal_string(limit)
            .map_err(IntegerDecimalBudgetError::Decimal),
    }
    .map_err(|error| match error {
        IntegerDecimalBudgetError::Denied => Error::ReservationDenied { requested: 0 },
        IntegerDecimalBudgetError::Decimal(IntegerDecimalError::Allocation { requested }) => {
            Error::Allocation { requested }
        }
        IntegerDecimalBudgetError::Decimal(IntegerDecimalError::ByteLimit { required, limit }) => {
            Error::Limit(quire_canonical::LimitExceeded {
                kind: quire_canonical::LimitKind::CanonicalBytes,
                bound: u64::try_from(limit).unwrap_or(u64::MAX),
                required: u64::try_from(required).unwrap_or(u64::MAX),
            })
        }
        IntegerDecimalBudgetError::Decimal(IntegerDecimalError::OutputSizeOverflow) => {
            Error::Allocation {
                requested: usize::MAX,
            }
        }
    })?;
    writer.string(&rendered)
}

/// Write a declared or compound unit; the caller owns integer charging and cancellation.
///
/// # Errors
/// Structural unit failure or any refusal of the canonical writer/integer callback.
pub fn unit<S: Sink + ?Sized>(
    unit: Option<&UnitId>,
    writer: &mut Writer<'_, S>,
    mut integer: impl FnMut(&Integer, &mut Writer<'_, S>) -> Result<(), Error>,
) -> Result<(), Error> {
    let Some(unit) = unit else {
        return writer.null();
    };
    writer.begin_array()?;
    if let Some(node) = unit.declared_key() {
        writer.string("declared")?;
        crate::node_wire::encode_node_key(node, writer)?;
    } else {
        let terms = unit
            .compound_terms()
            .ok_or_else(|| Error::Serialize("unit has no structural descriptor".to_string()))?;
        writer.string("compound")?;
        writer.begin_array()?;
        for (node, exponent) in terms {
            writer.begin_object()?;
            writer.name("unit")?;
            crate::node_wire::encode_node_key(node, writer)?;
            writer.name("exponent")?;
            integer(exponent, writer)?;
            writer.end_object()?;
        }
        writer.end_array()?;
    }
    writer.end_array()
}
