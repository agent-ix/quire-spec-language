// SPDX-License-Identifier: AGPL-3.0-only
//! FR-026/028: explicit JSON views of local native outcomes and actual provenance.

use super::{wire, RunCause, RunError, RunResult};
use crate::formal_source::FormalSource;
use crate::native_model::NativeModel;
use crate::runtime::{
    EvaluationOutcome, ExecutionOutcome, ExecutionReport, ImplicationEventKind, RuntimePathSegment,
    RuntimeReference, ValidationStatus,
};
use crate::{ByteDigest, Code, Diagnostic, LocatedSpan, SourceIdentity};
use serde_json::{json, Value};

fn identity(value: &SourceIdentity) -> Value {
    json!({"identity":value.identity,"revision":value.revision})
}

fn source(value: &FormalSource) -> Value {
    json!({"identity":value.source().identity().identity,"revision":value.source().identity().revision,
        "digest":value.source().digest().to_string(),"path":value.source().path(),"formal":value.identity()})
}

fn span(value: LocatedSpan) -> Value {
    json!({"start":{"byte":value.start.byte,"line":value.start.line,"column":value.start.column},
        "end":{"byte":value.end.byte,"line":value.end.line,"column":value.end.column}})
}

fn reference(value: &RuntimeReference) -> Value {
    match value {
        RuntimeReference::Snapshot(value) => json!({"kind":"snapshot","reference":value}),
        RuntimeReference::Invocation(value) => json!({"kind":"invocation","reference":value}),
    }
}

fn runtime_path(value: &RuntimePathSegment) -> Value {
    match value {
        RuntimePathSegment::Model(value) => json!({"model":value}),
        RuntimePathSegment::Population { record, universe } => {
            json!({"population":{"record":record,"universe":universe}})
        }
        RuntimePathSegment::Object(value) => json!({"object":value}),
        RuntimePathSegment::State(value) => json!({"state":value}),
        RuntimePathSegment::Parameter(value) => json!({"parameter":value}),
        RuntimePathSegment::Result => json!({"result":true}),
        RuntimePathSegment::Field(value) => json!({"field":value}),
        RuntimePathSegment::Index(value) => json!({"index":value}),
    }
}

fn diagnostic(value: &Diagnostic) -> Value {
    json!({"phase":value.phase.as_str(),"code":value.code.as_str(),"message":value.message,
        "source":identity(&value.source),"path":value.path,"span":span(value.span),
        "upstream":value.upstream,
        "runtime":value.runtime.as_ref().map(|location| json!({"artifact":reference(&location.artifact),
            "observation":location.observation,"requirement":location.requirement,"clause":location.clause,
            "path":location.path.iter().map(runtime_path).collect::<Vec<_>>()}))})
}

pub(super) fn error(error: &RunError) -> Value {
    let (stage, code, details) = match &error.cause {
        RunCause::Io { path, error } => (
            "file",
            "io-error",
            json!({"path":path.to_string_lossy(),"os_code":error.raw_os_error()}),
        ),
        RunCause::Json(e) => (
            "request",
            "invalid-request",
            json!({"line":e.line(),"column":e.column()}),
        ),
        RunCause::Format => ("envelope", Code::UnknownWire.as_str(), Value::Null),
        RunCause::Limit(name) => (
            "intake",
            Code::ResourceExhausted.as_str(),
            json!({"limit":name}),
        ),
        RunCause::Digest(_) => ("request", "invalid-digest", Value::Null),
        RunCause::Identifier(e) => ("request", "invalid-identifier", json!(e)),
        RunCause::Native(e) => (e.phase.as_str(), e.code.as_str(), diagnostic(e)),
        RunCause::Model(e) => (
            "model",
            e.code().as_str(),
            json!({"source":source(e.source())}),
        ),
        RunCause::Package(e) => (
            "package",
            e.code.as_str(),
            json!({"stage":e.stage.to_string()}),
        ),
        RunCause::SelectedPackage {
            file,
            expected,
            error,
        } => (
            "package",
            error.code.as_str(),
            json!({
                "file":file,"expected":{"format":expected.format(),"digest":expected.digest().to_string()},
                "stage":error.stage.to_string(),
                "path":error.path.iter().map(|part|match part {
                    crate::package::PackagePathSegment::Field(name) => json!({"field":name}),
                    crate::package::PackagePathSegment::Index(index) => json!({"index":index}),
                }).collect::<Vec<_>>(),
                "cause":error.cause.as_ref().map(|cause|match cause {
                    crate::package::PackageCause::Native(value) => diagnostic(value),
                    crate::package::PackageCause::Json(value) => json!({"line":value.line(),"column":value.column(),"message":value.to_string()}),
                }),
            }),
        ),
        RunCause::Input(e) => (
            "input",
            e.code.as_str(),
            json!({"expected":reference(&e.expected),"stage":match e.stage {
                crate::runtime::InputReadStage::Selection => "selection",
                crate::runtime::InputReadStage::Envelope => "envelope",
                crate::runtime::InputReadStage::Body => "body",
                crate::runtime::InputReadStage::Construction => "construction",
            }}),
        ),
    };
    json!({"format":"native-run-result/1", "request_digest":error.request_digest.map(|value|value.to_string()),
        "status":if error.exit_code()==3 { "incomplete" } else { "refused" },
        "stage":stage,"code":code,"message":error.to_string(),"details":details})
}

pub(super) fn report(
    digest: ByteDigest,
    selection: &wire::Selection,
    models: &[NativeModel],
    report: &ExecutionReport<'_, '_>,
) -> RunResult {
    let usage = report.validation_usage();
    let mut value = json!({"format":"native-run-result/1","request_digest":digest.to_string(),
        "package":{"digest":report.package().digest().to_string(),"canonical_identity":report.package().canonical_identity().to_string()},
        "source":source(&report.package().checked().bindings().source),
        "models":models.iter().map(|model|json!({"owner":model.environment().owner(),"digest":model.digest().to_string(),"source":source(model.source())})).collect::<Vec<_>>(),
        "inputs":{"snapshots":report.input().snapshots.iter().map(|value|value.reference()).collect::<Vec<_>>(),
            "invocations":report.input().invocations.iter().map(|value|value.reference()).collect::<Vec<_>>()},
        "selection":selection,
        "validation_usage":{"artifacts":usage.artifacts,"artifact_bytes":usage.artifact_bytes,"objects":usage.objects,"work":usage.work,"text_steps":usage.text_steps,"diagnostics":usage.diagnostics}});
    let exit_code = match report.outcome() {
        ExecutionOutcome::ValidationFailed(failure) => {
            value["stage"] = json!("validate");
            value["diagnostics"] = json!(failure
                .diagnostics
                .iter()
                .map(diagnostic)
                .collect::<Vec<_>>());
            value["terminal"] = json!(failure.terminal.as_deref().map(diagnostic));
            match failure.status {
                ValidationStatus::Refused => {
                    value["status"] = json!("refused");
                    1
                }
                ValidationStatus::Incomplete => {
                    value["status"] = json!("incomplete");
                    3
                }
            }
        }
        ExecutionOutcome::Evaluated {
            result,
            usage,
            events,
            cost_model,
        } => {
            value["stage"] = json!("evaluate");
            value["cost_model"] = json!(cost_model);
            value["evaluation_usage"] = json!({"expression_steps":usage.expression_steps,"graph_steps":usage.graph_steps,
                "comparisons":usage.comparisons,"text_steps":usage.text_steps,"events":usage.events,
                "expression_depth":usage.expression_depth,"comparison_depth":usage.comparison_depth});
            value["events"] = json!(events.iter().map(|event| {
                let mut value = json!({"implication":event.implication.0,"operand":event.operand.0,"span":{"start":event.span.start,"end":event.span.end}});
                value["kind"] = json!(match event.kind {
                    ImplicationEventKind::AntecedentEntered => "antecedent_entered",
                    ImplicationEventKind::AntecedentCompleted(truth) => { value["truth"] = json!(truth); "antecedent_completed" },
                    ImplicationEventKind::ConsequentEntered => "consequent_entered",
                }); value
            }).collect::<Vec<_>>());
            match result {
                EvaluationOutcome::Completed(truth) => {
                    value["status"] = json!("completed");
                    value["truth"] = json!(truth);
                    u8::from(!truth)
                }
                EvaluationOutcome::Refused(error) => {
                    value["status"] = json!("refused");
                    value["diagnostic"] = diagnostic(error);
                    1
                }
                EvaluationOutcome::Incomplete(error) => {
                    value["status"] = json!("incomplete");
                    value["diagnostic"] = diagnostic(error);
                    3
                }
            }
        }
    };
    RunResult { exit_code, value }
}
