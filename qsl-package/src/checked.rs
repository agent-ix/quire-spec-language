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
//! import count. A trait, not an inherent impl: `CheckedPackage` is this
//! crate's type, and an inherent `impl CheckedPackage` outside its defining
//! crate is E0116.
//!
//! `EmittedPackage`'s constructor ([`EmittedPackage::new`]) is the v2
//! emitter (`CheckedPackage` -> these bytes, C-03), ADR-011 T-8 (M-4,
//! QSL-6/#242, slice S1a): [`super::emit`], a sibling module of this one, so
//! it stays within this crate's `pub(super)` reach without being
//! reachable from outside `qsl-package` (ADR-013 O-02: `package_id` is
//! computed from the package, never accepted from a caller).

use std::collections::BTreeMap;

use qsl_semantics::check::CheckedGraph;
use qsl_semantics::library::PackageId;
use qsl_semantics::value::IDENTITY_LIMITS;

/// S4 in-process checked package (ADR-013 T-1): this package's own checked
/// declarations (a [`CheckedGraph`], S3's stage output) plus the checked
/// dependency closure E4 names. Both fields are private to this module
/// (ADR-011 §4); [`Self::link`] is the sole constructor.
///
/// TC-244 row 2 (FR-087-AC-2): a `CheckedGraph` never becomes a
/// `CheckedPackage` by any path other than [`CheckedPackage::link`] -- in
/// particular, not by naming this struct's private fields directly from
/// outside `qsl-package`:
/// ```compile_fail,E0451
/// use qsl_package::CheckedPackage;
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
///
/// Stable rustdoc does not check a `compile_fail` block's error code, so a
/// renamed or moved import would make each block below fail for the wrong
/// reason and still pass. Each is paired with a block that must compile,
/// over the same `use` lines and the same value, so a rename or a move
/// breaks the build instead.
///
/// TC-244 row 2's pair:
/// ```no_run
/// use qsl_package::CheckedPackage;
/// fn holds(package: CheckedPackage) -> CheckedPackage {
///     package
/// }
/// ```
///
/// TC-244 row 3 (FR-087-AC-2, R-10): a wire-admitted `VerifiedPackage`
/// has no conversion into a `CheckedPackage`:
/// ```compile_fail,E0277
/// use qsl_package::CheckedPackage;
/// use qsl_semantics::library::VerifiedPackage;
/// fn forge(verified: VerifiedPackage) -> CheckedPackage {
///     verified.into()
/// }
/// ```
/// Its pair:
/// ```no_run
/// use qsl_package::CheckedPackage;
/// use qsl_semantics::library::VerifiedPackage;
/// fn holds(verified: VerifiedPackage, package: CheckedPackage) -> CheckedPackage {
///     drop(verified);
///     package
/// }
/// ```
///
/// TC-244 row 4: nor has an `ImportView`:
/// ```compile_fail,E0277
/// use qsl_package::CheckedPackage;
/// use qsl_semantics::library::ImportView;
/// fn forge(view: ImportView) -> CheckedPackage {
///     view.into()
/// }
/// ```
/// Its pair:
/// ```no_run
/// use qsl_package::CheckedPackage;
/// use qsl_semantics::library::ImportView;
/// fn holds(view: ImportView, package: CheckedPackage) -> CheckedPackage {
///     drop(view);
///     package
/// }
/// ```
///
/// Row 5, a `protocol_artifact`-read value, is at the root crate's
/// `protocol_artifact::AdmittedPackage`: only the root crate can name both
/// it and `CheckedPackage`.
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
    /// `qsl-package` re-declaring or re-importing `check`'s types itself.
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
/// naming this struct's private fields directly from outside `qsl-package`
/// (ADR-013 O-02) does not compile:
/// ```compile_fail,E0451
/// use qsl_package::EmittedPackage;
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
    /// reachable from anywhere in `qsl-package` (in particular,
    /// `emit`), never from outside it.
    #[allow(
        dead_code,
        reason = "no caller yet: emit::emit_package has no success arm until QSL-6 slice S1b lands and calls this"
    )]
    pub(super) fn new(
        identity_preimage: &quire_contract_ir::CheckedPackageIdentityPreimageV2,
        bytes: Vec<u8>,
    ) -> Self {
        // RFC 8785 bytes from `quire-canonical` (ADR-013 §2, ADR-013:113:
        // the one RFC 8785 implementation), encoded straight from the typed
        // preimage: the encoder orders members itself. A preimage of owned
        // strings and vectors always has an encoding, and `LIMITS` sets no
        // byte ceiling; the one refusal left is a failed heap reservation,
        // which the `serde_json` encoder this replaced aborted on.
        let preimage_bytes = quire_canonical::to_vec(identity_preimage, IDENTITY_LIMITS)
            .unwrap_or_else(|error| panic!("CheckedPackageIdentityPreimageV2 encodes: {error}"));
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

    /// FR-094 (TC-417), through the layer-4 `CheckedPackage`: `check` is
    /// the model correspondence's only writer. A function over a
    /// `Reference<M::A>` parameter keys `model.A`'s model node and records
    /// it, read back through `CheckedPackage::graph().resolve_declaration`
    /// (FR-088-AC-2, ADR-013 O-04: "Consumers read the correspondence from
    /// the `CheckedPackage`"). It lives here, in layer-4 `qsl-package`: a
    /// layer-3 test cannot name `CheckedPackage` (QSL-181 X-6a).
    #[trace("TC-248", "FR-088-AC-2", "TC-417", "FR-094-AC-2")]
    #[test]
    fn model_correspondence_is_recorded_by_a_real_check_run() {
        use qsl_semantics::check::{AdmittedModel, NodeTag};
        use qsl_semantics::model::accounting::ModelNormalizationLimits;
        use qsl_semantics::model::domain_package::{
            DomainPackage, DomainPackageRecord, DomainPackageRef, ObjectTypeRecord,
        };
        use qsl_semantics::model::key::DeclarationKey;
        use qsl_semantics::model::normalize::{normalize, NormalizeOutcome};
        use qsl_semantics::value::declaration::{ObjectTypeDeclaration, TypeEnvironment};

        let declaration = DeclarationKey::fixture("model.A");
        let domain_package = DomainPackage::new(
            DomainPackageRef::fixture("bundle.n01"),
            vec![DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: declaration.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: Vec::new(),
            })],
        );
        let NormalizeOutcome::Completed(view) =
            normalize(&domain_package, ModelNormalizationLimits::UNLIMITED)
        else {
            panic!("the one-type domain package normalizes");
        };
        let a = view.type_identities()[&declaration];
        let span = qsl_foundation::Span { start: 0, end: 0 };
        let graph = PackageDeclarations {
            types: TypeEnvironment::new([], [ObjectTypeDeclaration::new(a, "M::A", vec![])])
                .expect("one object type admits"),
            models: vec![AdmittedModel::new(&domain_package, &view).expect("the view is its own")],
            functions: vec![qsl_forms::FunctionDeclaration::new(
                "f",
                vec![("r".to_owned(), qsl_forms::TypeForm::name("M::A", span))],
                qsl_forms::TypeForm::builtin(qsl_forms::BuiltinType::Boolean, span),
                None,
                qsl_forms::Expression::Boolean(true),
            )],
            ..PackageDeclarations::new(qsl_semantics::check::fixture_source())
        }
        .check(CheckingLimits::default())
        .expect("a function over a model reference checks");
        let model_nodes: Vec<quire_exact::NodeKey> = graph
            .semantic_graph()
            .nodes()
            .filter(|node| node.node_tag() == NodeTag::Model)
            .map(|node| node.key())
            .collect();
        assert_eq!(model_nodes.len(), 1, "one model node: model.A's");
        let package = CheckedPackage::link(graph);
        assert_eq!(
            package.graph().resolve_declaration(model_nodes[0]),
            Some(&declaration)
        );

        // Adverse (R-05): a node `check` never keyed resolves to nothing.
        let other = quire_exact::NodeKey::from_digest([8_u8; 32]);
        assert_eq!(package.graph().resolve_declaration(other), None);
    }

    /// QSL-194 golden vector for `package_id`
    /// (`quire.package.semantic/v2`): the identity preimage's RFC 8785 text,
    /// written out by hand, and its SHA-256. `EmittedPackage::new` encodes
    /// IR's typed preimage through `quire-canonical`, and the `serde_json`
    /// `Value` round trip it replaced emitted the same text, so the minted
    /// `package_id` is unchanged.
    #[trace("TC-253", "FR-087-AC-3")]
    #[test]
    fn package_id_matches_its_golden_vector() {
        use sha2::{Digest as _, Sha256};
        const TEXT: &str = r#"{"definition_selections":[],"dependency_selections":[],"edition":{"definition":{"authority":"pkg","digest":"1c3a0ee911df60393f84d48f2779a0d9c39df4bef16ac5dd6bb5c1620be1eda9","digest_domain":"quire.definition.bytes/v1","identity":"edition-def","revision":{"namespace":"semver","value":"1"}},"role":"edition"},"identity_projection":[{"body":{"term":"literal","type":{"digest":"afbb1f4913f26cb385723382e760adb9d35629d8b46d83ff73bb33b0caf50768","domain":"quire.checked-semantic-node/v1"},"value":true,"value_kind":"boolean"},"declaration":{"qualified_name":["R"]},"dependencies":[],"node_id":{"digest":"afbb1f4913f26cb385723382e760adb9d35629d8b46d83ff73bb33b0caf50768","domain":"quire.checked-semantic-node/v1"},"node_tag":"scalar_type","schema_version":"quire.checked-semantic-graph/v2","semantic_form":"boolean","semantic_type":{"digest":"afbb1f4913f26cb385723382e760adb9d35629d8b46d83ff73bb33b0caf50768","domain":"quire.checked-semantic-node/v1"}}],"model_selections":[],"profile_selections":[],"required_features":[],"version":"quire.checked-package-id/v2"}"#;
        const DIGEST: &str = "5023edc801f355fbf8299ff374a5e9e3b3fc50ca51961947eecc5b403367f6b4";
        assert_eq!(format!("{:x}", Sha256::digest(TEXT.as_bytes())), DIGEST);
        let preimage: quire_contract_ir::CheckedPackageIdentityPreimageV2 =
            serde_json::from_str(TEXT).expect("the vector is a v2 identity preimage");
        assert_eq!(
            EmittedPackage::new(&preimage, Vec::new())
                .package_id()
                .hex(),
            DIGEST
        );
        let value: serde_json::Value = serde_json::from_str(TEXT).unwrap();
        assert_eq!(serde_json::to_string(&value).unwrap(), TEXT);
    }
}
