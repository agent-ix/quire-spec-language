// SPDX-License-Identifier: AGPL-3.0-or-later
//! The definition-admission refusal vocabulary (ADR-011 §6.1 layer SV): the
//! lock's closed `selection_refusal_codes` ([`SelectionRefusalCode`]) and the
//! I04 `invalid_package` refusal with its closed `cause_tag` subset
//! ([`PackageRefusal`]). The lock catalog and the admission that raises them
//! stay in `qsl-semantics`' `value::definition`.

/// The lock's closed `selection_refusal_codes`, in check order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionRefusalCode {
    /// `selection_unknown_trigger`.
    SelectionUnknownTrigger,
    /// `selection_duplicate_trigger`.
    SelectionDuplicateTrigger,
    /// `selection_unknown_role`.
    SelectionUnknownRole,
    /// `selection_duplicate_role`.
    SelectionDuplicateRole,
    /// `selection_required_missing`.
    SelectionRequiredMissing,
    /// `selection_alternative_conflict`.
    SelectionAlternativeConflict,
    /// `selection_trigger_unsatisfied`.
    SelectionTriggerUnsatisfied,
    /// `selection_untriggered_profile`.
    SelectionUntriggeredProfile,
}

impl SelectionRefusalCode {
    /// Every code in normative check order.
    pub const ALL: [Self; 8] = [
        Self::SelectionUnknownTrigger,
        Self::SelectionDuplicateTrigger,
        Self::SelectionUnknownRole,
        Self::SelectionDuplicateRole,
        Self::SelectionRequiredMissing,
        Self::SelectionAlternativeConflict,
        Self::SelectionTriggerUnsatisfied,
        Self::SelectionUntriggeredProfile,
    ];

    /// Normative spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SelectionUnknownTrigger => "selection_unknown_trigger",
            Self::SelectionDuplicateTrigger => "selection_duplicate_trigger",
            Self::SelectionUnknownRole => "selection_unknown_role",
            Self::SelectionDuplicateRole => "selection_duplicate_role",
            Self::SelectionRequiredMissing => "selection_required_missing",
            Self::SelectionAlternativeConflict => "selection_alternative_conflict",
            Self::SelectionTriggerUnsatisfied => "selection_trigger_unsatisfied",
            Self::SelectionUntriggeredProfile => "selection_untriggered_profile",
        }
    }

    /// Resolve a spelling.
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|refusal| refusal.as_str() == code)
    }
}

/// The I04 diagnostic code of a definition-closure refusal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PackageRefusalCode {
    /// `invalid_package`.
    InvalidPackage,
}

impl PackageRefusalCode {
    /// Normative spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidPackage => "invalid_package",
        }
    }
}

/// The subset of the closed I04 `cause_tag` vocabulary that division admission
/// reports.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PackageCause {
    /// `missing-member`: no `div`/`rem` definition is retained.
    MissingMember,
    /// `conflicting-definition`: more than one law is retained.
    ConflictingDefinition,
    /// `incompatible-definition`: not a division definition, or `mod` claims a
    /// non-Euclidean law.
    IncompatibleDefinition,
}

impl PackageCause {
    /// Normative spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingMember => "missing-member",
            Self::ConflictingDefinition => "conflicting-definition",
            Self::IncompatibleDefinition => "incompatible-definition",
        }
    }
}

/// `refused { code, cause }` at semantic admission.
///
/// quire:canonical
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("{}: {}", code.as_str(), cause.as_str())]
pub struct PackageRefusal {
    /// Diagnostic code.
    pub code: PackageRefusalCode,
    /// Typed cause.
    pub cause: PackageCause,
}

impl PackageRefusal {
    /// The `invalid_package` refusal with `cause`.
    pub fn invalid_package(cause: PackageCause) -> Self {
        Self {
            code: PackageRefusalCode::InvalidPackage,
            cause,
        }
    }
}
