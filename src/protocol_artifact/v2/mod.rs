// SPDX-License-Identifier: AGPL-3.0-only
//! FR-050: strict compiled-protocol version-2 production and admission.

pub(super) mod intake;
pub(crate) mod refusal;
pub mod wire;

pub use intake::{encode_candidate, read};
pub use refusal::{
    ArtifactField, BindingCause, ClockField, DeclarationField, DefinitionField, HeaderField,
    InventorySide, Refusal,
};

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
    /// Exact registered profile identity.
    pub identity: &'a str,
    /// Exact semantic definition revision.
    pub revision: &'a super::wire::Revision,
    /// Independently selected original definition artifact.
    pub artifact: &'a super::wire::ArtifactRef,
}

/// Independent temporal selection, never derived from offered bytes.
#[derive(Clone, Copy, Debug)]
pub struct ExpectedTemporal<'a> {
    /// Exact original source artifact owning the declaration.
    pub source: &'a super::wire::ArtifactRef,
    /// Independent authored declaration selection.
    pub declaration: &'a ExpectedDeclaration<'a>,
    /// Independent exact definition selection.
    pub definition: ExpectedDefinition<'a>,
    /// Independent exact clock configuration selection.
    pub clock: &'a wire::ClockConfiguration,
}

/// Complete version-2 expectation: inherited selections plus temporal bindings.
#[derive(Clone, Copy, Debug)]
pub struct Expected<'a> {
    /// Complete inherited artifact, source, dependency and model selections.
    pub inherited: super::Expected<'a>,
    /// Complete independent temporal selection table.
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
