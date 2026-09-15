// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-005/013/015: exact imports and formal/native declaration correspondence.
//! Linking establishes names and provenance, not typing or evaluability.

pub mod composed;
mod native;

use std::collections::BTreeSet;

use quire_contract_ir::{
    CanonicalProfile, DeclarationEnvironment, RecordDeclaration, RequirementRef, SourceSpan,
    SymbolName, TypeDeclaration, ValueDeclarationKind, ValueType,
};

use crate::native_model::{NativeModel, NativeModelProfile, ObjectRole, OperationRole};
use crate::syntax::{BinaryOp, Builtin, Clause, ClauseKind, ExprId, ExprKind, UnaryOp};
use crate::{ByteDigest, Code, Diagnostic, ParsedUnit, Phase, Span};

/// Caller-lowered ceilings for native formal linkage.
#[derive(Clone, Copy, Debug)]
pub struct LinkLimits {
    /// Supplied environments, at most 64.
    pub models: usize,
    /// Native imports, at most 64.
    pub imports: usize,
    /// Native clauses, at most 256.
    pub clauses: usize,
    /// Flat native expression nodes, at most 10,000.
    pub nodes: usize,
    /// Recursive traversal levels, at most 64.
    pub depth: usize,
    /// Selected artifact bytes per model, at most 1 MiB.
    pub model_bytes: usize,
    /// Selected artifact bytes across the inventory, at most 8 MiB.
    pub total_model_bytes: usize,
}

impl Default for LinkLimits {
    fn default() -> Self {
        Self {
            models: 64,
            imports: 64,
            clauses: 256,
            nodes: 10_000,
            depth: 64,
            model_bytes: 1_048_576,
            total_model_bytes: 8_388_608,
        }
    }
}

impl LinkLimits {
    fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            models: self.models.min(hard.models),
            imports: self.imports.min(hard.imports),
            clauses: self.clauses.min(hard.clauses),
            nodes: self.nodes.min(hard.nodes),
            depth: self.depth.min(hard.depth),
            model_bytes: self.model_bytes.min(hard.model_bytes),
            total_model_bytes: self.total_model_bytes.min(hard.total_model_bytes),
        }
    }
}

/// A declaration path within one formal requirement owner.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DeclarationKey {
    /// Named record or enumeration type.
    Type(SymbolName),
    /// Field of one named record.
    Field {
        /// Containing record.
        record: SymbolName,
        /// Selected field.
        field: SymbolName,
    },
    /// Input or state value.
    Value(SymbolName),
    /// Variant of one named enumeration.
    Variant {
        /// Containing enumeration.
        enumeration: SymbolName,
        /// Selected variant.
        variant: SymbolName,
    },
    /// Explicit native nominal scalar role.
    Scalar(SymbolName),
    /// Explicit native operation under its object context.
    Operation {
        /// Object context record.
        context: SymbolName,
        /// Operation name.
        name: SymbolName,
    },
}

/// Complete owner-qualified identity of a formal declaration.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DeclarationIdentity {
    /// Exact formal package/requirement/revision owner.
    pub owner: RequirementRef,
    /// Declaration namespace and path under that owner.
    pub key: DeclarationKey,
}

/// Formal declaration identity and its original supplied provenance.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DeclarationLocation {
    /// Owner-qualified identity, distinct from its source position.
    pub identity: DeclarationIdentity,
    /// Original IR declaration source and revision.
    pub source: SourceSpan,
}

/// Target of a resolved native name occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolutionTarget {
    /// A declaration in a selected formal environment.
    Formal(DeclarationLocation),
    /// Binding-name span in this exact ParsedUnit.
    Local(Span),
}

/// Original native occurrence and the declaration it denotes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedOccurrence {
    /// Expression handle, or None for a clause context/operation declaration.
    pub expression: Option<ExprId>,
    /// Exact native token range in the retained source.
    pub span: Span,
    /// Formal or lexical declaration selected by the occurrence.
    pub target: ResolutionTarget,
}

/// One exact native import's formal environment and artifact byte digest.
#[derive(Clone, Copy, Debug)]
pub struct LinkedModel<'a> {
    environment: &'a DeclarationEnvironment,
    digest: ByteDigest,
    native: Option<&'a NativeModel>,
}

impl<'a> LinkedModel<'a> {
    /// The immutable, constructor-validated formal environment.
    pub fn environment(&self) -> &'a DeclarationEnvironment {
        self.environment
    }
    /// Raw SHA-256 of its artifact under the selected binding profile.
    pub fn digest(&self) -> ByteDigest {
        self.digest
    }
    /// Explicit native correspondence; absent for the original formal profile.
    pub fn native_model(&self) -> Option<&'a NativeModel> {
        self.native
    }
}

/// Linked context and name occurrences for one source-ordered clause.
#[derive(Clone, Debug)]
pub struct LinkedClause {
    model: usize,
    context: DeclarationLocation,
    operation: Option<DeclarationLocation>,
    occurrences: Vec<ResolvedOccurrence>,
}

impl LinkedClause {
    /// Selected import index in LinkedPackage::models.
    pub fn model(&self) -> usize {
        self.model
    }
    /// Exact record context identity and formal provenance.
    pub fn context(&self) -> &DeclarationLocation {
        &self.context
    }
    /// Selected explicit operation and its role locus; absent for invariants.
    pub fn operation(&self) -> Option<&DeclarationLocation> {
        self.operation.as_ref()
    }
    /// Resolved occurrences; no static-typing judgment is implied.
    pub fn occurrences(&self) -> &[ResolvedOccurrence] {
        &self.occurrences
    }
}

/// Atomically linked names, retaining the original source and borrowed models.
#[derive(Debug)]
pub struct LinkedPackage<'a> {
    profile: BindingProfile,
    unit: ParsedUnit,
    models: Vec<LinkedModel<'a>>,
    clauses: Vec<LinkedClause>,
}

impl<'a> LinkedPackage<'a> {
    /// Binding interpretation selected by the entry point. Consumers also verify
    /// the actual selected model profiles through the typed boundary.
    pub fn binding_profile(&self) -> &'static str {
        match self.profile {
            BindingProfile::Formal => "native-formal-environment/1",
            BindingProfile::Native => NativeModelProfile::V1.as_str(),
        }
    }
    /// Historical checking validates actual model selections, not a display label.
    pub(crate) fn require_historical_native(&self) -> Result<(), Box<Diagnostic>> {
        match self.profile {
            BindingProfile::Native => native::check_selected_profiles(&self.unit, &self.models),
            BindingProfile::Formal => Err(failure(
                &self.unit,
                Code::UnsupportedConstruct,
                Span { start: 0, end: 0 },
                "checking requires the explicit native model binding profile",
            )),
        }
    }
    /// Exact original syntax/source; cannot be replaced through this API.
    pub fn unit(&self) -> &ParsedUnit {
        &self.unit
    }
    /// Selected formal environments in native import order.
    pub fn models(&self) -> &[LinkedModel<'a>] {
        &self.models
    }
    /// Resolved clauses in native source order.
    pub fn clauses(&self) -> &[LinkedClause] {
        &self.clauses
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BindingProfile {
    Formal,
    Native,
}

fn location(
    environment: &DeclarationEnvironment,
    key: DeclarationKey,
    source: &SourceSpan,
) -> DeclarationLocation {
    DeclarationLocation {
        identity: DeclarationIdentity {
            owner: environment.owner().clone(),
            key,
        },
        source: source.clone(),
    }
}

fn failure(
    unit: &ParsedUnit,
    code: Code,
    span: Span,
    message: impl Into<String>,
) -> Box<Diagnostic> {
    crate::diagnostic::error(
        unit.source(),
        code,
        Phase::Link,
        span.start,
        span.end,
        message,
    )
}

fn ambiguous(
    unit: &ParsedUnit,
    span: Span,
    mut related: Vec<DeclarationLocation>,
) -> Box<Diagnostic> {
    related.sort();
    let mut error = failure(
        unit,
        Code::AmbiguousDeclaration,
        span,
        "multiple formal declarations match",
    );
    error.related = related;
    error
}

/// Link native names using the native-formal-environment/1 binding profile.
///
/// # Errors
/// Returns the existing located native diagnostic for missing, stale, ambiguous,
/// malformed or unsupported bindings, or incomplete work at a selected limit.
/// A failure exposes no partial package. Success does not establish types,
/// guarded definedness, observation availability or execution support.
pub fn link(
    unit: ParsedUnit,
    environments: &[DeclarationEnvironment],
    limits: LinkLimits,
) -> Result<LinkedPackage<'_>, Box<Diagnostic>> {
    let limits = limits.bounded();
    preflight(&unit, environments.len(), limits)?;
    let catalog = formal_catalog(&unit, environments, limits)?;
    let models = select_models(&unit, &catalog)?;
    let clauses = resolve_clauses(&unit, &models, limits, BindingProfile::Formal)?;
    Ok(LinkedPackage {
        profile: BindingProfile::Formal,
        unit,
        models,
        clauses,
    })
}

/// Link using exact native-state-model/1 artifacts and explicit native roles.
///
/// # Errors
/// Refuses conflicting inventory identity, missing/stale/ambiguous selections,
/// unbound native references or operations, and exhausted limits atomically.
/// Successful linkage does not establish types, definedness or runtime validity.
pub fn link_native(
    unit: ParsedUnit,
    models: &[NativeModel],
    limits: LinkLimits,
) -> Result<LinkedPackage<'_>, Box<Diagnostic>> {
    let limits = limits.bounded();
    preflight(&unit, models.len(), limits)?;
    let catalog = native::catalog(&unit, models, limits)?;
    let models = select_models(&unit, &catalog)?;
    native::check_selected_profiles(&unit, &models)?;
    let clauses = resolve_clauses(&unit, &models, limits, BindingProfile::Native)?;
    Ok(LinkedPackage {
        profile: BindingProfile::Native,
        unit,
        models,
        clauses,
    })
}

fn preflight(
    unit: &ParsedUnit,
    model_count: usize,
    limits: LinkLimits,
) -> Result<(), Box<Diagnostic>> {
    for (count, limit, dimension) in [
        (model_count, limits.models, "formal environments"),
        (unit.imports().len(), limits.imports, "native imports"),
        (unit.clauses().len(), limits.clauses, "native clauses"),
        (unit.expressions().len(), limits.nodes, "native nodes"),
    ] {
        if count > limit {
            return Err(failure(
                unit,
                Code::ResourceExhausted,
                Span { start: 0, end: 0 },
                format!("{dimension} limit exceeded"),
            ));
        }
    }
    Ok(())
}

fn formal_catalog<'a>(
    unit: &ParsedUnit,
    environments: &'a [DeclarationEnvironment],
    limits: LinkLimits,
) -> Result<Vec<LinkedModel<'a>>, Box<Diagnostic>> {
    let mut catalog = Vec::with_capacity(environments.len());
    let mut emitted = 0;
    for environment in environments {
        let remaining = limits.total_model_bytes - emitted;
        let maximum =
            u64::try_from(limits.model_bytes.min(remaining)).expect("bounded byte ceiling");
        let output = environment
            .canonical_declaration_with_limit(CanonicalProfile::V1, maximum)
            .map_err(|upstream| {
                let code = if upstream.code
                    == quire_contract_ir::DiagnosticCode::CanonicalizationResourceExhausted
                {
                    Code::ResourceExhausted
                } else {
                    Code::InvalidModelBinding
                };
                let mut error = failure(
                    unit,
                    code,
                    Span { start: 0, end: 0 },
                    "formal declaration canonicalization failed",
                );
                error.upstream = Some(Box::new(upstream));
                error
            })?;
        emitted += output.bytes().as_slice().len();
        catalog.push(LinkedModel {
            environment,
            digest: ByteDigest::of(output.bytes().as_slice()),
            native: None,
        });
    }
    Ok(catalog)
}

// Success returns exactly one model for each import, in the authored order.
// Profile checks and occurrence resolution retain that positional correspondence.
fn select_models<'a>(
    unit: &ParsedUnit,
    catalog: &[LinkedModel<'a>],
) -> Result<Vec<LinkedModel<'a>>, Box<Diagnostic>> {
    let mut models = Vec::with_capacity(unit.imports().len());
    for import in unit.imports() {
        let expected: ByteDigest = import.digest.value.parse().map_err(|_| {
            failure(
                unit,
                Code::InvalidModelBinding,
                import.digest.span,
                "expected a sha256 byte digest",
            )
        })?;
        let same_package: Vec<_> = catalog
            .iter()
            .filter(|model| model.environment.owner().package().as_str() == import.package.value)
            .collect();
        if same_package.is_empty() {
            return Err(failure(
                unit,
                Code::MissingImport,
                import.package.span,
                "selected package is absent",
            ));
        }
        let exact: Vec<_> = same_package
            .into_iter()
            .filter(|model| {
                model.environment.owner().revision().get().to_string() == import.version.value
                    && model.digest == expected
            })
            .collect();
        match exact.as_slice() {
            [] => {
                return Err(failure(
                    unit,
                    Code::StaleDependency,
                    import.span,
                    format!("package {} revision {} digest {expected} has no exact formal declaration artifact", import.package.value, import.version.value),
                ))
            }
            [model] => models.push(**model),
            _ => {
                return Err(ambiguous(
                    unit,
                    import.span,
                    exact
                        .iter()
                        .flat_map(|model| {
                            model.environment.types().iter().map(|declaration| {
                                location(
                                    model.environment,
                                    DeclarationKey::Type(declaration.name().clone()),
                                    declaration.source(),
                                )
                            })
                        })
                        .collect(),
                ))
            }
        }
    }
    Ok(models)
}

fn resolve_clauses(
    unit: &ParsedUnit,
    models: &[LinkedModel<'_>],
    limits: LinkLimits,
    profile: BindingProfile,
) -> Result<Vec<LinkedClause>, Box<Diagnostic>> {
    let mut names = BTreeSet::new();
    let mut clauses = Vec::with_capacity(unit.clauses().len());
    for clause in unit.clauses() {
        if !names.insert(&clause.name.value) {
            return Err(failure(
                unit,
                Code::InvalidModelBinding,
                clause.name.span,
                "duplicate native clause name",
            ));
        }
        if profile == BindingProfile::Formal && clause.kind != ClauseKind::Invariant {
            return Err(failure(
                unit,
                Code::UnsupportedConstruct,
                clause.span,
                "operation clauses need an explicit native operation mapping",
            ));
        }
        let (model, record) = resolve_context(unit, models, clause, profile)?;
        let environment = models[model].environment;
        let operation = models[model]
            .native
            .map(|model| native::operation(unit, model, clause))
            .transpose()?
            .flatten();
        let context = location(
            environment,
            DeclarationKey::Type(record.name().clone()),
            record.source(),
        );
        let operation_location = operation.map(|operation| {
            location(
                environment,
                DeclarationKey::Operation {
                    context: operation.context.clone(),
                    name: operation.name.clone(),
                },
                &operation.source,
            )
        });
        let mut resolver = Resolver {
            unit,
            models,
            current: model,
            context: record,
            operation,
            limits,
            locals: Vec::new(),
            occurrences: Vec::new(),
        };
        resolver.occurrences.push(ResolvedOccurrence {
            expression: None,
            span: clause.context.span,
            target: ResolutionTarget::Formal(context.clone()),
        });
        if let (Some(name), Some(location)) = (&clause.operation, &operation_location) {
            resolver.occurrences.push(ResolvedOccurrence {
                expression: None,
                span: name.span,
                target: ResolutionTarget::Formal(location.clone()),
            });
        }
        resolver.visit(clause.expression, 0)?;
        clauses.push(LinkedClause {
            model,
            context,
            operation: operation_location,
            occurrences: resolver.occurrences,
        });
    }
    Ok(clauses)
}

fn resolve_context<'a>(
    unit: &ParsedUnit,
    models: &[LinkedModel<'a>],
    clause: &Clause,
    profile: BindingProfile,
) -> Result<(usize, &'a RecordDeclaration), Box<Diagnostic>> {
    let (model, declaration) = exported(
        unit,
        models,
        &clause.model.value,
        &clause.context.value,
        clause.context.span,
    )?;
    let TypeDeclaration::Record {
        declaration: record,
    } = declaration
    else {
        return Err(failure(
            unit,
            Code::InvalidModelBinding,
            clause.context.span,
            "native context must select a record",
        ));
    };
    match profile {
        BindingProfile::Native => {
            if !models[model].native.is_some_and(|native| {
                native
                    .roles()
                    .objects
                    .iter()
                    .any(|object| &object.record == record.name())
            }) {
                return Err(failure(
                    unit,
                    Code::InvalidModelBinding,
                    clause.context.span,
                    "native context requires an explicit object role",
                ));
            }
        }
        BindingProfile::Formal => {
            let self_value = models[model]
                .environment
                .values()
                .iter()
                .find(|value| value.name().as_str() == "self");
            if !self_value.is_some_and(|value| {
                value.kind() == ValueDeclarationKind::State
                    && matches!(value.value_type(), ValueType::Record { name } if name == record.name())
            }) {
                return Err(failure(unit, Code::InvalidModelBinding, clause.context.span, "context requires an explicit self State value of its record type"));
            }
        }
    }
    Ok((model, record))
}

#[derive(Clone, Copy)]
enum Shape<'a> {
    Formal(&'a DeclarationEnvironment, &'a ValueType),
    Object(&'a DeclarationEnvironment, &'a SymbolName),
    Reference(&'a DeclarationEnvironment, &'a ObjectRole),
    Boolean,
    Integer,
    Text,
    Unknown,
}

impl Shape<'_> {
    fn same(self, other: Self) -> bool {
        match self {
            Self::Formal(a, ta) => {
                matches!(other, Self::Formal(b, tb) if std::ptr::eq(a, b) && ta == tb)
            }
            Self::Object(a, name) => {
                matches!(other, Self::Object(b, other_name) if std::ptr::eq(a, b) && name == other_name)
            }
            Self::Reference(a, role) => {
                matches!(other, Self::Reference(b, other_role) if std::ptr::eq(a, b) && role.record == other_role.record)
            }
            Self::Boolean => matches!(other, Self::Boolean),
            Self::Integer => matches!(other, Self::Integer),
            Self::Text => matches!(other, Self::Text),
            Self::Unknown => false,
        }
    }
}

struct Resolver<'u, 'a> {
    unit: &'u ParsedUnit,
    models: &'u [LinkedModel<'a>],
    current: usize,
    context: &'a RecordDeclaration,
    operation: Option<&'a OperationRole>,
    limits: LinkLimits,
    locals: Vec<(&'u str, Span, Shape<'a>)>,
    occurrences: Vec<ResolvedOccurrence>,
}

fn exported<'a>(
    unit: &ParsedUnit,
    models: &[LinkedModel<'a>],
    alias: &str,
    name: &str,
    span: Span,
) -> Result<(usize, &'a TypeDeclaration), Box<Diagnostic>> {
    let mut candidates = Vec::new();
    let mut alias_exists = false;
    for (index, (import, model)) in unit.imports().iter().zip(models).enumerate() {
        if import.alias.value == alias {
            alias_exists = true;
            for declaration in model.environment.types() {
                if declaration.name().as_str() == name {
                    candidates.push((index, declaration));
                }
            }
        }
    }
    match candidates.as_slice() {
        [] => Err(failure(
            unit,
            if alias_exists {
                Code::MissingDeclaration
            } else {
                Code::MissingImport
            },
            span,
            "selected formal export is absent",
        )),
        [candidate] => Ok(*candidate),
        _ => Err(ambiguous(
            unit,
            span,
            candidates
                .iter()
                .map(|(index, declaration)| {
                    location(
                        models[*index].environment,
                        DeclarationKey::Type(declaration.name().clone()),
                        declaration.source(),
                    )
                })
                .collect(),
        )),
    }
}

impl<'u, 'a> Resolver<'u, 'a> {
    fn occurrence(&mut self, id: ExprId, span: Span, target: ResolutionTarget) {
        self.occurrences.push(ResolvedOccurrence {
            expression: Some(id),
            span,
            target,
        });
    }

    fn lookup_value(
        &self,
        name: &str,
        span: Span,
    ) -> Result<&'a quire_contract_ir::ValueDeclaration, Box<Diagnostic>> {
        self.models[self.current]
            .environment
            .values()
            .iter()
            .find(|value| value.name().as_str() == name)
            .ok_or_else(|| {
                failure(
                    self.unit,
                    Code::MissingDeclaration,
                    span,
                    format!("value {name} is absent"),
                )
            })
    }

    fn bind_value(
        &mut self,
        id: ExprId,
        span: Span,
        value: &'a quire_contract_ir::ValueDeclaration,
    ) -> Shape<'a> {
        let environment = self.models[self.current].environment;
        self.occurrence(
            id,
            span,
            ResolutionTarget::Formal(location(
                environment,
                DeclarationKey::Value(value.name().clone()),
                value.source(),
            )),
        );
        self.shape(environment, value.value_type())
    }

    fn value(&mut self, id: ExprId, name: &str, span: Span) -> Result<Shape<'a>, Box<Diagnostic>> {
        let value = self.lookup_value(name, span)?;
        self.check_input_scope(value, span)?;
        Ok(self.bind_value(id, span, value))
    }

    fn field(
        &mut self,
        id: ExprId,
        name: &crate::Spanned<String>,
        shape: Shape<'a>,
    ) -> Result<Shape<'a>, Box<Diagnostic>> {
        let (environment, record_name) = match shape {
            Shape::Formal(environment, ValueType::Record { name })
            | Shape::Object(environment, name) => (environment, name),
            Shape::Reference(..) => {
                return Err(failure(
                    self.unit,
                    Code::UnsupportedConstruct,
                    name.span,
                    "reference carrier fields are not native field-access syntax",
                ))
            }
            Shape::Formal(..) | Shape::Boolean | Shape::Integer | Shape::Text | Shape::Unknown => {
                return Err(failure(
                    self.unit,
                    Code::MissingDeclaration,
                    name.span,
                    "field receiver has no unambiguous formal record shape",
                ))
            }
        };
        let field = environment
            .types()
            .iter()
            .find_map(|declaration| match declaration {
                TypeDeclaration::Record { declaration } if declaration.name() == record_name => {
                    declaration
                        .fields()
                        .iter()
                        .find(|field| field.name().as_str() == name.value)
                }
                TypeDeclaration::Enum { .. } | TypeDeclaration::Record { .. } => None,
            })
            .ok_or_else(|| {
                failure(
                    self.unit,
                    Code::MissingDeclaration,
                    name.span,
                    "record field is absent",
                )
            })?;
        self.occurrence(
            id,
            name.span,
            ResolutionTarget::Formal(location(
                environment,
                DeclarationKey::Field {
                    record: record_name.clone(),
                    field: field.name().clone(),
                },
                field.source(),
            )),
        );
        Ok(self.shape(environment, field.value_type()))
    }

    fn visit(&mut self, id: ExprId, depth: usize) -> Result<Shape<'a>, Box<Diagnostic>> {
        let unit = self.unit;
        let expression = unit
            .expression(id)
            .expect("ParsedUnit contains its expression handles");
        if depth >= self.limits.depth {
            return Err(failure(
                unit,
                Code::ResourceExhausted,
                expression.span,
                "link traversal depth limit exceeded",
            ));
        }
        let next = depth + 1;
        match &expression.kind {
            ExprKind::Group { inner } => self.visit(*inner, next),
            ExprKind::Boolean(_) => Ok(Shape::Boolean),
            ExprKind::Integer(_) => Ok(Shape::Integer),
            ExprKind::Text(_) => Ok(Shape::Text),
            ExprKind::SelfValue => self.context_value(id, expression.span),
            ExprKind::ResultValue => self.result_value(id, expression.span),
            ExprKind::Name(name) => {
                if let Some((_, declaration, shape)) = self
                    .locals
                    .iter()
                    .rev()
                    .find(|(candidate, _, _)| *candidate == name.value)
                    .copied()
                {
                    self.occurrence(id, name.span, ResolutionTarget::Local(declaration));
                    Ok(shape)
                } else {
                    self.value(id, &name.value, name.span)
                }
            }
            ExprKind::EnumValue {
                model,
                name,
                variant,
            } => {
                let (index, declaration) =
                    exported(unit, self.models, &model.value, &name.value, name.span)?;
                let TypeDeclaration::Enum { declaration } = declaration else {
                    return Err(failure(
                        unit,
                        Code::MissingDeclaration,
                        name.span,
                        "selected export is not an enumeration",
                    ));
                };
                let selected = declaration
                    .variants()
                    .iter()
                    .find(|item| item.name().as_str() == variant.value)
                    .ok_or_else(|| {
                        failure(
                            unit,
                            Code::MissingDeclaration,
                            variant.span,
                            "enumeration variant is absent",
                        )
                    })?;
                self.occurrence(
                    id,
                    variant.span,
                    ResolutionTarget::Formal(location(
                        self.models[index].environment,
                        DeclarationKey::Variant {
                            enumeration: declaration.name().clone(),
                            variant: selected.name().clone(),
                        },
                        selected.source(),
                    )),
                );
                Ok(Shape::Unknown)
            }
            ExprKind::Field { base, name } => {
                let shape = self.visit(*base, next)?;
                self.field(id, name, shape)
            }
            ExprKind::Unary { op, argument } => {
                self.visit(*argument, next)?;
                Ok(match op {
                    UnaryOp::Not => Shape::Boolean,
                    UnaryOp::Negate => Shape::Unknown,
                })
            }
            ExprKind::Binary { op, left, right } => {
                self.visit(*left, next)?;
                self.visit(*right, next)?;
                match op {
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder => Ok(Shape::Unknown),
                    BinaryOp::Implies
                    | BinaryOp::Or
                    | BinaryOp::And
                    | BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => Ok(Shape::Boolean),
                }
            }
            ExprKind::Call { builtin, argument } => match builtin {
                Builtin::Deref => self.dereference(id, *argument, expression.span, next),
                Builtin::Present => {
                    self.visit(*argument, next)?;
                    Ok(Shape::Boolean)
                }
                Builtin::Size => {
                    self.visit(*argument, next)?;
                    Ok(Shape::Integer)
                }
                Builtin::Pre => self.visit(*argument, next),
                Builtin::Value => Ok(match self.visit(*argument, next)? {
                    Shape::Formal(environment, ValueType::Option { value }) => {
                        self.shape(environment, value)
                    }
                    Shape::Formal(..)
                    | Shape::Object(..)
                    | Shape::Reference(..)
                    | Shape::Boolean
                    | Shape::Integer
                    | Shape::Text
                    | Shape::Unknown => Shape::Unknown,
                }),
            },
            ExprKind::Let { name, value, body } => {
                let shape = self.visit(*value, next)?;
                self.locals.push((&name.value, name.span, shape));
                let result = self.visit(*body, next);
                self.locals.pop();
                result
            }
            ExprKind::If {
                condition,
                then_value,
                else_value,
            } => {
                self.visit(*condition, next)?;
                let then_shape = self.visit(*then_value, next)?;
                let else_shape = self.visit(*else_value, next)?;
                Ok(if then_shape.same(else_shape) {
                    then_shape
                } else {
                    Shape::Unknown
                })
            }
            ExprKind::Quantifier {
                name,
                domain,
                predicate,
                ..
            } => {
                let shape = match self.visit(*domain, next)? {
                    Shape::Formal(environment, ValueType::Collection { value }) => {
                        self.shape(environment, value.element())
                    }
                    Shape::Formal(..)
                    | Shape::Object(..)
                    | Shape::Reference(..)
                    | Shape::Boolean
                    | Shape::Integer
                    | Shape::Text
                    | Shape::Unknown => Shape::Unknown,
                };
                self.locals.push((&name.value, name.span, shape));
                let result = self.visit(*predicate, next);
                self.locals.pop();
                result?;
                Ok(Shape::Boolean)
            }
            ExprKind::Reaches {
                start,
                target,
                field,
            } => self.reaches(id, *start, *target, field, expression.span, next),
        }
    }
}

#[cfg(test)]
mod profile_tests {
    use super::*;
    use crate::checking::{check, CheckBindings, CheckLimits};
    use crate::formal_source::FormalSource;
    use crate::native_model::ModelLimits;
    use crate::runtime_test_setup::native_rule_model;
    use crate::{parse, Limits, SourceIdentity};
    use ix_trace_rs::trace;
    use quire_contract_ir as ir;

    #[test]
    #[trace("TC-120", "FR-041-AC-4")]
    fn historical_checker_rechecks_actual_profiles_after_a_linker_invariant_bypass() {
        let parts = native_rule_model::parts();
        let model = NativeModel::new_with_profile(
            NativeModelProfile::V2,
            parts.source,
            parts.environment,
            parts.roles,
            ModelLimits::default(),
        )
        .unwrap();
        let text = format!(
            "language \"ix:native\" edition \"0-draft\"; profile \"state-finite/0-draft\"; model M = \"example/rule-tests\" version \"1\" digest \"{}\"; invariant Rule on M::Node at current {{ true }}",
            model.digest()
        );
        let unit = parse(
            SourceIdentity {
                identity: "historical-profile-check".into(),
                revision: "1".into(),
            },
            "historical.native",
            text.as_bytes(),
            Limits::default(),
        )
        .unwrap();
        let import = unit.imports()[0].span;
        let bindings = CheckBindings {
            source: FormalSource::new(
                unit.source().clone(),
                ir::SourceIdentity::new(
                    ir::SourceDocumentId::new("ProfileCheck").unwrap(),
                    ir::SourceRevision::new(1).unwrap(),
                ),
            ),
            clauses: Vec::new(),
        };
        let selected = LinkedModel {
            environment: model.environment(),
            digest: model.digest(),
            native: Some(&model),
        };
        for (models, expected, at) in [
            (vec![selected], Code::UnsupportedConstruct, import),
            (
                Vec::new(),
                Code::InvalidModelBinding,
                Span { start: 0, end: 0 },
            ),
            (
                vec![selected, selected],
                Code::InvalidModelBinding,
                Span { start: 0, end: 0 },
            ),
            (
                vec![LinkedModel {
                    native: None,
                    ..selected
                }],
                Code::InvalidModelBinding,
                import,
            ),
        ] {
            // Only this private invariant control bypasses normal link_native.
            // Its historical label cannot authorize the actual /2 selection or
            // let a malformed correspondence reach clause/type/proof processing.
            let linked = LinkedPackage {
                profile: BindingProfile::Native,
                unit: unit.clone(),
                models,
                clauses: Vec::new(),
            };
            assert_eq!(linked.binding_profile(), NativeModelProfile::V1.as_str());
            let error = check(linked, bindings.clone(), CheckLimits::default()).unwrap_err();
            assert_eq!(error.code, expected);
            assert_eq!(error.phase, Phase::Check);
            assert_eq!(
                (error.span.start.byte, error.span.end.byte),
                (at.start, at.end)
            );
        }
    }
}
