// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-cst`: the ADR-011 §6.1 layer **1** crate (QSL-178, ADR-011 §7.3 X-3).
//!
//! This layer preserves an exact, recovering concrete syntax tree for the
//! complete-V1 grammar. Recovery is authoring evidence only: a source with
//! any diagnostic or proposed edit is never exposed as an admitted semantic
//! unit.
//!
//! Module order: [`token`] < [`lexer`] < [`diagnostic`] < [`grammar`] <
//! [`cst`] < [`parser`]. Every module here depends only on `qsl-foundation`
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
pub mod parser;
pub mod token;

pub use cst::{
    CstElement, CstNode, CstToken, DefinitionDigest, DefinitionRef, ImportSelection,
    InvalidDefinitionComponent, InvalidModelComponent, LosslessCst, ModelDigest, ModelRef,
    ModelSelection, NodeIdentity, Production, ProfileSelection, Recovery, RecoveryKind,
    SourceSelections, StableNodeId, TokenClass, TokenKind,
};
pub use diagnostic::{CompleteCause, CompleteCode, CompleteDiagnostic, HostCause};
pub use lexer::Limits;

use qsl_foundation::{Source, SourceIdentity};

/// A version-bound source artifact and its lossless parse evidence.
#[derive(Clone, Debug)]
pub struct ParsedSource {
    source: Source,
    cst: LosslessCst,
    diagnostics: Vec<CompleteDiagnostic>,
    selections: SourceSelections,
    incremental: bool,
}

impl ParsedSource {
    /// Exact immutable input source.
    pub fn source(&self) -> &Source {
        &self.source
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
    /// Owned by layer 1: the caller (the full parser, the bounded
    /// incremental-edit path in the root crate's `complete::edit`, or the
    /// root crate's own `complete::parse_with_catalog`) has already produced
    /// every field, and this is the one place that pairs them, so no other
    /// module reaches into [`ParsedSource`]'s private fields directly.
    pub fn from_parts(
        source: Source,
        cst: LosslessCst,
        diagnostics: Vec<CompleteDiagnostic>,
        selections: SourceSelections,
        incremental: bool,
    ) -> Self {
        Self {
            source,
            cst,
            diagnostics,
            selections,
            incremental,
        }
    }

    /// Insert a diagnostic at the given position, ahead of every diagnostic
    /// already recorded from parsing. Used by the root crate's
    /// `complete::parse_with_catalog` to prepend a profile refusal that only
    /// a layer-3 catalog can detect.
    pub fn insert_diagnostic(&mut self, index: usize, diagnostic: CompleteDiagnostic) {
        self.diagnostics.insert(index, diagnostic);
    }
}

/// Parse the complete-V1 grammar while retaining every original source byte.
/// Invalid syntax returns a recovered [`ParsedSource`]; only fatal source or
/// resource failures use the outer error result.
/// Backend installation is deliberately absent from this authority boundary.
///
/// ```compile_fail
/// use std::collections::BTreeSet;
/// use qsl_cst::{self, Limits};
/// use qsl_foundation::SourceIdentity;
/// let installed_backends = BTreeSet::from(["solver:x"]);
/// let _ = qsl_cst::parse(
///     SourceIdentity { identity: "doc".into(), revision: "1".into() },
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
    let limits = limits.bounded();
    let source = diagnostic::read_source(identity, path, bytes, limits.source_bytes)?;
    parse_source(source, limits)
}

/// Parse an already loaded (and optionally digest-verified) exact source.
pub fn parse_source(
    source: Source,
    limits: Limits,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    parser::parse(source, limits.bounded())
}
