// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-030: consume actual Quire extraction and verify its original native body.
//! Quire retains Markdown, availability and schema ownership; native semantics
//! remain in the existing mapped compiler. Enabled by `quire-extraction`.

use quire_rs::semantic::{extract_clauses, AvailabilityState, ClauseRef, SourceLocus};
/// Pinned Quire-owned input/result contracts deliberately exposed by this feature.
/// Changes to their upstream shape require consumer compatibility review.
pub use quire_rs::semantic::{ClausesOutcome, SemanticContext};

mod preflight;
pub use preflight::PreflightFailure;

/// Quire module contract admitted by this consumer and command adapters.
pub const CONTRACT_VERSION: &str = "1.0.0";
/// Quire semantic-core contract admitted by this consumer and command adapters.
pub const SEMANTIC_CORE_VERSION: &str = "0.1.0";
/// Hard original-document byte ceiling before extraction.
pub const MAX_SOURCE_BYTES: usize = 1_048_576;
/// Hard original-document line ceiling, including trailing empty line.
pub const MAX_LINES: usize = 4096;

use crate::checking::ClauseBinding;
use crate::formal_source::SourceIdentities;
use crate::mapped::{self, CompileError, CompileLimits, MappedPackage};
use crate::native_model::NativeModel;
use qsl_foundation::source_map::{Layout, Segment, SourceMap};
use qsl_foundation::{Code, Diagnostic, Phase, Source, Span};

/// Explicit authored selection and independently assigned native/formal body identities.
#[derive(Clone, Debug)]
pub struct Selection {
    /// Authored clause ID selects the Quire heading; name selects the native clause.
    pub binding: ClauseBinding,
    /// Caller-assigned identity/revision for the extracted body, distinct from the original.
    pub body: SourceIdentities,
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
            source_bytes: MAX_SOURCE_BYTES,
            lines: MAX_LINES,
            compiler: CompileLimits::default(),
        }
    }
}

/// Actual failed join or native stage, without parsing display messages.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Cause {
    /// Input limits, profile selection or original-context mismatch before extraction.
    #[error("{0}")]
    Preflight(#[from] Box<PreflightFailure>),
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
            Cause::Preflight(error) => error.code(),
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
    qsl_foundation::diagnostic::error(source, code, Phase::SourceMap, 0, 0, message)
}

fn selected_clause<'a>(
    original: &Source,
    extraction: &'a ClausesOutcome,
    selection: &Selection,
) -> Result<&'a ClauseRef, Box<Diagnostic>> {
    if extraction.availability.state != AvailabilityState::Available {
        return Err(failure(
            original,
            Code::InvalidModelBinding,
            "Quire clause extraction is not available",
        ));
    }
    // The pre-extraction line ceiling bounds the entire upstream clause population.
    extraction
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
        })
}

fn locate_fence(original: &Source, locus: &SourceLocus) -> Result<Span, Box<Diagnostic>> {
    let invalid = |message| failure(original, Code::InvalidSourceMap, message);
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
    // Quire FR-071 uses byte columns here; native diagnostic columns count scalars.
    let closing_content = closing_line
        .strip_suffix('\n')
        .unwrap_or(closing_line)
        .trim_end_matches('\r');
    if closing_content.len().checked_add(1) != locus.end_column {
        return Err(invalid(
            "closing fence coordinate disagrees with original bytes",
        ));
    }
    Ok(Span { start, end })
}

fn verify_body_bytes<'a>(
    original: &Source,
    region: Span,
    extracted: &'a str,
) -> Result<&'a str, Box<Diagnostic>> {
    let invalid = |message| failure(original, Code::InvalidSourceMap, message);
    let extracted_end = region
        .start
        .checked_add(extracted.len())
        .ok_or_else(|| invalid("extracted body coordinate overflow"))?;
    let selected = original.slice(Span {
        start: region.start,
        end: extracted_end,
    });
    let suffix = original.slice(Span {
        start: extracted_end,
        end: region.end,
    });
    if selected != Some(extracted) || !matches!(suffix, Some("" | "\n")) {
        return Err(invalid(
            "Quire body differs from the reported original region",
        ));
    }
    // Verify Quire's bytes before dropping the CR left by its omitted final LF.
    Ok(if suffix == Some("\n") {
        extracted.strip_suffix('\r').unwrap_or(extracted)
    } else {
        extracted
    })
}

fn build_map(
    original: &Source,
    clause: &ClauseRef,
    selection: &Selection,
    region: Span,
    text: &str,
    byte_limit: usize,
) -> Result<SourceMap, Box<Diagnostic>> {
    let body = Source::read(
        selection.body.native.clone(),
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
                start: region.start,
                end: region.start.checked_add(text.len()).ok_or_else(|| {
                    failure(original, Code::InvalidSourceMap, "body coordinate overflow")
                })?,
            },
        }]
    };
    SourceMap::verify(
        original.clone(),
        body,
        region,
        segments,
        Layout {
            drop_final_newline: true,
            ..Layout::default()
        },
        1,
    )
}

fn body_map<'a>(
    original: &Source,
    extraction: &'a ClausesOutcome,
    selection: &Selection,
    byte_limit: usize,
) -> Result<(SourceMap, &'a str), Box<Diagnostic>> {
    let invalid = |message| failure(original, Code::InvalidSourceMap, message);
    let clause = selected_clause(original, extraction, selection)?;
    let locus = clause
        .source_span
        .as_ref()
        .ok_or_else(|| invalid("extracted clause has no original locus"))?;
    let extracted = extraction
        .clause_text
        .get(&clause.clause_id)
        .ok_or_else(|| invalid("extracted clause has no body"))?;
    let region = locate_fence(original, locus)?;
    let text = verify_body_bytes(original, region, extracted)?;
    Ok((
        build_map(original, clause, selection, region, text, byte_limit)?,
        &clause.language,
    ))
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
    if let Err(cause) = preflight::check(&original, context, &selection, limits) {
        return Err(Box::new(Error {
            original,
            selection,
            extraction: None,
            cause: Cause::Preflight(cause),
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
            language,
            selection.binding.clone(),
            selection.body.formal.clone(),
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
