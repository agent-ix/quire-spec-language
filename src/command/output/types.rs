// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-026: typed native result views; upstream identities retain their serializers.

use crate::runtime::{InvocationRef, SnapshotRef};
use qsl_foundation::{LocatedSpan, SourceIdentity, Span};
use quire_contract_model as ir;
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
    Spine(qsl_replay::spine::SpineStage),
    /// The stage a `1-draft` spine `run` refused at: the wrapped compile
    /// refusal's own stage, or `call` (FR-100).
    SpineRun(&'static str),
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
            Self::SpineRun(stage) => stage,
        })
    }
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
pub(super) struct Diagnostic<'a> {
    pub phase: &'static str,
    pub code: &'static str,
    pub message: &'a str,
    pub source: &'a SourceIdentity,
    pub path: &'a str,
    pub span: LocatedSpan,
    pub upstream: Option<&'a ir::Diagnostic>,
    /// FR-001: `cause` and `label`, on an `invalid_source_identity`
    /// refusal only.
    #[serde(flatten)]
    pub identity: Option<qsl_foundation::source::IdentityCauseFields>,
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
    ConfiguredBytes {
        limit: &'static str,
        maximum: usize,
        option: &'static str,
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
    /// FR-100: a stage-`call` refusal naming the request's `function`
    /// string (`missing_declaration`, `unsupported_construct`).
    Function {
        function: &'a str,
    },
    /// FR-100: a stage-`call` refusal naming the argument's `parameter`
    /// string (`invalid_runtime_input`).
    Parameter {
        parameter: &'a str,
    },
    /// FR-100: a stage-`call` refusal naming the argument's zero-based
    /// declared position (`invalid_runtime_input`, `WrongValueKind`).
    Position {
        position: usize,
    },
    /// FR-100 typed model admission's foreign-universe payload.
    AdmissionForeignUniverse {
        position: usize,
        required: String,
        supplied: String,
    },
    /// FR-100 typed model-selection admission details for a document offer.
    AdmissionForeignModelDocument {
        position: usize,
        model_identity: String,
        expected: AdmissionModelSelection,
    },
    /// FR-100 typed model-selection admission details for a population offer.
    AdmissionForeignModelPopulation {
        position: usize,
        population: AdmissionDeclarationKey,
        expected: AdmissionModelSelection,
    },
    /// FR-100 typed foreign-type admission details.
    AdmissionForeignType {
        position: usize,
        member: String,
        #[serde(rename = "type")]
        type_name: AdmissionDeclarationKey,
    },
    /// FR-100 "Internal failure at S6a": the `InternalFault`'s own stable
    /// stage and invariant identifiers (`runtime_invariant`, exit 30).
    Invariant {
        stage: &'static str,
        invariant: &'static str,
    },
}

#[derive(Serialize)]
pub(super) struct AdmissionModelSelection {
    pub identity: String,
    pub version: String,
    pub digest_domain: &'static str,
    pub digest: String,
}

#[derive(Serialize)]
pub(super) struct AdmissionDeclarationKey {
    pub package: String,
    pub node: String,
}

/// FR-100: the program source as `spine-run-result/1` renders one -- its
/// four FR-001 labels, `sha256:` source digest and authored path. Narrower
/// than [`Source`]: no formal document/revision member.
#[derive(Serialize)]
pub(super) struct RunSource<'a> {
    #[serde(flatten)]
    pub identity: &'a SourceIdentity,
    pub digest: String,
    pub path: &'a str,
}

/// FR-100: a value [`SpineOutcome::Completed`] carries.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum SpineValue {
    Boolean {
        value: bool,
    },
    Integer {
        decimal: String,
    },
    Reference {
        universe: String,
        #[serde(rename = "type")]
        object_type: String,
        object_identity: String,
    },
}

/// FR-096/FR-100: a `refused` record's or cause's resolved locus -- the
/// `sha256:` digest of the source the region belongs to, and its span
/// rendered over that source.
#[derive(Serialize)]
pub(super) struct SpineLocus {
    pub source_digest: String,
    pub span: LocatedSpan,
}

/// FR-096/FR-100: the declaration and child-index path a `refused` outcome
/// arose at (`Evaluation.location`).
#[derive(Serialize)]
pub(super) struct SpineLocation {
    pub origin: SpineOrigin,
    pub path: Vec<usize>,
}

/// FR-096/FR-100's `location.origin`.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum SpineOrigin {
    Body { function: String, index: usize },
    Measure { function: String, index: usize },
    Expression,
    TypeDeclaration { name: String },
    StateClause { clause: String, index: usize },
    ProtocolAttempt { protocol: usize, attempt: usize },
}

/// FR-100's `outcome` member. `Refused`'s members follow the refusal
/// table's two rows exactly, by which of `fields`/`locus` are present:
/// `code` and `cause` are on every refused outcome (FR-100's refusal table
/// gives both members to a record row and a family-no-record row alike; no
/// row omits either), a record row also carries `fields`/`locus` (plus
/// `location` when known), and a family-no-record row carries `location`
/// alone besides `code`/`cause`.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum SpineOutcome {
    Completed {
        value: SpineValue,
    },
    Refused {
        code: &'static str,
        cause: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        fields: Option<std::collections::BTreeMap<&'static str, String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        locus: Option<SpineLocus>,
        #[serde(skip_serializing_if = "Option::is_none")]
        location: Option<SpineLocation>,
    },
    Undefined {
        reason: &'static str,
    },
    Incomplete {
        limit: &'static str,
    },
}

/// The `spine-run-result/1` outcome document (FR-100).
#[derive(Serialize)]
pub(super) struct SpineRunReport<'a> {
    pub format: Format,
    pub request_digest: String,
    pub package_id: String,
    pub source: RunSource<'a>,
    pub function: &'a str,
    pub outcome: SpineOutcome,
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
    pub stage: Stage,
    pub code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<&'static str>,
    pub message: String,
    pub details: Details<'a>,
    pub basis: &'static str,
}

/// Artifact commands retain their request identity and completion status on
/// stderr. The `run` command uses the closed command-error shape above.
#[derive(Serialize)]
pub(super) struct ArtifactFailure<'a> {
    #[serde(flatten)]
    pub failure: Failure<'a>,
    pub request_digest: Option<String>,
    pub status: ArtifactFailureStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ArtifactFailureStatus {
    Refused,
    Incomplete,
}
