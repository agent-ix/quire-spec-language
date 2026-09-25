// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-cst`: the ADR-011 §6.1 layer **1** crate (QSL-178, ADR-011 §7.3 X-3).
//!
//! This layer preserves an exact, recovering concrete syntax tree for the
//! complete-V1 grammar. Recovery is authoring evidence only: a source with
//! any diagnostic or proposed edit is never exposed as an admitted semantic
//! unit.
//!
//! Module order: [`token`] < [`lexer`] < [`diagnostic`] < [`grammar`] <
//! [`cst`] < `parser` (crate-private: [`parse`]/[`parse_source`] are its
//! only outward surface). Every module here depends only on
//! `qsl-foundation`
//! (layer F) -- no module imports the QSL root crate or a SEAM module
//! (ADR-011 §6.1's leaf-of-QSL-workspace rule for layer 1).
//!
//! The root crate's `complete` module stays home to `editor`, `edit` (tooling
//! built on this layer) and `package` (ADR-011 §6.2 layer 3, moving with
//! QSL-181); it repoints its own uses of the layer-1 types here and
//! re-exports none of them (ADR-011 §7.2).

#![forbid(unsafe_code)]

pub mod cst;
pub mod diagnostic;
pub mod grammar;
pub mod lexer;
mod parser;
pub mod token;

pub use cst::{
    CstElement, CstNode, CstToken, LosslessCst, NodeIdentity, NodeIndex, Production, Recovery,
    RecoveryKind, SourceChange, TokenClass, TokenKind,
};
pub use diagnostic::{CompleteCause, CompleteCode, CompleteDiagnostic, HostCause};
pub use lexer::{declared_edition, DeclaredEdition, Limits};

use qsl_foundation::selection::SourceSelections;
use qsl_foundation::{Source, SourceIdentity, Span};

/// A version-bound source artifact and its lossless parse evidence.
#[derive(Clone, Debug)]
pub struct ParsedSource {
    source: Source,
    cst: LosslessCst,
    diagnostics: Vec<CompleteDiagnostic>,
    selections: SourceSelections,
    incremental: bool,
    effective_limits: Limits,
}

impl ParsedSource {
    /// Exact immutable input source.
    pub fn source(&self) -> &Source {
        &self.source
    }

    /// The [`Limits`] this parse was checked against, exactly as the caller
    /// supplied them: every field is enforced as given, with no hidden
    /// ceiling beneath it (QSL-199), so a caller-raised ceiling is visible
    /// with the result it produced. A whitespace fast-path edit
    /// ([`Self::with_whitespace_insertion`]) records its predecessor's limits,
    /// which it applies only when they equal the edit's own.
    pub fn effective_limits(&self) -> Limits {
        self.effective_limits
    }

    /// Lossless CST, including trivia and any proposed recovery edits.
    pub fn cst(&self) -> &LosslessCst {
        &self.cst
    }

    /// Stable typed diagnostics in source order.
    pub fn diagnostics(&self) -> &[CompleteDiagnostic] {
        &self.diagnostics
    }

    /// Exact package-relevant selections recovered from the source prelude.
    pub fn selections(&self) -> &SourceSelections {
        &self.selections
    }

    /// Only an unrecovered, diagnostic-free parse may enter semantic admission.
    pub fn is_admissible(&self) -> bool {
        self.diagnostics.is_empty() && self.cst.recoveries().is_empty()
    }

    /// Whether this artifact was updated through the bounded incremental CST
    /// path rather than reconstructed by the full parser.
    pub fn is_incremental_result(&self) -> bool {
        self.incremental
    }

    /// Assemble a parse result from its already-computed layer-1 evidence.
    /// Owned by layer 1: the caller (the full parser, or this crate's own
    /// [`Self::with_whitespace_insertion`] fast path) has already produced
    /// every field, and this is the one place that pairs them, so no other
    /// module reaches into [`ParsedSource`]'s private fields directly. Kept
    /// `pub(crate)`: a downstream crate that could pair arbitrary source,
    /// CST and diagnostics could forge a value [`Self::is_admissible`]
    /// accepts without ever having gone through a real parse (QSL-178
    /// review F2).
    pub(crate) fn from_parts(
        source: Source,
        cst: LosslessCst,
        diagnostics: Vec<CompleteDiagnostic>,
        selections: SourceSelections,
        incremental: bool,
        effective_limits: Limits,
    ) -> Self {
        Self {
            source,
            cst,
            diagnostics,
            selections,
            incremental,
            effective_limits,
        }
    }

    /// Prepend a diagnostic, ahead of every diagnostic already recorded from
    /// parsing. Used by the root crate's `complete::parse_with_catalog` to
    /// record a profile refusal that only a layer-3 catalog can detect --
    /// prepending only ever makes a parse less admissible, unlike an
    /// arbitrary-index insert, which could panic past the current length.
    pub fn prepend_diagnostic(&mut self, diagnostic: CompleteDiagnostic) {
        self.diagnostics.insert(0, diagnostic);
    }

    /// Apply a single whitespace-only insertion to this already-admissible
    /// parse without reparsing, or `Ok(None)` when the fast path does not
    /// apply (the caller should fall back to a full reparse of `bytes`).
    /// `bytes` is the whole edited document (already assembled and
    /// byte-budget-checked by the caller); `at`/`inserted` describe the
    /// single insertion within it. Kept as the only public entry point onto
    /// `LosslessCst`'s own crate-private whitespace-insertion fast path and
    /// this crate's own `from_parts`, since it is the one place that checks
    /// the insertion is actually whitespace-only against an admissible
    /// predecessor before pairing the resulting evidence (QSL-178 review
    /// F2).
    ///
    /// The fast path re-reads `bytes` under `limits.source_bytes` but does
    /// not re-count tokens, nodes or nesting: it only widens one existing
    /// whitespace token, so those counts are the predecessor's. It applies
    /// only when `limits` equals the predecessor's
    /// [`Self::effective_limits`], so the limits it records were enforced
    /// against this exact structure; any other `limits` returns `Ok(None)`
    /// for a full reparse.
    pub fn with_whitespace_insertion(
        &self,
        new_identity: SourceIdentity,
        at: usize,
        inserted: &str,
        bytes: &[u8],
        limits: Limits,
    ) -> Result<Option<Self>, Box<CompleteDiagnostic>> {
        if !self.is_admissible()
            || !token::is_lexer_whitespace(inserted)
            || limits != self.effective_limits
        {
            return Ok(None);
        }
        let edited_source =
            diagnostic::read_source(new_identity, self.source.path(), bytes, limits.source_bytes)?;
        let Some(cst) = self
            .cst
            .with_whitespace_insertion(edited_source.clone(), at, inserted)
        else {
            return Ok(None);
        };
        Ok(Some(Self::from_parts(
            edited_source,
            cst,
            Vec::new(),
            shifted_selections(&self.selections, at, inserted.len()),
            true,
            limits,
        )))
    }
}

/// Shift every span in `selections` by `added` bytes wherever it starts at
/// or after `at` (or only its end, when it merely spans across `at`) -- pure
/// span arithmetic over layer-1 selection types, used only by
/// [`ParsedSource::with_whitespace_insertion`] to keep an edited parse's
/// selections aligned with its shifted source.
fn shifted_selections(selections: &SourceSelections, at: usize, added: usize) -> SourceSelections {
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

/// Parse the complete-V1 grammar while retaining every original source byte.
/// Invalid syntax returns a recovered [`ParsedSource`]; only fatal source or
/// resource failures use the outer error result.
/// Backend installation is deliberately absent from this authority boundary.
///
/// ```compile_fail
/// use std::collections::BTreeSet;
/// use qsl_cst::Limits;
/// use qsl_foundation::SourceIdentity;
/// let installed_backends = BTreeSet::from(["solver:x"]);
/// let _ = qsl_cst::parse(
///     SourceIdentity { authority: "agent-ix".into(), identity: "doc".into(), revision_namespace: "git".into(), revision: "1".into() },
///     "doc.native",
///     b"",
///     Limits::default(),
///     &installed_backends,
/// );
/// ```
pub fn parse(
    identity: SourceIdentity,
    path: impl Into<String>,
    bytes: &[u8],
    limits: Limits,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    let source = diagnostic::read_source(identity, path, bytes, limits.source_bytes)?;
    parse_source(source, limits)
}

/// Parse an already loaded (and optionally digest-verified) exact source.
pub fn parse_source(
    source: Source,
    limits: Limits,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    parser::parse(source, limits)
}
