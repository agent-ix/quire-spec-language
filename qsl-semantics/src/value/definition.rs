// SPDX-License-Identifier: AGPL-3.0-or-later
//! The closed `quire.value.definition-lock/v1` catalog, per-package selection
//! admission, integer-division profile admission and IEEE profile admission.
//!
//! [`divide`] and [`modulo`] evaluate integer division under an admitted law.
//! They call `quire_exact::divide`/`modulo` and carry the kernel
//! [`Outcome`](quire_exact::Outcome) straight through.
//!
//! Profile misuse is refused here, at semantic admission, before any expression
//! is evaluated. Selection refusals use the lock's closed
//! `selection_refusal_codes`; definition-closure refusals use the I04
//! `invalid_package` code with its closed `cause_tag` vocabulary.
//!
//! The qualification catalog ([`CATALOG`]) is a closed table of forward
//! references: each row names the authority, identity, revision, artifact
//! path and raw-byte digest of one role's definition, as QSpec's
//! `complete-value-lock.json` records them. Admission recognizes a
//! caller-supplied [`DefinitionReference`] by identity and revision. The v2
//! emitter writes each row as the package lock's edition and definition
//! selections ([`CatalogEntry::reference`]).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use quire_exact::{
    ieee_intrinsic_identities, DivisionProfile, Integer, IntegerDomain, Meter, Outcome,
    QuotientRemainder,
};

/// The lock's `trigger_vocabulary`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
    RuleAd005,
    /// The `quire.rule.fr-140/v1` rule binding FR-140 exact-decimal evaluation
    /// (`FR-140-evaluate-exact-decimals.md`).
    RuleFr140,
    /// The `quire.rule.fr-141/v1` rule binding FR-141 text and enumeration
    /// evaluation (`FR-141-evaluate-text-and-enumerations.md`).
    RuleFr141,
    /// The `quire.rule.fr-142/v1` rule binding FR-142 quantity and unit
    /// evaluation (`FR-142-evaluate-quantities-and-units.md`).
    RuleFr142,
    /// The `quire.rule.fr-147/v1` rule binding FR-147 integer-division-domain
    /// evaluation (`FR-147-evaluate-integer-division-domains.md`).
    RuleFr147,
    /// The `quire.rule.fr-148/v1` rule binding FR-148 IEEE floating-point
    /// profile evaluation (`FR-148-evaluate-ieee-floating-profiles.md`).
    RuleFr148,
    /// The `quire.rule.fr-149/v1` rule binding FR-149's complete equality
    /// matrix (`FR-149-apply-complete-equality-matrix.md`).
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

/// A definition revision `{ namespace, value }`.
#[derive(Clone, Debug, Deserialize, Eq, Serialize, Hash, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct DefinitionRevision {
    /// Revision namespace.
    pub namespace: String,
    /// Revision value.
    pub value: String,
}

/// A retained DefinitionRef as it appears in the lock and in a checked package.
/// This is untrusted data; only admission turns it into authority.
#[derive(Clone, Debug, Deserialize, Eq, Serialize, Hash, Ord, PartialEq, PartialOrd)]
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

/// One qualification-catalog entry: a closed role's artifact path, the exact
/// identity/revision a caller-supplied [`DefinitionReference`] for that role
/// is checked against, and the definition's raw-byte digest.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatalogEntry {
    /// Catalog role.
    pub role: CatalogRole,
    /// Path relative to the standard's own definitions directory.
    pub artifact_path: &'static str,
    /// Publishing authority.
    pub authority: &'static str,
    /// Definition identity.
    pub identity: &'static str,
    /// Revision namespace.
    pub revision_namespace: &'static str,
    /// Revision value.
    pub revision_value: &'static str,
    /// Lowercase hex SHA-256 of the definition's bytes
    /// (`quire.definition.bytes/v1`), from QSpec's `complete-value-lock.json`.
    /// Informational; no reader verifies it yet.
    pub digest: &'static str,
}

impl CatalogEntry {
    /// This entry as the `DefinitionRef` a package lock selects.
    pub fn reference(&self) -> DefinitionReference {
        DefinitionReference {
            authority: self.authority.to_owned(),
            identity: self.identity.to_owned(),
            revision: DefinitionRevision {
                namespace: self.revision_namespace.to_owned(),
                value: self.revision_value.to_owned(),
            },
            digest_domain: DIGEST_DOMAIN.to_owned(),
            digest: self.digest.to_owned(),
        }
    }
}

const AGENT_IX: &str = "agent-ix";
const DRAFT: &str = "quire-draft";

/// The closed qualification catalog: where each role's definition is
/// resolved from.
const CATALOG: [CatalogEntry; 21] = [
    CatalogEntry {
        role: CatalogRole::Edition,
        artifact_path: "edition.md",
        authority: AGENT_IX,
        identity: "ix:native",
        revision_namespace: DRAFT,
        revision_value: "1-draft.2",
        digest: "4cf0b7ac51a3b9417bc1c10a06b26d1e6a19d02fc549ef70ec627fab35bea625",
    },
    CatalogEntry {
        role: CatalogRole::Root,
        artifact_path: "value-complete.md",
        authority: AGENT_IX,
        identity: "quire.value.complete/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.2",
        digest: "c8c7ae9fbe783286369ecc83f006190f83be4c3c8fc585766617c90f27a25b16",
    },
    CatalogEntry {
        role: CatalogRole::Accounting,
        artifact_path: "value-accounting.md",
        authority: AGENT_IX,
        identity: "quire.value.accounting/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "1d8b15f8b0cb20bfb04841101e6dd09735cca06fe80e38652f48555a0c6871fa",
    },
    CatalogEntry {
        role: CatalogRole::CompoundUnit,
        artifact_path: "value-compound-unit.md",
        authority: AGENT_IX,
        identity: "quire.value.compound-unit/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "320e3befa686f007f42ddccd58d2f8246699045abe45520adb9ff8357f6d2ca4",
    },
    CatalogEntry {
        role: CatalogRole::CompoundUnitSchema,
        artifact_path: "value-compound-unit.schema.json",
        authority: AGENT_IX,
        identity: "quire.value.compound-unit.schema/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "740824cbee8d83a9826d688106a429227e96408547db7aabe1bb607e81ce1654",
    },
    CatalogEntry {
        role: CatalogRole::CompoundUnitVectors,
        artifact_path: "value-compound-unit-vectors.json",
        authority: AGENT_IX,
        identity: "quire.value.compound-unit.vectors/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "8636d0d7f7db87d5f311bab3f2a2b0bd19e9753bb955e21f6ea23a3657b2589c",
    },
    CatalogEntry {
        role: CatalogRole::RuleManifest,
        artifact_path: "value-complete-rules.json",
        authority: AGENT_IX,
        identity: "quire.value.complete.rules/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.2",
        digest: "8b8500fa5d7d3f5b0683984e9a09beecdfffc10520820f68e21ce8fbfad7bd32",
    },
    CatalogEntry {
        role: CatalogRole::TextProfile,
        artifact_path: "value-text-unicode-17.md",
        authority: AGENT_IX,
        identity: "quire.value.text.unicode-17.0.0/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "cd4a985a0d7d2f2b3d3625caee3787832c00c5244e805fb49e1c2c7075b9de5e",
    },
    CatalogEntry {
        role: CatalogRole::IeeeProfile,
        artifact_path: "value-ieee754-2019-default.md",
        authority: AGENT_IX,
        identity: "quire.value.ieee754-2019-default/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "3e9736fb8e1637b554385192de34547bafc073e90b4b85256c824be31e0aa6e5",
    },
    CatalogEntry {
        role: CatalogRole::IntegerDivisionEuclidean,
        artifact_path: "value-integer-division-euclidean.md",
        authority: AGENT_IX,
        identity: "quire.value.integer-division.euclidean/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "9f5e59b3bfe1dd3c1efc74065b2e3e7869e21813a0c90b9c5938d82107267a51",
    },
    CatalogEntry {
        role: CatalogRole::IntegerDivisionFloor,
        artifact_path: "value-integer-division-floor.md",
        authority: AGENT_IX,
        identity: "quire.value.integer-division.floor/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "ca8c7a20407eaff7f61074cc997e44ad1ab9a73f675d686c6250997c6ae6192f",
    },
    CatalogEntry {
        role: CatalogRole::IntegerDivisionTruncating,
        artifact_path: "value-integer-division-truncating.md",
        authority: AGENT_IX,
        identity: "quire.value.integer-division.truncating/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "9998507608e4885b314d5dcc59a88bb3d04ef3c263d2d8ae5810f92ae1893364",
    },
    CatalogEntry {
        role: CatalogRole::RulePackageContract,
        artifact_path: "../package-contract.md",
        authority: AGENT_IX,
        identity: "quire.rule.package-contract/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "e1fc96174770947d3d6e48e045cd64c9b2a1cb9a8ec053e8109b5a7ddbf63b72",
    },
    CatalogEntry {
        role: CatalogRole::RuleSharedGrammar,
        artifact_path: "../shared-grammar.md",
        authority: AGENT_IX,
        identity: "quire.rule.shared-grammar/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.2",
        digest: "697b2458455e0c209120019013b24ad0d2506fdf5ce57c41655d066b2b057afd",
    },
    CatalogEntry {
        role: CatalogRole::RuleAd005,
        artifact_path: "../../../spec/assurance/AD-005-complete-value-expression-system.md",
        authority: AGENT_IX,
        identity: "quire.rule.ad-005/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "9b1c9d215fe3545215583484e067fcad15008eb4123d23685af2ae4ac9b2c447",
    },
    CatalogEntry {
        role: CatalogRole::RuleFr140,
        artifact_path: "../../../spec/functional/type-model/FR-140-evaluate-exact-decimals.md",
        authority: AGENT_IX,
        identity: "quire.rule.fr-140/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "4d0dcb64423014f8f2a64fadd7216f4974626dcacf586fe6930da6a54aa51131",
    },
    CatalogEntry {
        role: CatalogRole::RuleFr141,
        artifact_path:
            "../../../spec/functional/type-model/FR-141-evaluate-text-and-enumerations.md",
        authority: AGENT_IX,
        identity: "quire.rule.fr-141/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "5438519ea6d13e5df8947d13d5c257b46de1aed0c621295f1df76fa97e160785",
    },
    CatalogEntry {
        role: CatalogRole::RuleFr142,
        artifact_path:
            "../../../spec/functional/type-model/FR-142-evaluate-quantities-and-units.md",
        authority: AGENT_IX,
        identity: "quire.rule.fr-142/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "7eb40d7fbaea7ac5e28720ccb2bbc08595f84028bbcdcaa8998e72538a806a05",
    },
    CatalogEntry {
        role: CatalogRole::RuleFr147,
        artifact_path:
            "../../../spec/functional/expressions/FR-147-evaluate-integer-division-domains.md",
        authority: AGENT_IX,
        identity: "quire.rule.fr-147/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "644e325dd6c0b614b4a53a82ed77ef7d0f41b1d7b20c59fb7b3f7f12426af867",
    },
    CatalogEntry {
        role: CatalogRole::RuleFr148,
        artifact_path:
            "../../../spec/functional/expressions/FR-148-evaluate-ieee-floating-profiles.md",
        authority: AGENT_IX,
        identity: "quire.rule.fr-148/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "012e66aa399d4f63bbf75962b27d2f522243e652b3d6c0ac72bcb3466c0f0931",
    },
    CatalogEntry {
        role: CatalogRole::RuleFr149,
        artifact_path:
            "../../../spec/functional/type-model/FR-149-apply-complete-equality-matrix.md",
        authority: AGENT_IX,
        identity: "quire.rule.fr-149/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
        digest: "721d1624e8017d111147f1187d67b233634925142574068ae416fb73ece5b271",
    },
];

/// Roles every package selects.
const ALWAYS_ROLES: [CatalogRole; 16] = [
    CatalogRole::Edition,
    CatalogRole::Root,
    CatalogRole::Accounting,
    CatalogRole::CompoundUnit,
    CatalogRole::CompoundUnitSchema,
    CatalogRole::CompoundUnitVectors,
    CatalogRole::RuleManifest,
    CatalogRole::RulePackageContract,
    CatalogRole::RuleSharedGrammar,
    CatalogRole::RuleAd005,
    CatalogRole::RuleFr140,
    CatalogRole::RuleFr141,
    CatalogRole::RuleFr142,
    CatalogRole::RuleFr147,
    CatalogRole::RuleFr148,
    CatalogRole::RuleFr149,
];

/// A role offered only when its trigger is present.
const CONDITIONAL_ROLES: [(Trigger, CatalogRole); 2] = [
    (Trigger::TextBearing, CatalogRole::TextProfile),
    (Trigger::IeeeOperation, CatalogRole::IeeeProfile),
];

/// A trigger that requires exactly one of several roles.
const EXACTLY_ONE_ROLES: [(Trigger, &[CatalogRole]); 1] = [(
    Trigger::IntegerDivRem,
    &[
        CatalogRole::IntegerDivisionEuclidean,
        CatalogRole::IntegerDivisionFloor,
        CatalogRole::IntegerDivisionTruncating,
    ],
)];

/// The closed `quire.value.definition-lock/v1` catalog and package-selection
/// rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DefinitionLock;

pub(crate) const DIGEST_DOMAIN: &str = "quire.definition.bytes/v1";

impl DefinitionLock {
    /// The closed definition lock.
    pub fn pinned() -> &'static Self {
        &Self
    }

    /// The lock's revision identifier.
    pub fn revision(&self) -> &'static str {
        "1-draft.1"
    }

    /// The closed qualification catalog.
    pub fn catalog(&self) -> &'static [CatalogEntry] {
        &CATALOG
    }

    /// The catalog entry for `role`.
    pub fn entry(&self, role: CatalogRole) -> Option<&'static CatalogEntry> {
        self.catalog().iter().find(|entry| entry.role == role)
    }

    /// Roles every package selects.
    pub fn always_roles(&self) -> &'static [CatalogRole] {
        &ALWAYS_ROLES
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
        if !ALWAYS_ROLES.iter().all(|role| selected.contains(role)) {
            return Err(SelectionRefusalCode::SelectionRequiredMissing);
        }
        if EXACTLY_ONE_ROLES
            .iter()
            .any(|(_, roles)| roles.iter().filter(|role| selected.contains(role)).count() > 1)
        {
            return Err(SelectionRefusalCode::SelectionAlternativeConflict);
        }
        let unsatisfied = CONDITIONAL_ROLES
            .iter()
            .any(|(trigger, role)| triggers.contains(trigger) && !selected.contains(role))
            || EXACTLY_ONE_ROLES.iter().any(|(trigger, roles)| {
                triggers.contains(trigger) && !roles.iter().any(|role| selected.contains(role))
            });
        if unsatisfied {
            return Err(SelectionRefusalCode::SelectionTriggerUnsatisfied);
        }
        let untriggered = CONDITIONAL_ROLES
            .iter()
            .any(|(trigger, role)| !triggers.contains(trigger) && selected.contains(role))
            || EXACTLY_ONE_ROLES.iter().any(|(trigger, roles)| {
                !triggers.contains(trigger) && roles.iter().any(|role| selected.contains(role))
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

    /// Resolve `reference` to a division law by identity and revision.
    fn resolve_division(
        &self,
        reference: &DefinitionReference,
    ) -> Result<DivisionProfile, PackageRefusal> {
        let (profile, expected) = DivisionProfile::ALL
            .into_iter()
            .find_map(|profile| {
                let entry = self.catalog().iter().find(|entry| {
                    entry.role.division_profile() == Some(profile)
                        && entry.identity == reference.identity
                })?;
                Some((profile, entry))
            })
            .ok_or(PackageRefusal::invalid_package(
                PackageCause::IncompatibleDefinition,
            ))?;
        let cause = if reference.authority != expected.authority {
            Some(PackageCause::IncompatibleDefinition)
        } else if reference.revision.namespace != expected.revision_namespace
            || reference.revision.value != expected.revision_value
        {
            Some(PackageCause::RevisionMismatch)
        } else if reference.digest_domain != DIGEST_DOMAIN {
            Some(PackageCause::DigestDomainMismatch)
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

/// Evaluate paired `div`/`rem` under the admitted package law.
pub fn divide(
    selection: &AdmittedIntegerDivision,
    dividend: &Integer,
    divisor: &Integer,
    domain: &IntegerDomain,
    meter: &mut Meter,
) -> Outcome<QuotientRemainder> {
    quire_exact::divide(selection.profile(), dividend, divisor, domain, meter)
}

/// Evaluate `mod`: always the Euclidean remainder, independent of any selected
/// `div`/`rem` law, charged only at the four `integer-modulus.*` points.
pub fn modulo(
    dividend: &Integer,
    divisor: &Integer,
    domain: &IntegerDomain,
    meter: &mut Meter,
) -> Outcome<Integer> {
    quire_exact::modulo(dividend, divisor, domain, meter)
}

/// An IEEE package whose profile definition closure was admitted. Only
/// [`DefinitionLock::admit_ieee_profile`] constructs it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AdmittedIeeeProfile {
    definition: DefinitionReference,
}

impl AdmittedIeeeProfile {
    /// The retained, admitted DefinitionRef.
    pub fn definition(&self) -> &DefinitionReference {
        &self.definition
    }
}

impl DefinitionLock {
    /// Admit a checked package's IEEE profile closure.
    ///
    /// `retained` lists every DefinitionRef the package retains as IEEE policy;
    /// `declarations` lists the qualified identities of user declarations. A
    /// missing, repeated or mismatched profile, or a user declaration bound to a
    /// reserved intrinsic identity, is `refused { code: invalid_package }`.
    pub fn admit_ieee_profile(
        &self,
        retained: &[DefinitionReference],
        declarations: &[&str],
    ) -> Result<AdmittedIeeeProfile, PackageRefusal> {
        let expected = self
            .entry(CatalogRole::IeeeProfile)
            .ok_or(PackageRefusal::invalid_package(PackageCause::MissingMember))?;
        for reference in retained {
            let cause = if reference.identity != expected.identity
                || reference.authority != expected.authority
            {
                Some(PackageCause::IncompatibleDefinition)
            } else if reference.revision.namespace != expected.revision_namespace
                || reference.revision.value != expected.revision_value
            {
                Some(PackageCause::RevisionMismatch)
            } else if reference.digest_domain != DIGEST_DOMAIN {
                Some(PackageCause::DigestDomainMismatch)
            } else {
                None
            };
            if let Some(cause) = cause {
                return Err(PackageRefusal::invalid_package(cause));
            }
        }
        let definition = match retained {
            [] => return Err(PackageRefusal::invalid_package(PackageCause::MissingMember)),
            [definition] => definition.clone(),
            [_, _, ..] => {
                return Err(PackageRefusal::invalid_package(
                    PackageCause::ConflictingDefinition,
                ))
            }
        };
        // FR-148: a user declaration bound to a reserved intrinsic identity is
        // `invalid_package` with cause `conflicting-definition`.
        if declarations
            .iter()
            .any(|declared| ieee_intrinsic_identities().any(|reserved| reserved == *declared))
        {
            return Err(PackageRefusal::invalid_package(
                PackageCause::ConflictingDefinition,
            ));
        }
        Ok(AdmittedIeeeProfile { definition })
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
