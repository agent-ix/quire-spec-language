// SPDX-License-Identifier: AGPL-3.0-only
//! FR-038/042: exact compiled protocol data and bounded consumer validation.
//!
//! The wire records represent [the compiled protocol contract][contract].
//! They are freely constructible, untrusted transport data. Reader admission
//! checks those data against independent selections; it does not prove that a
//! source compiler produced them. Native family emission requires its own
//! completed compilation authority and is not provided by `encode_candidate`.
//!
//! [contract]: https://github.com/agent-ix/quire-spec-language/blob/main/docs/compiled-protocol-v1.md

mod decode;
mod encoding;
mod intake;
mod models;
pub mod native;
mod number;
mod recovery;
mod validate;
mod value_graph;
pub mod wire;
mod work;

pub use number::{
    ExactInteger, ExactRational, NumberComponent, NumberError, NumberWire, ProtocolNumber,
    NUMERIC_PROFILE,
};

pub use encoding::encode_candidate;
pub use intake::read;
pub use work::{Dimension, Exhaustion, Limits, Usage, ACCOUNTING_VERSION};

use crate::{native_model::NativeModel, ByteDigest};

/// Payload wire version; the external reference retains its separate wire pair.
pub const WIRE: &str = "quire.compiled-protocol/1";
/// Exact media selection in the payload header.
pub const MEDIA: &str = "application/vnd.quire.compiled-protocol+json;version=1";
/// Exact closed payload schema selection.
pub const SCHEMA: &str = "quire.compiled-protocol.schema/1";
/// Exact payload type selection.
pub const PACKAGE_TYPE: &str = "CompiledProtocolPackage";
/// Fixed-member-order CompactFormatter byte encoding, with no normalization.
pub const ENCODING: &str = "quire.protocol.compact-json/1";

/// One independently supplied immutable dependency and its exact direct closure.
#[derive(Clone, Copy, Debug)]
pub struct SuppliedDependency<'a> {
    /// Independently selected role, identity, revision, wire and raw-byte digest.
    pub artifact: &'a wire::ArtifactRef,
    /// Complete original artifact bytes, never canonically reconstructed.
    pub bytes: &'a [u8],
    /// Exact direct prerequisites; caller order need not be canonical wire order.
    pub requires: &'a [wire::ArtifactRef],
}

/// Exact external source selection for a model's original foreign loci.
#[derive(Clone, Copy, Debug)]
pub struct ExpectedForeignSource<'a> {
    /// Independently selected source artifact reference.
    pub artifact: &'a wire::ArtifactRef,
    /// Authored formal identity, distinct from the external source reference.
    pub formal: &'a wire::Formal,
    /// Original UTF-8 source bytes corresponding to the admitted model.
    pub bytes: &'a [u8],
}

/// Existing constructor-private model authority, separate from a wire export.
#[derive(Clone, Copy, Debug)]
pub struct AdmittedModel<'a> {
    /// Exact selected model-package reference, with no inferred namespace mapping.
    pub artifact: &'a wire::ArtifactRef,
    /// Existing native producer's admitted declarations, roles and artifact.
    pub model: &'a NativeModel,
    /// Independent source/formal correspondence for original model loci.
    pub source: &'a ExpectedForeignSource<'a>,
}

/// Independently selected authored declaration correspondence.
/// Equality with this input does not authenticate how its caller obtained it.
#[derive(Clone, Copy, Debug)]
pub struct ExpectedDeclaration<'a> {
    /// Authored name, without namespace substitution.
    pub name: &'a str,
    /// Half-open offsets into its exact expected source bytes.
    pub span: &'a wire::Span,
    /// Explicit formal requirement owner, never minted from a source label.
    pub requirement: &'a wire::Requirement,
    /// Explicit clause identity under that owner.
    pub clause: &'a str,
    /// Authored execution selection.
    pub execution: &'a wire::Execution,
}

/// Complete source input, retaining all three independent identity namespaces.
#[derive(Clone, Copy, Debug)]
pub struct ExpectedSource<'a> {
    /// Independently selected original source artifact.
    pub artifact: &'a wire::ArtifactRef,
    /// Original editable labels, independent of artifact/formal revisions.
    pub native: &'a wire::NativeSource,
    /// Original display path; it supplies no semantic authority.
    pub path: &'a str,
    /// Separately authored formal document and revision.
    pub formal: &'a wire::Formal,
    /// Exact original text, never reparsed by this reader.
    pub text: &'a str,
    /// Complete independently selected declaration inventory for this source.
    pub declarations: &'a [ExpectedDeclaration<'a>],
}

/// Independent acceptance selections. Nothing here is obtained from the offer.
#[derive(Clone, Copy, Debug)]
pub struct Expected<'a> {
    /// External compiled-artifact seal selected before inspecting the offer.
    pub artifact: &'a wire::ArtifactRef,
    /// Accepted compiled-protocol contract artifact.
    pub contract: &'a wire::ArtifactRef,
    /// Accepted semantic baseline artifact.
    pub baseline: &'a wire::ArtifactRef,
    /// Accepted producer implementation, revision and exact binary artifact.
    pub producer: &'a wire::Producer,
    /// Accepted native language edition, separate from definition revision.
    pub language: &'a wire::Language,
    /// Complete source inventory.
    pub sources: &'a [ExpectedSource<'a>],
    /// Complete dependency inventory; omitted entries are never discovered.
    pub dependencies: &'a [SuppliedDependency<'a>],
    /// Actual admitted native views for every model dependency retained as a model.
    pub models: &'a [AdmittedModel<'a>],
}

/// A recognized authority or feature lacking a supported artifact interpretation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Unsupported {
    Wire,
    Feature,
    Definition,
    Profile,
    ProducerCorrespondence,
    /// A native family obligation has no supplied supported proof interpretation.
    FamilyProof,
    Export,
}

/// Invalid data classified independently of human-readable diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Invalid {
    Selection,
    Seal,
    Name,
    StructuralInteger,
    WrongNumericKind,
    NumericDomain,
    Inventory,
    Order,
    Duplicate,
    Dependency,
    Definition,
    Model,
    ForeignLocus,
    Locus,
    Reference,
    Owner,
    Scope,
    Type,
    Profile,
    Call,
    Binding,
    Control,
    Cycle,
    Feature,
    Canonical,
    Encoding,
}

/// A single bounded refusal; exhausted work never returns a partial admission.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The allocator refused a reservation already bounded by the selected limits.
    #[error("compiled protocol allocation failed")]
    Allocation,
    /// Closed JSON shape refusal, with original byte-oriented line and column.
    #[error("invalid compiled protocol JSON at line {line}, column {column}")]
    Json { line: usize, column: usize },
    #[error(transparent)]
    Numeric(NumberError),
    #[error("invalid compiled protocol data: {0:?}")]
    Invalid(Invalid),
    #[error("unsupported compiled protocol interpretation: {0:?}")]
    Unsupported(Unsupported),
    #[error(transparent)]
    Incomplete(Exhaustion),
}

impl From<Exhaustion> for Error {
    fn from(value: Exhaustion) -> Self {
        Self::Incomplete(value)
    }
}

impl From<NumberError> for Error {
    fn from(value: NumberError) -> Self {
        Self::Numeric(value)
    }
}

/// Effective limits and successful work accompany either outcome.
#[derive(Debug)]
pub struct Report<T> {
    result: Result<T, Error>,
    limits: Limits,
    usage: Usage,
    locus: Option<wire::Locus>,
}

impl<T> Report<T> {
    /// Borrow the complete admission or its single typed refusal.
    pub fn result(&self) -> Result<&T, &Error> {
        self.result.as_ref()
    }
    /// Consume the report, transferring its complete result.
    pub fn into_result(self) -> Result<T, Error> {
        self.result
    }
    /// Effective independently clamped caller ceilings.
    pub fn limits(&self) -> Limits {
        self.limits
    }
    /// Successful work before completion or the refused next step.
    pub fn usage(&self) -> Usage {
        self.usage
    }
    /// Most specific original source region available at the terminal step.
    pub fn locus(&self) -> Option<&wire::Locus> {
        self.locus.as_ref()
    }
    /// Version interpreting every retained usage counter.
    pub fn accounting_version(&self) -> &'static str {
        ACCOUNTING_VERSION
    }
}

fn report<T>(work: work::Work, result: Result<T, Error>) -> Report<T> {
    // The terminal enum and locus are inline: no new diagnostic allocation, and
    // no later quota check can overwrite an already established refusal.
    Report {
        result,
        limits: work.limits,
        usage: work.usage,
        locus: work.locus,
    }
}

/// Canonical transport bytes, without native compilation or emission authority.
#[derive(Debug)]
pub struct Candidate {
    bytes: Vec<u8>,
    digest: ByteDigest,
}

impl Candidate {
    /// Exact canonical candidate bytes; this is not a production emission proof.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// SHA-256 of those complete raw bytes.
    pub fn digest(&self) -> ByteDigest {
        self.digest
    }
}

/// A wire package admitted against independent selections and actual model views.
/// This is consumer-side data integrity, not proof of native source compilation.
#[derive(Debug)]
pub struct AdmittedPackage {
    package: wire::Package,
    digest: ByteDigest,
}

impl AdmittedPackage {
    /// Read-only wire data admitted against the supplied selection context.
    pub fn package(&self) -> &wire::Package {
        &self.package
    }
    /// Raw-byte digest that matched the independently expected external seal.
    pub fn digest(&self) -> ByteDigest {
        self.digest
    }
}
