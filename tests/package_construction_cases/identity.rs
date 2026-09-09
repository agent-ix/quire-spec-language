// SPDX-License-Identifier: AGPL-3.0-only
//! TC-090/091: independent preimage controls and admissible static changes.

use sha2::{Digest, Sha256};

use super::*;

const DOMAIN: &[u8] = b"quire-spec-language\0quire.native.bound-package/v1\0linked-package\0";

fn hash(prefix: &[u8], content: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(prefix);
    hash.update(content);
    format!("{:x}", hash.finalize())
}

#[test]
#[trace("TC-090", "FR-021-AC-1", "FR-021-AC-6")]
fn nonconforming_domain_and_encoding_vectors_cannot_match_the_producer() {
    let models = [vectors::model()];
    let expected = vectors::expected(
        &models[0],
        r#""test:\"\\/\u0000\b\f\n\r\té🦀""#,
        r#""draft:é\u0001""#,
        9_007_199_254_740_993,
    );
    let package = NativePackage::new(
        minimal(
            &models,
            "test:\"\\/\0\u{8}\u{c}\n\r\té🦀",
            "draft:é\u{1}",
            9_007_199_254_740_993,
            "preimage.native",
        ),
        PackageLimits::default(),
    )
    .unwrap();
    let actual = package.canonical_identity().to_string();
    assert_eq!(actual, hash(DOMAIN, expected.canonical.as_bytes()));
    assert_eq!(package.bytes(), expected.artifact.as_bytes());

    let segments: [&[u8]; 3] = [
        b"quire-spec-language",
        b"quire.native.bound-package/v1",
        b"linked-package",
    ];
    for omitted in 0..3 {
        let mut missing_segment = Vec::new();
        let mut missing_nul = Vec::new();
        for (index, segment) in segments.iter().enumerate() {
            if index != omitted {
                missing_segment.extend_from_slice(segment);
            }
            missing_segment.push(0);
            missing_nul.extend_from_slice(segment);
            if index != omitted {
                missing_nul.push(0);
            }
        }
        assert_ne!(
            hash(&missing_segment, expected.canonical.as_bytes()),
            actual,
            "omitted segment {omitted}"
        );
        assert_ne!(
            hash(&missing_nul, expected.canonical.as_bytes()),
            actual,
            "omitted separator {omitted}"
        );
    }
    let sorted: serde_json::Value = serde_json::from_str(&expected.canonical).unwrap();
    for (name, content) in [
        ("final newline", format!("{}\n", expected.canonical)),
        (
            "global member sorting",
            serde_json::to_string(&sorted).unwrap(),
        ),
        ("escaped slash", expected.canonical.replace('/', "\\/")),
        (
            "Unicode normalization",
            expected.canonical.replace('é', "e\u{301}"),
        ),
        (
            "binary64 rounding",
            expected
                .canonical
                .replace("9007199254740993", "9007199254740992"),
        ),
        (
            "long newline escape",
            expected.canonical.replace("\\n", "\\u000a"),
        ),
    ] {
        assert_ne!(content, expected.canonical, "effective control: {name}");
        assert_ne!(hash(DOMAIN, content.as_bytes()), actual, "{name}");
    }
    assert_ne!(hash(b"", expected.canonical.as_bytes()), actual);
    assert_ne!(format!("{:x}", Sha256::digest(package.bytes())), actual);
}

#[test]
#[trace("TC-091", "FR-019-AC-2", "FR-021-AC-3")]
fn real_native_and_formal_binding_changes_match_independent_static_expectations() {
    let models = [vectors::model()];
    let baseline = NativePackage::new(
        minimal(&models, "test:package", "draft:1", 1, "source.native"),
        PackageLimits::default(),
    )
    .unwrap();
    for (identity, identity_json, revision, revision_json, formal_revision) in [
        (
            "test:other",
            r#""test:other""#,
            "draft:1",
            r#""draft:1""#,
            1,
        ),
        (
            "test:package",
            r#""test:package""#,
            "draft:2",
            r#""draft:2""#,
            1,
        ),
        (
            "test:package",
            r#""test:package""#,
            "draft:1",
            r#""draft:1""#,
            2,
        ),
        (
            "test:package",
            r#""test:package""#,
            "draft:1",
            r#""draft:1""#,
            u64::MAX,
        ),
    ] {
        let expected = vectors::expected(&models[0], identity_json, revision_json, formal_revision);
        let changed = NativePackage::new(
            minimal(
                &models,
                identity,
                revision,
                formal_revision,
                "source.native",
            ),
            PackageLimits::default(),
        )
        .unwrap();
        assert_eq!(changed.bytes(), expected.artifact.as_bytes());
        assert_eq!(changed.canonical_identity().to_string(), expected.digest);
        assert_ne!(changed.canonical_identity(), baseline.canonical_identity());
        assert_eq!(
            changed.checked().linked().unit().source().text(),
            baseline.checked().linked().unit().source().text()
        );
    }
}

#[test]
#[trace("TC-091", "FR-019-AC-1", "FR-021-AC-3")]
fn real_authored_identity_and_anchor_changes_match_independent_static_expectations() {
    let models = [vectors::model()];
    let expected = vectors::expected(&models[0], r#""test:package""#, r#""draft:1""#, 1);
    let original = binding();
    let mut foreign_owner = original.clone();
    foreign_owner.requirement =
        ir::RequirementRef::parse("example/other-rules", "Rule", 2).unwrap();
    let mut revision = original.clone();
    revision.requirement = ir::RequirementRef::parse("example/package-rules", "Rule", 3).unwrap();
    let mut clause = original.clone();
    clause.clause = ir::ClauseId::new("another_clause").unwrap();
    let mut anchor = original;
    anchor.execution_point = ir::ExecutionPoint::Initialization {
        name: ir::AnchorName::new("initialize").unwrap(),
    };
    for (binding, before, after) in [
        (
            foreign_owner,
            r#""owner":{"package":"example/package-rules","requirement":"Rule","revision":2}"#,
            r#""owner":{"package":"example/other-rules","requirement":"Rule","revision":2}"#,
        ),
        (
            revision,
            r#""owner":{"package":"example/package-rules","requirement":"Rule","revision":2}"#,
            r#""owner":{"package":"example/package-rules","requirement":"Rule","revision":3}"#,
        ),
        (clause, r#""clause":"rule""#, r#""clause":"another_clause""#),
        (
            anchor,
            r#""execution_point":{"kind":"handler","name":"validate"}"#,
            r#""execution_point":{"kind":"initialization","name":"initialize"}"#,
        ),
    ] {
        assert_eq!(expected.canonical.matches(before).count(), 1);
        let canonical = expected.canonical.replace(before, after);
        let digest = hash(DOMAIN, canonical.as_bytes());
        assert_ne!(digest, expected.digest);
        let artifact = expected
            .artifact
            .replace(before, after)
            .replace(&expected.digest, &digest);
        let package = NativePackage::new(
            checked(
                &vectors::source(&models[0]),
                "test:package",
                "draft:1",
                1,
                "authored.native",
                &models,
                vec![binding.clone()],
            ),
            PackageLimits::default(),
        )
        .unwrap();
        assert_eq!(package.canonical_identity().to_string(), digest);
        assert_eq!(package.bytes(), artifact.as_bytes());
        assert_eq!(package.checked().clauses()[0].binding(), &binding);
    }
}
