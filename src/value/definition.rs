// SPDX-License-Identifier: AGPL-3.0-or-later
//! The pinned `quire.value.definition-lock/v1` catalog, per-package selection
//! admission and integer-division profile admission.
//!
//! Profile misuse is refused here, at semantic admission, before any expression
//! is evaluated. Selection refusals use the lock's closed
//! `selection_refusal_codes`; definition-closure refusals use the I04
//! `invalid_package` code with its closed `cause_tag` vocabulary.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde::Deserialize;

use super::division::DivisionProfile;

/// Exact bytes of `complete-value-lock.json` at QSpec
/// `d227270fbeb28289df6abba7e94173118345c028`.
pub const PINNED_LOCK_BYTES: &[u8] = include_bytes!(
    "../../resources/complete-value/quire-specification/proposals/quire-v1/definitions/complete-value-lock.json"
);

/// The lock's `trigger_vocabulary`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// The package evaluates an IEEE `float32`/`float64` type, value or operation.
    IeeeOperation,
    /// The package evaluates integer `div` or `rem`.
    IntegerDivRem,
    /// The package declares or evaluates a text type or value.
    TextBearing,
}

impl Trigger {
    /// Closed vocabulary in lock order.
    pub const ALL: [Self; 3] = [Self::IeeeOperation, Self::IntegerDivRem, Self::TextBearing];

    /// Normative spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IeeeOperation => "ieee_operation",
            Self::IntegerDivRem => "integer_div_rem",
            Self::TextBearing => "text_bearing",
        }
    }

    /// Resolve a spelling.
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|trigger| trigger.as_str() == code)
    }
}

/// A qualification-catalog role; each variant is the lock role of the same name.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum CatalogRole {
    /// The `ix:native` language edition definition (`edition.md`).
    Edition,
    /// The `quire.value.complete/v1` root value-system definition (`value-complete.md`).
    Root,
    /// The `quire.value.accounting/v1` charge-accounting definition (`value-accounting.md`).
    Accounting,
    /// The `quire.value.compound-unit/v1` compound-unit definition (`value-compound-unit.md`).
    CompoundUnit,
    /// The `quire.value.compound-unit.schema/v1` JSON Schema for compound units
    /// (`value-compound-unit.schema.json`).
    CompoundUnitSchema,
    /// The `quire.value.compound-unit.vectors/v1` compound-unit test vectors
    /// (`value-compound-unit-vectors.json`).
    CompoundUnitVectors,
    /// The `quire.value.complete.rules/v1` manifest listing the always-selected
    /// rule roles (`value-complete-rules.json`).
    RuleManifest,
    /// The `quire.value.text.unicode-17.0.0/v1` text profile, selected when the
    /// package is `text_bearing` (`value-text-unicode-17.md`).
    TextProfile,
    /// The `quire.value.ieee754-2019-default/v1` IEEE binary32/binary64 profile,
    /// selected when the package is `ieee_operation` (`value-ieee754-2019-default.md`).
    IeeeProfile,
    /// The `quire.value.integer-division.euclidean/v1` Euclidean `div`/`rem` law
    /// (`value-integer-division-euclidean.md`).
    IntegerDivisionEuclidean,
    /// The `quire.value.integer-division.floor/v1` floored `div`/`rem` law
    /// (`value-integer-division-floor.md`).
    IntegerDivisionFloor,
    /// The `quire.value.integer-division.truncating/v1` truncating `div`/`rem` law
    /// (`value-integer-division-truncating.md`).
    IntegerDivisionTruncating,
    /// The `quire.rule.package-contract/v1` package-contract rule (`../package-contract.md`).
    RulePackageContract,
    /// The `quire.rule.shared-grammar/v1` shared-grammar rule (`../shared-grammar.md`).
    RuleSharedGrammar,
    /// The `quire.rule.ad-005/v1` rule binding AD-005's complete-value expression
    /// system (`AD-005-complete-value-expression-system.md`).
    #[serde(rename = "rule_ad_005")]
    RuleAd005,
    /// The `quire.rule.fr-140/v1` rule binding FR-140 exact-decimal evaluation
    /// (`FR-140-evaluate-exact-decimals.md`).
    #[serde(rename = "rule_fr_140")]
    RuleFr140,
    /// The `quire.rule.fr-141/v1` rule binding FR-141 text and enumeration
    /// evaluation (`FR-141-evaluate-text-and-enumerations.md`).
    #[serde(rename = "rule_fr_141")]
    RuleFr141,
    /// The `quire.rule.fr-142/v1` rule binding FR-142 quantity and unit
    /// evaluation (`FR-142-evaluate-quantities-and-units.md`).
    #[serde(rename = "rule_fr_142")]
    RuleFr142,
    /// The `quire.rule.fr-147/v1` rule binding FR-147 integer-division-domain
    /// evaluation (`FR-147-evaluate-integer-division-domains.md`).
    #[serde(rename = "rule_fr_147")]
    RuleFr147,
    /// The `quire.rule.fr-148/v1` rule binding FR-148 IEEE floating-point
    /// profile evaluation (`FR-148-evaluate-ieee-floating-profiles.md`).
    #[serde(rename = "rule_fr_148")]
    RuleFr148,
    /// The `quire.rule.fr-149/v1` rule binding FR-149's complete equality
    /// matrix (`FR-149-apply-complete-equality-matrix.md`).
    #[serde(rename = "rule_fr_149")]
    RuleFr149,
}

impl CatalogRole {
    /// Every role in lock catalog order.
    pub const ALL: [Self; 21] = [
        Self::Edition,
        Self::Root,
        Self::Accounting,
        Self::CompoundUnit,
        Self::CompoundUnitSchema,
        Self::CompoundUnitVectors,
        Self::RuleManifest,
        Self::TextProfile,
        Self::IeeeProfile,
        Self::IntegerDivisionEuclidean,
        Self::IntegerDivisionFloor,
        Self::IntegerDivisionTruncating,
        Self::RulePackageContract,
        Self::RuleSharedGrammar,
        Self::RuleAd005,
        Self::RuleFr140,
        Self::RuleFr141,
        Self::RuleFr142,
        Self::RuleFr147,
        Self::RuleFr148,
        Self::RuleFr149,
    ];

    /// Normative spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Edition => "edition",
            Self::Root => "root",
            Self::Accounting => "accounting",
            Self::CompoundUnit => "compound_unit",
            Self::CompoundUnitSchema => "compound_unit_schema",
            Self::CompoundUnitVectors => "compound_unit_vectors",
            Self::RuleManifest => "rule_manifest",
            Self::TextProfile => "text_profile",
            Self::IeeeProfile => "ieee_profile",
            Self::IntegerDivisionEuclidean => "integer_division_euclidean",
            Self::IntegerDivisionFloor => "integer_division_floor",
            Self::IntegerDivisionTruncating => "integer_division_truncating",
            Self::RulePackageContract => "rule_package_contract",
            Self::RuleSharedGrammar => "rule_shared_grammar",
            Self::RuleAd005 => "rule_ad_005",
            Self::RuleFr140 => "rule_fr_140",
            Self::RuleFr141 => "rule_fr_141",
            Self::RuleFr142 => "rule_fr_142",
            Self::RuleFr147 => "rule_fr_147",
            Self::RuleFr148 => "rule_fr_148",
            Self::RuleFr149 => "rule_fr_149",
        }
    }

    /// Resolve a spelling.
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.as_str() == code)
    }

    /// The division law this role selects, if it is a division role.
    pub fn division_profile(self) -> Option<DivisionProfile> {
        match self {
            Self::IntegerDivisionEuclidean => Some(DivisionProfile::Euclidean),
            Self::IntegerDivisionFloor => Some(DivisionProfile::Floor),
            Self::IntegerDivisionTruncating => Some(DivisionProfile::Truncating),
            Self::Edition
            | Self::Root
            | Self::Accounting
            | Self::CompoundUnit
            | Self::CompoundUnitSchema
            | Self::CompoundUnitVectors
            | Self::RuleManifest
            | Self::TextProfile
            | Self::IeeeProfile
            | Self::RulePackageContract
            | Self::RuleSharedGrammar
            | Self::RuleAd005
            | Self::RuleFr140
            | Self::RuleFr141
            | Self::RuleFr142
            | Self::RuleFr147
            | Self::RuleFr148
            | Self::RuleFr149 => None,
        }
    }
}

/// The lock's closed `selection_refusal_codes`, in check order.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
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

/// A definition revision `{ namespace, value }`.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct DefinitionRevision {
    /// Revision namespace.
    pub namespace: String,
    /// Revision value.
    pub value: String,
}

/// A retained DefinitionRef as it appears in the lock and in a checked package.
/// This is untrusted data; only admission turns it into authority.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct DefinitionReference {
    /// Publishing authority.
    pub authority: String,
    /// Definition identity.
    pub identity: String,
    /// Exact revision.
    pub revision: DefinitionRevision,
    /// Digest domain.
    pub digest_domain: String,
    /// Lowercase hex raw-byte SHA-256.
    pub digest: String,
}

/// One qualification-catalog entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CatalogEntry {
    /// Catalog role.
    pub role: CatalogRole,
    /// Path relative to the lock's directory in the pinned specification.
    pub artifact_path: String,
    /// The exact DefinitionRef.
    pub definition: DefinitionReference,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ConditionalSlot {
    trigger: Trigger,
    role: CatalogRole,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct AlternativeSlot {
    trigger: Trigger,
    selector: String,
    roles: Vec<CatalogRole>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct SelectionSlots {
    always: Vec<CatalogRole>,
    conditional: Vec<ConditionalSlot>,
    exactly_one: Vec<AlternativeSlot>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct LockDocument {
    lock_version: String,
    revision: String,
    digest_domain: String,
    language_edition: String,
    trigger_vocabulary: Vec<Trigger>,
    selection_refusal_codes: Vec<SelectionRefusalCode>,
    qualification_catalog: Vec<CatalogEntry>,
    package_selection: SelectionSlots,
}

/// Why lock bytes are not an admissible `quire.value.definition-lock/v1`.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LockError {
    /// The bytes are not the lock JSON shape.
    #[error("malformed definition lock at line {line}, column {column}")]
    Malformed {
        /// One-based JSON line.
        line: usize,
        /// One-based JSON column.
        column: usize,
    },
    /// `lock_version`, `digest_domain` or `language_edition` is not the v1 value.
    #[error("unsupported definition lock header")]
    UnsupportedHeader,
    /// `trigger_vocabulary` or `selection_refusal_codes` differs from the closed set.
    #[error("definition lock vocabulary differs from the closed v1 set")]
    VocabularyMismatch,
    /// A catalog role is absent or repeated.
    #[error("catalog role {0:?} is absent or repeated")]
    CatalogRole(CatalogRole),
    /// A catalog entry's digest domain or digest spelling is invalid.
    #[error("catalog role {0:?} has an invalid digest")]
    InvalidDigest(CatalogRole),
    /// A role is not assigned to exactly one selection slot.
    #[error("catalog role {0:?} is not in exactly one selection slot")]
    SelectionSlot(CatalogRole),
}

/// An admitted definition lock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinitionLock {
    document: LockDocument,
}

const DIGEST_DOMAIN: &str = "quire.definition.bytes/v1";

impl DefinitionLock {
    /// The lock embedded from the pinned QSpec revision.
    pub fn pinned() -> Result<&'static Self, LockError> {
        static PINNED: OnceLock<Result<DefinitionLock, LockError>> = OnceLock::new();
        PINNED
            .get_or_init(|| Self::parse(PINNED_LOCK_BYTES))
            .as_ref()
            .map_err(Clone::clone)
    }

    /// Parse and validate lock bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self, LockError> {
        let document: LockDocument =
            serde_json::from_slice(bytes).map_err(|error| LockError::Malformed {
                line: error.line(),
                column: error.column(),
            })?;
        if document.lock_version != "quire.value.definition-lock/v1"
            || document.digest_domain != DIGEST_DOMAIN
            || document.language_edition != "1-draft"
        {
            return Err(LockError::UnsupportedHeader);
        }
        if document.trigger_vocabulary != Trigger::ALL {
            return Err(LockError::VocabularyMismatch);
        }
        let codes: BTreeSet<_> = document.selection_refusal_codes.iter().copied().collect();
        if codes.len() != document.selection_refusal_codes.len()
            || codes != SelectionRefusalCode::ALL.into_iter().collect()
        {
            return Err(LockError::VocabularyMismatch);
        }
        for role in CatalogRole::ALL {
            let mut entries = document
                .qualification_catalog
                .iter()
                .filter(|entry| entry.role == role);
            let entry = entries.next().ok_or(LockError::CatalogRole(role))?;
            if entries.next().is_some() {
                return Err(LockError::CatalogRole(role));
            }
            let definition = &entry.definition;
            let hex = definition.digest.as_bytes();
            if definition.digest_domain != DIGEST_DOMAIN
                || hex.len() != 64
                || !hex.iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
            {
                return Err(LockError::InvalidDigest(role));
            }
        }
        let slots = &document.package_selection;
        let assigned: Vec<CatalogRole> = slots
            .always
            .iter()
            .copied()
            .chain(slots.conditional.iter().map(|slot| slot.role))
            .chain(
                slots
                    .exactly_one
                    .iter()
                    .flat_map(|slot| slot.roles.iter().copied()),
            )
            .collect();
        for role in CatalogRole::ALL {
            if assigned.iter().filter(|seen| **seen == role).count() != 1 {
                return Err(LockError::SelectionSlot(role));
            }
        }
        Ok(Self { document })
    }

    /// Lock revision.
    pub fn revision(&self) -> &str {
        &self.document.revision
    }

    /// The closed qualification catalog.
    pub fn catalog(&self) -> &[CatalogEntry] {
        &self.document.qualification_catalog
    }

    /// The catalog entry for `role`.
    pub fn entry(&self, role: CatalogRole) -> Option<&CatalogEntry> {
        self.catalog().iter().find(|entry| entry.role == role)
    }

    /// Roles every package selects.
    pub fn always_roles(&self) -> &[CatalogRole] {
        &self.document.package_selection.always
    }

    /// Admit one package's selection given its trigger and role spellings.
    ///
    /// Checks run in the normative order and report the first failure: every
    /// trigger spelling is resolved before a repeated trigger refuses.
    pub fn admit_selection(
        &self,
        triggers: &[&str],
        roles: &[&str],
    ) -> Result<AdmittedSelection, SelectionRefusalCode> {
        let triggers = trigger_set(triggers)?;
        let parsed = roles
            .iter()
            .map(|code| CatalogRole::from_code(code))
            .collect::<Option<Vec<_>>>()
            .ok_or(SelectionRefusalCode::SelectionUnknownRole)?;
        let selected: BTreeSet<_> = parsed.iter().copied().collect();
        if selected.len() != parsed.len() {
            return Err(SelectionRefusalCode::SelectionDuplicateRole);
        }
        let slots = &self.document.package_selection;
        if !slots.always.iter().all(|role| selected.contains(role)) {
            return Err(SelectionRefusalCode::SelectionRequiredMissing);
        }
        if slots.exactly_one.iter().any(|slot| {
            slot.roles
                .iter()
                .filter(|role| selected.contains(role))
                .count()
                > 1
        }) {
            return Err(SelectionRefusalCode::SelectionAlternativeConflict);
        }
        let unsatisfied = slots
            .conditional
            .iter()
            .any(|slot| triggers.contains(&slot.trigger) && !selected.contains(&slot.role))
            || slots.exactly_one.iter().any(|slot| {
                triggers.contains(&slot.trigger)
                    && !slot.roles.iter().any(|role| selected.contains(role))
            });
        if unsatisfied {
            return Err(SelectionRefusalCode::SelectionTriggerUnsatisfied);
        }
        let untriggered = slots
            .conditional
            .iter()
            .any(|slot| !triggers.contains(&slot.trigger) && selected.contains(&slot.role))
            || slots.exactly_one.iter().any(|slot| {
                !triggers.contains(&slot.trigger)
                    && slot.roles.iter().any(|role| selected.contains(role))
            });
        if untriggered {
            return Err(SelectionRefusalCode::SelectionUntriggeredProfile);
        }
        let division = selected.iter().find_map(|role| role.division_profile());
        Ok(AdmittedSelection {
            triggers,
            roles: selected,
            division,
        })
    }

    /// Admit a checked package's integer-division definition closure.
    ///
    /// `div_rem` lists every DefinitionRef the package retains for `div`/`rem`;
    /// `modulo` is the definition an explicit `mod` binding claims, if any.
    pub fn admit_integer_division(
        &self,
        div_rem: &[DefinitionReference],
        modulo: Option<&DefinitionReference>,
    ) -> Result<AdmittedIntegerDivision, PackageRefusal> {
        let mut profiles = Vec::new();
        for reference in div_rem {
            profiles.push(self.resolve_division(reference)?);
        }
        let profile = match profiles.as_slice() {
            [] => return Err(PackageRefusal::invalid_package(PackageCause::MissingMember)),
            [profile] => *profile,
            [_, _, ..] => {
                return Err(PackageRefusal::invalid_package(
                    PackageCause::ConflictingDefinition,
                ))
            }
        };
        if let Some(reference) = modulo {
            if self.resolve_division(reference)? != DivisionProfile::Euclidean {
                return Err(PackageRefusal::invalid_package(
                    PackageCause::IncompatibleDefinition,
                ));
            }
        }
        Ok(AdmittedIntegerDivision { profile })
    }

    fn resolve_division(
        &self,
        reference: &DefinitionReference,
    ) -> Result<DivisionProfile, PackageRefusal> {
        let (profile, expected) = DivisionProfile::ALL
            .into_iter()
            .find_map(|profile| {
                let entry = self.catalog().iter().find(|entry| {
                    entry.role.division_profile() == Some(profile)
                        && entry.definition.identity == reference.identity
                })?;
                Some((profile, &entry.definition))
            })
            .ok_or(PackageRefusal::invalid_package(
                PackageCause::IncompatibleDefinition,
            ))?;
        let cause = if reference.authority != expected.authority {
            Some(PackageCause::IncompatibleDefinition)
        } else if reference.revision != expected.revision {
            Some(PackageCause::RevisionMismatch)
        } else if reference.digest_domain != expected.digest_domain {
            Some(PackageCause::DigestDomainMismatch)
        } else if reference.digest != expected.digest {
            Some(PackageCause::ByteDigestMismatch)
        } else {
            None
        };
        match cause {
            Some(cause) => Err(PackageRefusal::invalid_package(cause)),
            None => Ok(profile),
        }
    }
}

fn trigger_set(codes: &[&str]) -> Result<BTreeSet<Trigger>, SelectionRefusalCode> {
    let parsed = codes
        .iter()
        .map(|code| Trigger::from_code(code))
        .collect::<Option<Vec<_>>>()
        .ok_or(SelectionRefusalCode::SelectionUnknownTrigger)?;
    let triggers: BTreeSet<_> = parsed.iter().copied().collect();
    if triggers.len() == parsed.len() {
        Ok(triggers)
    } else {
        Err(SelectionRefusalCode::SelectionDuplicateTrigger)
    }
}

/// An admitted package selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedSelection {
    triggers: BTreeSet<Trigger>,
    roles: BTreeSet<CatalogRole>,
    division: Option<DivisionProfile>,
}

impl AdmittedSelection {
    /// Present triggers.
    pub fn triggers(&self) -> &BTreeSet<Trigger> {
        &self.triggers
    }

    /// Selected roles.
    pub fn roles(&self) -> &BTreeSet<CatalogRole> {
        &self.roles
    }

    /// The selected `div`/`rem` law, when `integer_div_rem` is present.
    pub fn division_profile(&self) -> Option<DivisionProfile> {
        self.division
    }
}

/// The admitted `div`/`rem` law of one checked package. Only
/// [`DefinitionLock::admit_integer_division`] constructs it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AdmittedIntegerDivision {
    profile: DivisionProfile,
}

impl AdmittedIntegerDivision {
    /// The selected law.
    pub fn profile(&self) -> DivisionProfile {
        self.profile
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
    /// `revision-mismatch`.
    RevisionMismatch,
    /// `digest-domain-mismatch`.
    DigestDomainMismatch,
    /// `byte-digest-mismatch`.
    ByteDigestMismatch,
}

impl PackageCause {
    /// Normative spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingMember => "missing-member",
            Self::ConflictingDefinition => "conflicting-definition",
            Self::IncompatibleDefinition => "incompatible-definition",
            Self::RevisionMismatch => "revision-mismatch",
            Self::DigestDomainMismatch => "digest-domain-mismatch",
            Self::ByteDigestMismatch => "byte-digest-mismatch",
        }
    }
}

/// `refused { code, cause }` at semantic admission.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("{}: {}", code.as_str(), cause.as_str())]
pub struct PackageRefusal {
    /// Diagnostic code.
    pub code: PackageRefusalCode,
    /// Typed cause.
    pub cause: PackageCause,
}

impl PackageRefusal {
    fn invalid_package(cause: PackageCause) -> Self {
        Self {
            code: PackageRefusalCode::InvalidPackage,
            cause,
        }
    }
}
