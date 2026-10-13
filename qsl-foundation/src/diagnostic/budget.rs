// SPDX-License-Identifier: AGPL-3.0-or-later
//! A reached ceiling of a caller's budget: the `resource_exhausted` record
//! FR-255 Behavior 1 gives `ModelNormalizationLimitsV1`,
//! `PopulationAdmissionLimitsV1` and the intake byte limit (ADR-014
//! section 1). One builder serves every producer, so the field set is
//! written once.

use std::collections::BTreeMap;

use super::{CatalogCoded, LimitExceeded};
use crate::Setting;

/// The denied charge a meter ceiling retains: the stage and accounting
/// contract that owns the budget, what was consumed before it, the amount
/// denied and the named point that asked.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeniedCharge {
    stage: &'static str,
    accounting_contract: &'static str,
    limit_kind: &'static str,
    consumed: u64,
    next_charge: u64,
    charge_point: &'static str,
}

/// A caller-budget ceiling that was reached: the setting that raises it, the
/// effective limit, and the count the refused step would have reached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BudgetDenial {
    setting: Setting,
    limit: u64,
    actual: u128,
    charge: Option<DeniedCharge>,
}

impl BudgetDenial {
    /// A size ceiling (intake's document bytes): no charge is retained, and
    /// the record is the one a stage limit carries (`kind`, `bound`,
    /// `actual`, `setting`; FR-255 Behavior 1).
    pub fn size(setting: Setting, limit: u64, actual: u128) -> Self {
        Self {
            setting,
            limit,
            actual,
            charge: None,
        }
    }

    /// A denied meter charge: `next_charge` more on top of `consumed` would
    /// pass `limit`. `stage` and `accounting_contract` name the budget's
    /// owner, `limit_kind` the counter (`work_units`, `ancestor_steps`) and
    /// `charge_point` the point that asked.
    #[allow(clippy::too_many_arguments)]
    pub fn charge(
        setting: Setting,
        limit: u64,
        consumed: u64,
        next_charge: u64,
        stage: &'static str,
        accounting_contract: &'static str,
        limit_kind: &'static str,
        charge_point: &'static str,
    ) -> Self {
        Self {
            setting,
            limit,
            actual: u128::from(consumed) + u128::from(next_charge),
            charge: Some(DeniedCharge {
                stage,
                accounting_contract,
                limit_kind,
                consumed,
                next_charge,
                charge_point,
            }),
        }
    }

    /// The setting that raises the limit.
    pub fn setting(&self) -> Setting {
        self.setting
    }

    /// The effective limit.
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// The count the refused step would have reached.
    pub fn actual(&self) -> u128 {
        self.actual
    }

    /// The catalog payload (FR-255 Behavior 1). A size ceiling carries
    /// `kind`, `bound`, `actual` and `setting`. A denied charge
    /// (`insufficient-next-charge`) carries `setting`, `limit` (FR-272's
    /// canonical name for the effective limit), `actual`, `stage`,
    /// `accounting_contract`, `limit_kind`, `consumed`, `next_charge` and `charge_point`.
    pub fn catalog_fields(&self) -> BTreeMap<&'static str, String> {
        let Some(charge) = &self.charge else {
            return LimitExceeded::new(self.setting, self.limit, self.actual)
                .catalog_fields()
                .unwrap_or_default();
        };
        let mut fields = BTreeMap::from([
            ("setting", self.setting.name().to_owned()),
            ("limit", self.limit.to_string()),
            ("actual", self.actual.to_string()),
        ]);
        fields.extend([
            ("stage", charge.stage.to_owned()),
            ("accounting_contract", charge.accounting_contract.to_owned()),
            ("limit_kind", charge.limit_kind.to_owned()),
            ("consumed", charge.consumed.to_string()),
            ("next_charge", charge.next_charge.to_string()),
            ("charge_point", charge.charge_point.to_owned()),
        ]);
        fields
    }
}
