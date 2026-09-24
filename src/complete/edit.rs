// SPDX-License-Identifier: AGPL-3.0-or-later
use qsl_cst::{
    parse, CompleteCause, CompleteCode, CompleteDiagnostic, HostCause, Limits, ParsedSource,
};
use qsl_foundation::selection::ProfileCatalog;
use qsl_foundation::{Phase, SourceIdentity, Span, SyntaxLimit};

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
    catalog: &ProfileCatalog,
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
    catalog: &ProfileCatalog,
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
    catalog: Option<&ProfileCatalog>,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    let source = parsed.source();
    if let Some(catalog) = catalog {
        super::editor::validate_catalog_profiles(parsed, catalog)?;
    }
    let failure = |code, cause, span: Span, message| {
        qsl_cst::diagnostic::error(
            source,
            code,
            cause,
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
            CompleteCause::Host(HostCause::EditPredecessor),
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
                CompleteCause::Host(HostCause::EditRanges),
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
        return Err(qsl_cst::diagnostic::resource_exhausted(
            source,
            Phase::SourceMap,
            Span { start: 0, end: 0 },
            SyntaxLimit::SourceBytes {
                bound: limits.source_bytes,
            },
        ));
    };
    if output_len > limits.source_bytes {
        return Err(qsl_cst::diagnostic::resource_exhausted(
            source,
            Phase::SourceMap,
            Span { start: 0, end: 0 },
            SyntaxLimit::SourceBytes {
                bound: limits.source_bytes,
            },
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
    if edits.len() == 1
        && edits[0].range.start == edits[0].range.end
        && qsl_cst::token::is_lexer_whitespace(&edits[0].replacement)
        && limits.source_bytes == Limits::default().source_bytes
        && limits.tokens == Limits::default().tokens
        && limits.nodes == Limits::default().nodes
        && limits.nesting == Limits::default().nesting
    {
        if let Some(result) = parsed.with_whitespace_insertion(
            new_identity.clone(),
            edits[0].range.start,
            &edits[0].replacement,
            &bytes,
            limits,
        )? {
            return Ok(result);
        }
    }
    if let Some(catalog) = catalog {
        super::parse_with_catalog(new_identity, source.path(), &bytes, catalog, limits)
    } else {
        parse(new_identity, source.path(), &bytes, limits)
    }
}
