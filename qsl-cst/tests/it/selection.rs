// SPDX-License-Identifier: AGPL-3.0-or-later
//! Definition and model selection validation in the parser: each invalid
//! identity of a `profile`, and each invalid identity of an `import`, and each
//! invalid identity of a `model` declaration, is located at its own literal. Formerly in the root crate's
//! `complete::package_tests`; it parses and never resolves, so it is
//! layer 1's.
use ix_trace_rs::trace;
use qsl_cst::{parse, Limits};
use qsl_foundation::selection::{DefinitionRef, InvalidDefinitionComponent};
use qsl_foundation::{Code, SourceIdentity};

#[trace("QSpec-TC-180", "QSpec-FR-131-AC-2")]
#[test]
fn selection_validation_locates_each_invalid_component_for_every_declaration_kind() {
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
        ("model", &["identity"][..]),
    ] {
        for &invalid_component in components {
            let identity = if invalid_component == "identity" {
                ""
            } else {
                "acme/definition"
            };
            let declaration = match declaration_kind {
                "profile" => format!("profile Complete = \"{identity}\";"),
                "import" => format!("import \"{identity}\" as Base;"),
                "model" => {
                    format!("model M = \"{identity}\";")
                }
                _ => unreachable!("closed declaration-kind test table"),
            };
            let valid_profile = (declaration_kind != "profile")
                .then(|| "profile Complete = \"acme/profile\";\n".to_owned());
            let source = format!(
                "language \"ix:native\" edition \"1-draft\";\n{}{declaration}\nrecord R {{ datum: Integer; }}",
                valid_profile.as_deref().unwrap_or_default(),
            );
            let invalid_literal = format!("\"{identity}\"");
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
            assert_eq!(diagnostic.code, Code::InvalidIdentifier);
            assert_eq!(diagnostic.byte_span().unwrap().start, expected_start);
            assert_eq!(
                diagnostic.byte_span().unwrap().end,
                expected_start + invalid_literal.len()
            );
            assert!(diagnostic.message.contains(invalid_component));
        }
    }
}

/// FR-056 (QSpec FR-154): a `model` names its domain package by identity
/// alone, `model M = "<identity>";`. A `version` or `digest` token after the
/// identity is a syntax error and the selection is not recorded; a `model`
/// without a string identity is a located diagnostic, never a silent drop.
#[trace("TC-442", "FR-056-AC-9")]
#[test]
fn a_model_names_only_its_domain_package_identity() {
    let parse_model = |model: &str| {
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"acme/profile\";\n{model}\nrecord R {{ datum: Integer; }}"
        );
        parse(
            SourceIdentity::new("test", "test:model", "test", "r1"),
            "model.native",
            source.as_bytes(),
            Limits::default(),
        )
        .unwrap()
    };
    let parsed = parse_model("model M = \"acme/orders\";");
    assert_eq!(parsed.diagnostics(), []);
    let selection = &parsed.selections().models[0];
    assert_eq!(selection.alias, "M");
    assert_eq!(selection.identity, "acme/orders");

    let hex = "c".repeat(64);
    for refused in [
        "model M = \"acme/orders\" version \"1\";".to_owned(),
        format!("model M = \"acme/orders\" digest \"sha256-jcs:{hex}\";"),
        format!("model M = \"acme/orders\" version \"1\" digest \"sha256-jcs:{hex}\";"),
        "model M = ;".to_owned(),
        "model M = acme;".to_owned(),
    ] {
        let parsed = parse_model(&refused);
        assert!(!parsed.is_admissible(), "{refused}");
        assert!(parsed.selections().models.is_empty(), "{refused}");
        assert!(!parsed.diagnostics().is_empty(), "{refused}");
    }
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
