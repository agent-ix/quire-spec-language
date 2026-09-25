// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-026: typed native result views; upstream identities retain their serializers.

use super::super::wire;
use crate::runtime::{EvaluationUsage, InvocationRef, QualifiedName, SnapshotRef, ValidationUsage};
use qsl_foundation::{LocatedSpan, SourceIdentity, Span};
use quire_contract_ir as ir;
use serde::Serialize;
use std::borrow::Cow;

pub(super) use qsl_foundation::wire_format::WireFormat as Format;

pub(super) enum Stage {
    #[cfg(feature = "quire-extraction")]
    Extraction(super::extraction::Stage),
    File,
    Request,
    Output,
    Envelope,
    Intake,
    Native(qsl_foundation::Phase),
    Model,
    Package,
    SelectedPackage,
    Lower,
    Input,
    /// The spine stage that refused a `1-draft` program.
    Spine(crate::command::spine::SpineStage),
}

impl Serialize for Stage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            #[cfg(feature = "quire-extraction")]
            Self::Extraction(stage) => stage.as_str(),
            Self::File => "file",
            Self::Request => "request",
            Self::Output => "output",
            Self::Envelope => "envelope",
            Self::Intake => "intake",
            Self::Native(phase) => phase.as_str(),
            Self::Model => "model",
            Self::Package => "package",
            Self::SelectedPackage => "selected_package",
            Self::Lower => "lower",
            Self::Input => "input",
            Self::Spine(stage) => stage.as_str(),
        })
    }
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
    #[cfg(feature = "quire-extraction")]
    Extraction(super::extraction::Failure<'a>),
    None,
    Io {
        path: Cow<'a, str>,
        os_code: Option<i32>,
    },
    Json {
        line: usize,
        column: usize,
    },
    FileBytes {
        limit: &'static str,
    },
    FileCount {
        limit: &'static str,
        category: &'static str,
        requested: usize,
        remaining: usize,
        maximum: usize,
    },
    Identifier(&'a ir::Diagnostic),
    Native(Diagnostic<'a>),
    Model {
        source: Source<'a>,
    },
    Package {
        stage: String,
    },
    SelectedPackage {
        file: &'a str,
        expected: PackageReference,
        stage: String,
        path: Vec<PackagePath<'a>>,
        cause: Option<PackageCause<'a>>,
    },
    Lowering {
        profile: &'static str,
        package: PackageReference,
        source: Source<'a>,
        clause: Option<&'a ir::ClauseRef>,
        #[serde(flatten)]
        location: ProjectionLocation,
        upstream: &'a [ir::Diagnostic],
    },
    Input {
        expected: Reference<'a>,
        stage: &'static str,
    },
    Spine {
        source: &'a SourceIdentity,
        path: &'a str,
        span: Option<LocatedSpan>,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum SpanStatus {
    Absent,
    Located,
    Invalid,
}

#[derive(Serialize)]
pub(super) struct ProjectionLocation {
    pub span_status: SpanStatus,
    pub span: Option<LocatedSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unmapped_span: Option<Span>,
}

#[derive(Serialize)]
pub(super) struct PackageReference {
    pub format: &'static str,
    pub digest: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PackagePath<'a> {
    Field(&'a str),
    Index(usize),
}

#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum PackageCause<'a> {
    Native(Diagnostic<'a>),
    Json {
        line: usize,
        column: usize,
        message: String,
    },
}

#[derive(Serialize)]
pub(super) struct Failure<'a> {
    pub format: Format,
    pub request_digest: Option<String>,
    pub status: FailureStatus,
    pub stage: Stage,
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
    #[cfg(feature = "quire-extraction")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction: Option<super::extraction::Extracted<'a>>,
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
