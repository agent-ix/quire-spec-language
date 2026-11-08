// SPDX-License-Identifier: AGPL-3.0-or-later
//! `planner::read_premise_item` (QSL FR-529): the strict reader of a premise's
//! FR-331 request item. It reads exactly the slot-key members of QSpec FR-492
//! "Slot key" (subject, claim, kind, `item_kind`, fairness, `stutter_closure`,
//! `correspondence`, `premises`, extent classification) and the item's
//! `request_index`; a member that is not a slot-key member (limits,
//! candidates, options, ...) is skipped, and a malformed slot-key member is
//! refused.

use serde_json::Value;

use super::json::{count, object, require, string, Fault};
use super::key::{read_item_key, ItemKey};
use crate::proof_result::child;

/// A premise's request item as read: its slot key, its position in the
/// request and, for a `bounded` extent, the substituted bounds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PremiseItem {
    /// The slot key the item has.
    pub key: ItemKey,
    /// The item's `request_index`, the join key of its result entry.
    pub request_index: u64,
    /// The `ProofBound`s of a `bounded` extent, in order, as written; empty
    /// for an `unbounded` extent.
    pub bounds: Vec<Value>,
}

/// Read a premise's FR-331 request item.
///
/// # Errors
///
/// [`Fault`] with the member's RFC 6901 pointer relative to `item`:
/// `wrong-value-kind` for a member of the wrong JSON kind,
/// `missing-member` for an absent slot-key member and `invalid-value` for a
/// slot-key member outside its domain, including a `temporal`,
/// `correspondence` or `premises` member on an item of another kind.
#[ix_trace_rs::string_edge]
pub fn read_premise_item(item: &Value) -> Result<PremiseItem, Fault> {
    let members = object(item, "")?;
    let request_index = count(require(members, "request_index", "")?, "/request_index")?;
    let key = read_item_key(members, "")?;
    let extent_pointer = child("", "extent");
    let extent = object(require(members, "extent", "")?, &extent_pointer)?;
    let bounds = match extent.get("bounds") {
        None => Vec::new(),
        Some(bounds) => {
            let pointer = child(&extent_pointer, "bounds");
            let class = string(
                require(extent, "classification", &extent_pointer)?,
                &child(&extent_pointer, "classification"),
            )?;
            if class != "bounded" {
                return Err(Fault::invalid(pointer));
            }
            super::json::array(bounds, &pointer)?.clone()
        }
    };
    Ok(PremiseItem {
        key,
        request_index,
        bounds,
    })
}
