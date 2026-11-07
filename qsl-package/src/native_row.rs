// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531: compact refusals and allocation precharges for selected row adapters.

use quire_contract_model::{
    CheckedNodeId, CheckedNodePathSegment, NativeJoinBudget, NativeJoinChargeKind,
    NativeJoinDeniedCharge, NativeJoinLocus, NativeJoinLocusSource,
};
use quire_exact::{EnumShape, Integer, UnitId, ValueType};

/// Stable conditions returned by selected-row materialization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeRowErrorCode {
    /// The caller's original work allowance was exhausted.
    WorkLimit,
    /// The caller's original logical-storage allowance was exhausted.
    StorageLimit,
}

impl NativeRowErrorCode {
    /// Stable spelling of this condition.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorkLimit => "native-row-work-limit",
            Self::StorageLimit => "native-row-storage-limit",
        }
    }

    /// Every condition, in catalog order.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[Self::WorkLimit, Self::StorageLimit]
    }

    /// Decode an exact stable condition spelling.
    #[must_use]
    #[ix_trace_rs::string_edge]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|item| item.as_str() == code)
    }
}

/// A fixed-size selected-row refusal, requiring no diagnostic allocation.
///
/// Budget refusals retain the original owner charge and non-owning locus. The
/// envelope deliberately owns no message, node or context map: it must remain
/// returnable when the caller has no storage allowance.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("native selected row refused: {code:?} at {locus:?}")]
pub struct NativeRowError {
    code: NativeRowErrorCode,
    denied: Option<NativeJoinDeniedCharge>,
    locus: Option<NativeJoinLocus>,
    configured_bound: Option<u64>,
}

impl NativeRowError {
    /// Stable typed condition.
    #[must_use]
    pub fn code(self) -> NativeRowErrorCode {
        self.code
    }

    /// The owner's original first unpaid resource and attempted amount.
    #[must_use]
    pub fn denied(self) -> Option<NativeJoinDeniedCharge> {
        self.denied
    }

    /// The owner's original non-owning denial locus.
    #[must_use]
    pub fn locus(self) -> Option<NativeJoinLocus> {
        self.locus
    }

    /// Original configured ceiling for the owner's denied resource.
    #[must_use]
    pub fn configured_bound(self) -> Option<u64> {
        self.configured_bound
    }

    pub(crate) fn exhausted(denied: NativeJoinDeniedCharge, budget: &NativeJoinBudget) -> Self {
        Self {
            code: match denied.kind {
                NativeJoinChargeKind::Work => NativeRowErrorCode::WorkLimit,
                NativeJoinChargeKind::Storage => NativeRowErrorCode::StorageLimit,
            },
            denied: Some(denied),
            locus: budget.first_unpaid_locus(),
            configured_bound: Some(match denied.kind {
                NativeJoinChargeKind::Work => budget.work_limit(),
                NativeJoinChargeKind::Storage => budget.storage_limit(),
            }),
        }
    }
}

/// Continue an unrefused caller phase without reconstructing its allowance.
pub(crate) fn start(
    budget: &mut NativeJoinBudget,
    form: &'static str,
) -> Result<NativeJoinLocus, NativeRowError> {
    if let Some(denied) = budget.first_unpaid() {
        return Err(NativeRowError::exhausted(denied, budget));
    }
    budget.resume();
    Ok(NativeJoinLocus {
        source: NativeJoinLocusSource::Package,
        form,
    })
}

pub(crate) fn work(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    amount: u128,
) -> Result<(), NativeRowError> {
    charge(budget, locus, NativeJoinChargeKind::Work, amount)
}

fn charge(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    kind: NativeJoinChargeKind,
    amount: u128,
) -> Result<(), NativeRowError> {
    budget
        .accounted_charge(kind, amount, locus)
        .map_err(|denied| NativeRowError::exhausted(denied, budget))
}

/// Logical ownership is one container plus its initialized entries; copied text
/// and numeric payload bytes are charged separately. Shared handles are charged
/// in their containing row, and borrowed inspections own no additional storage.
pub(crate) fn items(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    count: usize,
) -> Result<(), NativeRowError> {
    if count != 0 {
        charge(
            budget,
            locus,
            NativeJoinChargeKind::Storage,
            1_u128.saturating_add(count as u128),
        )?;
    }
    Ok(())
}

pub(crate) fn text(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    value: &str,
) -> Result<(), NativeRowError> {
    work(budget, locus, (value.len() as u128).saturating_add(1))?;
    if !value.is_empty() {
        charge(
            budget,
            locus,
            NativeJoinChargeKind::Storage,
            1_u128.saturating_add(value.len() as u128),
        )?;
    }
    Ok(())
}

pub(crate) fn node(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    value: &CheckedNodeId,
) -> Result<(), NativeRowError> {
    text(budget, locus, &value.package)?;
    items(budget, locus, value.path.len())?;
    for segment in &value.path {
        work(budget, locus, 1)?;
        match segment {
            CheckedNodePathSegment::Name(value) => text(budget, locus, value)?,
            CheckedNodePathSegment::Node(_) | CheckedNodePathSegment::Index(_) => {}
        }
    }
    Ok(())
}

pub(crate) fn enum_shape(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    value: &EnumShape,
) -> Result<(), NativeRowError> {
    let count = value.variants().count();
    items(budget, locus, count)?;
    for variant in value.variants() {
        text(budget, locus, variant.case())?;
    }
    Ok(())
}

pub(crate) fn texts(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    values: &[Box<str>],
) -> Result<(), NativeRowError> {
    items(budget, locus, values.len())?;
    for value in values {
        text(budget, locus, value)?;
    }
    Ok(())
}

fn integer(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    value: &Integer,
) -> Result<(), NativeRowError> {
    work(budget, locus, 1)?;
    let footprint = value.copy_footprint();
    if footprint.bytes != 0 {
        charge(
            budget,
            locus,
            NativeJoinChargeKind::Storage,
            1_u128.saturating_add(footprint.bytes as u128),
        )?;
    }
    Ok(())
}

pub(crate) fn unit(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    value: &UnitId,
) -> Result<(), NativeRowError> {
    match value {
        UnitId::Declared(_) => work(budget, locus, 1),
        UnitId::Compound(terms) => {
            items(budget, locus, terms.len())?;
            for (_, exponent) in terms.iter() {
                integer(budget, locus, exponent)?;
            }
            Ok(())
        }
    }
}

pub(crate) fn value_type(
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
    value: &ValueType,
) -> Result<(), NativeRowError> {
    work(budget, locus, 1)?;
    match value {
        ValueType::Int(value) => {
            integer(budget, locus, value.lower())?;
            integer(budget, locus, value.upper())
        }
        ValueType::Rational(value) => {
            for interval in [value.numerator(), value.denominator()] {
                integer(budget, locus, interval.lower())?;
                integer(budget, locus, interval.upper())?;
            }
            Ok(())
        }
        ValueType::Decimal(value) => {
            integer(budget, locus, value.lower())?;
            integer(budget, locus, value.upper())
        }
        ValueType::Quantity(value) => unit(budget, locus, value),
        ValueType::Enum(value) => enum_shape(budget, locus, value),
        ValueType::Reference(value) => {
            items(budget, locus, value.owner_chain().len())?;
            for original in value.owner_chain() {
                text(budget, locus, original.package())?;
                text(budget, locus, original.node())?;
            }
            Ok(())
        }
        // These payloads are shared through Arc or contain only inline data.
        // Cloning does not traverse or copy the nested value type.
        ValueType::Option(_)
        | ValueType::Collection(_)
        | ValueType::Map(_)
        | ValueType::CarriedQuantity(_)
        | ValueType::BoundedQuantity(_)
        | ValueType::Boolean
        | ValueType::Integer
        | ValueType::Float(_)
        | ValueType::Text(_)
        | ValueType::String
        | ValueType::Uuid
        | ValueType::Timestamp
        | ValueType::Composite(_)
        | ValueType::Population(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    const LOCUS: NativeJoinLocus = NativeJoinLocus {
        source: NativeJoinLocusSource::Package,
        form: "identity_integer_copy",
    };

    #[test]
    #[trace("TC-1331", "FR-531-AC-33")]
    fn cold_signed_integer_copies_pay_the_exact_owned_buffer_before_copying() {
        let _ = allocation_counter::measure(|| {});
        for spelling in [
            "-170141183460469231731687303715884105728",
            "170141183460469231731687303715884105728",
            "-170141183460469231731687303715884105729",
            "340282366920938463463374607431768211457",
            "-340282366920938463463374607431768211457",
        ] {
            let original: Integer = spelling.parse().expect("valid exact integer");
            let cold = original.try_clone().expect("owner-produced cold digits");
            let mut budget = NativeJoinBudget::new(1_000_000, 1_000_000);
            let mut copy = None;
            let observed = allocation_counter::measure(|| {
                integer(&mut budget, LOCUS, &cold).expect("actual payload precharged");
                copy = Some(cold.clone());
            });
            assert_eq!(copy.expect("owned copy"), original);
            let expected = if observed.count_total == 0 {
                0
            } else {
                1 + observed.bytes_total
            };
            assert_eq!(budget.consumed().storage, expected);
            assert_eq!(
                observed.bytes_total as usize,
                original.copy_footprint().bytes
            );
            let cold = original.try_clone().expect("fresh cold control");
            let mut zero = NativeJoinBudget::new(1_000_000, 0);
            let mut refusal = None;
            let observed =
                allocation_counter::measure(|| refusal = Some(integer(&mut zero, LOCUS, &cold)));
            if expected == 0 {
                refusal
                    .expect("call ran")
                    .expect("inline integer needs no storage");
            } else {
                let refused = refusal
                    .expect("call ran")
                    .expect_err("heap buffer must be paid first");
                assert_eq!(refused.code(), NativeRowErrorCode::StorageLimit);
                assert_eq!(refused.configured_bound(), Some(0));
                assert_eq!(refused.locus(), Some(LOCUS));
            }
            assert_eq!(observed.count_total, 0);
            assert_eq!(observed.bytes_total, 0);
        }
    }
}

#[cfg(test)]
mod error_catalog_tests {
    use super::NativeRowErrorCode;

    #[test]
    #[ix_trace_rs::trace("TC-1331", "FR-531-AC-31")]
    fn compact_identity_row_error_codes_have_a_closed_stable_catalog() {
        let expected = [
            (NativeRowErrorCode::WorkLimit, "native-row-work-limit"),
            (NativeRowErrorCode::StorageLimit, "native-row-storage-limit"),
        ];
        assert_eq!(NativeRowErrorCode::all().len(), expected.len());
        for (code, spelling) in expected {
            assert!(NativeRowErrorCode::all().contains(&code));
            assert_eq!(code.as_str(), spelling);
            assert_eq!(NativeRowErrorCode::from_code(spelling), Some(code));
        }
        assert_eq!(NativeRowErrorCode::from_code("work-limit"), None);
        assert_eq!(NativeRowErrorCode::from_code(""), None);
    }
}
