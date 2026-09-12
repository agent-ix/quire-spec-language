// SPDX-License-Identifier: AGPL-3.0-only
//! FR-020/NFR-007: independent JSON grammar and charged-event controls.

use super::*;
use ix_trace_rs::trace;

#[test]
#[trace("TC-083", "FR-020-AC-2")]
fn tagged_record_reports_the_earlier_closed_field_defect() {
    let raw = br#"{"kind":"type","field":"unused","name":7}"#;
    let error = decode_wire::<super::super::wire::Key>(raw, PackageLimits::default()).unwrap_err();
    assert_eq!(error.code, Code::InvalidPackage);
    assert!(
        error.cause.to_string().contains("unknown field `field`"),
        "{error:?}"
    );
}

#[test]
#[trace("TC-083", "FR-020-AC-2", "FR-020-AC-3")]
fn buffered_variants_keep_string_tags_and_object_records() {
    for raw in [
        br#"{"kind":1,"binding":{"start":0,"end":0}}"#.as_slice(),
        br#"{"kind":"local","binding":[0,0]}"#,
    ] {
        let error =
            decode_wire::<super::super::wire::Target>(raw, PackageLimits::default()).unwrap_err();
        assert_eq!(error.code, Code::InvalidPackage);
    }
    let (header, _) = decode::<Recognized>(
        br#"{"format":"future","payload":{"kind":0}}"#,
        PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(header.0.as_deref(), Some("future"));
}

#[test]
#[trace("TC-083", "FR-020-AC-2")]
fn additive_dependency_features_cannot_expand_the_package_number_domain() {
    let error = decode::<Recognized>(
        br#"{"format":"future","value":1e9999}"#,
        PackageLimits::default(),
    )
    .err()
    .unwrap();
    assert_eq!(error.code, Code::InvalidPackage);

    let (recognized, _) = decode::<Recognized>(
        br#"{"format":"future","value":1e-9999}"#,
        PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(recognized.0.as_deref(), Some("future"));
}

#[test]
#[trace("TC-088", "FR-020-AC-9")]
fn hard_recognition_entry_and_string_limits_cannot_be_elevated() {
    let hard = PackageLimits::default();
    let elevated = PackageLimits {
        entries: usize::MAX,
        string_bytes: usize::MAX,
        ..hard
    };
    // Two root members plus the independently sized array elements.
    let mut values = vec![0; hard.entries - 2];
    let exact = serde_json::to_vec(&serde_json::json!({"format":"future", "v":values})).unwrap();
    let (_, usage) = decode::<Recognized>(&exact, elevated).unwrap();
    assert_eq!(usage.entries, hard.entries);
    values.push(0);
    let excess = serde_json::to_vec(&serde_json::json!({"format":"future", "v":values})).unwrap();
    let error = decode::<Recognized>(&excess, elevated).err().unwrap();
    assert_eq!(error.code, Code::ResourceExhausted);
    assert_eq!(error.usage.entries, hard.entries);

    // The public raw-byte ceiling makes this string maximum unreachable there;
    // isolate the real recognition pass without claiming public admission.
    for (extra, expected) in [(0, true), (1, false)] {
        let raw = format!(
            "{{\"format\":\"future\",\"v\":\"{}\"}}",
            "a".repeat(hard.string_bytes - 13 + extra)
        );
        let result = decode::<Recognized>(raw.as_bytes(), elevated);
        if expected {
            assert_eq!(result.unwrap().1.string_bytes, hard.string_bytes);
        } else {
            let error = result.err().unwrap();
            assert_eq!(error.code, Code::ResourceExhausted);
            assert_eq!(error.usage.string_bytes, 13);
        }
    }
}

#[test]
#[trace("TC-083", "TC-088", "FR-020-AC-2", "FR-020-AC-9")]
fn recognition_counts_decoded_content_and_rejects_nested_duplicates() {
    let bytes = br#"{"format":"future","a":["\u00e9[]",{"b":true}]}"#;
    let (header, usage) = decode::<Recognized>(bytes, PackageLimits::default()).unwrap();
    assert_eq!(header.0.as_deref(), Some("future"));
    assert_eq!(usage.string_bytes, 18); // format + future + a + é[] + b
    assert_eq!(usage.entries, 5); // two root keys, two array elements, one nested key
    assert_eq!(usage.max_depth, 3);
    assert_eq!(usage.output_bytes, 0);
    let duplicate = decode::<Recognized>(
        br#"{"format":"future","a":[{"b":0,"\u0062":1}]}"#,
        PackageLimits::default(),
    )
    .err()
    .unwrap();
    assert_eq!(duplicate.code, Code::InvalidPackage);
    assert_eq!(
        duplicate.path,
        vec![
            PackagePathSegment::Field("a".into()),
            PackagePathSegment::Index(0),
            PackagePathSegment::Field("b".into())
        ]
    );
}

#[test]
#[trace("TC-088", "FR-020-AC-9")]
fn exact_container_ceiling_is_owned_by_meter_and_retry_is_fresh() {
    let bytes = format!(
        "{{\"format\":\"future\",\"v\":{}0{}}}",
        "[".repeat(127),
        "]".repeat(127)
    );
    let (_, exact) = decode::<Recognized>(bytes.as_bytes(), PackageLimits::default()).unwrap();
    assert_eq!(exact.max_depth, 128);
    let lower = decode::<Recognized>(
        bytes.as_bytes(),
        PackageLimits {
            depth: 127,
            ..PackageLimits::default()
        },
    )
    .err()
    .unwrap();
    assert_eq!(lower.code, Code::ResourceExhausted);
    assert_eq!(lower.usage.max_depth, 127);
    assert_eq!(
        decode::<Recognized>(bytes.as_bytes(), PackageLimits::default())
            .unwrap()
            .1,
        exact
    );
    let overflow = format!("{{\"v\":{}0{}}}", "[".repeat(128), "]".repeat(128));
    assert_eq!(
        decode::<Recognized>(
            overflow.as_bytes(),
            PackageLimits {
                depth: usize::MAX,
                ..PackageLimits::default()
            }
        )
        .err()
        .unwrap()
        .code,
        Code::ResourceExhausted
    );
}

#[test]
#[trace("TC-088", "FR-020-AC-9")]
fn typed_pass_limits_are_isolated_without_claiming_public_reachability() {
    let raw = include_bytes!("../../../tests/fixtures/native-package/minimal.package.json");
    let (_, expected) =
        decode_wire::<super::super::wire::Manifest>(raw, PackageLimits::default()).unwrap();
    for limits in [
        PackageLimits {
            string_bytes: expected.string_bytes - 1,
            ..PackageLimits::default()
        },
        PackageLimits {
            entries: expected.entries - 1,
            ..PackageLimits::default()
        },
        PackageLimits {
            depth: expected.max_depth - 1,
            ..PackageLimits::default()
        },
    ] {
        let error = decode_wire::<super::super::wire::Manifest>(raw, limits).unwrap_err();
        assert_eq!(error.code, Code::ResourceExhausted);
        assert!(error.usage.entries <= limits.entries);
        assert!(error.usage.string_bytes <= limits.string_bytes);
        assert!(error.usage.max_depth <= limits.depth);
        assert_eq!(
            decode_wire::<super::super::wire::Manifest>(raw, PackageLimits::default())
                .unwrap()
                .1,
            expected
        );
    }
}
