// SPDX-License-Identifier: AGPL-3.0-only
//! FR-050: strict compiled-protocol version-2 production and admission.

pub(super) mod intake;
pub(crate) mod refusal;
pub mod wire;

pub use intake::{encode_candidate, read, read_with_producers};
pub use refusal::{
    ArtifactField, BindingCause, BindingIndex, ClockField, DeclarationField, DefinitionField,
    HeaderField, InventorySide, Refusal, SelectionSide,
};

use super::wire::ArtifactRef;
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
#[derive(Clone, Debug)]
pub struct AdmittedPackage {
    pub(super) package: wire::Package,
    pub(super) digest: ByteDigest,
    // Present only on the reader path, which admits an independently selected
    // artifact identity. Compiler emission has no published identity yet.
    pub(super) artifact: Option<ArtifactRef>,
    pub(super) model_schema: Vec<crate::native_model::NativeModel>,
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

    /// Independently admitted model schema retained for this model-table entry.
    pub fn schema_model(&self, index: u32) -> Option<&crate::native_model::NativeModel> {
        self.model_schema.get(usize::try_from(index).ok()?)
    }

    /// Independently selected compiled artifact identity admitted by the reader.
    ///
    /// This is `None` for a compiler emission, which produces bytes and their
    /// raw-byte digest before any caller publishes an artifact identity for
    /// them. It is never synthesized from the payload.
    pub fn artifact(&self) -> Option<&ArtifactRef> {
        self.artifact.as_ref()
    }
}
