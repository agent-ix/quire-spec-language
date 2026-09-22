// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 source authoring surface.
//!
//! This layer preserves an exact, recovering concrete syntax tree. Recovery is
//! authoring evidence only: a source with any diagnostic or proposed edit is
//! never exposed as an admitted semantic unit.

mod cst;
mod diagnostic;
mod edit;
mod editor;
mod grammar;
mod package;
#[cfg(test)]
mod package_tests;
mod parser;

pub use cst::{
    CstElement, CstNode, CstToken, DefinitionDigest, DefinitionRef, ImportSelection,
    InvalidDefinitionComponent, InvalidModelComponent, LosslessCst, ModelDigest, ModelRef,
    ModelSelection, NodeIdentity, Production, ProfileSelection, Recovery, RecoveryKind,
    SourceSelections, StableNodeId, TokenClass, TokenKind,
};
pub use diagnostic::{CompleteCause, CompleteCode, CompleteDiagnostic, HostCause};
pub use edit::{
    apply_edit, apply_edit_with_catalog, apply_edits, apply_edits_with_catalog, SourceEdit,
};
pub use editor::{
    analyze_document, format_document, CompletionItem, CompletionKind, DocumentBinding,
    DocumentNavigation, DocumentSnapshot, DocumentSymbol, FormatCorrespondence, FormatResponse,
};
pub use package::{
    resolve_source_package, CompleteBundle, Definition, DefinitionCatalog, DefinitionConflict,
    DefinitionRole, Facet, ModelArtifact, ModelCatalog, PackageLimits, PackageRefusal,
    ReaderAuthority, ResolvedSourcePackage, SemanticDigest, SourceAuthority,
};
pub use package::{CapabilityId, PackageError, ProfileCatalog, SourceDigest};

pub use crate::lexer::Limits;
use crate::{Source, SourceIdentity};

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
    /// Owned by layer 1: the caller (the full parser or the bounded
    /// incremental-edit path) has already produced every field, and this is
    /// the one place that pairs them, so no other module reaches into
    /// [`ParsedSource`]'s private fields directly.
    pub(crate) fn from_parts(
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
    /// already recorded from parsing.
    pub(crate) fn insert_diagnostic(&mut self, index: usize, diagnostic: CompleteDiagnostic) {
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
/// use quire_spec_language::complete::{self, Limits};
/// use quire_spec_language::SourceIdentity;
/// let installed_backends = BTreeSet::from(["solver:x"]);
/// let _ = complete::parse(
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

/// Parse while validating every selected profile against an exact definition
/// catalog. An older catalog retains all bytes and reports the unknown profile
/// at its source selection.
pub fn parse_with_catalog(
    identity: SourceIdentity,
    path: impl Into<String>,
    bytes: &[u8],
    catalog: &ProfileCatalog,
    limits: Limits,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    let limits = limits.bounded();
    let source = diagnostic::read_source(identity, path, bytes, limits.source_bytes)?;
    let mut base = parser::parse(source.clone(), limits)?;
    let refused = base.selections().profiles.iter().find_map(|selection| {
        editor::profile_refusal(catalog.profile_status(&selection.definition))
            .map(|refusal| (selection.identity_span, refusal))
    });
    if let Some((identity_span, (code, cause, message))) = refused {
        base.insert_diagnostic(
            0,
            *diagnostic::error(
                &source,
                code,
                cause,
                crate::Phase::Profile,
                identity_span.start,
                identity_span.end,
                message,
            ),
        );
    }
    Ok(base)
}
