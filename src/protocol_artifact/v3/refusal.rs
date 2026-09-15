// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-054: stable refusal identities for the strict version-3 boundary.

use std::fmt;

/// The side of the activation mapping inventory that is defective.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventorySide {
    /// The offered package table.
    Offer,
    /// The independently supplied expectation table.
    Expected,
}

/// Structural mapping-table defect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingCause {
    /// A required mapping is absent.
    Missing,
    /// A mapping has no independent counterpart.
    Surplus,
    /// More than one mapping names one control.
    Duplicate,
}

/// Exact mapping coordinate which differs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingField {
    /// The declaration-local control handle.
    Control,
    /// The temporal declaration index.
    TemporalDeclaration,
}

/// Stable machine-readable version-3 refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Refusal {
    /// The payload or selected artifact declares a non-v3 header.
    Header,
    /// One complete activation mapping population is invalid.
    Mapping {
        side: InventorySide,
        cause: MappingCause,
    },
    /// Offered mappings are not in ascending control-handle order.
    OfferOrder,
    /// An offered control handle is foreign, out of range, or not event-triggered.
    Control,
    /// An offered temporal declaration is foreign, out of range, or non-temporal.
    TemporalDeclaration,
    /// An independently expected mapping differs from the offer.
    Expected(MappingField),
}

impl Refusal {
    /// Stable refusal code; codes are never derived from diagnostics.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Header => "v3.header",
            Self::Mapping {
                side: InventorySide::Offer,
                cause: MappingCause::Missing,
            } => "v3.mapping.offer-missing",
            Self::Mapping {
                side: InventorySide::Offer,
                cause: MappingCause::Surplus,
            } => "v3.mapping.offer-surplus",
            Self::Mapping {
                side: InventorySide::Offer,
                cause: MappingCause::Duplicate,
            } => "v3.mapping.offer-duplicate",
            Self::Mapping {
                side: InventorySide::Expected,
                cause: MappingCause::Missing,
            } => "v3.mapping.expected-missing",
            Self::Mapping {
                side: InventorySide::Expected,
                cause: MappingCause::Surplus,
            } => "v3.mapping.expected-surplus",
            Self::Mapping {
                side: InventorySide::Expected,
                cause: MappingCause::Duplicate,
            } => "v3.mapping.expected-duplicate",
            Self::OfferOrder => "v3.mapping.offer-order",
            Self::Control => "v3.mapping.control",
            Self::TemporalDeclaration => "v3.mapping.temporal-declaration",
            Self::Expected(MappingField::Control) => "v3.mapping.expected-control",
            Self::Expected(MappingField::TemporalDeclaration) => {
                "v3.mapping.expected-temporal-declaration"
            }
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}
