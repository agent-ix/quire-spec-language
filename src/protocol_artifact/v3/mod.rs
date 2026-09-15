// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-054: strict compiled-protocol version-3 activation mappings.

mod intake;
pub mod refusal;
pub mod wire;

use crate::protocol_artifact::{v2, wire::Handle};
use crate::ByteDigest;
pub use intake::{encode_candidate, read, read_with_producers};
pub use refusal::{InventorySide, MappingCause, MappingField, Refusal};

/// Exact version-3 payload selection.
pub const WIRE: &str = "quire.compiled-protocol/3";
/// Exact version-3 media selection.
pub const MEDIA: &str = "application/vnd.quire.compiled-protocol+json;version=3";
/// Exact closed version-3 schema selection.
pub const SCHEMA: &str = "quire.compiled-protocol.schema/3";

/// One independently authored control-to-temporal relation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedActivation {
    /// Exact declaration-local event control handle.
    pub control: Handle,
    /// Exact temporal declaration selected by that control.
    pub temporal_declaration: u32,
}

/// Complete version-3 expectation, independently constructed before bytes arrive.
#[derive(Clone, Copy, Debug)]
pub struct Expected<'a> {
    /// Complete inherited v2 selection population.
    pub inherited: v2::Expected<'a>,
    /// Complete authored control-to-temporal relation population.
    pub activations: &'a [ExpectedActivation],
}

/// A constructor-private package admitted against independent selections.
#[derive(Clone, Debug)]
pub struct AdmittedPackage {
    pub(super) package: wire::Package,
    pub(super) digest: ByteDigest,
    pub(super) artifact: Option<crate::protocol_artifact::wire::ArtifactRef>,
    pub(super) model_schema: Vec<crate::native_model::NativeModel>,
}

impl AdmittedPackage {
    /// Read-only admitted version-3 package.
    pub fn package(&self) -> &wire::Package {
        &self.package
    }
    /// Digest of the exact admitted bytes.
    pub fn digest(&self) -> ByteDigest {
        self.digest
    }
    /// Retained independently selected artifact identity, if read rather than emitted.
    pub fn artifact(&self) -> Option<&crate::protocol_artifact::wire::ArtifactRef> {
        self.artifact.as_ref()
    }
    /// Retained independently admitted model schema.
    pub fn schema_model(&self, index: u32) -> Option<&crate::native_model::NativeModel> {
        self.model_schema.get(usize::try_from(index).ok()?)
    }

    /// Complete strict-reader-admitted authored control-to-temporal mapping.
    ///
    /// The returned rows retain declaration-local control handles and temporal
    /// declaration indices exactly as validated; callers cannot infer or
    /// substitute a relation from unrelated package coordinates.
    pub fn activation_mappings(&self) -> &[wire::ActivationMapping] {
        &self.package.activation_mappings
    }
}
