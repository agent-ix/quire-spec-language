// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared helpers for the root integration-test binary.

#[path = "../../examples/config-version/spine_corpus.rs"]
pub mod config_version_spine;
pub mod front_end;
#[path = "refinement_cas.rs"]
pub mod refinement_cas;
pub mod spine_model;
pub mod workspace;

#[path = "admitted_charge.rs"]
mod admitted_charge;
pub(crate) use admitted_charge::admitted_charge_points;

/// A checked-package document's JSON in its resolved form: every FR-322
/// operand slot a node holds replaced by the graph index its row names, so a
/// node's ids are comparable across nodes and edits keep their meaning.
#[cfg(feature = "checked-handoff")]
pub(crate) fn resolved_package(bytes: &[u8]) -> serde_json::Value {
    let mut wire: serde_json::Value = serde_json::from_slice(bytes).expect("a package document");
    qsl_semantics::check::wire_id::resolve_operand_slots(&mut wire)
        .expect("every operand slot names a row entry");
    wire
}

/// The bytes of a document edited in resolved form, with operand slots and
/// rows written for its nodes.
#[cfg(feature = "checked-handoff")]
pub(crate) fn slotted_package_bytes(wire: &serde_json::Value) -> Vec<u8> {
    let mut wire = wire.clone();
    qsl_semantics::check::wire_id::write_operand_rows(&mut wire);
    serde_json::to_vec(&wire).expect("the wire encodes")
}
