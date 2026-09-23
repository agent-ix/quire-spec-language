// SPDX-License-Identifier: AGPL-3.0-or-later
//! The caller step of complete-V1 package resolution (QSL-181): layer 3's
//! `complete::resolve_source_package` sees no syntax, so the layer that
//! parses refuses an inadmissible parse and hands resolution the source
//! authority and the selections of one and the same admitted parse.
//! [`resolve_parsed_source`] is that step; it is layer 6 (ADR-011 §6.1), the
//! one layer that may name both the layer-1 parse and layer-3 resolution.

use qsl_cst::{CompleteDiagnostic, ParsedSource};

use qsl_semantics::complete::{
    resolve_source_package, DefinitionCatalog, ModelCatalog, PackageLimits, PackageRefusal,
    ResolvedSourcePackage, SourceAuthority,
};

/// Why [`resolve_parsed_source`] produced no resolved package.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SourcePackageRefusal {
    /// The parse is not admissible; `diagnostic` is its first diagnostic,
    /// which classifies the source (its code and layer-1 cause).
    #[error("{} is not admissible: {:?} ({:?})", authority.path, diagnostic.code, diagnostic.cause)]
    Inadmissible {
        /// The refused source's identity, raw-byte digest and path.
        authority: Box<SourceAuthority>,
        /// The parse's first diagnostic.
        diagnostic: Box<CompleteDiagnostic>,
    },
    /// The parse is not admissible but recorded no diagnostic: the parser's
    /// established invariant broke.
    #[error("{} is not admissible and its parse recorded no diagnostic", authority.path)]
    UndiagnosedRecovery {
        /// The refused source's identity, raw-byte digest and path.
        authority: Box<SourceAuthority>,
    },
    /// Resolution refused the admitted source's selections.
    #[error(transparent)]
    Resolution(#[from] PackageRefusal),
}

/// Resolve `parsed`'s exact selections into a dependency-closed complete
/// bundle. An inadmissible parse is refused before resolution; otherwise the
/// source authority and the selections both come from `parsed`, so a
/// resolved package's `authority()` always names the source whose selections
/// were resolved.
pub fn resolve_parsed_source(
    parsed: &ParsedSource,
    catalog: &DefinitionCatalog,
    models: &ModelCatalog,
    limits: PackageLimits,
) -> Result<ResolvedSourcePackage, SourcePackageRefusal> {
    let authority = Box::new(SourceAuthority::of(parsed.source()));
    if !parsed.is_admissible() {
        return Err(match parsed.diagnostics().first() {
            Some(first) => SourcePackageRefusal::Inadmissible {
                authority,
                diagnostic: Box::new(first.clone()),
            },
            None => SourcePackageRefusal::UndiagnosedRecovery { authority },
        });
    }
    Ok(resolve_source_package(
        *authority,
        parsed.selections(),
        catalog,
        models,
        limits,
    )?)
}
