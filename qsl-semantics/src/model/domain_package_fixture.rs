//! Test-only [`DomainPackageRef`] fixtures. They live apart from
//! `domain_package.rs`, which names `serde_json` (ADR-013 §2).

use super::domain_package::DomainPackageRef;

impl DomainPackageRef {
    /// A `test/orders` version-`1` selection. `placeholder` is accepted for
    /// call-site readability and names nothing: a package is selected by its
    /// identity alone.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture(placeholder: impl Into<String>) -> Self {
        Self::fixture_with_version(placeholder, "1")
    }

    /// [`Self::fixture`], with an explicit `version`.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_with_version(
        placeholder: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        let _ = placeholder.into();
        Self {
            identity: "test/orders".to_owned(),
            version: version.into(),
        }
    }
}
