// SPDX-License-Identifier: AGPL-3.0-only
//! TC-088: isolated formatter ceilings where earlier public passes are coupled.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use super::*;

#[test]
#[trace("TC-088", "NFR-007-M-2", "NFR-007-M-3", "NFR-007-M-4", "NFR-007-M-5")]
fn escaped_string_content_is_not_structure_and_failed_paths_are_real() {
    let data = json!({"q\"": ["[{}]\\\n\0é🦀", {"x": true}]});
    let limits = PackageLimits::default();
    let (bytes, usage) = run(&data, limits, true).unwrap();
    let expected = r#"{"q\"":["[{}]\\\n\u0000é🦀",{"x":true}]}"#;
    assert_eq!(bytes, expected.as_bytes());
    assert_eq!(
        usage,
        PackagePassUsage {
            output_bytes: expected.len(),
            string_bytes: 2 + "[{}]\\\n\0é🦀".len() + 1,
            entries: 4,
            max_depth: 3,
        }
    );
    let (_, sink) = run(&data, limits, false).unwrap();
    assert_eq!(
        sink,
        PackagePassUsage {
            output_bytes: 0,
            ..usage
        }
    );
    let failure = run(&data, PackageLimits { depth: 2, ..limits }, true).unwrap_err();
    assert_eq!(failure.code, Code::ResourceExhausted);
    assert_eq!(
        failure.path,
        vec![
            PackagePathSegment::Field("q\"".into()),
            PackagePathSegment::Index(1)
        ]
    );
    assert_eq!(failure.usage.max_depth, 2);
    assert_eq!(run(&data, limits, true).unwrap(), (bytes, usage));
}

#[test]
#[trace("TC-088", "NFR-007-M-4", "NFR-007-M-5")]
fn hard_container_and_entry_ceilings_cannot_be_raised() {
    let elevated = PackageLimits {
        depth: usize::MAX,
        entries: usize::MAX,
        ..PackageLimits::default()
    }
    .bounded();
    assert_eq!(elevated.depth, 128);
    assert_eq!(elevated.entries, 100_000);
    let mut nested = Value::Bool(true);
    for _ in 0..128 {
        nested = Value::Array(vec![nested]);
    }
    let (_, exact) = run(&nested, elevated, true).unwrap();
    assert_eq!(exact.max_depth, 128);
    assert_eq!(exact.entries, 128);
    assert_eq!(exact.output_bytes, 2 * 128 + 4);
    let below = run(
        &nested,
        PackageLimits {
            depth: 127,
            ..elevated
        },
        true,
    )
    .unwrap_err();
    assert_eq!(
        (below.code, below.usage.max_depth),
        (Code::ResourceExhausted, 127)
    );
    let too_deep = Value::Array(vec![nested]);
    let above = run(&too_deep, elevated, true).unwrap_err();
    assert_eq!(
        (above.code, above.usage.max_depth),
        (Code::ResourceExhausted, 128)
    );

    let mut entries = vec![Value::Bool(false); 100_000];
    let (_, exact) = run(&entries, elevated, true).unwrap();
    assert_eq!(exact.entries, 100_000);
    assert_eq!(exact.output_bytes, 600_001);
    let below = run(
        &entries,
        PackageLimits {
            entries: 99_999,
            ..elevated
        },
        true,
    )
    .unwrap_err();
    assert_eq!(
        (below.code, below.usage.entries),
        (Code::ResourceExhausted, 99_999)
    );
    entries.push(Value::Bool(false));
    let above = run(&entries, elevated, true).unwrap_err();
    assert_eq!(
        (above.code, above.usage.entries),
        (Code::ResourceExhausted, 100_000)
    );
    assert_eq!(above.path, vec![PackagePathSegment::Index(100_000)]);
}

#[test]
#[trace("TC-088", "NFR-007-M-2", "NFR-007-M-3")]
fn hard_byte_and_string_limits_are_independent_and_check_before_retention() {
    let elevated = PackageLimits {
        artifact_bytes: usize::MAX,
        string_bytes: usize::MAX,
        ..PackageLimits::default()
    }
    .bounded();
    assert_eq!(elevated.artifact_bytes, 16_777_216);
    assert_eq!(elevated.string_bytes, 16_777_216);
    let text = "x".repeat(16_777_216);
    // Sink isolates string accounting; no public package with this bare shape
    // is claimed, and output punctuation would exceed the same byte ceiling.
    let (empty, exact) = run(&text, elevated, false).unwrap();
    assert!(empty.is_empty());
    assert_eq!(exact.string_bytes, 16_777_216);
    let below = run(
        &text,
        PackageLimits {
            string_bytes: 16_777_215,
            ..elevated
        },
        false,
    )
    .unwrap_err();
    assert_eq!(below.code, Code::ResourceExhausted);
    assert_eq!(below.usage.string_bytes, 0);
    let above = run(&format!("{text}x"), elevated, false).unwrap_err();
    assert_eq!(
        (above.code, above.usage.string_bytes),
        (Code::ResourceExhausted, 0)
    );
    let (bytes, exact) = run(&&text[..16_777_214], elevated, true).unwrap();
    assert_eq!(bytes.len(), 16_777_216);
    assert_eq!(exact.output_bytes, 16_777_216);
    drop(bytes);
    let below = run(
        &&text[..16_777_214],
        PackageLimits {
            artifact_bytes: 16_777_215,
            ..elevated
        },
        true,
    )
    .unwrap_err();
    assert_eq!(
        (below.code, below.usage.output_bytes),
        (Code::ResourceExhausted, 16_777_215)
    );
    let above = run(&&text[..16_777_215], elevated, true).unwrap_err();
    assert_eq!(
        (above.code, above.usage.output_bytes),
        (Code::ResourceExhausted, 16_777_216)
    );
}
