// SPDX-License-Identifier: AGPL-3.0-only
//! FR-030 / IT-003: real Quire extraction into the native compiler/runtime.
#![cfg(feature = "quire-extraction")]

// The shared runtime setup also serves graph/operation targets.
#[allow(dead_code)]
#[path = "support/runtime_setup.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_rs::semantic::{
    extract_clauses, read_semantic_block, AvailabilityState, BundleIndex, SemanticContext,
};
use quire_spec_language::checking::ClauseBinding;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::quire_source::{compile, Cause, Limits, PreflightFailure, Selection};
use quire_spec_language::runtime::{
    execute, ExecutionLimits, ExecutionOutcome, ValueId, ValueNode,
};
use quire_spec_language::{ByteDigest, Code, Source, SourceIdentity, Span};
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
        binding: ClauseBinding {
            name: "Rule".into(),
            requirement: setup::authored_owner(),
            clause: ir::ClauseId::new("population_rule").unwrap(),
            execution_point: ir::ExecutionPoint::Handler {
                name: ir::AnchorName::new("validate").unwrap(),
            },
        },
        body: SourceIdentity {
            identity: "test:quire-body".into(),
            revision: "body:7".into(),
        },
        formal: ir::SourceIdentity::new(
            ir::SourceDocumentId::new("QuireNativeBody").unwrap(),
            ir::SourceRevision::new(7).unwrap(),
        ),
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
#[trace("TC-108", "FR-030-AC-1", "FR-030-AC-2")]
#[trace("FR-011-AC-1")]
fn actual_quire_body_reaches_native_truth_and_refusal_with_unchanged_extraction() {
    let models = [setup::native_rule_model::parts().model()];
    let model = &models[0];
    let original = document(
        model,
        "true implies forall(item in self.items: item < self.n)",
        true,
    );
    let ctx = context();
    let expected = extract_clauses(original.text(), &ctx);
    let program = compile(
        original.clone(),
        &ctx,
        selection(),
        &models,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(program.extraction(), &expected);
    assert_eq!(
        program.extraction().availability.state,
        AvailabilityState::Available
    );
    assert!(program.extraction().availability.lossy);
    assert_eq!(program.extraction().clauses.as_ref().unwrap().len(), 2);
    assert!(program
        .extraction()
        .diagnostics
        .iter()
        .any(|value| value.code == "semantic.clause-language-unchecked"));
    assert_eq!(
        program.mapped().mapping().original().digest(),
        original.digest()
    );
    assert_eq!(
        program.mapped().native().checked().clauses()[0].binding(),
        &selection().binding
    );
    let body = program.mapped().mapping().body();
    assert_eq!(
        body.text(),
        expected.clause_text["population_rule"]
            .strip_suffix('\r')
            .unwrap()
    );
    assert!(body.text().contains("\r\n  "));
    let spans = program
        .mapped()
        .original_spans(Span {
            start: 0,
            end: body.text().len(),
        })
        .unwrap();
    assert_eq!(spans.len(), 1);
    assert_eq!(
        original
            .slice(Span {
                start: spans[0].start.byte,
                end: spans[0].end.byte
            })
            .unwrap(),
        body.text()
    );

    for (number, dangling, truth) in [
        (2, false, Some(true)),
        (1, false, Some(false)),
        (2, true, None),
    ] {
        let mut draft = setup::draft(model);
        setup::change_field(&mut draft, "n", ValueNode::Integer { value: number });
        setup::change_field(
            &mut draft,
            "items",
            ValueNode::Sequence {
                values: vec![ValueId::new(0); 3],
            },
        );
        if dangling {
            setup::change_field(
                &mut draft,
                "peer",
                ValueNode::Reference {
                    identity: setup::object(model, "missing"),
                },
            );
        }
        let snapshot = setup::snapshot(draft);
        let selected = setup::selection(model, snapshot.reference());
        let report = execute(
            program.mapped().native(),
            setup::input(snapshot),
            selected.clone(),
            ExecutionLimits::default(),
            || false,
        );
        assert_eq!(report.truth(), truth);
        assert_eq!(report.selection(), &selected);
        assert_eq!(
            report.package().checked().clauses()[0].binding(),
            &selection().binding
        );
        if dangling {
            let ExecutionOutcome::ValidationFailed(failure) = report.outcome() else {
                panic!("invalid population must refuse before evaluation");
            };
            assert!(failure
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == Code::DanglingReference));
        }
    }
}

#[test]
#[trace("TC-108", "FR-030-AC-2", "FR-011-AC-2", "FR-011-AC-4")]
fn unchecked_tags_and_native_refusals_retain_the_actual_upstream_population() {
    let models = [setup::native_rule_model::parts().model()];
    let ctx = context();
    for (expression, code) in [
        ("@", Code::InvalidSyntax),
        ("collect(self.items)", Code::UnsupportedConstruct),
    ] {
        let original = document(&models[0], expression, true);
        let expected = extract_clauses(original.text(), &ctx);
        assert_eq!(expected.availability.state, AvailabilityState::Available);
        let error = compile(
            original.clone(),
            &ctx,
            selection(),
            &models,
            Limits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code(), code);
        assert_eq!(error.extraction(), Some(&expected));
        assert_eq!(error.original().digest(), original.digest());
        assert_eq!(error.selection().binding, selection().binding);
        let Cause::Compile(native) = &error.cause else {
            panic!("actual native parser refusal required");
        };
        let mapped = native.original_spans().unwrap().unwrap();
        assert!(!mapped.is_empty());
        assert_eq!(
            mapped[0].start.byte,
            original.text().find(expression).unwrap()
        );
        assert!(mapped.iter().all(|span| original
            .slice(Span {
                start: span.start.byte,
                end: span.end.byte
            })
            .is_some()));
        assert!(native.mapping().original().text().contains('Ω'));
    }
    let original = document(&models[0], "true", false);
    let wrong_language = source(&original.text().replacen("```ix:native", "```ocl", 1));
    let expected = extract_clauses(wrong_language.text(), &ctx);
    let error = compile(
        wrong_language,
        &ctx,
        selection(),
        &models,
        Limits::default(),
    )
    .unwrap_err();
    assert_eq!(error.code(), Code::UnknownLanguage);
    assert_eq!(error.extraction(), Some(&expected));
}

#[test]
#[trace("TC-108", "FR-030-AC-3", "FR-030-AC-4", "FR-011-AC-3")]
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
        let error = compile(input, &ctx, selection(), &models, Limits::default()).unwrap_err();
        assert_eq!(error.code(), Code::InvalidModelBinding);
        assert_eq!(error.extraction(), Some(&expected));
    }
    let mut reused = selection();
    reused.body = original.identity().clone();
    let error = compile(original.clone(), &ctx, reused, &models, Limits::default()).unwrap_err();
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
            ..Limits::default()
        };
        let program = compile(original.clone(), &ctx, selection(), &models, exact).unwrap();
        assert_eq!(program.extraction(), &upstream);
        let map = program.mapped().mapping();
        let body_end = map.body().text().len();
        let eof = program
            .mapped()
            .original_spans(Span {
                start: body_end,
                end: body_end,
            })
            .unwrap();
        assert_eq!(eof.len(), 1);
        assert_eq!(eof[0].start, eof[0].end);
        assert_eq!(
            original
                .slice(Span {
                    start: eof[0].end.byte,
                    end: map.region().end
                })
                .unwrap(),
            if crlf { "\r\n" } else { "\n" }
        );
        assert!(program
            .mapped()
            .original_spans(Span {
                start: 0,
                end: usize::MAX
            })
            .is_err());
        let mut byte_stop = exact;
        byte_stop.source_bytes -= 1;
        let mut line_stop = exact;
        line_stop.lines -= 1;
        let mut compiler_stop = exact;
        compiler_stop.compiler.package.artifact_bytes = 0;
        for (limits, preflight) in [
            (
                byte_stop,
                Some(PreflightFailure::SourceBytes {
                    actual: original.text().len(),
                    maximum: byte_stop.source_bytes,
                }),
            ),
            (
                line_stop,
                Some(PreflightFailure::SourceLines {
                    actual: Some(exact.lines),
                    maximum: line_stop.lines,
                }),
            ),
            (compiler_stop, None),
        ] {
            let error = compile(original.clone(), &ctx, selection(), &models, limits).unwrap_err();
            assert_eq!(error.code(), Code::ResourceExhausted);
            assert_eq!(error.extraction().is_none(), preflight.is_some());
            if let Some(expected) = preflight {
                let Cause::Preflight(actual) = &error.cause else {
                    panic!("expected typed input ceiling");
                };
                assert_eq!(**actual, expected);
            } else {
                assert!(matches!(error.cause, Cause::Compile(_)));
            }
        }
        assert!(compile(original, &ctx, selection(), &models, exact).is_ok());
    }
    let many_lines = source(&format!(
        "{}{}",
        "\n".repeat(4096),
        document(&models[0], "true", false).text()
    ));
    let error = compile(
        many_lines,
        &ctx,
        selection(),
        &models,
        Limits {
            lines: usize::MAX,
            source_bytes: usize::MAX,
            ..Limits::default()
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
        let error = compile(
            original.clone(),
            &foreign,
            selection(),
            &models,
            Limits::default(),
        )
        .unwrap_err();
        let Cause::Preflight(actual) = &error.cause else {
            panic!("{change:?}: expected preflight refusal, got {error:?}");
        };
        assert_eq!(**actual, expected, "{change:?}");
        assert_eq!(error.code(), expected.code());
        assert!(error.extraction().is_none());
        assert_eq!(error.original().digest(), original.digest());
    }
}
