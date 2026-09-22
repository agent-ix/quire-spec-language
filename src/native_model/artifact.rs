// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-015: exact, bounded native model artifact encoding over existing IR bytes.

use std::io::{self, Write};

use quire_contract_ir as ir;
use serde::Serialize;

use super::{failure, NativeModelProfile, NativeRoles, ScalarKind, ScalarSite, Unit};
use crate::{formal_source::FormalSource, Code, Diagnostic};

type Result<T> = std::result::Result<T, Box<Diagnostic>>;

struct StringBudget<'a> {
    source: &'a FormalSource,
    remaining: usize,
}

impl StringBudget<'_> {
    fn text(&mut self, text: &str) -> Result<()> {
        self.remaining = self.remaining.checked_sub(text.len()).ok_or_else(|| {
            failure(
                self.source,
                Code::ResourceExhausted,
                "model string content exceeds artifact limit",
            )
        })?;
        Ok(())
    }

    fn span(&mut self, span: &ir::SourceSpan) -> Result<()> {
        // Every serialized span contains this identity at both endpoints.
        self.text(span.source().document().as_str())
    }

    fn value_type(&mut self, ty: &ir::ValueType) -> Result<()> {
        match ty {
            ir::ValueType::Record { name } | ir::ValueType::Enum { name } => {
                self.text(name.as_str())
            }
            ir::ValueType::Option { value } => self.value_type(value),
            ir::ValueType::Collection { value } => self.value_type(value.element()),
            ir::ValueType::Boolean
            | ir::ValueType::Text
            | ir::ValueType::Integer { .. }
            | ir::ValueType::Rational { .. } => Ok(()),
        }
    }
}

pub(super) fn check_string_content(
    source: &FormalSource,
    environment: &ir::DeclarationEnvironment,
    roles: &NativeRoles,
    maximum: usize,
) -> Result<()> {
    let mut budget = StringBudget {
        source,
        remaining: maximum,
    };
    budget.text(&source.source().identity().identity)?;
    budget.text(&source.source().identity().revision)?;
    budget.text(source.identity().document().as_str())?;
    budget.text(environment.owner().package().as_str())?;
    budget.text(environment.owner().requirement().as_str())?;
    for declaration in environment.types() {
        budget.text(declaration.name().as_str())?;
        budget.span(declaration.source())?;
        match declaration {
            ir::TypeDeclaration::Record { declaration } => {
                for field in declaration.fields() {
                    budget.text(field.name().as_str())?;
                    budget.span(field.source())?;
                    budget.value_type(field.value_type())?;
                }
            }
            ir::TypeDeclaration::Enum { declaration } => {
                for variant in declaration.variants() {
                    budget.text(variant.name().as_str())?;
                    budget.span(variant.source())?;
                }
            }
        }
    }
    for value in environment.values() {
        budget.text(value.name().as_str())?;
        budget.span(value.source())?;
        budget.value_type(value.value_type())?;
    }
    for scalar in &roles.scalars {
        budget.text(scalar.name.as_str())?;
        budget.span(&scalar.source)?;
        if let ScalarKind::Integer {
            unit: Unit::Named(name),
        }
        | ScalarKind::Rational {
            unit: Unit::Named(name),
        } = &scalar.kind
        {
            budget.text(name.as_str())?;
        }
        for site in &scalar.sites {
            match site {
                ScalarSite::Value { name } => budget.text(name.as_str())?,
                ScalarSite::Field { record, field } => {
                    budget.text(record.as_str())?;
                    budget.text(field.as_str())?;
                }
            }
        }
    }
    for object in &roles.objects {
        for name in [
            &object.record,
            &object.reference,
            &object.identity_field,
            &object.universe,
        ] {
            budget.text(name.as_str())?;
        }
        budget.span(&object.source)?;
    }
    for operation in &roles.operations {
        budget.text(operation.context.as_str())?;
        budget.text(operation.name.as_str())?;
        budget.text(operation.anchor.as_str())?;
        budget.span(&operation.source)?;
        for name in operation
            .parameters
            .iter()
            .chain(operation.result.iter())
            .chain(&operation.frame.created)
            .chain(&operation.frame.deleted)
        {
            budget.text(name.as_str())?;
        }
        for (record, field) in &operation.frame.fields {
            budget.text(record.as_str())?;
            budget.text(field.as_str())?;
        }
    }
    Ok(())
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Path<'a> {
    Type {
        name: &'a ir::SymbolName,
    },
    Field {
        record: &'a ir::SymbolName,
        field: &'a ir::SymbolName,
    },
    Value {
        name: &'a ir::SymbolName,
    },
    Variant {
        enumeration: &'a ir::SymbolName,
        variant: &'a ir::SymbolName,
    },
}

#[derive(Serialize)]
struct Locus<'a> {
    key: Path<'a>,
    source: &'a ir::SourceSpan,
}

fn loci(environment: &ir::DeclarationEnvironment) -> Vec<Locus<'_>> {
    let mut loci = Vec::new();
    for declaration in environment.types() {
        loci.push(Locus {
            key: Path::Type {
                name: declaration.name(),
            },
            source: declaration.source(),
        });
        match declaration {
            ir::TypeDeclaration::Record { declaration } => {
                loci.extend(declaration.fields().iter().map(|field| Locus {
                    key: Path::Field {
                        record: declaration.name(),
                        field: field.name(),
                    },
                    source: field.source(),
                }));
            }
            ir::TypeDeclaration::Enum { declaration } => {
                loci.extend(declaration.variants().iter().map(|variant| Locus {
                    key: Path::Variant {
                        enumeration: declaration.name(),
                        variant: variant.name(),
                    },
                    source: variant.source(),
                }));
            }
        }
    }
    loci.extend(environment.values().iter().map(|value| Locus {
        key: Path::Value { name: value.name() },
        source: value.source(),
    }));
    loci.sort_by(|a, b| a.key.cmp(&b.key));
    loci
}

#[derive(Serialize)]
struct ModelSource<'a> {
    identity: &'a str,
    revision: &'a str,
    digest: String,
    formal: &'a ir::SourceIdentity,
}

#[derive(Serialize)]
struct Artifact<'a> {
    profile: &'static str,
    declarations: &'a str,
    source: ModelSource<'a>,
    loci: Vec<Locus<'a>>,
    roles: &'a NativeRoles,
}

struct BoundedBytes {
    bytes: Vec<u8>,
    maximum: usize,
}

impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let fits = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_some_and(|length| length <= self.maximum);
        if !fits {
            return Err(io::Error::other("model artifact content limit exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn encode(
    profile: NativeModelProfile,
    source: &FormalSource,
    environment: &ir::DeclarationEnvironment,
    roles: &NativeRoles,
    maximum: usize,
) -> Result<Vec<u8>> {
    let formal_limit = u64::try_from(maximum).map_err(|_| {
        failure(
            source,
            Code::ResourceExhausted,
            "model artifact limit cannot be represented",
        )
    })?;
    let canonical = environment
        .canonical_declaration_with_limit(ir::CanonicalProfile::V1, formal_limit)
        .map_err(|upstream| {
            let code = if upstream.code == ir::DiagnosticCode::CanonicalizationResourceExhausted {
                Code::ResourceExhausted
            } else {
                Code::InvalidModelBinding
            };
            failure(
                source,
                code,
                format!("formal declaration canonicalization failed: {upstream}"),
            )
        })?;
    let declarations = std::str::from_utf8(canonical.bytes().as_slice()).map_err(|_| {
        failure(
            source,
            Code::InvalidModelBinding,
            "formal canonical declaration was not UTF-8",
        )
    })?;
    let artifact = Artifact {
        profile: profile.as_str(),
        declarations,
        source: ModelSource {
            identity: &source.source().identity().identity,
            revision: &source.source().identity().revision,
            digest: source.source().digest().to_string(),
            formal: source.identity(),
        },
        loci: loci(environment),
        roles,
    };
    let mut output = BoundedBytes {
        bytes: Vec::new(),
        maximum,
    };
    serde_json::to_writer(&mut output, &artifact).map_err(|_| {
        failure(
            source,
            Code::ResourceExhausted,
            "native model artifact content limit exceeded",
        )
    })?;
    Ok(output.bytes)
}
