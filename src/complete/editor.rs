// SPDX-License-Identifier: AGPL-3.0-or-later
//! Revision-bound formatter and editor snapshot over the lossless CST.
use qsl_foundation::{Phase, SourceIdentity, Span};
use std::collections::BTreeSet;

use super::package::{ProfileStatus, StaleProfile};
use super::{
    CompleteCause, CompleteCode, CompleteDiagnostic, DefinitionRef, HostCause, Limits,
    NodeIdentity, ParsedSource, Production, ProfileCatalog, SourceEdit, TokenClass, TokenKind,
};

/// Exact document/profile tuple carried by every editor request and response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentBinding {
    /// Opaque document identity.
    pub identity: String,
    /// Exact source revision.
    pub revision: String,
    /// Exact selected formatting/language profile.
    pub profile: DefinitionRef,
}

/// One deterministic source-outline symbol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentSymbol {
    /// Grammar production that owns the source region.
    pub production: Production,
    /// Exact half-open source region.
    pub span: Span,
}

/// One revision-bound syntax navigation entry for language-server consumers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentNavigation {
    /// Grammar production owning the navigable region.
    pub production: Production,
    /// Exact half-open source region.
    pub span: Span,
    /// Exchange identity bound to the exact document bytes and revision.
    pub identity: NodeIdentity,
}

/// Typed origin of one deterministic completion spelling.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CompletionKind {
    /// Closed base complete-V1 grammar spelling.
    GrammarToken,
    /// Selected profile alias.
    ProfileAlias,
    /// Selected import alias.
    ImportAlias,
    /// Selected model alias.
    ModelAlias,
}

/// One bounded completion candidate from the same parsed document authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletionItem {
    /// Exact insertion spelling.
    pub spelling: String,
    /// Typed origin used for deterministic client presentation.
    pub kind: CompletionKind,
}

/// Versioned diagnostics and symbols returned by the shared parser service.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentSnapshot {
    /// Request tuple this result answers.
    pub binding: DocumentBinding,
    /// Diagnostics in stable primary-span/code/related-span/message order.
    pub diagnostics: Vec<CompleteDiagnostic>,
    /// Document symbols in source order.
    pub symbols: Vec<DocumentSymbol>,
    /// Revision-bound CST navigation index in source order.
    pub navigation: Vec<DocumentNavigation>,
    /// Closed grammar and selected package aliases in spelling order.
    pub completions: Vec<CompletionItem>,
}

/// Deterministic, non-overlapping formatting edits for an exact revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatResponse {
    /// Request tuple this result answers.
    pub binding: DocumentBinding,
    /// At most one whole-document edit; empty means already formatted.
    pub edits: Vec<SourceEdit>,
    /// Exact token correspondence from the original to formatted revision.
    pub correspondence: Vec<FormatCorrespondence>,
}

/// One exact non-whitespace token correspondence produced by formatting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormatCorrespondence {
    /// Token range in the original revision.
    pub original: Span,
    /// Same exact token spelling in the formatted candidate.
    pub formatted: Span,
}

/// Produce diagnostics and outline symbols through the same parsed artifact used
/// by full and incremental requests.
pub fn analyze_document(
    parsed: &ParsedSource,
    binding: DocumentBinding,
    cancelled: bool,
    catalog: &ProfileCatalog,
) -> Result<DocumentSnapshot, Box<CompleteDiagnostic>> {
    validate_binding(parsed, &binding, catalog)?;
    if cancelled {
        return Err(super::diagnostic::error(
            parsed.source(),
            CompleteCode::Cancelled,
            CompleteCause::CallerCancelled,
            Phase::Check,
            0,
            0,
            "document analysis cancelled before publication",
        ));
    }
    let mut diagnostics = parsed.diagnostics().to_vec();
    diagnostics.sort_by(|left, right| {
        (left.span.start.byte, left.span.end.byte, left.code.as_str())
            .cmp(&(
                right.span.start.byte,
                right.span.end.byte,
                right.code.as_str(),
            ))
            .then_with(|| {
                left.related
                    .iter()
                    .map(|span| (span.start.byte, span.end.byte))
                    .cmp(
                        right
                            .related
                            .iter()
                            .map(|span| (span.start.byte, span.end.byte)),
                    )
            })
            .then_with(|| left.message.cmp(&right.message))
    });
    let mut symbols: Vec<_> = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| is_symbol(node.production()))
        .map(|node| DocumentSymbol {
            production: node.production(),
            span: node.span(),
        })
        .collect();
    symbols.sort_by_key(|symbol| (symbol.span.start, symbol.span.end, symbol.production));
    let mut navigation: Vec<_> = parsed
        .cst()
        .nodes()
        .iter()
        .map(|node| DocumentNavigation {
            production: node.production(),
            span: node.span(),
            identity: node.identity().clone(),
        })
        .collect();
    navigation.sort_by_key(|entry| (entry.span.start, entry.span.end, entry.production));
    let mut completion_set = BTreeSet::new();
    completion_set.extend(
        super::grammar::base_reserved_spellings()
            .into_iter()
            .map(|spelling| (spelling.to_string(), CompletionKind::GrammarToken)),
    );
    completion_set.extend(
        parsed
            .selections()
            .profiles
            .iter()
            .map(|selection| (selection.alias.clone(), CompletionKind::ProfileAlias)),
    );
    completion_set.extend(parsed.selections().imports.iter().filter_map(|selection| {
        selection
            .alias
            .clone()
            .map(|alias| (alias, CompletionKind::ImportAlias))
    }));
    completion_set.extend(
        parsed
            .selections()
            .models
            .iter()
            .map(|selection| (selection.alias.clone(), CompletionKind::ModelAlias)),
    );
    let completions = completion_set
        .into_iter()
        .map(|(spelling, kind)| CompletionItem { spelling, kind })
        .collect();
    Ok(DocumentSnapshot {
        binding,
        diagnostics,
        symbols,
        navigation,
        completions,
    })
}

/// Format valid complete-V1 source from exact CST token spellings. The candidate
/// is reparsed under the same profile catalog and explicit limits, then every
/// non-whitespace token must correspond exactly before edits are returned. The
/// later checked-package authority owns semantic-identity validation.
pub fn format_document(
    parsed: &ParsedSource,
    binding: DocumentBinding,
    catalog: &ProfileCatalog,
    limits: Limits,
) -> Result<FormatResponse, Box<CompleteDiagnostic>> {
    let limits = limits.bounded();
    validate_binding(parsed, &binding, catalog)?;
    if !parsed.is_admissible() {
        // The refusal retains the originating code and cause. The parser
        // records a diagnostic with every recovery, so a recovery without one
        // breaks that invariant.
        let (code, cause) = parsed.diagnostics().first().map_or(
            (
                CompleteCode::RuntimeInvariant,
                CompleteCause::EstablishedInvariantBroken,
            ),
            |origin| (origin.code, origin.cause),
        );
        return Err(super::diagnostic::error(
            parsed.source(),
            code,
            cause,
            Phase::Format,
            0,
            parsed.source().text().len(),
            "formatting requires an unrecovered complete-V1 source",
        ));
    }
    let formatted = formatted_text(parsed, limits.source_bytes)?;
    let candidate = super::parse_with_catalog(
        SourceIdentity {
            identity: binding.identity.clone(),
            revision: format!("{}:format-candidate", binding.revision),
        },
        parsed.source().path(),
        formatted.as_bytes(),
        catalog,
        limits,
    )?;
    if !candidate.is_admissible() {
        return Err(super::diagnostic::error(
            parsed.source(),
            CompleteCode::InvalidProjectionCorrespondence,
            CompleteCause::CorrespondenceLoss,
            Phase::Format,
            0,
            parsed.source().text().len(),
            "formatter candidate is not admissible under the selected profile catalog",
        ));
    }
    let original_tokens: Vec<_> = parsed
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() != TokenClass::Whitespace)
        .collect();
    let candidate_tokens: Vec<_> = candidate
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() != TokenClass::Whitespace)
        .collect();
    if original_tokens.len() != candidate_tokens.len()
        || original_tokens
            .iter()
            .zip(&candidate_tokens)
            .any(|(original, formatted)| original.spelling() != formatted.spelling())
    {
        return Err(super::diagnostic::error(
            parsed.source(),
            CompleteCode::InvalidProjectionCorrespondence,
            CompleteCause::CorrespondenceLoss,
            Phase::Format,
            0,
            parsed.source().text().len(),
            "formatter could not retain exact token correspondence",
        ));
    }
    let correspondence = original_tokens
        .iter()
        .zip(candidate_tokens)
        .map(|(original, formatted)| FormatCorrespondence {
            original: original.span(),
            formatted: formatted.span(),
        })
        .collect();
    let edits = if formatted == parsed.source().text() {
        Vec::new()
    } else {
        vec![SourceEdit {
            range: Span {
                start: 0,
                end: parsed.source().text().len(),
            },
            replacement: formatted,
        }]
    };
    Ok(FormatResponse {
        binding,
        edits,
        correspondence,
    })
}

fn validate_binding(
    parsed: &ParsedSource,
    binding: &DocumentBinding,
    catalog: &ProfileCatalog,
) -> Result<(), Box<CompleteDiagnostic>> {
    validate_catalog_profiles(parsed, catalog)?;
    let identity = parsed.source().identity();
    if identity.identity != binding.identity || identity.revision != binding.revision {
        return Err(super::diagnostic::error(
            parsed.source(),
            CompleteCode::InvalidSourceIdentity,
            CompleteCause::Host(HostCause::RequestRevision),
            Phase::SourceMap,
            0,
            0,
            "editor request is bound to a different document revision",
        ));
    }
    let selected_profiles = &parsed.selections().profiles;
    let selected = selected_profiles.is_empty()
        || selected_profiles
            .iter()
            .any(|selection| selection.definition == binding.profile);
    let status = catalog.profile_status(&binding.profile);
    let refusal = match (status, selected) {
        (ProfileStatus::Exact, true) => return Ok(()),
        (ProfileStatus::Stale(stale), _) => (
            CompleteCode::StaleDependency,
            stale_cause(stale),
            "editor request selected a stale complete-V1 profile",
        ),
        (ProfileStatus::Exact, false) | (ProfileStatus::Unknown, _) => (
            CompleteCode::UnknownProfile,
            CompleteCause::UnsupportedSelection,
            "editor request did not select a known complete-V1 profile",
        ),
    };
    let (code, cause, message) = refusal;
    Err(super::diagnostic::error(
        parsed.source(),
        code,
        cause,
        Phase::Profile,
        0,
        0,
        message,
    ))
}

fn stale_cause(stale: StaleProfile) -> CompleteCause {
    match stale {
        StaleProfile::Revision => CompleteCause::RevisionMismatch,
        StaleProfile::ByteDigest => CompleteCause::ByteDigestMismatch,
    }
}

/// The refusal of a profile selection with `status`, if it is not exact.
pub(super) fn profile_refusal(
    status: ProfileStatus,
) -> Option<(CompleteCode, CompleteCause, &'static str)> {
    match status {
        ProfileStatus::Exact => None,
        ProfileStatus::Stale(stale) => Some((
            CompleteCode::StaleDependency,
            stale_cause(stale),
            "selected profile version or digest is stale for this consumer",
        )),
        ProfileStatus::Unknown => Some((
            CompleteCode::UnknownProfile,
            CompleteCause::UnsupportedSelection,
            "selected profile definition is unknown to this consumer",
        )),
    }
}

pub(super) fn validate_catalog_profiles(
    parsed: &ParsedSource,
    catalog: &ProfileCatalog,
) -> Result<(), Box<CompleteDiagnostic>> {
    for selection in &parsed.selections().profiles {
        let Some((code, cause, message)) =
            profile_refusal(catalog.profile_status(&selection.definition))
        else {
            continue;
        };
        return Err(super::diagnostic::error(
            parsed.source(),
            code,
            cause,
            Phase::Profile,
            selection.identity_span.start,
            selection.identity_span.end,
            message,
        ));
    }
    Ok(())
}

fn formatted_text(
    parsed: &ParsedSource,
    source_byte_limit: usize,
) -> Result<String, Box<CompleteDiagnostic>> {
    let mut output = String::new();
    let mut indent = 0_usize;
    let mut previous: Option<&super::CstToken> = None;
    for token in parsed
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() != TokenClass::Whitespace)
    {
        let spelling = token.spelling();
        if token.class() == TokenClass::Comment {
            begin_line(parsed, &mut output, indent, source_byte_limit, token.span())?;
            append_bounded(
                parsed,
                &mut output,
                token_text(parsed, token)?,
                source_byte_limit,
                token.span(),
            )?;
            newline(parsed, &mut output, source_byte_limit, token.span())?;
            previous = None;
            continue;
        }
        if spelling == b"}" {
            indent = indent.saturating_sub(1);
            newline(parsed, &mut output, source_byte_limit, token.span())?;
            begin_line(parsed, &mut output, indent, source_byte_limit, token.span())?;
        } else if output.ends_with('\n') {
            begin_line(parsed, &mut output, indent, source_byte_limit, token.span())?;
        }
        if needs_space(previous, token, &output) {
            append_bounded(parsed, &mut output, " ", source_byte_limit, token.span())?;
        }
        append_bounded(
            parsed,
            &mut output,
            token_text(parsed, token)?,
            source_byte_limit,
            token.span(),
        )?;
        match spelling {
            b"{" => {
                indent = indent.saturating_add(1);
                newline(parsed, &mut output, source_byte_limit, token.span())?;
            }
            b";" => newline(parsed, &mut output, source_byte_limit, token.span())?,
            b"}" => {}
            _ => {}
        }
        previous = Some(token);
    }
    if !output.ends_with('\n') {
        let end = parsed.source().text().len();
        append_bounded(
            parsed,
            &mut output,
            "\n",
            source_byte_limit,
            Span { start: end, end },
        )?;
    }
    Ok(output)
}

fn token_text<'a>(
    parsed: &'a ParsedSource,
    token: &'a super::CstToken,
) -> Result<&'a str, Box<CompleteDiagnostic>> {
    std::str::from_utf8(token.spelling()).map_err(|_| {
        super::diagnostic::error(
            parsed.source(),
            CompleteCode::RuntimeInvariant,
            CompleteCause::EstablishedInvariantBroken,
            Phase::Format,
            token.span().start,
            token.span().end,
            "lossless CST token violated the validated UTF-8 source invariant",
        )
    })
}

fn begin_line(
    parsed: &ParsedSource,
    output: &mut String,
    indent: usize,
    source_byte_limit: usize,
    span: Span,
) -> Result<(), Box<CompleteDiagnostic>> {
    if output.is_empty() || output.ends_with('\n') {
        for _ in 0..indent {
            append_bounded(parsed, output, "  ", source_byte_limit, span)?;
        }
    }
    Ok(())
}

fn newline(
    parsed: &ParsedSource,
    output: &mut String,
    source_byte_limit: usize,
    span: Span,
) -> Result<(), Box<CompleteDiagnostic>> {
    while output.ends_with(' ') {
        output.pop();
    }
    if !output.is_empty() && !output.ends_with('\n') {
        append_bounded(parsed, output, "\n", source_byte_limit, span)?;
    }
    Ok(())
}

fn append_bounded(
    parsed: &ParsedSource,
    output: &mut String,
    value: &str,
    source_byte_limit: usize,
    span: Span,
) -> Result<(), Box<CompleteDiagnostic>> {
    if output
        .len()
        .checked_add(value.len())
        .is_none_or(|length| length > source_byte_limit)
    {
        return Err(super::diagnostic::error(
            parsed.source(),
            CompleteCode::ResourceExhausted,
            CompleteCause::InsufficientNextCharge,
            Phase::Format,
            span.start,
            span.end,
            "formatted output byte budget exhausted",
        ));
    }
    output.push_str(value);
    Ok(())
}

fn needs_space(
    previous: Option<&super::CstToken>,
    current: &super::CstToken,
    output: &str,
) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    if output.ends_with([' ', '\n']) {
        return false;
    }
    if previous.span().end == current.span().start
        && matches!(
            (previous.kind(), current.kind()),
            (
                TokenKind::HexPrefix | TokenKind::HexDigit,
                TokenKind::HexDigit
            )
        )
    {
        return false;
    }
    let current = current.spelling();
    if matches!(current, b")" | b"]" | b"," | b";" | b"." | b"::" | b"?") {
        return false;
    }
    !matches!(previous.spelling(), b"(" | b"[" | b"." | b"::" | b"^")
}

fn is_symbol(production: Production) -> bool {
    matches!(
        production,
        Production::Profile
            | Production::ImportDeclaration
            | Production::Model
            | Production::DimensionDeclaration
            | Production::UnitDeclaration
            | Production::EnumDeclaration
            | Production::RecordDeclaration
            | Production::TupleDeclaration
            | Production::AliasDeclaration
            | Production::FunctionDeclaration
            | Production::Predicate
            | Production::StateClause
            | Production::TemporalClause
            | Production::ProtocolClause
            | Production::RelationClause
            | Production::HyperClause
            | Production::HybridDeclaration
            | Production::SynthesisDeclaration
            | Production::VerificationPlan
    )
}
