// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared helpers for `qsl-eval`'s `tests/it/*`, reached as
//! `crate::support::<name>`.

pub mod model;
pub mod type_form;

#[path = "../../../../tests/support/admitted_charge.rs"]
mod admitted_charge;
pub(crate) use admitted_charge::admitted_charge_points;
