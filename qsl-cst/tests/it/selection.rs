// SPDX-License-Identifier: AGPL-3.0-or-later
//! Definition and model selection validation in the parser: each invalid
//! identity, version or digest of a `profile`, `import` or `model`
//! declaration is located at its own literal. Formerly in the root crate's
//! `complete::package_tests`; it parses and never resolves, so it is
//! layer 1's (QSL-181).
use ix_trace_rs::trace;
use qsl_cst::{parse, Limits};
use qsl_foundation::selection::{DefinitionDigest, DefinitionRef, InvalidDefinitionComponent};
use qsl_foundation::{Code, SourceIdentity};

#[trace("TC-180", "FR-131-AC-2")]
#[test]
fn selection_validation_locates_each_invalid_component_for_every_declaration_kind() {
    let valid_digest = format!("sha256:{}", "a".repeat(64));
    let parsed_digest = DefinitionDigest::parse(&valid_digest).unwrap();
    assert_eq!(
        DefinitionRef::new("", "1", parsed_digest).unwrap_err(),
        InvalidDefinitionComponent::Identity
    );
    assert_eq!(
        DefinitionRef::new("acme/definition", "", parsed_digest).unwrap_err(),
        InvalidDefinitionComponent::Version
    );

    for declaration_kind in ["profile", "import", "model"] {
        for invalid_component in ["identity", "version", "digest"] {
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
            // An import records a bare 64-hex `package_id` (ADR-015 D-2).
            let bare_digest = "a".repeat(64);
            let digest = if invalid_component == "digest" {
                "not-a-digest"
            } else if declaration_kind == "import" {
                &bare_digest
            } else {
                &valid_digest
            };
            let declaration = match declaration_kind {
                "profile" => format!(
                    "profile Complete = \"{identity}\" version \"{version}\" digest \"{digest}\";"
                ),
                "import" => format!(
                    "import \"{identity}\" version \"{version}\" digest \"{digest}\" as Base;"
                ),
                "model" => {
                    format!("model M = \"{identity}\" version \"{version}\" digest \"{digest}\";")
                }
                _ => unreachable!("closed declaration-kind test table"),
            };
            let valid_profile = (declaration_kind != "profile").then(|| {
                format!(
                    "profile Complete = \"acme/profile\" version \"1\" digest \"{valid_digest}\";\n"
                )
            });
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
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"acme/profile\" version \"1\" digest \"sha256:{hex}\";\nmodel M = \"acme/orders\" version \"1\" digest \"sha256-jcs:{hex}\";\nrecord R {{ datum: Integer; }}"
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

/// FR-099-AC-2 (TC-446 step 2): an import's digest is exactly 64 lowercase
/// hexadecimal characters, read as a `quire.package.semantic/v2` digest
/// record. A `sha256:` prefix, uppercase hex, and 63 or 65 characters each
/// refuse with `invalid-digest` at the digest string.
#[trace("FR-099-AC-2", "TC-446")]
#[test]
fn an_import_digest_is_bare_lowercase_hex() {
    use qsl_foundation::digest::{DigestDomain, DigestRecord};
    let hex = "d".repeat(64);
    let parse_import = |digest: &str| {
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             import \"test/geometry\" version \"1\" digest \"{digest}\" as g;\n"
        );
        let parsed = parse(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            source.as_bytes(),
            Limits::default(),
        )
        .unwrap();
        (source, parsed)
    };
    let (_, parsed) = parse_import(&hex);
    assert!(parsed.diagnostics().is_empty(), "{:?}", parsed.diagnostics());
    let import = &parsed.selections().imports[0];
    assert_eq!(import.identity, "test/geometry");
    assert_eq!(import.version, "1");
    assert_eq!(
        import.digest,
        DigestRecord::from_domain_and_hex(DigestDomain::PackageSemanticV2, &hex).unwrap()
    );
    for invalid in [
        format!("sha256:{hex}"),
        "D".repeat(64),
        "d".repeat(63),
        "d".repeat(65),
    ] {
        let (source, parsed) = parse_import(&invalid);
        assert!(parsed.selections().imports.is_empty(), "{invalid}");
        assert_eq!(parsed.diagnostics().len(), 1, "{invalid}");
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(diagnostic.code, Code::InvalidDigest, "{invalid}");
        assert_eq!(diagnostic.cause.as_str(), "invalid-digest", "{invalid}");
        let literal = format!("\"{invalid}\"");
        let start = source.find(&literal).unwrap();
        assert_eq!(
            diagnostic.byte_span().unwrap(),
            qsl_foundation::Span {
                start,
                end: start + literal.len()
            },
            "{invalid}"
        );
    }
}
