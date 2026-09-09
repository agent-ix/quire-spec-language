// SPDX-License-Identifier: AGPL-3.0-only
//! FR-030: consume actual Quire extraction and verify its original native body.
//! Quire retains Markdown, availability and schema ownership; native semantics
//! remain in the existing mapped compiler. Enabled by `quire-extraction`.

use quire_contract_ir as ir;
use quire_rs::semantic::{extract_clauses, AvailabilityState, ClausesOutcome, SemanticContext};

use crate::checking::ClauseBinding;
use crate::mapped::{self, CompileError, CompileLimits, MappedPackage};
use crate::native_model::NativeModel;
use crate::source_map::{Layout, Segment, SourceMap};
use crate::{Code, Diagnostic, Phase, Source, SourceIdentity, Span};

/// Explicit authored selection and independently assigned native/formal body identities.
#[derive(Clone, Debug)]
pub struct Selection {
    /// Authored clause ID selects the Quire heading; name selects the native clause.
    pub binding: ClauseBinding,
    /// Caller-assigned identity/revision for the extracted body, distinct from the original.
    pub body: SourceIdentity,
    /// Explicit Contract IR identity for the body; no revision-string conversion.
    pub formal: ir::SourceIdentity,
}

/// Pre-extraction input bounds and the unchanged native compiler stage limits.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Original document bytes; hard maximum 1 MiB.
    pub source_bytes: usize,
    /// Original lines, including trailing empty line; hard maximum 4096.
    pub lines: usize,
    /// Independent caller-lowered native compiler stages.
    pub compiler: CompileLimits,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            source_bytes: 1_048_576,
            lines: 4096,
            compiler: CompileLimits::default(),
        }
    }
}

/// Actual failed join or native stage, without parsing display messages.
#[derive(Debug, thiserror::Error)]
pub enum Cause {
    /// Original selection/correspondence diagnostic.
    #[error("{0}")]
    Join(#[from] Box<Diagnostic>),
    /// Complete original mapped compiler failure.
    #[error("{0}")]
    Compile(#[from] Box<CompileError>),
}

/// A failed request preserves its source, selection and any completed extraction.
#[derive(Debug, thiserror::Error)]
#[error("{cause}")]
pub struct Error {
    original: Source,
    selection: Selection,
    extraction: Option<ClausesOutcome>,
    /// Actual failure with no partial native package.
    #[source]
    pub cause: Cause,
}

impl Error {
    /// Original immutable document, including exact byte digest.
    pub fn original(&self) -> &Source {
        &self.original
    }

    /// The caller's authored/native/formal selection, including on preflight refusal.
    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    /// Unchanged upstream result; absent when preflight prevented extraction.
    pub fn extraction(&self) -> Option<&ClausesOutcome> {
        self.extraction.as_ref()
    }

    /// Code of the stage that actually failed.
    pub fn code(&self) -> Code {
        match &self.cause {
            Cause::Join(error) => error.code,
            Cause::Compile(error) => error.code(),
        }
    }
}

/// Successful native compilation with complete, unchanged upstream extraction data.
#[derive(Debug)]
pub struct ExtractedPackage<'model> {
    mapped: MappedPackage<'model>,
    extraction: ClausesOutcome,
}

impl<'model> ExtractedPackage<'model> {
    /// Verified original/body correspondence and the actual native package.
    pub fn mapped(&self) -> &MappedPackage<'model> {
        &self.mapped
    }

    /// Original clause population, availability and diagnostics, including unchecked tags.
    pub fn extraction(&self) -> &ClausesOutcome {
        &self.extraction
    }
}

fn failure(source: &Source, code: Code, message: &str) -> Box<Diagnostic> {
    crate::diagnostic::error(source, code, Phase::SourceMap, 0, 0, message)
}

fn preflight(
    original: &Source,
    context: &SemanticContext,
    selection: &Selection,
    limits: Limits,
) -> Result<(), Box<Diagnostic>> {
    let hard = Limits::default();
    if original.text().len() > limits.source_bytes.min(hard.source_bytes)
        || original
            .position(original.text().len())
            .is_none_or(|position| position.line > limits.lines.min(hard.lines))
    {
        return Err(failure(
            original,
            Code::ResourceExhausted,
            "Quire extraction input limit exceeded",
        ));
    }
    if context.module.contract_version != "1.0.0" || context.module.semantic_core != "0.1.0" {
        return Err(failure(
            original,
            Code::UnknownProfile,
            "Quire consumer requires contract 1.0.0 / semantic-core 0.1.0",
        ));
    }
    if context.source_identity.as_deref() != Some(original.identity().identity.as_str())
        || context.path != original.path()
        || context.identity_package() != selection.binding.requirement.package().as_str()
    {
        return Err(failure(
            original,
            Code::InvalidModelBinding,
            "Quire context differs from the selected source or authored package",
        ));
    }
    Ok(())
}

fn body_map(
    original: &Source,
    extraction: &ClausesOutcome,
    selection: &Selection,
    byte_limit: usize,
) -> Result<(SourceMap, String), Box<Diagnostic>> {
    let invalid = |message| failure(original, Code::InvalidSourceMap, message);
    if extraction.availability.state != AvailabilityState::Available {
        return Err(failure(
            original,
            Code::InvalidModelBinding,
            "Quire clause extraction is not available",
        ));
    }
    let clause = extraction
        .clauses
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .find(|clause| clause.clause_id == selection.binding.clause.as_str())
        .ok_or_else(|| {
            failure(
                original,
                Code::InvalidModelBinding,
                "selected authored clause is absent from Quire extraction",
            )
        })?;
    let locus = clause
        .source_span
        .as_ref()
        .ok_or_else(|| invalid("extracted clause has no original locus"))?;
    let extracted_text = extraction
        .clause_text
        .get(&clause.clause_id)
        .ok_or_else(|| invalid("extracted clause has no body"))?;
    let close = locus
        .end_line
        .ok_or_else(|| invalid("extracted fence has no closing line"))?;
    if locus.source_identity != original.identity().identity
        || locus.path != original.path()
        || locus.start_column != 1
        || locus.start_line >= close
    {
        return Err(invalid(
            "extracted fence does not belong to the selected source",
        ));
    }
    let start = locus
        .start_line
        .checked_add(1)
        .and_then(|line| original.line_start(line))
        .ok_or_else(|| invalid("opening fence is outside the source"))?;
    let end = original
        .line_start(close)
        .ok_or_else(|| invalid("closing fence is outside the source"))?;
    let next = close
        .checked_add(1)
        .and_then(|line| original.line_start(line))
        .unwrap_or(original.text().len());
    let closing_line = original
        .slice(Span {
            start: end,
            end: next,
        })
        .ok_or_else(|| invalid("invalid closing line range"))?;
    // Quire FR-071 defines endColumn as one past the closing line's byte
    // length. Native diagnostics use scalar columns; do not compare those units.
    let closing_content = closing_line
        .strip_suffix('\n')
        .unwrap_or(closing_line)
        .trim_end_matches('\r');
    if closing_content.len().checked_add(1) != locus.end_column {
        return Err(invalid(
            "closing fence coordinate disagrees with original bytes",
        ));
    }
    let extracted_end = start
        .checked_add(extracted_text.len())
        .ok_or_else(|| invalid("extracted body coordinate overflow"))?;
    if original.slice(Span {
        start,
        end: extracted_end,
    }) != Some(extracted_text)
        || !matches!(
            original.slice(Span {
                start: extracted_end,
                end
            }),
            Some("" | "\n")
        )
    {
        return Err(invalid(
            "Quire body differs from the reported original region",
        ));
    }
    // Quire omits the final LF but preserves a preceding CR. Having checked its
    // exact bytes above, remove only that dangling CR so native syntax sees the
    // declared final-newline deletion, never an inserted LF or an invalid bare CR.
    let text = if original.slice(Span {
        start: extracted_end,
        end,
    }) == Some("\n")
    {
        extracted_text.strip_suffix('\r').unwrap_or(extracted_text)
    } else {
        extracted_text.as_str()
    };
    let body = Source::read(
        selection.body.clone(),
        format!("{}#{}", original.path(), clause.clause_id),
        text.as_bytes(),
        byte_limit,
    )?;
    let segments = if text.is_empty() {
        Vec::new()
    } else {
        vec![Segment {
            body: Span {
                start: 0,
                end: text.len(),
            },
            original: Span {
                start,
                end: start
                    .checked_add(text.len())
                    .ok_or_else(|| invalid("body coordinate overflow"))?,
            },
        }]
    };
    let mapping = SourceMap::verify(
        original.clone(),
        body,
        Span { start, end },
        segments,
        Layout {
            drop_final_newline: true,
            ..Layout::default()
        },
        1,
    )?;
    Ok((mapping, clause.language.clone()))
}

/// Extract from the original document, verify the selected body and compile it.
///
/// The caller must select/verify original bytes and load its Quire context before
/// this call. No source text, availability result or authored binding is inferred
/// from a successful native parse. Each request starts fresh extraction/compiler limits.
pub fn compile<'model>(
    original: Source,
    context: &SemanticContext,
    selection: Selection,
    models: &'model [NativeModel],
    limits: Limits,
) -> Result<ExtractedPackage<'model>, Box<Error>> {
    if let Err(cause) = preflight(&original, context, &selection, limits) {
        return Err(Box::new(Error {
            original,
            selection,
            extraction: None,
            cause: Cause::Join(cause),
        }));
    }
    let extraction = extract_clauses(original.text(), context);
    let join = || -> Result<MappedPackage<'model>, Cause> {
        let (mapping, language) = body_map(
            &original,
            &extraction,
            &selection,
            limits.compiler.syntax.source_bytes,
        )?;
        Ok(mapped::compile(
            mapping,
            &language,
            selection.binding.clone(),
            selection.formal.clone(),
            models,
            limits.compiler,
        )?)
    };
    match join() {
        Ok(mapped) => Ok(ExtractedPackage { mapped, extraction }),
        Err(cause) => Err(Box::new(Error {
            original,
            selection,
            extraction: Some(extraction),
            cause,
        })),
    }
}
