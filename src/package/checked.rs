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
//! output, `crate::check`) and other already-checked `CheckedPackage`s (E4's
//! dependency closure), never over an unchecked or wire-admitted value
//! (R-10, O-15) -- there is no `From`/`Into` impl from `VerifiedPackage`,
//! `ImportView` or any `protocol_artifact`-read value, and no struct-literal
//! construction reachable from outside this module (TC-244 rows 2-6).
//!
//! Every check-owned-type accessor (`Node`, `Signature`, and the rest of
//! `CheckedGraph`'s own accessor surface) is reached only by delegating
//! through [`CheckedPackage::graph`]: this module imports exactly one
//! layer-3 `check`-core item, `CheckedGraph` itself (FR-087-AC-9/TC-256).
//! `value::expression`'s own inherent `impl CheckedPackage` (`call`,
//! `evaluate`, the v2 function-identity codec) reaches `check`-owned state
//! through that same `graph()` accessor, under its own separate, permitted
//! layer-5-depends-on-layer-3 edge -- not routed through this module, and
//! outside this module's own one-item `check`-core import count.
//!
//! `EmittedPackage`'s constructor ([`EmittedPackage::new`]) is the v2
//! emitter (`CheckedPackage` -> these bytes, C-03), ADR-011 T-8 (M-4,
//! QSL-6/#242, slice S1a): [`crate::package::emit`], a sibling module of
//! this one, so it stays within `package`'s own `pub(super)` reach without
//! being reachable from outside `package` (ADR-013 O-02: `package_id` is
//! computed from the package, never accepted from a caller).

use std::collections::BTreeMap;

use crate::check::CheckedGraph;
use crate::library::PackageId;

/// S4 in-process checked package (ADR-013 T-1): this package's own checked
/// declarations (a [`CheckedGraph`], S3's stage output) plus the checked
/// dependency closure E4 names. Both fields are private to this module
/// (ADR-011 §4); [`Self::link`] is the sole constructor.
///
/// TC-244 row 2 (FR-087-AC-2): a `CheckedGraph` never becomes a
/// `CheckedPackage` by any path other than [`CheckedPackage::link`] -- in
/// particular, not by naming this struct's private fields directly from
/// outside `package`:
/// ```compile_fail,E0451
/// use quire_spec_language::package::CheckedPackage;
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
    /// `package` re-declaring or re-importing `check`'s types itself.
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
/// naming this struct's private fields directly from outside `package`
/// (ADR-013 O-02) does not compile:
/// ```compile_fail,E0451
/// use quire_spec_language::package::EmittedPackage;
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
    /// slice S1a). `package_id` is minted here, from `identity_preimage`'s
    /// own RFC 8785 JCS bytes (`PackageId::of_preimage`, ADR-013 O-02) --
    /// there is no parameter through which a caller could instead supply an
    /// arbitrary `package_id`. `pub(super)`: reachable from anywhere in
    /// `package` (in particular, `package::emit`), never from outside it.
    #[allow(
        dead_code,
        reason = "no caller yet: package::emit::emit_package has no success arm until QSL-6 slice S1b lands and calls this"
    )]
    pub(super) fn new(identity_preimage: &[u8], bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            package_id: PackageId::of_preimage(identity_preimage),
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
