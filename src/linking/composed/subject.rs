// SPDX-License-Identifier: AGPL-3.0-only
//! FR-036: compare the static subject through its declared semantic components.
//!
//! The [package contract][contract] names the static components as Sources,
//! Language, Profiles, Models, Native dependencies and Binding requirements.
//! This module retains exactly those components from a completed binding report
//! and compares two subjects component by component.
//!
//! No canonical digest, structural hash or JSON fingerprint is derived here.
//! The contract states that canonical identity is unavailable until its exact
//! domain, version and algorithm are selected and qualified; an equality over
//! the declared components is the only comparison this stage may claim.
//!
//! Build provenance is retained separately. A resource-only configuration change
//! is visible in [`BuildProvenance`] and changes no static component.
//!
//! [contract]: ../../../../resources/native-v1/proposals/quire-v1/package-contract.md

use super::binding;
use super::binding_work;
use super::definition_source::RegisteredDefinition;
use super::definitions::{Cause, Selection, UseKind};
use super::models::{ImportRefusal, ModelInput};
use super::scopes::{Anchor, BinderKind, BinderType};
use super::{DependencyKind, DependencySite, SourceInventory, WorkLimits};
use crate::{ByteDigest, Limits, SourceIdentity};

/// Which declared static component of two subjects differs.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ComponentKind {
    /// Exact authority, identity, revision and byte digest of every source unit.
    Sources,
    /// Exact language/edition selection and its supplied definition artifact.
    Language,
    /// Each declaration's selected root definition and required closure.
    Profiles,
    /// Each authored model import and the exact artifact selection it resolved.
    Models,
    /// Resolved predicate, operation-contract and temporal-obligation references.
    NativeDependencies,
    /// Typed roles with their kinds, anchors and retained type inputs.
    BindingRequirements,
}

/// Sources component: one selected source unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceComponent {
    /// Editable authority from the explicit inventory, never a file path.
    pub authority: String,
    /// Exact selected source identity and revision.
    pub identity: SourceIdentity,
    /// SHA-256 of the unit's original bytes.
    pub digest: ByteDigest,
}

/// Language component: the single selection every source header must agree with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageComponent {
    /// Inventory language selection.
    pub language: String,
    /// Inventory edition selection.
    pub edition: String,
    /// Exact supplied edition definition identity, revision and byte digest.
    pub definition: Selection,
    /// Checked edition closure, empty when the edition selection refused.
    pub closure: Vec<Selection>,
    /// Edition-level refusal, itself part of the subject's declared meaning.
    pub refusal: Option<Cause>,
}

/// Profiles component: one authored profile occurrence and its exact closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileComponent {
    /// Package-wide native declaration name owning the occurrence.
    pub declaration: String,
    /// Required interpretation at this occurrence.
    pub kind: UseKind,
    /// Unit-local alias, retained for diagnostics rather than as a lookup key.
    pub alias: String,
    /// Root-first closure as exact identity/revision/digest selections.
    pub closure: Vec<Selection>,
    /// First failed selection in this occurrence's closure, if any.
    pub refusal: Option<Cause>,
}

/// The exact compiled model artifact an import resolved to, in its own digest
/// domain. A producer canonical digest never occupies this position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelSelection {
    /// Package label of the admitted compiled model document.
    pub package: String,
    /// The revision exactly as the selected input states it. It is retained as
    /// its original spelling so that two differing revisions can never be
    /// coerced into one comparable number.
    pub revision: String,
    /// SHA-256 of that document's exact bytes.
    pub digest: ByteDigest,
}

/// Models component: one authored import and the selection it resolved to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelComponent {
    /// Editable authority of the unit declaring the alias.
    pub authority: String,
    /// Unit-local alias spelling.
    pub alias: String,
    /// Resolved artifact selection, or the exact typed selection refusal.
    pub selection: Result<ModelSelection, ImportRefusal>,
}

/// Native dependencies component: one resolved cross-declaration reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyComponent {
    /// Package-wide name of the referring declaration.
    pub declaration: String,
    /// Required native declaration kind at this occurrence.
    pub kind: DependencyKind,
    /// Typed syntax occurrence in the referring unit.
    pub site: DependencySite,
    /// Authored name token spelling, never reconstructed from a target.
    pub name: String,
    /// Package-wide name of the unique resolved target, if one was found.
    pub target: Option<String>,
}

/// Binding requirements component: one declaration-owned typed role.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleComponent {
    /// Owning package-wide declaration name.
    pub declaration: String,
    /// Lexical spelling; implicit slots have none. Never a global lookup key.
    pub name: Option<String>,
    /// Why the slot exists, independent of its type.
    pub kind: BinderKind,
    /// Authored anchor, retained even when the value is read later.
    pub anchor: Anchor,
    /// Retained type input for the existing model/type stage.
    pub ty: BinderType,
}

/// The static subject, compared through the contract's declared components.
///
/// Equality is component-wise. Two subjects that compare equal assert only that
/// every declared static component agrees; that is not a checked package, an
/// executable package or a qualified canonical identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticSubject {
    sources: Vec<SourceComponent>,
    language: LanguageComponent,
    profiles: Vec<ProfileComponent>,
    models: Vec<ModelComponent>,
    dependencies: Vec<DependencyComponent>,
    roles: Vec<RoleComponent>,
}

/// Build provenance: the complete compile configuration and its charged work.
///
/// Resource controls are assessment/configuration inputs, not static meaning.
/// Changing only a limit changes this record and no static component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuildProvenance {
    /// Effective whole-package namespace capacities.
    pub package_limits: WorkLimits,
    /// Effective per-unit parser capacities, separate from package accounting.
    pub parser_limits: Limits,
    /// Effective binding-stage capacities.
    pub binding_limits: binding_work::Limits,
    /// Successful namespace-stage charges from this invocation only.
    pub package_usage: super::Usage,
    /// Successful binding-stage charges from this invocation only.
    pub binding_usage: binding_work::Usage,
}

/// The subject was not established, so no component set can be claimed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unavailable {
    /// Namespace or binding work did not finish; components remain partial.
    IncompleteBinding,
    /// The definition stage was not reached, so no profile closure exists.
    MissingDefinitions,
    /// The model stage was not reached, so no import selection exists.
    MissingModels,
    /// The lexical stage was not reached, so no typed roles exist.
    MissingScopes,
    /// A namespace unit is outside the supplied inventory's expected entries.
    UnmatchedInventory,
    /// A namespace declaration handle has no original syntax to name.
    UnmatchedDeclaration,
    /// A resolved import selects no entry in the supplied model inputs.
    UnmatchedModelInput,
    /// An authored import index is outside its declaring unit's imports.
    UnmatchedModelAlias,
}

impl StaticSubject {
    /// Retain every declared static component of a completed binding report.
    ///
    /// The inventory supplies each unit's editable authority and the single
    /// language/edition selection; the report supplies everything else. An
    /// unfinished stage yields [`Unavailable`] rather than a partial subject.
    pub fn of(
        inventory: &SourceInventory,
        report: &binding::Report<'_>,
    ) -> Result<Self, Unavailable> {
        if !report.complete() {
            return Err(Unavailable::IncompleteBinding);
        }
        let namespace = report.namespace();
        let definitions = report
            .definitions()
            .ok_or(Unavailable::MissingDefinitions)?;
        let models = report.models().ok_or(Unavailable::MissingModels)?;
        let scopes = report.scopes().ok_or(Unavailable::MissingScopes)?;

        let mut sources = Vec::with_capacity(namespace.units().len());
        for (index, unit) in namespace.units().iter().enumerate() {
            let expected = namespace
                .expected_index(super::UnitId(index))
                .and_then(|expected| inventory.units.get(expected))
                .ok_or(Unavailable::UnmatchedInventory)?;
            sources.push(SourceComponent {
                authority: expected.authority.clone(),
                identity: unit.source().identity().clone(),
                digest: unit.source().digest(),
            });
        }

        let language = LanguageComponent {
            language: inventory.language.clone(),
            edition: inventory.edition.clone(),
            definition: definitions.inventory.edition.clone(),
            closure: definitions.edition.iter().map(selection).collect(),
            refusal: definitions.edition_refusal.clone(),
        };

        // Package-wide declaration names, resolved once. A missing original
        // declaration is reported, never substituted with an empty name that
        // would silently compare equal to another missing one.
        let mut names = Vec::with_capacity(namespace.declarations().len());
        for index in 0..namespace.declarations().len() {
            let declaration = namespace
                .syntax(super::DeclarationId(index))
                .ok_or(Unavailable::UnmatchedDeclaration)?;
            names.push(declaration.name.value.clone());
        }
        let name = |id: super::DeclarationId| -> Result<String, Unavailable> {
            names
                .get(id.index())
                .cloned()
                .ok_or(Unavailable::UnmatchedDeclaration)
        };

        let mut profiles = Vec::new();
        for entry in &definitions.declarations {
            for use_site in &entry.uses {
                profiles.push(ProfileComponent {
                    declaration: name(entry.declaration)?,
                    kind: use_site.kind,
                    alias: use_site.alias.value.clone(),
                    closure: use_site.closure.iter().map(selection).collect(),
                    refusal: use_site.refusal.clone(),
                });
            }
        }

        let mut imports = Vec::new();
        for import in models.imports() {
            let unit = namespace
                .expected_index(import.unit)
                .and_then(|expected| inventory.units.get(expected))
                .ok_or(Unavailable::UnmatchedInventory)?;
            let alias = namespace
                .unit(import.unit)
                .and_then(|unit| unit.models().get(import.import))
                .ok_or(Unavailable::UnmatchedModelAlias)?
                .alias
                .value
                .clone();
            imports.push(ModelComponent {
                authority: unit.authority.clone(),
                alias,
                selection: match &import.selection {
                    Ok(input) => Ok(model_selection(models.inputs(), *input)
                        .ok_or(Unavailable::UnmatchedModelInput)?),
                    Err(refusal) => Err(refusal.clone()),
                },
            });
        }

        let mut dependencies = Vec::new();
        for (index, entry) in namespace.declarations().iter().enumerate() {
            let owner = name(super::DeclarationId(index))?;
            for reference in entry.references() {
                dependencies.push(DependencyComponent {
                    declaration: owner.clone(),
                    kind: reference.kind,
                    site: reference.site,
                    name: reference.name.value.clone(),
                    target: reference.target.map(name).transpose()?,
                });
            }
        }

        let mut roles = Vec::new();
        for scope in scopes.declarations().iter().flatten() {
            let owner = name(scope.declaration)?;
            for binder in &scope.binders {
                roles.push(RoleComponent {
                    declaration: owner.clone(),
                    name: binder.name.clone(),
                    kind: binder.kind,
                    anchor: binder.anchor,
                    ty: binder.ty.clone(),
                });
            }
        }

        Ok(Self {
            sources,
            language,
            profiles,
            models: imports,
            dependencies,
            roles,
        })
    }

    /// Sources component in namespace unit order.
    pub fn sources(&self) -> &[SourceComponent] {
        &self.sources
    }

    /// The single language/edition component.
    pub fn language(&self) -> &LanguageComponent {
        &self.language
    }

    /// Profiles component in declaration then authored occurrence order.
    pub fn profiles(&self) -> &[ProfileComponent] {
        &self.profiles
    }

    /// Models component in authored import order.
    pub fn models(&self) -> &[ModelComponent] {
        &self.models
    }

    /// Native dependencies component in declaration then occurrence order.
    pub fn dependencies(&self) -> &[DependencyComponent] {
        &self.dependencies
    }

    /// Binding requirements component in declaration then binder order.
    pub fn roles(&self) -> &[RoleComponent] {
        &self.roles
    }

    /// Every declared component in which these two subjects differ.
    ///
    /// An empty result means every declared static component agrees. It is a
    /// component-wise comparison, never a claim about a canonical identity.
    pub fn differences(&self, other: &Self) -> Vec<ComponentKind> {
        let mut differences = Vec::new();
        if self.sources != other.sources {
            differences.push(ComponentKind::Sources);
        }
        if self.language != other.language {
            differences.push(ComponentKind::Language);
        }
        if self.profiles != other.profiles {
            differences.push(ComponentKind::Profiles);
        }
        if self.models != other.models {
            differences.push(ComponentKind::Models);
        }
        if self.dependencies != other.dependencies {
            differences.push(ComponentKind::NativeDependencies);
        }
        if self.roles != other.roles {
            differences.push(ComponentKind::BindingRequirements);
        }
        differences
    }
}

impl BuildProvenance {
    /// Retain the full compile configuration consumed by both linking stages.
    pub fn of(namespace: &super::NamespaceReport<'_>, report: &binding::Report<'_>) -> Self {
        Self {
            package_limits: namespace.limits(),
            parser_limits: namespace.parser_limits(),
            binding_limits: report.limits(),
            package_usage: namespace.usage(),
            binding_usage: report.usage(),
        }
    }
}

fn selection(definition: &RegisteredDefinition) -> Selection {
    definition.selection()
}

fn model_selection(inputs: &[ModelInput<'_>], input: usize) -> Option<ModelSelection> {
    Some(match *inputs.get(input)? {
        ModelInput::Native(model) => ModelSelection {
            package: model.environment().owner().package().as_str().to_owned(),
            revision: model.environment().owner().revision().get().to_string(),
            digest: model.digest(),
        },
        ModelInput::UnsupportedProducer {
            package,
            revision,
            digest,
            ..
        } => ModelSelection {
            package: package.to_owned(),
            revision: revision.to_owned(),
            digest,
        },
    })
}
