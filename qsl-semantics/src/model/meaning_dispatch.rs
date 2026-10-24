// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-208 meaning-to-binder dispatch for the model intake reader.

use std::collections::HashMap;

use serde_json::Value;

use super::domain_package::DomainPackageRecord;
use super::intake::{meaning, PendingFrame};
use super::normalize::Refusals;

/// Shared inputs for a meaning-specific binder. Every binder receives the
/// same admitted occurrence, package-wide meaning index, output records and
/// pending operation frames.
pub(super) struct MeaningBindingContext<'a, 'm> {
    pub(super) package: &'a str,
    pub(super) type_value: &'a Value,
    pub(super) node: &'a str,
    pub(super) at: &'a str,
    pub(super) meanings_by_node: &'a HashMap<&'m str, &'m str>,
    /// The `shape` of the type's own construct, when it declares one.
    pub(super) construct_shape: Option<&'a str>,
    pub(super) records: &'a mut Vec<DomainPackageRecord>,
    pub(super) pending: &'a mut Vec<PendingFrame>,
}

/// Route the FR-208 meanings with dedicated binder modules. `None` leaves
/// other meanings to the existing reader dispatch; every returned refusal
/// retains the binder's typed intake result.
#[ix_trace_rs::string_edge]
pub(super) fn bind(
    meaning_id: &str,
    context: &mut MeaningBindingContext<'_, '_>,
) -> Option<Result<(), Refusals>> {
    Some(match meaning_id {
        meaning::RECORD_VALUE_TYPE => super::record_value::bind(context).map_err(Refusals::from),
        meaning::EVENT_TYPE => super::event::bind(context).map_err(Refusals::from),
        meaning::STATE_MACHINE => super::state_machine::bind(context).map_err(Refusals::from),
        meaning::PROCESS => super::process::bind(context).map_err(Refusals::from),
        meaning::PERSISTENCE_INTERFACE => {
            super::persistence_interface::bind(context).map_err(Refusals::from)
        }
        meaning::NAMESPACE => super::namespace::bind(context).map_err(Refusals::from),
        meaning::VARIANT_TYPE => super::variant_type::bind(context),
        _ => return None,
    })
}
