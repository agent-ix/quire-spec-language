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
            let digest = if invalid_component == "digest" {
                "not-a-digest"
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
                    identity: format!("test:{declaration_kind}-{invalid_component}"),
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
            assert_eq!(diagnostic.span.start.byte, expected_start);
            assert_eq!(
                diagnostic.span.end.byte,
                expected_start + invalid_literal.len()
            );
            assert!(diagnostic.message.contains(invalid_component));
        }
    }
}
