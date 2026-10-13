//! Test-only [`DomainPackageRef`] fixtures. They hash a placeholder, so they live
//! apart from `domain_package.rs`, which names `serde_json` (ADR-013 §2).

use super::domain_package::DomainPackageRef;

impl DomainPackageRef {
    /// A `test/orders` version-`1` selection whose digest is the SHA-256 of
    /// `placeholder`'s exact UTF-8 bytes, following TC-195's own placeholder
    /// selection convention: F1 selects `n01`, F2 selects `n02`, and F1 as
    /// version `2` selects `n01v2`.
    ///
    /// Test-only (PR #140 F13): this derives a digest from a display
    /// identity, which is exactly the name-derived-identity defect this
    /// engine exists to exclude. Gated behind `test-support` so a production
    /// caller cannot reach it; `cargo test --all-features` enables it for
    /// `tests/model_normalization.rs`.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture(placeholder: impl Into<String>) -> Self {
        Self::fixture_with_version(placeholder, "1")
    }

    /// [`Self::fixture`], with an explicit `version` rather than the
    /// hard-wired `"1"` — needed to reach F1 as version `2` under
    /// placeholder `n01v2` (TC-195 N09's third clause).
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_with_version(
        placeholder: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            identity: "test/orders".to_owned(),
            version: version.into(),
            digest: qsl_foundation::ByteDigest::of(placeholder.into().as_bytes()).as_bytes(),
        }
    }
}
