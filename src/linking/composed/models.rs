// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-036: exact unit-local imports over admitted native models and FR-056
//! domain packages. No model schema, runtime population or producer
//! correspondence is inferred.

use std::collections::BTreeMap;
use std::str::FromStr;

use quire_contract_ir as ir;

use super::binding_work::{Dimension, Exhaustion, Work};
use super::{DeclarationId, SyntaxNamespace, UnitId};
use crate::checking::{Catalog, DomainType, NativeType};
use crate::linking::{location, DeclarationKey, DeclarationLocation};
use crate::native_model::{NativeModel, OperationRole, ScalarRole, ScalarSite};
use crate::syntax::composed::{self as c, Operation, ParameterType, QualifiedName};
use qsl_foundation::{ByteDigest, Span, Spanned};
use qsl_semantics::model::admitted::{AdmittedPackage, Declaration, DeclarationKind};
use qsl_semantics::model::domain_package::DomainPackageRef;
use qsl_semantics::model::key::DeclarationKey as DomainKey;

/// Supplied model selections: admitted native models and admitted domain
/// packages. Nothing else binds.
#[derive(Clone, Copy, Debug)]
pub enum ModelInput<'a> {
    /// Constructor-admitted native model, preserving its actual profile and declarations.
    Native(&'a NativeModel),
    /// A domain package admitted under FR-056 and classified under FR-152.
    Domain(&'a AdmittedPackage),
}

impl<'a> ModelInput<'a> {
    /// The admitted native model, absent for a domain package.
    pub fn native_model(self) -> Option<&'a NativeModel> {
        match self {
            Self::Native(model) => Some(model),
            Self::Domain(_) => None,
        }
    }

    /// The admitted domain package, absent for a native model.
    pub fn domain_package(self) -> Option<&'a AdmittedPackage> {
        match self {
            Self::Native(_) => None,
            Self::Domain(package) => Some(package),
        }
    }
}

/// An import's selected digest, in its own digest domain. A native model is
/// selected by the SHA-256 of its artifact bytes (`sha256:<hex>`); a domain
/// package by the SHA-256 of its RFC 8785 document (`sha256-jcs:<hex>`,
/// FR-056-CON-4). Neither spelling selects the other kind of input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedDigest {
    /// `sha256:<hex>`: raw artifact bytes.
    Artifact(ByteDigest),
    /// `sha256-jcs:<hex>`: a domain package's JCS document.
    DomainPackage([u8; 32]),
}

impl FromStr for SelectedDigest {
    type Err = qsl_foundation::digest::InvalidDigest;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.strip_prefix("sha256-jcs:") {
            Some(hex) => {
                ByteDigest::from_hex(hex).map(|digest| Self::DomainPackage(digest.as_bytes()))
            }
            None => text.parse().map(Self::Artifact),
        }
    }
}

/// Shared conflict, retaining all supplied candidates through input indices.
#[derive(Debug)]
pub struct ModelConflict {
    /// Which immutable correspondence disagreed.
    pub kind: ModelConflictKind,
    /// Indices into ModelBindings::inputs().
    pub inputs: Vec<usize>,
}

/// Conflicts are local to imports selecting an affected candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelConflictKind {
    /// One formal declaration owner supplies different artifact bytes.
    Owner,
    /// One formal source identity maps to differing native source identities/bytes.
    Source,
}

/// Exact import selection failure, separate from qualified export lookup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportRefusal {
    /// Digest spelling is neither `sha256:<hex>` nor `sha256-jcs:<hex>`.
    InvalidDigest,
    /// A native model revision is not the canonical unsigned decimal IR revision.
    InvalidRevision,
    /// No supplied model has this package label.
    MissingPackage,
    /// The package is supplied only in the other digest domain: a
    /// `sha256-jcs` selection of a native model, or a `sha256` selection of
    /// a domain package (FR-056-CON-4).
    DigestDomainMismatch,
    /// No candidate has all selected revision/digest components.
    StaleSelection,
    /// Multiple candidates match the complete selection.
    AmbiguousSelection {
        /// Indices into ModelBindings::inputs() for the matching candidates.
        inputs: Vec<usize>,
    },
    /// Shared conflict indices into ModelBindings::conflicts().
    ConflictingModel {
        /// Indices into ModelBindings::conflicts() for the shared conflicts.
        groups: Vec<usize>,
    },
}

/// One authored model import, kept even when no export can bind through it.
#[derive(Debug)]
pub struct ModelImportBinding {
    /// Namespace-local source owner of the alias.
    pub unit: UnitId,
    /// Index into that source unit's models().
    pub import: usize,
    /// Selected supplied input index, or the exact selection refusal.
    pub selection: Result<usize, ImportRefusal>,
}

/// Located qualified-reference failure; source spans stay in the authored unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelError {
    /// Namespace-local source for span.
    pub unit: UnitId,
    /// Original alias, name or member occurrence.
    pub span: Span,
    /// Typed cause, with shared catalog evidence retained separately.
    pub kind: ModelErrorKind,
}

/// Model binding cannot manufacture an unknown or partially checked type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelErrorKind {
    /// Supplied inventory/import processing did not finish.
    IncompleteCatalog,
    /// The declaring source has no such model alias.
    MissingAlias,
    /// More than one local import defines this alias; indices address imports().
    AmbiguousAlias {
        /// Indices into imports() for the competing local imports.
        imports: Vec<usize>,
    },
    /// The selected import refused; index addresses imports().
    RefusedImport {
        /// Index into imports() for the import that refused.
        import: usize,
    },
    /// No permitted export has this name.
    MissingExport,
    /// Scalar and formal type namespaces export the same qualified type spelling.
    AmbiguousExport,
    /// The selected export is not of the required native kind.
    WrongExportKind,
    /// The type or operation belongs to a different model binding inventory.
    ForeignModel,
    /// A record name cannot supply authoritative related-instance semantics.
    UnsupportedRelationshipContract,
    /// Charge-before-work prevented completing this occurrence.
    ResourceExhausted(Exhaustion),
}

/// Existing native type and its original nominal declaration/role location.
#[derive(Clone, Debug)]
pub struct BoundType<'a> {
    model: &'a NativeModel,
    native: NativeType<'a>,
    location: DeclarationLocation,
}

impl<'a> BoundType<'a> {
    /// Actual admitted model, borrowed without artifact/source copying.
    pub fn model(&self) -> &'a NativeModel {
        self.model
    }
    /// Existing native scalar/object/reference/record/enumeration meaning.
    pub fn native(&self) -> &NativeType<'a> {
        &self.native
    }
    /// Original formal declaration or native scalar role, with exact owner.
    pub fn location(&self) -> &DeclarationLocation {
        &self.location
    }
}

/// Operation role from the selected model; a function is not an operation.
#[derive(Clone, Debug)]
pub struct BoundOperation<'a> {
    model: &'a NativeModel,
    role: &'a OperationRole,
    location: DeclarationLocation,
}

impl<'a> BoundOperation<'a> {
    /// Model owning the operation and all its input/result declarations.
    pub fn model(&self) -> &'a NativeModel {
        self.model
    }
    /// Actual invocation parameters, result, anchor and frame.
    pub fn role(&self) -> &'a OperationRole {
        self.role
    }
    /// Owner-qualified original operation locus.
    pub fn location(&self) -> &DeclarationLocation {
        &self.location
    }
}

/// A domain-package declaration a qualified occurrence bound to: its exact
/// FR-154 declaration key and kind, under the selection it was admitted by.
#[derive(Clone, Copy, Debug)]
pub struct BoundDeclaration<'a> {
    package: &'a AdmittedPackage,
    declaration: Declaration<'a>,
}

impl<'a> BoundDeclaration<'a> {
    /// The admitted domain package owning the declaration.
    pub fn package(&self) -> &'a AdmittedPackage {
        self.package
    }
    /// The selection (identity, version, `sha256-jcs` digest) it was bound under.
    pub fn selection(&self) -> &'a DomainPackageRef {
        self.package.selection()
    }
    /// The declaration's exact key.
    pub fn key(&self) -> &'a DomainKey {
        self.declaration.key
    }
    /// The declaration's kind.
    pub fn kind(&self) -> DeclarationKind {
        self.declaration.kind
    }
    /// The record intake read for the declaration.
    pub fn declaration(&self) -> Declaration<'a> {
        self.declaration
    }
}

/// The declaration kinds a qualified-name site binds in a domain package.
#[derive(Clone, Copy, Debug)]
enum Site {
    /// A parameter, capture, channel, state context or value type.
    Type,
    /// A protocol role: a type, or the FR-152 Part or Port it participates as.
    Role,
    /// A protocol relationship: a Connection or a navigation relationship.
    Relationship,
}

impl Site {
    fn admits(self, kind: DeclarationKind) -> bool {
        match kind {
            DeclarationKind::ObjectType
            | DeclarationKind::Interface
            | DeclarationKind::ValueType
            | DeclarationKind::RecordValueType => {
                matches!(self, Self::Type | Self::Role)
            }
            DeclarationKind::Part | DeclarationKind::Port => matches!(self, Self::Role),
            DeclarationKind::Connection | DeclarationKind::Relationship => {
                matches!(self, Self::Relationship)
            }
            DeclarationKind::Allocation
            | DeclarationKind::Field
            | DeclarationKind::Operation
            | DeclarationKind::Population => false,
        }
    }
}

/// What a selected input exposes to qualified lookup.
#[derive(Debug)]
enum Selected<'a> {
    Native(Box<Exports<'a>>),
    Domain(&'a AdmittedPackage),
}

#[derive(Debug)]
struct Exports<'a> {
    catalog: Catalog<'a>,
    scalars: BTreeMap<&'a str, &'a ScalarRole>,
    records: BTreeMap<&'a str, &'a ir::RecordDeclaration>,
    enumerations: BTreeMap<&'a str, &'a ir::EnumDeclaration>,
    values: BTreeMap<&'a str, &'a ir::ValueDeclaration>,
    operations: BTreeMap<(&'a str, &'a str), &'a OperationRole>,
}

impl<'a> Exports<'a> {
    // charge_exports reserves these borrowed indexes before their allocation.
    fn new(model: &'a NativeModel) -> Self {
        let catalog = Catalog::composed(model);
        Self {
            records: catalog
                .records
                .values()
                .map(|declaration| (declaration.name().as_str(), *declaration))
                .collect(),
            enumerations: catalog
                .enumerations
                .values()
                .map(|declaration| (declaration.name().as_str(), *declaration))
                .collect(),
            values: catalog
                .values
                .values()
                .map(|declaration| (declaration.name().as_str(), *declaration))
                .collect(),
            scalars: model
                .roles()
                .scalars
                .iter()
                .map(|role| (role.name.as_str(), role))
                .collect(),
            operations: model
                .roles()
                .operations
                .iter()
                .map(|role| ((role.context.as_str(), role.name.as_str()), role))
                .collect(),
            catalog,
        }
    }
}

/// Partial import evidence remains inspectable after exhaustion. Qualified
/// lookups require the complete supplied inventory to have been examined.
#[derive(Debug)]
pub struct ModelBindings<'a> {
    inputs: &'a [ModelInput<'a>],
    catalogs: Vec<Option<Selected<'a>>>,
    imports: Vec<ModelImportBinding>,
    aliases: BTreeMap<UnitId, BTreeMap<String, Vec<usize>>>,
    conflicts: Vec<ModelConflict>,
    exhaustion: Option<Exhaustion>,
    complete: bool,
}

impl<'a> ModelBindings<'a> {
    /// Borrow an already constructed catalog; callers must still enforce the
    /// owning declaration's completed binding disposition before using it.
    pub(crate) fn catalog_at(&self, input: usize) -> Option<&crate::checking::Catalog<'a>> {
        self.catalogs
            .get(input)
            .and_then(Option::as_ref)
            .and_then(|selected| match selected {
                Selected::Native(exports) => Some(&exports.catalog),
                Selected::Domain(_) => None,
            })
    }
    /// Original supplied native models and domain packages.
    pub fn inputs(&self) -> &'a [ModelInput<'a>] {
        self.inputs
    }
    /// Source-ordered import selections, including refused imports.
    pub fn imports(&self) -> &[ModelImportBinding] {
        &self.imports
    }
    /// Shared source/owner conflicts, without duplicated model artifacts.
    pub fn conflicts(&self) -> &[ModelConflict] {
        &self.conflicts
    }
    /// First unaffordable catalog/import operation.
    pub fn exhaustion(&self) -> Option<&Exhaustion> {
        self.exhaustion.as_ref()
    }
    /// Inventory processing finished; individual import refusals may remain.
    pub fn complete(&self) -> bool {
        self.complete
    }
}

/// Charge every supplied Model and its artifact Bytes before import selection.
/// Only exact selected inputs receive export indexes, once per input. Bindings
/// reserve stored map/set entries and field-vector elements, plus imports and
/// returned records; borrowed operation parameters allocate no index entries.
/// References count each successful import's cache lookup, each borrowed-name
/// index entry, lookup attempts, field-search comparisons, and inspected catalog,
/// parameter, variant, owner/source and conflict candidates. No edge is added.
pub fn bind_models<'a>(
    namespace: &SyntaxNamespace,
    inputs: &'a [ModelInput<'a>],
    work: &mut Work,
) -> ModelBindings<'a> {
    let mut bindings = ModelBindings {
        inputs,
        catalogs: Vec::new(),
        imports: Vec::new(),
        aliases: BTreeMap::new(),
        conflicts: Vec::new(),
        exhaustion: None,
        complete: false,
    };
    match bindings.collect(namespace, work) {
        Ok(()) => bindings.complete = true,
        Err(error) => bindings.exhaustion = Some(error),
    }
    bindings
}

fn text(work: &mut Work, values: &[&str]) -> Result<(), Exhaustion> {
    for value in values {
        work.charge(Dimension::Bytes, value.len())?;
    }
    Ok(())
}

fn failure(unit: UnitId, span: Span, kind: ModelErrorKind) -> ModelError {
    ModelError { unit, span, kind }
}

fn charge(
    work: &mut Work,
    dimension: Dimension,
    amount: usize,
    unit: UnitId,
    span: Span,
) -> Result<(), ModelError> {
    work.charge(dimension, amount)
        .map_err(|error| failure(unit, span, ModelErrorKind::ResourceExhausted(error)))
}

// Catalog keys retain upstream SymbolName identities. Where a borrowed string
// cannot address that key, charge each compared entry rather than one whole scan.
fn find_charged<T>(
    entries: impl IntoIterator<Item = T>,
    work: &mut Work,
    unit: UnitId,
    span: Span,
    mut matches: impl FnMut(&T) -> bool,
) -> Result<Option<T>, ModelError> {
    for entry in entries {
        charge(work, Dimension::References, 1, unit, span)?;
        if matches(&entry) {
            return Ok(Some(entry));
        }
    }
    Ok(None)
}

impl<'a> ModelBindings<'a> {
    fn collect(&mut self, namespace: &SyntaxNamespace, work: &mut Work) -> Result<(), Exhaustion> {
        let mut owners = BTreeMap::<&ir::RequirementRef, Vec<(usize, &NativeModel)>>::new();
        let mut sources = BTreeMap::<&ir::SourceIdentity, Vec<(usize, &NativeModel)>>::new();
        for (index, input) in self.inputs.iter().enumerate() {
            work.charge(Dimension::Models, 1)?;
            match *input {
                ModelInput::Native(model) => {
                    work.charge(Dimension::Bytes, model.artifact_bytes().len())?;
                    owners
                        .entry(model.environment().owner())
                        .or_default()
                        .push((index, model));
                    sources
                        .entry(model.source().identity())
                        .or_default()
                        .push((index, model));
                }
                ModelInput::Domain(package) => {
                    let selection = package.selection();
                    text(work, &[&selection.identity, &selection.version])?;
                }
            }
            self.catalogs.push(None);
        }
        for inputs in owners.into_values() {
            if inputs.len() < 2 {
                continue;
            }
            work.charge(Dimension::References, 1)?;
            let first = inputs[0].1.digest();
            let mut different = false;
            for (_, model) in &inputs[1..] {
                work.charge(Dimension::References, 1)?;
                if model.digest() != first {
                    different = true;
                    break;
                }
            }
            if different {
                work.charge(Dimension::Bindings, 1)?;
                self.conflicts.push(ModelConflict {
                    kind: ModelConflictKind::Owner,
                    inputs: inputs.into_iter().map(|(input, _)| input).collect(),
                });
            }
        }
        for inputs in sources.into_values() {
            if inputs.len() < 2 {
                continue;
            }
            work.charge(Dimension::References, 1)?;
            let first = inputs[0].1.source().source();
            let mut different = false;
            for (_, model) in &inputs[1..] {
                work.charge(Dimension::References, 1)?;
                let source = model.source().source();
                if source.identity() != first.identity() || source.digest() != first.digest() {
                    different = true;
                    break;
                }
            }
            if different {
                work.charge(Dimension::Bindings, 1)?;
                self.conflicts.push(ModelConflict {
                    kind: ModelConflictKind::Source,
                    inputs: inputs.into_iter().map(|(input, _)| input).collect(),
                });
            }
        }
        for (unit, syntax) in namespace.units().iter().enumerate() {
            let unit = UnitId(unit);
            for (import, selection) in syntax.models().iter().enumerate() {
                work.charge(Dimension::Bindings, 1)?;
                text(
                    work,
                    &[
                        &selection.alias.value,
                        &selection.package.value,
                        &selection.version.value,
                        &selection.digest.value,
                    ],
                )?;
                let selected = match selection.digest.value.parse::<SelectedDigest>() {
                    Err(_) => Err(ImportRefusal::InvalidDigest),
                    Ok(digest) => self.select(
                        &selection.package.value,
                        &selection.version.value,
                        digest,
                        work,
                    )?,
                };
                let selected_input = selected.as_ref().ok().copied();
                self.aliases
                    .entry(unit)
                    .or_default()
                    .entry(selection.alias.value.clone())
                    .or_default()
                    .push(self.imports.len());
                self.imports.push(ModelImportBinding {
                    unit,
                    import,
                    selection: selected,
                });
                if let Some(input) = selected_input {
                    work.charge(Dimension::References, 1)?;
                    if self.catalogs[input].is_none() {
                        self.catalogs[input] = Some(match self.inputs[input] {
                            ModelInput::Native(model) => {
                                charge_exports(model, work)?;
                                Selected::Native(Box::new(Exports::new(model)))
                            }
                            // The package's declaration index was built at
                            // admission; selecting it allocates nothing.
                            ModelInput::Domain(package) => Selected::Domain(package),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn select(
        &self,
        package: &str,
        revision: &str,
        digest: SelectedDigest,
        work: &mut Work,
    ) -> Result<Result<usize, ImportRefusal>, Exhaustion> {
        let mut same_package = false;
        let mut same_domain = false;
        // Parse a native revision once; a domain package version is compared
        // exactly as spelled.
        let native_revision = revision
            .parse::<u64>()
            .ok()
            .filter(|_| {
                revision.bytes().all(|byte| byte.is_ascii_digit()) && !revision.starts_with('0')
            })
            .and_then(|value| ir::RequirementRevision::new(value).ok());
        let mut malformed_native_revision = false;
        let mut exact = Vec::new();
        for (index, input) in self.inputs.iter().enumerate() {
            work.charge(Dimension::References, 1)?;
            let matches = match *input {
                ModelInput::Native(model) => {
                    let owner = model.environment().owner();
                    let named = owner.package().as_str() == package;
                    same_package |= named;
                    let SelectedDigest::Artifact(digest) = digest else {
                        continue;
                    };
                    same_domain |= named;
                    malformed_native_revision |= named && native_revision.is_none();
                    named
                        && native_revision.as_ref() == Some(&owner.revision())
                        && model.digest() == digest
                }
                ModelInput::Domain(domain) => {
                    let selection = domain.selection();
                    let named = selection.identity == package;
                    same_package |= named;
                    let SelectedDigest::DomainPackage(digest) = digest else {
                        continue;
                    };
                    same_domain |= named;
                    named && selection.version == revision && selection.digest == digest
                }
            };
            if matches {
                exact.push(index);
            }
        }
        Ok(match exact.as_slice() {
            [] if !same_package => Err(ImportRefusal::MissingPackage),
            [] if !same_domain => Err(ImportRefusal::DigestDomainMismatch),
            [] if malformed_native_revision => Err(ImportRefusal::InvalidRevision),
            [] => Err(ImportRefusal::StaleSelection),
            [input] => {
                let mut groups = Vec::new();
                for (index, conflict) in self.conflicts.iter().enumerate() {
                    work.charge(Dimension::References, 1)?;
                    for candidate in &conflict.inputs {
                        work.charge(Dimension::References, 1)?;
                        if candidate == input {
                            groups.push(index);
                            break;
                        }
                    }
                }
                if groups.is_empty() {
                    Ok(*input)
                } else {
                    Err(ImportRefusal::ConflictingModel { groups })
                }
            }
            _ => Err(ImportRefusal::AmbiguousSelection { inputs: exact }),
        })
    }

    fn alias(
        &self,
        unit: UnitId,
        alias: &Spanned<String>,
        work: &mut Work,
    ) -> Result<&Selected<'a>, ModelError> {
        charge(work, Dimension::References, 1, unit, alias.span)?;
        charge(work, Dimension::Bytes, alias.value.len(), unit, alias.span)?;
        if !self.complete {
            return Err(failure(unit, alias.span, ModelErrorKind::IncompleteCatalog));
        }
        let Some(imports) = self
            .aliases
            .get(&unit)
            .and_then(|aliases| aliases.get(alias.value.as_str()))
        else {
            return Err(failure(unit, alias.span, ModelErrorKind::MissingAlias));
        };
        if imports.len() != 1 {
            return Err(failure(
                unit,
                alias.span,
                ModelErrorKind::AmbiguousAlias {
                    imports: imports.clone(),
                },
            ));
        }
        let input = self.imports[imports[0]].selection.as_ref().map_err(|_| {
            failure(
                unit,
                alias.span,
                ModelErrorKind::RefusedImport { import: imports[0] },
            )
        })?;
        Ok(self.catalogs[*input]
            .as_ref()
            .expect("collect indexes every selected input"))
    }

    /// The native exports behind `alias`; a domain package alias has no
    /// native type, operation or variant to export.
    fn native_alias(
        &self,
        unit: UnitId,
        alias: &Spanned<String>,
        span: Span,
        work: &mut Work,
    ) -> Result<&Exports<'a>, ModelError> {
        match self.alias(unit, alias, work)? {
            Selected::Native(exports) => Ok(exports),
            Selected::Domain(_) => Err(failure(unit, span, ModelErrorKind::WrongExportKind)),
        }
    }

    /// Resolve `name` at a type site: a native type as [`ModelTarget::Type`],
    /// a domain-package object, interface or value type as
    /// [`ModelTarget::Declaration`], bound by its FR-154 key
    /// (`ix://<package>/<artifact id>`). Any other domain declaration kind
    /// refuses `WrongExportKind`.
    pub fn resolve_target(
        &self,
        unit: UnitId,
        name: &QualifiedName,
        work: &mut Work,
    ) -> Result<ModelTarget<'a>, ModelError> {
        self.resolve_at(unit, name, Site::Type, work)
    }

    fn resolve_at(
        &self,
        unit: UnitId,
        name: &QualifiedName,
        site: Site,
        work: &mut Work,
    ) -> Result<ModelTarget<'a>, ModelError> {
        match self.alias(unit, &name.model, work)? {
            Selected::Native(exports) => {
                Self::export_type(exports, unit, name, work).map(ModelTarget::Type)
            }
            Selected::Domain(package) => {
                let bound = Self::domain_declaration(package, unit, &name.name, work)?;
                if site.admits(bound.kind()) {
                    Ok(ModelTarget::Declaration(bound))
                } else {
                    Err(failure(
                        unit,
                        name.name.span,
                        ModelErrorKind::WrongExportKind,
                    ))
                }
            }
        }
    }

    /// The type-definition node `name` names in `package`, with charges
    /// mirroring a native export lookup.
    fn domain_declaration(
        package: &'a AdmittedPackage,
        unit: UnitId,
        name: &Spanned<String>,
        work: &mut Work,
    ) -> Result<BoundDeclaration<'a>, ModelError> {
        charge(work, Dimension::References, 1, unit, name.span)?;
        charge(work, Dimension::Bytes, name.value.len(), unit, name.span)?;
        let declaration = package
            .declaration(&name.value)
            .ok_or_else(|| failure(unit, name.span, ModelErrorKind::MissingExport))?;
        charge(work, Dimension::Bindings, 1, unit, name.span)?;
        Ok(BoundDeclaration {
            package,
            declaration,
        })
    }

    /// The member `member` of the object type `owner` names, keyed
    /// `<owner identity>/<member>` (FR-154).
    fn domain_member(
        &self,
        unit: UnitId,
        owner: &QualifiedName,
        member: &Spanned<String>,
        work: &mut Work,
    ) -> Result<BoundDeclaration<'a>, ModelError> {
        let Selected::Domain(package) = self.alias(unit, &owner.model, work)? else {
            return Err(failure(
                unit,
                owner.name.span,
                ModelErrorKind::WrongExportKind,
            ));
        };
        let owner = Self::domain_declaration(package, unit, &owner.name, work)?;
        if !matches!(
            owner.kind(),
            DeclarationKind::ObjectType | DeclarationKind::Interface
        ) {
            return Err(failure(unit, member.span, ModelErrorKind::WrongExportKind));
        }
        charge(work, Dimension::References, 1, unit, member.span)?;
        charge(
            work,
            Dimension::Bytes,
            member.value.len(),
            unit,
            member.span,
        )?;
        let declaration = package
            .member(owner.key(), &member.value)
            .ok_or_else(|| failure(unit, member.span, ModelErrorKind::MissingExport))?;
        charge(work, Dimension::Bindings, 1, unit, member.span)?;
        Ok(BoundDeclaration {
            package: owner.package,
            declaration,
        })
    }

    /// Whether `alias` selects a domain package. Refused and unknown aliases
    /// report `false`; their own lookup refuses them.
    fn is_domain_alias(&self, unit: UnitId, alias: &Spanned<String>) -> bool {
        let Some([import]) = self
            .aliases
            .get(&unit)
            .and_then(|aliases| aliases.get(alias.value.as_str()))
            .map(Vec::as_slice)
        else {
            return false;
        };
        matches!(
            self.imports[*import].selection,
            Ok(input) if matches!(self.inputs[input], ModelInput::Domain(_))
        )
    }

    /// Resolve one qualified type through its source-local exact import.
    pub fn resolve_type(
        &self,
        unit: UnitId,
        name: &QualifiedName,
        work: &mut Work,
    ) -> Result<BoundType<'a>, ModelError> {
        let exports = self.native_alias(unit, &name.model, name.name.span, work)?;
        Self::export_type(exports, unit, name, work)
    }

    fn export_type(
        exports: &Exports<'a>,
        unit: UnitId,
        name: &QualifiedName,
        work: &mut Work,
    ) -> Result<BoundType<'a>, ModelError> {
        charge(work, Dimension::References, 1, unit, name.name.span)?;
        charge(
            work,
            Dimension::Bytes,
            name.name.value.len(),
            unit,
            name.name.span,
        )?;
        let catalog = &exports.catalog;
        let record = exports.records.get(name.name.value.as_str()).copied();
        let enumeration = exports.enumerations.get(name.name.value.as_str()).copied();
        let scalar = exports.scalars.get(name.name.value.as_str()).copied();
        if scalar.is_some() && (record.is_some() || enumeration.is_some()) {
            return Err(failure(
                unit,
                name.name.span,
                ModelErrorKind::AmbiguousExport,
            ));
        }
        let (native, key, source) = if let Some(role) = scalar {
            let native = scalar_type(catalog, role, unit, name.name.span, work)?;
            (
                native,
                DeclarationKey::Scalar(role.name.clone()),
                &role.source,
            )
        } else if let Some(record) = record {
            (
                catalog.record_type(record.name()).expect("indexed record"),
                DeclarationKey::Type(record.name().clone()),
                record.source(),
            )
        } else if let Some(declaration) = enumeration {
            (
                NativeType::Enumeration {
                    model: catalog.model,
                    declaration,
                },
                DeclarationKey::Type(declaration.name().clone()),
                declaration.source(),
            )
        } else {
            return Err(failure(unit, name.name.span, ModelErrorKind::MissingExport));
        };
        charge(work, Dimension::Bindings, 1, unit, name.name.span)?;
        Ok(BoundType {
            model: catalog.model,
            native,
            location: location(catalog.model.environment(), key, source),
        })
    }

    /// Resolve an authored parameter's existing native type; no value is supplied.
    pub fn parameter_type(
        &self,
        unit: UnitId,
        ty: &ParameterType,
        work: &mut Work,
    ) -> Result<NativeType<'a>, ModelError> {
        match ty {
            ParameterType::Boolean(span) => {
                charge(work, Dimension::References, 1, unit, *span)?;
                Ok(NativeType::Boolean)
            }
            ParameterType::Model(name) => self
                .resolve_type(unit, name, work)
                .map(|bound| bound.native),
        }
    }

    /// Resolve an explicit operation under its exact identity-bearing context.
    pub fn resolve_operation(
        &self,
        unit: UnitId,
        operation: &Operation,
        work: &mut Work,
    ) -> Result<BoundOperation<'a>, ModelError> {
        let exports = self.native_alias(
            unit,
            &operation.context.model,
            operation.context.name.span,
            work,
        )?;
        let context = Self::export_type(exports, unit, &operation.context, work)?;
        let NativeType::Object { role, .. } = &context.native else {
            return Err(failure(
                unit,
                operation.context.name.span,
                ModelErrorKind::WrongExportKind,
            ));
        };
        charge(work, Dimension::References, 1, unit, operation.name.span)?;
        charge(
            work,
            Dimension::Bytes,
            operation.name.value.len(),
            unit,
            operation.name.span,
        )?;
        let role = exports
            .operations
            .get(&(role.record.as_str(), operation.name.value.as_str()))
            .copied()
            .ok_or_else(|| failure(unit, operation.name.span, ModelErrorKind::MissingExport))?;
        charge(work, Dimension::Bindings, 1, unit, operation.name.span)?;
        Ok(BoundOperation {
            model: context.model,
            role,
            location: location(
                context.model.environment(),
                DeclarationKey::Operation {
                    context: role.context.clone(),
                    name: role.name.clone(),
                },
                &role.source,
            ),
        })
    }

    fn exports_for(
        &self,
        model: &NativeModel,
        unit: UnitId,
        span: Span,
        work: &mut Work,
    ) -> Result<&Exports<'a>, ModelError> {
        if !self.complete {
            return Err(failure(unit, span, ModelErrorKind::IncompleteCatalog));
        }
        for (input, exports) in self.catalogs.iter().enumerate() {
            charge(work, Dimension::References, 1, unit, span)?;
            let Some(Selected::Native(exports)) = exports else {
                continue;
            };
            if std::ptr::eq(exports.catalog.model, model) {
                for imports in self
                    .aliases
                    .get(&unit)
                    .into_iter()
                    .flat_map(|aliases| aliases.values())
                {
                    charge(work, Dimension::References, 1, unit, span)?;
                    if imports.len() == 1 && self.imports[imports[0]].selection == Ok(input) {
                        return Ok(exports);
                    }
                }
                return Err(failure(unit, span, ModelErrorKind::ForeignModel));
            }
        }
        Err(failure(unit, span, ModelErrorKind::ForeignModel))
    }

    /// Bind an actual operation input/result declaration by name. Callers still
    /// enforce pre/post availability and may seed only the operation's parameters.
    pub fn value(
        &self,
        unit: UnitId,
        operation: &BoundOperation<'a>,
        name: &Spanned<String>,
        work: &mut Work,
    ) -> Result<BoundType<'a>, ModelError> {
        let exports = self.exports_for(operation.model, unit, name.span, work)?;
        let catalog = &exports.catalog;
        charge(work, Dimension::Bytes, name.value.len(), unit, name.span)?;
        charge(work, Dimension::References, 1, unit, name.span)?;
        let parameter = find_charged(&operation.role.parameters, work, unit, name.span, |value| {
            value.as_str() == name.value
        })?;
        let result = if parameter.is_none() {
            find_charged(
                operation.role.result.iter(),
                work,
                unit,
                name.span,
                |value| value.as_str() == name.value,
            )?
        } else {
            None
        };
        if parameter.is_none() && result.is_none() {
            return Err(failure(unit, name.span, ModelErrorKind::MissingExport));
        }
        let value = exports
            .values
            .get(name.value.as_str())
            .copied()
            .ok_or_else(|| failure(unit, name.span, ModelErrorKind::MissingExport))?;
        formal_type(
            catalog,
            value.value_type(),
            ScalarSite::Value {
                name: value.name().clone(),
            },
            unit,
            name.span,
            work,
        )
        .map(|native| BoundType {
            model: operation.model,
            native,
            location: location(
                operation.model.environment(),
                DeclarationKey::Value(value.name().clone()),
                value.source(),
            ),
        })
    }

    /// Bind a record/object member while preserving its original field and scalar
    /// owner. Opaque reference-carrier fields require explicit dereference first.
    pub fn field(
        &self,
        unit: UnitId,
        receiver: &BoundType<'a>,
        name: &Spanned<String>,
        work: &mut Work,
    ) -> Result<BoundType<'a>, ModelError> {
        let catalog = &self
            .exports_for(receiver.model, unit, name.span, work)?
            .catalog;
        charge(work, Dimension::Bytes, name.value.len(), unit, name.span)?;
        charge(work, Dimension::References, 1, unit, name.span)?;
        let record = match &receiver.native {
            NativeType::Object { role, .. } => &role.record,
            NativeType::Record { declaration, .. } => declaration.name(),
            NativeType::Boolean
            | NativeType::Scalar { .. }
            | NativeType::Enumeration { .. }
            | NativeType::Reference { .. }
            | NativeType::Option(_)
            | NativeType::Sequence { .. }
            | NativeType::Domain(_) => {
                return Err(failure(unit, name.span, ModelErrorKind::WrongExportKind));
            }
        };
        // Catalog stores each record's fields sorted by their exact names.
        // Charge each binary-search comparison before inspecting its candidate.
        let fields = catalog
            .ordered_fields
            .get(record)
            .expect("admitted record fields");
        let mut low = 0;
        let mut high = fields.len();
        let mut selected = None;
        while low < high {
            charge(work, Dimension::References, 1, unit, name.span)?;
            let middle = low + (high - low) / 2;
            let field = fields[middle];
            match field.name().as_str().cmp(name.value.as_str()) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => {
                    selected = Some(field);
                    break;
                }
            }
        }
        let field =
            selected.ok_or_else(|| failure(unit, name.span, ModelErrorKind::MissingExport))?;
        let native = formal_type(
            catalog,
            field.value_type(),
            ScalarSite::Field {
                record: record.clone(),
                field: field.name().clone(),
            },
            unit,
            name.span,
            work,
        )?;
        Ok(BoundType {
            model: receiver.model,
            native,
            location: location(
                receiver.model.environment(),
                DeclarationKey::Field {
                    record: record.clone(),
                    field: field.name().clone(),
                },
                field.source(),
            ),
        })
    }

    /// Resolve an enumeration member to the actual owner-qualified variant locus.
    pub fn variant(
        &self,
        unit: UnitId,
        enumeration: &QualifiedName,
        variant: &Spanned<String>,
        work: &mut Work,
    ) -> Result<BoundType<'a>, ModelError> {
        let mut bound = self.resolve_type(unit, enumeration, work)?;
        charge(work, Dimension::References, 1, unit, variant.span)?;
        charge(
            work,
            Dimension::Bytes,
            variant.value.len(),
            unit,
            variant.span,
        )?;
        let NativeType::Enumeration { declaration, .. } = &bound.native else {
            return Err(failure(unit, variant.span, ModelErrorKind::WrongExportKind));
        };
        let member = find_charged(declaration.variants(), work, unit, variant.span, |member| {
            member.name().as_str() == variant.value
        })?
        .ok_or_else(|| failure(unit, variant.span, ModelErrorKind::MissingExport))?;
        bound.location = location(
            bound.model.environment(),
            DeclarationKey::Variant {
                enumeration: declaration.name().clone(),
                variant: member.name().clone(),
            },
            member.source(),
        );
        Ok(bound)
    }
}

// Reserve every stored entry of the selected shared Catalog and Exports before
// constructing either. This counts entries, not allocator capacity or byte size.
fn charge_exports(model: &NativeModel, work: &mut Work) -> Result<(), Exhaustion> {
    for ty in model.environment().types() {
        work.charge(Dimension::References, 1)?;
        // Shared name map + ordered-fields/variant map + borrowed-name map.
        work.charge(Dimension::Bindings, 3)?;
        match ty {
            ir::TypeDeclaration::Record { declaration } => {
                // Shared field map and the ordered field vector.
                work.charge(Dimension::Bindings, declaration.fields().len())?;
                work.charge(Dimension::Bindings, declaration.fields().len())?;
            }
            ir::TypeDeclaration::Enum { declaration } => {
                work.charge(Dimension::Bindings, declaration.variants().len())?
            }
        }
    }
    work.charge(Dimension::Bindings, model.environment().values().len())?;
    work.charge(Dimension::References, model.environment().values().len())?;
    work.charge(Dimension::Bindings, model.environment().values().len())?;
    for scalar in &model.roles().scalars {
        work.charge(Dimension::References, 1)?;
        work.charge(Dimension::Bindings, 1)?;
        work.charge(Dimension::Bindings, scalar.sites.len())?;
    }
    work.charge(Dimension::Bindings, model.roles().objects.len())?;
    work.charge(Dimension::Bindings, model.roles().objects.len())?;
    for operation in &model.roles().operations {
        work.charge(Dimension::References, 1)?;
        work.charge(Dimension::Bindings, 2)?;
        work.charge(Dimension::Bindings, operation.frame.fields.len())?;
        work.charge(Dimension::Bindings, operation.frame.created.len())?;
        work.charge(Dimension::Bindings, operation.frame.deleted.len())?;
    }
    Ok(())
}

fn scalar_type<'a>(
    catalog: &Catalog<'a>,
    role: &'a ScalarRole,
    unit: UnitId,
    span: Span,
    work: &mut Work,
) -> Result<NativeType<'a>, ModelError> {
    let site = &role.sites[0];
    let representation = match site {
        ScalarSite::Value { name } => catalog.values[name].value_type(),
        ScalarSite::Field { record, field } => catalog.fields[&(record, field)].value_type(),
    };
    let (representation, _) = primitive(representation, unit, span, work)?;
    Ok(NativeType::Scalar {
        model: catalog.model,
        role,
        representation,
    })
}

fn formal_type<'a>(
    catalog: &Catalog<'a>,
    ty: &'a ir::ValueType,
    site: ScalarSite,
    unit: UnitId,
    span: Span,
    work: &mut Work,
) -> Result<NativeType<'a>, ModelError> {
    let (_, layers) = primitive(ty, unit, span, work)?;
    // Unlike a scalar-name lookup, a field/value export constructs each actual
    // Option/Sequence wrapper as well as its leaf. Reserve those type records
    // before Catalog::formal allocates them; traversal itself only costs reads.
    charge(work, Dimension::Bindings, layers, unit, span)?;
    catalog
        .formal(ty, &site)
        .ok_or_else(|| failure(unit, span, ModelErrorKind::WrongExportKind))
}

fn primitive<'a>(
    mut ty: &'a ir::ValueType,
    unit: UnitId,
    span: Span,
    work: &mut Work,
) -> Result<(&'a ir::ValueType, usize), ModelError> {
    let mut layers = 0;
    loop {
        charge(work, Dimension::References, 1, unit, span)?;
        layers += 1; // admitted NativeModel type depth is bounded by 64
        ty = match ty {
            ir::ValueType::Option { value } => value,
            ir::ValueType::Collection { value } => value.element(),
            ir::ValueType::Boolean
            | ir::ValueType::Integer { .. }
            | ir::ValueType::Rational { .. }
            | ir::ValueType::Text
            | ir::ValueType::Enum { .. }
            | ir::ValueType::Record { .. } => return Ok((ty, layers)),
        };
    }
}

/// The concrete export selected by an authored qualified occurrence.
#[derive(Clone, Debug)]
pub enum ModelTarget<'a> {
    /// Nominal type, scalar, or enumeration variant with its original location.
    Type(BoundType<'a>),
    /// Explicit operation role and invocation contract.
    Operation(BoundOperation<'a>),
    /// A domain-package declaration, bound by its exact key and kind.
    Declaration(BoundDeclaration<'a>),
}

impl<'a> ModelTarget<'a> {
    /// The value type a type-site occurrence names: the bound native type,
    /// or the domain type of a domain object type, Interface or record value
    /// type. `None` for an operation or any other domain declaration kind.
    pub fn value_type(&self) -> Option<NativeType<'a>> {
        match self {
            Self::Type(bound) => Some(bound.native().clone()),
            Self::Declaration(bound) => {
                DomainType::new(bound.package(), bound.declaration()).map(NativeType::Domain)
            }
            Self::Operation(_) => None,
        }
    }
}

/// An occurrence belongs to the report's declaration and declaring source unit.
#[derive(Clone, Debug)]
pub struct ModelOccurrence<'a> {
    /// Original qualified occurrence, including alias and member tokens.
    pub span: Span,
    /// Actual selected model export; no expression evaluation is implied.
    pub target: ModelTarget<'a>,
}

/// Dependency-local model results. Orchestration propagates these refusals along
/// the namespace's existing native dependency graph.
#[derive(Debug)]
pub struct ModelDeclarationReport<'a> {
    /// Namespace-local owner of every occurrence and refusal.
    pub declaration: DeclarationId,
    /// Successfully resolved original occurrences, including before exhaustion.
    pub occurrences: Vec<ModelOccurrence<'a>>,
    /// All observed model causes; resource exhaustion ends further traversal.
    pub refusals: Vec<ModelError>,
    /// Traversal finished over a complete catalog; errors may still refuse binding.
    pub complete: bool,
}

impl ModelDeclarationReport<'_> {
    /// First unaffordable lookup or traversal charge, if work did not finish.
    pub fn exhaustion(&self) -> Option<&Exhaustion> {
        self.refusals.iter().find_map(|error| match &error.kind {
            ModelErrorKind::ResourceExhausted(exhaustion) => Some(exhaustion),
            ModelErrorKind::IncompleteCatalog
            | ModelErrorKind::MissingAlias
            | ModelErrorKind::AmbiguousAlias { .. }
            | ModelErrorKind::RefusedImport { .. }
            | ModelErrorKind::MissingExport
            | ModelErrorKind::AmbiguousExport
            | ModelErrorKind::WrongExportKind
            | ModelErrorKind::ForeignModel
            | ModelErrorKind::UnsupportedRelationshipContract => None,
        })
    }
}

impl<'a> ModelBindings<'a> {
    /// Bind every authored type/operation/variant qualification in one declaration.
    /// Each inspected parameter/control/value node costs one Reference; imports
    /// and actual lookups keep their separately documented charges. Value/control
    /// arenas are restricted to this declaration through charged binary boundary
    /// lookups; unrelated nodes are not rescanned. Each retained cause also costs
    /// one Binding, except the terminal exhaustion record. An out-of-range
    /// declaration handle returns None.
    pub fn resolve_declaration(
        &self,
        namespace: &SyntaxNamespace,
        declaration: DeclarationId,
        work: &mut Work,
    ) -> Option<ModelDeclarationReport<'a>> {
        let entry = namespace.declaration(declaration)?;
        let syntax = namespace.syntax(declaration)?;
        let unit = namespace.unit(entry.unit())?;
        let mut walk = ModelWalk {
            models: self,
            work,
            unit: entry.unit(),
            report: ModelDeclarationReport {
                declaration,
                occurrences: Vec::new(),
                refusals: Vec::new(),
                complete: false,
            },
        };
        let result = (|| {
            charge(walk.work, Dimension::Bindings, 1, walk.unit, syntax.span)?;
            match &syntax.kind {
                c::DeclarationKind::Predicate { parameters, .. } => {
                    for parameter in parameters {
                        walk.parameter(parameter)?;
                    }
                }
                c::DeclarationKind::State {
                    context, operation, ..
                } => {
                    if let Some(name) = operation {
                        walk.operation(&Operation {
                            context: context.clone(),
                            name: name.clone(),
                        })?;
                    } else {
                        walk.ty(context)?;
                    }
                }
                c::DeclarationKind::Temporal {
                    input,
                    activation,
                    captures,
                    ..
                } => {
                    walk.parameter(input)?;
                    walk.activation(activation)?;
                    walk.captures(captures)?;
                }
                c::DeclarationKind::Protocol(protocol) => {
                    walk.parameter(&protocol.input)?;
                    walk.activation(&protocol.activation)?;
                    walk.captures(&protocol.captures)?;
                    for role in &protocol.roles {
                        walk.role(&role.model)?;
                    }
                    for relation in &protocol.relationships {
                        walk.visit(relation.span)?;
                        walk.relationship(relation)?;
                    }
                    for channel in &protocol.channels {
                        walk.ty(&channel.carries)?;
                        if let c::Ordering::Fifo { parameter, .. } = &channel.ordering {
                            walk.parameter(parameter)?;
                        }
                    }
                    for requirement in &protocol.requirements {
                        walk.visit(syntax.span)?;
                        if let c::ProtocolRequirement::Compensation(compensation) = requirement {
                            walk.parameter(&compensation.forward)?;
                            walk.operation(&compensation.operation)?;
                            walk.captures(&compensation.registration_captures)?;
                            walk.parameter(&compensation.trigger)?;
                            walk.captures(&compensation.activation_captures)?;
                            walk.ty(&compensation.attempt_type)?;
                            walk.parameter(&compensation.earlier)?;
                            walk.parameter(&compensation.later)?;
                            walk.parameter(&compensation.recovery)?;
                        }
                    }
                    walk.parameter(&protocol.finish.parameter)?;
                }
            }
            let controls = super::arena::owned(
                unit.controls(),
                syntax.span,
                |control| control.span,
                walk.work,
            )
            .map_err(|error| {
                failure(
                    walk.unit,
                    syntax.span,
                    ModelErrorKind::ResourceExhausted(error),
                )
            })?;
            for control in controls {
                walk.visit(control.span)?;
                match &control.kind {
                    c::ControlKind::Event(event) => {
                        walk.parameter(&event.parameter)?;
                        if let c::EventKind::Attempt { operation, .. } = &event.kind {
                            walk.operation(operation)?;
                        }
                    }
                    c::ControlKind::Commit { parameter, .. } => walk.parameter(parameter)?,
                    c::ControlKind::Sequence(_)
                    | c::ControlKind::Choice { .. }
                    | c::ControlKind::Parallel { .. }
                    | c::ControlKind::Repeat { .. }
                    | c::ControlKind::Await { .. }
                    | c::ControlKind::Check { .. } => {}
                }
            }
            let expressions = super::arena::owned(
                unit.expressions(),
                syntax.span,
                |expression| expression.span,
                walk.work,
            )
            .map_err(|error| {
                failure(
                    walk.unit,
                    syntax.span,
                    ModelErrorKind::ResourceExhausted(error),
                )
            })?;
            for expression in expressions {
                walk.visit(expression.span)?;
                match &expression.kind {
                    c::ValueKind::Size { domain, .. } => walk.ty(domain)?,
                    c::ValueKind::Query {
                        result: Some(result),
                        ..
                    } => walk.ty(result)?,
                    c::ValueKind::Shared(crate::syntax::ExprKind::EnumValue {
                        model,
                        name,
                        variant,
                    }) => {
                        let name = QualifiedName {
                            model: model.clone(),
                            name: name.clone(),
                        };
                        let result = if self.is_domain_alias(walk.unit, &name.model) {
                            self.domain_member(walk.unit, &name, variant, walk.work)
                                .and_then(|bound| {
                                    if bound.kind() == DeclarationKind::Field {
                                        Ok(ModelTarget::Declaration(bound))
                                    } else {
                                        Err(failure(
                                            walk.unit,
                                            variant.span,
                                            ModelErrorKind::WrongExportKind,
                                        ))
                                    }
                                })
                        } else {
                            self.variant(walk.unit, &name, variant, walk.work)
                                .map(ModelTarget::Type)
                        };
                        walk.record(
                            result,
                            Span {
                                start: name.model.span.start,
                                end: variant.span.end,
                            },
                        )?;
                    }
                    c::ValueKind::Shared(_)
                    | c::ValueKind::Invoke { .. }
                    | c::ValueKind::Rational { .. }
                    | c::ValueKind::Product { .. }
                    | c::ValueKind::Contains { .. }
                    | c::ValueKind::Query { result: None, .. } => {}
                }
            }
            Ok::<(), ModelError>(())
        })();
        match result {
            Ok(()) => walk.report.complete = self.complete,
            Err(error) => walk.report.refusals.push(error),
        }
        Some(walk.report)
    }
}

struct ModelWalk<'b, 'a> {
    models: &'b ModelBindings<'a>,
    work: &'b mut Work,
    unit: UnitId,
    report: ModelDeclarationReport<'a>,
}

impl<'a> ModelWalk<'_, 'a> {
    fn visit(&mut self, span: Span) -> Result<(), ModelError> {
        charge(self.work, Dimension::References, 1, self.unit, span)
    }
    fn record(
        &mut self,
        result: Result<ModelTarget<'a>, ModelError>,
        span: Span,
    ) -> Result<(), ModelError> {
        match result {
            Ok(target) => self
                .report
                .occurrences
                .push(ModelOccurrence { span, target }),
            Err(error) if matches!(error.kind, ModelErrorKind::ResourceExhausted(_)) => {
                return Err(error)
            }
            Err(error) => {
                charge(self.work, Dimension::Bindings, 1, self.unit, span)?;
                self.report.refusals.push(error);
            }
        }
        Ok(())
    }
    fn role(&mut self, name: &QualifiedName) -> Result<(), ModelError> {
        let result = self
            .models
            .resolve_at(self.unit, name, Site::Role, self.work);
        self.record(
            result,
            Span {
                start: name.model.span.start,
                end: name.name.span.end,
            },
        )
    }
    fn ty(&mut self, name: &QualifiedName) -> Result<(), ModelError> {
        let result = self.models.resolve_target(self.unit, name, self.work);
        self.record(
            result,
            Span {
                start: name.model.span.start,
                end: name.name.span.end,
            },
        )
    }
    fn operation(&mut self, operation: &Operation) -> Result<(), ModelError> {
        let result = if self
            .models
            .is_domain_alias(self.unit, &operation.context.model)
        {
            self.models
                .domain_member(self.unit, &operation.context, &operation.name, self.work)
                .and_then(|bound| {
                    if bound.kind() == DeclarationKind::Operation {
                        Ok(ModelTarget::Declaration(bound))
                    } else {
                        Err(failure(
                            self.unit,
                            operation.name.span,
                            ModelErrorKind::WrongExportKind,
                        ))
                    }
                })
        } else {
            self.models
                .resolve_operation(self.unit, operation, self.work)
                .map(ModelTarget::Operation)
        };
        self.record(
            result,
            Span {
                start: operation.context.model.span.start,
                end: operation.name.span.end,
            },
        )
    }
    // A domain package's relationship declaration (a Connection or a
    // navigation relationship) binds by key and kind. A native model has no
    // relationship export: its occurrence records the named type, so the
    // occurrence still carries a located type reference, and then refuses.
    fn relationship(&mut self, relationship: &c::Relationship) -> Result<(), ModelError> {
        let name = &relationship.model;
        let span = Span {
            start: name.model.span.start,
            end: name.name.span.end,
        };
        match self
            .models
            .resolve_at(self.unit, name, Site::Relationship, self.work)
        {
            Ok(target @ ModelTarget::Declaration(_)) => self.record(Ok(target), span),
            Ok(ModelTarget::Operation(_)) => self.record(
                Err(failure(
                    self.unit,
                    name.name.span,
                    ModelErrorKind::WrongExportKind,
                )),
                span,
            ),
            Ok(ModelTarget::Type(bound)) => {
                self.record(Ok(ModelTarget::Type(bound)), span)?;
                self.record(
                    Err(failure(
                        self.unit,
                        relationship.span,
                        ModelErrorKind::UnsupportedRelationshipContract,
                    )),
                    relationship.span,
                )
            }
            Err(error) => self.record(Err(error), span),
        }
    }
    fn parameter(&mut self, parameter: &c::Parameter) -> Result<(), ModelError> {
        self.visit(parameter.span)?;
        if let ParameterType::Model(name) = &parameter.ty {
            self.ty(name)?;
        }
        Ok(())
    }
    fn activation(&mut self, activation: &c::Activation) -> Result<(), ModelError> {
        if let c::Activation::Each { trigger, .. } = activation {
            self.parameter(trigger)?;
        }
        Ok(())
    }
    fn captures(&mut self, captures: &[c::Capture]) -> Result<(), ModelError> {
        for capture in captures {
            self.parameter(&capture.parameter)?;
        }
        Ok(())
    }
}
