// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-091: admissible static changes against independent expectations.

use super::*;

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
            9_007_199_254_740_992,
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
        assert_ne!(changed.bytes(), baseline.bytes());
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
        name: ir::AnchorName::new("initialize").unwrap().into(),
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
        assert_ne!(canonical, expected.canonical);
        let artifact = expected.artifact.replace(before, after);
        let package = NativePackage::new(
            checked(
                &vectors::source(),
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
        assert_eq!(package.bytes(), artifact.as_bytes());
        assert_eq!(package.checked().clauses()[0].binding(), &binding);
    }
}
