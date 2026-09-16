// SPDX-License-Identifier: AGPL-3.0-or-later
use super::{parse, CompleteCode, CompleteDiagnostic, ParsedSource};
use crate::{Limits, Phase, SourceIdentity, Span};

/// One UTF-8-boundary-preserving source replacement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceEdit {
    /// Half-open range in the predecessor revision.
    pub range: Span,
    /// Exact replacement text.
    pub replacement: String,
}

/// Apply one edit and reparse under a new exact source revision.
pub fn apply_edit(
    parsed: &ParsedSource,
    expected_revision: &str,
    new_identity: SourceIdentity,
    edit: SourceEdit,
    limits: Limits,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    apply_edits_selected(
        parsed,
        expected_revision,
        new_identity,
        &[edit],
        limits,
        None,
    )
}

/// Apply one edit and reparse with the same exact definition catalog used to
/// validate selected profiles.
pub fn apply_edit_with_catalog(
    parsed: &ParsedSource,
    expected_revision: &str,
    new_identity: SourceIdentity,
    edit: SourceEdit,
    catalog: &super::ProfileCatalog,
    limits: Limits,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    apply_edits_selected(
        parsed,
        expected_revision,
        new_identity,
        &[edit],
        limits,
        Some(catalog),
    )
}

/// Apply ordered, non-overlapping edits and reparse under a new revision.
pub fn apply_edits(
    parsed: &ParsedSource,
    expected_revision: &str,
    new_identity: SourceIdentity,
    edits: &[SourceEdit],
    limits: Limits,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    apply_edits_selected(parsed, expected_revision, new_identity, edits, limits, None)
}

/// Apply ordered edits and reparse with exact profile-catalog validation.
pub fn apply_edits_with_catalog(
    parsed: &ParsedSource,
    expected_revision: &str,
    new_identity: SourceIdentity,
    edits: &[SourceEdit],
    catalog: &super::ProfileCatalog,
    limits: Limits,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    apply_edits_selected(
        parsed,
        expected_revision,
        new_identity,
        edits,
        limits,
        Some(catalog),
    )
}

fn apply_edits_selected(
    parsed: &ParsedSource,
    expected_revision: &str,
    new_identity: SourceIdentity,
    edits: &[SourceEdit],
    limits: Limits,
    catalog: Option<&super::ProfileCatalog>,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    let limits = limits.bounded();
    let source = parsed.source();
    if let Some(catalog) = catalog {
        super::editor::validate_catalog_profiles(parsed, catalog)?;
    }
    let failure = |code, span: Span, message| {
        super::diagnostic::error(
            source,
            code,
            Phase::SourceMap,
            span.start,
            span.end,
            message,
        )
    };
    if source.identity().revision != expected_revision
        || new_identity.identity != source.identity().identity
        || new_identity.revision == source.identity().revision
    {
        return Err(failure(
            CompleteCode::InvalidSourceIdentity,
            Span { start: 0, end: 0 },
            "incremental edit revision does not match the exact source predecessor",
        ));
    }
    let mut previous = None;
    for (index, edit) in edits.iter().enumerate() {
        let range_valid = source.slice(edit.range).is_some();
        let conflicts = previous.is_some_and(|previous: Span| {
            edit.range.start < previous.end
                || (edit.range.start == previous.end
                    && edit.range.start == edit.range.end
                    && previous.start == previous.end)
        });
        if !range_valid || (index > 0 && conflicts) {
            return Err(failure(
                CompleteCode::InvalidSourceMap,
                if range_valid {
                    edit.range
                } else {
                    Span { start: 0, end: 0 }
                },
                "incremental edits must be ordered, non-overlapping UTF-8 ranges",
            ));
        }
        previous = Some(edit.range);
    }
    let removed_bytes = edits.iter().try_fold(0_usize, |total, edit| {
        total.checked_add(edit.range.end - edit.range.start)
    });
    let replacement_bytes = edits.iter().try_fold(0_usize, |total, edit| {
        total.checked_add(edit.replacement.len())
    });
    let output_len = removed_bytes
        .and_then(|removed| source.text().len().checked_sub(removed))
        .and_then(|retained| replacement_bytes.and_then(|added| retained.checked_add(added)));
    let Some(output_len) = output_len else {
        return Err(failure(
            CompleteCode::ResourceExhausted,
            Span { start: 0, end: 0 },
            "incremental edit output length overflowed",
        ));
    };
    if output_len > limits.source_bytes {
        return Err(failure(
            CompleteCode::ResourceExhausted,
            Span { start: 0, end: 0 },
            "incremental edit exceeds the source byte budget",
        ));
    }
    let mut bytes = Vec::with_capacity(output_len);
    let mut cursor = 0;
    for edit in edits {
        bytes.extend_from_slice(&source.text().as_bytes()[cursor..edit.range.start]);
        bytes.extend_from_slice(edit.replacement.as_bytes());
        cursor = edit.range.end;
    }
    bytes.extend_from_slice(&source.text().as_bytes()[cursor..]);
    if parsed.is_admissible()
        && edits.len() == 1
        && edits[0].range.start == edits[0].range.end
        && crate::token::is_lexer_whitespace(&edits[0].replacement)
        && limits.source_bytes == Limits::default().source_bytes
        && limits.tokens == Limits::default().tokens
        && limits.nodes == Limits::default().nodes
        && limits.nesting == Limits::default().nesting
    {
        let edited_source = crate::Source::read(
            new_identity.clone(),
            source.path(),
            &bytes,
            limits.source_bytes,
        )
        .map_err(|diagnostic| Box::new(CompleteDiagnostic::from_legacy(*diagnostic)))?;
        if let Some(cst) = parsed.cst().with_whitespace_insertion(
            edited_source.clone(),
            edits[0].range.start,
            &edits[0].replacement,
        ) {
            return Ok(ParsedSource {
                source: edited_source,
                cst,
                diagnostics: Vec::new(),
                selections: shifted_selections(
                    parsed.selections(),
                    edits[0].range.start,
                    edits[0].replacement.len(),
                ),
                incremental: true,
            });
        }
    }
    if let Some(catalog) = catalog {
        super::parse_with_catalog(new_identity, source.path(), &bytes, catalog, limits)
    } else {
        parse(new_identity, source.path(), &bytes, limits)
    }
}

fn shifted_selections(
    selections: &super::SourceSelections,
    at: usize,
    added: usize,
) -> super::SourceSelections {
    fn shifted(mut span: Span, at: usize, added: usize) -> Span {
        if span.start >= at {
            span.start = span.start.saturating_add(added);
            span.end = span.end.saturating_add(added);
        } else if span.end > at {
            span.end = span.end.saturating_add(added);
        }
        span
    }

    let mut selections = selections.clone();
    for profile in &mut selections.profiles {
        profile.span = shifted(profile.span, at, added);
        profile.identity_span = shifted(profile.identity_span, at, added);
    }
    for import in &mut selections.imports {
        import.span = shifted(import.span, at, added);
    }
    for model in &mut selections.models {
        model.span = shifted(model.span, at, added);
    }
    selections
}
