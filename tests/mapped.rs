// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-022: mapped source admission through the actual native compiler/runtime.

// The shared fixture also serves operation/graph tests in other binaries.
#[allow(dead_code)]
#[path = "support/runtime_setup.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::checking::ClauseBinding;
use quire_spec_language::mapped::{compile, CompileCause, CompileLimits};
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::runtime::{
    evaluate, validate, EvaluationLimits, EvaluationOutcome, ValidationLimits, ValueId, ValueNode,
};
use quire_spec_language::source_map::{Layout, Segment, SourceMap};
use quire_spec_language::{Code, Phase, Source, SourceIdentity, Span};

fn binding() -> ClauseBinding {
    ClauseBinding {
        name: "Rule".into(),
        requirement: setup::authored_owner(),
        clause: ir::ClauseId::new("population_rule").unwrap(),
        execution_point: ir::ExecutionPoint::Handler {
            name: ir::AnchorName::new("validate").unwrap(),
        },
    }
}

fn formal() -> ir::SourceIdentity {
    ir::SourceIdentity::new(
        ir::SourceDocumentId::new("MappedNativeBody").unwrap(),
        ir::SourceRevision::new(7).unwrap(),
    )
}

fn text(model: &NativeModel, expression: &str) -> String {
    format!(
        "language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"example/rule-tests\" version \"1\" digest \"{}\";\n// Ω preserves Unicode coordinates\ninvariant Rule on M::Node at current {{ {expression} }}\n",
        model.digest()
    )
}

fn source(identity: &str, path: &str, text: &str) -> Source {
    Source::read(
        SourceIdentity {
            identity: identity.into(),
            revision: "7".into(),
        },
        path,
        text.as_bytes(),
        1_048_576,
    )
    .unwrap()
}

fn mapping(body: &str) -> SourceMap {
    // Programmatic producer-boundary fixture, not a substitute for Quire extraction.
    let mut original = "# Ω authored document\r\n  ```ix:native\r\n".to_owned();
    let region_start = original.len();
    let mut segments = Vec::new();
    let mut body_at = 0;
    for line in body.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        original.push_str("  ");
        let start = original.len();
        original.push_str(content);
        if !content.is_empty() {
            segments.push(Segment {
                body: Span {
                    start: body_at,
                    end: body_at + content.len(),
                },
                original: Span {
                    start,
                    end: original.len(),
                },
            });
        }
        body_at += content.len();
        if line.ends_with('\n') {
            original.push('\r');
            let start = original.len();
            original.push('\n');
            segments.push(Segment {
                body: Span {
                    start: body_at,
                    end: body_at + 1,
                },
                original: Span {
                    start,
                    end: start + 1,
                },
            });
            body_at += 1;
        }
    }
    let region = Span {
        start: region_start,
        end: original.len(),
    };
    original.push_str("  ```\r\nUnselected prose.\r\n");
    SourceMap::verify(
        source("test:authored-markdown", "rules.md", &original),
        source("test:extracted-body", "rules.md#population_rule", body),
        region,
        segments,
        Layout {
            strip_indentation: true,
            normalize_crlf: true,
            drop_final_newline: false,
        },
        50_000,
    )
    .unwrap()
}

#[test]
#[trace("TC-095", "FR-022-AC-1", "FR-022-AC-5")]
fn mapped_parent_workflow_preserves_authorship_and_runtime_results() {
    let models = [setup::native_rule_model::parts().model()];
    let model = &models[0];
    let expression = "present(self.parent) implies deref(value(self.parent)).n < self.n";
    let body = text(model, expression);
    let map = mapping(&body);
    let original_digest = map.original().digest();
    let body_digest = map.body().digest();
    let program = compile(
        map,
        "ix:native",
        binding(),
        formal(),
        &models,
        CompileLimits::default(),
    )
    .unwrap();
    assert_eq!(program.mapping().original().digest(), original_digest);
    assert_eq!(
        program.native().checked().linked().unit().source().digest(),
        body_digest
    );
    assert_eq!(
        program.native().checked().clauses()[0].binding(),
        &binding()
    );
    let all = program
        .original_spans(Span {
            start: 0,
            end: body.len(),
        })
        .unwrap();
    assert!(
        all.len() > 1,
        "deleted indentation/CRLF remains discontiguous"
    );
    let reconstructed: String = all
        .iter()
        .map(|span| {
            program
                .mapping()
                .original()
                .slice(Span {
                    start: span.start.byte,
                    end: span.end.byte,
                })
                .unwrap()
        })
        .collect();
    assert_eq!(reconstructed, body);
    assert!(program
        .original_spans(Span {
            start: 0,
            end: usize::MAX
        })
        .is_err());

    for (parent_number, dangling, expected) in
        [(1, false, true), (2, false, false), (1, true, false)]
    {
        let mut draft = setup::draft(model);
        if !dangling {
            let mut parent = draft.populations[0].objects[0].clone();
            parent.key = "parent".into();
            parent
                .fields
                .iter_mut()
                .find(|field| field.name.as_str() == "n")
                .unwrap()
                .value = ValueId::new(5);
            draft.populations[0].objects.push(parent);
        }
        draft.arena.push(ValueNode::Integer {
            value: parent_number,
        });
        draft.arena.push(ValueNode::Reference {
            identity: setup::object(model, "parent"),
        });
        setup::change_field(&mut draft, "n", ValueNode::Integer { value: 2 });
        setup::change_field(
            &mut draft,
            "parent",
            ValueNode::Present {
                value: ValueId::new(6),
            },
        );
        let snapshot = setup::snapshot(draft);
        let selected = setup::selection(model, snapshot.reference());
        let result = validate(
            program.native().checked(),
            setup::input(snapshot),
            selected,
            ValidationLimits::default(),
            || false,
        );
        if dangling {
            let error = result.unwrap_err();
            assert!(error
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == Code::DanglingReference));
        } else {
            let context = result.unwrap();
            assert_eq!(
                evaluate(&context, EvaluationLimits::default(), || false).outcome(),
                &EvaluationOutcome::Completed(expected)
            );
            assert_eq!(context.clause().binding(), &binding());
        }
    }
}

#[test]
#[trace("TC-096", "FR-022-AC-2", "FR-022-AC-3")]
fn mapped_refusals_keep_the_selected_clause_and_original_bytes() {
    let models = [setup::native_rule_model::parts().model()];
    for (language, expression, code) in [
        ("ocl", "true", Code::UnknownLanguage),
        ("ix:native", "@", Code::InvalidSyntax),
        ("ix:native", "self.missing", Code::MissingDeclaration),
        ("ix:native", "self.n", Code::IllTyped),
    ] {
        let map = mapping(&text(&models[0], expression));
        let body_digest = map.body().digest();
        let error = compile(
            map,
            language,
            binding(),
            formal(),
            &models,
            CompileLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code(), code);
        assert_eq!(error.binding(), &binding());
        assert_eq!(error.mapping().body().digest(), body_digest);
        let diagnostic = error.native_diagnostic().unwrap();
        let spans = error.original_spans().unwrap().unwrap();
        let mapped: String = spans
            .iter()
            .map(|span| {
                error
                    .mapping()
                    .original()
                    .slice(Span {
                        start: span.start.byte,
                        end: span.end.byte,
                    })
                    .unwrap()
            })
            .collect();
        assert_eq!(
            mapped,
            error
                .mapping()
                .body()
                .slice(Span {
                    start: diagnostic.span.start.byte,
                    end: diagnostic.span.end.byte,
                })
                .unwrap()
        );
        assert!(spans[0].start.line > diagnostic.span.start.line);
    }
    for (body, code) in [
        (
            text(&models[0], "true").replace("invariant Rule", "invariant Foreign"),
            Code::InvalidModelBinding,
        ),
        (
            format!(
                "{}invariant Other on M::Node at current {{ true }}\n",
                text(&models[0], "true")
            ),
            Code::InvalidModelBinding,
        ),
        (
            "language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\n"
                .into(),
            Code::InvalidSyntax,
        ),
    ] {
        let error = compile(
            mapping(&body),
            "ix:native",
            binding(),
            formal(),
            &models,
            CompileLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code(), code);
        assert_eq!(error.binding(), &binding());
    }
}

#[test]
#[trace("TC-096", "FR-022-AC-4")]
fn mapped_stage_limits_preserve_causes_and_allow_fresh_retries() {
    let models = [setup::native_rule_model::parts().model()];
    let map = mapping(&text(&models[0], "true"));
    let mut syntax = CompileLimits::default();
    syntax.syntax.source_bytes = 0;
    let mut linking = CompileLimits::default();
    linking.linking.nodes = 0;
    let mut checking = CompileLimits::default();
    checking.checking.nodes = 0;
    let mut package = CompileLimits::default();
    package.package.artifact_bytes = 0;
    for (limits, phase) in [
        (syntax, Some(Phase::Source)),
        (linking, Some(Phase::Link)),
        (checking, Some(Phase::Check)),
        (package, None),
    ] {
        let error = compile(
            map.clone(),
            "ix:native",
            binding(),
            formal(),
            &models,
            limits,
        )
        .unwrap_err();
        assert_eq!(error.code(), Code::ResourceExhausted);
        assert_eq!(
            error.native_diagnostic().map(|diagnostic| diagnostic.phase),
            phase
        );
        if phase.is_none() {
            assert!(matches!(error.cause, CompileCause::Package(_)));
            assert_eq!(error.original_spans().unwrap(), None);
        }
        let retry = compile(
            map.clone(),
            "ix:native",
            binding(),
            formal(),
            &models,
            CompileLimits::default(),
        )
        .unwrap();
        assert_eq!(retry.native().checked().clauses()[0].binding(), &binding());
    }
}
