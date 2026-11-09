// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531: the evaluation entry over a checked package read from its bytes
//! alone. [`BarePackage`] implements [`crate::value::CheckedPackageEvaluation`]
//! for the protocol-slot call (FR-533) and gives
//! [`crate::temporal::evidence::EvidencePackage`] its `from_package` constructor.

pub use crate::value::bare_evaluate_selected_protocol_slot as evaluate_selected_protocol_slot;
pub use qsl_package::{
    BarePackage, BareSelectionFailure, BareSlot, BareSupply, ReadEqualityRows, ReadIdentityRows,
};
