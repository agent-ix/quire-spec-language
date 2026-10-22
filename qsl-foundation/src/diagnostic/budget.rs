// SPDX-License-Identifier: AGPL-3.0-or-later
//! A reached ceiling of a caller's budget: the `resource_exhausted` record
//! FR-255 Behavior 1 gives `ModelNormalizationLimitsV1`,
//! `PopulationAdmissionLimitsV1` (ADR-014 section 1) and the intake byte
//! limit (FR-255 Behavior 1, FR-260 item 5). One builder serves every producer, so the field set is
//! written once.

use std::collections::BTreeMap;

use super::LimitExceeded;
use crate::Setting;
use quire_semantic_value::catalog::CatalogCoded;

/// The stage and accounting contract that own a budget.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BudgetOwner {
    /// The stage whose meter was charged.
    pub stage: &'static str,
    /// The accounting contract that defines the limits.
    pub accounting_contract: &'static str,
}

/// The denied charge a meter ceiling retains.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeniedCharge {
    /// The budget's owner.
    pub owner: BudgetOwner,
    /// The counter whose limit was reached (`work_units`, `ancestor_steps`).
    pub limit_kind: &'static str,
    /// The named point that asked for the charge.
    pub charge_point: &'static str,
    /// The counter's value before the denied charge.
    pub consumed: u64,
    /// The denied amount.
    pub next_charge: u64,
}

/// A caller-budget ceiling that was reached: the setting that raises it, the
/// effective limit, and the count the refused step would have reached.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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

    /// A denied meter charge: `charge.next_charge` more on top of
    /// `charge.consumed` would pass `limit`.
    pub fn charge(setting: Setting, limit: u64, charge: DeniedCharge) -> Self {
        Self {
            setting,
            limit,
            actual: u128::from(charge.consumed) + u128::from(charge.next_charge),
            charge: Some(charge),
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
            ("stage", charge.owner.stage.to_owned()),
            (
                "accounting_contract",
                charge.owner.accounting_contract.to_owned(),
            ),
            ("limit_kind", charge.limit_kind.to_owned()),
            ("consumed", charge.consumed.to_string()),
            ("next_charge", charge.next_charge.to_string()),
            ("charge_point", charge.charge_point.to_owned()),
        ]);
        fields
    }
}
