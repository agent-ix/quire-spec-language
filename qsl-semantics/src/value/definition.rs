// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `Value` family's definition catalog, per-package selection admission,
//! integer-division profile admission and IEEE profile admission.
//!
//! [`divide`] and [`modulo`] evaluate integer division under an admitted law.
//! They call `quire_exact::divide`/`modulo` and carry the kernel
//! [`Outcome`](quire_exact::Outcome) straight through.
//!
//! Profile misuse is refused here, at semantic admission, before any expression
//! is evaluated. Selection refusals use QSpec's closed
//! `selection_refusal_codes`; definition-closure refusals use the I04
//! `invalid_package` code with its closed `cause_tag` vocabulary.
//!
//! [`DefinitionLock::pinned`] is the catalog ADR-011 §2.4 names: each
//! [`CatalogRole`] names the definition the QSL build implements for it, by
//! authority and identity ([`CatalogRole::identity`]), and each role's
//! package-selection rule is QSpec FR-001's. A definition is named by its
//! authority and identity alone, so the catalog holds no revision, no digest
//! and no QSpec document. Admission recognizes a caller-supplied [`DefinitionReference`]
//! by authority and identity. The v2 emitter writes each row as the package
//! lock's edition and definition selections ([`CatalogEntry::reference`]).
//! `make conformance` compares the catalog and its selection rules with
//! QSpec's `complete-value-lock.json` and `complete-model-lock.json`, read at
//! run time.

use std::collections::BTreeSet;

use quire_canonical::FixedShape;
use serde::{Deserialize, Serialize};

use quire_semantic_value::definition::{PackageCause, PackageRefusal, SelectionRefusalCode};

use super::layer::{HeaderLayer, LayerForm, LayerRow, LAYER_ROWS};
use quire_exact::{
    ieee_intrinsic_identities, DivisionMember, DivisionProfile, Integer, IntegerDomain, Meter,
    Outcome,
};

/// QSpec's closed package-selection `trigger_vocabulary`.
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

/// A qualification-catalog role; each variant is the QSpec lock role of the
/// same name.
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

/// A role of QSpec's `complete-model-lock.json` `qualification_catalog`; each
/// variant is the lock role of the same name (FR-361 item 3). The lock
/// requires the complete-value lock, so these rows come beside
/// [`CatalogRole`]'s. The model lock's `root` is the `quire.model.complete/v1`
/// layer and its `state_graph` is the `quire.state.graph/v1` layer; neither
/// is the complete-value lock's `root` ([`CatalogRole::Root`]).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ModelRole {
    /// QSpec's `root` role.
    Root,
    /// QSpec's `state_graph` role.
    StateGraph,
    /// QSpec's `rule_manifest` role.
    RuleManifest,
    /// QSpec's `effective_declaration_schema` role.
    EffectiveDeclarationSchema,
    /// QSpec's `effective_declaration_vectors` role.
    EffectiveDeclarationVectors,
    /// QSpec's `rule_ad_006` role.
    RuleAd006,
    /// QSpec's `rule_fr_043` role.
    RuleFr043,
    /// QSpec's `rule_fr_143` role.
    RuleFr143,
    /// QSpec's `rule_fr_144` role.
    RuleFr144,
    /// QSpec's `rule_fr_150` role.
    RuleFr150,
    /// QSpec's `rule_fr_151` role.
    RuleFr151,
    /// QSpec's `rule_fr_152` role.
    RuleFr152,
    /// QSpec's `rule_fr_153` role.
    RuleFr153,
    /// QSpec's `rule_fr_154` role.
    RuleFr154,
    /// QSpec's `rule_fr_204` role.
    RuleFr204,
    /// QSpec's `rule_fr_208` role.
    RuleFr208,
    /// QSpec's `rule_fr_321` role.
    RuleFr321,
    /// QSpec's `rule_interface_003` role.
    RuleInterface003,
}

impl ModelRole {
    /// Every role in QSpec's complete-model lock catalog order.
    pub const ALL: [Self; 18] = [
        Self::Root,
        Self::StateGraph,
        Self::RuleManifest,
        Self::EffectiveDeclarationSchema,
        Self::EffectiveDeclarationVectors,
        Self::RuleAd006,
        Self::RuleFr043,
        Self::RuleFr143,
        Self::RuleFr144,
        Self::RuleFr150,
        Self::RuleFr151,
        Self::RuleFr152,
        Self::RuleFr153,
        Self::RuleFr154,
        Self::RuleFr204,
        Self::RuleFr208,
        Self::RuleFr321,
        Self::RuleInterface003,
    ];

    /// Normative spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::StateGraph => "state_graph",
            Self::RuleManifest => "rule_manifest",
            Self::EffectiveDeclarationSchema => "effective_declaration_schema",
            Self::EffectiveDeclarationVectors => "effective_declaration_vectors",
            Self::RuleAd006 => "rule_ad_006",
            Self::RuleFr043 => "rule_fr_043",
            Self::RuleFr143 => "rule_fr_143",
            Self::RuleFr144 => "rule_fr_144",
            Self::RuleFr150 => "rule_fr_150",
            Self::RuleFr151 => "rule_fr_151",
            Self::RuleFr152 => "rule_fr_152",
            Self::RuleFr153 => "rule_fr_153",
            Self::RuleFr154 => "rule_fr_154",
            Self::RuleFr204 => "rule_fr_204",
            Self::RuleFr208 => "rule_fr_208",
            Self::RuleFr321 => "rule_fr_321",
            Self::RuleInterface003 => "rule_interface_003",
        }
    }

    /// The identity of the definition the QSL build implements for this
    /// role, published by `agent-ix`.
    pub fn identity(self) -> &'static str {
        match self {
            Self::Root => "quire.model.complete/v1",
            Self::StateGraph => "quire.state.graph/v1",
            Self::RuleManifest => "quire.model.complete.rules/v1",
            Self::EffectiveDeclarationSchema => "quire.model.effective-declaration.schema/v1",
            Self::EffectiveDeclarationVectors => "quire.model.effective-declaration.vectors/v1",
            Self::RuleAd006 => "quire.rule.ad-006/v1",
            Self::RuleFr043 => "quire.rule.fr-043/v1",
            Self::RuleFr143 => "quire.rule.fr-143/v1",
            Self::RuleFr144 => "quire.rule.fr-144/v1",
            Self::RuleFr150 => "quire.rule.fr-150/v1",
            Self::RuleFr151 => "quire.rule.fr-151/v1",
            Self::RuleFr152 => "quire.rule.fr-152/v1",
            Self::RuleFr153 => "quire.rule.fr-153/v1",
            Self::RuleFr154 => "quire.rule.fr-154/v1",
            Self::RuleFr204 => "quire.rule.fr-204/v1",
            Self::RuleFr208 => "quire.rule.fr-208/v1",
            Self::RuleFr321 => "quire.rule.fr-321/v1",
            Self::RuleInterface003 => "quire.rule.interface_003/v1",
        }
    }
}

/// One complete-model lock catalog entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelEntry {
    /// Catalog role.
    pub role: ModelRole,
    /// Publishing authority.
    pub authority: &'static str,
    /// Definition identity.
    pub identity: &'static str,
}

impl ModelEntry {
    /// This entry as the `DefinitionRef` a package lock selects.
    pub fn reference(&self) -> DefinitionReference {
        DefinitionReference {
            authority: self.authority.to_owned(),
            identity: self.identity.to_owned(),
        }
    }
}

/// How QSpec FR-001's package-selection rules select one role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectionRule {
    /// Every package selects the role.
    Always,
    /// A package selects the role exactly when the trigger is present.
    When(Trigger),
    /// A package with the trigger selects exactly one of the roles that
    /// share it; one without the trigger selects none of them.
    OneOf(Trigger),
}

/// The lock's `package_selection.without_layer` rows: the roles a package
/// selects when its layers do not hold `package_selection.layer`.
const WITHOUT_LAYER: [CatalogRole; 1] = [CatalogRole::Edition];

/// The publishing authority of every catalog definition.
const AUTHORITY: &str = "agent-ix";

impl CatalogRole {
    /// Every role in QSpec's lock catalog order.
    pub const ALL: [Self; 20] = [
        Self::Edition,
        Self::Root,
        Self::Accounting,
        Self::CompoundUnit,
        Self::CompoundUnitSchema,
        Self::CompoundUnitVectors,
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

    /// The identity of the definition the QSL build implements for this
    /// role, published by `agent-ix` (ADR-011 §2.4).
    pub fn identity(self) -> &'static str {
        match self {
            Self::Edition => "ix:native",
            Self::Root => "quire.value.complete/v1",
            Self::Accounting => "quire.value.accounting/v1",
            Self::CompoundUnit => "quire.value.compound-unit/v1",
            Self::CompoundUnitSchema => "quire.value.compound-unit.schema/v1",
            Self::CompoundUnitVectors => "quire.value.compound-unit.vectors/v1",
            Self::TextProfile => "quire.value.text.unicode-17.0.0/v1",
            Self::IeeeProfile => "quire.value.ieee754-2019-default/v1",
            Self::IntegerDivisionEuclidean => "quire.value.integer-division.euclidean/v1",
            Self::IntegerDivisionFloor => "quire.value.integer-division.floor/v1",
            Self::IntegerDivisionTruncating => "quire.value.integer-division.truncating/v1",
            Self::RulePackageContract => "quire.rule.package-contract/v1",
            Self::RuleSharedGrammar => "quire.rule.shared-grammar/v1",
            Self::RuleAd005 => "quire.rule.ad-005/v1",
            Self::RuleFr140 => "quire.rule.fr-140/v1",
            Self::RuleFr141 => "quire.rule.fr-141/v1",
            Self::RuleFr142 => "quire.rule.fr-142/v1",
            Self::RuleFr147 => "quire.rule.fr-147/v1",
            Self::RuleFr148 => "quire.rule.fr-148/v1",
            Self::RuleFr149 => "quire.rule.fr-149/v1",
        }
    }

    /// The QSpec FR-001 package-selection rule that selects this role.
    fn selection_rule(self) -> SelectionRule {
        match self {
            Self::TextProfile => SelectionRule::When(Trigger::TextBearing),
            Self::IeeeProfile => SelectionRule::When(Trigger::IeeeOperation),
            Self::IntegerDivisionEuclidean
            | Self::IntegerDivisionFloor
            | Self::IntegerDivisionTruncating => SelectionRule::OneOf(Trigger::IntegerDivRem),
            Self::Edition
            | Self::Root
            | Self::Accounting
            | Self::CompoundUnit
            | Self::CompoundUnitSchema
            | Self::CompoundUnitVectors
            | Self::RulePackageContract
            | Self::RuleSharedGrammar
            | Self::RuleAd005
            | Self::RuleFr140
            | Self::RuleFr141
            | Self::RuleFr142
            | Self::RuleFr147
            | Self::RuleFr148
            | Self::RuleFr149 => SelectionRule::Always,
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

/// A retained DefinitionRef as it appears in a checked package: QSpec's
/// closed `{authority, identity}`, with no revision and no digest. This is
/// untrusted data; only admission turns it into authority.
#[derive(
    Clone, Debug, Deserialize, Eq, Serialize, FixedShape, Hash, Ord, PartialEq, PartialOrd,
)]
#[serde(deny_unknown_fields)]
pub struct DefinitionReference {
    /// Publishing authority.
    pub authority: String,
    /// Definition identity.
    pub identity: String,
}

/// One qualification-catalog entry: a closed role and the authority and
/// identity a caller-supplied [`DefinitionReference`] for that role is
/// checked against.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatalogEntry {
    /// Catalog role.
    pub role: CatalogRole,
    /// Publishing authority.
    pub authority: &'static str,
    /// Definition identity.
    pub identity: &'static str,
}

impl CatalogEntry {
    /// This entry as the `DefinitionRef` a package lock selects.
    pub fn reference(&self) -> DefinitionReference {
        DefinitionReference {
            authority: self.authority.to_owned(),
            identity: self.identity.to_owned(),
        }
    }
}

/// The `Value` family's catalog: one entry per [`CatalogRole`] and QSpec
/// FR-001's package-selection rules over them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinitionLock {
    catalog: [CatalogEntry; CatalogRole::ALL.len()],
    always: Vec<CatalogRole>,
    conditional: Vec<(Trigger, CatalogRole)>,
    exactly_one: Vec<(Trigger, Vec<CatalogRole>)>,
    layers: [LayerRow; HeaderLayer::ALL.len()],
    model_catalog: [ModelEntry; ModelRole::ALL.len()],
}

impl DefinitionLock {
    /// The catalog of the definitions the QSL build implements: each role's
    /// entry and its selection rule, in catalog order.
    pub fn pinned() -> Self {
        let catalog = CatalogRole::ALL.map(|role| CatalogEntry {
            role,
            authority: AUTHORITY,
            identity: role.identity(),
        });
        let mut always = Vec::new();
        let mut conditional = Vec::new();
        let mut exactly_one: Vec<(Trigger, Vec<CatalogRole>)> = Vec::new();
        for role in CatalogRole::ALL {
            match role.selection_rule() {
                SelectionRule::Always => always.push(role),
                SelectionRule::When(trigger) => conditional.push((trigger, role)),
                SelectionRule::OneOf(trigger) => {
                    match exactly_one.iter_mut().find(|(group, _)| *group == trigger) {
                        Some((_, roles)) => roles.push(role),
                        None => exactly_one.push((trigger, vec![role])),
                    }
                }
            }
        }
        Self {
            catalog,
            always,
            conditional,
            exactly_one,
            layers: LAYER_ROWS,
            model_catalog: ModelRole::ALL.map(|role| ModelEntry {
                role,
                authority: AUTHORITY,
                identity: role.identity(),
            }),
        }
    }

    /// The `header_selectable_layers` rows, in the lock's order.
    pub fn layers(&self) -> &[LayerRow] {
        &self.layers
    }

    /// The row of `layer`. The table has one row per layer, in the
    /// declaration order of [`HeaderLayer::ALL`], so the lookup is total.
    pub fn layer_row(&self, layer: HeaderLayer) -> &LayerRow {
        &self.layers[layer as usize]
    }

    /// The `DefinitionRef` a package lock selects for `layer`.
    pub fn layer_reference(&self, layer: HeaderLayer) -> DefinitionReference {
        DefinitionReference {
            authority: AUTHORITY.to_owned(),
            identity: layer.identity().to_owned(),
        }
    }

    /// The `requires` closure of `selected`: the selected layers and every
    /// layer they require, transitively, in the lock's row order. A layer
    /// that requires a selected layer is not in it.
    pub fn closure(&self, selected: &[HeaderLayer]) -> Vec<HeaderLayer> {
        let mut reached: BTreeSet<HeaderLayer> = BTreeSet::new();
        let mut pending: Vec<HeaderLayer> = selected.to_vec();
        while let Some(layer) = pending.pop() {
            if reached.insert(layer) {
                pending.extend_from_slice(self.layer_row(layer).requires);
            }
        }
        reached.into_iter().collect()
    }

    /// The admitted-form set of `selected`: the forms of every layer in its
    /// `requires` closure, and no form of a wider layer.
    pub fn admitted_forms(&self, selected: HeaderLayer) -> BTreeSet<LayerForm> {
        self.closure(&[selected])
            .into_iter()
            .flat_map(|layer| self.layer_row(layer).forms.iter().copied())
            .collect()
    }

    /// Whether `selected`'s admitted-form set holds `form`.
    pub fn admits(&self, selected: HeaderLayer, form: LayerForm) -> bool {
        self.admitted_forms(selected).contains(&form)
    }

    /// The qualification-catalog roles QSpec FR-001 selects for a union of
    /// layers: the `always` rules when the union holds `quire.value.complete/v1`
    /// (the lock's `package_selection.layer`), and the `without_layer` rows
    /// otherwise.
    pub fn qualification_roles(&self, union: &[HeaderLayer]) -> &[CatalogRole] {
        if union.contains(&HeaderLayer::ValueComplete) {
            &self.always
        } else {
            &WITHOUT_LAYER
        }
    }

    /// The complete-model lock's catalog, in its order. Its
    /// `package_selection.always` rows are every row: the lock has no
    /// conditional or exactly-one rule.
    pub fn model_catalog(&self) -> &[ModelEntry] {
        &self.model_catalog
    }

    /// The closed qualification catalog, in catalog order.
    pub fn catalog(&self) -> &[CatalogEntry] {
        &self.catalog
    }

    /// The catalog entry for `role`. The catalog has one entry per role, in
    /// the declaration order of [`CatalogRole::ALL`], so the lookup is total.
    pub fn entry(&self, role: CatalogRole) -> &CatalogEntry {
        &self.catalog[role as usize]
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

    /// Resolve `reference` to a division law by authority and identity.
    #[ix_trace_rs::string_edge]
    fn resolve_division(
        &self,
        reference: &DefinitionReference,
    ) -> Result<DivisionProfile, PackageRefusal> {
        DivisionProfile::ALL
            .into_iter()
            .find(|profile| {
                self.catalog().iter().any(|entry| {
                    entry.role.division_profile() == Some(*profile)
                        && entry.authority == reference.authority
                        && entry.identity == reference.identity
                })
            })
            .ok_or(PackageRefusal::invalid_package(
                PackageCause::IncompatibleDefinition,
            ))
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

/// Evaluate the one member of `div`/`rem` an expression exposes (`div` is the
/// quotient, `rem` the remainder) under the admitted package law. Both members
/// are computed exactly; only the exposed `member` must lie in `domain`.
pub fn divide(
    selection: &AdmittedIntegerDivision,
    member: DivisionMember,
    dividend: &Integer,
    divisor: &Integer,
    domain: &IntegerDomain,
    meter: &mut Meter,
) -> Outcome<Integer> {
    quire_exact::divide(
        selection.profile(),
        member,
        dividend,
        divisor,
        domain,
        meter,
    )
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
    #[ix_trace_rs::string_edge]
    pub fn admit_ieee_profile(
        &self,
        retained: &[DefinitionReference],
        declarations: &[&str],
    ) -> Result<AdmittedIeeeProfile, PackageRefusal> {
        let expected = self.entry(CatalogRole::IeeeProfile);
        if retained.iter().any(|reference| {
            reference.identity != expected.identity || reference.authority != expected.authority
        }) {
            return Err(PackageRefusal::invalid_package(
                PackageCause::IncompatibleDefinition,
            ));
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
