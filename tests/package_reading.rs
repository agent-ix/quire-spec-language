// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-020: real compiler reconstruction from independently frozen package bytes.

#[path = "package_reading_cases/mod.rs"]
mod cases;
#[path = "support/native_rule_model.rs"]
mod native_rule_model;
#[path = "support/package_vector_setup.rs"]
mod package_vector_setup;

use ix_trace_rs::trace;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::package::{NativePackage, PackageLimits};
use quire_spec_language::package::{
    NativePackageRef, PackageReadLimits, PackageStage, PackageSupport,
};
use quire_spec_language::{ByteDigest, Code};

fn model() -> NativeModel {
    native_rule_model::from_text(
        include_str!("fixtures/native-package/model-source.json"),
        "package-model.json",
        "1",
    )
    .expect("source-derived model fixture must decode")
    .model()
}

#[test]
#[trace("TC-082", "TC-087", "FR-020-AC-8", "FR-020-AC-10")]
fn independent_reader_fixtures_have_successful_real_compiler_setup() {
    let models = [model()];
    assert_eq!(
        models[0].artifact_bytes(),
        include_bytes!("fixtures/native-package/model-artifact.json")
    );
    for vector in package_vector_setup::cases() {
        let checked = package_vector_setup::checked(&vector, &models);
        let package = NativePackage::new(checked, PackageLimits::default()).unwrap();
        assert_eq!(package.bytes(), vector.artifact, "{}", vector.name);
        assert_eq!(
            package.canonical_identity().to_string(),
            vector.digest.trim_end()
        );
        assert_eq!(
            package.usage().canonical.unwrap().output_bytes,
            vector.canonical.len()
        );
    }
}

#[test]
#[trace("TC-082", "TC-087", "FR-020-AC-8", "FR-020-AC-10")]
fn independently_frozen_packages_reconstruct_after_original_checker_is_dropped() {
    let models = [model()];
    for vector in package_vector_setup::cases() {
        let original = package_vector_setup::checked(&vector, &models);
        let mut bindings = original.bindings().clone();
        bindings.clauses.reverse();
        drop(original);
        let package = NativePackage::read_verified(
            vector.artifact,
            NativePackageRef::new(ByteDigest::of(vector.artifact)),
            bindings,
            &models,
            &PackageSupport::default(),
            PackageReadLimits::default(),
        )
        .unwrap();
        assert_eq!(package.bytes(), vector.artifact);
        assert_eq!(package.checked().clauses().len(), vector.clauses.len());
        assert_eq!(
            package.canonical_identity().to_string(),
            vector.digest.trim_end()
        );
        assert_eq!(package.usage().admitted_input_bytes, vector.artifact.len());
        assert!(package.usage().recognition.is_some());
        assert!(package.usage().decode.is_some());
        assert!(package.usage().compare.is_some());
    }
}

#[test]
#[trace("TC-083", "TC-084", "TC-088")]
#[trace("FR-020-AC-2", "FR-020-AC-3", "FR-020-AC-9")]
fn selected_raw_requests_preserve_admission_and_recognition_precedence() {
    let models = [model()];
    let [vector, _, _] = package_vector_setup::cases();
    let bindings = package_vector_setup::checked(&vector, &models)
        .bindings()
        .clone();
    let requests: &[(&[u8], Code)] = &[
        (
            b"{\"format\":\"future\",\"unknown\":[1,2.5]}",
            Code::UnknownWire,
        ),
        (
            b"{\"format\":\"future\",\"x\":{\"a\":0,\"\\u0061\":1}}",
            Code::InvalidPackage,
        ),
        (
            b"{\"format\":\"future\",\"x\":1e9999}",
            Code::InvalidPackage,
        ),
        (b"{\"format\":7}", Code::InvalidPackage),
        (br#"{"format":"future","kind":0}"#, Code::UnknownWire),
        (br#"{"format":"future","v":"\uD800"}"#, Code::InvalidPackage),
        (br#"{"format":"future","v":"\x"}"#, Code::InvalidPackage),
        (br#"{"format":"future","v":01}"#, Code::InvalidPackage),
        (br#"{"format":"future","v":1.}"#, Code::InvalidPackage),
        (br#"{"format":"future","v":1e}"#, Code::InvalidPackage),
        (b"{\"format\":\"future\"} {}", Code::InvalidPackage),
        (b"\xef\xbb\xbf{}", Code::InvalidPackage),
        (b"\xff", Code::InvalidUtf8),
    ];
    for (bytes, code) in requests {
        let expected = NativePackageRef::new(ByteDigest::of(bytes));
        let failure = NativePackage::read_verified(
            bytes,
            expected,
            bindings.clone(),
            &models,
            &PackageSupport::default(),
            PackageReadLimits::default(),
        )
        .unwrap_err();
        assert_eq!(failure.code, *code, "{:?}", bytes);
        assert_eq!(failure.stage, PackageStage::Decode);
        assert_eq!(failure.usage.admitted_input_bytes, bytes.len());
        assert!(failure.usage.decode.is_none());
        let mut limits = PackageReadLimits::default();
        limits.package.artifact_bytes = bytes.len() - 1;
        let limited = NativePackage::read_verified(
            bytes,
            expected,
            bindings.clone(),
            &models,
            &PackageSupport::default(),
            limits,
        )
        .unwrap_err();
        assert_eq!(limited.code, Code::ResourceExhausted);
        assert_eq!(limited.usage.admitted_input_bytes, 0);
        assert!(limited.usage.recognition.is_none());
        let stale = NativePackage::read_verified(
            bytes,
            NativePackageRef::new(ByteDigest::of(b"wrong")),
            bindings.clone(),
            &models,
            &PackageSupport::default(),
            PackageReadLimits::default(),
        )
        .unwrap_err();
        assert_eq!(stale.code, Code::StaleDependency);
        assert!(stale.usage.recognition.is_none());
    }
}
