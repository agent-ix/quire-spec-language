// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec's `quire.value.definition-lock/v1` catalog, per-package selection
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
//! [`DefinitionLock::pinned`] reads QSpec's `complete-value-lock.json` by
//! reference, from the `quire_specification` crate's compiled-in bytes: each
//! catalog row names the authority, identity, revision, artifact path and
//! raw-byte digest of one role's definition, and the package-selection rules
//! and trigger vocabulary come from the same document. QSL holds no copy of
//! any of it. Admission recognizes a caller-supplied [`DefinitionReference`]
//! by identity and revision. The v2 emitter writes each row as the package
//! lock's edition and definition selections ([`CatalogEntry::reference`]).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use quire_semantic_value::definition::{PackageCause, PackageRefusal, SelectionRefusalCode};

use quire_exact::{
    ieee_intrinsic_identities, DivisionProfile, Integer, IntegerDomain, Meter, Outcome,
    QuotientRemainder,
};

/// The lock's `trigger_vocabulary`. [`DefinitionLock::read`] refuses a lock
/// whose vocabulary is not exactly [`Trigger::ALL`].
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
    /// The closed vocabulary.
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
    /// The language edition definition.
    Edition,
    /// The root value-system definition, the one row a header profile selects.
    Root,
    /// The charge-accounting definition.
    Accounting,
    /// The compound-unit definition.
    CompoundUnit,
    /// The JSON Schema for compound units.
    CompoundUnitSchema,
    /// The compound-unit test vectors.
    CompoundUnitVectors,
    /// The manifest listing the always-selected rule roles.
    RuleManifest,
    /// The text profile, selected when the package is `text_bearing`.
    TextProfile,
    /// The IEEE binary32/binary64 profile, selected when the package is
    /// `ieee_operation`.
    IeeeProfile,
    /// The Euclidean `div`/`rem` law.
    IntegerDivisionEuclidean,
    /// The floored `div`/`rem` law.
    IntegerDivisionFloor,
    /// The truncating `div`/`rem` law.
    IntegerDivisionTruncating,
    /// The package-contract rule.
    RulePackageContract,
    /// The shared-grammar rule.
    RuleSharedGrammar,
    /// The rule binding the complete-value expression system.
    RuleAd005,
    /// The rule binding exact-decimal evaluation.
    RuleFr140,
    /// The rule binding text and enumeration evaluation.
    RuleFr141,
    /// The rule binding quantity and unit evaluation.
    RuleFr142,
    /// The rule binding integer-division-domain evaluation.
    RuleFr147,
    /// The rule binding IEEE floating-point profile evaluation.
    RuleFr148,
    /// The rule binding the complete equality matrix.
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
/// is checked against, and the definition's raw-byte digest, all read from
/// QSpec's `complete-value-lock.json`.
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
    /// Digest domain (`quire.definition.bytes/v1`).
    pub digest_domain: &'static str,
    /// Lowercase hex SHA-256 of the definition's bytes. FR-110 compares a
    /// header profile's digest with the `root` row's.
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
            digest_domain: self.digest_domain.to_owned(),
            digest: self.digest.to_owned(),
        }
    }
}

/// The parts of QSpec's lock this reader uses, borrowed from the compiled-in
/// bytes.
#[derive(Deserialize)]
struct LockDocument<'a> {
    #[serde(borrow)]
    revision: &'a str,
    #[serde(borrow)]
    trigger_vocabulary: Vec<&'a str>,
    #[serde(borrow)]
    selection_refusal_codes: Vec<&'a str>,
    #[serde(borrow)]
    qualification_catalog: Vec<LockEntry<'a>>,
    #[serde(borrow)]
    package_selection: LockSelection<'a>,
}

#[derive(Deserialize)]
struct LockEntry<'a> {
    #[serde(borrow)]
    role: &'a str,
    #[serde(borrow)]
    artifact_path: &'a str,
    #[serde(borrow)]
    definition: LockDefinition<'a>,
}

#[derive(Deserialize)]
struct LockDefinition<'a> {
    #[serde(borrow)]
    authority: &'a str,
    #[serde(borrow)]
    identity: &'a str,
    #[serde(borrow)]
    revision: LockRevision<'a>,
    #[serde(borrow)]
    digest_domain: &'a str,
    #[serde(borrow)]
    digest: &'a str,
}

#[derive(Deserialize)]
struct LockRevision<'a> {
    #[serde(borrow)]
    namespace: &'a str,
    #[serde(borrow)]
    value: &'a str,
}

#[derive(Deserialize)]
struct LockSelection<'a> {
    #[serde(borrow)]
    always: Vec<&'a str>,
    #[serde(borrow)]
    conditional: Vec<LockConditional<'a>>,
    #[serde(borrow)]
    exactly_one: Vec<LockExactlyOne<'a>>,
}

#[derive(Deserialize)]
struct LockConditional<'a> {
    #[serde(borrow)]
    trigger: &'a str,
    #[serde(borrow)]
    role: &'a str,
}

#[derive(Deserialize)]
struct LockExactlyOne<'a> {
    #[serde(borrow)]
    trigger: &'a str,
    #[serde(borrow)]
    roles: Vec<&'a str>,
}

/// Why QSpec's lock bytes do not read as this crate's [`DefinitionLock`].
#[derive(Debug, thiserror::Error)]
pub enum LockReadError {
    /// The bytes are not the lock's JSON shape.
    #[error("the lock is not well-formed: {0}")]
    Malformed(#[from] serde_json::Error),
    /// The lock names a role [`CatalogRole`] does not have.
    #[error("the lock names unknown role `{0}`")]
    UnknownRole(String),
    /// A [`CatalogRole`] has no catalog row, or more than one.
    #[error("role `{}` has {count} catalog rows, not one", role.as_str())]
    RoleRowCount {
        /// The role.
        role: CatalogRole,
        /// How many rows name it.
        count: usize,
    },
    /// The package-selection rules name a trigger outside [`Trigger::ALL`].
    #[error("the package-selection rules name unknown trigger `{0}`")]
    UnknownTrigger(String),
    /// `trigger_vocabulary` is not exactly [`Trigger::ALL`].
    #[error("trigger_vocabulary {0:?} is not the closed trigger set")]
    TriggerVocabulary(Vec<String>),
    /// `selection_refusal_codes` is not exactly [`SelectionRefusalCode::ALL`].
    #[error("selection_refusal_codes {0:?} is not the closed refusal set")]
    RefusalCodes(Vec<String>),
}

/// QSpec's `quire.value.definition-lock/v1` catalog and package-selection
/// rules, read from `complete-value-lock.json` by reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinitionLock {
    revision: &'static str,
    catalog: Vec<CatalogEntry>,
    always: Vec<CatalogRole>,
    conditional: Vec<(Trigger, CatalogRole)>,
    exactly_one: Vec<(Trigger, Vec<CatalogRole>)>,
}

fn role(code: &str) -> Result<CatalogRole, LockReadError> {
    CatalogRole::from_code(code).ok_or_else(|| LockReadError::UnknownRole(code.to_owned()))
}

impl DefinitionLock {
    /// QSpec's lock, read from the compiled-in
    /// `quire_specification::COMPLETE_VALUE_LOCK` bytes on each call. A
    /// caller that needs it more than once reads it once and passes it down.
    ///
    /// # Panics
    ///
    /// If those compiled-in bytes do not read ([`DefinitionLock::read`]);
    /// `the_compiled_in_lock_reads` holds them to it.
    pub fn pinned() -> Self {
        Self::read(quire_specification::COMPLETE_VALUE_LOCK)
            .unwrap_or_else(|error| panic!("QSpec's complete-value-lock.json: {error}"))
    }

    /// Read a `quire.value.definition-lock/v1` document, borrowing its
    /// strings. Every lock role must be a [`CatalogRole`] with exactly one
    /// row, `trigger_vocabulary` must be exactly [`Trigger::ALL`] and name
    /// every selection trigger, and `selection_refusal_codes` must be the
    /// closed refusal set.
    pub fn read(bytes: &'static str) -> Result<Self, LockReadError> {
        let document: LockDocument<'static> = serde_json::from_str(bytes)?;
        let catalog = document
            .qualification_catalog
            .iter()
            .map(|entry| {
                let definition = &entry.definition;
                Ok(CatalogEntry {
                    role: role(entry.role)?,
                    artifact_path: entry.artifact_path,
                    authority: definition.authority,
                    identity: definition.identity,
                    revision_namespace: definition.revision.namespace,
                    revision_value: definition.revision.value,
                    digest_domain: definition.digest_domain,
                    digest: definition.digest,
                })
            })
            .collect::<Result<Vec<_>, LockReadError>>()?;
        for role in CatalogRole::ALL {
            let count = catalog.iter().filter(|entry| entry.role == role).count();
            if count != 1 {
                return Err(LockReadError::RoleRowCount { role, count });
            }
        }
        let vocabulary: BTreeSet<&str> = document.trigger_vocabulary.iter().copied().collect();
        let closed: BTreeSet<&str> = Trigger::ALL.into_iter().map(Trigger::as_str).collect();
        if vocabulary != closed || vocabulary.len() != document.trigger_vocabulary.len() {
            return Err(LockReadError::TriggerVocabulary(
                document
                    .trigger_vocabulary
                    .iter()
                    .map(|code| (*code).to_owned())
                    .collect(),
            ));
        }
        let trigger = |code: &str| {
            Trigger::from_code(code).ok_or_else(|| LockReadError::UnknownTrigger(code.to_owned()))
        };
        let selection = document.package_selection;
        let always = selection
            .always
            .iter()
            .map(|code| role(code))
            .collect::<Result<Vec<_>, _>>()?;
        let conditional = selection
            .conditional
            .iter()
            .map(|rule| Ok((trigger(rule.trigger)?, role(rule.role)?)))
            .collect::<Result<Vec<_>, LockReadError>>()?;
        let exactly_one = selection
            .exactly_one
            .iter()
            .map(|rule| {
                let roles = rule
                    .roles
                    .iter()
                    .map(|code| role(code))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok((trigger(rule.trigger)?, roles))
            })
            .collect::<Result<Vec<_>, LockReadError>>()?;
        let refusals: BTreeSet<&str> = document.selection_refusal_codes.iter().copied().collect();
        let closed: BTreeSet<&str> = SelectionRefusalCode::ALL
            .into_iter()
            .map(SelectionRefusalCode::as_str)
            .collect();
        if refusals != closed || refusals.len() != document.selection_refusal_codes.len() {
            return Err(LockReadError::RefusalCodes(
                document
                    .selection_refusal_codes
                    .iter()
                    .map(|code| (*code).to_owned())
                    .collect(),
            ));
        }
        Ok(Self {
            revision: document.revision,
            catalog,
            always,
            conditional,
            exactly_one,
        })
    }

    /// The lock's revision identifier.
    pub fn revision(&self) -> &'static str {
        self.revision
    }

    /// The closed qualification catalog, in lock order.
    pub fn catalog(&self) -> &[CatalogEntry] {
        &self.catalog
    }

    /// The catalog entry for `role`.
    pub fn entry(&self, role: CatalogRole) -> Option<&CatalogEntry> {
        self.catalog.iter().find(|entry| entry.role == role)
    }

    /// Roles every package selects.
    pub fn always_roles(&self) -> &[CatalogRole] {
        &self.always
    }

    /// Roles offered only when their trigger is present.
    pub fn conditional_roles(&self) -> &[(Trigger, CatalogRole)] {
        &self.conditional
    }

    /// Triggers that require exactly one of several roles.
    pub fn exactly_one_roles(&self) -> &[(Trigger, Vec<CatalogRole>)] {
        &self.exactly_one
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
        let triggers = Self::trigger_set(triggers)?;
        let parsed = roles
            .iter()
            .map(|code| CatalogRole::from_code(code))
            .collect::<Option<Vec<_>>>()
            .ok_or(SelectionRefusalCode::SelectionUnknownRole)?;
        let selected: BTreeSet<_> = parsed.iter().copied().collect();
        if selected.len() != parsed.len() {
            return Err(SelectionRefusalCode::SelectionDuplicateRole);
        }
        if !self.always.iter().all(|role| selected.contains(role)) {
            return Err(SelectionRefusalCode::SelectionRequiredMissing);
        }
        if self
            .exactly_one
            .iter()
            .any(|(_, roles)| roles.iter().filter(|role| selected.contains(role)).count() > 1)
        {
            return Err(SelectionRefusalCode::SelectionAlternativeConflict);
        }
        let unsatisfied = self
            .conditional
            .iter()
            .any(|(trigger, role)| triggers.contains(trigger) && !selected.contains(role))
            || self.exactly_one.iter().any(|(trigger, roles)| {
                triggers.contains(trigger) && !roles.iter().any(|role| selected.contains(role))
            });
        if unsatisfied {
            return Err(SelectionRefusalCode::SelectionTriggerUnsatisfied);
        }
        let untriggered = self
            .conditional
            .iter()
            .any(|(trigger, role)| !triggers.contains(trigger) && selected.contains(role))
            || self.exactly_one.iter().any(|(trigger, roles)| {
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
    #[qsl_attrs::string_edge]
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
        } else if reference.digest_domain != expected.digest_domain {
            Some(PackageCause::DigestDomainMismatch)
        } else {
            None
        };
        match cause {
            Some(cause) => Err(PackageRefusal::invalid_package(cause)),
            None => Ok(profile),
        }
    }

    /// Resolve each trigger spelling to its [`Trigger`].
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
    #[qsl_attrs::string_edge]
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
            } else if reference.digest_domain != expected.digest_domain {
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
