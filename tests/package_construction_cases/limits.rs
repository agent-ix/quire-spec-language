// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-088: independent JSON workloads, pass boundaries and atomic retries.

use quire_spec_language::package::{PackagePassUsage, PackageStage};
use serde_json::Value;

use super::*;

// Count the independent expected data tree, not production formatter callbacks
// or reported usage. Strings embedded as model artifacts remain one string.
fn workload(value: &Value, parents: usize) -> PackagePassUsage {
    let mut own = PackagePassUsage::default();
    match value {
        Value::Object(fields) => {
            own.entries = fields.len();
            own.string_bytes = fields.keys().map(String::len).sum();
            own.max_depth = parents + 1;
            for value in fields.values() {
                add(&mut own, workload(value, parents + 1));
            }
        }
        Value::Array(values) => {
            own.entries = values.len();
            own.max_depth = parents + 1;
            for value in values {
                add(&mut own, workload(value, parents + 1));
            }
        }
        Value::String(text) => own.string_bytes = text.len(),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    own
}

fn add(total: &mut PackagePassUsage, part: PackagePassUsage) {
    total.entries += part.entries;
    total.string_bytes += part.string_bytes;
    total.max_depth = total.max_depth.max(part.max_depth);
}

fn expected_usage(expected: &vectors::Expected) -> [PackagePassUsage; 3] {
    let mut full: Value = serde_json::from_str(&expected.artifact).unwrap();
    let mut encode = workload(&full, 0);
    encode.output_bytes = expected.artifact.len();
    full.as_object_mut().unwrap().remove("canonical_identity");
    let derive = workload(&full, 0);
    let mut canonical = workload(&serde_json::from_str(&expected.canonical).unwrap(), 0);
    canonical.output_bytes = expected.canonical.len();
    [derive, canonical, encode]
}

#[test]
#[trace("TC-088", "FR-019-AC-10", "FR-021-AC-6")]
#[trace("NFR-007-M-2", "NFR-007-M-3", "NFR-007-M-4", "NFR-007-M-5")]
fn all_producer_passes_match_independently_counted_workloads() {
    let models = [vectors::model()];
    let expected = vectors::expected(&models[0], r#""test:package""#, r#""draft:1""#, 1);
    let counts = expected_usage(&expected);
    let package = NativePackage::new(
        minimal(&models, "test:package", "draft:1", 1, "counts.native"),
        PackageLimits::default(),
    )
    .unwrap();
    let usage = package.usage();
    assert_eq!(usage.derive, Some(counts[0]));
    assert_eq!(usage.canonical, Some(counts[1]));
    assert_eq!(usage.encode, Some(counts[2]));
    assert!(usage.recognition.is_none());
    assert!(usage.decode.is_none());
    assert!(usage.compare.is_none());
    assert_eq!(usage.admitted_input_bytes, 0);
    assert_eq!(package.bytes(), expected.artifact.as_bytes());
}

#[test]
#[trace("TC-088", "FR-019-AC-10", "FR-021-AC-6")]
#[trace("NFR-007-M-2", "NFR-007-M-3", "NFR-007-M-4", "NFR-007-M-5")]
fn inclusive_limits_refuse_before_excess_and_retry_without_shared_usage() {
    let models = [vectors::model()];
    let source_before = vectors::source(&models[0]);
    let model_before = models[0].artifact_bytes().to_vec();
    let expected = vectors::expected(&models[0], r#""test:package""#, r#""draft:1""#, 1);
    let [derive, canonical, encode] = expected_usage(&expected);
    let make = |limits| {
        NativePackage::new(
            minimal(&models, "test:package", "draft:1", 1, "limits.native"),
            limits,
        )
    };
    let exact_limits = PackageLimits {
        artifact_bytes: encode.output_bytes,
        string_bytes: encode.string_bytes,
        entries: encode.entries,
        depth: encode.max_depth,
    };
    let exact = make(exact_limits).unwrap();
    assert_eq!(exact.bytes(), expected.artifact.as_bytes());
    assert_eq!(exact.usage().derive, Some(derive));
    assert_eq!(exact.usage().canonical, Some(canonical));
    assert_eq!(exact.usage().encode, Some(encode));
    for limits in [
        PackageLimits {
            artifact_bytes: encode.output_bytes - 1,
            ..exact_limits
        },
        PackageLimits {
            string_bytes: encode.string_bytes - 1,
            ..exact_limits
        },
        PackageLimits {
            entries: encode.entries - 1,
            ..exact_limits
        },
        PackageLimits {
            depth: encode.max_depth - 1,
            ..exact_limits
        },
    ] {
        let error = make(limits).unwrap_err();
        assert_eq!(
            (error.code, error.stage),
            (Code::ResourceExhausted, PackageStage::Encode)
        );
        assert!(error.cause.is_some());
        assert!(error.usage.recognition.is_none());
        assert!(error.usage.decode.is_none());
        assert!(error.usage.compare.is_none());
        for measured in [
            error.usage.derive,
            error.usage.canonical,
            error.usage.encode,
        ]
        .into_iter()
        .flatten()
        {
            assert!(measured.output_bytes <= limits.artifact_bytes);
            assert!(measured.string_bytes <= limits.string_bytes);
            assert!(measured.entries <= limits.entries);
            assert!(measured.max_depth <= limits.depth);
        }
        assert_eq!(models[0].artifact_bytes(), model_before);
        assert_eq!(vectors::source(&models[0]), source_before);
        let retry = make(exact_limits).unwrap();
        assert_eq!(retry.bytes(), exact.bytes());
        assert_eq!(retry.usage(), exact.usage());
    }
    // A caller cannot change the hard defaults by supplying elevated options.
    let elevated = make(PackageLimits {
        artifact_bytes: usize::MAX,
        string_bytes: usize::MAX,
        entries: usize::MAX,
        depth: usize::MAX,
    })
    .unwrap();
    assert_eq!(elevated.bytes(), exact.bytes());
    assert_eq!(elevated.usage(), exact.usage());
}

#[test]
#[trace("TC-088", "FR-019-AC-10", "FR-021-AC-6")]
#[trace("NFR-007-M-2", "NFR-007-M-3", "NFR-007-M-4", "NFR-007-M-5")]
fn zero_limits_and_coupled_passes_report_only_entered_work() {
    let models = [vectors::model()];
    let expected = vectors::expected(&models[0], r#""test:package""#, r#""draft:1""#, 1);
    let [derive, canonical, _] = expected_usage(&expected);
    let make = |limits| {
        NativePackage::new(
            minimal(&models, "test:package", "draft:1", 1, "zero.native"),
            limits,
        )
    };
    let byte_zero = make(PackageLimits {
        artifact_bytes: 0,
        ..PackageLimits::default()
    })
    .unwrap_err();
    assert_eq!(byte_zero.code, Code::ResourceExhausted);
    assert_eq!(byte_zero.usage.derive, Some(derive));
    assert_eq!(byte_zero.usage.canonical.unwrap().output_bytes, 0);
    assert!(byte_zero.usage.encode.is_none());
    assert!(byte_zero.path.is_empty());
    for limits in [
        PackageLimits {
            string_bytes: 0,
            ..PackageLimits::default()
        },
        PackageLimits {
            entries: 0,
            ..PackageLimits::default()
        },
        PackageLimits {
            depth: 0,
            ..PackageLimits::default()
        },
        PackageLimits {
            string_bytes: canonical.string_bytes - 1,
            ..PackageLimits::default()
        },
        PackageLimits {
            entries: canonical.entries - 1,
            ..PackageLimits::default()
        },
    ] {
        let error = make(limits).unwrap_err();
        assert_eq!(
            (error.code, error.stage),
            (Code::ResourceExhausted, PackageStage::Encode)
        );
        assert!(error.usage.derive.is_some());
        assert!(error.usage.canonical.is_none());
        assert!(error.usage.encode.is_none());
    }
    let canonical_byte = make(PackageLimits {
        artifact_bytes: canonical.output_bytes - 1,
        ..PackageLimits::default()
    })
    .unwrap_err();
    assert_eq!(canonical_byte.code, Code::ResourceExhausted);
    assert_eq!(canonical_byte.usage.derive, Some(derive));
    assert!(canonical_byte.usage.canonical.unwrap().output_bytes < canonical.output_bytes);
    assert!(canonical_byte.usage.encode.is_none());
}
