// SPDX-License-Identifier: AGPL-3.0-only
//! FR-036: exact unit-local imports over the existing admitted NativeModel.
//! No model schema, runtime population or producer canonical correspondence is inferred.

use std::collections::BTreeMap;

use quire_contract_ir as ir;

use super::binding_work::{Dimension, Exhaustion, Work};
use super::{DeclarationId, SyntaxNamespace, UnitId};
use crate::checking::{Catalog, NativeType};
use crate::linking::{location, DeclarationKey, DeclarationLocation};
use crate::native_model::{NativeModel, OperationRole, ScalarRole, ScalarSite};
use crate::syntax::composed::{self as c, Operation, ParameterType, QualifiedName};
use crate::{ByteDigest, Span, Spanned};

/// Supplied model selections; only constructor-admitted native artifacts bind.
#[derive(Clone, Copy, Debug)]
pub enum ModelInput<'a> {
    /// Constructor-admitted native model, preserving its actual profile and declarations.
    Native(&'a NativeModel),
    /// An explicit external selection whose correspondence is not implemented.
    /// These labels select a refusal, never assert producer validity.
    UnsupportedProducer {
        /// Selected package label.
        package: &'a str,
        /// Selected revision spelling.
        revision: &'a str,
        /// Selected raw native artifact digest, never a canonical producer hash.
        digest: ByteDigest,
        /// Required producer interface retained for the caller's disposition.
        interface: &'a str,
    },
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
    /// Digest spelling is not the existing canonical raw SHA-256 spelling.
    InvalidDigest,
    /// A native model revision is not the canonical unsigned decimal IR revision.
    InvalidRevision,
    /// No supplied model has this package label.
    MissingPackage,
    /// No candidate has all selected revision/digest components.
    StaleSelection,
    /// Multiple candidates match the complete selection.
    AmbiguousSelection { inputs: Vec<usize> },
    /// Shared conflict indices into ModelBindings::conflicts().
    ConflictingModel { groups: Vec<usize> },
    /// The exact selected producer interface has no admitted correspondence.
    UnsupportedCorrespondence { input: usize },
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
    AmbiguousAlias { imports: Vec<usize> },
    /// The selected import refused; index addresses imports().
    RefusedImport { import: usize },
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

#[derive(Debug)]
struct Exports<'a> {
    catalog: Catalog<'a>,
    scalars: BTreeMap<&'a str, &'a ScalarRole>,
}

/// Partial import evidence remains inspectable after exhaustion. Qualified
/// lookups require the complete supplied inventory to have been examined.
#[derive(Debug)]
pub struct ModelBindings<'a> {
    inputs: &'a [ModelInput<'a>],
    catalogs: Vec<Option<Exports<'a>>>,
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
            .map(|exports| &exports.catalog)
    }
    /// Original supplied artifacts and explicit unsupported interface selections.
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

/// Charge supplied Models and artifact/selector Bytes first. Bindings count
/// export-index entries, imports and returned resolution records; References
/// count lookup attempts plus each inspected catalog, operation, parameter,
/// member, owner/source comparison and conflict-input entry. Borrowed indexes
/// retain their existing lookup charge. No dependency edge is added.
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
        let mut owners = BTreeMap::<&ir::RequirementRef, Vec<usize>>::new();
        let mut sources = BTreeMap::<&ir::SourceIdentity, Vec<usize>>::new();
        for (index, input) in self.inputs.iter().enumerate() {
            work.charge(Dimension::Models, 1)?;
            match input {
                ModelInput::Native(model) => {
                    work.charge(Dimension::Bytes, model.artifact_bytes().len())?;
                    owners
                        .entry(model.environment().owner())
                        .or_default()
                        .push(index);
                    sources
                        .entry(model.source().identity())
                        .or_default()
                        .push(index);
                    charge_exports(model, work)?;
                    self.catalogs.push(Some(Exports {
                        catalog: Catalog::new(model),
                        scalars: model
                            .roles()
                            .scalars
                            .iter()
                            .map(|role| (role.name.as_str(), role))
                            .collect(),
                    }));
                }
                ModelInput::UnsupportedProducer {
                    package,
                    revision,
                    interface,
                    ..
                } => {
                    text(work, &[package, revision, interface])?;
                    self.catalogs.push(None);
                }
            }
        }
        for inputs in owners.into_values() {
            if inputs.len() < 2 {
                continue;
            }
            work.charge(Dimension::References, 1)?;
            let first = self.native(inputs[0]).digest();
            let mut different = false;
            for input in &inputs[1..] {
                work.charge(Dimension::References, 1)?;
                if self.native(*input).digest() != first {
                    different = true;
                    break;
                }
            }
            if different {
                work.charge(Dimension::Bindings, 1)?;
                self.conflicts.push(ModelConflict {
                    kind: ModelConflictKind::Owner,
                    inputs,
                });
            }
        }
        for inputs in sources.into_values() {
            if inputs.len() < 2 {
                continue;
            }
            work.charge(Dimension::References, 1)?;
            let first = self.native(inputs[0]).source().source();
            let mut different = false;
            for input in &inputs[1..] {
                work.charge(Dimension::References, 1)?;
                let source = self.native(*input).source().source();
                if source.identity() != first.identity() || source.digest() != first.digest() {
                    different = true;
                    break;
                }
            }
            if different {
                work.charge(Dimension::Bindings, 1)?;
                self.conflicts.push(ModelConflict {
                    kind: ModelConflictKind::Source,
                    inputs,
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
                let selected = match selection.digest.value.parse::<ByteDigest>() {
                    Err(_) => Err(ImportRefusal::InvalidDigest),
                    Ok(digest) => self.select(
                        &selection.package.value,
                        &selection.version.value,
                        digest,
                        work,
                    )?,
                };
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
            }
        }
        Ok(())
    }

    fn native(&self, input: usize) -> &'a NativeModel {
        match self.inputs[input] {
            ModelInput::Native(model) => model,
            ModelInput::UnsupportedProducer { .. } => unreachable!("native index only"),
        }
    }

    fn select(
        &self,
        package: &str,
        revision: &str,
        digest: ByteDigest,
        work: &mut Work,
    ) -> Result<Result<usize, ImportRefusal>, Exhaustion> {
        let mut same_package = false;
        // Parse a native revision once. Unsupported producer labels remain opaque
        // and can only select their explicit unsupported-correspondence refusal.
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
            let matches = match input {
                ModelInput::Native(model) => {
                    let owner = model.environment().owner();
                    same_package |= owner.package().as_str() == package;
                    malformed_native_revision |=
                        owner.package().as_str() == package && native_revision.is_none();
                    owner.package().as_str() == package
                        && native_revision.as_ref() == Some(&owner.revision())
                        && model.digest() == digest
                }
                ModelInput::UnsupportedProducer {
                    package: selected,
                    revision: selected_revision,
                    digest: selected_digest,
                    ..
                } => {
                    same_package |= *selected == package;
                    *selected == package
                        && *selected_revision == revision
                        && *selected_digest == digest
                }
            };
            if matches {
                exact.push(index);
            }
        }
        Ok(match exact.as_slice() {
            [] if !same_package => Err(ImportRefusal::MissingPackage),
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
                if !groups.is_empty() {
                    Err(ImportRefusal::ConflictingModel { groups })
                } else if matches!(self.inputs[*input], ModelInput::UnsupportedProducer { .. }) {
                    Err(ImportRefusal::UnsupportedCorrespondence { input: *input })
                } else {
                    Ok(*input)
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
    ) -> Result<&Exports<'a>, ModelError> {
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
            .expect("selected native catalog"))
    }

    /// Resolve one qualified type through its source-local exact import.
    pub fn resolve_type(
        &self,
        unit: UnitId,
        name: &QualifiedName,
        work: &mut Work,
    ) -> Result<BoundType<'a>, ModelError> {
        let exports = self.alias(unit, &name.model, work)?;
        charge(work, Dimension::References, 1, unit, name.name.span)?;
        charge(
            work,
            Dimension::Bytes,
            name.name.value.len(),
            unit,
            name.name.span,
        )?;
        let catalog = &exports.catalog;
        let record = find_charged(
            catalog.records.values().copied(),
            work,
            unit,
            name.name.span,
            |record| record.name().as_str() == name.name.value,
        )?;
        let enumeration = find_charged(
            catalog.enumerations.values().copied(),
            work,
            unit,
            name.name.span,
            |enumeration| enumeration.name().as_str() == name.name.value,
        )?;
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
        let context = self.resolve_type(unit, &operation.context, work)?;
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
        let role = find_charged(
            &context.model.roles().operations,
            work,
            unit,
            operation.name.span,
            |candidate| {
                candidate.context == role.record && candidate.name.as_str() == operation.name.value
            },
        )?
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

    fn catalog_for(
        &self,
        model: &NativeModel,
        unit: UnitId,
        span: Span,
        work: &mut Work,
    ) -> Result<&Catalog<'a>, ModelError> {
        if !self.complete {
            return Err(failure(unit, span, ModelErrorKind::IncompleteCatalog));
        }
        for (input, exports) in self.catalogs.iter().enumerate() {
            charge(work, Dimension::References, 1, unit, span)?;
            let Some(exports) = exports else {
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
                        return Ok(&exports.catalog);
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
        let catalog = self.catalog_for(operation.model, unit, name.span, work)?;
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
        let value = find_charged(
            catalog.values.values().copied(),
            work,
            unit,
            name.span,
            |value| value.name().as_str() == name.value,
        )?
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
        let catalog = self.catalog_for(receiver.model, unit, name.span, work)?;
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
            | NativeType::Sequence { .. } => {
                return Err(failure(unit, name.span, ModelErrorKind::WrongExportKind));
            }
        };
        // The receiver supplies the actual SymbolName, so use the existing
        // borrowed record index before scanning only that record's fields.
        let fields = catalog
            .ordered_fields
            .get(record)
            .expect("admitted record fields");
        let field = find_charged(fields.iter().copied(), work, unit, name.span, |field| {
            field.name().as_str() == name.value
        })?
        .ok_or_else(|| failure(unit, name.span, ModelErrorKind::MissingExport))?;
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

// Reserve the existing Catalog's export/index records before it allocates them.
fn charge_exports(model: &NativeModel, work: &mut Work) -> Result<(), Exhaustion> {
    for ty in model.environment().types() {
        work.charge(Dimension::Bindings, 1)?;
        match ty {
            ir::TypeDeclaration::Record { declaration } => {
                work.charge(Dimension::Bindings, declaration.fields().len())?
            }
            ir::TypeDeclaration::Enum { declaration } => {
                work.charge(Dimension::Bindings, declaration.variants().len())?
            }
        }
    }
    work.charge(Dimension::Bindings, model.environment().values().len())?;
    for scalar in &model.roles().scalars {
        work.charge(Dimension::Bindings, 1)?;
        work.charge(Dimension::Bindings, scalar.sites.len())?;
    }
    work.charge(Dimension::Bindings, model.roles().objects.len())?;
    for operation in &model.roles().operations {
        work.charge(Dimension::Bindings, 1)?;
        work.charge(Dimension::Bindings, operation.parameters.len())?;
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
                        walk.ty(&role.model)?;
                    }
                    for relation in &protocol.relationships {
                        walk.visit(relation.span)?;
                        walk.ty(&relation.model)?;
                        walk.record(
                            Err(failure(
                                walk.unit,
                                relation.span,
                                ModelErrorKind::UnsupportedRelationshipContract,
                            )),
                            relation.span,
                        )?;
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
                        let result = self.variant(walk.unit, &name, variant, walk.work);
                        walk.record(
                            result.map(ModelTarget::Type),
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
    fn ty(&mut self, name: &QualifiedName) -> Result<(), ModelError> {
        let result = self
            .models
            .resolve_type(self.unit, name, self.work)
            .map(ModelTarget::Type);
        self.record(
            result,
            Span {
                start: name.model.span.start,
                end: name.name.span.end,
            },
        )
    }
    fn operation(&mut self, operation: &Operation) -> Result<(), ModelError> {
        let result = self
            .models
            .resolve_operation(self.unit, operation, self.work)
            .map(ModelTarget::Operation);
        self.record(
            result,
            Span {
                start: operation.context.model.span.start,
                end: operation.name.span.end,
            },
        )
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
