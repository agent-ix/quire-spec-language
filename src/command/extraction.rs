// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-031: select one authored binding, extract through the I3 adapter and invoke the
//! actual native compiler. The native compile join lives here (the SEAM-1 native arm),
//! not in the `qsl-source` crate (ADR-011 §2.1 I3 row; §6.2 `qsl-source` row).

use super::{wire, Intake, Result, RunCause};
use crate::checking::ClauseBinding;
use crate::formal_source::{FormalSource, SourceIdentities};
use crate::mapped::{self, CompileError, CompileLimits, MappedPackage};
use crate::native_model::NativeModel;
use qsl_foundation::{Code, Diagnostic, Source};
use qsl_source::{ClausesOutcome, SemanticContext, SemanticFailure};

/// Unsupported combinations at the extracted-command boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ExtractionMode {
    /// Source-only export does not admit an extracted program.
    #[error("extraction is supported by run only")]
    CompileCommand,
    /// Selected package bytes and extracted compilation cannot both supply a program.
    #[error("extraction cannot select a package artifact")]
    PackageSelected,
    /// Extracted compilation requires one authored binding.
    #[error("extraction requires one authored clause binding; found {actual}")]
    WrongClauseCount {
        /// Number of selected authored bindings.
        actual: usize,
    },
}

impl ExtractionMode {
    /// Stable catalogued code for the particular unsupported combination.
    pub fn code(self) -> Code {
        match self {
            Self::CompileCommand => Code::ExtractionRequiresRun,
            Self::PackageSelected => Code::ExtractionPackageConflict,
            Self::WrongClauseCount { .. } => Code::ExtractionClauseCount,
        }
    }
}

/// Extraction-specific failures behind the optional command seam.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExtractionError {
    /// Explicitly unsupported request combination.
    #[error("{0}")]
    Mode(#[from] ExtractionMode),
    /// Pinned Quire rejected the constructed clause-only context.
    /// Its diagnostics remain Quire-owned; wire output includes the selected versions.
    #[error("Quire rejected the clause-only semantic context")]
    Context(Vec<SemanticFailure>),
    /// Actual Quire join or native compilation failure.
    #[error("{0}")]
    Compile(#[from] Box<JoinFailure>),
}

impl ExtractionError {
    /// Existing native or selected command code, independent of display text.
    pub fn code(&self) -> Code {
        match self {
            Self::Mode(mode) => mode.code(),
            Self::Context(_) => Code::InvalidQuireContext,
            Self::Compile(error) => error.code(),
        }
    }
}

impl From<ExtractionMode> for RunCause {
    fn from(mode: ExtractionMode) -> Self {
        Self::Extraction(Box::new(ExtractionError::Mode(mode)))
    }
}

/// Actual failed join or native stage, without parsing display messages.
///
/// This lives here, not in `qsl-source`, because only the SEAM-1 caller depends on
/// both the I3 adapter and the native compiler (ADR-011 §2.1 I3 row).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum JoinCause {
    /// Input limits, profile selection or original-context mismatch before extraction.
    #[error("{0}")]
    Preflight(#[from] Box<qsl_source::PreflightFailure>),
    /// Original selection/correspondence diagnostic.
    #[error("{0}")]
    Join(#[from] Box<Diagnostic>),
    /// Complete original mapped compiler failure.
    #[error("{0}")]
    Compile(#[from] Box<CompileError>),
}

/// A failed request preserves its source, selection and any completed extraction.
#[derive(Debug, thiserror::Error)]
#[error("{cause}")]
pub struct JoinFailure {
    original: Source,
    binding: ClauseBinding,
    extraction: Option<ClausesOutcome>,
    /// Actual failure with no partial native package.
    #[source]
    pub cause: JoinCause,
}

impl JoinFailure {
    /// Original immutable document, including exact byte digest.
    pub fn original(&self) -> &Source {
        &self.original
    }

    /// The caller's authored clause binding, including on preflight refusal.
    pub fn binding(&self) -> &ClauseBinding {
        &self.binding
    }

    /// Unchanged upstream result; absent when preflight prevented extraction.
    pub fn extraction(&self) -> Option<&ClausesOutcome> {
        self.extraction.as_ref()
    }

    /// Code of the stage that actually failed.
    pub fn code(&self) -> Code {
        match &self.cause {
            JoinCause::Preflight(error) => error.code(),
            JoinCause::Join(error) => error.code,
            JoinCause::Compile(error) => error.code(),
        }
    }
}

/// Pre-extraction input bounds and the unchanged native compiler stage limits.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Original document bytes; hard maximum 1 MiB.
    pub source_bytes: usize,
    /// Original lines, including trailing empty line; hard maximum 4096.
    pub lines: usize,
    /// Independent caller-lowered native compiler stages.
    pub compiler: CompileLimits,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            source_bytes: qsl_source::MAX_SOURCE_BYTES,
            lines: qsl_source::MAX_LINES,
            compiler: CompileLimits::default(),
        }
    }
}

/// Successful native compilation with complete, unchanged upstream extraction data.
#[derive(Debug)]
pub struct ExtractedPackage<'model> {
    mapped: MappedPackage<'model>,
    extraction: ClausesOutcome,
}

impl<'model> ExtractedPackage<'model> {
    /// Verified original/body correspondence and the actual native package.
    pub fn mapped(&self) -> &MappedPackage<'model> {
        &self.mapped
    }

    /// Original clause population, availability and diagnostics, including unchecked tags.
    pub fn extraction(&self) -> &ClausesOutcome {
        &self.extraction
    }
}

/// The I3 selection for an authored binding and its caller-assigned native body identity.
fn selection(binding: &ClauseBinding, body: &SourceIdentities) -> qsl_source::Selection {
    qsl_source::Selection {
        clause_id: binding.clause.as_str().to_owned(),
        package: binding.requirement.package().as_str().to_owned(),
        body: body.native.clone(),
    }
}

/// Extract through the I3 adapter, then compile the extracted native body.
///
/// The native join lives here (SEAM-1), not in `qsl-source` (ADR-011 §2.1 I3 row;
/// §6.2 `qsl-source` row).
fn extract_and_compile<'model>(
    original: Source,
    context: &SemanticContext,
    binding: ClauseBinding,
    body: SourceIdentities,
    models: &'model [NativeModel],
    limits: Limits,
) -> std::result::Result<ExtractedPackage<'model>, Box<JoinFailure>> {
    let extracted = match qsl_source::extract(
        original.clone(),
        context,
        selection(&binding, &body),
        qsl_source::Limits {
            source_bytes: limits.source_bytes,
            lines: limits.lines,
        },
    ) {
        Ok(extracted) => extracted,
        Err(error) => {
            let (original, _, extraction, cause) = error.into_parts();
            let cause = match cause {
                qsl_source::Cause::Preflight(error) => JoinCause::Preflight(error),
                qsl_source::Cause::Join(error) => JoinCause::Join(error),
            };
            return Err(Box::new(JoinFailure {
                original,
                binding,
                extraction,
                cause,
            }));
        }
    };
    let (map, language, extraction) = extracted.into_parts();
    match mapped::compile(
        map,
        &language,
        binding.clone(),
        body.formal,
        models,
        limits.compiler,
    ) {
        Ok(mapped) => Ok(ExtractedPackage { mapped, extraction }),
        Err(error) => Err(Box::new(JoinFailure {
            original,
            binding,
            extraction: Some(extraction),
            cause: JoinCause::Compile(error),
        })),
    }
}

/// The one selected binding is retained here after preflight; no second count guard.
pub(super) struct Selected<'a> {
    source: &'a wire::SourceFile,
    binding: &'a wire::Binding,
    body: &'a wire::Identity,
}

pub(super) fn select<'a>(
    program: &'a wire::Program,
    extraction: &'a wire::Extraction,
) -> Result<Selected<'a>> {
    let [binding] = program.clauses.as_slice() else {
        return Err(ExtractionMode::WrongClauseCount {
            actual: program.clauses.len(),
        }
        .into());
    };
    Ok(Selected {
        source: &program.source,
        binding,
        body: &extraction.body,
    })
}

pub(super) struct ExtractedRun<'model> {
    pub package: ExtractedPackage<'model>,
    pub original: FormalSource,
}

impl Selected<'_> {
    pub(super) fn compile<'model>(
        self,
        intake: &mut Intake<'_>,
        models: &'model [NativeModel],
    ) -> Result<ExtractedRun<'model>> {
        let binding = self.binding.bind()?;
        let body = self.body.bind()?;
        let original = intake.source(self.source)?;
        let context =
            qsl_source::clause_context(binding.requirement.package().as_str(), original.source())
                .map_err(|failures| {
                RunCause::Extraction(Box::new(ExtractionError::Context(failures)))
            })?;
        let package = extract_and_compile(
            original.source().clone(),
            &context,
            binding,
            body,
            models,
            Limits::default(),
        )
        .map_err(|error| RunCause::Extraction(Box::new(ExtractionError::Compile(error))))?;
        Ok(ExtractedRun { package, original })
    }
}

#[cfg(test)]
mod tests {
    //! FR-030-AC-1 ("reaches mapped compilation") and native-compile-failure coverage
    //! for the join this module owns (ADR-011 §2.1 I3 row; §6.2 `qsl-source` row).
    //! Pure I3 extraction (preflight, fence location, byte boundaries) is tested at
    //! `qsl-source/tests/it/quire_source.rs`.
    use super::*;
    use crate::runtime::{execute, ExecutionLimits, ExecutionOutcome, ValueId, ValueNode};
    use crate::runtime_test_setup as setup;
    use ix_trace_rs::trace;
    use qsl_foundation::{SourceIdentity, Span};
    use qsl_source::{AvailabilityState, ExtractedSource};
    use quire_contract_ir as ir;

    fn identity() -> SourceIdentity {
        SourceIdentity {
            authority: "test".into(),
            identity: "ix://example/runtime-rules/spec".into(),
            revision_namespace: "test".into(),
            revision: "draft:7".into(),
        }
    }

    fn source(text: &str) -> Source {
        Source::read(identity(), "rules.md", text.as_bytes(), 1_048_576).unwrap()
    }

    /// The production clause-only context. It names only the path and source
    /// identity, which every `source(..)` fixture shares.
    fn context() -> SemanticContext {
        qsl_source::clause_context("example/runtime-rules", &source("")).unwrap()
    }

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

    fn body_identities() -> SourceIdentities {
        SourceIdentities {
            native: SourceIdentity {
                authority: "test".into(),
                identity: "test:quire-body".into(),
                revision_namespace: "test".into(),
                revision: "body:7".into(),
            },
            formal: ir::SourceIdentity::new(
                ir::SourceDocumentId::new("QuireNativeBody").unwrap(),
                ir::SourceRevision::new(7).unwrap(),
            ),
        }
    }

    /// The I3 adapter's own verified extraction, which the join must pass through
    /// unchanged. `qsl-source/tests/it/quire_source.rs` pins it to Quire's upstream result.
    fn extracted(original: &Source, ctx: &SemanticContext) -> ExtractedSource {
        qsl_source::extract(
            original.clone(),
            ctx,
            selection(&binding(), &body_identities()),
            qsl_source::Limits::default(),
        )
        .unwrap()
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
        let upstream = extracted(&original, &ctx);
        let expected = upstream.extraction();
        let program = extract_and_compile(
            original.clone(),
            &ctx,
            binding(),
            body_identities(),
            &models,
            Limits::default(),
        )
        .unwrap();
        assert_eq!(program.extraction(), expected);
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
            &binding()
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
                &binding()
            );
            if dangling {
                let ExecutionOutcome::ValidationFailed(failure) = report.outcome() else {
                    panic!("invalid population must refuse before evaluation");
                };
                assert!(failure
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.diagnostic.code == Code::DanglingReference));
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
            let upstream = extracted(&original, &ctx);
            let expected = upstream.extraction();
            assert_eq!(expected.availability.state, AvailabilityState::Available);
            let error = extract_and_compile(
                original.clone(),
                &ctx,
                binding(),
                body_identities(),
                &models,
                Limits::default(),
            )
            .unwrap_err();
            assert_eq!(error.code(), code);
            assert_eq!(error.extraction(), Some(expected));
            assert_eq!(error.original().digest(), original.digest());
            assert_eq!(error.binding(), &binding());
            let JoinCause::Compile(native) = &error.cause else {
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
        let upstream = extracted(&wrong_language, &ctx);
        let error = extract_and_compile(
            wrong_language,
            &ctx,
            binding(),
            body_identities(),
            &models,
            Limits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code(), Code::UnknownLanguage);
        assert_eq!(error.extraction(), Some(upstream.extraction()));
    }

    #[test]
    #[trace("TC-108", "FR-030-AC-5")]
    fn native_stage_limits_reach_compile_after_a_successful_extraction() {
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
            let upstream = extracted(&original, &ctx);
            let body_bytes = upstream.map().body().text().len();
            let exact = Limits {
                source_bytes: original.text().len(),
                lines: original.position(original.text().len()).unwrap().line,
                ..Limits::default()
            };
            let mut body_exact = exact;
            body_exact.compiler.syntax.source_bytes = body_bytes;
            let mut body_stop = body_exact;
            body_stop.compiler.syntax.source_bytes -= 1;
            let mut package_stop = exact;
            package_stop.compiler.package.artifact_bytes = 0;
            // `body_stop` trips the S1 syntax source-bytes ceiling, which
            // QSL-236 moved onto `stage_limit_exceeded`/`input-bytes-exceeded`
            // (`LimitKind::InputBytes`). `package_stop` trips the
            // complete-V1 package-graph artifact-bytes ceiling
            // (`PackageLimitKind::ArtifactBytes`), which stays on
            // `resource_exhausted`: only `PackageLimitKind::Depth` maps
            // cleanly onto one of the four catalogued stage-limit kinds.
            for (limits, expected) in [
                (body_stop, Code::StageLimitExceeded),
                (package_stop, Code::ResourceExhausted),
            ] {
                let error = extract_and_compile(
                    original.clone(),
                    &ctx,
                    binding(),
                    body_identities(),
                    &models,
                    limits,
                )
                .unwrap_err();
                assert_eq!(error.code(), expected);
                assert_eq!(error.extraction(), Some(upstream.extraction()));
                let JoinCause::Compile(native) = &error.cause else {
                    panic!("a native stage limit must refuse at compile, after extraction");
                };
                assert_eq!(native.code(), expected);
            }
            for limits in [body_exact, exact] {
                assert!(extract_and_compile(
                    original.clone(),
                    &ctx,
                    binding(),
                    body_identities(),
                    &models,
                    limits,
                )
                .is_ok());
            }
        }
    }
}
