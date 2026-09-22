// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 tooling and package selection built on the `qsl-cst` layer.
//!
//! The recovering concrete syntax tree itself -- token vocabulary, lexer,
//! parser, declarative grammar and located diagnostics -- lives in the
//! `qsl-cst` crate (ADR-011 §6.1 layer 1, QSL-178). This module holds
//! `editor`/`edit` (tooling built on that layer) and `package` (ADR-011 §6.2
//! layer 3, moving to its own crate with QSL-181); it repoints its own uses
//! of the layer-1 types at `qsl_cst` and re-exports none of them
//! (ADR-011 §7.2).

mod edit;
mod editor;
mod package;
#[cfg(test)]
mod package_tests;

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

use qsl_cst::{CompleteDiagnostic, Limits, ParsedSource};
use qsl_foundation::SourceIdentity;

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
    let source = qsl_cst::diagnostic::read_source(identity, path, bytes, limits.source_bytes)?;
    let mut base = qsl_cst::parse_source(source.clone(), limits)?;
    let refused = base.selections().profiles.iter().find_map(|selection| {
        editor::profile_refusal(catalog.profile_status(&selection.definition))
            .map(|refusal| (selection.identity_span, refusal))
    });
    if let Some((identity_span, (code, cause, message))) = refused {
        base.prepend_diagnostic(*qsl_cst::diagnostic::error(
            &source,
            code,
            cause,
            qsl_foundation::Phase::Profile,
            identity_span.start,
            identity_span.end,
            message,
        ));
    }
    Ok(base)
}
