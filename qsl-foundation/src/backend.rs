// SPDX-License-Identifier: AGPL-3.0-or-later
//! The provider backend identity shared by routing and replay envelopes.

use alloc::string::String;
use core::fmt;

/// A provider's exact backend identity (FR-288, ADR-013 O-19).
///
/// The identity is opaque data. It is kept verbatim from the provider
/// manifest and is the same value in routing candidates and replay envelopes.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct BackendId(String);

impl BackendId {
    /// Build an identity from its exact wire spelling.
    pub fn new(identity: impl Into<String>) -> Self {
        Self(identity.into())
    }

    /// Read an identity from a wire `backend` member.
    pub fn from_wire(identity: &str) -> Result<Self, EmptyBackendIdentity> {
        if identity.is_empty() {
            Err(EmptyBackendIdentity)
        } else {
            Ok(Self::new(identity))
        }
    }

    /// The identity's exact wire spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// [`BackendId::from_wire`]'s refusal: the `backend` member is empty.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("invalid_identifier: the backend identity is empty")]
pub struct EmptyBackendIdentity;

impl EmptyBackendIdentity {
    /// The catalog code of this refusal.
    pub const fn code(&self) -> crate::Code {
        crate::Code::InvalidIdentifier
    }

    /// The refusal category.
    pub const fn category(&self) -> quire_semantic_value::catalog::Category {
        quire_semantic_value::catalog::Category::Refusal
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
