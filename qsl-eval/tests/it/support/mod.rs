// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared helpers for `qsl-eval`'s `tests/it/*`, reached as
//! `crate::support::<name>`.

pub mod model;
pub mod type_form;

pub(crate) fn admitted_charge_points(meter: &quire_exact::Meter) -> Vec<quire_exact::ChargePoint> {
    meter
        .admitted_charges()
        .iter()
        .map(|charge| charge.point)
        .collect()
}
