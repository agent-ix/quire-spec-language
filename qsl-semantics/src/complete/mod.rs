// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 package selection (ADR-011 §6.2: layer 3 `library`).
//!
//! `resolve_source_package` takes the source's selections as
//! `qsl_foundation::selection` values the caller pulled out of its own parse
//! (lead ruling R1, QSL-181): this crate never parses and names no layer-1
//! type. The editor tooling that parses stays in the root crate's own
//! `complete` module.

mod package;
#[cfg(test)]
mod package_tests;

pub use package::{
    resolve_source_package, CompleteBundle, Definition, DefinitionCatalog, DefinitionConflict,
    DefinitionRole, Facet, ModelArtifact, ModelCatalog, PackageLimits, PackageRefusal,
    ReaderAuthority, ResolutionCause, ResolvedSourcePackage, SemanticDigest, SourceAuthority,
};
pub use package::{CapabilityId, PackageError, SourceDigest};
