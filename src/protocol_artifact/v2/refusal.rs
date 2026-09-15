// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-050: stable refusal identities for the strict version-2 boundary.

use std::fmt;

use crate::protocol_artifact::wire::ArtifactRef;

/// The side of the temporal inventory that is incomplete or excessive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventorySide {
    /// The offered package's `temporal_bindings` table.
    Offer,
    /// The caller's independently constructed expected temporal table.
    Expected,
    /// The producer caller's independently selected temporal table.
    Producer,
}

/// A temporal-binding table defect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingCause {
    /// One or more required records are absent.
    Missing,
    /// One or more records exist without a matching temporal declaration.
    Surplus,
    /// Two records carry the same declaration key.
    Duplicate,
}

/// A caller selection whose exact source does not own the selected span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionSide {
    /// The independent reader expectation.
    Expected,
    /// The native producer selection.
    Producer,
}

/// One index member of an offered temporal binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingIndex {
    /// Declaration index.
    Declaration,
    /// Definition index.
    Definition,
}

/// One exact version-2 header selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeaderField {
    /// Payload wire identity/version.
    Wire,
    /// Payload media selection.
    Media,
    /// Closed schema selection.
    Schema,
    /// Payload object type.
    PackageType,
    /// Canonical byte encoding.
    Encoding,
    /// Exact-number profile.
    Numeric,
    /// External artifact kind.
    ArtifactKind,
    /// External artifact wire identity.
    ArtifactWire,
    /// External artifact wire version.
    ArtifactVersion,
}

/// One independently selected declaration member.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationField {
    /// Authored declaration name.
    Name,
    /// Formal requirement owner.
    Requirement,
    /// Formal clause identity.
    Clause,
    /// Authored execution point.
    Execution,
}

/// One exact member of a selected definition artifact reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactField {
    /// Artifact-reference envelope version.
    RefVersion,
    /// Artifact kind.
    Kind,
    /// Artifact authority.
    Authority,
    /// Artifact identity.
    Identity,
    /// Revision namespace.
    RevisionNamespace,
    /// Revision value.
    RevisionValue,
    /// Raw-byte digest; it cannot stand for a canonical producer digest.
    Digest,
    /// Artifact wire identity, which selects the raw-byte interpretation domain.
    WireIdentity,
    /// Artifact wire version.
    WireVersion,
}

/// One exact selected temporal-definition member.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefinitionField {
    /// Registered definition identity.
    Identity,
    /// Registered semantic revision.
    Revision,
    /// Original definition artifact member.
    Artifact(ArtifactField),
}

/// One exact clock-configuration selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockField {
    /// Tagged clock alternative.
    Alternative,
    /// Event-position sequence authority.
    SequenceAuthority,
    /// Fixed-sample epoch.
    Epoch,
    /// Fixed-sample positive period.
    Period,
    /// Fixed-sample unit.
    Unit,
    /// Timestamped-event timestamp unit.
    TimestampUnit,
}

/// A stable machine-readable version-2 refusal.
///
/// The nested field enums are the public discriminants. Callers never need to
/// parse an error message to tell two independently mutable selections apart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Refusal {
    /// A strict version/header selection differs.
    Header(HeaderField),
    /// The temporal inventory or declaration-index table is invalid.
    Binding {
        /// Inventory side relevant to missing/surplus causes.
        side: InventorySide,
        /// Exact structural cause.
        cause: BindingCause,
    },
    /// Offered bindings are not in ascending declaration-index order.
    OfferOrder,
    /// The selected source does not own the declaration span.
    ForeignOwner(SelectionSide),
    /// An offered binding selects a different or out-of-range index.
    OfferIndex(BindingIndex),
    /// An independently selected declaration differs.
    Declaration(DeclarationField),
    /// An independently selected definition differs.
    Definition(DefinitionField),
    /// An independently selected clock configuration differs.
    Clock(ClockField),
}

impl Refusal {
    /// Stable refusal code. Codes are append-only and are never message-derived.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Header(HeaderField::Wire) => "v2.header.wire",
            Self::Header(HeaderField::Media) => "v2.header.media",
            Self::Header(HeaderField::Schema) => "v2.header.schema",
            Self::Header(HeaderField::PackageType) => "v2.header.package-type",
            Self::Header(HeaderField::Encoding) => "v2.header.encoding",
            Self::Header(HeaderField::Numeric) => "v2.header.numeric",
            Self::Header(HeaderField::ArtifactKind) => "v2.header.artifact-kind",
            Self::Header(HeaderField::ArtifactWire) => "v2.header.artifact-wire",
            Self::Header(HeaderField::ArtifactVersion) => "v2.header.artifact-version",
            Self::Binding {
                side: InventorySide::Offer,
                cause: BindingCause::Missing,
            } => "v2.binding.offer-missing",
            Self::Binding {
                side: InventorySide::Offer,
                cause: BindingCause::Surplus,
            } => "v2.binding.offer-surplus",
            Self::Binding {
                side: InventorySide::Expected,
                cause: BindingCause::Missing,
            } => "v2.binding.expected-missing",
            Self::Binding {
                side: InventorySide::Expected,
                cause: BindingCause::Surplus,
            } => "v2.binding.expected-surplus",
            Self::Binding {
                side: InventorySide::Producer,
                cause: BindingCause::Missing,
            } => "v2.binding.producer-missing",
            Self::Binding {
                side: InventorySide::Producer,
                cause: BindingCause::Surplus,
            } => "v2.binding.producer-surplus",
            Self::Binding {
                side: InventorySide::Offer,
                cause: BindingCause::Duplicate,
            } => "v2.binding.offer-duplicate",
            Self::Binding {
                side: InventorySide::Expected,
                cause: BindingCause::Duplicate,
            } => "v2.binding.expected-duplicate",
            Self::Binding {
                side: InventorySide::Producer,
                cause: BindingCause::Duplicate,
            } => "v2.binding.producer-duplicate",
            Self::OfferOrder => "v2.binding.offer-order",
            Self::ForeignOwner(SelectionSide::Expected) => "v2.binding.expected-foreign-owner",
            Self::ForeignOwner(SelectionSide::Producer) => "v2.binding.producer-foreign-owner",
            Self::OfferIndex(BindingIndex::Declaration) => "v2.binding.offer-declaration-index",
            Self::OfferIndex(BindingIndex::Definition) => "v2.binding.offer-definition-index",
            Self::Declaration(DeclarationField::Name) => "v2.declaration.name",
            Self::Declaration(DeclarationField::Requirement) => "v2.declaration.requirement",
            Self::Declaration(DeclarationField::Clause) => "v2.declaration.clause",
            Self::Declaration(DeclarationField::Execution) => "v2.declaration.execution",
            Self::Definition(DefinitionField::Identity) => "v2.definition.identity",
            Self::Definition(DefinitionField::Revision) => "v2.definition.revision",
            Self::Definition(DefinitionField::Artifact(ArtifactField::RefVersion)) => {
                "v2.definition.artifact.ref-version"
            }
            Self::Definition(DefinitionField::Artifact(ArtifactField::Kind)) => {
                "v2.definition.artifact.kind"
            }
            Self::Definition(DefinitionField::Artifact(ArtifactField::Authority)) => {
                "v2.definition.artifact.authority"
            }
            Self::Definition(DefinitionField::Artifact(ArtifactField::Identity)) => {
                "v2.definition.artifact.identity"
            }
            Self::Definition(DefinitionField::Artifact(ArtifactField::RevisionNamespace)) => {
                "v2.definition.artifact.revision-namespace"
            }
            Self::Definition(DefinitionField::Artifact(ArtifactField::RevisionValue)) => {
                "v2.definition.artifact.revision-value"
            }
            Self::Definition(DefinitionField::Artifact(ArtifactField::Digest)) => {
                "v2.definition.artifact.digest"
            }
            Self::Definition(DefinitionField::Artifact(ArtifactField::WireIdentity)) => {
                "v2.definition.artifact.wire-identity"
            }
            Self::Definition(DefinitionField::Artifact(ArtifactField::WireVersion)) => {
                "v2.definition.artifact.wire-version"
            }
            Self::Clock(ClockField::Alternative) => "v2.clock.alternative",
            Self::Clock(ClockField::SequenceAuthority) => "v2.clock.sequence-authority",
            Self::Clock(ClockField::Epoch) => "v2.clock.epoch",
            Self::Clock(ClockField::Period) => "v2.clock.period",
            Self::Clock(ClockField::Unit) => "v2.clock.unit",
            Self::Clock(ClockField::TimestampUnit) => "v2.clock.timestamp-unit",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

pub(crate) fn artifact_field(
    actual: &ArtifactRef,
    expected: &ArtifactRef,
) -> Option<ArtifactField> {
    if actual.ref_version != expected.ref_version {
        Some(ArtifactField::RefVersion)
    } else if actual.kind != expected.kind {
        Some(ArtifactField::Kind)
    } else if actual.authority != expected.authority {
        Some(ArtifactField::Authority)
    } else if actual.identity != expected.identity {
        Some(ArtifactField::Identity)
    } else if actual.revision.namespace != expected.revision.namespace {
        Some(ArtifactField::RevisionNamespace)
    } else if actual.revision.value != expected.revision.value {
        Some(ArtifactField::RevisionValue)
    } else if actual.digest != expected.digest {
        Some(ArtifactField::Digest)
    } else if actual.wire.identity != expected.wire.identity {
        Some(ArtifactField::WireIdentity)
    } else if actual.wire.version != expected.wire.version {
        Some(ArtifactField::WireVersion)
    } else {
        None
    }
}
