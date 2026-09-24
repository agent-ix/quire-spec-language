// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FR-091 forms assembler, the entry of E3 (ADR-011 §2.1 E3 owner: QSL
//! `check`): it turns the S2 parsed unit of one source into the
//! [`PackageDeclarations`] value [`PackageDeclarations::check`] consumes.
//!
//! It reads the parsed unit only: no `qsl_cst` type, no source text and no
//! token text (ADR-011 §3 FB-01, FR-091-AC-20). It resolves every type form
//! of the unit to a kernel `ValueType` (the E3 name-binding phase, ADR-011
//! §1; ADR-013 O-11), each `using` alias to one of the unit's profile
//! selections, and gives each declared record and tuple the handle its
//! `ValueType::Composite` names, minted here over the unit's
//! `SourceOwner` (FB-13: only `check` mints). A refusal carries every
//! error found, each with its typed cause and the span it concerns, and no
//! partial output (ADR-011 §2.3 E3).
//!
//! Enum, dimension, unit and predicate declarations have no S2 production
//! yet (FR-091-AC-6), so no enum or unit reaches here.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{
    AliasForm, BuiltinType, DeclarationForm, DeclaredName, Expression, FunctionDeclaration,
    ParsedUnit, RecordFieldForm, TypeForm, TypeFormHead,
};
use qsl_foundation::diagnostic::CatalogCode;
use qsl_foundation::source::provenance::RawSourceRef;
use qsl_foundation::Span;
use quire_exact::{EffectiveId, IeeeWidth, Presence, RoundingMode, ValueType};

use super::check::{PackageDeclarations, ResolvedSignature};
use super::lowering::strongly_connected;
use super::node_key::{declared_type_handle, NodeKeyRefusal, SourceOwner};
use super::type_form::{
    parse_rounding_mode, resolve_form, TypeFormError, TypeFormFault, TypeNames,
};
use crate::value::declaration::{
    CompositeDeclaration, CompositeShape, DeclarationCause, FieldDeclaration, InvalidDeclaration,
    TypeEnvironment,
};

/// Why the assembler refused one part of a unit (FR-091 "The assembler
/// refuses in these cases").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssemblyCause {
    /// A qualified type name names no declaration of the unit and nothing
    /// in an admitted domain package.
    UnresolvedTypeName {
        /// The name as written.
        name: String,
    },
    /// A qualified type name names more than one declaration of the unit
    /// (ADR-013 O-11).
    AmbiguousTypeName {
        /// The name as written.
        name: String,
        /// The span of each candidate declaration's name.
        candidates: Vec<Span>,
    },
    /// A built-in constructor's declared bounds are rejected by its value
    /// type, with that value type's own cause.
    IllFormedBounds(TypeFormFault),
    /// A floating type, which the producer does not represent: its rounding
    /// mode is part of the type, and `ValueType::Float` would drop it
    /// (ADR-013 R-07).
    FloatingType {
        /// The width.
        width: IeeeWidth,
        /// The rounding mode, `exact` when none is written.
        rounding: RoundingMode,
        /// The profile selection the declaration's `using` alias names,
        /// when the type is in a function's signature.
        profile: Option<String>,
    },
    /// An alias's resolution reaches that alias again.
    AliasCycle {
        /// The cycle's dependency edges, `(alias, alias it names)`.
        edges: Vec<(String, String)>,
    },
    /// A `using` alias names no profile selection of the unit.
    UndeclaredAlias {
        /// The alias as written.
        alias: String,
    },
    /// The unit declares one selection alias more than once.
    DuplicateAlias {
        /// The alias.
        alias: String,
        /// The span of each selection declaring it.
        spans: Vec<Span>,
    },
    /// The unit's records and tuples are not an admitted declaration set
    /// (FR-143): a duplicate field name, an ill-typed member, or a
    /// recursion the rule refuses.
    InvalidTypeDeclaration(InvalidDeclaration),
    /// A declared type's handle could not be encoded: a broken invariant,
    /// never a property of the source.
    Handle(NodeKeyRefusal),
}

impl AssemblyCause {
    /// This cause's catalog code (FR-091 "Catalog codes", ADR-013 O-17).
    pub fn catalog_code(&self) -> CatalogCode {
        match self {
            Self::UnresolvedTypeName { .. } => {
                CatalogCode::new("missing_declaration", "missing-name")
            }
            Self::AmbiguousTypeName { .. } | Self::DuplicateAlias { .. } => {
                CatalogCode::new("ambiguous_declaration", "ambiguous-name")
            }
            Self::IllFormedBounds(_) => CatalogCode::new("ill_typed", "type-mismatch"),
            Self::FloatingType { .. } => {
                CatalogCode::new("unknown_required_feature", "unsupported-feature")
            }
            Self::AliasCycle { .. } => CatalogCode::new("invalid_package", "definition-cycle"),
            Self::UndeclaredAlias { .. } => {
                CatalogCode::new("missing_declaration", "missing-selection")
            }
            Self::InvalidTypeDeclaration(invalid) => match &invalid.cause {
                DeclarationCause::DuplicateMember(_) => {
                    CatalogCode::new("ambiguous_declaration", "ambiguous-name")
                }
                DeclarationCause::Type(cause) => {
                    CatalogCode::new("ill_typed", cause.tag().unwrap_or("type-mismatch"))
                }
                DeclarationCause::Recursion { .. }
                | DeclarationCause::GeneralizationCycle { .. } => {
                    CatalogCode::new("ill_typed", "type-mismatch")
                }
                DeclarationCause::DuplicateKey
                | DeclarationCause::UnknownDeclaration(_)
                | DeclarationCause::UnknownObjectType(_) => {
                    CatalogCode::new("runtime_invariant", "established-invariant-broken")
                }
            },
            Self::Handle(_) => {
                CatalogCode::new("runtime_invariant", "established-invariant-broken")
            }
        }
    }
}

/// One assembler error: its cause and the span it concerns, a
/// `Locus::Region` over the unit (ADR-013 T-5): the assembler runs before
/// check mints any occurrence key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssemblyError {
    /// The typed cause.
    pub cause: AssemblyCause,
    /// The span of the type form, `using` field, selection or declared
    /// name it concerns.
    pub span: Span,
}

/// The assembler's refusal: every error found in the unit, in the order
/// found (ADR-011 §2.3 E3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssemblyRefusal {
    /// Every error, never empty.
    pub errors: Vec<AssemblyError>,
}

/// A declared record's or tuple's members, as written.
enum Members {
    Record(Vec<RecordFieldForm>),
    Tuple(Vec<TypeForm>),
}

/// A declared record or tuple.
struct Composite {
    name: DeclaredName,
    members: Members,
}

/// Which declaration a declared type name binds.
#[derive(Clone, Copy)]
enum Declared {
    Alias(usize),
    Composite,
}

/// The unit's declarations, split by kind, and the names they bind.
struct Unit {
    aliases: Vec<AliasForm>,
    composites: Vec<Composite>,
    functions: Vec<FunctionDeclaration>,
    /// Each declared type name's declarations, with the span of each name.
    declared: BTreeMap<String, Vec<(Declared, Span)>>,
}

impl Unit {
    fn new(forms: Vec<qsl_forms::ParsedForm>) -> Self {
        let mut unit = Self {
            aliases: Vec::new(),
            composites: Vec::new(),
            functions: Vec::new(),
            declared: BTreeMap::new(),
        };
        for form in forms {
            match form.into_form() {
                DeclarationForm::Function(function) => unit.functions.push(*function),
                DeclarationForm::Alias(alias) => {
                    unit.declare(&alias.name, Declared::Alias(unit.aliases.len()));
                    unit.aliases.push(alias);
                }
                DeclarationForm::Record(record) => {
                    unit.declare(&record.name, Declared::Composite);
                    unit.composites.push(Composite {
                        name: record.name,
                        members: Members::Record(record.fields),
                    });
                }
                DeclarationForm::Tuple(tuple) => {
                    unit.declare(&tuple.name, Declared::Composite);
                    unit.composites.push(Composite {
                        name: tuple.name,
                        members: Members::Tuple(tuple.elements),
                    });
                }
            }
        }
        unit
    }

    fn declare(&mut self, name: &DeclaredName, declared: Declared) {
        self.declared
            .entry(name.name.clone())
            .or_default()
            .push((declared, name.span));
    }

    /// The spans of every declaration `name` binds.
    fn candidates(&self, name: &str) -> Vec<Span> {
        self.declared
            .get(name)
            .map(|declared| declared.iter().map(|(_, span)| *span).collect())
            .unwrap_or_default()
    }
}

/// The names the assembler resolves type forms against while it builds the
/// package: each declared record's and tuple's `ValueType::Composite`, and
/// each alias once resolved. The unit admits no domain package yet, so no
/// name is an object type.
#[derive(Default)]
struct Names {
    types: BTreeMap<String, Vec<ValueType>>,
}

impl TypeNames for Names {
    fn named_types(&self, name: &str) -> &[ValueType] {
        self.types.get(name).map_or(&[], Vec::as_slice)
    }

    fn object_type_named(&self, _name: &str) -> Option<EffectiveId> {
        None
    }
}

/// A function signature's type forms: its parameters' and its result's,
/// in source order.
fn signature_type_forms(function: &FunctionDeclaration) -> Vec<&TypeForm> {
    function
        .parameters
        .iter()
        .map(|(_, form)| form)
        .chain(std::iter::once(&function.result))
        .collect()
}

/// The type forms inside a function's `decreases` measure and body, in
/// source order: `convert<T>` and `allInstances<T>` targets, and the named
/// types of `fold<A>`, `reduce<A>`, `count<N>` and `sum<N>` as name forms
/// over the name's own span.
fn body_type_forms(function: &FunctionDeclaration) -> Vec<TypeForm> {
    let mut forms = Vec::new();
    let roots = [function.measure.as_ref(), Some(&function.body)];
    for root in roots.into_iter().flatten() {
        let mut stack = vec![root];
        while let Some(expression) = stack.pop() {
            match expression {
                Expression::Convert { target, .. } | Expression::AllInstances { target, .. } => {
                    forms.push(target.clone());
                }
                Expression::Accumulate {
                    accumulator_type: name,
                    accumulator_type_span: span,
                    ..
                }
                | Expression::Count {
                    result_type: name,
                    result_type_span: span,
                    ..
                }
                | Expression::Sum {
                    result_type: name,
                    result_type_span: span,
                    ..
                } => forms.push(TypeForm::name(name.clone(), *span)),
                Expression::Boolean(_)
                | Expression::Integer(_)
                | Expression::Rational(..)
                | Expression::Name(_)
                | Expression::Let { .. }
                | Expression::If { .. }
                | Expression::Binary { .. }
                | Expression::Negate(_)
                | Expression::Not(_)
                | Expression::Field { .. }
                | Expression::Present(_)
                | Expression::Value(_)
                | Expression::Deref(_)
                | Expression::Call { .. }
                | Expression::Record { .. }
                | Expression::Collection { .. }
                | Expression::Query { .. }
                | Expression::Flatten(_)
                | Expression::Size(_)
                | Expression::Contains { .. }
                | Expression::Lookup { .. }
                | Expression::Dispatch { .. }
                | Expression::Pre(_) => {}
            }
            // Children last-first, so the first child is visited next and
            // forms come out in source order.
            stack.extend(expression.children().into_iter().rev());
        }
    }
    forms
}

/// The declared-name check of one type form tree (FR-091 "The assembler
/// refuses"): every name must bind exactly one declaration of the unit, a
/// `Reference<Q>` target must name an object type of an admitted domain
/// package (the unit admits none yet), and a floating type is refused.
fn check_names(
    unit: &Unit,
    form: &TypeForm,
    profile: Option<&str>,
    errors: &mut Vec<AssemblyError>,
) {
    let mut stack = vec![form];
    while let Some(form) = stack.pop() {
        match &form.head {
            TypeFormHead::Builtin(BuiltinType::Float32 | BuiltinType::Float64) => {
                let width = if matches!(form.head, TypeFormHead::Builtin(BuiltinType::Float32)) {
                    IeeeWidth::Binary32
                } else {
                    IeeeWidth::Binary64
                };
                let rounding = match form.bounds.first() {
                    Some(spelled) => match parse_rounding_mode(spelled) {
                        Some(rounding) => rounding,
                        None => {
                            errors.push(AssemblyError {
                                cause: AssemblyCause::IllFormedBounds(TypeFormFault::Malformed),
                                span: form.span,
                            });
                            continue;
                        }
                    },
                    None => RoundingMode::Exact,
                };
                errors.push(AssemblyError {
                    cause: AssemblyCause::FloatingType {
                        width,
                        rounding,
                        profile: profile.map(str::to_owned),
                    },
                    span: form.span,
                });
            }
            TypeFormHead::Builtin(BuiltinType::Reference) => {
                for target in &form.arguments {
                    let name = match &target.head {
                        TypeFormHead::Name(name) => name.clone(),
                        TypeFormHead::Builtin(_)
                        | TypeFormHead::Collection(_)
                        | TypeFormHead::Population => continue,
                    };
                    errors.push(AssemblyError {
                        cause: AssemblyCause::UnresolvedTypeName { name },
                        span: target.span,
                    });
                }
            }
            TypeFormHead::Name(name) => match unit.declared.get(name).map(Vec::len) {
                None | Some(0) => errors.push(AssemblyError {
                    cause: AssemblyCause::UnresolvedTypeName { name: name.clone() },
                    span: form.span,
                }),
                Some(1) => {}
                Some(_) => errors.push(AssemblyError {
                    cause: AssemblyCause::AmbiguousTypeName {
                        name: name.clone(),
                        candidates: unit.candidates(name),
                    },
                    span: form.span,
                }),
            },
            TypeFormHead::Builtin(
                BuiltinType::Boolean
                | BuiltinType::Integer
                | BuiltinType::Int
                | BuiltinType::Rational
                | BuiltinType::Decimal
                | BuiltinType::Text
                | BuiltinType::Option,
            )
            | TypeFormHead::Collection(_)
            | TypeFormHead::Population => stack.extend(&form.arguments),
        }
    }
}

/// The aliases one type form names, by index into the unit's aliases.
fn named_aliases(unit: &Unit, form: &TypeForm) -> Vec<usize> {
    let mut named = Vec::new();
    let mut stack = vec![form];
    while let Some(form) = stack.pop() {
        if let TypeFormHead::Name(name) = &form.head {
            if let Some([(Declared::Alias(index), _)]) = unit.declared.get(name).map(Vec::as_slice)
            {
                named.push(*index);
            }
        }
        stack.extend(&form.arguments);
    }
    named
}

/// A resolution error as an assembler error.
fn resolution_error(unit: &Unit, error: TypeFormError) -> AssemblyError {
    let cause = match error.fault {
        TypeFormFault::MissingName(name) => AssemblyCause::UnresolvedTypeName { name },
        TypeFormFault::AmbiguousName(name) => AssemblyCause::AmbiguousTypeName {
            candidates: unit.candidates(&name),
            name,
        },
        fault @ (TypeFormFault::Malformed
        | TypeFormFault::EmptyInterval
        | TypeFormFault::DenominatorBelowOne
        | TypeFormFault::MalformedDecimal
        | TypeFormFault::EmptyTextBounds
        | TypeFormFault::EmptyCardinality) => AssemblyCause::IllFormedBounds(fault),
    };
    AssemblyError {
        cause,
        span: error.span,
    }
}

fn refuse<T>(errors: Vec<AssemblyError>) -> Result<T, AssemblyRefusal> {
    Err(AssemblyRefusal { errors })
}

/// The unit's records and tuples as one admitted declaration set (FR-143),
/// or every refusal: each refused declaration is set aside and the rest
/// admitted again, so one declaration's refusal does not hide another's.
/// A declaration refused only because it names one set aside
/// (`UnknownDeclaration`) is a consequence, not an error of its own, and is
/// not reported. Each round sets aside at least one declaration, so this
/// ends.
fn admit_types(
    mut declarations: Vec<CompositeDeclaration>,
    spans: &BTreeMap<String, Span>,
) -> Result<TypeEnvironment, AssemblyRefusal> {
    let mut errors = Vec::new();
    loop {
        match TypeEnvironment::new(declarations.clone(), []) {
            Ok(types) if errors.is_empty() => return Ok(types),
            Ok(_) => return refuse(errors),
            Err(invalid) => {
                let before = declarations.len();
                declarations.retain(|declaration| declaration.name() != invalid.declaration);
                let set_aside = declarations.len() < before;
                let consequence =
                    set_aside && matches!(invalid.cause, DeclarationCause::UnknownDeclaration(_));
                if !consequence {
                    let span = spans
                        .get(&invalid.declaration)
                        .copied()
                        .unwrap_or(Span { start: 0, end: 0 });
                    errors.push(AssemblyError {
                        cause: AssemblyCause::InvalidTypeDeclaration(invalid),
                        span,
                    });
                }
                if !set_aside {
                    return refuse(errors);
                }
            }
        }
    }
}

impl PackageDeclarations {
    /// FR-091's assembler: the package declared by `unit`, the S2 output of
    /// the source unit `source` names, whose authority and identity are the
    /// owner of every declared node (ADR-013 O-04).
    pub fn assemble(source: RawSourceRef, unit: ParsedUnit) -> Result<Self, AssemblyRefusal> {
        let (selections, forms) = unit.into_parts();
        let unit = Unit::new(forms);
        let mut errors = Vec::new();

        // Selection aliases are unique within a unit, and a `using` alias
        // names one of its profile selections.
        let mut aliases: BTreeMap<&str, Vec<Span>> = BTreeMap::new();
        for (alias, span) in selections
            .profiles
            .iter()
            .map(|profile| (profile.alias.as_str(), profile.span))
            .chain(
                selections
                    .imports
                    .iter()
                    .filter_map(|import| Some((import.alias.as_deref()?, import.span))),
            )
            .chain(
                selections
                    .models
                    .iter()
                    .map(|model| (model.alias.as_str(), model.span)),
            )
        {
            aliases.entry(alias).or_default().push(span);
        }
        for (alias, spans) in &aliases {
            if let [_, second, ..] = spans.as_slice() {
                errors.push(AssemblyError {
                    cause: AssemblyCause::DuplicateAlias {
                        alias: (*alias).to_owned(),
                        spans: spans.clone(),
                    },
                    span: *second,
                });
            }
        }
        let profiles: BTreeSet<&str> = selections
            .profiles
            .iter()
            .map(|profile| profile.alias.as_str())
            .collect();
        for function in &unit.functions {
            if let Some(using) = function.using() {
                if !profiles.contains(using.alias.as_str()) {
                    errors.push(AssemblyError {
                        cause: AssemblyCause::UndeclaredAlias {
                            alias: using.alias.clone(),
                        },
                        span: using.span,
                    });
                }
            }
        }

        // A declared type is named by its declared name alone (its handle,
        // its region, every reference to it), so a name binds one
        // declaration: every declaration after the first is refused, in
        // whichever order the declarations are written.
        for (name, declared) in &unit.declared {
            let candidates: Vec<Span> = declared.iter().map(|(_, span)| *span).collect();
            for (_, span) in declared.iter().skip(1) {
                errors.push(AssemblyError {
                    cause: AssemblyCause::AmbiguousTypeName {
                        name: name.clone(),
                        candidates: candidates.clone(),
                    },
                    span: *span,
                });
            }
        }

        // Every type form names declarations of the unit only.
        for alias in &unit.aliases {
            check_names(&unit, &alias.target, None, &mut errors);
        }
        for composite in &unit.composites {
            match &composite.members {
                Members::Record(fields) => {
                    for field in fields {
                        check_names(&unit, &field.type_form, None, &mut errors);
                    }
                }
                Members::Tuple(elements) => {
                    for element in elements {
                        check_names(&unit, element, None, &mut errors);
                    }
                }
            }
        }
        for function in &unit.functions {
            let profile = function.using().map(|using| using.alias.as_str());
            for form in signature_type_forms(function) {
                check_names(&unit, form, profile, &mut errors);
            }
            for form in body_type_forms(function) {
                check_names(&unit, &form, profile, &mut errors);
            }
        }
        if !errors.is_empty() {
            return refuse(errors);
        }

        // Each declared record and tuple's handle, over the unit's owner.
        let owner = SourceOwner::from(&source);
        let mut names = Names::default();
        let mut handles = Vec::with_capacity(unit.composites.len());
        for composite in &unit.composites {
            match declared_type_handle(&owner, &composite.name.name) {
                Ok(handle) => {
                    names
                        .types
                        .entry(composite.name.name.clone())
                        .or_default()
                        .push(ValueType::Composite(handle));
                    handles.push(handle);
                }
                Err(refusal) => errors.push(AssemblyError {
                    cause: AssemblyCause::Handle(refusal),
                    span: composite.name.span,
                }),
            }
        }
        if !errors.is_empty() {
            return refuse(errors);
        }

        // Aliases, each after every alias it names; a cycle is refused.
        let edges: Vec<Vec<usize>> = unit
            .aliases
            .iter()
            .map(|alias| named_aliases(&unit, &alias.target))
            .collect();
        let mut resolved: Vec<Option<ValueType>> = vec![None; unit.aliases.len()];
        let mut failed = vec![false; unit.aliases.len()];
        for component in strongly_connected(&edges) {
            let cyclic = component.len() > 1
                || component
                    .first()
                    .is_some_and(|member| edges[*member].contains(member));
            if cyclic {
                let members: BTreeSet<usize> = component.iter().copied().collect();
                let mut cycle = Vec::new();
                for &member in &component {
                    failed[member] = true;
                    for &target in &edges[member] {
                        if members.contains(&target) {
                            cycle.push((
                                unit.aliases[member].name.name.clone(),
                                unit.aliases[target].name.name.clone(),
                            ));
                        }
                    }
                }
                cycle.sort();
                cycle.dedup();
                let first = component.iter().min().copied().unwrap_or_default();
                errors.push(AssemblyError {
                    cause: AssemblyCause::AliasCycle { edges: cycle },
                    span: unit.aliases[first].name.span,
                });
                continue;
            }
            for member in component {
                if edges[member].iter().any(|target| failed[*target]) {
                    failed[member] = true;
                    continue;
                }
                let alias = &unit.aliases[member];
                match resolve_form(&names, &alias.target) {
                    Ok(value_type) => {
                        names
                            .types
                            .entry(alias.name.name.clone())
                            .or_default()
                            .push(value_type.clone());
                        resolved[member] = Some(value_type);
                    }
                    Err(error) => {
                        failed[member] = true;
                        errors.push(resolution_error(&unit, error));
                    }
                }
            }
        }
        if !errors.is_empty() {
            return refuse(errors);
        }

        // Records and tuples.
        let mut declarations = Vec::with_capacity(unit.composites.len());
        let mut declared_type_spans = BTreeMap::new();
        for (composite, handle) in unit.composites.iter().zip(&handles) {
            let shape = match &composite.members {
                Members::Record(fields) => {
                    let mut declared = Vec::with_capacity(fields.len());
                    for field in fields {
                        match resolve_form(&names, &field.type_form) {
                            Ok(value_type) => declared.push(FieldDeclaration::new(
                                field.name.clone(),
                                value_type,
                                if field.optional {
                                    Presence::Optional
                                } else {
                                    Presence::Required
                                },
                            )),
                            Err(error) => errors.push(resolution_error(&unit, error)),
                        }
                    }
                    CompositeShape::Record(declared)
                }
                Members::Tuple(elements) => {
                    let mut declared = Vec::with_capacity(elements.len());
                    for element in elements {
                        match resolve_form(&names, element) {
                            Ok(value_type) => declared.push(value_type),
                            Err(error) => errors.push(resolution_error(&unit, error)),
                        }
                    }
                    CompositeShape::Tuple(declared)
                }
            };
            declarations.push(CompositeDeclaration::new(
                *handle,
                composite.name.name.clone(),
                shape,
            ));
            declared_type_spans.insert(composite.name.name.clone(), composite.name.span);
        }

        // Each function's check-owned resolved signature.
        let mut signatures: Vec<ResolvedSignature> = Vec::with_capacity(unit.functions.len());
        for function in &unit.functions {
            let mut parameters = Vec::with_capacity(function.parameters.len());
            for (name, form) in &function.parameters {
                match resolve_form(&names, form) {
                    Ok(value_type) => parameters.push((name.clone(), value_type)),
                    Err(error) => errors.push(resolution_error(&unit, error)),
                }
            }
            match resolve_form(&names, &function.result) {
                Ok(result) => signatures.push((parameters, result)),
                Err(error) => errors.push(resolution_error(&unit, error)),
            }
            // A type form inside the body or measure is resolved here too,
            // so its errors are assembler errors like a signature's; check
            // resolves it again against the package scope.
            for form in body_type_forms(function) {
                if let Err(error) = resolve_form(&names, &form) {
                    errors.push(resolution_error(&unit, error));
                }
            }
        }
        if !errors.is_empty() {
            return refuse(errors);
        }
        let types = admit_types(declarations, &declared_type_spans)?;

        let mut package = PackageDeclarations::new(source);
        package.types = types;
        package.aliases = unit
            .aliases
            .iter()
            .zip(resolved)
            .filter_map(|(alias, value_type)| Some((alias.name.name.clone(), value_type?)))
            .collect();
        for (index, signature) in signatures.into_iter().enumerate() {
            package.resolved_signatures.insert(index, signature);
        }
        package.functions = unit.functions;
        package.declared_type_spans = declared_type_spans;
        Ok(package)
    }
}

#[cfg(test)]
mod tests;
