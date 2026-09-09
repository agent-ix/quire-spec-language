// SPDX-License-Identifier: AGPL-3.0-only
//! NFR-007: independent charged-content counts at the public reader boundary.
use super::*;
use quire_spec_language::package::PackagePassUsage;

#[test]
#[trace("TC-088", "FR-020-AC-9")]
fn elevated_input_limit_cannot_admit_one_byte_over_the_hard_ceiling() {
    let models = [model()];
    let [vector, _, _] = package_vector_setup::cases();
    let bindings = package_vector_setup::checked(&vector, &models)
        .bindings()
        .clone();
    let bytes = vec![0xff; PackageLimits::default().artifact_bytes + 1];
    let mut limits = PackageReadLimits::default();
    limits.package.artifact_bytes = usize::MAX;
    let error = NativePackage::read_verified(
        &bytes,
        NativePackageRef::new(ByteDigest::of(b"different")),
        bindings,
        &models,
        &PackageSupport::default(),
        limits,
    )
    .unwrap_err();
    assert_eq!(error.code, Code::ResourceExhausted);
    assert_eq!(error.usage.admitted_input_bytes, 0);
    assert!(error.usage.recognition.is_none());
}

fn count(value: &Value, depth: usize, usage: &mut PackagePassUsage) {
    match value {
        Value::String(text) => usage.string_bytes += text.len(),
        Value::Array(array) => {
            usage.max_depth = usage.max_depth.max(depth + 1);
            usage.entries += array.len();
            for item in array {
                count(item, depth + 1, usage);
            }
        }
        Value::Object(object) => {
            usage.max_depth = usage.max_depth.max(depth + 1);
            usage.entries += object.len();
            for (key, item) in object {
                usage.string_bytes += key.len();
                count(item, depth + 1, usage);
            }
        }
        _ => {}
    }
}

#[test]
#[trace("TC-088", "FR-020-AC-9", "FR-021-AC-6")]
fn exact_public_pass_counts_and_lowered_limits_preserve_inputs_and_retries() {
    let models = [model()];
    for vector in package_vector_setup::cases() {
        let checked = package_vector_setup::checked(&vector, &models);
        let bindings = checked.bindings().clone();
        let original_model = models[0].artifact_bytes().to_vec();
        let original_source = bindings.source.source().text().to_string();
        let parsed: Value = serde_json::from_slice(vector.artifact).unwrap();
        let mut expected = PackagePassUsage::default();
        count(&parsed, 0, &mut expected);
        let exact = PackageLimits {
            artifact_bytes: vector.artifact.len(),
            string_bytes: expected.string_bytes,
            entries: expected.entries,
            depth: expected.max_depth,
        };
        let request = |package| {
            NativePackage::read_verified(
                vector.artifact,
                NativePackageRef::new(ByteDigest::of(vector.artifact)),
                bindings.clone(),
                &models,
                &PackageSupport::default(),
                PackageReadLimits {
                    package,
                    ..PackageReadLimits::default()
                },
            )
        };
        let success = request(exact).unwrap();
        assert_eq!(success.usage().recognition, Some(expected));
        assert_eq!(success.usage().decode, Some(expected));
        assert_eq!(success.usage().compare, Some(expected));
        assert_eq!(success.usage().checking, Some(*checked.usage()));
        for limited in [
            PackageLimits {
                artifact_bytes: exact.artifact_bytes - 1,
                ..exact
            },
            PackageLimits {
                string_bytes: exact.string_bytes - 1,
                ..exact
            },
            PackageLimits {
                entries: exact.entries - 1,
                ..exact
            },
            PackageLimits {
                depth: exact.depth - 1,
                ..exact
            },
            PackageLimits {
                string_bytes: 0,
                ..exact
            },
            PackageLimits {
                entries: 0,
                ..exact
            },
            PackageLimits { depth: 0, ..exact },
        ] {
            let error = request(limited).unwrap_err();
            assert_eq!(
                (error.code, error.stage),
                (Code::ResourceExhausted, PackageStage::Decode)
            );
            if let Some(usage) = error.usage.recognition {
                assert!(usage.entries <= limited.entries);
                assert!(usage.string_bytes <= limited.string_bytes);
                assert!(usage.max_depth <= limited.depth);
            }
            assert!(error.usage.decode.is_none());
            assert!(error.usage.compare.is_none());
            assert_eq!(request(exact).unwrap().usage(), success.usage());
        }
        let elevated = PackageLimits {
            artifact_bytes: usize::MAX,
            string_bytes: usize::MAX,
            entries: usize::MAX,
            depth: usize::MAX,
        };
        assert_eq!(request(elevated).unwrap().usage(), success.usage());
        assert_eq!(models[0].artifact_bytes(), original_model);
        assert_eq!(bindings.source.source().text(), original_source);
    }
}
