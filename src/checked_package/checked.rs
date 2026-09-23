// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 in-process checked-package
//! typestate ([`CheckedPackage`]) and its packaged wire counterpart
//! ([`EmittedPackage`]). Both are distinct from
//! `protocol_artifact::native::EmittedPackage` (SEAM-3, unrelated:
//! FR-087-AC-8/TC-247) and from `checking::CheckedPackage<'a>`
//! (lane-private, ADR-013 §6: O-15's canonical owner is this module's
//! `CheckedPackage`, not that one).
//!
//! `CheckedPackage`'s constructor ([`CheckedPackage::link`]) and both its
//! fields are private to this module: the only conversion into it is the S4
//! link step, over an already-checked [`CheckedGraph`] (S3's own stage
//! output, `qsl_semantics::check`) and other already-checked `CheckedPackage`s (E4's
//! dependency closure), never over an unchecked or wire-admitted value
//! (R-10, O-15) -- there is no `From`/`Into` impl from `VerifiedPackage`,
//! `ImportView` or any `protocol_artifact`-read value, and no struct-literal
//! construction reachable from outside this module (TC-244 rows 2-6).
//!
//! Every check-owned-type accessor (`Node`, `Signature`, and the rest of
//! `CheckedGraph`'s own accessor surface) is reached only by delegating
//! through [`CheckedPackage::graph`]: this module imports exactly one
//! layer-3 `check`-core item, `CheckedGraph` itself (FR-087-AC-9/TC-256).
//! `value::expression`'s own `CheckedPackageEvaluation` trait impl for this
//! type (`call`, `evaluate`, `emit_function_package_v2`) reaches
//! `check`-owned state through that same `graph()` accessor, under its own
//! separate, permitted layer-5-depends-on-layer-3 edge -- not routed
//! through this module, and outside this module's own one-item `check`-core
//! import count. A trait, not an inherent impl: once
//! `CheckedPackage` is `qsl-package`'s own foreign type (X-7), an inherent
//! `impl CheckedPackage` outside its defining crate is E0116.
//!
//! `EmittedPackage`'s constructor ([`EmittedPackage::new`]) is the v2
//! emitter (`CheckedPackage` -> these bytes, C-03), ADR-011 T-8 (M-4,
//! QSL-6/#242, slice S1a): [`super::emit`], a sibling module of this one, so
//! it stays within `checked_package`'s own `pub(super)` reach without being
//! reachable from outside `checked_package` (ADR-013 O-02: `package_id` is
//! computed from the package, never accepted from a caller).

use std::collections::BTreeMap;

use qsl_semantics::check::CheckedGraph;
use qsl_semantics::library::PackageId;

/// S4 in-process checked package (ADR-013 T-1): this package's own checked
/// declarations (a [`CheckedGraph`], S3's stage output) plus the checked
/// dependency closure E4 names. Both fields are private to this module
/// (ADR-011 §4); [`Self::link`] is the sole constructor.
///
/// TC-244 row 2 (FR-087-AC-2): a `CheckedGraph` never becomes a
/// `CheckedPackage` by any path other than [`CheckedPackage::link`] -- in
/// particular, not by naming this struct's private fields directly from
/// outside `checked_package`:
/// ```compile_fail,E0451
/// use quire_spec_language::checked_package::CheckedPackage;
/// let forged = CheckedPackage {
///     graph: todo!(),
///     dependencies: Default::default(),
/// };
/// ```
///
/// TC-244 row 6 (FR-087-AC-2) names a second forbidden path -- decoding
/// `EmittedPackage`'s wire bytes and forcing the result directly into a
/// `CheckedPackage`, bypassing the verified binding and the S1-S4 recompile
/// `replay`'s own E9 uses instead (ADR-013 T-2). No constructor accepting
/// decoded bytes is exposed at all; the only way to attempt it is the same
/// private struct literal row 2's doctest already forecloses, so row 6 is
/// covered by row 2 rather than carrying its own separate doctest.
#[derive(Debug)]
pub struct CheckedPackage {
    graph: CheckedGraph,
    /// E4's checked dependency closure: each imported identity's own
    /// checked package, compiled from its digest-addressed source and
    /// verified against the identity this package's own I2 resolution
    /// named. Always empty today: no #213 slice before S-3a gives the
    /// checker an import syntax to populate this from -- the `library`
    /// relocation that builds `PackageNodeKey`/`ImportView` resolution is
    /// FR-087's own later slice under QSL-158, and QSL-6/M-4 (the v2
    /// emitter and importer) is what actually populates a real dependency
    /// closure in production. The field is part of the type's shape now so
    /// that M-4 has a stable constructor to build against (FR-087-CON-1),
    /// not a claim that dependency resolution is implemented here.
    dependencies: BTreeMap<PackageId, CheckedPackage>,
}

impl CheckedPackage {
    /// The S4 link step (ADR-013 T-1): the sole conversion from
    /// `CheckedGraph` to `CheckedPackage`. Fed only by already-checked
    /// typestate -- a `CheckedGraph` -- never by an unchecked or
    /// wire-admitted value (R-10). The dependency closure starts empty:
    /// no #213 slice before S-3a gives the checker an import syntax to
    /// populate it from. M-4 adds the dependency-bearing step
    /// (`pub(crate)`, with verification) when it lands.
    pub fn link(graph: CheckedGraph) -> Self {
        Self {
            graph,
            dependencies: BTreeMap::new(),
        }
    }

    /// This package's own checked declarations (S3's stage output) -- the
    /// delegation point FR-087-AC-9/TC-256 names: every check-owned-type
    /// accessor a consumer needs is reached through this method, not by
    /// `checked_package` re-declaring or re-importing `check`'s types itself.
    pub fn graph(&self) -> &CheckedGraph {
        &self.graph
    }

    /// The checked dependency closure (E4): each imported identity's own
    /// checked package.
    pub fn dependencies(&self) -> &BTreeMap<PackageId, CheckedPackage> {
        &self.dependencies
    }
}

/// S4 wire output (ADR-013 T-1): the `quire.checked-package/v2` bytes with
/// their `package_id`. Distinct from `protocol_artifact::native::EmittedPackage`
/// (SEAM-3, unrelated: FR-087-AC-8/TC-247) -- two separate types under two
/// separate module paths, with no shared field, method or re-export, and
/// with no `pub use`/glob anywhere that would place both names in one import
/// scope.
///
/// A caller cannot supply a `package_id`: both fields are private, and this
/// type's own (`pub(super)`, so not documented on this public page)
/// constructor takes no `package_id` parameter to forge one through --
/// naming this struct's private fields directly from outside `checked_package`
/// (ADR-013 O-02) does not compile:
/// ```compile_fail,E0451
/// use quire_spec_language::checked_package::EmittedPackage;
/// let forged = EmittedPackage {
///     bytes: Vec::new(),
///     package_id: todo!(),
/// };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmittedPackage {
    bytes: Vec<u8>,
    package_id: PackageId,
}

impl EmittedPackage {
    /// The v2 emitter's sole constructor (ADR-011 T-8, M-4, QSL-6/#242,
    /// slice S1a). `identity_preimage` is IR's own typed
    /// `CheckedPackageIdentityPreimageV2`, JCS-encoded inside this
    /// constructor before `package_id` is minted from those bytes
    /// (`PackageId::of_preimage`, ADR-013 O-02): there is no parameter
    /// through which a caller could instead supply arbitrary preimage
    /// bytes, let alone an arbitrary `package_id` directly. `pub(super)`:
    /// reachable from anywhere in `checked_package` (in particular,
    /// `checked_package::emit`), never from outside it.
    #[allow(
        dead_code,
        reason = "no caller yet: checked_package::emit::emit_package has no success arm until QSL-6 slice S1b lands and calls this"
    )]
    pub(super) fn new(
        identity_preimage: &quire_contract_ir::CheckedPackageIdentityPreimageV2,
        bytes: Vec<u8>,
    ) -> Self {
        // Matches IR's own `digest_json` procedure: re-serialize through
        // `serde_json::Value` (whose `Map` sorts keys lexicographically
        // without this crate's `preserve_order` feature) before hashing,
        // rather than hashing the typed struct's own `Serialize` output
        // directly, which would emit fields in declaration order, not RFC
        // 8785 order.
        let preimage_value = serde_json::to_value(identity_preimage).expect(
            "CheckedPackageIdentityPreimageV2 is composed only of owned strings and vecs, \
             which always serialize",
        );
        let preimage_bytes = serde_json::to_vec(&preimage_value)
            .expect("a serde_json::Value re-serializes without error");
        Self {
            bytes,
            package_id: PackageId::of_preimage(&preimage_bytes),
        }
    }

    /// The emitted `quire.checked-package/v2` bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The `package_id` these bytes declare (O-02).
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_semantics::check::{CheckingLimits, PackageDeclarations};

    /// PR #300 review finding 1: `ModelCorrespondence` is recorded by a
    /// real `PackageDeclarations::check` run, from its own new
    /// `model_correspondence` field, and read back only through
    /// `CheckedGraph::resolve_declaration` -- not a hand-built
    /// `ModelCorrespondence` sitting outside the checker (FR-088-AC-2). The
    /// frame-subject *resolution mechanics* over that correspondence stay
    /// covered by `check::identity::tests::frame_subjects_resolve_only_through_the_recorded_correspondence`,
    /// since FR-340 frame syntax does not exist yet (FR-088-CON-2); this
    /// test is the "the checker really records it" half.
    ///
    /// PR #300 review round 2, MEDIUM-3: reads the correspondence through
    /// `CheckedPackage::link(graph).graph().resolve_declaration`,
    /// matching what FR-088-AC-2/ADR-013 O-04 itself names ("Consumers read
    /// the correspondence from the `CheckedPackage`") -- not `CheckedGraph`
    /// directly, which the prior version of this test read from.
    /// It lives here, in layer-4 `checked_package`, not in `check`'s own
    /// tests: it reads the layer-4 `CheckedPackage`, which a layer-3 test
    /// cannot name once `check` is its own crate (QSL-181 X-6a).
    #[trace("TC-248", "FR-088-AC-2")]
    #[test]
    fn model_correspondence_is_recorded_by_a_real_check_run() {
        let node = quire_exact::NodeKey::from_digest([7_u8; 32]);
        let declaration = qsl_semantics::model::key::DeclarationKey {
            package: "test/orders".to_owned(),
            node: "Order.status".to_owned(),
        };
        let graph = PackageDeclarations {
            model_correspondence: vec![(node, declaration.clone())],
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        .expect("an empty package with a correspondence seed checks cleanly");
        let package = CheckedPackage::link(graph);

        assert_eq!(
            package.graph().resolve_declaration(node),
            Some(&declaration)
        );

        // Adverse (R-05): a node the caller never supplied resolves to
        // nothing -- `check` never re-derives an entry by search.
        let other = quire_exact::NodeKey::from_digest([8_u8; 32]);
        assert_eq!(package.graph().resolve_declaration(other), None);
    }
}
