// SPDX-License-Identifier: AGPL-3.0-only
//! FR-026: typed native result views; upstream identities retain their serializers.

use super::super::wire;
use crate::runtime::{EvaluationUsage, InvocationRef, QualifiedName, SnapshotRef, ValidationUsage};
use crate::{LocatedSpan, SourceIdentity, Span};
use quire_contract_ir as ir;
use serde::Serialize;
use std::borrow::Cow;

#[derive(Serialize)]
pub(super) enum Format {
    #[serde(rename = "native-run-result/1")]
    NativeResult,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum FailureStatus {
    Refused,
    Incomplete,
}

#[derive(Serialize)]
pub(super) struct Source<'a> {
    #[serde(flatten)]
    pub identity: &'a SourceIdentity,
    pub digest: String,
    pub path: &'a str,
    pub formal: &'a ir::SourceIdentity,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "reference", rename_all = "snake_case")]
pub(super) enum Reference<'a> {
    Snapshot(&'a SnapshotRef),
    Invocation(&'a InvocationRef),
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RuntimePath<'a> {
    Model(&'a ir::RequirementRef),
    Population {
        record: &'a ir::SymbolName,
        universe: &'a ir::SymbolName,
    },
    Object(&'a str),
    State(&'a QualifiedName),
    Parameter(&'a QualifiedName),
    Result(bool),
    Field(&'a ir::SymbolName),
    Index(usize),
}

#[derive(Serialize)]
pub(super) struct RuntimeLocation<'a> {
    pub artifact: Reference<'a>,
    pub observation: Option<ir::StateObservation>,
    pub requirement: &'a ir::RequirementRef,
    pub clause: &'a ir::ClauseId,
    pub path: Vec<RuntimePath<'a>>,
}

#[derive(Serialize)]
pub(super) struct Diagnostic<'a> {
    pub phase: &'static str,
    pub code: &'static str,
    pub message: &'a str,
    pub source: &'a SourceIdentity,
    pub path: &'a str,
    pub span: LocatedSpan,
    pub upstream: Option<&'a ir::Diagnostic>,
    pub runtime: Option<RuntimeLocation<'a>>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum Details<'a> {
    None,
    Io {
        path: Cow<'a, str>,
        os_code: Option<i32>,
    },
    Json {
        line: usize,
        column: usize,
    },
    Limit {
        limit: &'static str,
    },
    Identifier(&'a ir::Diagnostic),
    Native(Diagnostic<'a>),
    Model {
        source: Source<'a>,
    },
    Package {
        stage: String,
    },
    Input {
        expected: Reference<'a>,
        stage: &'static str,
    },
}

#[derive(Serialize)]
pub(super) struct Failure<'a> {
    pub format: Format,
    pub request_digest: Option<String>,
    pub status: FailureStatus,
    pub stage: &'static str,
    pub code: &'static str,
    pub message: String,
    pub details: Details<'a>,
}

#[derive(Serialize)]
pub(super) struct Package {
    pub digest: String,
    pub canonical_identity: String,
}

#[derive(Serialize)]
pub(super) struct Model<'a> {
    pub owner: &'a ir::RequirementRef,
    pub digest: String,
    pub source: Source<'a>,
}

#[derive(Serialize)]
pub(super) struct Inputs {
    pub snapshots: Vec<SnapshotRef>,
    pub invocations: Vec<InvocationRef>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum EventKind {
    AntecedentEntered,
    AntecedentCompleted { truth: bool },
    ConsequentEntered,
}

#[derive(Serialize)]
pub(super) struct Event {
    pub implication: usize,
    pub operand: usize,
    pub span: Span,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(super) enum Evaluation<'a> {
    Completed { truth: bool },
    Refused { diagnostic: Diagnostic<'a> },
    Incomplete { diagnostic: Diagnostic<'a> },
}

#[derive(Serialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub(super) enum Outcome<'a> {
    Validate {
        status: FailureStatus,
        diagnostics: Vec<Diagnostic<'a>>,
        terminal: Option<Diagnostic<'a>>,
    },
    Evaluate {
        cost_model: &'static str,
        evaluation_usage: &'a EvaluationUsage,
        events: Vec<Event>,
        #[serde(flatten)]
        result: Evaluation<'a>,
    },
}

#[derive(Serialize)]
pub(super) struct Report<'a> {
    pub format: Format,
    pub request_digest: String,
    pub package: Package,
    pub source: Source<'a>,
    pub models: Vec<Model<'a>>,
    pub inputs: Inputs,
    pub selection: &'a wire::Selection,
    pub validation_usage: ValidationUsage,
    #[serde(flatten)]
    pub outcome: Outcome<'a>,
}
