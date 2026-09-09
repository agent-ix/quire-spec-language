// SPDX-License-Identifier: AGPL-3.0-only
//! FR-031: clause-only Quire context and retained local command observations.

use super::{output, wire, Intake, Result, RunCause, RunResult};
use crate::formal_source::FormalSource;
use crate::native_model::NativeModel;
use crate::quire_source::{self, ExtractedPackage, Selection};
use crate::SourceIdentity;
use quire_contract_ir as ir;
use quire_rs::semantic::{read_semantic_block, BundleIndex, ClausesOutcome, SemanticContext};
use serde_json::{json, Value};

pub(super) struct ExtractedRun<'model> {
    pub package: ExtractedPackage<'model>,
    original: FormalSource,
}

impl ExtractedRun<'_> {
    pub fn attach(&self, mut result: RunResult) -> RunResult {
        let mapping = self.package.mapped().mapping();
        result.value["extraction"] = json!({
            "original":output::source(&self.original),
            "outcome":outcome(self.package.extraction()),
            "mapping":{
                "region":{"start":mapping.region().start,"end":mapping.region().end},
                "segments":mapping.segments().iter().map(|segment|json!({
                    "body":{"start":segment.body.start,"end":segment.body.end},
                    "original":{"start":segment.original.start,"end":segment.original.end}
                })).collect::<Vec<_>>(),
                "drop_final_newline":mapping.layout().drop_final_newline,
            }
        });
        result
    }
}

pub(super) fn compile<'model>(
    intake: &mut Intake<'_>,
    program: &wire::Program,
    extraction: &wire::Extraction,
    models: &'model [NativeModel],
) -> Result<ExtractedRun<'model>> {
    let [binding] = program.clauses.as_slice() else {
        return Err(RunCause::ExtractionMode(
            "extraction requires exactly one authored clause binding",
        ));
    };
    let binding = binding.bind()?;
    let original = intake.source(&program.source)?;
    let body = &extraction.body;
    let selection = Selection {
        binding,
        body: SourceIdentity {
            identity: body.identity.clone(),
            revision: body.revision.clone(),
        },
        formal: ir::SourceIdentity::new(
            ir::SourceDocumentId::new(&body.document)?,
            ir::SourceRevision::new(body.formal_revision)?,
        ),
    };
    // A local clause-only context has no exported archetypes or imported schemas.
    // Quire still validates its public configuration; model imports remain native.
    let module = read_semantic_block(
        &json!({
            "contract_version":"1.0.0", "semantic_core":"0.1.0",
            "package":selection.binding.requirement.package().as_str(),
            "exports":[], "targets":["markdown"]
        }),
        &[],
        &|_| false,
    )
    .map_err(RunCause::QuireContext)?;
    let context = SemanticContext::new(module, original.source().path(), BundleIndex::default())
        .with_source_identity(original.source().identity().identity.clone());
    let package = quire_source::compile(
        original.source().clone(),
        &context,
        selection,
        models,
        quire_source::Limits::default(),
    )?;
    Ok(ExtractedRun { package, original })
}

fn outcome(value: &ClausesOutcome) -> Value {
    json!({"availability":value.availability,"clauses":value.clauses,
        "clause_text":value.clause_text,"diagnostics":value.diagnostics})
}

pub(super) fn error(value: &quire_source::Error) -> Value {
    let original = value.original();
    let native = match &value.cause {
        quire_source::Cause::Join(error) => output::diagnostic(error),
        quire_source::Cause::Compile(error) => json!({
            "diagnostic":error.native_diagnostic().map(output::diagnostic),
            "original_spans":match error.original_spans() {
                Ok(spans) => json!(spans.map(|spans|spans.into_iter().map(output::span).collect::<Vec<_>>())),
                Err(error) => json!({"mapping_error":output::diagnostic(&error)}),
            }
        }),
    };
    let binding = &value.selection().binding;
    json!({
        "original":{"identity":original.identity().identity,"revision":original.identity().revision,
            "path":original.path(),"digest":original.digest().to_string()},
        "selection":{"name":binding.name,"requirement":binding.requirement,"clause":binding.clause,
            "execution_point":binding.execution_point},
        "outcome":value.extraction().map(outcome),"cause":native,
    })
}
