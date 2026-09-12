// SPDX-License-Identifier: AGPL-3.0-only
//! FR-050: strict compiled-protocol version-2 production and admission.

pub(super) mod intake;
pub mod wire;

pub use intake::{encode_candidate, read};

use super::ExpectedDeclaration;
use crate::ByteDigest;

/// Exact version-2 payload selection.
pub const WIRE: &str = "quire.compiled-protocol/2";
/// Exact version-2 media selection.
pub const MEDIA: &str = "application/vnd.quire.compiled-protocol+json;version=2";
/// Exact closed version-2 schema selection.
pub const SCHEMA: &str = "quire.compiled-protocol.schema/2";

/// Independent expected definition selection for one temporal declaration.
#[derive(Clone, Copy, Debug)]
pub struct ExpectedDefinition<'a> {
    pub identity: &'a str,
    pub revision: &'a super::wire::Revision,
    pub artifact: &'a super::wire::ArtifactRef,
}

/// Independent temporal selection, never derived from offered bytes.
#[derive(Clone, Copy, Debug)]
pub struct ExpectedTemporal<'a> {
    pub source: &'a super::wire::ArtifactRef,
    pub declaration: &'a ExpectedDeclaration<'a>,
    pub definition: ExpectedDefinition<'a>,
    pub clock: &'a wire::ClockConfiguration,
}

/// Complete version-2 expectation: inherited selections plus temporal bindings.
#[derive(Clone, Copy, Debug)]
pub struct Expected<'a> {
    pub inherited: super::Expected<'a>,
    pub temporal: &'a [ExpectedTemporal<'a>],
}

/// A version-2 package admitted against independent selections.
#[derive(Debug)]
pub struct AdmittedPackage {
    pub(super) package: wire::Package,
    pub(super) digest: ByteDigest,
}

impl AdmittedPackage {
    /// Read-only admitted version-2 package.
    pub fn package(&self) -> &wire::Package {
        &self.package
    }

    /// The inherited version-1-shaped graph retained without translation.
    pub fn inherited(&self) -> &super::wire::Package {
        &self.package.inherited
    }

    /// Digest of the exact admitted version-2 bytes.
    pub fn digest(&self) -> ByteDigest {
        self.digest
    }
}
