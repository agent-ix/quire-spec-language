// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-082/091: independent canonical mutations and admissible changed inputs.

use sha2::{Digest, Sha256};

use super::*;

fn hash(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"quire-spec-language\0quire.native.bound-package/v1\0linked-package\0");
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

fn change(expected: &mut vectors::Expected, before: &str, after: &str) {
    assert_ne!(before, after);
    assert!(
        expected.canonical.contains(before),
        "independent precondition: {before}"
    );
    expected.canonical = expected.canonical.replace(before, after);
    let digest = hash(expected.canonical.as_bytes());
    expected.artifact = expected
        .artifact
        .replace(before, after)
        .replace(&expected.digest, &digest);
    expected.digest = digest;
}

#[test]
#[trace("TC-082", "TC-091", "FR-019-AC-3", "FR-021-AC-3")]
fn used_context_unused_declarations_and_selected_roles_change_the_expected_identity() {
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
            package.canonical_identity().to_string(),
            expected.digest,
            "{name}"
        );
        assert_eq!(
            package.digest(),
            ByteDigest::of(expected.artifact.as_bytes())
        );
        assert_ne!(
            package.canonical_identity(),
            baseline.canonical_identity(),
            "{name}"
        );
        assert_ne!(models[0].digest(), baseline_models[0].digest());
    }
}

#[test]
#[trace("TC-082", "TC-091", "FR-019-AC-2", "FR-021-AC-3")]
fn native_source_bytes_and_formal_document_identity_match_independent_changes() {
    let models = [vectors::model()];
    let original = vectors::source(&models[0]);
    let baseline = NativePackage::new(
        minimal(&models, "test:package", "draft:1", 1, "original.native"),
        PackageLimits::default(),
    )
    .unwrap();
    let changed_source = format!("{original}// different exact source bytes\n");
    let mut expected = vectors::expected(&models[0], r#""test:package""#, r#""draft:1""#, 1);
    change(
        &mut expected,
        &ByteDigest::of(original.as_bytes()).to_string(),
        &ByteDigest::of(changed_source.as_bytes()).to_string(),
    );
    let changed = NativePackage::new(
        checked(
            &changed_source,
            "test:package",
            "draft:1",
            1,
            "bytes.native",
            &models,
            vec![binding()],
        ),
        PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(changed.bytes(), expected.artifact.as_bytes());
    assert_eq!(changed.canonical_identity().to_string(), expected.digest);
    assert_ne!(changed.canonical_identity(), baseline.canonical_identity());

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
    assert_eq!(changed.canonical_identity().to_string(), expected.digest);
    assert_ne!(changed.canonical_identity(), baseline.canonical_identity());
}

#[test]
#[trace("TC-091", "FR-021-AC-3", "FR-021-AC-4")]
fn independently_changed_static_claims_have_distinct_preimages() {
    let models = [vectors::model()];
    let expected = vectors::expected(&models[0], r#""test:package""#, r#""draft:1""#, 1);
    let package = NativePackage::new(
        minimal(&models, "test:package", "draft:1", 1, "claims.native"),
        PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(
        package.canonical_identity().to_string(),
        hash(expected.canonical.as_bytes())
    );
    for (before, after) in [
        (r#""language":"ix:native""#, r#""language":"ix:foreign""#),
        (r#""edition":"0-draft""#, r#""edition":"1-draft""#),
        (
            r#""syntax_profile":"state-finite/0-draft""#,
            r#""syntax_profile":"state-finite/1-draft""#,
        ),
        (
            r#""model_profile":"native-state-model/1""#,
            r#""model_profile":"native-state-model/2""#,
        ),
        (
            r#""checking_contract":"native-checked-clauses/1""#,
            r#""checking_contract":"native-checked-clauses/2""#,
        ),
        (
            r#""ir_revision":"690bde7f2dc58662cf9ff0595c2c0e3b17107c6f""#,
            r#""ir_revision":"790bde7f2dc58662cf9ff0595c2c0e3b17107c6f""#,
        ),
        (
            "8bc68a3c7e46d26c7191dfbb662d4d63070af9fedc9ce985fb1885f7efe29429",
            "9bc68a3c7e46d26c7191dfbb662d4d63070af9fedc9ce985fb1885f7efe29429",
        ),
        (
            "d9eb316752ac45d7984b355a054cd279ef9749164a92c7f61fbf621fe280588b",
            "e9eb316752ac45d7984b355a054cd279ef9749164a92c7f61fbf621fe280588b",
        ),
        (
            r#""context_observations":["current"]"#,
            r#""context_observations":["pre"]"#,
        ),
        (r#""validate_frame":false"#, r#""validate_frame":true"#),
        (
            r#""expression":null,"span":{"start":221,"end":225}"#,
            r#""expression":null,"span":{"start":220,"end":225}"#,
        ),
        (
            r#""base_definition":{"revision":"e897f810a7356d4ce8fd19026221ebda7b65596f""#,
            r#""base_definition":{"revision":"f897f810a7356d4ce8fd19026221ebda7b65596f""#,
        ),
        (
            r#""rules_definition":{"revision":"e897f810a7356d4ce8fd19026221ebda7b65596f""#,
            r#""rules_definition":{"revision":"f897f810a7356d4ce8fd19026221ebda7b65596f""#,
        ),
        (
            r#""universe":"nodes","observations":["current"]"#,
            r#""universe":"nodes","observations":["pre"]"#,
        ),
        (
            r#""line":10,"column":5,"byte_offset":212"#,
            r#""line":10,"column":6,"byte_offset":213"#,
        ),
        (
            r#""key":{"kind":"type","name":"Node"}"#,
            r#""key":{"kind":"type","name":"Foreign"}"#,
        ),
    ] {
        assert!(
            expected.canonical.contains(before),
            "independently located claim {before}"
        );
        let changed = expected.canonical.replace(before, after);
        assert_ne!(hash(changed.as_bytes()), expected.digest, "{before}");
    }
    // Unsupported selectors and forged derivations stay raw reader controls.
    // No constructor is used to fabricate a successfully checked package.
    assert!(!expected.canonical.contains("projections"));
    assert!(!expected.canonical.contains("canonical_identity"));
}
