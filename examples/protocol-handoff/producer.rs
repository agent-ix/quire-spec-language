// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042/TC-121: example compilation recipe and independent input selections.
//! It calls production APIs; it is not a production request format or B reader.

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use quire_contract_ir as ir;
use quire_spec_language::{
    checking::{
        composed::{self, proofs},
        CheckBindings, ClauseBinding,
    },
    formal_source::FormalSource,
    linking::composed::{
        self as linking, binding, definition_source::RegisteredDefinition as R, definitions,
        models::ModelInput,
    },
    model_source::{self, ModelSourceLimits},
    native_model::{ModelLimits, NativeModel},
    protocol_artifact::{
        self as artifact,
        handoff::{
            MutationCase, MutationInput, MutationManifest, SelectedArtifactLimits,
            SelectedClockInput, SelectedDeclaration, SelectedDependency, SelectedModel,
            SelectedSource, SelectedTemporal, Selection, SelectionV2, MUTATION_MANIFEST_FORMAT,
            PUBLISHED_ARTIFACT_REFERENCE_FILE, PUBLISHED_MUTATION_MANIFEST_FILE,
            PUBLISHED_OFFER_FILE, PUBLISHED_SELECTION_FILE,
        },
        native, v2, wire as w,
    },
    ByteDigest, Source, SourceIdentity,
};

const AUTHORITY: &str = "ix://agent-ix/quire-spec-language";
const STANDARD: &str = "ix://agent-ix/quire-specification";
const FORMAL_NAMESPACE: &str = "quire-contract-ir/source-revision";
const REQUIREMENT_NAMESPACE: &str = "quire-contract-ir/requirement-revision";
const DEFINITION_NAMESPACE: &str = "quire/native-definition-revision";
const CONTRACT: &[u8] = include_bytes!("../../docs/compiled-protocol-v1.md");
const CONTRACT_V2: &[u8] = include_bytes!("../../docs/compiled-protocol-v2.md");
const EVENT_CLOCK: &[u8] =
    b"{ \"kind\": \"event_position\", \"sequence_authority\": \"workflow-events\" }\n";
const SAMPLE_CLOCK: &[u8] = b"{ \"kind\": \"fixed_sample\", \"epoch\": { \"kind\": \"integer\", \"decimal\": \"0\" }, \"period\": { \"kind\": \"rational\", \"numerator\": \"1\", \"denominator\": \"2\" }, \"unit\": \"second\" }\n";
const TIMESTAMP_CLOCK: &[u8] =
    b"{ \"kind\": \"timestamped_event\", \"timestamp_unit\": \"millisecond\" }\n";

struct TemporalRecipe {
    name: &'static str,
    definition: R,
    clock_identity: &'static str,
    clock_file: &'static str,
    clock_bytes: &'static [u8],
}

const TEMPORAL_V2: &[TemporalRecipe] = &[
    TemporalRecipe {
        name: "Due",
        definition: R::EventPosition,
        clock_identity: "event-clock",
        clock_file: "event-clock.json",
        clock_bytes: EVENT_CLOCK,
    },
    TemporalRecipe {
        name: "DueSample",
        definition: R::FixedSample,
        clock_identity: "sample-clock",
        clock_file: "sample-clock.json",
        clock_bytes: SAMPLE_CLOCK,
    },
    TemporalRecipe {
        name: "DueTimestamp",
        definition: R::TimestampedWindow,
        clock_identity: "timestamp-clock",
        clock_file: "timestamp-clock.json",
        clock_bytes: TIMESTAMP_CLOCK,
    },
];

/// The recipe author selects clause owners and execution points before parsing.
struct UnitRecipe {
    file: &'static str,
    identity: &'static str,
    document: &'static str,
    requirement: &'static str,
    body: &'static str,
    clauses: &'static [AuthoredClause],
    profiles: &'static [(&'static str, R)],
}

struct AuthoredClause {
    name: &'static str,
    clause: &'static str,
    execution: AuthoredExecution,
}

#[derive(Clone, Copy)]
enum AuthoredExecution {
    Handler,
    Pre,
    Post,
}

const UNITS: &[UnitRecipe] = &[
    UnitRecipe {
        file: "predicates.native",
        identity: "ix://agent-ix/quire-spec-language/examples/protocol-handoff/predicates",
        document: "ProtocolHandoffPredicates",
        requirement: "HandoffPredicates",
        body: include_str!("predicates.body.native"),
        clauses: &[
            AuthoredClause {
                name: "Allowed",
                clause: "allowed",
                execution: AuthoredExecution::Handler,
            },
            AuthoredClause {
                name: "Bounded",
                clause: "bounded",
                execution: AuthoredExecution::Handler,
            },
        ],
        profiles: &[
            ("G", R::StateGraph),
            ("T", R::EventPosition),
            ("P", R::Protocol),
        ],
    },
    UnitRecipe {
        file: "state.native",
        identity: "ix://agent-ix/quire-spec-language/examples/protocol-handoff/state",
        document: "ProtocolHandoffState",
        requirement: "HandoffState",
        body: include_str!("state.body.native"),
        clauses: &[
            AuthoredClause {
                name: "Healthy",
                clause: "healthy",
                execution: AuthoredExecution::Handler,
            },
            AuthoredClause {
                name: "BeforeApply",
                clause: "before_apply",
                execution: AuthoredExecution::Pre,
            },
            AuthoredClause {
                name: "AfterApply",
                clause: "after_apply",
                execution: AuthoredExecution::Post,
            },
        ],
        profiles: &[
            ("G", R::StateGraph),
            ("T", R::EventPosition),
            ("P", R::Protocol),
        ],
    },
    UnitRecipe {
        file: "temporal.native",
        identity: "ix://agent-ix/quire-spec-language/examples/protocol-handoff/temporal",
        document: "ProtocolHandoffTemporal",
        requirement: "HandoffTemporal",
        body: include_str!("temporal.body.native"),
        clauses: &[AuthoredClause {
            name: "Due",
            clause: "due",
            execution: AuthoredExecution::Handler,
        }],
        profiles: &[
            ("G", R::StateGraph),
            ("T", R::EventPosition),
            ("P", R::Protocol),
        ],
    },
    UnitRecipe {
        file: "workflow.native",
        identity: "ix://agent-ix/quire-spec-language/examples/protocol-handoff/workflow",
        document: "ProtocolHandoffWorkflow",
        requirement: "HandoffWorkflow",
        body: include_str!("workflow.body.native"),
        clauses: &[AuthoredClause {
            name: "Flow",
            clause: "flow",
            execution: AuthoredExecution::Handler,
        }],
        profiles: &[
            ("G", R::StateGraph),
            ("T", R::EventPosition),
            ("P", R::Protocol),
        ],
    },
];

// The committed `/2` handoff predates the distinct-authority `/1` correction.
// Keep its exact authored source/model inputs frozen rather than silently
// regenerating another contract version as a side effect of `/1` maintenance.
const UNITS_V2: &[UnitRecipe] = &[
    UnitRecipe {
        file: "predicates.native",
        identity: "ix://agent-ix/quire-spec-language/examples/protocol-handoff/predicates",
        document: "ProtocolHandoffPredicates",
        requirement: "HandoffPredicates",
        body: include_str!("predicates.body.native"),
        clauses: &[
            AuthoredClause {
                name: "Allowed",
                clause: "allowed",
                execution: AuthoredExecution::Handler,
            },
            AuthoredClause {
                name: "Bounded",
                clause: "bounded",
                execution: AuthoredExecution::Handler,
            },
        ],
        profiles: &[
            ("G", R::StateGraph),
            ("T", R::EventPosition),
            ("F", R::FixedSample),
            ("W", R::TimestampedWindow),
            ("P", R::Protocol),
        ],
    },
    UnitRecipe {
        file: "state.native",
        identity: "ix://agent-ix/quire-spec-language/examples/protocol-handoff/state",
        document: "ProtocolHandoffState",
        requirement: "HandoffState",
        body: include_str!("state.body.native"),
        clauses: &[
            AuthoredClause {
                name: "Healthy",
                clause: "healthy",
                execution: AuthoredExecution::Handler,
            },
            AuthoredClause {
                name: "BeforeApply",
                clause: "before_apply",
                execution: AuthoredExecution::Pre,
            },
            AuthoredClause {
                name: "AfterApply",
                clause: "after_apply",
                execution: AuthoredExecution::Post,
            },
        ],
        profiles: &[
            ("G", R::StateGraph),
            ("T", R::EventPosition),
            ("F", R::FixedSample),
            ("W", R::TimestampedWindow),
            ("P", R::Protocol),
        ],
    },
    UnitRecipe {
        file: "temporal-v2.native",
        identity: "ix://agent-ix/quire-spec-language/examples/protocol-handoff/temporal-v2",
        document: "ProtocolHandoffTemporalV2",
        requirement: "HandoffTemporalV2",
        body: include_str!("temporal-v2.body.native"),
        clauses: &[
            AuthoredClause {
                name: "Due",
                clause: "due",
                execution: AuthoredExecution::Handler,
            },
            AuthoredClause {
                name: "DueSample",
                clause: "due_sample",
                execution: AuthoredExecution::Handler,
            },
            AuthoredClause {
                name: "DueTimestamp",
                clause: "due_timestamp",
                execution: AuthoredExecution::Handler,
            },
        ],
        profiles: &[
            ("G", R::StateGraph),
            ("T", R::EventPosition),
            ("F", R::FixedSample),
            ("W", R::TimestampedWindow),
            ("P", R::Protocol),
        ],
    },
    UnitRecipe {
        file: "workflow.native",
        identity: "ix://agent-ix/quire-spec-language/examples/protocol-handoff/workflow",
        document: "ProtocolHandoffWorkflow",
        requirement: "HandoffWorkflow",
        body: include_str!("workflow-v2-frozen.body.native"),
        clauses: &[AuthoredClause {
            name: "Flow",
            clause: "flow",
            execution: AuthoredExecution::Handler,
        }],
        profiles: &[
            ("G", R::StateGraph),
            ("T", R::EventPosition),
            ("F", R::FixedSample),
            ("W", R::TimestampedWindow),
            ("P", R::Protocol),
        ],
    },
];

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("usage: native_protocol_handoff <new-output-directory>")]
    Arguments,
    #[error("usage: native_protocol_v2_handoff <new-output-directory>")]
    ArgumentsV2,
    #[error("cannot access {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("the producer executable is not an ELF version-1 binary")]
    BinaryFormat,
    #[error("invalid authored formal identifier: {0:?}")]
    Identifier(ir::Diagnostic),
    #[error("{0}")]
    Source(#[from] Box<quire_spec_language::Diagnostic>),
    #[error("{0}")]
    Model(#[from] Box<model_source::ModelSourceError>),
    #[error("{stage} did not complete: {completed}/{expected} records, {issues} issues, incomplete={incomplete}; first issue: {first:?}")]
    Stage {
        stage: &'static str,
        completed: usize,
        expected: usize,
        issues: usize,
        incomplete: bool,
        first: Option<Box<StageIssue>>,
    },
    #[error("{stage}: {cause}; source locus: {locus:?}")]
    Artifact {
        stage: &'static str,
        #[source]
        cause: artifact::Error,
        locus: Option<Box<w::Locus>>,
    },
    #[error("cannot serialize an example selection: {0}")]
    Json(#[from] serde_json::Error),
    #[error("cannot serialize the selected reader limits: {0}")]
    HandoffLimits(#[from] artifact::handoff::LimitWidthError),
    #[error("invalid generated handoff path: {0}")]
    HandoffPath(String),
    #[error("the independently read package differs from the native emission")]
    RoundTrip,
    #[error("authored declaration {name}: {cause}")]
    Declaration {
        name: String,
        #[source]
        cause: DeclarationCause,
    },
    #[error("dependency {identity}: expected one selection, found {matches}")]
    Dependency { identity: String, matches: usize },
    #[error("source span {span:?} exceeds the wire offset range")]
    Span { span: quire_spec_language::Span },
    #[error("the authored recipe requires one operation, found {count}")]
    OperationCount { count: usize },
    #[error("the authored recipe requires Workflow::apply, found {context:?}::{name:?}")]
    OperationIdentity {
        context: ir::SymbolName,
        name: ir::SymbolName,
    },
    #[error(
        "authored precondition {name}: expected operation anchor {expected:?}, found {actual:?}"
    )]
    PreAnchorMismatch {
        name: String,
        expected: ir::AnchorName,
        actual: ir::AnchorName,
    },
    #[error(
        "authored postcondition {name}: expected operation anchor {expected:?}, found {actual:?}"
    )]
    PostAnchorMismatch {
        name: String,
        expected: ir::AnchorName,
        actual: ir::AnchorName,
    },
    #[error("the authored recipe inventory exceeds the wire index range")]
    InventoryLimit,
    #[error("cannot construct deterministic v2 mutation fixture {0}")]
    MutationFixture(&'static str),
    #[error("v2 mutation {identity} expected refusal {expected}, got {actual}")]
    MutationReplay {
        identity: String,
        expected: String,
        actual: String,
    },
}

/// Distinct failures of the recipe's authored declaration correspondence.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DeclarationCause {
    #[error("not present in the original namespace")]
    Missing,
    #[error("ambiguous in the original namespace: {matches} declarations")]
    Ambiguous { matches: usize },
    #[error("original syntax is unavailable")]
    MissingSyntax,
    #[error("original unit is unavailable")]
    MissingUnit,
    #[error("clause was selected in a different source unit")]
    DifferentSource,
}

/// Fixed-size summaries never retain a report, source body or diagnostic list.
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum StageIssue {
    #[error("{kind}, supplied={supplied:?}, span={span:?}, code={code:?}")]
    Namespace {
        kind: &'static str,
        supplied: Option<usize>,
        span: Option<quire_spec_language::Span>,
        code: Option<quire_spec_language::Code>,
    },
    #[error("declaration {declaration}: {disposition:?}, site={site:?}, cause={cause:?}")]
    Proof {
        declaration: usize,
        disposition: proofs::ProofDisposition,
        site: Option<composed::Site>,
        cause: Option<ProofCause>,
    },
}

#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum ProofCause {
    #[error("upstream type refusal")]
    UpstreamType,
    #[error("authored correspondence: {0:?}")]
    Correspondence(proofs::CorrespondenceError),
    #[error("unproved obligation: {diagnostics} diagnostics")]
    Unproved { diagnostics: usize },
    #[error("unsupported proof prerequisite: {0:?}")]
    Unsupported(proofs::Unsupported),
    #[error("dependency declaration {declaration}")]
    Dependency { declaration: usize },
    #[error("unrecognized proof cause")]
    Unrecognized,
}

fn io_at(path: &Path, source: io::Error) -> Error {
    Error::Io {
        path: path.to_owned(),
        source,
    }
}

fn revision(namespace: &str, value: &str) -> w::Revision {
    w::Revision {
        namespace: namespace.into(),
        value: value.into(),
    }
}

fn reference(
    authority: &str,
    kind: w::ArtifactKind,
    identity: &str,
    revision: w::Revision,
    wire: &str,
    version: &str,
    bytes: &[u8],
) -> w::ArtifactRef {
    digest_reference(
        authority,
        kind,
        identity,
        revision,
        wire,
        version,
        ByteDigest::of(bytes),
    )
}

/// Build an artifact reference from an already-computed digest, for the one
/// case (the producer's own binary) whose original bytes are never
/// materialized as a dependency (see `producer_binary_digest`).
fn digest_reference(
    authority: &str,
    kind: w::ArtifactKind,
    identity: &str,
    revision: w::Revision,
    wire: &str,
    version: &str,
    digest: ByteDigest,
) -> w::ArtifactRef {
    w::ArtifactRef {
        ref_version: "ix.artifact-ref/3-draft".into(),
        kind,
        authority: authority.into(),
        identity: identity.into(),
        revision,
        digest,
        wire: w::Wire {
            identity: wire.into(),
            version: version.into(),
        },
    }
}

/// FR-042-AC-10: hash the actual running producer executable, proving a real
/// compiler binary produced this fixture. Producer identity is a digest on
/// `Producer.binary` -- not a dependency whose original bytes an independent
/// reader must recover -- so the bytes are read only to compute the digest
/// and are never retained, written to a fixture file, or supplied as a
/// dependency's exact-byte content.
fn producer_binary_digest() -> Result<ByteDigest, Error> {
    let path = std::env::current_exe().map_err(|error| io_at(Path::new("current_exe"), error))?;
    let bytes = fs::read(&path).map_err(|error| io_at(&path, error))?;
    if !bytes.starts_with(b"\x7fELF") || bytes.get(6) != Some(&1) {
        return Err(Error::BinaryFormat);
    }
    Ok(ByteDigest::of(&bytes))
}

fn formal(source: Source, document: &str) -> Result<FormalSource, Error> {
    Ok(FormalSource::new(
        source,
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new(document).map_err(Error::Identifier)?,
            ir::SourceRevision::new(1).map_err(Error::Identifier)?,
        ),
    ))
}

fn model(bytes: &[u8]) -> Result<NativeModel, Error> {
    let source = Source::read(
        SourceIdentity {
            identity: format!("{AUTHORITY}/examples/protocol-handoff/model"),
            revision: "1".into(),
        },
        "examples/protocol-handoff/model.json",
        bytes,
        quire_spec_language::Limits::default().source_bytes,
    )?;
    Ok(model_source::read(
        formal(source, "ProtocolHandoffModel")?,
        model_source::FORMAT_V2,
        ModelSourceLimits::default(),
    )?
    .admit(ModelLimits::default())?)
}

fn selected_definitions(temporal: &[R]) -> Vec<R> {
    let mut selected = BTreeSet::new();
    let mut pending = vec![
        R::Edition,
        R::Package,
        R::StateGraph,
        R::EventPosition,
        R::Protocol,
        R::ObservationBinding,
        R::Progress,
    ];
    pending.extend_from_slice(temporal);
    while let Some(next) = pending.pop() {
        if selected.insert(next) {
            pending.extend(next.requirements());
        }
    }
    selected.into_iter().collect()
}

fn source(model: &NativeModel, recipe: &UnitRecipe) -> Result<Source, Error> {
    let mut text = "language \"ix:native\" edition \"1-draft\";\n".to_owned();
    for (alias, definition) in recipe.profiles {
        let selected = definition.selection();
        text.push_str(&format!(
            "profile {alias} = \"{}\" version \"{}\" digest \"{}\";\n",
            selected.identity, selected.revision, selected.digest
        ));
    }
    text.push_str(&format!(
        "model M = \"{}\" version \"{}\" digest \"{}\";\n",
        model.environment().owner().package().as_str(),
        model.environment().owner().revision().get(),
        model.digest()
    ));
    text.push_str(recipe.body);
    Ok(Source::read(
        SourceIdentity {
            identity: recipe.identity.into(),
            revision: "1".into(),
        },
        format!("examples/protocol-handoff/{}", recipe.file),
        text.as_bytes(),
        quire_spec_language::Limits::default().source_bytes,
    )?)
}

struct Dependency {
    artifact: w::ArtifactRef,
    bytes: Cow<'static, [u8]>,
    requires: Vec<w::ArtifactRef>,
    file: String,
}

type MutationFiles = Vec<(String, Vec<u8>)>;

fn declaration_selection(
    namespace: &linking::SyntaxNamespace,
    unit: &UnitInput,
    operation: &OperationSelection,
) -> Result<Vec<SelectedDeclaration>, Error> {
    unit.mapping
        .clauses
        .iter()
        .map(|clause| {
            let candidates = namespace.lookup(&clause.name);
            let [id] = candidates else {
                return Err(Error::Declaration {
                    name: clause.name.clone(),
                    cause: if candidates.is_empty() {
                        DeclarationCause::Missing
                    } else {
                        DeclarationCause::Ambiguous {
                            matches: candidates.len(),
                        }
                    },
                });
            };
            let syntax = namespace.syntax(*id).ok_or_else(|| Error::Declaration {
                name: clause.name.clone(),
                cause: DeclarationCause::MissingSyntax,
            })?;
            let original = namespace
                .declaration(*id)
                .and_then(|entry| namespace.unit(entry.unit()))
                .ok_or_else(|| Error::Declaration {
                    name: clause.name.clone(),
                    cause: DeclarationCause::MissingUnit,
                })?;
            let expected = unit.mapping.source.source();
            if original.source().identity() != expected.identity()
                || original.source().path() != expected.path()
                || original.source().text() != expected.text()
            {
                return Err(Error::Declaration {
                    name: clause.name.clone(),
                    cause: DeclarationCause::DifferentSource,
                });
            }
            Ok(SelectedDeclaration {
                name: clause.name.clone(),
                span: w::Span {
                    start: u32::try_from(syntax.span.start)
                        .map_err(|_| Error::Span { span: syntax.span })?,
                    end: u32::try_from(syntax.span.end)
                        .map_err(|_| Error::Span { span: syntax.span })?,
                },
                requirement: w::Requirement {
                    package: clause.requirement.package().as_str().into(),
                    identity: clause.requirement.requirement().as_str().into(),
                    revision: revision(
                        REQUIREMENT_NAMESPACE,
                        &clause.requirement.revision().get().to_string(),
                    ),
                },
                clause: clause.clause.as_str().into(),
                execution: match &clause.execution_point {
                    ir::ExecutionPoint::Handler { name } => w::Execution::Handler {
                        name: name.as_str().into(),
                    },
                    ir::ExecutionPoint::Initialization { name } => w::Execution::Initialization {
                        name: name.as_str().into(),
                    },
                    ir::ExecutionPoint::Pre { operation: anchor }
                        if anchor == &operation.anchor =>
                    {
                        w::Execution::Pre {
                            operation: operation.export.clone(),
                        }
                    }
                    ir::ExecutionPoint::Post { operation: anchor }
                        if anchor == &operation.anchor =>
                    {
                        w::Execution::Post {
                            operation: operation.export.clone(),
                        }
                    }
                    ir::ExecutionPoint::Pre { operation: anchor } => {
                        return Err(Error::PreAnchorMismatch {
                            name: clause.name.clone(),
                            expected: operation.anchor.clone(),
                            actual: anchor.clone(),
                        })
                    }
                    ir::ExecutionPoint::Post { operation: anchor } => {
                        return Err(Error::PostAnchorMismatch {
                            name: clause.name.clone(),
                            expected: operation.anchor.clone(),
                            actual: anchor.clone(),
                        })
                    }
                },
            })
        })
        .collect()
}

fn dependency_inputs(dependencies: &[Dependency]) -> Vec<artifact::SuppliedDependency<'_>> {
    dependencies
        .iter()
        .map(|dependency| artifact::SuppliedDependency {
            artifact: &dependency.artifact,
            bytes: &dependency.bytes,
            requires: &dependency.requires,
        })
        .collect()
}

struct Inputs {
    model: NativeModel,
    units: Vec<UnitInput>,
    operation: OperationSelection,
    definitions: DefinitionInputs,
}

/// Keep each exact source, formal correspondence and publication path together.
struct UnitInput {
    file: &'static str,
    artifact: w::ArtifactRef,
    mapping: CheckBindings,
}

struct OperationSelection {
    anchor: ir::AnchorName,
    export: w::ExportRef,
}

impl OperationSelection {
    fn new(model: &NativeModel) -> Result<Self, Error> {
        let [operation] = model.roles().operations.as_slice() else {
            return Err(Error::OperationCount {
                count: model.roles().operations.len(),
            });
        };
        if operation.context.as_str() != "Workflow" || operation.name.as_str() != "apply" {
            return Err(Error::OperationIdentity {
                context: operation.context.clone(),
                name: operation.name.clone(),
            });
        }
        // The wire orders exports by kind label then path. In this actual native
        // model, enums, fields and objects precede its single operation. Derive
        // its expected handle from admitted inputs, never from the emitted table.
        let preceding = model.environment().types().iter().try_fold(
            model.roles().objects.len(),
            |count, declaration| {
                count
                    .checked_add(match declaration {
                        ir::TypeDeclaration::Record { declaration } => declaration.fields().len(),
                        ir::TypeDeclaration::Enum { .. } => 1,
                    })
                    .ok_or(Error::InventoryLimit)
            },
        )?;
        Ok(Self {
            anchor: operation.anchor.clone(),
            export: w::ExportRef {
                model: 0,
                export: u32::try_from(preceding).map_err(|_| Error::InventoryLimit)?,
            },
        })
    }
}

impl UnitInput {
    fn new(
        recipe: &UnitRecipe,
        model: &NativeModel,
        operation: &OperationSelection,
    ) -> Result<Self, Error> {
        let source = formal(source(model, recipe)?, recipe.document)?;
        let owner = ir::RequirementRef::new(
            ir::PackageId::new("agent-ix/quire-spec-language").map_err(Error::Identifier)?,
            ir::RequirementId::new(recipe.requirement).map_err(Error::Identifier)?,
            ir::RequirementRevision::new(1).map_err(Error::Identifier)?,
        );
        let clauses = recipe
            .clauses
            .iter()
            .map(|clause| {
                Ok(ClauseBinding {
                    name: clause.name.into(),
                    requirement: owner.clone(),
                    clause: ir::ClauseId::new(clause.clause).map_err(Error::Identifier)?,
                    execution_point: match clause.execution {
                        AuthoredExecution::Handler => ir::ExecutionPoint::Handler {
                            name: ir::AnchorName::new("validate").map_err(Error::Identifier)?,
                        },
                        AuthoredExecution::Pre => ir::ExecutionPoint::Pre {
                            operation: operation.anchor.clone(),
                        },
                        AuthoredExecution::Post => ir::ExecutionPoint::Post {
                            operation: operation.anchor.clone(),
                        },
                    },
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            file: recipe.file,
            artifact: reference(
                AUTHORITY,
                w::ArtifactKind::Source,
                recipe.identity,
                revision(
                    "native-source-revision",
                    &source.source().identity().revision,
                ),
                "ix:native",
                "1-draft",
                source.source().text().as_bytes(),
            ),
            mapping: CheckBindings { source, clauses },
        })
    }
}

impl Inputs {
    fn new() -> Result<Self, Error> {
        Self::new_with(UNITS, &[R::EventPosition], include_bytes!("model.json"))
    }

    fn new_v2() -> Result<Self, Error> {
        Self::new_with(
            UNITS_V2,
            &[R::EventPosition, R::FixedSample, R::TimestampedWindow],
            include_bytes!("model-v2-frozen.json"),
        )
    }

    fn new_with(recipes: &[UnitRecipe], temporal: &[R], model_bytes: &[u8]) -> Result<Self, Error> {
        let model = model(model_bytes)?;
        let operation = OperationSelection::new(&model)?;
        let units = recipes
            .iter()
            .map(|recipe| UnitInput::new(recipe, &model, &operation))
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(Self {
            model,
            units,
            operation,
            definitions: DefinitionInputs::new(temporal),
        })
    }
}

struct DefinitionInputs {
    selected: Vec<R>,
    artifacts: Vec<definitions::Artifact<'static>>,
    rules: Vec<definitions::RuleInput<'static>>,
}

impl DefinitionInputs {
    fn new(temporal: &[R]) -> Self {
        let selected = selected_definitions(temporal);
        let definitions = selected
            .iter()
            .map(|definition| definitions::Artifact {
                selection: definition.selection(),
                bytes: definition.bytes(),
            })
            .collect::<Vec<_>>();
        let rules = selected
            .iter()
            .flat_map(|definition| definition.rules())
            .map(|rule| (rule.path, *rule))
            .collect::<BTreeMap<_, _>>();
        let rule_inputs = rules
            .values()
            .map(|rule| definitions::RuleInput {
                path: rule.path,
                digest: ByteDigest::of(rule.bytes),
                bytes: rule.bytes,
            })
            .collect::<Vec<_>>();

        Self {
            selected,
            artifacts: definitions,
            rules: rule_inputs,
        }
    }
}

struct SelectedInputs {
    contract: w::ArtifactRef,
    baseline: w::ArtifactRef,
    producer: w::Producer,
    language: w::Language,
    model: SelectedModel,
    dependencies: Vec<Dependency>,
}

impl SelectedInputs {
    fn new(inputs: &Inputs) -> Result<Self, Error> {
        let model = &inputs.model;
        let contract = reference(
            AUTHORITY,
            w::ArtifactKind::Source,
            "quire.compiled-protocol/1",
            revision("compiled-protocol-contract", "1"),
            "text/markdown",
            "1",
            CONTRACT,
        );
        let binary_ref = digest_reference(
            AUTHORITY,
            w::ArtifactKind::GeneratedArtifact,
            "native_protocol_handoff",
            revision("crate-version", env!("CARGO_PKG_VERSION")),
            "ELF",
            "1",
            producer_binary_digest()?,
        );
        let producer = w::Producer {
            implementation: "quire-spec-language/native_protocol_handoff".into(),
            revision: revision("crate-version", env!("CARGO_PKG_VERSION")),
            binary: binary_ref,
        };
        let model_ref = reference(
            AUTHORITY,
            w::ArtifactKind::ModelPackage,
            model.environment().owner().package().as_str(),
            revision(
                REQUIREMENT_NAMESPACE,
                &model.environment().owner().revision().get().to_string(),
            ),
            "native-state-model",
            "2",
            model.artifact_bytes(),
        );
        let foreign_ref = reference(
            AUTHORITY,
            w::ArtifactKind::Source,
            &model.source().source().identity().identity,
            revision(
                "native-source-revision",
                &model.source().source().identity().revision,
            ),
            "native-rule-model",
            "2",
            model.source().source().text().as_bytes(),
        );

        let dependencies = selected_dependencies(inputs, &contract, &model_ref)?;
        let baseline = dependency_reference(&dependencies, R::Edition.identity())?;
        Ok(Self {
            contract,
            baseline,
            producer,
            language: w::Language {
                identity: "ix:native".into(),
                edition: "1-draft".into(),
            },
            model: SelectedModel {
                artifact: model_ref,
                source: wire_source(foreign_ref, model.source()),
                source_file: "model-source.json".into(),
                source_format: model_source::FORMAT_V2.into(),
            },
            dependencies,
        })
    }

    fn new_v2(inputs: &Inputs) -> Result<Self, Error> {
        let mut selected = Self::new(inputs)?;
        let contract = reference(
            AUTHORITY,
            w::ArtifactKind::Source,
            "quire.compiled-protocol/2",
            revision("compiled-protocol-contract", "2"),
            "text/markdown",
            "1",
            CONTRACT_V2,
        );
        let contract_dependency = selected
            .dependencies
            .iter_mut()
            .find(|dependency| dependency.artifact == selected.contract)
            .ok_or_else(|| Error::Dependency {
                identity: selected.contract.identity.clone(),
                matches: 0,
            })?;
        contract_dependency.artifact = contract.clone();
        contract_dependency.bytes = Cow::Borrowed(CONTRACT_V2);
        selected.contract = contract;

        // The producer's binary is Producer identity, not a byte-sealed
        // dependency (see `producer_binary_digest`), so there is no
        // `dependencies` entry to find here -- only the reference itself.
        selected.producer.binary.identity = "native_protocol_v2_handoff".into();
        selected.producer.implementation = "quire-spec-language/native_protocol_v2_handoff".into();

        selected
            .dependencies
            .sort_by(|a, b| dependency_key(&a.artifact).cmp(&dependency_key(&b.artifact)));
        for (index, dependency) in selected.dependencies.iter_mut().enumerate() {
            dependency.file = format!("dependencies/{index}.bin");
        }
        Ok(selected)
    }

    fn output_selection(
        &self,
        artifact: w::ArtifactRef,
        sources: Vec<SelectedSource>,
    ) -> Selection {
        Selection {
            artifact,
            contract: self.contract.clone(),
            baseline: self.baseline.clone(),
            producer: self.producer.clone(),
            language: self.language.clone(),
            sources,
            dependencies: self
                .dependencies
                .iter()
                .map(|dependency| SelectedDependency {
                    artifact: dependency.artifact.clone(),
                    file: dependency.file.clone(),
                    requires: dependency.requires.clone(),
                })
                .collect(),
            model: self.model.clone(),
        }
    }
}

fn selected_dependencies(
    inputs: &Inputs,
    contract: &w::ArtifactRef,
    model_ref: &w::ArtifactRef,
) -> Result<Vec<Dependency>, Error> {
    let selected = &inputs.definitions.selected;
    let rules = &inputs.definitions.rules;
    let model = &inputs.model;
    let mut dependencies = selected
        .iter()
        .map(|definition| Dependency {
            artifact: reference(
                STANDARD,
                w::ArtifactKind::Source,
                definition.identity(),
                revision(DEFINITION_NAMESPACE, definition.revision()),
                "text/markdown",
                "1",
                definition.bytes(),
            ),
            bytes: Cow::Borrowed(definition.bytes()),
            requires: Vec::new(),
            file: String::new(),
        })
        .collect::<Vec<_>>();
    for rule in rules {
        dependencies.push(Dependency {
            artifact: reference(
                STANDARD,
                w::ArtifactKind::Source,
                rule.path,
                revision("selected-rule-source", "1"),
                "text/markdown",
                "1",
                rule.bytes,
            ),
            bytes: Cow::Borrowed(rule.bytes),
            requires: Vec::new(),
            file: String::new(),
        });
    }
    // The producer's own binary is not one of these exact-byte dependencies
    // (see `producer_binary_digest`): its identity is Producer.binary, an
    // artifact reference recording only a digest.
    dependencies.extend([
        Dependency {
            artifact: contract.clone(),
            bytes: Cow::Borrowed(CONTRACT),
            requires: Vec::new(),
            file: String::new(),
        },
        Dependency {
            artifact: model_ref.clone(),
            bytes: Cow::Owned(model.artifact_bytes().to_vec()),
            requires: Vec::new(),
            file: String::new(),
        },
    ]);
    // The registry owns definition/rule prerequisites. This fixture supplies
    // those exact direct edges instead of discovering dependencies from output.
    for definition in selected {
        let mut requires = Vec::new();
        for required in definition.requirements() {
            requires.push(dependency_reference(&dependencies, required.identity())?);
        }
        for rule in definition.rules() {
            requires.push(dependency_reference(&dependencies, rule.path)?);
        }
        let dependency = dependencies
            .iter_mut()
            .find(|dependency| dependency.artifact.identity == definition.identity())
            .ok_or_else(|| Error::Dependency {
                identity: definition.identity().into(),
                matches: 0,
            })?;
        dependency.requires = requires;
    }
    dependencies.sort_by(|a, b| dependency_key(&a.artifact).cmp(&dependency_key(&b.artifact)));

    // Assign the output path once; sidecar and publication use this same value.
    for (index, dependency) in dependencies.iter_mut().enumerate() {
        dependency.file = format!("dependencies/{index}.bin");
    }
    Ok(dependencies)
}

fn wire_source(artifact: w::ArtifactRef, source: &FormalSource) -> w::Source {
    w::Source {
        artifact,
        native: w::NativeSource {
            identity: source.source().identity().identity.clone(),
            revision: source.source().identity().revision.clone(),
        },
        path: source.source().path().into(),
        formal: w::Formal {
            document: source.identity().document().as_str().into(),
            revision: revision(
                FORMAL_NAMESPACE,
                &source.identity().revision().get().to_string(),
            ),
        },
        text: source.source().text().into(),
    }
}

struct Output {
    selection: Selection,
    emitted: native::EmittedPackage,
}

struct OutputV2 {
    selection: SelectionV2,
    emitted: native::AdmissionV2,
    mutation_manifest: MutationManifest,
    mutation_files: MutationFiles,
}

fn compile_with<T>(
    inputs: &Inputs,
    selected: &SelectedInputs,
    finish: impl FnOnce(
        &Inputs,
        &SelectedInputs,
        &proofs::ProofReport<'_, '_, '_>,
        Vec<SelectedSource>,
    ) -> Result<T, Error>,
) -> Result<T, Error> {
    // These short-lived API views preserve the same authored unit order; the
    // owning records above keep source, correspondence and identity together.
    let sources = inputs
        .units
        .iter()
        .map(|unit| unit.mapping.source.source().clone())
        .collect::<Vec<_>>();
    let formal_sources = inputs
        .units
        .iter()
        .map(|unit| unit.mapping.source.clone())
        .collect::<Vec<_>>();
    let mappings = inputs
        .units
        .iter()
        .map(|unit| unit.mapping.clone())
        .collect::<Vec<_>>();
    let inventory = linking::SourceInventory {
        language: "ix:native".into(),
        edition: "1-draft".into(),
        units: inputs
            .units
            .iter()
            .map(|unit| linking::ExpectedSource {
                authority: unit.artifact.identity.clone(),
                identity: unit.mapping.source.source().identity().clone(),
                digest: unit.mapping.source.source().digest(),
            })
            .collect(),
    };
    let namespace = linking::admit_namespace(
        &inventory,
        &sources,
        linking::WorkLimits::default(),
        quire_spec_language::Limits::default(),
    );
    let namespace = namespace
        .namespace()
        .ok_or_else(|| namespace_failure(&namespace))?;
    let definition_inputs = definitions::Inventory {
        edition: R::Edition.selection(),
        definitions: &inputs.definitions.artifacts,
        rules: &inputs.definitions.rules,
    };
    let model_inputs = [ModelInput::Native(&inputs.model)];
    let binding = binding::bind(
        namespace,
        &definition_inputs,
        &model_inputs,
        linking::binding_work::Limits::default(),
    );
    let typed = composed::admit_types(&binding, &formal_sources, composed::TypeLimits::default());
    let proofs = proofs::discharge(&typed, &mappings, proofs::ProofLimits::default());
    let expected = inputs.units.iter().try_fold(0usize, |count, unit| {
        count
            .checked_add(unit.mapping.clauses.len())
            .ok_or(Error::InventoryLimit)
    })?;
    require_proofs(&proofs, expected)?;
    let original = inputs
        .units
        .iter()
        .map(|unit| {
            Ok(SelectedSource {
                file: unit.file.into(),
                source: wire_source(unit.artifact.clone(), &unit.mapping.source),
                declarations: declaration_selection(namespace, unit, &inputs.operation)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    finish(inputs, selected, &proofs, original)
}

fn compile(inputs: &Inputs, selected: &SelectedInputs) -> Result<Output, Error> {
    compile_with(inputs, selected, emit_and_read)
}

fn compile_v2(inputs: &Inputs, selected: &SelectedInputs) -> Result<OutputV2, Error> {
    compile_with(inputs, selected, emit_and_read_v2)
}

fn namespace_failure(report: &linking::NamespaceReport<'_>) -> Error {
    Error::Stage {
        stage: "namespace",
        completed: report.parsed_sources().len(),
        expected: report.inventory().units.len(),
        issues: report.issues().len(),
        incomplete: report.is_incomplete(),
        first: report.issues().first().map(namespace_issue).map(Box::new),
    }
}

fn namespace_issue(issue: &linking::InventoryIssue) -> StageIssue {
    use linking::InventoryIssue as I;
    let (kind, supplied, span, code) = match issue {
        I::EmptyInventory => ("empty inventory", None, None, None),
        I::UnsupportedSelection => ("unsupported selection", None, None, None),
        I::InvalidExpectedSource { .. } => ("invalid expected source", None, None, None),
        I::DuplicateExpectedSource { .. } => ("duplicate expected source", None, None, None),
        I::DuplicateSuppliedSource { supplied } => (
            "duplicate supplied source",
            supplied.first().copied(),
            None,
            None,
        ),
        I::MissingSource { .. } => ("missing source", None, None, None),
        I::UnexpectedSource { supplied } => ("unexpected source", Some(*supplied), None, None),
        I::DigestMismatch { supplied, .. } => {
            ("source digest mismatch", Some(*supplied), None, None)
        }
        I::HeaderConflict {
            supplied,
            language,
            edition,
        } => (
            "source header conflict",
            Some(*supplied),
            Some(quire_spec_language::Span {
                start: language.span.start,
                end: edition.span.end,
            }),
            None,
        ),
        I::ParseFailure {
            supplied,
            diagnostic,
        } => (
            "source parse failure",
            Some(*supplied),
            Some(quire_spec_language::Span {
                start: diagnostic.span.start.byte,
                end: diagnostic.span.end.byte,
            }),
            Some(diagnostic.code),
        ),
    };
    StageIssue::Namespace {
        kind,
        supplied,
        span,
        code,
    }
}

fn require_proofs(report: &proofs::ProofReport<'_, '_, '_>, expected: usize) -> Result<(), Error> {
    let completed = report
        .declarations()
        .iter()
        .filter(|declaration| declaration.disposition() == proofs::ProofDisposition::Discharged)
        .count();
    if completed == expected
        && report.declarations().len() == expected
        && report.exhaustion().is_none()
    {
        return Ok(());
    }
    let first = report
        .declarations()
        .iter()
        .find(|declaration| declaration.disposition() != proofs::ProofDisposition::Discharged)
        .map(|declaration| {
            let cause = declaration.causes().first();
            Box::new(StageIssue::Proof {
                declaration: declaration.declaration().index(),
                disposition: declaration.disposition(),
                site: cause
                    .map(|cause| cause.site)
                    .or_else(|| report.exhaustion().map(|exhaustion| exhaustion.site)),
                cause: cause.map(|cause| proof_cause(&cause.kind)),
            })
        });
    Err(Error::Stage {
        stage: "type/definedness",
        completed,
        expected,
        issues: report
            .declarations()
            .iter()
            .map(|declaration| declaration.causes().len())
            .sum(),
        incomplete: report.exhaustion().is_some()
            || report.declarations().iter().any(|declaration| {
                declaration.disposition() == proofs::ProofDisposition::Unfinished
            }),
        first,
    })
}

fn proof_cause(cause: &proofs::CauseKind) -> ProofCause {
    match cause {
        proofs::CauseKind::UpstreamType => ProofCause::UpstreamType,
        proofs::CauseKind::Correspondence(cause) => ProofCause::Correspondence(*cause),
        proofs::CauseKind::Unproved { diagnostics } => ProofCause::Unproved {
            diagnostics: diagnostics.len(),
        },
        proofs::CauseKind::Unsupported(cause) => ProofCause::Unsupported(*cause),
        proofs::CauseKind::Dependency { target } => ProofCause::Dependency {
            declaration: target.index(),
        },
        _ => ProofCause::Unrecognized,
    }
}

fn emit_and_read(
    inputs: &Inputs,
    selected: &SelectedInputs,
    proofs: &proofs::ProofReport<'_, '_, '_>,
    selected_sources: Vec<SelectedSource>,
) -> Result<Output, Error> {
    let supplied = dependency_inputs(&selected.dependencies);
    let foreign = artifact::ExpectedForeignSource {
        artifact: &selected.model.source.artifact,
        formal: &selected.model.source.formal,
        bytes: inputs.model.source().source().text().as_bytes(),
    };
    let models = [artifact::AdmittedModel {
        artifact: &selected.model.artifact,
        model: &inputs.model,
        source: &foreign,
    }];
    let native_sources = inputs
        .units
        .iter()
        .map(|unit| native::SourceSelection {
            artifact: &unit.artifact,
            source: &unit.mapping.source,
            revision_namespace: FORMAL_NAMESPACE,
        })
        .collect::<Vec<_>>();
    let selections = native::Selections {
        contract: &selected.contract,
        baseline: &selected.baseline,
        producer: &selected.producer,
        sources: &native_sources,
        dependencies: &supplied,
        models: &models,
        definition_revision_namespace: DEFINITION_NAMESPACE,
        requirement_revision_namespace: REQUIREMENT_NAMESPACE,
    };
    let admitted = artifact_result(
        "native family admission",
        native::admit(proofs, &selections, artifact::Limits::default()),
    )?;
    let emitted = artifact_result(
        "native encoding",
        native::emit(&admitted, artifact::Limits::default()),
    )?;
    // The producing side selects the exact output seal once emission finishes;
    // a later consumer must accept it independently of an offered payload.
    let output = reference(
        AUTHORITY,
        w::ArtifactKind::LinkedPackage,
        "examples/protocol-handoff/workflow",
        revision("example-output", "1"),
        "quire.compiled-protocol",
        "1",
        emitted.bytes(),
    );
    let expected_units = selected_sources
        .iter()
        .map(ExpectedUnit::new)
        .collect::<Vec<_>>();
    let expected_sources = expected_units
        .iter()
        .map(ExpectedUnit::source)
        .collect::<Vec<_>>();
    let read = artifact_result(
        "independent selection reader",
        artifact::read(
            emitted.bytes(),
            &artifact::Expected {
                artifact: &output,
                contract: &selected.contract,
                baseline: &selected.baseline,
                producer: &selected.producer,
                language: &selected.language,
                sources: &expected_sources,
                dependencies: &supplied,
                models: &models,
            },
            artifact::Limits::default(),
        ),
    )?;
    if read.package() != admitted.package() {
        return Err(Error::RoundTrip);
    }

    Ok(Output {
        selection: selected.output_selection(output, selected_sources),
        emitted,
    })
}

fn emit_and_read_v2(
    inputs: &Inputs,
    selected: &SelectedInputs,
    proofs: &proofs::ProofReport<'_, '_, '_>,
    selected_sources: Vec<SelectedSource>,
) -> Result<OutputV2, Error> {
    let supplied = dependency_inputs(&selected.dependencies);
    let foreign = artifact::ExpectedForeignSource {
        artifact: &selected.model.source.artifact,
        formal: &selected.model.source.formal,
        bytes: inputs.model.source().source().text().as_bytes(),
    };
    let models = [artifact::AdmittedModel {
        artifact: &selected.model.artifact,
        model: &inputs.model,
        source: &foreign,
    }];
    let native_sources = inputs
        .units
        .iter()
        .map(|unit| native::SourceSelection {
            artifact: &unit.artifact,
            source: &unit.mapping.source,
            revision_namespace: FORMAL_NAMESPACE,
        })
        .collect::<Vec<_>>();
    let selections = native::Selections {
        contract: &selected.contract,
        baseline: &selected.baseline,
        producer: &selected.producer,
        sources: &native_sources,
        dependencies: &supplied,
        models: &models,
        definition_revision_namespace: DEFINITION_NAMESPACE,
        requirement_revision_namespace: REQUIREMENT_NAMESPACE,
    };

    let mut positions = Vec::new();
    positions
        .try_reserve_exact(TEMPORAL_V2.len())
        .map_err(|_| Error::InventoryLimit)?;
    for recipe in TEMPORAL_V2 {
        let matches = selected_sources
            .iter()
            .enumerate()
            .flat_map(|(source, selected)| {
                selected
                    .declarations
                    .iter()
                    .enumerate()
                    .filter(|(_, declaration)| declaration.name == recipe.name)
                    .map(move |(declaration, _)| (source, declaration))
            })
            .collect::<Vec<_>>();
        let [position] = matches.as_slice() else {
            return Err(Error::Declaration {
                name: recipe.name.into(),
                cause: if matches.is_empty() {
                    DeclarationCause::Missing
                } else {
                    DeclarationCause::Ambiguous {
                        matches: matches.len(),
                    }
                },
            });
        };
        positions.push(*position);
    }
    let definition_artifacts = TEMPORAL_V2
        .iter()
        .map(|recipe| dependency_reference(&selected.dependencies, recipe.definition.identity()))
        .collect::<Result<Vec<_>, Error>>()?;
    let definition_revisions = TEMPORAL_V2
        .iter()
        .map(|recipe| revision(DEFINITION_NAMESPACE, recipe.definition.revision()))
        .collect::<Vec<_>>();
    let clocks = TEMPORAL_V2
        .iter()
        .map(|recipe| serde_json::from_slice(recipe.clock_bytes))
        .collect::<Result<Vec<v2::wire::ClockConfiguration>, _>>()?;
    for (recipe, clock) in TEMPORAL_V2.iter().zip(&clocks) {
        if serde_json::to_vec(clock)? == recipe.clock_bytes {
            return Err(Error::MutationFixture("clock-input-recanonicalization"));
        }
    }
    let temporal = TEMPORAL_V2
        .iter()
        .enumerate()
        .map(|(index, recipe)| {
            let (source, declaration) = positions[index];
            native::TemporalSelection {
                source: &selected_sources[source].source.artifact,
                span: &selected_sources[source].declarations[declaration].span,
                definition_identity: recipe.definition.identity(),
                definition_revision: &definition_revisions[index],
                definition_artifact: &definition_artifacts[index],
                clock: &clocks[index],
            }
        })
        .collect::<Vec<_>>();
    let limits = artifact::Limits::default();
    let emitted = artifact_result(
        "native v2 admission and encoding",
        native::admit_v2(proofs, &selections, &temporal, limits),
    )?;
    let output = reference(
        AUTHORITY,
        w::ArtifactKind::LinkedPackage,
        "examples/protocol-handoff/workflow-v2",
        revision("example-output", "2"),
        "quire.compiled-protocol",
        "2",
        emitted.bytes(),
    );

    let expected_units = selected_sources
        .iter()
        .map(ExpectedUnit::new)
        .collect::<Vec<_>>();
    let expected_sources = expected_units
        .iter()
        .map(ExpectedUnit::source)
        .collect::<Vec<_>>();
    let expected_temporal = TEMPORAL_V2
        .iter()
        .enumerate()
        .map(|(index, recipe)| {
            let (source, declaration) = positions[index];
            v2::ExpectedTemporal {
                source: expected_sources[source].artifact,
                declaration: &expected_units[source].declarations[declaration],
                definition: v2::ExpectedDefinition {
                    identity: recipe.definition.identity(),
                    revision: &definition_revisions[index],
                    artifact: &definition_artifacts[index],
                },
                clock: &clocks[index],
            }
        })
        .collect::<Vec<_>>();
    let read = artifact_result(
        "independent v2 selection reader",
        v2::read(
            emitted.bytes(),
            &v2::Expected {
                inherited: artifact::Expected {
                    artifact: &output,
                    contract: &selected.contract,
                    baseline: &selected.baseline,
                    producer: &selected.producer,
                    language: &selected.language,
                    sources: &expected_sources,
                    dependencies: &supplied,
                    models: &models,
                },
                temporal: &expected_temporal,
            },
            limits,
        ),
    )?;
    if read.package() != emitted.admitted().package() {
        return Err(Error::RoundTrip);
    }

    let temporal = TEMPORAL_V2
        .iter()
        .enumerate()
        .map(|(index, recipe)| {
            let (source, declaration) = positions[index];
            SelectedTemporal {
                source: selected_sources[source].source.artifact.clone(),
                declaration: selected_sources[source].declarations[declaration].clone(),
                definition_identity: recipe.definition.identity().into(),
                definition_revision: definition_revisions[index].clone(),
                definition_artifact: definition_artifacts[index].clone(),
                clock_input: SelectedClockInput {
                    identity: format!(
                        "{AUTHORITY}/examples/protocol-handoff/{}",
                        recipe.clock_identity
                    ),
                    digest: ByteDigest::of(recipe.clock_bytes).to_string(),
                    file: recipe.clock_file.into(),
                    configuration: clocks[index].clone(),
                },
            }
        })
        .collect();
    let selection = SelectionV2 {
        inherited: selected.output_selection(output.clone(), selected_sources.clone()),
        temporal,
        limits: SelectedArtifactLimits::try_new(limits)?,
    };
    let package = emitted.admitted().package().clone();
    let (mutation_manifest, mutation_files) =
        mutation_fixtures(&package, &selection, &selected.dependencies)?;
    verify_mutation_fixtures(
        &mutation_manifest,
        &mutation_files,
        emitted.bytes(),
        &output,
        artifact::Expected {
            artifact: &output,
            contract: &selected.contract,
            baseline: &selected.baseline,
            producer: &selected.producer,
            language: &selected.language,
            sources: &expected_sources,
            dependencies: &supplied,
            models: &models,
        },
        &expected_temporal,
        &selected.dependencies,
        limits,
    )?;
    Ok(OutputV2 {
        selection,
        emitted,
        mutation_manifest,
        mutation_files,
    })
}

/// Borrowed public-reader inputs, still selected from the authored recipe.
struct ExpectedUnit<'a> {
    original: &'a w::Source,
    declarations: Vec<artifact::ExpectedDeclaration<'a>>,
}

impl<'a> ExpectedUnit<'a> {
    fn new(selected: &'a SelectedSource) -> Self {
        Self {
            original: &selected.source,
            declarations: selected
                .declarations
                .iter()
                .map(|declaration| artifact::ExpectedDeclaration {
                    name: &declaration.name,
                    span: &declaration.span,
                    requirement: &declaration.requirement,
                    clause: &declaration.clause,
                    execution: &declaration.execution,
                })
                .collect(),
        }
    }

    fn source(&self) -> artifact::ExpectedSource<'_> {
        artifact::ExpectedSource {
            artifact: &self.original.artifact,
            native: &self.original.native,
            path: &self.original.path,
            formal: &self.original.formal,
            text: &self.original.text,
            declarations: &self.declarations,
        }
    }
}

/// Compile the authored recipe and publish its exact selections in a fresh directory.
pub fn write(directory: &Path) -> Result<(), Error> {
    let inputs = Inputs::new()?;
    let selected = SelectedInputs::new(&inputs)?;
    let output = compile(&inputs, &selected)?;
    write_files(
        directory,
        &output.selection,
        &selected.dependencies,
        output.emitted.bytes(),
    )
}

/// Compile the authored temporal recipe as strict v2 and publish its complete handoff.
pub fn write_v2(directory: &Path) -> Result<(), Error> {
    let inputs = Inputs::new_v2()?;
    let selected = SelectedInputs::new_v2(&inputs)?;
    let output = compile_v2(&inputs, &selected)?;
    write_files_v2(
        directory,
        &output.selection,
        &selected.dependencies,
        output.emitted.bytes(),
        &output.mutation_manifest,
        &output.mutation_files,
    )
}

fn dependency_reference(
    dependencies: &[Dependency],
    identity: &str,
) -> Result<w::ArtifactRef, Error> {
    let mut found = dependencies
        .iter()
        .filter(|dependency| dependency.artifact.identity == identity);
    match (found.next(), found.next()) {
        (Some(dependency), None) => Ok(dependency.artifact.clone()),
        (None, _) => Err(Error::Dependency {
            identity: identity.into(),
            matches: 0,
        }),
        (Some(_), Some(_)) => Err(Error::Dependency {
            identity: identity.into(),
            matches: 2 + found.count(),
        }),
    }
}

fn artifact_result<T>(stage: &'static str, report: artifact::Report<T>) -> Result<T, Error> {
    let locus = report.locus().cloned().map(Box::new);
    report.into_result().map_err(|cause| Error::Artifact {
        stage,
        cause,
        locus,
    })
}

fn dependency_key(reference: &w::ArtifactRef) -> (&str, &str, &str, &str, &str, &str, &str) {
    (
        reference.kind.as_str(),
        &reference.authority,
        &reference.identity,
        &reference.revision.namespace,
        &reference.revision.value,
        &reference.wire.identity,
        &reference.wire.version,
    )
}

fn write_files(
    directory: &Path,
    selection: &Selection,
    dependencies: &[Dependency],
    bytes: &[u8],
) -> Result<(), Error> {
    let selected_bytes = serde_json::to_vec_pretty(selection)?;
    let reference_bytes = serde_json::to_vec_pretty(&selection.artifact)?;
    // Create a fresh directory only after every compiler/reader stage succeeds.
    fs::create_dir(directory).map_err(|error| io_at(directory, error))?;
    let dependency_directory = directory.join("dependencies");
    fs::create_dir(&dependency_directory).map_err(|error| io_at(&dependency_directory, error))?;
    for dependency in dependencies {
        write_file(&directory.join(&dependency.file), &dependency.bytes)?;
    }
    for selected in &selection.sources {
        write_file(
            &directory.join(&selected.file),
            selected.source.text.as_bytes(),
        )?;
    }
    write_file(
        &directory.join(&selection.model.source_file),
        selection.model.source.text.as_bytes(),
    )?;
    write_file(&directory.join("expected.json"), &selected_bytes)?;
    write_file(
        &directory.join("compiled-protocol.ref.json"),
        &reference_bytes,
    )?;
    write_file(&directory.join("compiled-protocol.json"), bytes)?;
    write_checksum_inventory(directory)
}

fn collect_handoff_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeSet<PathBuf>,
) -> Result<(), Error> {
    for entry in fs::read_dir(directory).map_err(|error| io_at(directory, error))? {
        let entry = entry.map_err(|error| io_at(directory, error))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| io_at(&path, error))?;
        if file_type.is_dir() {
            collect_handoff_files(root, &path, files)?;
        } else if file_type.is_file() {
            files.insert(
                path.strip_prefix(root)
                    .map_err(|error| Error::HandoffPath(error.to_string()))?
                    .to_owned(),
            );
        } else {
            return Err(Error::HandoffPath(path.display().to_string()));
        }
    }
    Ok(())
}

fn write_checksum_inventory(directory: &Path) -> Result<(), Error> {
    let mut files = BTreeSet::new();
    collect_handoff_files(directory, directory, &mut files)?;
    let mut sums = String::new();
    for relative in files {
        let path = directory.join(&relative);
        let bytes = fs::read(&path).map_err(|error| io_at(&path, error))?;
        use std::fmt::Write as _;
        writeln!(
            sums,
            "{:x}  ./{}",
            ByteDigest::of(&bytes),
            relative.display()
        )
        .map_err(|error| Error::HandoffPath(error.to_string()))?;
    }
    write_file(&directory.join("SHA256SUMS"), sums.as_bytes())
}

fn write_files_v2(
    directory: &Path,
    selection: &SelectionV2,
    dependencies: &[Dependency],
    bytes: &[u8],
    mutation_manifest: &MutationManifest,
    mutation_files: &MutationFiles,
) -> Result<(), Error> {
    let selected_bytes = serde_json::to_vec_pretty(selection)?;
    let reference_bytes = serde_json::to_vec_pretty(&selection.inherited.artifact)?;
    let mutation_manifest = serde_json::to_vec_pretty(&mutation_manifest)?;
    fs::create_dir(directory).map_err(|error| io_at(directory, error))?;
    let dependency_directory = directory.join("dependencies");
    fs::create_dir(&dependency_directory).map_err(|error| io_at(&dependency_directory, error))?;
    let mutation_directory = directory.join("mutations");
    fs::create_dir(&mutation_directory).map_err(|error| io_at(&mutation_directory, error))?;
    for dependency in dependencies {
        write_file(&directory.join(&dependency.file), &dependency.bytes)?;
    }
    for selected in &selection.inherited.sources {
        write_file(
            &directory.join(&selected.file),
            selected.source.text.as_bytes(),
        )?;
    }
    write_file(
        &directory.join(&selection.inherited.model.source_file),
        selection.inherited.model.source.text.as_bytes(),
    )?;
    for recipe in TEMPORAL_V2 {
        write_file(&directory.join(recipe.clock_file), recipe.clock_bytes)?;
    }
    for (file, content) in mutation_files {
        write_file(&directory.join(file), content)?;
    }
    write_file(
        &directory.join(PUBLISHED_MUTATION_MANIFEST_FILE),
        &mutation_manifest,
    )?;
    write_file(&directory.join(PUBLISHED_SELECTION_FILE), &selected_bytes)?;
    write_file(
        &directory.join(PUBLISHED_ARTIFACT_REFERENCE_FILE),
        &reference_bytes,
    )?;
    write_file(&directory.join(PUBLISHED_OFFER_FILE), bytes)?;
    write_checksum_inventory(directory)
}

fn mutation_fixtures(
    package: &v2::wire::Package,
    selection: &SelectionV2,
    dependencies: &[Dependency],
) -> Result<(MutationManifest, MutationFiles), Error> {
    let mut cases = Vec::new();
    let mut files = Vec::new();
    let mut offer = |identity: &'static str,
                     axis: &'static str,
                     expected_refusal_code: &'static str,
                     package: v2::wire::Package|
     -> Result<(), Error> {
        let bytes = serde_json::to_vec(&package)?;
        let file = format!("mutations/{identity}.json");
        let artifact = reference(
            AUTHORITY,
            w::ArtifactKind::LinkedPackage,
            "examples/protocol-handoff/workflow-v2",
            revision("example-output", "2"),
            "quire.compiled-protocol",
            "2",
            &bytes,
        );
        files.push((file.clone(), bytes));
        cases.push(MutationCase {
            identity: identity.into(),
            axis: axis.into(),
            input: MutationInput::Offer { file, artifact },
            expected_refusal_code: expected_refusal_code.into(),
        });
        Ok(())
    };

    let mut header = package.clone();
    header.inherited.wire = "invalid-version-substitution".into();
    offer(
        "header-wire-version-substitution",
        "header.version",
        v2::Refusal::Header(v2::HeaderField::Wire).code(),
        header,
    )?;

    let mut missing = package.clone();
    missing.temporal_bindings.pop();
    offer(
        "binding-missing",
        "temporal_bindings.inventory",
        v2::Refusal::Binding {
            side: v2::InventorySide::Offer,
            cause: v2::BindingCause::Missing,
        }
        .code(),
        missing,
    )?;

    let mut surplus = package.clone();
    let first = surplus
        .temporal_bindings
        .first()
        .cloned()
        .ok_or(Error::MutationFixture("binding-surplus"))?;
    surplus.temporal_bindings.push(first);
    offer(
        "binding-surplus",
        "temporal_bindings.inventory",
        v2::Refusal::Binding {
            side: v2::InventorySide::Offer,
            cause: v2::BindingCause::Surplus,
        }
        .code(),
        surplus,
    )?;

    let mut duplicate = package.clone();
    let duplicate_value = duplicate
        .temporal_bindings
        .get(1)
        .cloned()
        .ok_or(Error::MutationFixture("binding-duplicate"))?;
    *duplicate
        .temporal_bindings
        .get_mut(2)
        .ok_or(Error::MutationFixture("binding-duplicate"))? = duplicate_value;
    offer(
        "binding-duplicate",
        "temporal_bindings.uniqueness",
        v2::Refusal::Binding {
            side: v2::InventorySide::Offer,
            cause: v2::BindingCause::Duplicate,
        }
        .code(),
        duplicate,
    )?;

    let mut reordered = package.clone();
    if reordered.temporal_bindings.len() < 2 {
        return Err(Error::MutationFixture("binding-reordered"));
    }
    reordered.temporal_bindings.swap(0, 1);
    offer(
        "binding-reordered",
        "temporal_bindings.order",
        v2::Refusal::OfferOrder.code(),
        reordered,
    )?;

    let mut declaration_index = package.clone();
    declaration_index
        .temporal_bindings
        .last_mut()
        .ok_or(Error::MutationFixture("declaration-index"))?
        .declaration = 9_999;
    offer(
        "declaration-index",
        "temporal_binding.declaration",
        v2::Refusal::OfferIndex(v2::BindingIndex::Declaration).code(),
        declaration_index,
    )?;

    let mut definition_index = package.clone();
    definition_index
        .temporal_bindings
        .first_mut()
        .ok_or(Error::MutationFixture("definition-index"))?
        .definition = 9_999;
    offer(
        "definition-index",
        "temporal_binding.definition",
        v2::Refusal::OfferIndex(v2::BindingIndex::Definition).code(),
        definition_index,
    )?;

    let declaration_at = usize::try_from(
        package
            .temporal_bindings
            .first()
            .ok_or(Error::MutationFixture("declaration-selection"))?
            .declaration,
    )
    .map_err(|_| Error::MutationFixture("declaration-index-conversion"))?;
    let mut declaration_name = package.clone();
    declaration_name
        .inherited
        .declarations
        .get_mut(declaration_at)
        .ok_or(Error::MutationFixture("declaration-name"))?
        .name
        .push_str("Other");
    offer(
        "declaration-name",
        "declaration.name",
        v2::Refusal::Declaration(v2::DeclarationField::Name).code(),
        declaration_name,
    )?;
    let mut declaration_requirement = package.clone();
    declaration_requirement
        .inherited
        .declarations
        .get_mut(declaration_at)
        .ok_or(Error::MutationFixture("declaration-requirement"))?
        .requirement
        .identity
        .push_str("Other");
    offer(
        "declaration-requirement",
        "declaration.requirement",
        v2::Refusal::Declaration(v2::DeclarationField::Requirement).code(),
        declaration_requirement,
    )?;
    let mut declaration_clause = package.clone();
    declaration_clause
        .inherited
        .declarations
        .get_mut(declaration_at)
        .ok_or(Error::MutationFixture("declaration-clause"))?
        .clause
        .push_str("-other");
    offer(
        "declaration-clause",
        "declaration.clause",
        v2::Refusal::Declaration(v2::DeclarationField::Clause).code(),
        declaration_clause,
    )?;
    let mut declaration_execution = package.clone();
    declaration_execution
        .inherited
        .declarations
        .get_mut(declaration_at)
        .ok_or(Error::MutationFixture("declaration-execution"))?
        .execution = w::Execution::Handler {
        name: "other".into(),
    };
    offer(
        "declaration-execution",
        "declaration.execution",
        v2::Refusal::Declaration(v2::DeclarationField::Execution).code(),
        declaration_execution,
    )?;

    let binding = package
        .temporal_bindings
        .first()
        .ok_or(Error::MutationFixture("definition-selection"))?;
    let definition_at = usize::try_from(binding.definition)
        .map_err(|_| Error::MutationFixture("definition-index-conversion"))?;
    let definition = package
        .inherited
        .definitions
        .get(definition_at)
        .ok_or(Error::MutationFixture("definition-selection"))?;
    let dependency_at = usize::try_from(definition.artifact)
        .map_err(|_| Error::MutationFixture("dependency-index-conversion"))?;

    let mut identity = package.clone();
    identity.inherited.definitions[definition_at]
        .identity
        .push_str("-other");
    offer(
        "definition-identity",
        "definition.identity",
        v2::Refusal::Definition(v2::DefinitionField::Identity).code(),
        identity,
    )?;

    let mut definition_revision = package.clone();
    definition_revision.inherited.definitions[definition_at]
        .revision
        .value
        .push_str("-other");
    offer(
        "definition-revision",
        "definition.revision",
        v2::Refusal::Definition(v2::DefinitionField::Revision).code(),
        definition_revision,
    )?;

    let mut artifact_digest = package.clone();
    artifact_digest.inherited.dependencies[dependency_at]
        .artifact
        .digest = selection.inherited.artifact.digest;
    offer(
        "definition-artifact-digest",
        "definition.artifact.digest",
        v2::Refusal::Definition(v2::DefinitionField::Artifact(v2::ArtifactField::Digest)).code(),
        artifact_digest,
    )?;

    let mut digest_domain = package.clone();
    digest_domain.inherited.dependencies[dependency_at]
        .artifact
        .wire
        .identity = "filament-canonical-json-1".into();
    offer(
        "definition-artifact-digest-domain",
        "definition.artifact.wire_identity",
        v2::Refusal::Definition(v2::DefinitionField::Artifact(
            v2::ArtifactField::WireIdentity,
        ))
        .code(),
        digest_domain,
    )?;

    let mut clock_alternative = package.clone();
    clock_alternative
        .temporal_bindings
        .first_mut()
        .ok_or(Error::MutationFixture("clock-alternative"))?
        .clock = v2::wire::ClockConfiguration::TimestampedEvent {
        timestamp_unit: "millisecond".into(),
    };
    offer(
        "clock-alternative",
        "clock.kind",
        v2::Refusal::Clock(v2::ClockField::Alternative).code(),
        clock_alternative,
    )?;

    let mut clock_field = package.clone();
    let v2::wire::ClockConfiguration::EventPosition { sequence_authority } = &mut clock_field
        .temporal_bindings
        .first_mut()
        .ok_or(Error::MutationFixture("clock-sequence-authority"))?
        .clock
    else {
        return Err(Error::MutationFixture("clock-sequence-authority"));
    };
    *sequence_authority = "other-workflow-events".into();
    offer(
        "clock-sequence-authority",
        "clock.sequence_authority",
        v2::Refusal::Clock(v2::ClockField::SequenceAuthority).code(),
        clock_field,
    )?;

    let mut fixed_epoch = package.clone();
    let v2::wire::ClockConfiguration::FixedSample { epoch, .. } = &mut fixed_epoch
        .temporal_bindings
        .get_mut(1)
        .ok_or(Error::MutationFixture("clock-epoch"))?
        .clock
    else {
        return Err(Error::MutationFixture("clock-epoch"));
    };
    *epoch = w::Number(artifact::NumberWire::Integer {
        decimal: "1".into(),
    });
    offer(
        "clock-epoch",
        "clock.epoch",
        v2::Refusal::Clock(v2::ClockField::Epoch).code(),
        fixed_epoch,
    )?;
    let mut fixed_period = package.clone();
    let v2::wire::ClockConfiguration::FixedSample { period, .. } = &mut fixed_period
        .temporal_bindings
        .get_mut(1)
        .ok_or(Error::MutationFixture("clock-period"))?
        .clock
    else {
        return Err(Error::MutationFixture("clock-period"));
    };
    *period = w::Number(artifact::NumberWire::Integer {
        decimal: "1".into(),
    });
    offer(
        "clock-period",
        "clock.period",
        v2::Refusal::Clock(v2::ClockField::Period).code(),
        fixed_period,
    )?;
    let mut fixed_unit = package.clone();
    let v2::wire::ClockConfiguration::FixedSample { unit, .. } = &mut fixed_unit
        .temporal_bindings
        .get_mut(1)
        .ok_or(Error::MutationFixture("clock-unit"))?
        .clock
    else {
        return Err(Error::MutationFixture("clock-unit"));
    };
    *unit = "minute".into();
    offer(
        "clock-unit",
        "clock.unit",
        v2::Refusal::Clock(v2::ClockField::Unit).code(),
        fixed_unit,
    )?;
    let mut timestamp_unit = package.clone();
    let v2::wire::ClockConfiguration::TimestampedEvent {
        timestamp_unit: unit,
    } = &mut timestamp_unit
        .temporal_bindings
        .get_mut(2)
        .ok_or(Error::MutationFixture("clock-timestamp-unit"))?
        .clock
    else {
        return Err(Error::MutationFixture("clock-timestamp-unit"));
    };
    *unit = "nanosecond".into();
    offer(
        "clock-timestamp-unit",
        "clock.timestamp_unit",
        v2::Refusal::Clock(v2::ClockField::TimestampUnit).code(),
        timestamp_unit,
    )?;

    let raw: serde_json::Value = serde_json::to_value(package)?;
    for (identity, binding, field, replacement, axis) in [
        (
            "clock-missing-sequence-authority",
            0,
            "sequence_authority",
            None,
            "clock.sequence_authority",
        ),
        (
            "clock-renamed-sequence-authority",
            0,
            "sequence_authority",
            Some("sequenceAuthority"),
            "clock.sequence_authority",
        ),
        ("clock-missing-period", 1, "period", None, "clock.period"),
        (
            "clock-renamed-period",
            1,
            "period",
            Some("sample_period"),
            "clock.period",
        ),
        (
            "clock-missing-timestamp-unit",
            2,
            "timestamp_unit",
            None,
            "clock.timestamp_unit",
        ),
        (
            "clock-renamed-timestamp-unit",
            2,
            "timestamp_unit",
            Some("timestampUnit"),
            "clock.timestamp_unit",
        ),
    ] {
        let mut mutated = raw.clone();
        let clock = mutated["temporal_bindings"][binding]["clock"]
            .as_object_mut()
            .ok_or(Error::MutationFixture(identity))?;
        let value = clock
            .remove(field)
            .ok_or(Error::MutationFixture(identity))?;
        if let Some(replacement) = replacement {
            clock.insert(replacement.into(), value);
        }
        let bytes = serde_json::to_vec(&mutated)?;
        let file = format!("mutations/{identity}.json");
        let artifact = reference(
            AUTHORITY,
            w::ArtifactKind::LinkedPackage,
            "examples/protocol-handoff/workflow-v2",
            revision("example-output", "2"),
            "quire.compiled-protocol",
            "2",
            &bytes,
        );
        files.push((file.clone(), bytes));
        cases.push(MutationCase {
            identity: identity.into(),
            axis: axis.into(),
            input: MutationInput::Offer { file, artifact },
            expected_refusal_code: "json".into(),
        });
    }

    let selected_dependency = dependencies
        .get(dependency_at)
        .ok_or(Error::MutationFixture("definition-original-bytes"))?;
    let mut changed_definition_bytes = selected_dependency.bytes.to_vec();
    changed_definition_bytes.push(b'\n');
    let changed_file = "mutations/definition-original-bytes.bin".to_owned();
    files.push((changed_file.clone(), changed_definition_bytes));
    cases.push(MutationCase {
        identity: "definition-original-bytes".into(),
        axis: "definition.original_bytes".into(),
        input: MutationInput::ReplaceOriginal {
            target: selected_dependency.file.clone(),
            file: changed_file,
        },
        expected_refusal_code: "invalid.seal".into(),
    });

    Ok((
        MutationManifest {
            format: MUTATION_MANIFEST_FORMAT.into(),
            base_offer: PUBLISHED_OFFER_FILE.into(),
            base_artifact: PUBLISHED_ARTIFACT_REFERENCE_FILE.into(),
            independent_selection: PUBLISHED_SELECTION_FILE.into(),
            cases,
        },
        files,
    ))
}

fn mutation_file<'a>(files: &'a MutationFiles, name: &str) -> Result<&'a [u8], Error> {
    files
        .iter()
        .find(|(file, _)| file == name)
        .map(|(_, bytes)| bytes.as_slice())
        .ok_or(Error::MutationFixture("manifest-file"))
}

#[allow(
    clippy::too_many_arguments,
    reason = "the replay keeps every independent selection explicit"
)]
fn verify_mutation_fixtures(
    manifest: &MutationManifest,
    files: &MutationFiles,
    base_bytes: &[u8],
    base_artifact: &w::ArtifactRef,
    inherited: artifact::Expected<'_>,
    temporal: &[v2::ExpectedTemporal<'_>],
    dependencies: &[Dependency],
    limits: artifact::Limits,
) -> Result<(), Error> {
    for case in &manifest.cases {
        let report = match &case.input {
            MutationInput::Offer { file, artifact } => v2::read(
                mutation_file(files, file)?,
                &v2::Expected {
                    inherited: artifact::Expected {
                        artifact,
                        ..inherited
                    },
                    temporal,
                },
                limits,
            ),
            MutationInput::ReplaceOriginal { target, file } => {
                let at = dependencies
                    .iter()
                    .position(|dependency| dependency.file == *target)
                    .ok_or(Error::MutationFixture("replacement-target"))?;
                let mut changed = inherited.dependencies.to_vec();
                let selected = changed
                    .get_mut(at)
                    .ok_or(Error::MutationFixture("replacement-index"))?;
                selected.bytes = mutation_file(files, file)?;
                v2::read(
                    base_bytes,
                    &v2::Expected {
                        inherited: artifact::Expected {
                            artifact: base_artifact,
                            dependencies: &changed,
                            ..inherited
                        },
                        temporal,
                    },
                    limits,
                )
            }
        };
        let actual = report.result().err().map(artifact::Error::code);
        if actual != Some(case.expected_refusal_code.as_str()) {
            return Err(Error::MutationReplay {
                identity: case.identity.clone(),
                expected: case.expected_refusal_code.clone(),
                actual: report
                    .result()
                    .err()
                    .map_or_else(|| "success".into(), |error| format!("{error:?}")),
            });
        }
    }
    Ok(())
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    fs::write(path, bytes).map_err(|error| io_at(path, error))
}
