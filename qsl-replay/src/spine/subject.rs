// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-510: component-wise static comparison of a successful spine check.

use qsl_forms::{DeclarationForm, ParsedForm};
use qsl_foundation::selection::{ModelSelection, ProfileSelection};
use qsl_foundation::source::Source;
use qsl_semantics::model::intake::{PackageInput, SelectedModels};
use qsl_semantics::value::HeaderLayer;

use super::SpineLimits;

/// A declared static component that differs between two subjects.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ComponentKind {
    /// Source references and exact source contents.
    Sources,
    /// The edition selected by the unit headers.
    Language,
    /// Header profiles and each declaration's profile closure.
    Profiles,
    /// Selected domain packages and their exact input contents.
    Models,
    /// Resolved libraries and references to their declarations.
    Dependencies,
    /// Declaration-owned protocol roles.
    Roles,
}

/// The source reference and content identity of one unit or resolved library.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceComponent {
    /// Source authority.
    pub authority: String,
    /// Source identity within its authority.
    pub identity: String,
    /// Exact admitted UTF-8 source content.
    pub content: String,
}

/// One header profile selection and its resolved definition closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileComponent {
    /// Header-local alias.
    pub alias: String,
    /// Resolved profile identity.
    pub identity: String,
    /// Root-first profile closure.
    pub closure: Vec<String>,
}

/// One declaration's checked profile closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclarationProfileComponent {
    /// Declaration name.
    pub declaration: String,
    /// Header-local alias used by the declaration.
    pub alias: String,
    /// Resolved root identity and its closure.
    pub closure: Vec<String>,
}

/// One selected model package in source declaration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelComponent {
    /// Unit-local model alias.
    pub alias: String,
    /// Selected package identity.
    pub package: String,
    /// Selected package's declared version.
    pub version: String,
    /// Exact package input bytes, retained as content identity.
    pub content: Vec<u8>,
}

/// One resolved library in canonical library-identity order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyComponent {
    /// Resolved library identity.
    pub identity: String,
    /// Declaration kinds referenced from this library, in first-use order.
    pub declaration_kinds: Vec<qsl_semantics::check::CheckedDependencyKind>,
}

/// One resolved native declaration reference in the dependencies component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyReferenceComponent {
    /// The declaration containing this reference.
    pub referring_declaration: String,
    /// The required declaration kind.
    pub kind: qsl_semantics::check::CheckedDependencyKind,
    /// The typed source occurrence.
    pub site: qsl_semantics::check::CheckedDependencySite,
    /// The name as authored.
    pub name: String,
    /// The exact checked declaration identity.
    pub target: quire_exact::NodeKey,
    /// The resolved library identity for an imported predicate.
    pub library: Option<String>,
}

/// One role declared by a protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleComponent {
    /// The protocol declaration that owns this role.
    pub owner: String,
    /// Role spelling.
    pub name: String,
    /// Whether the role is static or replicated.
    pub kind: RoleKind,
    /// Authored role-name span.
    pub anchor: qsl_foundation::Span,
    /// Resolved model package and object-type spelling.
    pub type_identity: String,
}

/// The kind of a protocol role binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoleKind {
    /// `role R on M::T`.
    Static,
    /// `role R each M::T from ...`.
    Replicated,
}

/// A checked unit's static semantic subject. Equality compares the six
/// declared components; it does not claim a package identity or package bytes.
///
/// This value intentionally offers no package-identity or emitted-byte API:
///
/// ```compile_fail
/// fn checked_identity(subject: &qsl_replay::spine::StaticSubject) -> &str {
///     subject.checked_package_identity()
/// }
/// ```
///
/// ```compile_fail
/// fn emitted_bytes(subject: &qsl_replay::spine::StaticSubject) -> &[u8] {
///     subject.emitted_bytes()
/// }
/// ```
///
/// ```compile_fail
/// fn convert_to_bytes(subject: qsl_replay::spine::StaticSubject) -> Vec<u8> {
///     subject.into()
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticSubject {
    sources: Vec<SourceComponent>,
    edition: Option<String>,
    profiles: Vec<ProfileComponent>,
    declarations: Vec<DeclarationProfileComponent>,
    models: Vec<ModelComponent>,
    dependencies: Vec<DependencyComponent>,
    dependency_references: Vec<DependencyReferenceComponent>,
    roles: Vec<RoleComponent>,
}

pub(super) struct StaticSubjectInput<'a> {
    pub(super) source: &'a Source,
    pub(super) libraries: &'a [Source],
    pub(super) edition: Option<&'a str>,
    pub(super) selected_profiles: &'a [ProfileSelection],
    pub(super) model_selections: &'a [ModelSelection],
    pub(super) forms: &'a [ParsedForm],
    pub(super) selected_models: &'a SelectedModels,
    pub(super) package_input: &'a PackageInput,
    pub(super) resolved_libraries: &'a [String],
    pub(super) graph: &'a qsl_semantics::check::CheckedGraph,
}

/// The resource configuration and stage usage of the successful check that
/// produced a checked unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuildProvenance {
    /// Limits supplied to the spine check.
    pub limits: SpineLimits,
    /// Work reported by the check stage.
    pub usage: qsl_foundation::diagnostic::StageWork,
}

impl StaticSubject {
    pub(super) fn from_check(input: StaticSubjectInput<'_>) -> Result<Self, SubjectUnavailable> {
        let StaticSubjectInput {
            source,
            libraries,
            edition,
            selected_profiles,
            model_selections,
            forms,
            selected_models,
            package_input,
            resolved_libraries,
            graph,
        } = input;
        let mut sources = Vec::with_capacity(libraries.len() + 1);
        sources.push(source_component(source));
        let mut library_sources: Vec<_> = libraries.iter().map(source_component).collect();
        library_sources.sort_by(|left, right| {
            left.authority
                .as_bytes()
                .cmp(right.authority.as_bytes())
                .then_with(|| left.identity.as_bytes().cmp(right.identity.as_bytes()))
        });
        sources.extend(library_sources);

        let profiles: Vec<_> = selected_profiles
            .iter()
            .map(|profile| ProfileComponent {
                alias: profile.alias.clone(),
                identity: profile.identity.clone(),
                closure: profile_closure(&profile.identity),
            })
            .collect();
        let profiles_by_alias: std::collections::BTreeMap<_, _> = profiles
            .iter()
            .map(|profile| (profile.alias.as_str(), profile))
            .collect();
        let mut declarations = Vec::new();
        for form in forms {
            let Some(alias) = declaration_profile(form.form()) else {
                continue;
            };
            let Some(profile) = profiles_by_alias.get(alias) else {
                return Err(SubjectUnavailable::UnmatchedProfileAlias(alias.to_owned()));
            };
            if let Some(declaration) = form.declared_name() {
                declarations.push(DeclarationProfileComponent {
                    declaration: declaration.to_owned(),
                    alias: alias.to_owned(),
                    closure: profile.closure.clone(),
                });
            }
        }

        let mut models = Vec::new();
        for selection in model_selections {
            let selected = selected_models
                .iter()
                .find(|model| {
                    model.alias == selection.alias
                        && model.view.model_selection().identity == selection.identity
                })
                .ok_or_else(|| SubjectUnavailable::UnmatchedModelAlias(selection.alias.clone()))?;
            let identity = selected.view.model_selection().identity.clone();
            let version = selected_models
                .version_for_identity(&selection.identity)
                .ok_or_else(|| SubjectUnavailable::MissingModelVersion(selection.alias.clone()))?;
            let content = package_input
                .get(&identity)
                .ok_or_else(|| SubjectUnavailable::MissingModelContent(identity.clone()))?
                .clone();
            models.push(ModelComponent {
                alias: selection.alias.clone(),
                package: identity,
                version: version.to_owned(),
                content,
            });
        }

        let dependency_references: Vec<_> = graph
            .dependency_references()
            .iter()
            .filter(|reference| reference.source == *source.reference())
            .map(|reference| DependencyReferenceComponent {
                referring_declaration: reference.referring_declaration.clone(),
                kind: reference.kind,
                site: reference.site.clone(),
                name: reference.name.clone(),
                target: reference.target.clone(),
                library: reference.library.clone(),
            })
            .collect();
        let mut dependencies: Vec<_> = resolved_libraries
            .iter()
            .cloned()
            .map(|identity| DependencyComponent {
                declaration_kinds: dependency_references
                    .iter()
                    .filter(|reference| reference.library.as_deref() == Some(identity.as_str()))
                    .map(|reference| reference.kind)
                    .fold(Vec::new(), |mut kinds, kind| {
                        if !kinds.contains(&kind) {
                            kinds.push(kind);
                        }
                        kinds
                    }),
                identity,
            })
            .collect();
        dependencies.sort_by(|left, right| left.identity.cmp(&right.identity));

        let model_identities: std::collections::BTreeMap<_, _> = models
            .iter()
            .map(|model| (model.alias.as_str(), model.package.as_str()))
            .collect();
        let mut roles = Vec::new();
        for form in forms {
            let DeclarationForm::Protocol(protocol) = form.form() else {
                continue;
            };
            for role in &protocol.roles {
                let Some((model_alias, object_type)) = role.context.segments.split_first() else {
                    continue;
                };
                let Some(package) = model_identities.get(model_alias.as_str()) else {
                    return Err(SubjectUnavailable::UnmatchedModelAlias(
                        model_alias.to_owned(),
                    ));
                };
                roles.push(RoleComponent {
                    owner: protocol.name.name.clone(),
                    name: role.name.name.clone(),
                    kind: if role.replicated.is_some() {
                        RoleKind::Replicated
                    } else {
                        RoleKind::Static
                    },
                    anchor: role.name.span,
                    type_identity: format!("{package}::{}", object_type.join("::")),
                });
            }
        }

        Ok(Self {
            sources,
            edition: edition.map(str::to_owned),
            profiles,
            declarations,
            models,
            dependencies,
            dependency_references,
            roles,
        })
    }

    /// Source references and contents, unit first then resolved libraries.
    pub fn sources(&self) -> &[SourceComponent] {
        &self.sources
    }

    /// The language edition selected by the unit header.
    pub fn edition(&self) -> Option<&str> {
        self.edition.as_deref()
    }

    /// Header profile selections, in source order.
    pub fn profiles(&self) -> &[ProfileComponent] {
        &self.profiles
    }

    /// The profile closure each declaration is checked under, in source order.
    pub fn declarations(&self) -> &[DeclarationProfileComponent] {
        &self.declarations
    }

    /// Selected model packages, in source declaration order.
    pub fn models(&self) -> &[ModelComponent] {
        &self.models
    }

    /// Resolved libraries, in canonical library-identity order.
    pub fn dependencies(&self) -> &[DependencyComponent] {
        &self.dependencies
    }

    /// Resolved predicate, operation-contract and temporal-obligation
    /// references, in source order.
    pub fn dependency_references(&self) -> &[DependencyReferenceComponent] {
        &self.dependency_references
    }

    /// Declaration-owned protocol roles, in source order.
    pub fn roles(&self) -> &[RoleComponent] {
        &self.roles
    }

    /// Components that differ, in the FR-510 order.
    pub fn differences(&self, other: &Self) -> Vec<ComponentKind> {
        let mut differences = Vec::new();
        if self.sources != other.sources {
            differences.push(ComponentKind::Sources);
        }
        if self.edition != other.edition {
            differences.push(ComponentKind::Language);
        }
        if self.profiles != other.profiles || self.declarations != other.declarations {
            differences.push(ComponentKind::Profiles);
        }
        if self.models != other.models {
            differences.push(ComponentKind::Models);
        }
        if self.dependencies != other.dependencies
            || self.dependency_references != other.dependency_references
        {
            differences.push(ComponentKind::Dependencies);
        }
        if self.roles != other.roles {
            differences.push(ComponentKind::Roles);
        }
        differences
    }
}

fn source_component(source: &Source) -> SourceComponent {
    SourceComponent {
        authority: source.reference().authority().to_owned(),
        identity: source.reference().identity().to_owned(),
        content: source.text().to_owned(),
    }
}

fn profile_closure(identity: &str) -> Vec<String> {
    HeaderLayer::from_identity(identity).map_or_else(
        || vec![identity.to_owned()],
        |layer| {
            HeaderLayer::closure(&[layer])
                .into_iter()
                .map(|member| member.identity().to_owned())
                .collect()
        },
    )
}

fn declaration_profile(form: &DeclarationForm) -> Option<&str> {
    use DeclarationForm as F;
    match form {
        F::Abstraction(form) => Some(&form.profile.alias),
        F::Function(form) => form.using().map(|using| using.alias.as_str()),
        F::StateClause(form) => Some(&form.profile.alias),
        F::OperationBody(form) => Some(&form.profile.alias),
        F::Protocol(form) => Some(&form.using.alias),
        F::Temporal(form) => Some(&form.profile.alias),
        F::Terminal(form) => Some(&form.profile.alias),
        F::Refinement(form) => Some(&form.profile.alias),
        F::HyperBehaviours(form) => Some(&form.profile.alias),
        F::HyperFinite(form) => Some(&form.profile.alias),
        F::ModelRelation(form) => Some(&form.profile.alias),
        F::Probabilistic(form) => Some(&form.profile.alias),
        F::StateGraphClaim(form) => Some(&form.profile.alias),
        F::Hybrid(form) => Some(&form.profile.alias),
        F::Extension(_)
        | F::Union(_)
        | F::Alias(_)
        | F::Record(_)
        | F::Tuple(_)
        | F::Enum(_)
        | F::Dimension(_)
        | F::DimensionEquivalence(_)
        | F::Unit(_)
        | F::PopulationBound(_)
        | F::Random(_)
        | F::Reward(_)
        | F::Delay(_)
        | F::Workload(_)
        | F::TimeSource(_)
        | F::Release(_)
        | F::TimeInvariant(_)
        | F::Urgent(_)
        | F::Taskset(_)
        | F::Schedulable(_)
        | F::Response(_) => None,
    }
}

/// Why the successful spine result does not have a complete subject.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SubjectUnavailable {
    /// The check retained an attributable refusal for continuation.
    #[error("a refused or incomplete check has no complete static subject")]
    RefusedCheck,
    /// A declaration uses an alias absent from the checked header.
    #[error("declaration uses profile alias {0:?} without a checked header selection")]
    UnmatchedProfileAlias(String),
    /// A source model alias has no admitted model.
    #[error("model alias {0:?} has no admitted model")]
    UnmatchedModelAlias(String),
    /// An admitted model document has no retained source version.
    #[error("model alias {0:?} has no source package version")]
    MissingModelVersion(String),
    /// An admitted model has no retained input document.
    #[error("model package {0:?} has no retained input content")]
    MissingModelContent(String),
}
