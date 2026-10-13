// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-208 meaning-to-binder dispatch for the model intake reader.

use std::collections::HashMap;

use serde_json::Value;

use super::domain_package::DomainPackageRecord;
use super::intake::{meaning, PendingFrame};
use super::normalize::ModelRefusal;

/// Shared inputs for a meaning-specific binder. Every binder receives the
/// same admitted occurrence, package-wide meaning index, output records and
/// pending operation frames.
pub(super) struct MeaningBindingContext<'a, 'm> {
    pub(super) package: &'a str,
    pub(super) type_value: &'a Value,
    pub(super) node: &'a str,
    pub(super) at: &'a str,
    pub(super) type_meanings: &'a HashMap<&'m str, &'m str>,
    pub(super) records: &'a mut Vec<DomainPackageRecord>,
    pub(super) pending: &'a mut Vec<PendingFrame>,
}

/// Route the FR-208 meanings with dedicated binder modules. `None` leaves
/// other meanings to the existing reader dispatch; every returned refusal
/// retains the binder's typed intake result.
#[qsl_attrs::string_edge]
pub(super) fn bind(
    meaning_id: &str,
    context: &mut MeaningBindingContext<'_, '_>,
) -> Option<Result<(), ModelRefusal>> {
    Some(match meaning_id {
        meaning::RECORD_VALUE_TYPE => super::record_value::bind(context),
        meaning::EVENT_TYPE => super::event::bind(context),
        meaning::STATE_MACHINE => super::state_machine::bind(context),
        meaning::PROCESS => super::process::bind(context),
        meaning::PERSISTENCE_INTERFACE => super::persistence_interface::bind(context),
        meaning::NAMESPACE => super::namespace::bind(context),
        _ => return None,
    })
}
