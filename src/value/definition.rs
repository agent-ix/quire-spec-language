// SPDX-License-Identifier: AGPL-3.0-or-later
//! The closed `quire.value.definition-lock/v1` catalog, per-package selection
//! admission and integer-division profile admission.
//!
//! Profile misuse is refused here, at semantic admission, before any expression
//! is evaluated. Selection refusals use the lock's closed
//! `selection_refusal_codes`; definition-closure refusals use the I04
//! `invalid_package` code with its closed `cause_tag` vocabulary.
//!
//! QSL-169 (PLAT-887): this catalog used to be parsed at compile time from a
//! vendored `complete-value-lock.json` copied from the private
//! `quire-specification` repository. Two of its four sections
//! (`trigger_vocabulary`, `selection_refusal_codes`) were tautological --
//! each deserializes as `Vec<Trigger>`/`Vec<SelectionRefusalCode>`, so no
//! spelling outside the enum could ever reach the equality check that
//! "verified" it -- and the other two (`qualification_catalog`,
//! `package_selection`) are real data with no vendored-bytes dependency:
//! a role's identity, revision and artifact path, and which roles a package
//! selection requires, offers conditionally or picks exactly one of. All
//! four are Rust-native below. The vendored catalog's per-role `digest` --
//! a SHA-256 over the private standard document's exact bytes -- is not
//! carried forward: recognizing a caller-supplied [`DefinitionReference`]
//! by identity and revision is the check; freezing it to byte-identical
//! private content is the defect PLAT-887 exists to remove.

use std::collections::BTreeSet;

use serde::Deserialize;

use super::division::DivisionProfile;

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
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct DefinitionRevision {
    /// Revision namespace.
    pub namespace: String,
    /// Revision value.
    pub value: String,
}

/// A retained DefinitionRef as it appears in a checked package. This is
/// untrusted data; only admission turns it into authority.
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

/// One qualification-catalog entry: a closed role's artifact path and the
/// exact identity/revision a caller-supplied [`DefinitionReference`] for that
/// role is checked against (QSL-169). There is no digest here: recognizing a
/// definition is by identity and revision, not by freezing it to a private
/// standard document's exact bytes.
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
}

const AGENT_IX: &str = "agent-ix";
const DRAFT: &str = "quire-draft";

/// The closed qualification catalog, ported from the removed vendored
/// `complete-value-lock.json`'s `qualification_catalog` (QSL-169).
const CATALOG: [CatalogEntry; 21] = [
    CatalogEntry {
        role: CatalogRole::Edition,
        artifact_path: "edition.md",
        authority: AGENT_IX,
        identity: "ix:native",
        revision_namespace: DRAFT,
        revision_value: "1-draft.2",
    },
    CatalogEntry {
        role: CatalogRole::Root,
        artifact_path: "value-complete.md",
        authority: AGENT_IX,
        identity: "quire.value.complete/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::Accounting,
        artifact_path: "value-accounting.md",
        authority: AGENT_IX,
        identity: "quire.value.accounting/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::CompoundUnit,
        artifact_path: "value-compound-unit.md",
        authority: AGENT_IX,
        identity: "quire.value.compound-unit/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::CompoundUnitSchema,
        artifact_path: "value-compound-unit.schema.json",
        authority: AGENT_IX,
        identity: "quire.value.compound-unit.schema/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::CompoundUnitVectors,
        artifact_path: "value-compound-unit-vectors.json",
        authority: AGENT_IX,
        identity: "quire.value.compound-unit.vectors/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::RuleManifest,
        artifact_path: "value-complete-rules.json",
        authority: AGENT_IX,
        identity: "quire.value.complete.rules/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::TextProfile,
        artifact_path: "value-text-unicode-17.md",
        authority: AGENT_IX,
        identity: "quire.value.text.unicode-17.0.0/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::IeeeProfile,
        artifact_path: "value-ieee754-2019-default.md",
        authority: AGENT_IX,
        identity: "quire.value.ieee754-2019-default/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::IntegerDivisionEuclidean,
        artifact_path: "value-integer-division-euclidean.md",
        authority: AGENT_IX,
        identity: "quire.value.integer-division.euclidean/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::IntegerDivisionFloor,
        artifact_path: "value-integer-division-floor.md",
        authority: AGENT_IX,
        identity: "quire.value.integer-division.floor/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::IntegerDivisionTruncating,
        artifact_path: "value-integer-division-truncating.md",
        authority: AGENT_IX,
        identity: "quire.value.integer-division.truncating/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::RulePackageContract,
        artifact_path: "../package-contract.md",
        authority: AGENT_IX,
        identity: "quire.rule.package-contract/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::RuleSharedGrammar,
        artifact_path: "../shared-grammar.md",
        authority: AGENT_IX,
        identity: "quire.rule.shared-grammar/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::RuleAd005,
        artifact_path: "../../../spec/assurance/AD-005-complete-value-expression-system.md",
        authority: AGENT_IX,
        identity: "quire.rule.ad-005/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::RuleFr140,
        artifact_path: "../../../spec/functional/type-model/FR-140-evaluate-exact-decimals.md",
        authority: AGENT_IX,
        identity: "quire.rule.fr-140/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::RuleFr141,
        artifact_path:
            "../../../spec/functional/type-model/FR-141-evaluate-text-and-enumerations.md",
        authority: AGENT_IX,
        identity: "quire.rule.fr-141/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::RuleFr142,
        artifact_path:
            "../../../spec/functional/type-model/FR-142-evaluate-quantities-and-units.md",
        authority: AGENT_IX,
        identity: "quire.rule.fr-142/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::RuleFr147,
        artifact_path:
            "../../../spec/functional/expressions/FR-147-evaluate-integer-division-domains.md",
        authority: AGENT_IX,
        identity: "quire.rule.fr-147/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::RuleFr148,
        artifact_path:
            "../../../spec/functional/expressions/FR-148-evaluate-ieee-floating-profiles.md",
        authority: AGENT_IX,
        identity: "quire.rule.fr-148/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
    CatalogEntry {
        role: CatalogRole::RuleFr149,
        artifact_path:
            "../../../spec/functional/type-model/FR-149-apply-complete-equality-matrix.md",
        authority: AGENT_IX,
        identity: "quire.rule.fr-149/v1",
        revision_namespace: DRAFT,
        revision_value: "1-draft.1",
    },
];

/// Roles every package selects (the removed lock's `package_selection.always`).
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

/// A role offered only when its trigger is present (`package_selection.conditional`).
const CONDITIONAL_ROLES: [(Trigger, CatalogRole); 2] = [
    (Trigger::TextBearing, CatalogRole::TextProfile),
    (Trigger::IeeeOperation, CatalogRole::IeeeProfile),
];

/// A trigger that requires exactly one of several roles (`package_selection.exactly_one`).
const EXACTLY_ONE_ROLES: [(Trigger, &[CatalogRole]); 1] = [(
    Trigger::IntegerDivRem,
    &[
        CatalogRole::IntegerDivisionEuclidean,
        CatalogRole::IntegerDivisionFloor,
        CatalogRole::IntegerDivisionTruncating,
    ],
)];

/// The closed `quire.value.definition-lock/v1` catalog and package-selection
/// rules (QSL-169: Rust-native data, ported from the removed vendored
/// `complete-value-lock.json`; see the module doc for what was and was not
/// carried forward).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DefinitionLock;

impl DefinitionLock {
    /// The closed definition lock. Infallible: there is nothing left to parse.
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

    /// Resolve `reference` to a division law by identity and revision. The
    /// removed vendored lock additionally compared a per-role digest over the
    /// private standard document's exact bytes; QSL-169 (PLAT-887) drops that
    /// comparison as the same byte-freeze this repository does not vendor.
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

pub(crate) const DIGEST_DOMAIN: &str = "quire.definition.bytes/v1";

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
