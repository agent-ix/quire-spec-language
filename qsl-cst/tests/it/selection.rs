// SPDX-License-Identifier: AGPL-3.0-or-later
//! Definition and model selection validation in the parser: each invalid
//! identity of a `profile`, and each invalid identity of an `import`, and each
//! invalid identity, version or digest of a `model` declaration, is located at its own literal. Formerly in the root crate's
//! `complete::package_tests`; it parses and never resolves, so it is
//! layer 1's.
use ix_trace_rs::trace;
use qsl_cst::{parse, Limits};
use qsl_foundation::selection::{DefinitionRef, InvalidDefinitionComponent};
use qsl_foundation::{Code, SourceIdentity};

#[trace("QSpec-TC-180", "QSpec-FR-131-AC-2")]
#[test]
fn selection_validation_locates_each_invalid_component_for_every_declaration_kind() {
    let valid_digest = format!("sha256:{}", "a".repeat(64));
    assert_eq!(
        DefinitionRef::new("agent-ix", "").unwrap_err(),
        InvalidDefinitionComponent::Identity
    );
    assert_eq!(
        DefinitionRef::new("", "acme/definition").unwrap_err(),
        InvalidDefinitionComponent::Authority
    );

    for (declaration_kind, components) in [
        ("profile", &["identity"][..]),
        ("import", &["identity"][..]),
        ("model", &["identity", "version", "digest"][..]),
    ] {
        for &invalid_component in components {
            let identity = if invalid_component == "identity" {
                ""
            } else {
                "acme/definition"
            };
            let version = if invalid_component == "version" {
                ""
            } else {
                "1"
            };
            let digest = if invalid_component == "digest" {
                "not-a-digest"
            } else {
                &valid_digest
            };
            let declaration = match declaration_kind {
                "profile" => format!("profile Complete = \"{identity}\";"),
                "import" => format!("import \"{identity}\" as Base;"),
                "model" => {
                    format!("model M = \"{identity}\" version \"{version}\" digest \"{digest}\";")
                }
                _ => unreachable!("closed declaration-kind test table"),
            };
            let valid_profile = (declaration_kind != "profile")
                .then(|| "profile Complete = \"acme/profile\";\n".to_owned());
            let source = format!(
                "language \"ix:native\" edition \"1-draft\";\n{}{declaration}\nrecord R {{ datum: Integer; }}",
                valid_profile.as_deref().unwrap_or_default(),
            );
            let invalid_value = match invalid_component {
                "identity" => identity,
                "version" => version,
                "digest" => digest,
                _ => unreachable!("closed invalid-component test table"),
            };
            let invalid_literal = format!("\"{invalid_value}\"");
            let expected_start = source.find(&invalid_literal).unwrap();
            let parsed = parse(
                SourceIdentity {
                    authority: "test".into(),
                    identity: format!("test:{declaration_kind}-{invalid_component}"),
                    revision_namespace: "test".into(),
                    revision: "r1".into(),
                },
                "invalid-selection.native",
                source.as_bytes(),
                Limits::default(),
            )
            .unwrap();

            assert!(!parsed.is_admissible());
            assert_eq!(parsed.diagnostics().len(), 1);
            let diagnostic = &parsed.diagnostics()[0];
            assert_eq!(
                diagnostic.code,
                if invalid_component == "digest" {
                    Code::InvalidDigest
                } else {
                    Code::InvalidIdentifier
                }
            );
            assert_eq!(diagnostic.byte_span().unwrap().start, expected_start);
            assert_eq!(
                diagnostic.byte_span().unwrap().end,
                expected_start + invalid_literal.len()
            );
            assert!(diagnostic.message.contains(invalid_component));
        }
    }
}

/// FR-056-AC-9 (TC-442): a `model` declaration's digest names its slot:
/// `sha256-jcs:` selects a domain package, `sha256:` a compiled-model
/// artifact, and the two never compare equal.
#[trace("TC-442", "FR-056-AC-9")]
#[test]
fn a_model_digest_keeps_the_slot_its_prefix_names() {
    use qsl_foundation::selection::ModelDigest;
    let hex = "c".repeat(64);
    let domain = ModelDigest::parse(&format!("sha256-jcs:{hex}")).unwrap();
    let artifact = ModelDigest::parse(&format!("sha256:{hex}")).unwrap();
    assert!(matches!(domain, ModelDigest::DomainPackage(_)));
    assert!(matches!(artifact, ModelDigest::Artifact(_)));
    assert_ne!(domain, artifact);
    assert_eq!(domain.digest(), artifact.digest());
    assert!(ModelDigest::parse(&format!("sha256-jcs:{}", "C".repeat(64))).is_err());

    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"acme/profile\";\nmodel M = \"acme/orders\" version \"1\" digest \"sha256-jcs:{hex}\";\nrecord R {{ datum: Integer; }}"
    );
    let parsed = parse(
        SourceIdentity::new("test", "test:model-jcs", "test", "r1"),
        "model-jcs.native",
        source.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(parsed.diagnostics(), []);
}

/// FR-099-AC-2 (TC-446 step 2): an `import` names its library by identity
/// alone, `import "L" [as a];`. A `version` or `digest` token after the
/// identity is a syntax error, and the selection is not recorded.
#[trace("FR-099-AC-2", "TC-446")]
#[test]
fn an_import_names_only_its_library_identity() {
    let parse_import = |import: &str| {
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\";\n\
             {import}\n\
             record R {{ datum: Integer; }}\n"
        );
        parse(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            source.as_bytes(),
            Limits::default(),
        )
        .unwrap()
    };
    let parsed = parse_import("import \"test/geometry\" as g;");
    assert!(
        parsed.diagnostics().is_empty(),
        "{:?}",
        parsed.diagnostics()
    );
    let import = &parsed.selections().imports[0];
    assert_eq!(import.identity, "test/geometry");
    assert_eq!(import.alias.as_deref(), Some("g"));
    let bare = parse_import("import \"test/geometry\";");
    assert!(bare.diagnostics().is_empty());
    assert_eq!(bare.selections().imports[0].alias, None);

    let hex = "d".repeat(64);
    for old in [
        "import \"test/geometry\" version \"1\" as g;".to_owned(),
        format!("import \"test/geometry\" digest \"{hex}\" as g;"),
        format!("import \"test/geometry\" version \"1\" digest \"{hex}\" as g;"),
    ] {
        let parsed = parse_import(&old);
        assert!(!parsed.is_admissible(), "{old}");
        assert!(parsed.selections().imports.is_empty(), "{old}");
        assert_eq!(parsed.diagnostics()[0].code, Code::InvalidSyntax, "{old}");
    }
}
