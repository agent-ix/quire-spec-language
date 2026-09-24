// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "no production caller yet: emit_package has no success arm until QSL-6 slice S1b lands; until then only this module's own tests reach any of it"
)]
//! ADR-011 T-8 (M-4, QSL-6/agent-ix/quire-spec-language#242), slice S1a:
//! the S4 v2 emitter's identity-preimage builder, the first step toward
//! [`CheckedPackage`] -> [`EmittedPackage`](super::EmittedPackage) (the `quire.checked-package/v2`
//! bytes with their own `package_id`, FR-322). Preimage construction is
//! delegated to `quire_contract_ir`'s typed I04 structs
//! ([`quire_contract_ir::CheckedPackageIdentityPreimageV2`] and the rest of
//! its `checked_package::v2` vocabulary): this module defines no wire
//! member set or contract-version spelling of its own, reusing
//! [`qsl_semantics::library::PACKAGE_ID_VERSION`] for the identity preimage's own
//! `quire.checked-package-id/v2` version tag.
//!
//! # Scope (S1a) and what still blocks a real emission
//!
//! [`emit_package`] has no success arm yet: it always refuses. This
//! module builds the identity preimage that a later slice (S1b) needs in
//! order to mint `package_id` and assemble the full envelope
//! ([`EmittedPackage::new`](super::EmittedPackage::new), ADR-013 O-02's sole constructor, already
//! exists and takes IR's own typed preimage -- there is simply no caller
//! for it yet). [`quire_contract_ir::CheckedPackageIdentityPreimageV2`]'s
//! own field types ([`quire_contract_ir::CheckedSelection`],
//! [`quire_contract_ir::CheckedArtifactRef`],
//! [`quire_contract_ir::CheckedDomainPackageRef`]) define every member's
//! exact shape; this doc does not restate it in prose, so the compiler,
//! not this comment, is what stays honest as IR's schema evolves. Whether
//! the schema's `dependency_selections` shape (a bare `CheckedSelection`,
//! the same shape `profile_selections` uses) is the right one for
//! FR-322-AC-26 is an open question this slice does not settle -- nothing
//! here should be read as resolving it.
//!
//! No part of this crate tracks `profile_selections`,
//! `definition_selections`, `model_selections`, `required_features` or
//! `dependency_selections` yet, so all five are correctly, honestly empty
//! ([`identity_preimage`]). `identity_projection` is also empty for a
//! package with no declaration, but refuses
//! ([`EmitRefusal::ProjectionNotYetImplemented`]) for one with any: `check`
//! already lowers every node and mints its key
//! ([`CheckedGraph::semantic_graph`], FR-092, FR-093, FR-094; ADR-011 FB-13:
//! only `check` mints a `NodeKey`), and QSL-6 slice S1b only serializes
//! those nodes into the projection ([`identity_projection`]). An empty
//! projection for a non-empty graph would mint a `package_id` that ignores
//! the package's content (prior review finding 3).
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

use qsl_foundation::Code;
use qsl_semantics::check::CheckedGraph;

use super::CheckedPackage;

/// Why the v2 emitter refuses to emit `package`'s bytes. FR-062-AC-9's
/// packaging rule is per item, not per package: a fault partway through
/// one checked item's v2 emission yields no v2 bytes for that item and a
/// refusal, never a partial node set for it, while other items in the same
/// package are unaffected. This enum's own refusals are broader than
/// AC-9's fault-injection scenario (S1a has no real data to emit at all
/// yet, for any item), but they are still per-item in the same sense: a
/// refusal here means "no v2 bytes for the affected item," not "no v2
/// bytes for the whole package."
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum EmitRefusal {
    /// No real `lock.edition`/`identity_preimage.edition` selection exists
    /// yet (see the module doc): this crate has no evidenced
    /// `CheckedArtifactRef` for the edition definition it checked against.
    #[error("no edition-selection evidence exists yet to emit a checked-package/v2 lock")]
    EditionNotYetSelected,
    /// The serialization of `check`'s keyed nodes into
    /// `identity_projection` (QSL-6 slice S1b) does not exist yet: a package
    /// with any declaration cannot be projected today, since an empty
    /// projection would ignore that declaration and mint a `package_id`
    /// blind to the package's actual content (prior review finding 3).
    #[error("identity_projection is not implemented yet for a package with any declaration")]
    ProjectionNotYetImplemented,
    /// Defensive: `emit_package`'s own success arm (minting `package_id`
    /// and calling `EmittedPackage::new`) does not exist until QSL-6 slice
    /// S1b lands, even for the rare package `identity_preimage` could
    /// otherwise build in full today (see the module doc). Guards against
    /// a panic if a future edit to `edition_selection` alone made
    /// `identity_preimage` start succeeding before S1b's emission arm is
    /// wired in to actually consume the result.
    #[error("checked-package/v2 emission is not implemented yet")]
    EmissionNotYetImplemented,
}

impl EmitRefusal {
    /// The catalog code (FR-010). Every variant here names a real,
    /// admitted capability this build does not implement yet -- not a
    /// fault in `package`'s own content -- so all three are
    /// `is_unsupported()`, not a source-data code (see
    /// [`Code::UnsupportedProjection`]'s own doc).
    pub(crate) fn code(self) -> Code {
        match self {
            Self::EditionNotYetSelected
            | Self::ProjectionNotYetImplemented
            | Self::EmissionNotYetImplemented => Code::UnsupportedProjection,
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

/// `identity_projection` (FR-322): one entry per node `check` lowered and
/// keyed ([`CheckedGraph::semantic_graph`]); QSL-6 slice S1b serializes
/// them, and mints no key of its own. Until it does, a graph with no
/// declaration correctly has an empty projection, but a graph with any
/// declaration refuses rather than silently return an empty projection
/// that would ignore it and mint a content-blind `package_id` (prior
/// review finding 3).
fn identity_projection(
    graph: &CheckedGraph,
) -> Result<Vec<quire_contract_ir::CheckedNodeProjectionV2>, EmitRefusal> {
    if graph.function_identities().next().is_some() {
        return Err(EmitRefusal::ProjectionNotYetImplemented);
    }
    Ok(Vec::new())
}

/// The `quire.checked-package-id/v2` identity preimage (FR-322), or a
/// refusal where this crate has no real data for a required member (see
/// the module doc).
fn identity_preimage(
    package: &CheckedPackage,
) -> Result<CheckedPackageIdentityPreimageV2, EmitRefusal> {
    let edition = edition_selection(package).ok_or(EmitRefusal::EditionNotYetSelected)?;
    let identity_projection = identity_projection(package.graph())?;
    Ok(CheckedPackageIdentityPreimageV2 {
        version: qsl_semantics::library::PACKAGE_ID_VERSION.into(),
        edition,
        profile_selections: Vec::new(),
        definition_selections: Vec::new(),
        model_selections: Vec::new(),
        required_features: Vec::new(),
        dependency_selections: Vec::new(),
        identity_projection,
    })
}

/// The S4 v2 emitter's current, honest capability (ADR-011 T-8, M-4): it
/// can build `package`'s identity preimage, but has no success arm yet
/// (see the module doc) -- `Infallible` on the `Ok` side makes that a
/// structural fact, not just today's behavior. S1b adds the arm that
/// mints `package_id` from the preimage and calls
/// [`EmittedPackage::new`](super::EmittedPackage::new). The `Ok(_)` arm
/// below is a refusal, not a panic (rust-review: no panic on a path whose
/// safety depends on another function's current body): `identity_preimage`
/// cannot succeed today only because `edition_selection` always returns
/// `None`, a fact this function must not assume stays true forever.
pub(crate) fn emit_package(package: &CheckedPackage) -> Result<Infallible, EmitRefusal> {
    match identity_preimage(package) {
        Ok(_) => Err(EmitRefusal::EmissionNotYetImplemented),
        Err(refusal) => Err(refusal),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qsl_forms::{Expression, FunctionDeclaration, TypeForm};
    use qsl_semantics::check::{CheckingLimits, PackageDeclarations};

    /// An empty checked package: no functions. `CheckedPackage::link`'s
    /// dependency closure always starts empty today (S-3a review fix); M-4
    /// adds the dependency-bearing constructor when it lands.
    fn empty() -> CheckedPackage {
        let graph = PackageDeclarations::new(qsl_semantics::check::fixture_source())
            .check(CheckingLimits::default())
            .expect("an empty package always checks");
        CheckedPackage::link(graph)
    }

    fn function(name: &str) -> FunctionDeclaration {
        FunctionDeclaration::new(
            name,
            Vec::new(),
            TypeForm::builtin(
                qsl_forms::BuiltinType::Boolean,
                qsl_foundation::Span { start: 0, end: 0 },
            ),
            None,
            Expression::Boolean(true),
        )
    }

    /// A checked package with one function, distinguishing it from
    /// [`empty`] at the `check`-core level.
    fn with_function(name: &str) -> CheckedPackage {
        let graph = PackageDeclarations {
            functions: vec![function(name)],
            ..PackageDeclarations::new(qsl_semantics::check::fixture_source())
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

    /// `identity_projection` for a graph with no declaration is correctly
    /// empty, not a refusal.
    #[test]
    fn identity_projection_is_empty_for_a_graph_with_no_declaration() {
        let package = empty();
        assert_eq!(identity_projection(package.graph()), Ok(Vec::new()));
    }

    /// `identity_projection` refuses, rather than silently drop, any
    /// declaration a graph carries (F2/prior finding 3): an empty
    /// projection here would mint a `package_id` blind to the package's
    /// actual content.
    #[test]
    fn identity_projection_refuses_a_graph_with_any_declaration() {
        let package = with_function("f");
        assert_eq!(
            identity_projection(package.graph()),
            Err(EmitRefusal::ProjectionNotYetImplemented)
        );
    }
}
