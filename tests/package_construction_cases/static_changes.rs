// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-082/091: independent canonical mutations and admissible changed inputs.

use super::*;

fn change(expected: &mut vectors::Expected, before: &str, after: &str) {
    assert_ne!(before, after);
    assert!(
        expected.canonical.contains(before),
        "independent precondition: {before}"
    );
    expected.canonical = expected.canonical.replace(before, after);
    expected.artifact = expected.artifact.replace(before, after);
}

#[test]
#[trace("TC-082", "TC-091", "FR-019-AC-3", "FR-021-AC-3")]
fn used_context_unused_declarations_and_selected_roles_change_the_expected_content() {
    let baseline_models = [vectors::model()];
    let baseline = NativePackage::new(
        minimal(
            &baseline_models,
            "test:package",
            "draft:1",
            1,
            "original.native",
        ),
        PackageLimits::default(),
    )
    .unwrap();
    let source = include_str!("../fixtures/native-package/model-source.json");
    let original_node = r#"{"name": "Node", "fields": []}"#;
    let changed_node =
        r#"{"name": "Node", "fields": [{"name": "flag", "type": {"kind": "boolean"}}]}"#;
    for (name, before, after) in [
        ("context declaration", original_node, changed_node),
        (
            "unused value declaration",
            r#""name": "self""#,
            r#""name": "this""#,
        ),
        (
            "selected identity scalar",
            r#""max_scalars": 8"#,
            r#""max_scalars": 7"#,
        ),
        (
            "object universe role",
            r#""universe": "nodes""#,
            r#""universe": "other_nodes""#,
        ),
        (
            "unused enum declaration",
            r#""operations": []"#,
            r#""operations": [], "enums": [{"name":"Unused","variants":["Only"]}]"#,
        ),
        (
            "unused operation role",
            r#""operations": []"#,
            r#""operations": [{"name":"inspect","context":"Node","anchor":"inspect","parameters":[],"result":null,"frame":{"fields":[],"created":[],"deleted":[]}}]"#,
        ),
    ] {
        assert_eq!(source.matches(before).count(), 1, "{name}");
        let changed_source = source.replace(before, after);
        let models = [
            native_rule_model::from_text(&changed_source, "changed-model.json", "1")
                .unwrap()
                .model(),
        ];
        let mut expected = vectors::expected(&models[0], r#""test:package""#, r#""draft:1""#, 1);
        match name {
            "context declaration" => {
                let delta = changed_node.len() - original_node.len();
                // Frozen model input places Node's end at line 10, column 35,
                // byte 242. The inserted field stays on that physical line.
                change(
                    &mut expected,
                    r#""line":10,"column":35,"byte_offset":242"#,
                    &format!(
                        r#""line":10,"column":{},"byte_offset":{}"#,
                        35 + delta,
                        242 + delta
                    ),
                );
            }
            "object universe role" => change(
                &mut expected,
                r#""universe":"nodes""#,
                r#""universe":"other_nodes""#,
            ),
            "unused enum declaration" => change(
                &mut expected,
                r#""required_features":["boolean","object""#,
                r#""required_features":["boolean","enumeration","object""#,
            ),
            "unused value declaration" | "selected identity scalar" | "unused operation role" => {}
            _ => unreachable!(),
        }
        let package = NativePackage::new(
            minimal(&models, "test:package", "draft:1", 1, "changed.native"),
            PackageLimits::default(),
        )
        .unwrap();
        assert_eq!(package.bytes(), expected.artifact.as_bytes(), "{name}");
        assert_eq!(
            package.usage().canonical.unwrap().output_bytes,
            expected.canonical.len(),
            "{name}"
        );
        assert_ne!(package.bytes(), baseline.bytes(), "{name}");
        assert_ne!(
            models[0].artifact_bytes(),
            baseline_models[0].artifact_bytes()
        );
    }
}

#[test]
#[trace("TC-082", "TC-091", "FR-019-AC-2", "FR-021-AC-3")]
fn formal_document_identity_matches_independent_change() {
    let models = [vectors::model()];
    let baseline = NativePackage::new(
        minimal(&models, "test:package", "draft:1", 1, "original.native"),
        PackageLimits::default(),
    )
    .unwrap();
    let mut expected = vectors::expected(&models[0], r#""test:package""#, r#""draft:1""#, 1);
    change(
        &mut expected,
        r#""document":"PackageSource""#,
        r#""document":"OtherPackageSource""#,
    );
    let mut bindings = baseline.checked().bindings().clone();
    bindings.source = FormalSource::new(
        bindings.source.source().clone(),
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new("OtherPackageSource").unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        ),
    );
    let linked = link_native(
        baseline.checked().linked().unit().clone(),
        &models,
        LinkLimits::default(),
    )
    .unwrap();
    let rebound = check(linked, bindings, CheckLimits::default()).unwrap();
    let changed = NativePackage::new(rebound, PackageLimits::default()).unwrap();
    assert_eq!(changed.bytes(), expected.artifact.as_bytes());
    assert_ne!(changed.bytes(), baseline.bytes());
}

#[test]
#[trace("TC-091", "FR-021-AC-3", "FR-021-AC-4")]
fn the_canonical_content_excludes_projections_and_carries_no_identity_member() {
    let models = [vectors::model()];
    let expected = vectors::expected(&models[0], r#""test:package""#, r#""draft:1""#, 1);
    let package = NativePackage::new(
        minimal(&models, "test:package", "draft:1", 1, "claims.native"),
        PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(
        package.usage().canonical.unwrap().output_bytes,
        expected.canonical.len()
    );
    assert!(!expected.canonical.contains("projections"));
    assert!(expected.artifact.contains("projections"));
    assert!(!expected.artifact.contains("canonical_identity"));
}
