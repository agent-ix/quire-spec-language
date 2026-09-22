// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "no production caller yet: emit_package has no success arm until QSL-6 slice S1b lands; until then only this module's own tests reach any of it"
)]
//! ADR-011 T-8 (M-4, QSL-6/agent-ix/quire-spec-language#242), slice S1a:
//! the S4 v2 emitter's identity-preimage builder, the first step toward
//! [`CheckedPackage`] -> [`EmittedPackage`] (the `quire.checked-package/v2`
//! bytes with their own `package_id`, FR-322). Preimage construction is
//! delegated to `quire_contract_ir`'s typed I04 structs
//! ([`quire_contract_ir::CheckedPackageIdentityPreimageV2`] and the rest of
//! its `checked_package::v2` vocabulary): this module defines no wire
//! member set or contract-version spelling of its own, reusing
//! [`crate::library::PACKAGE_ID_VERSION`] for the identity preimage's own
//! `quire.checked-package-id/v2` version tag.
//!
//! # Scope (S1a) and what still blocks a real emission
//!
//! [`emit_package`] has no success arm yet: it always refuses. This
//! module builds the identity preimage that a later slice (S1b) needs in
//! order to mint `package_id` and assemble the full envelope
//! ([`EmittedPackage::new`], ADR-013 O-02's sole constructor, already
//! exists and takes no `package_id` parameter -- there is simply no
//! caller for it yet). Per the published schema's `IdentityPreimage`
//! definition (`agent-ix/quire-specification`,
//! `proposals/checked-package-v2/schema.json`), the preimage's members
//! are not uniformly shaped:
//!
//! - `profile_selections` and `dependency_selections` are
//!   `Selection[]` -- each entry a closed `role` plus a `DefinitionRef`
//!   (`quire.definition.bytes/v1`-domain digest of an actual definition
//!   artifact).
//! - `definition_selections` is a bare `DefinitionRef[]` (no `role`
//!   wrapper).
//! - `model_selections` is a bare `ModelRef[]` (`sha256-jcs`-domain
//!   digest of a domain package, not a definition).
//! - `required_features` is a plain non-empty-string array, no artifact
//!   reference at all.
//!
//! No part of this crate tracks any of these five yet, so all five are
//! correctly, honestly empty ([`identity_preimage`]). `identity_projection`
//! is likewise empty: its node bodies (ADR-011 §6.1's emission arm per
//! family, matching `check::ir::NodeKind`) are QSL-6 slice S1b
//! ([`identity_projection`]).
//!
//! `edition` is different in kind from the five above: it is FR-322's one
//! *non-list* lock-selection member, and IR's
//! [`quire_contract_ir::CheckedSelection`] requires a real
//! `CheckedArtifactRef` (authority, identity, revision and a digest of the
//! actual locked edition definition's bytes) -- there is no empty value
//! for it to fall back to. `value::definition`'s qualification catalog
//! names the fixed authority/identity/revision for `CatalogRole::Edition`
//! (`edition.md`) but -- by that catalog's own documented design -- never
//! resolves or retains a content digest ("a digest exists only to verify
//! a copy, and this table holds none"), and neither `CheckedGraph` nor
//! `CheckedPackage` carries a `value::definition::DefinitionReference` at
//! all today. There is therefore no real `edition` value anywhere in this
//! crate to emit. Fabricating one (a synthetic digest, an empty object)
//! would be exactly the placeholder-bytes failure mode this slice must
//! not reproduce, so [`edition_selection`] returns [`None`] and
//! [`emit_package`] refuses ([`EmitRefusal::EditionNotYetSelected`])
//! rather than emit. Closing this gap needs real definition-lock evidence
//! threaded into `CheckedPackage` (or an equivalent caller-supplied
//! input) -- new cross-layer plumbing outside a review-finding fix, not
//! something to invent here.

use std::convert::Infallible;

use quire_contract_ir::{CheckedPackageIdentityPreimageV2, CheckedSelection};

use crate::check::CheckedGraph;
use crate::Code;

use super::CheckedPackage;

/// Why the v2 emitter refuses to emit `package`'s bytes. FR-062's
/// packaging rule: a refusal yields no v2 bytes for the whole package,
/// never a partial one.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum EmitRefusal {
    /// No real `lock.edition`/`identity_preimage.edition` selection exists
    /// yet (see the module doc): this crate has no evidenced
    /// `CheckedArtifactRef` for the edition definition it checked against.
    #[error("no edition-selection evidence exists yet to emit a checked-package/v2 lock")]
    EditionNotYetSelected,
}

impl EmitRefusal {
    /// The catalog code (FR-010). A real, admitted capability this build
    /// does not implement yet -- not a fault in `package`'s own content --
    /// so this is `is_unsupported()`, not a source-data code.
    pub(crate) fn code(self) -> Code {
        match self {
            Self::EditionNotYetSelected => Code::UnsupportedProjection,
        }
    }
}

/// The lock's `edition` selection, if this crate has real evidence for one
/// (see the module doc). Always [`None`] today: no path from
/// `value::definition`'s qualification catalog or from `CheckedGraph`
/// retains an evidenced digest of the edition definition a package was
/// actually checked against.
fn edition_selection(_package: &CheckedPackage) -> Option<CheckedSelection> {
    None
}

/// `identity_projection` (FR-322): one entry per family-emitted v2 node
/// (ADR-011 §6.1's emission arm per family, matching `check::ir::NodeKind`;
/// QSL-6 slice S1b). `CheckedGraph` exposes no such per-family dispatch
/// yet, so this returns no nodes until S1b lands.
fn identity_projection(_graph: &CheckedGraph) -> Vec<quire_contract_ir::CheckedNodeProjectionV2> {
    Vec::new()
}

/// The `quire.checked-package-id/v2` identity preimage (FR-322), or a
/// refusal where this crate has no real data for a required member (see
/// the module doc).
fn identity_preimage(
    package: &CheckedPackage,
) -> Result<CheckedPackageIdentityPreimageV2, EmitRefusal> {
    let edition = edition_selection(package).ok_or(EmitRefusal::EditionNotYetSelected)?;
    Ok(CheckedPackageIdentityPreimageV2 {
        version: crate::library::PACKAGE_ID_VERSION.into(),
        edition,
        profile_selections: Vec::new(),
        definition_selections: Vec::new(),
        model_selections: Vec::new(),
        required_features: Vec::new(),
        dependency_selections: Vec::new(),
        identity_projection: identity_projection(package.graph()),
    })
}

/// The S4 v2 emitter's current, honest capability (ADR-011 T-8, M-4): it
/// can build `package`'s identity preimage, but has no success arm yet
/// (see the module doc) -- `Infallible` on the `Ok` side makes that a
/// structural fact, not just today's behavior. S1b adds the arm that
/// mints `package_id` from the preimage and calls
/// [`EmittedPackage::new`](super::EmittedPackage::new).
pub(crate) fn emit_package(package: &CheckedPackage) -> Result<Infallible, EmitRefusal> {
    match identity_preimage(package) {
        Ok(_) => unreachable!(
            "S1b adds emit_package's success arm; identity_preimage cannot succeed before it lands"
        ),
        Err(refusal) => Err(refusal),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{CheckingLimits, PackageDeclarations};
    use crate::forms::{Expression, FunctionDeclaration};
    use crate::value::ValueType;

    /// An empty checked package: no functions. `CheckedPackage::link`'s
    /// dependency closure always starts empty today (S-3a review fix); M-4
    /// adds the dependency-bearing constructor when it lands.
    fn empty() -> CheckedPackage {
        let graph = PackageDeclarations::default()
            .check(CheckingLimits::default())
            .expect("an empty package always checks");
        CheckedPackage::link(graph)
    }

    fn function(name: &str) -> FunctionDeclaration {
        FunctionDeclaration::new(
            name,
            Vec::new(),
            ValueType::Boolean,
            None,
            Expression::Boolean(true),
        )
    }

    /// A checked package with one function, distinguishing it from
    /// [`empty`] at the `check`-core level.
    fn with_function(name: &str) -> CheckedPackage {
        let graph = PackageDeclarations {
            functions: vec![function(name)],
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        .expect("a single nullary function always checks");
        CheckedPackage::link(graph)
    }

    /// No package can be emitted today (module doc): `edition_selection`
    /// has no real data source, regardless of what else the package
    /// carries. This is the current, honest, and only reachable behavior
    /// of [`emit_package`] -- not a placeholder assertion of future work.
    #[test]
    fn refuses_every_package_for_lack_of_an_edition_selection() {
        for package in [empty(), with_function("f")] {
            let refusal = emit_package(&package).unwrap_err();
            assert_eq!(refusal, EmitRefusal::EditionNotYetSelected);
            assert_eq!(refusal.code(), Code::UnsupportedProjection);
        }
    }

    /// The refusal is deterministic, not just present.
    #[test]
    fn the_edition_refusal_is_deterministic() {
        let package = with_function("f");
        assert_eq!(
            emit_package(&package).unwrap_err(),
            emit_package(&package).unwrap_err()
        );
    }
}
