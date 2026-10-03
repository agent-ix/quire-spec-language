// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec's native diagnostics catalog, named by identity.
//!
//! [`native_diagnostics_catalog`] is the `DefinitionRef` a checked package's
//! diagnostics are qualified by: QSpec's `native-diagnostics.md`, authority
//! `agent-ix` and identity [`NATIVE_DIAGNOSTICS_IDENTITY`] (FR-093-AC-17). A
//! definition is named by its identity alone, so no byte of the document is
//! read.

use super::definition::DefinitionReference;

/// The identity of QSpec's `native-diagnostics.md` interpretation catalog.
pub const NATIVE_DIAGNOSTICS_IDENTITY: &str = "quire.native.diagnostics/v1";

/// QSpec's `quire.native.diagnostics/v1` catalog as the `DefinitionRef` a
/// checked package's diagnostics are qualified by.
pub fn native_diagnostics_catalog() -> DefinitionReference {
    DefinitionReference {
        authority: "agent-ix".to_owned(),
        identity: NATIVE_DIAGNOSTICS_IDENTITY.to_owned(),
    }
}
