// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-source`: the ADR-011 §6.1 layer **I3** crate (ADR-011 §7.3 X-4).
//!
//! ADR-011 §2.1 I3: verify Quire's reported extraction against the original document
//! and yield S0 bytes plus a document `SourceMap`. Quire retains Markdown, availability
//! and schema ownership. This adapter never reaches the native compiler; the join lives
//! at the SEAM-1 caller (the root crate's `command::extraction`). Every dependency,
//! `qsl-foundation` (layer F) and quire-rs among them, is behind this crate's
//! `quire-extraction` feature. Without that feature the crate is empty.
//!
//! This crate is the root crate's only code path to quire-rs (ADR-011 §6.1 layer-6
//! row): the root crate names no quire-rs dependency. quire-rs is still built
//! transitively through FCD's `agent-ix-extraction-frontend` (`model::intake`).

#![cfg(feature = "quire-extraction")]

use quire_rs::semantic::{extract_clauses, SourceLocus};
use quire_rs::semantic::{read_semantic_block, BundleIndex};
/// Pinned Quire-owned input/result contracts deliberately exposed by this crate.
/// Changes to their upstream shape require consumer compatibility review.
/// `ClauseRef`, `KindAvailability`, `SemanticDiagnostic` and `SemanticFailure` are
/// the field and failure types of `ClausesOutcome` and of [`clause_context`]; the root
/// crate's extracted `run` command renders Quire's result through them unchanged.
pub use quire_rs::semantic::{
    AvailabilityState, ClauseRef, ClausesOutcome, KindAvailability, SemanticContext,
    SemanticDiagnostic, SemanticFailure,
};

mod preflight;
pub use preflight::PreflightFailure;

/// Quire module contract admitted by this consumer and command adapters.
pub const CONTRACT_VERSION: &str = "1.0.0";
/// Quire semantic-core contract admitted by this consumer and command adapters.
pub const SEMANTIC_CORE_VERSION: &str = "0.1.0";
/// Default original-document byte ceiling before extraction; a caller's
/// [`Limits::source_bytes`] is used as given (NFR-001).
pub const MAX_SOURCE_BYTES: usize = 1_048_576;
/// Default original-document line ceiling, including the trailing empty
/// line; a caller's [`Limits::lines`] is used as given (NFR-001).
pub const MAX_LINES: usize = 4096;

use qsl_foundation::source_map::{Layout, Segment, SourceMap};
use qsl_foundation::{Code, Diagnostic, Phase, Source, SourceIdentity, Span};

/// Authored clause selection and the caller-assigned extracted-body identity.
///
/// This is the I3-level selection: the SEAM-1 caller resolves the full authored
/// `ClauseBinding` (name, requirement, execution point) for its own native join and
/// hands this adapter only the two fields it verifies against Quire's extraction.
#[derive(Clone, Debug)]
pub struct Selection {
    /// Authored clause ID selects the Quire heading.
    pub clause_id: String,
    /// Authored requirement's owning package, checked against the Quire context.
    pub package: String,
    /// Caller-assigned identity/revision for the extracted body, distinct from the original.
    pub body: SourceIdentity,
}

/// Pre-extraction input bounds. Each field is enforced exactly as the
/// caller supplies it, above or below [`Limits::default`].
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Original document bytes; defaults to [`MAX_SOURCE_BYTES`].
    pub source_bytes: usize,
    /// Original lines, including trailing empty line; defaults to
    /// [`MAX_LINES`].
    pub lines: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            source_bytes: MAX_SOURCE_BYTES,
            lines: MAX_LINES,
        }
    }
}

/// Actual failed extraction stage, without parsing display messages.
///
/// Exhaustive: the SEAM-1 caller matches every arm, so a new cause is a compile
/// error there rather than something a catch-all arm absorbs.
#[derive(Debug, thiserror::Error)]
pub enum Cause {
    /// Input limits, profile selection or original-context mismatch before extraction.
    #[error("{0}")]
    Preflight(#[from] Box<PreflightFailure>),
    /// Original selection/correspondence diagnostic.
    #[error("{0}")]
    Join(#[from] Box<Diagnostic>),
}

/// A failed request preserves its source, selection and any completed extraction.
#[derive(Debug, thiserror::Error)]
#[error("{cause}")]
pub struct Error {
    original: Source,
    selection: Selection,
    extraction: Option<ClausesOutcome>,
    /// Actual failure with no partial extraction.
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
        }
    }

    /// Moves out the original document, selection, any completed extraction and cause.
    pub fn into_parts(self) -> (Source, Selection, Option<ClausesOutcome>, Cause) {
        (self.original, self.selection, self.extraction, self.cause)
    }
}

/// Verified S0 bytes, their document `SourceMap`, and the unchanged upstream Quire result.
///
/// ADR-011 §2.1 I3 row: extraction stops here. The SEAM-1 caller feeds the map's body
/// and the declared language into the native compiler; this adapter never does.
/// Only [`extract`] constructs one, so the map, language and extraction always describe
/// the same verified clause.
#[derive(Debug)]
pub struct ExtractedSource {
    map: SourceMap,
    language: String,
    extraction: ClausesOutcome,
}

impl ExtractedSource {
    /// Exact original/body byte correspondence; `map().body()` is the verified native body.
    pub fn map(&self) -> &SourceMap {
        &self.map
    }

    /// The selected clause's declared language, from the unchanged upstream result.
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Unchanged upstream clause population, availability and diagnostics.
    pub fn extraction(&self) -> &ClausesOutcome {
        &self.extraction
    }

    /// Moves out the map, declared language and unchanged upstream result.
    pub fn into_parts(self) -> (SourceMap, String, ClausesOutcome) {
        (self.map, self.language, self.extraction)
    }
}

/// Quire's own validated clause-only context for `original` and the authored `package`.
///
/// It selects [`CONTRACT_VERSION`] and [`SEMANTIC_CORE_VERSION`], exports nothing,
/// installs no archetype schemas and targets Markdown; it names `original`'s path and
/// source identity. Native imports keep model authority. Quire validates the block
/// and its failures are returned unchanged.
pub fn clause_context(
    package: &str,
    original: &Source,
) -> Result<SemanticContext, Vec<SemanticFailure>> {
    let module = read_semantic_block(
        &serde_json::json!({
            "contract_version": CONTRACT_VERSION, "semantic_core": SEMANTIC_CORE_VERSION,
            "package": package, "exports": [], "targets": ["markdown"]
        }),
        &[],
        &|_| false,
    )?;
    Ok(
        SemanticContext::new(module, original.path(), BundleIndex::default())
            .with_source_identity(original.identity().identity.clone()),
    )
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
        .find(|clause| clause.clause_id == selection.clause_id)
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
) -> Result<SourceMap, Box<Diagnostic>> {
    // The body is a verified subslice of the already bounded original, so it adds no
    // byte budget here; the native compiler applies its own source-byte limit.
    let body = Source::read(
        selection.body.clone(),
        format!("{}#{}", original.path(), clause.clause_id),
        text.as_bytes(),
        text.len(),
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
        build_map(original, clause, selection, region, text)?,
        &clause.language,
    ))
}

/// Extract from the original document and verify the selected body's correspondence.
///
/// The caller must select/verify original bytes and load its Quire context before
/// this call. No source text, availability result or authored binding is inferred
/// from a successful extraction. Each request starts fresh extraction limits. This
/// adapter stops at S0 bytes plus the document `SourceMap`; it never compiles.
pub fn extract(
    original: Source,
    context: &SemanticContext,
    selection: Selection,
    limits: Limits,
) -> Result<ExtractedSource, Box<Error>> {
    if let Err(cause) = preflight::check(&original, context, &selection, limits) {
        return Err(Box::new(Error {
            original,
            selection,
            extraction: None,
            cause: Cause::Preflight(cause),
        }));
    }
    let extraction = extract_clauses(original.text(), context);
    match body_map(&original, &extraction, &selection) {
        Ok((map, language)) => Ok(ExtractedSource {
            map,
            language: language.to_owned(),
            extraction,
        }),
        Err(cause) => Err(Box::new(Error {
            original,
            selection,
            extraction: Some(extraction),
            cause: Cause::Join(cause),
        })),
    }
}
