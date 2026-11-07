// SPDX-License-Identifier: AGPL-3.0-or-later
//! The declaration handle a native source namespace issues.

/// Declaration handle local to one namespace, not a cross-package identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DeclarationId(pub(crate) usize);

impl DeclarationId {
    /// Index into this namespace's declarations(), not a unit's AST.
    pub fn index(self) -> usize {
        self.0
    }
}
