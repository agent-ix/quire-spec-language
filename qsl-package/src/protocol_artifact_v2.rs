// SPDX-License-Identifier: AGPL-3.0-or-later
//! Version 2 refusal-code vocabulary (FR-513-AC-17).

/// Stable refusal code for the closed version 2 header and binding vocabulary.
///
/// This table is the one source of the `v2.header.*` and `v2.binding.*`
/// spellings; the legacy v2 reader that once mapped onto it is deleted.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum RefusalCode {
    /// Payload wire identity/version mismatch.
    HeaderWire,
    /// Payload media selection mismatch.
    HeaderMedia,
    /// Closed schema selection mismatch.
    HeaderSchema,
    /// Payload object type mismatch.
    HeaderPackageType,
    /// Canonical byte encoding mismatch.
    HeaderEncoding,
    /// Exact-number profile mismatch.
    HeaderNumeric,
    /// External artifact kind mismatch.
    HeaderArtifactKind,
    /// External artifact wire identity mismatch.
    HeaderArtifactWire,
    /// External artifact wire version mismatch.
    HeaderArtifactVersion,
    /// Offered temporal inventory is missing a required binding.
    BindingOfferMissing,
    /// Offered temporal inventory contains an unselected binding.
    BindingOfferSurplus,
    /// Independent expected temporal inventory is incomplete.
    BindingExpectedMissing,
    /// Independent expected temporal inventory contains a surplus row.
    BindingExpectedSurplus,
    /// Producer temporal inventory is incomplete.
    BindingProducerMissing,
    /// Producer temporal inventory contains a surplus row.
    BindingProducerSurplus,
    /// Offered temporal inventory repeats a binding.
    BindingOfferDuplicate,
    /// Expected temporal inventory repeats a binding.
    BindingExpectedDuplicate,
    /// Producer temporal inventory repeats a binding.
    BindingProducerDuplicate,
    /// Offered binding's clock name disagrees with its legacy binding.
    BindingOfferClockName,
    /// Offered temporal bindings are not in canonical order.
    BindingOfferOrder,
    /// Expected selection does not own the selected source span.
    BindingExpectedForeignOwner,
    /// Producer selection does not own the selected source span.
    BindingProducerForeignOwner,
    /// Offered declaration index is invalid.
    BindingOfferDeclarationIndex,
    /// Offered definition index is invalid.
    BindingOfferDefinitionIndex,
}

impl RefusalCode {
    /// Exact stable version 2 spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HeaderWire => "v2.header.wire",
            Self::HeaderMedia => "v2.header.media",
            Self::HeaderSchema => "v2.header.schema",
            Self::HeaderPackageType => "v2.header.package-type",
            Self::HeaderEncoding => "v2.header.encoding",
            Self::HeaderNumeric => "v2.header.numeric",
            Self::HeaderArtifactKind => "v2.header.artifact-kind",
            Self::HeaderArtifactWire => "v2.header.artifact-wire",
            Self::HeaderArtifactVersion => "v2.header.artifact-version",
            Self::BindingOfferMissing => "v2.binding.offer-missing",
            Self::BindingOfferSurplus => "v2.binding.offer-surplus",
            Self::BindingExpectedMissing => "v2.binding.expected-missing",
            Self::BindingExpectedSurplus => "v2.binding.expected-surplus",
            Self::BindingProducerMissing => "v2.binding.producer-missing",
            Self::BindingProducerSurplus => "v2.binding.producer-surplus",
            Self::BindingOfferDuplicate => "v2.binding.offer-duplicate",
            Self::BindingExpectedDuplicate => "v2.binding.expected-duplicate",
            Self::BindingProducerDuplicate => "v2.binding.producer-duplicate",
            Self::BindingOfferClockName => "v2.binding.offer-clock-name",
            Self::BindingOfferOrder => "v2.binding.offer-order",
            Self::BindingExpectedForeignOwner => "v2.binding.expected-foreign-owner",
            Self::BindingProducerForeignOwner => "v2.binding.producer-foreign-owner",
            Self::BindingOfferDeclarationIndex => "v2.binding.offer-declaration-index",
            Self::BindingOfferDefinitionIndex => "v2.binding.offer-definition-index",
        }
    }
}
