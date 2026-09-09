// SPDX-License-Identifier: AGPL-3.0-only
//! Compare the actual private canonical pass separately from its digest.

use crate::runtime_test_setup::native_rule_model;
#[path = "../../tests/support/package_vector_setup.rs"]
mod package_vector_setup;

use ix_trace_rs::trace;

use super::*;

#[test]
#[trace("TC-090", "FR-021-AC-1", "FR-021-AC-6")]
fn canonical_pass_bytes_match_each_independent_fixed_vector_before_hashing() {
    let models = [native_rule_model::from_text(
        include_str!("../../tests/fixtures/native-package/model-source.json"),
        "fixed-model.json",
        "1",
    )
    .unwrap()
    .model()];
    for vector in package_vector_setup::cases() {
        let checked = package_vector_setup::checked(&vector, &models);
        let features = features::derive(&checked).unwrap();
        let mut usage = PackageUsage::default();
        let bytes = run(
            &view::Manifest::new(&checked, &features, false, None),
            PackageLimits::default(),
            Pass::Canonical,
            &mut usage,
        )
        .unwrap();
        assert_eq!(bytes, vector.canonical, "{} canonical bytes", vector.name);
        assert_eq!(
            usage.canonical.unwrap().output_bytes,
            vector.canonical.len()
        );
        assert!(usage.encode.is_none());
        let package = NativePackage::new(checked, PackageLimits::default()).unwrap();
        assert_eq!(package.bytes(), vector.artifact);
        assert_eq!(
            package.canonical_identity().to_string(),
            vector.digest.strip_suffix('\n').unwrap()
        );
    }
}
