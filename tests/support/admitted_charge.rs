// SPDX-License-Identifier: AGPL-3.0-or-later
//! Project admitted-charge records to their public charge-point values.

pub(crate) fn admitted_charge_points(meter: &quire_exact::Meter) -> Vec<quire_exact::ChargePoint> {
    meter
        .admitted_charges()
        .iter()
        .map(|charge| charge.point)
        .collect()
}
