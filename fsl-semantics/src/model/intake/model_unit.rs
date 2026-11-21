// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-710: compile an FSL model unit to its FR-154 domain-package document.

use fsl_forms::cst::{self, CompleteCause, CompleteCode, CompleteDiagnostic, ParsedSource};
use fsl_foundation::diagnostic::{Code, Diagnostic};
use fsl_foundation::{Phase, Source, SourceIdentity, Span};
use serde_json::{json, Value};

use crate::model::accounting::ModelNormalizationLimits;

/// A model unit compiled to the Semantic IR document FR-154 admits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledModelUnit {
    identity: String,
    bytes: Vec<u8>,
}

/// Whether bytes begin with an FSL model-unit header and `domain` selection.
pub(super) fn looks_like_model_unit(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    if !text.trim_start().starts_with("language") {
        return false;
    }
    let Ok(parsed) = cst::parse(
        SourceIdentity::new("filament", "model-unit-probe"),
        "model-unit.fsl",
        bytes,
        cst::Limits::default(),
    ) else {
        return false;
    };
    let tokens = parsed
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() == cst::TokenClass::Token)
        .map(|token| token.spelling())
        .collect::<Vec<_>>();
    tokens.get(5).is_some_and(|token| *token == b"domain")
}

/// Whether source bytes are an admitted FSL model unit with a `domain` identity.
pub fn is_model_unit_source(bytes: &[u8]) -> bool {
    looks_like_model_unit(bytes)
}

/// Compile one anonymous model-unit byte source using its domain as the
/// source identity label.
pub(super) fn compile_model_unit_bytes(
    bytes: &[u8],
) -> Result<CompiledModelUnit, super::ModelRefusal> {
    let probe = cst::parse(
        SourceIdentity::new("filament", "model-unit-probe"),
        "model-unit.fsl",
        bytes,
        cst::Limits::default(),
    )
    .map_err(|diagnostic| refusal(format!("{diagnostic}")))?;
    let tokens = probe
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() == cst::TokenClass::Token)
        .map(|token| String::from_utf8_lossy(token.spelling()).into_owned())
        .collect::<Vec<_>>();
    let identity = tokens
        .get(6)
        .and_then(|value| serde_json::from_str::<String>(value).ok())
        .ok_or_else(|| refusal("model-unit domain identity is missing"))?;
    let source = cst::read_source(
        SourceIdentity::new("filament", identity),
        "model-unit.fsl",
        bytes,
        cst::Limits::default().source_bytes,
    )
    .map_err(|error| refusal(format!("{error}")))?;
    let parsed = cst::parse_source(source, cst::Limits::default())
        .map_err(|diagnostic| refusal(format!("{diagnostic}")))?;
    compile_model_unit_with_causes(&parsed, ModelNormalizationLimits::default()).map_err(
        |diagnostics| {
            refusal(diagnostics.first().map_or_else(
                || "model-unit compilation failed".to_owned(),
                ToString::to_string,
            ))
        },
    )
}

/// Compile already-labelled sources to package input, preserving each source
/// identity in node origins.
pub fn package_input_from_sources<'a>(
    sources: impl IntoIterator<Item = (&'a str, &'a Source)>,
) -> Result<super::PackageInput, super::ModelRefusal> {
    let mut documents = Vec::new();
    for (selected_identity, source) in sources {
        let parsed = cst::parse_source(source.clone(), cst::Limits::default())
            .map_err(|diagnostic| refusal(format!("{diagnostic}")))?;
        let compiled = compile_model_unit_with_causes(&parsed, ModelNormalizationLimits::default())
            .map_err(|diagnostics| {
                refusal(diagnostics.first().map_or_else(
                    || "model-unit compilation failed".to_owned(),
                    ToString::to_string,
                ))
            })?;
        documents.push((selected_identity.to_owned(), compiled.bytes().to_vec()));
    }
    let mut packages = super::PackageInput::new();
    for (identity, bytes) in documents {
        if packages.insert(identity.clone(), bytes).is_some() {
            return Err(super::ModelRefusal {
                code: fsl_foundation::Code::InvalidPackage,
                cause: super::ModelRefusalCause::ConflictingPackage {
                    identity: identity.clone(),
                },
                detail: format!("two supplied model units declare identity {identity:?}"),
            });
        }
    }
    Ok(packages)
}

fn refusal(detail: impl Into<String>) -> super::ModelRefusal {
    super::ModelRefusal {
        code: fsl_foundation::Code::InvalidModelBinding,
        cause: super::ModelRefusalCause::MalformedDeclaration,
        detail: detail.into(),
    }
}

impl CompiledModelUnit {
    /// The domain identity declared by the unit.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// The RFC 8785 bytes of its Semantic IR 2.0.0 document.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Clone, Debug)]
struct Token {
    text: String,
    span: Span,
}

/// Compile an admitted model unit. Source labels and declaration spans are
/// retained in each document node's `origin`.
pub fn compile_model_unit(
    parsed: &ParsedSource,
    limits: ModelNormalizationLimits,
) -> Result<CompiledModelUnit, Vec<Diagnostic>> {
    compile_model_unit_with_causes(parsed, limits).map_err(|diagnostics| {
        diagnostics
            .iter()
            .map(|diagnostic| project_diagnostic(parsed.source(), diagnostic))
            .collect()
    })
}

/// Compile a model unit while retaining the complete-V1 diagnostic causes
/// needed by command outcome projection.
pub fn compile_model_unit_with_causes(
    parsed: &ParsedSource,
    limits: ModelNormalizationLimits,
) -> Result<CompiledModelUnit, Vec<CompleteDiagnostic>> {
    if !parsed.is_admissible() {
        return Err(parsed.diagnostics().to_vec());
    }
    let tokens = parsed
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() == cst::TokenClass::Token)
        .map(|token| Token {
            text: String::from_utf8_lossy(token.spelling()).into_owned(),
            span: token.span(),
        })
        .collect::<Vec<_>>();
    let source = parsed.source();
    let mut cursor = 0;
    let Some(identity) = read_identity(&tokens, &mut cursor) else {
        return Err(vec![diagnostic(
            parsed,
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedToken,
            tokens
                .get(cursor)
                .map_or(Span { start: 0, end: 0 }, |t| t.span),
            "expected a model unit's domain declaration",
        )]);
    };
    if identity == "filament/native"
        || identity == "filament/unit"
        || identity.starts_with("filament/native/")
        || identity.starts_with("filament/unit/")
    {
        let span = tokens
            .get(cursor.saturating_sub(2))
            .map_or(Span { start: 0, end: 0 }, |token| token.span);
        return Err(vec![diagnostic(
            parsed,
            CompleteCode::InvalidModelBinding,
            CompleteCause::MalformedDeclaration,
            span,
            "the domain identity is reserved by FR-154",
        )]);
    }

    let source_identity = format!(
        "ix://{}/{}",
        source.identity().authority,
        source.identity().identity
    );
    let mut types = Vec::new();
    let mut populations = Vec::new();
    let mut dimensions = Vec::new();
    let mut units = Vec::new();
    let mut object_names = std::collections::BTreeSet::new();
    let mut declaration_names = std::collections::BTreeSet::new();
    let mut population_names = std::collections::BTreeSet::new();
    let mut references = Vec::new();
    let mut supertype_references = Vec::new();
    let mut object_links = Vec::new();
    let mut has_object_type = false;
    let mut has_record_type = false;
    let mut has_tuple_type = false;
    let mut has_value_type = false;
    let mut has_variant_type = false;
    let mut has_event_type = false;
    let mut has_namespace = false;
    let mut has_systems_interface = false;
    let mut has_systems_part = false;
    let mut has_systems_port = false;
    let mut has_systems_connection = false;
    let mut has_systems_allocation = false;
    let mut has_state_machine = false;
    let mut has_process = false;
    let mut has_persistence_interface = false;
    let mut index = cursor;
    while index < tokens.len() {
        let declaration_start = index;
        if tokens[index].text == "abstract" {
            index += 1;
        }
        validate_declaration_inheritance_syntax(&tokens, index, parsed)?;
        match tokens.get(index).map(|token| token.text.as_str()) {
            Some("object") => {
                let Some((object, next)) = read_object(&tokens, index, &identity, parsed) else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid object declaration",
                    )]);
                };
                let unique_object = object_names.insert(object.name.clone());
                let unique_declaration = declaration_names.insert(object.name.clone());
                if !unique_object || !unique_declaration {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        object.name_span,
                        format!("object {} is declared more than once", object.name),
                    )]);
                }
                references.extend(object.relationship_targets.iter().cloned());
                supertype_references.extend(object.links.supertypes.iter().cloned());
                types.extend(object.additional_types);
                let mut links = object.links;
                links.type_index = types.len();
                object_links.push(links);
                types.push(object.wire);
                has_object_type = true;
                index = next;
            }
            Some("population") | Some("open") | Some("symmetric") => {
                let Some((population, next)) = read_population(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid population declaration",
                    )]);
                };
                if !population_names.insert(
                    population["displayName"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                ) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start].span,
                        "population is declared more than once",
                    )]);
                }
                populations.push(population);
                index = next;
            }
            Some("type") => {
                let Some((value_type, name, next)) =
                    read_scalar_alias(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::UnsupportedConstruct,
                        CompleteCause::DeclarationForm,
                        tokens[declaration_start].span,
                        "only scalar native type aliases are supported by model-unit compilation",
                    )]);
                };
                if !declaration_names.insert(name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {name} is declared more than once"),
                    )]);
                }
                types.push(value_type);
                has_value_type = true;
                index = next;
            }
            Some("map") => {
                let Some((map_type, name, next)) = read_map_type(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid map type declaration",
                    )]);
                };
                if !declaration_names.insert(name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {name} is declared more than once"),
                    )]);
                }
                types.push(map_type);
                index = next;
            }
            Some("event") => {
                let Some((event, next)) = read_event_type(&tokens, index, &identity, parsed) else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid event type declaration",
                    )]);
                };
                let event_name = event.name.clone();
                if !declaration_names.insert(event_name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {event_name} is declared more than once"),
                    )]);
                }
                push_model_type(
                    event,
                    &mut types,
                    &mut object_links,
                    &mut supertype_references,
                );
                has_event_type = true;
                index = next;
            }
            Some("namespace") => {
                let Some((namespace, name, next)) =
                    read_namespace(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid namespace declaration",
                    )]);
                };
                if !declaration_names.insert(name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {name} is declared more than once"),
                    )]);
                }
                types.push(namespace);
                has_namespace = true;
                index = next;
            }
            Some("interface") => {
                let Some((interface, next)) =
                    read_systems_interface(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid systems interface declaration",
                    )]);
                };
                let interface_name = interface.name.clone();
                if !declaration_names.insert(interface_name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {interface_name} is declared more than once"),
                    )]);
                }
                push_model_type(
                    interface,
                    &mut types,
                    &mut object_links,
                    &mut supertype_references,
                );
                has_systems_interface = true;
                index = next;
            }
            Some("part") => {
                let Some((node, name, next)) = read_systems_part(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid systems part declaration",
                    )]);
                };
                if !declaration_names.insert(name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {name} is declared more than once"),
                    )]);
                }
                types.push(node);
                has_systems_part = true;
                index = next;
            }
            Some("port") => {
                let Some((node, name, next)) = read_systems_port(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid systems port declaration",
                    )]);
                };
                if !declaration_names.insert(name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {name} is declared more than once"),
                    )]);
                }
                types.push(node);
                has_systems_port = true;
                index = next;
            }
            Some("connection") => {
                let Some((node, name, next)) =
                    read_systems_connection(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid systems connection declaration",
                    )]);
                };
                if !declaration_names.insert(name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {name} is declared more than once"),
                    )]);
                }
                types.push(node);
                has_systems_connection = true;
                index = next;
            }
            Some("allocate") => {
                let Some((node, name, next)) =
                    read_systems_allocation(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid systems allocation declaration",
                    )]);
                };
                if !declaration_names.insert(name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {name} is declared more than once"),
                    )]);
                }
                types.push(node);
                has_systems_allocation = true;
                index = next;
            }
            Some("machine") => {
                let Some((machine, next)) = read_state_machine(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid state machine declaration",
                    )]);
                };
                if !declaration_names.insert(machine.name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {} is declared more than once", machine.name),
                    )]);
                }
                supertype_references.extend(machine.links.supertypes.iter().cloned());
                types.extend(machine.additional_types);
                let mut links = machine.links;
                links.type_index = types.len();
                object_links.push(links);
                types.push(machine.wire);
                has_state_machine = true;
                index = next;
            }
            Some("process") => {
                let Some((process, next)) = read_process(&tokens, index, &identity, parsed) else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid process declaration",
                    )]);
                };
                if !declaration_names.insert(process.name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {} is declared more than once", process.name),
                    )]);
                }
                supertype_references.extend(process.links.supertypes.iter().cloned());
                types.extend(process.additional_types);
                let mut links = process.links;
                links.type_index = types.len();
                object_links.push(links);
                types.push(process.wire);
                has_process = true;
                index = next;
            }
            Some("store") => {
                let Some((store, next)) = read_store(&tokens, index, &identity, parsed) else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid persistence interface declaration",
                    )]);
                };
                if !declaration_names.insert(store.name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {} is declared more than once", store.name),
                    )]);
                }
                push_model_type(
                    store,
                    &mut types,
                    &mut object_links,
                    &mut supertype_references,
                );
                has_persistence_interface = true;
                index = next;
            }
            Some("dimension") => {
                let Some((dimension, name, next)) =
                    read_dimension(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid dimension declaration",
                    )]);
                };
                if !declaration_names.insert(name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {name} is declared more than once"),
                    )]);
                }
                dimensions.push(dimension);
                index = next;
            }
            Some("unit") => {
                let Some((unit, name, next)) = read_unit(&tokens, index, &identity, parsed) else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid unit declaration",
                    )]);
                };
                if !declaration_names.insert(name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start + 1].span,
                        format!("declaration {name} is declared more than once"),
                    )]);
                }
                units.push(unit);
                index = next;
            }
            Some("tuple") => {
                let Some((tuple_type, name, next)) =
                    read_tuple_type(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid tuple type declaration",
                    )]);
                };
                if !declaration_names.insert(name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens
                            .get(declaration_start + 1)
                            .map_or(tokens[declaration_start].span, |token| token.span),
                        format!("declaration {name} is declared more than once"),
                    )]);
                }
                types.push(tuple_type);
                has_tuple_type = true;
                index = next;
            }
            Some("enum") | Some("ordered") | Some("union") => {
                let Some((variant_type, name, next)) =
                    read_variant_type(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid variant type declaration",
                    )]);
                };
                if !declaration_names.insert(name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens
                            .get(declaration_start + 1)
                            .map_or(tokens[declaration_start].span, |token| token.span),
                        format!("declaration {name} is declared more than once"),
                    )]);
                }
                types.push(variant_type);
                has_variant_type = true;
                index = next;
            }
            Some("record") => {
                if let Some(span) = record_operation_span(&tokens, index) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::UnsupportedConstruct,
                        CompleteCause::DeclarationForm,
                        span,
                        "operations are not admitted on a record value type",
                    )]);
                }
                let Some((mut record, next)) = read_object(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[index].span,
                        "invalid record declaration",
                    )]);
                };
                if !declaration_names.insert(record.name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens
                            .get(index + 1)
                            .map_or(tokens[index].span, |token| token.span),
                        format!("declaration {} is declared more than once", record.name),
                    )]);
                }
                record.wire["kind"]["name"] = json!("record_value_type");
                if let Some(object) = record.wire.as_object_mut() {
                    object.remove("operations");
                }
                push_model_type(
                    record,
                    &mut types,
                    &mut object_links,
                    &mut supertype_references,
                );
                has_record_type = true;
                index = next;
            }
            Some(_) => {
                let token = &tokens[index];
                return Err(vec![diagnostic(
                    parsed,
                    CompleteCode::UnsupportedConstruct,
                    CompleteCause::DeclarationForm,
                    token.span,
                    format!(
                        "model declaration `{}` is not supported by intake",
                        token.text
                    ),
                )]);
            }
            None => break,
        }
    }
    for (target, span) in references {
        if !object_names.contains(&target) {
            return Err(vec![diagnostic(
                parsed,
                CompleteCode::MissingDeclaration,
                CompleteCause::MissingDeclaration,
                span,
                format!("relationship target {target:?} names no object declaration"),
            )]);
        }
    }
    for (supertype, span) in supertype_references {
        if !declaration_names.contains(&supertype) {
            return Err(vec![diagnostic(
                parsed,
                CompleteCode::MissingDeclaration,
                CompleteCause::MissingDeclaration,
                span,
                format!("supertype {supertype:?} names no declaration"),
            )]);
        }
    }
    link_inheritance(&mut types, &object_links, &identity, parsed)?;
    for population in &populations {
        if let Some(members) = population["members"].as_array() {
            for member in members {
                let Some(member_identity) = member.as_str() else {
                    continue;
                };
                let Some(target) = member_identity.strip_prefix(&format!("ix://{identity}/"))
                else {
                    continue;
                };
                if !object_names.contains(target) {
                    let span = tokens
                        .get(cursor.saturating_sub(1))
                        .map_or(Span { start: 0, end: 0 }, |token| token.span);
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::MissingDeclaration,
                        CompleteCause::MissingDeclaration,
                        span,
                        format!("population member {target:?} names no object declaration"),
                    )]);
                }
            }
        }
    }
    let _ = limits;
    let mut meanings = Vec::new();
    if has_object_type {
        meanings.push(("object_type", "filament.meaning.model.object-type/v1"));
    }
    if has_value_type {
        meanings.push(("value_type", "filament.meaning.model.value-type/v1"));
    }
    if has_record_type {
        meanings.push((
            "record_value_type",
            "filament.meaning.model.record-value-type/v1",
        ));
    }
    if has_tuple_type {
        meanings.push(("tuple_type", "filament.meaning.model.tuple-value-type/v1"));
    }
    if has_variant_type {
        meanings.push(("variant_type", "filament.meaning.model.variant-type/v1"));
    }
    if has_event_type {
        meanings.push(("event_type", "filament.meaning.model.event-type/v1"));
    }
    if has_namespace {
        meanings.push(("namespace", "filament.meaning.model.namespace/v1"));
    }
    if has_systems_interface {
        meanings.push(("interface", "filament.meaning.systems.interface/v1"));
    }
    if has_systems_part {
        meanings.push(("part", "filament.meaning.systems.part/v1"));
    }
    if has_systems_port {
        meanings.push(("port", "filament.meaning.systems.port/v1"));
    }
    if has_systems_connection {
        meanings.push(("connection", "filament.meaning.systems.connection/v1"));
    }
    if has_systems_allocation {
        meanings.push(("allocation", "filament.meaning.systems.allocation/v1"));
    }
    if has_state_machine {
        meanings.push(("machine", "filament.meaning.model.state-machine/v1"));
    }
    if has_process {
        meanings.push(("process", "filament.meaning.model.process/v1"));
    }
    if has_persistence_interface {
        meanings.push(("store", "filament.meaning.model.persistence-interface/v1"));
    }
    if !populations.is_empty() {
        meanings.push(("population", "filament.meaning.model.population/v1"));
    }
    let constructs = meanings
        .into_iter()
        .map(|(name, meaning)| {
            let members = match name {
                "object_type" => json!({"identityFields": "optional"}),
                "record_value_type" => json!({"identityFields": "optional"}),
                "tuple_type" => json!({"positions": "required"}),
                "variant_type" => json!({"variants": "required"}),
                "event_type" => json!({
                    "identityFields": "optional",
                    "occurrenceField": "required",
                }),
                "namespace" => json!({"members": "required", "vocabulary": "required"}),
                "interface" => json!({
                    "featureOrder": "required",
                    "fields": "optional",
                    "operations": "optional",
                    "supertypes": "optional",
                }),
                "part" => json!({
                    "declaredType": "required",
                    "fields": "forbidden",
                    "multiplicity": "required",
                    "operations": "forbidden",
                    "owner": "required",
                }),
                "port" => json!({
                    "direction": "required",
                    "fields": "forbidden",
                    "interfaceType": "required",
                    "multiplicity": "required",
                    "operations": "forbidden",
                    "owner": "required",
                }),
                "connection" => json!({
                    "fields": "forbidden",
                    "flowDirection": "required",
                    "operations": "forbidden",
                    "sourceEnd": "required",
                    "targetEnd": "required",
                }),
                "allocation" => json!({
                    "fields": "forbidden",
                    "operations": "forbidden",
                    "sourceElement": "required",
                    "targetElement": "required",
                }),
                "machine" => json!({"states": "required", "transitions": "required"}),
                "process" => json!({"identityFields": "optional", "steps": "required"}),
                "store" => json!({"persists": "required"}),
                "population" => json!({}),
                _ => json!({}),
            };
            json!({
                "kind": {"module": identity, "name": name},
                "moduleVersion": "1.0.0",
                "construct": {
                    "identity": "none",
                    "shape": match name {
                        "interface" => "interface",
                        "tuple_type" => "tuple",
                        "variant_type" => "enumeration",
                        _ => "record",
                    },
                    "members": members,
                    "meaning": meaning
                },
            })
        })
        .collect::<Vec<_>>();
    for wire in &mut types {
        let empty = wire["identityFields"]
            .as_array()
            .is_some_and(|fields| fields.is_empty());
        let inherited_process = wire["kind"]["name"] == "process"
            && wire["supertypes"]
                .as_array()
                .is_some_and(|types| !types.is_empty());
        if empty && !inherited_process {
            if let Some(object) = wire.as_object_mut() {
                object.remove("identityFields");
            }
        }
    }
    let document = json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": source_identity, "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {"identity": identity, "version": "1.0.0", "mappingVersions": [], "profileVersions": []},
        "occurrences": [], "extensions": [], "constructs": constructs,
        "types": types, "populations": populations,
        "dimensions": dimensions, "units": units,
    });
    let encoded = serde_json::to_vec(&document).map_err(|error| {
        vec![diagnostic(
            parsed,
            CompleteCode::RuntimeInvariant,
            CompleteCause::EstablishedInvariantBroken,
            Span { start: 0, end: 0 },
            format!("model-unit document serialization failed: {error}"),
        )]
    })?;
    validate_nonobject_links(&document, &encoded, &object_links, parsed, limits)?;
    let tree = quire_canonical::read(&encoded, limits.intake.input_bytes).map_err(|error| {
        vec![diagnostic(
            parsed,
            CompleteCode::RuntimeInvariant,
            CompleteCause::EstablishedInvariantBroken,
            Span { start: 0, end: 0 },
            format!("model-unit document could not be read for canonicalization: {error}"),
        )]
    })?;
    let bytes = quire_canonical::to_vec(
        &tree,
        quire_canonical::Limits::new(limits.intake.input_bytes),
    )
    .map_err(|error| {
        vec![diagnostic(
            parsed,
            CompleteCode::RuntimeInvariant,
            CompleteCause::EstablishedInvariantBroken,
            Span { start: 0, end: 0 },
            format!("model-unit document could not be canonicalized: {error}"),
        )]
    })?;
    Ok(CompiledModelUnit { identity, bytes })
}

fn record_operation_span(tokens: &[Token], mut index: usize) -> Option<Span> {
    while tokens.get(index)?.text != "{" {
        index += 1;
    }
    index += 1;
    let mut depth = 1usize;
    while let Some(token) = tokens.get(index) {
        match token.text.as_str() {
            "{" => depth += 1,
            "}" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return None;
                }
            }
            "operation"
                if depth == 1 && tokens.get(index + 1).is_some_and(|next| next.text != ":") =>
            {
                return Some(token.span);
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn read_identity(tokens: &[Token], cursor: &mut usize) -> Option<String> {
    if tokens.get(*cursor)?.text != "language" {
        return None;
    }
    *cursor += 1;
    *cursor += 1;
    *cursor += 1;
    *cursor += 1;
    if tokens.get(*cursor)?.text != ";" {
        return None;
    }
    *cursor += 1;
    if tokens.get(*cursor)?.text != "domain" {
        return None;
    }
    *cursor += 1;
    let identity = serde_json::from_str::<String>(&tokens.get(*cursor)?.text).ok()?;
    *cursor += 1;
    if tokens.get(*cursor)?.text != ";" {
        return None;
    }
    *cursor += 1;
    Some(identity)
}

/// Keep the generic member reader within FR-710's per-declaration grammar.
fn validate_declaration_inheritance_syntax(
    tokens: &[Token],
    index: usize,
    parsed: &ParsedSource,
) -> Result<(), Vec<CompleteDiagnostic>> {
    let Some(kind) = tokens.get(index).map(|token| token.text.as_str()) else {
        return Ok(());
    };
    if !["record", "event", "interface", "store"].contains(&kind) {
        return Ok(());
    }
    let unexpected = |token: &Token| {
        vec![diagnostic(
            parsed,
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedToken,
            token.span,
            format!("{} is not admitted in a {kind} declaration", token.text),
        )]
    };
    if let Some(token) = tokens[index..]
        .iter()
        .take_while(|token| token.text != "{")
        .find(|token| token.text == "extends")
    {
        return Err(unexpected(token));
    }
    if ["record", "event", "interface"].contains(&kind) {
        if let Some(token) = tokens.get(index + 2).filter(|token| token.text != "{") {
            return Err(unexpected(token));
        }
    }
    let Some(body) = tokens[index..]
        .iter()
        .position(|token| token.text == "{")
        .map(|offset| index + offset)
    else {
        return Ok(());
    };
    let mut depth = 1;
    let mut statement = body + 1;
    for at in body + 1..tokens.len() {
        let token = &tokens[at];
        if depth == 1 && token.text != "}" {
            let operation = tokens
                .get(statement)
                .is_some_and(|first| first.text == "operation")
                && tokens
                    .get(statement + 1)
                    .is_some_and(|next| next.text != ":");
            if (kind == "event" && operation)
                || (kind == "store" && !operation)
                || (kind == "record"
                    && !operation
                    && at > statement + 2
                    && ["subsets", "redefines"].contains(&token.text.as_str()))
            {
                return Err(unexpected(token));
            }
        }
        match token.text.as_str() {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                if depth == 1 {
                    statement = at + 1;
                }
            }
            ";" if depth == 1 => statement = at + 1,
            _ => {}
        }
    }
    Ok(())
}

fn push_model_type(
    model: ObjectRead,
    types: &mut Vec<Value>,
    links: &mut Vec<ObjectLinks>,
    references: &mut Vec<(String, Span)>,
) {
    references.extend(model.links.supertypes.iter().cloned());
    types.extend(model.additional_types);
    let mut model_links = model.links;
    model_links.type_index = types.len();
    links.push(model_links);
    types.push(model.wire);
}

/// FR-710 projects shared FR-154 refusals of the newly linked non-object
/// clauses back to their authored declaration, before publishing a document.
fn validate_nonobject_links(
    tree: &Value,
    encoded: &[u8],
    objects: &[ObjectLinks],
    parsed: &ParsedSource,
    limits: ModelNormalizationLimits,
) -> Result<(), Vec<CompleteDiagnostic>> {
    let changed: Vec<_> = objects
        .iter()
        .filter_map(|object| {
            let wire = tree.get("types")?.as_array()?.get(object.type_index)?;
            let kind = wire.get("kind")?.get("name")?.as_str()?;
            let links = object
                .fields
                .iter()
                .any(|field| field.subsets.is_some() || field.redefines.is_some())
                || object
                    .operations
                    .iter()
                    .any(|operation| operation.redefines.is_some());
            (["record_value_type", "event_type", "interface", "store"].contains(&kind) && links)
                .then(|| (object, wire.get("identity").and_then(Value::as_str)))
        })
        .collect();
    if changed.is_empty() {
        return Ok(());
    }
    let document = super::PackageDocument::parse(encoded, limits.intake)
        .map_err(|refusal| vec![project_link_refusal(parsed, changed[0].0.span, &refusal)])?;
    let refusals = super::validate_with_semantic_ir(&document)
        .err()
        .unwrap_or_default();
    let diagnostics: Vec<_> = refusals
        .iter()
        .filter_map(|refusal| {
            let node = super::refusal_node(refusal)?;
            let (object, _) = changed.iter().find(|(_, identity)| {
                identity.is_some_and(|identity| {
                    node == identity
                        || node
                            .strip_prefix(identity)
                            .is_some_and(|rest| rest.starts_with('/'))
                })
            })?;
            Some(project_link_refusal(parsed, object.span, refusal))
        })
        .collect();
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn project_link_refusal(
    parsed: &ParsedSource,
    span: Span,
    refusal: &super::ModelRefusal,
) -> CompleteDiagnostic {
    if let super::ModelRefusalCause::IntakeLimitExceeded { bound, .. } = refusal.cause {
        if let Ok(bound) = usize::try_from(bound) {
            let mut reported = *cst::resource_exhausted(
                parsed.source(),
                Phase::Parse,
                span,
                fsl_foundation::SyntaxLimit::SourceBytes { bound },
            );
            reported.code = refusal.code;
            reported.message = refusal.detail.clone();
            return reported;
        }
    }
    let cause = [
        CompleteCause::MalformedDeclaration,
        CompleteCause::MissingDeclaration,
        CompleteCause::DeclarationForm,
        CompleteCause::ConflictingBinding,
    ]
    .into_iter()
    .find(|cause| cause.as_str() == refusal.cause.as_str());
    match cause {
        Some(cause) => diagnostic(parsed, refusal.code, cause, span, refusal.detail.clone()),
        None => diagnostic(
            parsed,
            CompleteCode::RuntimeInvariant,
            CompleteCause::EstablishedInvariantBroken,
            span,
            format!(
                "the model-unit diagnostic API cannot retain shared refusal {}/{}: {}",
                refusal.code.as_str(),
                refusal.cause.as_str(),
                refusal.detail
            ),
        ),
    }
}

/// Writes each `subsets` and `redefines` of the unit's objects into its wire
/// as the identity of the member it names, found on the object that declares
/// it (FR-081): a field `subsets` name is looked up on its owner and supertypes, a
/// `redefines Owner::member` on `Owner` and its supertypes. A name that finds
/// no member is `missing_declaration` at the name; whether `Owner` is a
/// supertype of the redefining object is the model checker's decision (FR-082).
fn link_inheritance(
    types: &mut [Value],
    objects: &[ObjectLinks],
    package: &str,
    parsed: &ParsedSource,
) -> Result<(), Vec<CompleteDiagnostic>> {
    let by_name: std::collections::BTreeMap<&str, &ObjectLinks> = objects
        .iter()
        .map(|object| (object.name.as_str(), object))
        .collect();
    let missing = |span: Span, what: &str, name: &str| {
        vec![diagnostic(
            parsed,
            CompleteCode::MissingDeclaration,
            CompleteCause::MissingDeclaration,
            span,
            format!("{what} {name:?} names no member"),
        )]
    };
    for object in objects {
        for (parent, _) in &object.supertypes {
            let same_meaning = by_name.get(parent.as_str()).is_some_and(|parent| {
                types[object.type_index]["kind"] == types[parent.type_index]["kind"]
            });
            if !same_meaning {
                return Err(vec![diagnostic(
                    parsed,
                    CompleteCode::InvalidModelBinding,
                    CompleteCause::MalformedDeclaration,
                    object.span,
                    "a supertype must have the meaning of the declaration that specializes it",
                )]);
            }
        }
        for (position, field) in object.fields.iter().enumerate() {
            let wire = &mut types[object.type_index]["fields"][position];
            if let Some((name, span)) = &field.subsets {
                let mut subsets = Vec::new();
                match declaring(&by_name, &object.name, name, MemberKind::Field) {
                    Some(owner) => subsets.push(format!("ix://{package}/{owner}/{name}")),
                    None => return Err(missing(*span, "subsetted field", name)),
                }
                wire["subsets"] = json!(subsets);
            }
            if let Some(target) = &field.redefines {
                wire["redefines"] = json!(redefined(
                    &by_name,
                    target,
                    MemberKind::Field,
                    package,
                    &missing
                )?);
            }
        }
        for (position, operation) in object.operations.iter().enumerate() {
            let mut receiver_fields = Vec::new();
            for (name, span) in &operation.receiver_fields {
                let Some(owner) = declaring(&by_name, &object.name, name, MemberKind::Field) else {
                    return Err(missing(*span, "modified field", name));
                };
                receiver_fields.push(format!("ix://{package}/{owner}/{name}"));
            }
            types[object.type_index]["operations"][position]["frame"]["modifies_receiver"] =
                json!(receiver_fields);
            if let Some(target) = &operation.redefines {
                types[object.type_index]["operations"][position]["redefines"] = json!(redefined(
                    &by_name,
                    target,
                    MemberKind::Operation,
                    package,
                    &missing
                )?);
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum MemberKind {
    Field,
    Operation,
}

fn redefined(
    by_name: &std::collections::BTreeMap<&str, &ObjectLinks>,
    target: &MemberTarget,
    kind: MemberKind,
    package: &str,
    missing: &dyn Fn(Span, &str, &str) -> Vec<CompleteDiagnostic>,
) -> Result<String, Vec<CompleteDiagnostic>> {
    if !by_name.contains_key(target.owner.as_str()) {
        return Err(missing(target.owner_span, "redefined type", &target.owner));
    }
    match declaring(by_name, &target.owner, &target.member, kind) {
        Some(owner) => Ok(format!("ix://{package}/{owner}/{}", target.member)),
        None => Err(missing(
            target.member_span,
            "redefined member",
            &target.member,
        )),
    }
}

/// The nearest object, searching `start` and then its supertypes breadth
/// first, that declares `member`. Each object is visited once, so a
/// supertype cycle ends the search (the model checker refuses the cycle).
fn declaring<'a>(
    by_name: &std::collections::BTreeMap<&'a str, &'a ObjectLinks>,
    start: &str,
    member: &str,
    kind: MemberKind,
) -> Option<&'a str> {
    let mut queue = std::collections::VecDeque::from([start]);
    let mut seen = std::collections::BTreeSet::new();
    while let Some(name) = queue.pop_front() {
        let Some(object) = by_name.get(name) else {
            continue;
        };
        if !seen.insert(name) {
            continue;
        }
        let declares = match kind {
            MemberKind::Field => object.fields.iter().any(|field| field.name == member),
            MemberKind::Operation => object
                .operations
                .iter()
                .any(|operation| operation.name == member),
        };
        if declares {
            return Some(object.name.as_str());
        }
        queue.extend(object.supertypes.iter().map(|(name, _)| name.as_str()));
    }
    None
}

struct ObjectRead {
    name: String,
    name_span: Span,
    wire: Value,
    additional_types: Vec<Value>,
    relationship_targets: Vec<(String, Span)>,
    links: ObjectLinks,
}

/// The inheritance edges an object declaration writes, by name, resolved once
/// every object of the unit is read (FR-081, FR-082).
struct ObjectLinks {
    name: String,
    span: Span,
    /// Where the object's wire sits in the document's `types`.
    type_index: usize,
    supertypes: Vec<(String, Span)>,
    /// Every field in declaration order, as the wire lists them.
    fields: Vec<FieldLink>,
    /// Every operation in declaration order, as the wire lists them.
    operations: Vec<OperationLink>,
}

struct FieldLink {
    name: String,
    subsets: Option<(String, Span)>,
    redefines: Option<MemberTarget>,
}

struct OperationLink {
    name: String,
    redefines: Option<MemberTarget>,
    receiver_fields: Vec<(String, Span)>,
}

/// `Owner :: member` of a `redefines` clause.
struct MemberTarget {
    owner: String,
    owner_span: Span,
    member: String,
    member_span: Span,
}

fn read_object(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(ObjectRead, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name_span = tokens.get(index)?.span;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    let mut supertypes = Vec::new();
    if tokens
        .get(index)
        .is_some_and(|token| token.text == "extends")
    {
        index += 1;
        loop {
            let supertype = tokens.get(index)?;
            supertypes.push((supertype.text.clone(), supertype.span));
            index += 1;
            if tokens.get(index).is_some_and(|token| token.text == ",") {
                index += 1;
            } else {
                break;
            }
        }
    }
    let mut identity_fields = Vec::new();
    if tokens
        .get(index)
        .is_some_and(|token| token.text == "identity")
    {
        index += 1;
        if tokens.get(index)?.text != "(" {
            return None;
        }
        index += 1;
        while tokens.get(index)?.text != ")" {
            if tokens.get(index)?.text != "," {
                identity_fields.push(tokens[index].text.clone());
            }
            index += 1;
        }
        index += 1;
    }
    while tokens.get(index)?.text != "{" {
        index += 1;
    }
    index += 1;
    let mut fields = Vec::new();
    let mut operations = Vec::new();
    let mut relationships = Vec::new();
    let mut clauses = Vec::new();
    let mut additional_types = Vec::new();
    let mut relationship_targets = Vec::new();
    let mut field_links = Vec::new();
    let mut operation_links = Vec::new();
    while tokens.get(index)?.text != "}" {
        let member_start = tokens.get(index)?.span.start;
        match tokens.get(index)?.text.as_str() {
            "relationship" => {
                let start = index;
                index += 1;
                let relationship_name = tokens.get(index)?.text.clone();
                index += 1;
                if tokens.get(index)?.text != ":" {
                    return None;
                }
                index += 1;
                let target = tokens.get(index)?.text.clone();
                let target_span = tokens.get(index)?.span;
                index += 1;
                let (lower, upper, ordered, unique, next) = read_multiplicity(tokens, index)?;
                index = next;
                let mut target_role = None;
                // Without an `inverse` the reverse end is unnamed and
                // claims no multiplicity.
                let mut source_multiplicity = end_multiplicity(0, None, false, true);
                if tokens
                    .get(index)
                    .is_some_and(|token| token.text == "inverse")
                {
                    target_role = Some(tokens.get(index + 1)?.text.clone());
                    index += 2;
                    let (lower, upper, ordered, unique, next) = read_multiplicity(tokens, index)?;
                    source_multiplicity = end_multiplicity(lower, upper, ordered, unique);
                    index = next;
                }
                if tokens.get(index)?.text != ";" {
                    return None;
                }
                let end = tokens.get(index)?.span.end;
                index += 1;
                relationship_targets.push((target.clone(), target_span));
                let mut target_end = json!({
                    "type": format!("ix://{package}/{target}"),
                    "multiplicity": end_multiplicity(lower, upper, ordered, unique),
                });
                if let Some(role) = target_role {
                    target_end["role"] = json!(role);
                }
                relationships.push(json!({
                    "identity": format!("ix://{package}/relationship/{name}-{relationship_name}-{target}"),
                    "sourceEnd": {
                        "type": format!("ix://{package}/{name}"), "role": relationship_name,
                        "multiplicity": source_multiplicity,
                    },
                    "targetEnd": target_end,
                    "category": "structural", "composite": false,
                    "direction": "source-to-target",
                    "origin": source_origin(parsed, Span { start: tokens[start].span.start, end }),
                }));
            }
            "operation" if tokens.get(index + 1).is_some_and(|next| next.text != ":") => {
                let (operation, links, next) =
                    read_operation(tokens, index, &name, package, parsed)?;
                operation_links.push(links);
                operations.push(operation);
                index = next;
            }
            "occurs" => {
                index = skip_statement(tokens, index)?;
            }
            "states" | "transition" | "step" => {
                index = skip_statement(tokens, index)?;
            }
            "invariant" => {
                let (clause, next) =
                    read_clause_declaration(tokens, index, &name, package, parsed, true)?;
                clauses.push(clause);
                index = next;
            }
            _ => {
                let field_name = tokens.get(index)?.text.clone();
                index += 1;
                if tokens.get(index)?.text != ":" {
                    return None;
                }
                index += 1;
                let mut type_name = tokens.get(index)?.text.clone();
                index += 1;
                let mut type_reference = None;
                if type_name == "Reference" && tokens.get(index).is_some_and(|t| t.text == "<") {
                    let target = tokens.get(index + 1)?.text.clone();
                    if target == "Clock" && tokens.get(index + 2)?.text == ">" {
                        index += 3;
                        let identity = format!("ix://{package}/Reference_{name}_{field_name}");
                        additional_types.push(json!({
                            "identity": identity,
                            "displayName": "Reference<Clock>",
                            "kind": "reference", "roles": [],
                            "target": "ix://filament/native/Clock",
                            "constraints": [], "extensions": [], "unknownPolicy": "reject",
                            "origin": source_origin(parsed, Span { start: member_start, end: tokens[index - 1].span.end }),
                        }));
                        type_reference = Some(identity);
                    }
                }
                if type_name == "Option" && tokens.get(index).is_some_and(|t| t.text == "<") {
                    index += 1;
                    let inner = tokens.get(index)?.text.clone();
                    index += 1;
                    if tokens.get(index)?.text != ">" {
                        return None;
                    }
                    index += 1;
                    let option_identity = format!("{package}/Option_{name}_{field_name}");
                    let option_identity = format!("ix://{option_identity}");
                    additional_types.push(json!({
                        "identity": option_identity,
                        "displayName": format!("Option<{inner}>"),
                        "kind": "option",
                        "roles": [],
                        "inner": type_ref(&inner, package),
                        "constraints": [],
                        "extensions": [],
                        "unknownPolicy": "reject",
                        "origin": source_origin(parsed, Span { start: member_start, end: tokens[index - 1].span.end }),
                    }));
                    type_reference = Some(option_identity);
                }
                let mut optional = false;
                let inline_type_bounds =
                    matches!(type_name.as_str(), "Int" | "Rational" | "Decimal" | "Text")
                        || (matches!(type_name.as_str(), "Float32" | "Float64")
                            && tokens.get(index + 1).is_some_and(|token| {
                                matches!(
                                    token.text.as_str(),
                                    "exact"
                                        | "toward-zero"
                                        | "toward-positive"
                                        | "toward-negative"
                                        | "nearest-even"
                                        | "nearest-away"
                                )
                            }));
                if inline_type_bounds && tokens.get(index).is_some_and(|token| token.text == "[") {
                    let mut depth = 0usize;
                    while let Some(token) = tokens.get(index) {
                        match token.text.as_str() {
                            "[" => depth += 1,
                            "]" => {
                                depth -= 1;
                                index += 1;
                                if depth == 0 {
                                    break;
                                }
                                continue;
                            }
                            _ => {}
                        }
                        index += 1;
                    }
                }
                if type_name == "Int" {
                    type_name = "Integer".to_owned();
                }
                if tokens.get(index).is_some_and(|token| token.text == "?") {
                    optional = true;
                    index += 1;
                }
                let mut lower = 1u64;
                let mut upper = Some(1u64);
                let mut ordered = false;
                let mut unique = type_name != "Clock";
                if tokens.get(index).is_some_and(|token| token.text == "[") {
                    let (field_lower, field_upper, field_ordered, field_unique, next) =
                        read_multiplicity(tokens, index)?;
                    lower = field_lower;
                    upper = field_upper;
                    ordered = field_ordered;
                    unique = field_unique;
                    index = next;
                }
                let mut subsets = None;
                if tokens
                    .get(index)
                    .is_some_and(|token| token.text == "subsets")
                {
                    let target = tokens.get(index + 1)?;
                    subsets = Some((target.text.clone(), target.span));
                    index += 2;
                }
                let redefines = read_member_target(tokens, &mut index)?;
                if tokens.get(index)?.text != ";" {
                    return None;
                }
                index += 1;
                field_links.push(FieldLink {
                    name: field_name.clone(),
                    subsets,
                    redefines,
                });
                let field_identity = format!("ix://{package}/{name}/{field_name}");
                fields.push(json!({
                    "identity": field_identity,
                    "name": field_name,
                    "typeRef": type_reference.unwrap_or_else(|| type_ref(&type_name, package)),
                    "presence": if optional {"optional"} else {"required"},
                    "nullable": false,
                    "defaultKind": "none",
                    "multiplicity": {"lower": lower, "upper": upper, "ordered": ordered, "unique": unique},
                    "origin": source_origin(parsed, Span { start: member_start, end: tokens[index - 1].span.end }),
                }));
            }
        }
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    let object_identity = format!("ix://{package}/{name}");
    let wire = json!({
        "identity": object_identity,
        "displayName": object_identity,
        "kind": {"module": package, "name": "object_type"},
        "roles": [],
        "origin": source_origin(parsed, Span { start, end }),
        "constraints": [], "extensions": [], "unknownPolicy": "reject",
        "supertypes": supertypes.iter().map(|(supertype, _)| format!("ix://{package}/{supertype}")).collect::<Vec<_>>(),
        "fields": fields, "operations": operations, "relationships": relationships,
        "identityFields": identity_fields.iter().map(|field| format!("ix://{package}/{name}/{field}")).collect::<Vec<_>>(),
        "clauses": clauses,
    });
    Some((
        ObjectRead {
            links: ObjectLinks {
                name: name.clone(),
                span: Span { start, end },
                type_index: 0,
                supertypes,
                fields: field_links,
                operations: operation_links,
            },
            name,
            name_span,
            wire,
            additional_types,
            relationship_targets,
        },
        index,
    ))
}

fn read_scalar_alias(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, String, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != "=" {
        return None;
    }
    index += 1;
    let native = tokens.get(index)?.text.as_str();
    let scalar = match native {
        "Int" | "Integer" => "integer",
        "Boolean" => "boolean",
        "Text" => "text",
        _ => return None,
    };
    index += 1;
    let mut bounds = Vec::new();
    if tokens.get(index).is_some_and(|token| token.text == "[") {
        index += 1;
        let lower = signed_integer(tokens, &mut index)?;
        if tokens.get(index)?.text != "," {
            return None;
        }
        index += 1;
        let upper = signed_integer(tokens, &mut index)?;
        if tokens.get(index)?.text != "]" {
            return None;
        }
        index += 1;
        bounds.extend([("min", lower), ("max", upper)]);
    }
    if tokens.get(index)?.text != ";" {
        return None;
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    let identity = format!("ix://{package}/{name}");
    let applies_to = match native {
        "Int" | "Integer" => "ix://filament/native/Integer",
        "Boolean" => "ix://filament/native/Boolean",
        "Text" => "ix://filament/native/Text",
        _ => return None,
    };
    let constraints = bounds
        .into_iter()
        .map(|(keyword, value)| {
            let constraint_identity = format!("{identity}/constraints/{keyword}");
            json!({
                "identity": constraint_identity,
                "keyword": keyword,
                "operands": {"value": value},
                "appliesTo": applies_to,
                "diagnosticCode": format!("bound.{keyword}"),
                "origin": source_origin(parsed, Span { start, end }),
            })
        })
        .collect::<Vec<_>>();
    Some((
        json!({
            "identity": identity,
            "displayName": identity,
            "kind": {"module": package, "name": "value_type"},
            "origin": source_origin(parsed, Span { start, end }),
            "extensions": [], "unknownPolicy": "reject", "scalar": scalar,
            "constraints": constraints,
        }),
        name,
        index,
    ))
}

fn signed_integer(tokens: &[Token], index: &mut usize) -> Option<String> {
    let negative = tokens.get(*index)?.text == "-";
    if negative {
        *index += 1;
    }
    let digits = tokens.get(*index)?.text.as_str();
    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    *index += 1;
    Some(if negative {
        format!("-{digits}")
    } else {
        digits.to_owned()
    })
}

fn read_variant_type(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, String, usize)> {
    let start = tokens.get(index)?.span.start;
    if tokens.get(index)?.text == "ordered" {
        index += 1;
    }
    let kind = tokens.get(index)?.text.as_str();
    if kind != "enum" && kind != "union" {
        return None;
    }
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != "{" {
        return None;
    }
    index += 1;
    let mut variants = Vec::new();
    while tokens.get(index)?.text != "}" {
        if tokens.get(index)?.text == "," {
            index += 1;
            continue;
        }
        let case_start = tokens.get(index)?.span.start;
        let case_name = tokens.get(index)?.text.clone();
        index += 1;
        let mut payload_type = None;
        if kind == "union" && tokens.get(index).is_some_and(|token| token.text == "(") {
            index += 1;
            let payload = tokens.get(index)?.text.clone();
            index += 1;
            while tokens.get(index)?.text != ")" {
                index += 1;
            }
            let mut reference = type_ref(&payload, package);
            if matches!(payload.as_str(), "Int" | "Integer") {
                reference = "ix://filament/native/Integer".to_owned();
            }
            payload_type = Some(reference);
            index += 1;
        }
        if kind == "enum" && tokens.get(index).is_some_and(|token| token.text == "=") {
            index += 2;
        }
        let case_end = tokens
            .get(index.saturating_sub(1))
            .map_or(case_start, |token| token.span.end);
        let case_identity = format!("ix://{package}/variant/{name}-{case_name}");
        let mut case = json!({
            "identity": case_identity,
            "name": case_name,
            "origin": source_origin(parsed, Span { start: case_start, end: case_end }),
        });
        if let Some(payload_type) = payload_type {
            case["payloadType"] = json!(payload_type);
        }
        variants.push(case);
        if tokens.get(index).is_some_and(|token| token.text == ",") {
            index += 1;
        }
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    let identity = format!("ix://{package}/{name}");
    Some((
        json!({
            "identity": identity,
            "displayName": name,
            "kind": {"module": package, "name": "variant_type"},
            "roles": [], "origin": source_origin(parsed, Span { start, end }),
            "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "variants": variants,
        }),
        name,
        index,
    ))
}

fn read_map_type(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, String, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != "=" || tokens.get(index + 1)?.text != "Map" {
        return None;
    }
    index += 2;
    if tokens.get(index)?.text != "<" {
        return None;
    }
    index += 1;
    let keys = tokens.get(index)?.text.clone();
    index += 1;
    let mut nesting = 0usize;
    while let Some(token) = tokens.get(index) {
        match token.text.as_str() {
            "<" | "[" => nesting += 1,
            ">" | "]" if nesting > 0 => nesting -= 1,
            "," if nesting == 0 => break,
            _ => {}
        }
        index += 1;
    }
    index += 1;
    let values = tokens.get(index)?.text.clone();
    index += 1;
    nesting = 0;
    while let Some(token) = tokens.get(index) {
        match token.text.as_str() {
            "<" | "[" => nesting += 1,
            ">" if nesting == 0 => break,
            ">" | "]" if nesting > 0 => nesting -= 1,
            _ => {}
        }
        index += 1;
    }
    index += 1;
    if tokens.get(index)?.text != "[" {
        return None;
    }
    index += 1;
    let lower = signed_integer(tokens, &mut index)?.parse::<u64>().ok()?;
    if tokens.get(index)?.text != "," {
        return None;
    }
    index += 1;
    let upper = if tokens.get(index)?.text == "*" {
        index += 1;
        None
    } else {
        let value = tokens.get(index)?.text.parse::<u64>().ok()?;
        index += 1;
        Some(value)
    };
    if tokens.get(index)?.text != "]" {
        return None;
    }
    index += 1;
    if tokens.get(index)?.text != ";" {
        return None;
    }
    index += 1;
    Some((
        json!({
            "identity": format!("ix://{package}/{name}"),
            "displayName": name,
            "kind": "map",
            "roles": [], "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "keys": type_ref(&keys, package),
            "values": type_ref(&values, package),
            "bound": {"lower": lower, "upper": upper},
            "origin": source_origin(parsed, Span { start, end: tokens[index - 1].span.end }),
        }),
        name,
        index,
    ))
}

fn read_event_type(
    tokens: &[Token],
    index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(ObjectRead, usize)> {
    let occurs_index = (index..tokens.len()).find(|candidate| {
        tokens[*candidate].text == "occurs"
            && tokens[index..=*candidate]
                .iter()
                .any(|token| token.text == "{")
    })?;
    let occurrence = tokens.get(occurs_index + 1)?.text.clone();
    let (mut event, next) = read_object(tokens, index, package, parsed)?;
    let event_name = event.name.clone();
    event.wire["kind"]["name"] = json!("event_type");
    event.wire["supertypes"] = json!([]);
    event.wire["occurrenceField"] = json!(format!("ix://{package}/{event_name}/{occurrence}"));
    if let Some(object) = event.wire.as_object_mut() {
        object.remove("relationships");
    }
    Some((event, next))
}

fn read_namespace(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, String, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != "{" {
        return None;
    }
    index += 1;
    let mut members = Vec::new();
    let mut vocabulary = Vec::new();
    while tokens.get(index)?.text != "}" {
        match tokens.get(index)?.text.as_str() {
            "members" => {
                index += 1;
                while tokens.get(index)?.text != ";" {
                    if tokens.get(index)?.text != "," {
                        members.push(format!("ix://{package}/{}", tokens[index].text));
                    }
                    index += 1;
                }
                index += 1;
            }
            "term" => {
                let term_start = tokens.get(index)?.span.start;
                index += 1;
                let term = tokens.get(index)?.text.clone();
                index += 1;
                if tokens.get(index)?.text != "=" {
                    return None;
                }
                index += 1;
                let doc = serde_json::from_str::<String>(&tokens.get(index)?.text).ok()?;
                index += 1;
                if tokens.get(index)?.text != ";" {
                    return None;
                }
                let term_end = tokens.get(index)?.span.end;
                index += 1;
                vocabulary.push(json!({
                    "term": term,
                    "doc": doc,
                    "origin": source_origin(parsed, Span { start: term_start, end: term_end }),
                }));
            }
            _ => return None,
        }
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    let identity = format!("ix://{package}/{name}");
    Some((
        json!({
            "identity": identity,
            "displayName": name,
            "kind": {"module": package, "name": "namespace"},
            "roles": [],
            "origin": source_origin(parsed, Span { start, end }),
            "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "supertypes": [], "fields": [], "operations": [], "relationships": [],
            "identityFields": [], "members": members, "vocabulary": vocabulary,
        }),
        name,
        index,
    ))
}

fn read_systems_interface(
    tokens: &[Token],
    index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(ObjectRead, usize)> {
    let (mut interface, next) = read_object(tokens, index, package, parsed)?;
    interface.wire["kind"]["name"] = json!("interface");
    if let Some(fields) = interface.wire["fields"].as_array_mut() {
        for field in fields {
            field["multiplicity"]["unique"] = json!(false);
        }
    }
    interface.wire.as_object_mut()?.remove("identityFields");
    interface.wire.as_object_mut()?.remove("relationships");
    let features = interface.wire["fields"]
        .as_array()?
        .iter()
        .chain(interface.wire["operations"].as_array()?)
        .filter_map(|feature| {
            let identity = feature.get("identity")?.as_str()?;
            Some((identity.rsplit('/').next()?.to_owned(), identity.to_owned()))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let feature_order = tokens[index..next]
        .iter()
        .enumerate()
        .filter(|(offset, token)| {
            token.text == "operation"
                || tokens[index + offset + 1..next]
                    .first()
                    .is_some_and(|after| after.text == ":")
        })
        .filter_map(|(offset, token)| {
            let feature_name = if token.text == "operation" {
                tokens.get(index + offset + 1)?.text.as_str()
            } else {
                token.text.as_str()
            };
            features.get(feature_name).cloned()
        })
        .collect::<Vec<_>>();
    interface.wire["featureOrder"] = json!(feature_order);
    Some((interface, next))
}

fn read_systems_part(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, String, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != ":" {
        return None;
    }
    index += 1;
    let declared_type = tokens.get(index)?.text.clone();
    index += 1;
    let (lower, upper, ordered, unique, next) = read_multiplicity(tokens, index)?;
    index = next;
    if tokens.get(index)?.text != "in" {
        return None;
    }
    index += 1;
    let owner = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != ";" {
        return None;
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    Some((
        json!({
            "identity": format!("ix://{package}/{name}"),
            "displayName": name,
            "kind": {"module": package, "name": "part"},
            "roles": [], "origin": source_origin(parsed, Span { start, end }),
            "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "owner": format!("ix://{package}/{owner}"),
            "declaredType": format!("ix://{package}/{declared_type}"),
            "multiplicity": {"lower": lower, "upper": upper, "ordered": ordered, "unique": unique},
        }),
        name,
        index,
    ))
}

fn read_systems_port(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, String, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != ":" {
        return None;
    }
    index += 1;
    let interface_type = tokens.get(index)?.text.clone();
    index += 1;
    let (lower, upper, ordered, unique, next) = read_multiplicity(tokens, index)?;
    index = next;
    let direction = tokens.get(index)?.text.clone();
    if !matches!(direction.as_str(), "in" | "out" | "inout") {
        return None;
    }
    index += 1;
    if tokens.get(index)?.text != "on" {
        return None;
    }
    index += 1;
    let owner = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != ";" {
        return None;
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    Some((
        json!({
            "identity": format!("ix://{package}/{name}"),
            "displayName": name,
            "kind": {"module": package, "name": "port"},
            "roles": [], "origin": source_origin(parsed, Span { start, end }),
            "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "owner": format!("ix://{package}/{owner}"),
            "interfaceType": format!("ix://{package}/{interface_type}"),
            "direction": direction,
            "multiplicity": {"lower": lower, "upper": upper, "ordered": ordered, "unique": unique},
        }),
        name,
        index,
    ))
}

fn read_systems_connection(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, String, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    let _source_part = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != "." {
        return None;
    }
    index += 1;
    let source_port = tokens.get(index)?.text.clone();
    index += 1;
    let operator = tokens.get(index)?.text.clone();
    if operator != "->" && operator != "<->" {
        return None;
    }
    index += 1;
    let _target_part = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != "." {
        return None;
    }
    index += 1;
    let target_port = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != ";" {
        return None;
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    let identity = format!("ix://{package}/{name}");
    let source_end = json!({
        "type": format!("ix://{package}/{source_port}"),
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
    });
    let target_end = json!({
        "type": format!("ix://{package}/{target_port}"),
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
    });
    Some((
        json!({
            "identity": identity,
            "displayName": name,
            "kind": {"module": package, "name": "connection"},
            "roles": [], "origin": source_origin(parsed, Span { start, end }),
            "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "sourceEnd": source_end, "targetEnd": target_end,
            "flowDirection": if operator == "->" { "source-to-target" } else { "bidirectional" },
        }),
        name,
        index,
    ))
}

fn read_systems_allocation(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, String, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != "from" {
        return None;
    }
    index += 1;
    let source = tokens.get(index)?.text.clone();
    index += 1;
    let mut source_member = None;
    if tokens.get(index).is_some_and(|token| token.text == "::") {
        index += 1;
        source_member = Some(tokens.get(index)?.text.clone());
        index += 1;
    }
    if tokens.get(index)?.text != "to" {
        return None;
    }
    index += 1;
    let target = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != ";" {
        return None;
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    let source = source_member.map_or_else(
        || format!("ix://{package}/{source}"),
        |member| format!("ix://{package}/{source}/{member}"),
    );
    Some((
        json!({
            "identity": format!("ix://{package}/{name}"),
            "displayName": name,
            "kind": {"module": package, "name": "allocation"},
            "roles": [], "origin": source_origin(parsed, Span { start, end }),
            "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "sourceElement": source,
            "targetElement": format!("ix://{package}/{target}"),
        }),
        name,
        index,
    ))
}

fn read_state_machine(
    tokens: &[Token],
    index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(ObjectRead, usize)> {
    let (mut machine, next) = read_object(tokens, index, package, parsed)?;
    let name = machine.name.clone();
    let states_index = (index..next).find(|candidate| tokens[*candidate].text == "states")?;
    let mut cursor = states_index + 2;
    let mut states = Vec::new();
    while tokens.get(cursor)?.text != ")" {
        if tokens.get(cursor)?.text != "," {
            let state = tokens[cursor].text.clone();
            states.push(json!({
                "identity": format!("ix://{package}/{name}/{state}"),
                "name": state,
                "origin": source_origin(parsed, tokens[cursor].span),
            }));
        }
        cursor += 1;
    }
    let mut transitions = Vec::new();
    let mut serial = 0;
    cursor = states_index + 2;
    while cursor < next {
        if tokens[cursor].text != "transition" {
            cursor += 1;
            continue;
        }
        let start = tokens[cursor].span.start;
        cursor += 1;
        let from = tokens.get(cursor)?.text.clone();
        cursor += 1;
        if tokens.get(cursor)?.text != "->" {
            return None;
        }
        cursor += 1;
        let to = tokens.get(cursor)?.text.clone();
        cursor += 1;
        if tokens.get(cursor)?.text != "on" {
            return None;
        }
        cursor += 1;
        let trigger = tokens.get(cursor)?.text.clone();
        cursor += 1;
        let mut emits = Vec::new();
        if tokens
            .get(cursor)
            .is_some_and(|token| token.text == "emits")
        {
            cursor += 1;
            while tokens.get(cursor)?.text != ";" {
                if tokens.get(cursor)?.text != "," {
                    emits.push(format!("ix://{package}/{}", tokens[cursor].text));
                }
                cursor += 1;
            }
        }
        if tokens.get(cursor)?.text != ";" {
            return None;
        }
        let end = tokens[cursor].span.end;
        transitions.push(json!({
            "identity": format!("ix://{package}/{name}/transition{serial}"),
            "from": format!("ix://{package}/{name}/{from}"),
            "to": format!("ix://{package}/{name}/{to}"),
            "trigger": format!("ix://{package}/{name}/{trigger}"),
            "emits": emits,
            "origin": source_origin(parsed, Span { start, end }),
        }));
        serial += 1;
        cursor += 1;
    }
    machine.wire["kind"]["name"] = json!("machine");
    machine.wire["identityFields"] = json!([]);
    machine.wire["states"] = json!(states);
    machine.wire["transitions"] = json!(transitions);
    Some((machine, next))
}

fn read_process(
    tokens: &[Token],
    index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(ObjectRead, usize)> {
    let (mut process, next) = read_object(tokens, index, package, parsed)?;
    let name = process.name.clone();
    let mut steps = Vec::new();
    let mut cursor = index;
    while cursor < next {
        if tokens[cursor].text != "step" {
            cursor += 1;
            continue;
        }
        let start = tokens[cursor].span.start;
        cursor += 1;
        let step_name = tokens.get(cursor)?.text.clone();
        cursor += 1;
        if tokens.get(cursor)?.text != ":" {
            return None;
        }
        cursor += 1;
        let step_kind = match tokens.get(cursor)?.text.as_str() {
            "begin" => "command".to_owned(),
            step_kind => step_kind.to_owned(),
        };
        cursor += 1;
        let mut consumes = Vec::new();
        let mut emits = Vec::new();
        while tokens.get(cursor)?.text != ";" {
            let member = match tokens[cursor].text.as_str() {
                "consumes" => Some(&mut consumes),
                "emits" => Some(&mut emits),
                _ => None,
            };
            if let Some(values) = member {
                cursor += 1;
                while tokens.get(cursor)?.text != ";"
                    && tokens.get(cursor)?.text != "consumes"
                    && tokens.get(cursor)?.text != "emits"
                {
                    if tokens.get(cursor)?.text != "," {
                        values.push(format!("ix://{package}/{}", tokens[cursor].text));
                    }
                    cursor += 1;
                }
            } else {
                cursor += 1;
            }
        }
        let end = tokens[cursor].span.end;
        steps.push(json!({
            "identity": format!("ix://{package}/{name}/{step_name}"),
            "name": step_name,
            "stepKind": step_kind,
            "consumes": consumes,
            "emits": emits,
            "origin": source_origin(parsed, Span { start, end }),
        }));
        cursor += 1;
    }
    process.wire["kind"]["name"] = json!("process");
    process.wire["identityFields"] = if process.links.supertypes.is_empty() {
        json!(process.wire["fields"]
            .as_array()?
            .iter()
            .filter_map(|field| field.get("identity").and_then(Value::as_str))
            .map(str::to_owned)
            .collect::<Vec<_>>())
    } else {
        json!([])
    };
    process.wire["steps"] = json!(steps);
    Some((process, next))
}

fn read_store(
    tokens: &[Token],
    index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(ObjectRead, usize)> {
    let (mut store, next) = read_object(tokens, index, package, parsed)?;
    let mut cursor = index + 2;
    if tokens.get(cursor)?.text != "persists" {
        return None;
    }
    cursor += 1;
    let mut persists = Vec::new();
    while tokens.get(cursor)?.text != "{" {
        if tokens.get(cursor)?.text != "," {
            persists.push(format!("ix://{package}/{}", tokens[cursor].text));
        }
        cursor += 1;
    }
    store.wire["kind"]["name"] = json!("store");
    store.wire["identityFields"] = json!([]);
    store.wire["supertypes"] = json!([]);
    store.wire["fields"] = json!([]);
    store.wire["relationships"] = json!([]);
    store.wire["persists"] = json!(persists);
    Some((store, next))
}

fn read_dimension(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, String, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != ";" {
        return None;
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    Some((
        json!({
            "identity": format!("ix://{package}/dimension/{name}"),
            "name": name,
            "base": true,
            "terms": [],
            "origin": source_origin(parsed, Span { start, end }),
        }),
        name,
        index,
    ))
}

fn read_unit(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, String, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != ":" {
        return None;
    }
    index += 1;
    let dimension = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != "=" {
        return None;
    }
    index += 1;
    if tokens.get(index)?.text != "rational" || tokens.get(index + 1)?.text != "(" {
        return None;
    }
    index += 2;
    let numerator = signed_integer(tokens, &mut index)?;
    if tokens.get(index)?.text != "," {
        return None;
    }
    index += 1;
    let denominator = signed_integer(tokens, &mut index)?;
    if tokens.get(index)?.text != ")" {
        return None;
    }
    index += 1;
    let scale = if denominator == "1" {
        numerator
    } else {
        format!("{numerator}/{denominator}")
    };
    if tokens.get(index)?.text != ";" {
        return None;
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    Some((
        json!({
            "identity": format!("ix://{package}/unit/{name}"),
            "name": name,
            "dimension": format!("ix://{package}/dimension/{dimension}"),
            "scale": scale,
            "offset": "0",
            "origin": source_origin(parsed, Span { start, end }),
        }),
        name,
        index,
    ))
}

fn read_tuple_type(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, String, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != "(" {
        return None;
    }
    index += 1;
    let mut positions = Vec::new();
    while tokens.get(index)?.text != ")" {
        if tokens.get(index)?.text == "," {
            index += 1;
            continue;
        }
        let position_start = tokens.get(index)?.span.start;
        let type_name = tokens.get(index)?.text.clone();
        index += 1;
        let mut nesting = 0usize;
        while let Some(token) = tokens.get(index) {
            match token.text.as_str() {
                "<" | "[" => nesting += 1,
                ">" | "]" if nesting > 0 => nesting -= 1,
                "," | ")" if nesting == 0 => break,
                _ => {}
            }
            index += 1;
        }
        let position_end = tokens
            .get(index.saturating_sub(1))
            .map_or(position_start, |token| token.span.end);
        let ordinal = positions.len();
        positions.push(json!({
            "identity": format!("ix://{package}/{name}/p{ordinal}"),
            "typeRef": type_ref(&type_name, package),
            "origin": source_origin(parsed, Span { start: position_start, end: position_end }),
        }));
    }
    index += 1;
    if tokens.get(index)?.text != ";" {
        return None;
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    let identity = format!("ix://{package}/{name}");
    Some((
        json!({
            "identity": identity,
            "displayName": name,
            "kind": {"module": package, "name": "tuple_type"},
            "roles": [], "origin": source_origin(parsed, Span { start, end }),
            "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "positions": positions,
        }),
        name,
        index,
    ))
}

/// A relationship end's multiplicity; an unbounded `upper` is left out, as
/// the Semantic IR schema admits no `null` bound.
fn end_multiplicity(lower: u64, upper: Option<u64>, ordered: bool, unique: bool) -> Value {
    let mut multiplicity = json!({"lower": lower, "ordered": ordered, "unique": unique});
    if let Some(upper) = upper {
        multiplicity["upper"] = json!(upper);
    }
    multiplicity
}

fn read_multiplicity(
    tokens: &[Token],
    mut index: usize,
) -> Option<(u64, Option<u64>, bool, bool, usize)> {
    if tokens.get(index)?.text != "[" {
        return None;
    }
    index += 1;
    let lower = tokens.get(index)?.text.parse().ok()?;
    index += 1;
    if tokens.get(index)?.text != "," {
        return None;
    }
    index += 1;
    let upper = if tokens.get(index)?.text == "*" {
        None
    } else {
        Some(tokens.get(index)?.text.parse().ok()?)
    };
    index += 1;
    if tokens.get(index)?.text != "]" {
        return None;
    }
    index += 1;
    let ordered = tokens
        .get(index)
        .is_some_and(|token| token.text == "ordered");
    if ordered {
        index += 1;
    }
    let explicit_unique = tokens
        .get(index)
        .is_some_and(|token| token.text == "unique");
    if explicit_unique {
        index += 1;
    }
    let unique = true;
    Some((lower, upper, ordered, unique, index))
}

fn read_operation(
    tokens: &[Token],
    mut index: usize,
    owner: &str,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, OperationLink, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != "(" {
        return None;
    }
    index += 1;
    let mut params = Vec::new();
    while tokens.get(index)?.text != ")" {
        if tokens.get(index)?.text == "," {
            index += 1;
            continue;
        }
        let parameter_start = tokens.get(index)?.span.start;
        let parameter_name = tokens.get(index)?.text.clone();
        index += 1;
        if tokens.get(index)?.text != ":" {
            return None;
        }
        index += 1;
        let type_name = tokens.get(index)?.text.clone();
        index += 1;
        let (reference, next) = read_member_type_ref(tokens, index - 1, &type_name, package)?;
        index = next;
        let parameter_end = tokens.get(index.saturating_sub(1))?.span.end;
        params.push(json!({
            "identity": format!("ix://{package}/{owner}/{name}/{parameter_name}"),
            "name": parameter_name,
            "typeRef": reference,
            "presence": "required", "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": source_origin(parsed, Span { start: parameter_start, end: parameter_end }),
        }));
        if tokens.get(index)?.text != ")" && tokens.get(index)?.text != "," {
            return None;
        }
    }
    index += 1;
    let returns = if tokens.get(index).is_some_and(|token| token.text == ":") {
        index += 1;
        let type_name = tokens.get(index)?.text.clone();
        index += 1;
        if tokens.get(index).is_some_and(|token| token.text == "[") {
            while tokens.get(index)?.text != "]" {
                index += 1;
            }
            index += 1;
        }
        Some(json!({
        "typeRef": type_ref(&type_name, package),
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        }))
    } else {
        None
    };
    let redefines = read_member_target(tokens, &mut index)?;
    let mut modifies_receiver = Vec::new();
    let mut receiver_fields = Vec::new();
    let mut creates = Vec::new();
    let mut deletes = Vec::new();
    while tokens.get(index)?.text != ";" && tokens.get(index)?.text != "{" {
        match tokens.get(index)?.text.as_str() {
            "modifies" => {
                index += 1;
                while tokens.get(index)?.text != ";"
                    && tokens.get(index)?.text != "{"
                    && tokens.get(index)?.text != "creates"
                    && tokens.get(index)?.text != "deletes"
                {
                    if tokens[index].text == "self"
                        && tokens.get(index + 1).is_some_and(|token| token.text == ".")
                    {
                        if let Some(field) = tokens.get(index + 2) {
                            receiver_fields.push((field.text.clone(), field.span));
                            modifies_receiver
                                .push(format!("ix://{package}/{owner}/{}", field.text));
                        }
                        index += 3;
                    } else {
                        index += 1;
                    }
                }
            }
            "creates" => {
                index += 1;
                if let Some(target) = tokens.get(index) {
                    creates.push(format!("ix://{package}/{}", target.text));
                    index += 1;
                }
            }
            "deletes" => {
                index += 1;
                if let Some(target) = tokens.get(index) {
                    deletes.push(format!("ix://{package}/{}", target.text));
                    index += 1;
                }
            }
            _ => index += 1,
        }
    }
    let mut preconditions = Vec::new();
    let mut postconditions = Vec::new();
    if tokens.get(index)?.text == ";" {
        index += 1;
    } else {
        if tokens.get(index)?.text != "{" {
            return None;
        }
        index += 1;
        while tokens.get(index)?.text != "}" {
            let member = tokens.get(index)?.text.as_str();
            if member != "pre" && member != "post" {
                return None;
            }
            let (clause, next) =
                read_clause_declaration(tokens, index, &name, package, parsed, false)?;
            if member == "pre" {
                preconditions.push(clause);
            } else {
                postconditions.push(clause);
            }
            index = next;
        }
        index += 1;
    }
    let end = tokens.get(index.saturating_sub(1))?.span.end;
    let mut operation = json!({
        "identity": format!("ix://{package}/{owner}/{name}"),
        "name": name, "params": params, "pre": preconditions, "post": postconditions,
        "origin": source_origin(parsed, Span { start, end }),
        "frame": {"modifies": [], "modifies_receiver": modifies_receiver, "creates": creates, "deletes": deletes},
    });
    if let Some(returns) = returns {
        operation["returns"] = returns;
    }
    Some((
        operation,
        OperationLink {
            name,
            redefines,
            receiver_fields,
        },
        index,
    ))
}

/// An optional `redefines Owner :: member` at `index`, which it advances past.
/// `None` is a malformed target, `Some(None)` an absent one.
fn read_member_target(tokens: &[Token], index: &mut usize) -> Option<Option<MemberTarget>> {
    if tokens
        .get(*index)
        .is_none_or(|token| token.text != "redefines")
    {
        return Some(None);
    }
    let owner = tokens.get(*index + 1)?;
    if tokens.get(*index + 2)?.text != "::" {
        return None;
    }
    let member = tokens.get(*index + 3)?;
    *index += 4;
    Some(Some(MemberTarget {
        owner: owner.text.clone(),
        owner_span: owner.span,
        member: member.text.clone(),
        member_span: member.span,
    }))
}

fn read_clause_declaration(
    tokens: &[Token],
    mut index: usize,
    owner: &str,
    package: &str,
    parsed: &ParsedSource,
    include_identity: bool,
) -> Option<(Value, usize)> {
    let clause_kind = tokens.get(index)?.text.as_str();
    if !matches!(clause_kind, "invariant" | "pre" | "post") {
        return None;
    }
    let start = tokens.get(index)?.span.start;
    index += 1;
    let clause_id = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != "{" {
        return None;
    }
    index += 1;
    let text_start = tokens.get(index)?.span.start;
    let mut depth = 1usize;
    while depth > 0 {
        match tokens.get(index)?.text.as_str() {
            "{" => depth += 1,
            "}" => depth -= 1,
            _ => {}
        }
        if depth > 0 {
            index += 1;
        }
    }
    let text_end = tokens.get(index)?.span.start;
    let end = tokens.get(index)?.span.end;
    let text = parsed
        .source()
        .text()
        .get(text_start..text_end)?
        .trim()
        .to_owned();
    index += 1;
    if tokens.get(index).is_some_and(|token| token.text == ";") {
        index += 1;
    }
    let mut clause = json!({
        "language": "quire",
        "text": text,
        "origin": source_origin(parsed, Span { start, end }),
        "sourceSpan": source_span(parsed, Span { start, end }),
    });
    if include_identity {
        clause["identity"] = json!(format!("ix://{package}/{owner}/{clause_id}"));
        clause["clauseId"] = json!(clause_id);
    }
    Some((clause, index))
}

fn read_population(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, usize)> {
    let start = tokens.get(index)?.span.start;
    let open = tokens.get(index)?.text == "open";
    let symmetric = tokens[index..]
        .iter()
        .take_while(|token| token.text != "population")
        .any(|token| token.text == "symmetric");
    while tokens.get(index)?.text != "population" {
        index += 1;
    }
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != ":" {
        return None;
    }
    index += 1;
    let first_type = tokens.get(index)?.text.clone();
    let mut members = vec![first_type];
    index += 1;
    while tokens.get(index).is_some_and(|token| token.text == ",") {
        index += 1;
        members.push(tokens.get(index)?.text.clone());
        index += 1;
    }
    if tokens.get(index)?.text != ";" {
        return None;
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    let identity = format!("ix://{package}/{name}");
    Some((
        json!({
            "identity": identity, "displayName": identity,
            "kind": {"module": package, "name": "population"},
            "members": members.iter().map(|member| format!("ix://{package}/{member}")).collect::<Vec<_>>(),
        "extent": if open { "open" } else { "closed" },
        "symmetric": symmetric,
        "origin": source_origin(parsed, Span { start, end }),
        }),
        index,
    ))
}

fn skip_statement(tokens: &[Token], mut index: usize) -> Option<usize> {
    while tokens.get(index)?.text != ";" {
        index += 1;
    }
    Some(index + 1)
}

fn type_ref(type_name: &str, package: &str) -> String {
    let native = match type_name {
        "Boolean" => "Boolean",
        "Int" => "Integer",
        "Text" => "Text",
        "Integer" => "Integer",
        "Rational" => "Rational",
        "Float32" => "Float32",
        "Float64" => "Float64",
        "Timestamp" => "Timestamp",
        "UUID" => "UUID",
        "String" => "String",
        "Decimal" => "Decimal",
        "Bytes" => "Bytes",
        "Duration" => "Duration",
        "JsonObject" => "JsonObject",
        "Clock" => "Clock",
        _ => return format!("ix://{package}/{type_name}"),
    };
    format!("ix://filament/native/{native}")
}

fn read_member_type_ref(
    tokens: &[Token],
    index: usize,
    type_name: &str,
    package: &str,
) -> Option<(String, usize)> {
    if type_name == "Reference" && tokens.get(index + 1)?.text == "<" {
        let target = tokens.get(index + 2)?.text.as_str();
        if tokens.get(index + 3)?.text != ">" {
            return None;
        }
        return Some((type_ref(target, package), index + 4));
    }
    Some((type_ref(type_name, package), index + 1))
}

fn source_origin(parsed: &ParsedSource, span: Span) -> Value {
    let source = parsed.source();
    let position = source.locate(Span {
        end: span.end,
        ..span
    });
    let (line, column) = position.map_or((1, 1), |position| {
        (position.start.line, position.start.column)
    });
    json!({"source": {
        "sourceIdentity": format!("ix://{}/{}", source.identity().authority, source.identity().identity),
        "path": source.path(), "startLine": line, "startColumn": column,
    }})
}

fn source_span(parsed: &ParsedSource, span: Span) -> Value {
    let source = parsed.source();
    let position = source.locate(Span {
        end: span.end,
        ..span
    });
    let (line, column) = position.map_or((1, 1), |position| {
        (position.start.line, position.start.column)
    });
    let (end_line, end_column) =
        position.map_or((1, 1), |position| (position.end.line, position.end.column));
    json!({
        "sourceIdentity": format!("ix://{}/{}", source.identity().authority, source.identity().identity),
        "path": source.path(), "startLine": line, "startColumn": column,
        "endLine": end_line, "endColumn": end_column,
    })
}

fn diagnostic(
    parsed: &ParsedSource,
    code: CompleteCode,
    cause: CompleteCause,
    span: Span,
    message: impl Into<String>,
) -> CompleteDiagnostic {
    *cst::error(
        parsed.source(),
        code,
        cause,
        Phase::Parse,
        span.start,
        span.end,
        message,
    )
}

fn project_diagnostic(source: &Source, diagnostic: &CompleteDiagnostic) -> Diagnostic {
    let code = Code::from_code(diagnostic.code.as_str()).unwrap_or(Code::InvalidModelBinding);
    let span = diagnostic.byte_span().unwrap_or(Span { start: 0, end: 0 });
    *fsl_foundation::diagnostic::error(
        source,
        code,
        diagnostic.phase,
        span.start,
        span.end,
        diagnostic.message.clone(),
    )
}
