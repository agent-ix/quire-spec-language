// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §2.1 I3 / FR-030-AC-2 to AC-5: pure extraction against the original document —
//! preflight, fence location and exact byte-boundary verification. This adapter never
//! reaches the native compiler. FR-030-AC-1 ("reaches mapped compilation") and every
//! native-compile-failure case are tested at the SEAM-1 caller instead: see the
//! `#[cfg(test)]` module in `src/command/extraction.rs`.
#![cfg(feature = "quire-extraction")]

// The shared runtime setup also serves graph/operation targets.
use crate::support::runtime_setup as setup;

use ix_trace_rs::trace;
use qsl_foundation::{ByteDigest, Code, Source, SourceIdentity, Span};
use quire_rs::semantic::{extract_clauses, read_semantic_block, BundleIndex, SemanticContext};
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::quire_source::{extract, Cause, Limits, PreflightFailure, Selection};
use serde_json::json;

fn identity() -> SourceIdentity {
    SourceIdentity {
        identity: "ix://example/runtime-rules/spec".into(),
        revision: "draft:7".into(),
    }
}

fn source(text: &str) -> Source {
    Source::read(identity(), "rules.md", text.as_bytes(), 1_048_576).unwrap()
}

fn context() -> SemanticContext {
    // Use Quire's real module validator, not an invented semantic context decoder.
    let module = read_semantic_block(
        &json!({"contract_version":"1.0.0","semantic_core":"0.1.0","package":"example/runtime-rules","exports":["entity"],"targets":["markdown"]}),
        &["entity".to_owned()],
        &|name| name == "entity",
    ).unwrap();
    SemanticContext::new(module, "rules.md", BundleIndex::default())
        .with_source_identity(identity().identity)
}

fn selection() -> Selection {
    Selection {
        clause_id: "population_rule".into(),
        package: setup::authored_owner().package().as_str().into(),
        body: SourceIdentity {
            identity: "test:quire-body".into(),
            revision: "body:7".into(),
        },
    }
}

fn document(model: &NativeModel, expression: &str, crlf: bool) -> Source {
    // Authored input fixture. Only the production Quire extractor selects its body.
    let text = format!(
        "# Ω authored document\n\n## Invariants\n\n### population_rule\n  ```ix:native\n  language \"ix:native\" edition \"0-draft\";\n  profile \"state-finite/0-draft\";\n  model M = \"example/rule-tests\" version \"1\" digest \"{}\";\n  invariant Rule on M::Node at current {{ {expression} }}\n  ```\n\n### unselected\n```ix:native\nopaque body remains unparsed by Quire\n```\n\n## Notes\nλ unselected prose.\n",
        model.digest(),
    );
    source(&if crlf {
        text.replace('\n', "\r\n")
    } else {
        text
    })
}

#[test]
#[trace("TC-108", "FR-030-AC-3", "FR-011-AC-3")]
fn stale_foreign_unavailable_and_inconsistent_source_selections_refuse() {
    let models = [setup::native_rule_model::parts().model()];
    let original = document(&models[0], "true", false);
    let ctx = context();
    let changed = format!("{}Changed unselected prose.\n", original.text());
    let error = Source::read_verified(
        identity(),
        "rules.md",
        changed.as_bytes(),
        original.digest(),
        1_048_576,
    )
    .unwrap_err();
    assert_eq!(error.code, Code::SourceDigestMismatch);
    assert_ne!(ByteDigest::of(changed.as_bytes()), original.digest());

    for text in [
        original
            .text()
            .replace("### population_rule", "### missing_rule"),
        format!("{}\n## Invariants\n", original.text()),
        original
            .text()
            .replace("### unselected", "### population_rule"),
        "# No invariants\n".to_owned(),
    ] {
        let input = source(&text);
        let expected = extract_clauses(input.text(), &ctx);
        let error = extract(input, &ctx, selection(), Limits::default()).unwrap_err();
        assert_eq!(error.code(), Code::InvalidModelBinding);
        assert_eq!(error.extraction(), Some(&expected));
    }
    let mut reused = selection();
    reused.body = original.identity().clone();
    let error = extract(original.clone(), &ctx, reused, Limits::default()).unwrap_err();
    assert_eq!(error.code(), Code::InvalidSourceMap);
    assert!(error.extraction().is_some());
}

#[test]
#[trace("TC-108", "FR-030-AC-4", "FR-030-AC-5")]
fn source_coordinates_and_limits_keep_exact_boundaries_and_fresh_retries() {
    let models = [setup::native_rule_model::parts().model()];
    let ctx = context();
    for crlf in [false, true] {
        let original = document(&models[0], "true", crlf);
        // Quire's byte column and native scalar columns differ for this valid
        // closing fence. Its trailing Unicode whitespace must remain accepted.
        let (closing, spaced) = if crlf {
            ("  ```\r\n", "  ```\u{a0}\r\n")
        } else {
            ("  ```\n", "  ```\u{a0}\n")
        };
        let original = source(&original.text().replacen(closing, spaced, 1));
        let upstream = extract_clauses(original.text(), &ctx);
        let exact = Limits {
            source_bytes: original.text().len(),
            lines: original.position(original.text().len()).unwrap().line,
        };
        let extracted = extract(original.clone(), &ctx, selection(), exact).unwrap();
        assert_eq!(extracted.extraction, upstream);
        let body_end = extracted.body.text().len();
        let eof = extracted
            .map
            .map_span(
                &extracted.body,
                Span {
                    start: body_end,
                    end: body_end,
                },
            )
            .unwrap();
        assert_eq!(eof.len(), 1);
        assert_eq!(eof[0].start, eof[0].end);
        assert_eq!(
            original
                .slice(Span {
                    start: eof[0].end.byte,
                    end: extracted.map.region().end
                })
                .unwrap(),
            if crlf { "\r\n" } else { "\n" }
        );
        assert!(extracted
            .map
            .map_span(
                &extracted.body,
                Span {
                    start: 0,
                    end: usize::MAX
                }
            )
            .is_err());
        let mut byte_stop = exact;
        byte_stop.source_bytes -= 1;
        let mut line_stop = exact;
        line_stop.lines -= 1;
        for (limits, expected) in [
            (
                byte_stop,
                PreflightFailure::SourceBytes {
                    actual: original.text().len(),
                    maximum: byte_stop.source_bytes,
                },
            ),
            (
                line_stop,
                PreflightFailure::SourceLines {
                    actual: Some(exact.lines),
                    maximum: line_stop.lines,
                },
            ),
        ] {
            let error = extract(original.clone(), &ctx, selection(), limits).unwrap_err();
            assert_eq!(error.code(), Code::ResourceExhausted);
            assert!(error.extraction().is_none());
            let Cause::Preflight(actual) = &error.cause else {
                panic!("expected typed input ceiling");
            };
            assert_eq!(**actual, expected);
        }
        assert!(extract(original, &ctx, selection(), exact).is_ok());
    }
    let many_lines = source(&format!(
        "{}{}",
        "\n".repeat(4096),
        document(&models[0], "true", false).text()
    ));
    let error = extract(
        many_lines,
        &ctx,
        selection(),
        Limits {
            lines: usize::MAX,
            source_bytes: usize::MAX,
        },
    )
    .unwrap_err();
    assert_eq!(error.code(), Code::ResourceExhausted);
    assert!(error.extraction().is_none());
    let Cause::Preflight(actual) = &error.cause else {
        panic!("expected hard line ceiling");
    };
    assert!(matches!(
        **actual,
        PreflightFailure::SourceLines { maximum: 4096, .. }
    ));
}

#[test]
#[trace("TC-108", "FR-030-AC-3", "FR-011-AC-3")]
fn each_foreign_context_refuses_with_its_own_typed_preflight_cause() {
    use quire_spec_language::quire_source::{CONTRACT_VERSION, SEMANTIC_CORE_VERSION};
    #[derive(Debug)]
    enum Foreign {
        Identity,
        Path,
        Package,
        Contract,
        Semantic,
    }
    let models = [setup::native_rule_model::parts().model()];
    let original = document(&models[0], "true", false);
    for change in [
        Foreign::Identity,
        Foreign::Path,
        Foreign::Package,
        Foreign::Contract,
        Foreign::Semantic,
    ] {
        let mut foreign = context();
        let expected = match change {
            Foreign::Identity => {
                foreign.source_identity = None;
                PreflightFailure::SourceIdentity {
                    actual: None,
                    expected: identity().identity,
                }
            }
            Foreign::Path => {
                foreign.path = "foreign.md".into();
                PreflightFailure::SourcePath {
                    actual: "foreign.md".into(),
                    expected: "rules.md".into(),
                }
            }
            Foreign::Package => {
                foreign.bundle.package = "example/foreign".into();
                PreflightFailure::Package {
                    actual: "example/foreign".into(),
                    expected: "example/runtime-rules".into(),
                }
            }
            Foreign::Contract => {
                foreign.module.contract_version = "future".into();
                PreflightFailure::ContractVersion {
                    actual: "future".into(),
                    expected: CONTRACT_VERSION,
                }
            }
            Foreign::Semantic => {
                foreign.module.semantic_core = "future".into();
                PreflightFailure::SemanticCore {
                    actual: "future".into(),
                    expected: SEMANTIC_CORE_VERSION,
                }
            }
        };
        let error =
            extract(original.clone(), &foreign, selection(), Limits::default()).unwrap_err();
        let Cause::Preflight(actual) = &error.cause else {
            panic!("{change:?}: expected preflight refusal, got {error:?}");
        };
        assert_eq!(**actual, expected, "{change:?}");
        assert_eq!(error.code(), expected.code());
        assert!(error.extraction().is_none());
        assert_eq!(error.original().digest(), original.digest());
    }
}
