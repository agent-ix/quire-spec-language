// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042/TC-121: example compilation recipe and independent input selections.
//! It calls production APIs; it is not a production request format or B reader.

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, Read},
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
    protocol_artifact::{self as artifact, native, wire as w},
    ByteDigest, Source, SourceIdentity,
};
use serde::Serialize;

const AUTHORITY: &str = "ix://agent-ix/quire-spec-language";
const STANDARD: &str = "ix://agent-ix/quire-specification";
const FORMAL_NAMESPACE: &str = "quire-contract-ir/source-revision";
const REQUIREMENT_NAMESPACE: &str = "quire-contract-ir/requirement-revision";
const DEFINITION_NAMESPACE: &str = "quire/native-definition-revision";
const BINARY_BYTES: usize = 16 * 1_048_576;
const CONTRACT: &[u8] = include_bytes!("../../docs/compiled-protocol-v1.md");

/// The recipe author selects clause owners and execution points before parsing.
struct UnitRecipe {
    file: &'static str,
    identity: &'static str,
    document: &'static str,
    requirement: &'static str,
    body: &'static str,
    clauses: &'static [AuthoredClause],
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
    },
];

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("usage: native_protocol_handoff <new-output-directory>")]
    Arguments,
    #[error("cannot access {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("the producer executable exceeds {maximum} bytes; build this example in release mode with debug information stripped")]
    BinaryLimit { maximum: usize },
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
    w::ArtifactRef {
        ref_version: "ix.artifact-ref/3-draft".into(),
        kind,
        authority: authority.into(),
        identity: identity.into(),
        revision,
        digest: ByteDigest::of(bytes),
        wire: w::Wire {
            identity: wire.into(),
            version: version.into(),
        },
    }
}

fn current_binary() -> Result<Vec<u8>, Error> {
    let path = std::env::current_exe().map_err(|error| io_at(Path::new("current_exe"), error))?;
    let file = File::open(&path).map_err(|error| io_at(&path, error))?;
    if file.metadata().map_err(|error| io_at(&path, error))?.len() > BINARY_BYTES as u64 {
        return Err(Error::BinaryLimit {
            maximum: BINARY_BYTES,
        });
    }
    let mut bytes = Vec::new();
    file.take(BINARY_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io_at(&path, error))?;
    if bytes.len() > BINARY_BYTES {
        return Err(Error::BinaryLimit {
            maximum: BINARY_BYTES,
        });
    }
    if !bytes.starts_with(b"\x7fELF") || bytes.get(6) != Some(&1) {
        return Err(Error::BinaryFormat);
    }
    Ok(bytes)
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

fn model() -> Result<NativeModel, Error> {
    let source = Source::read(
        SourceIdentity {
            identity: format!("{AUTHORITY}/examples/protocol-handoff/model"),
            revision: "1".into(),
        },
        "examples/protocol-handoff/model.json",
        include_bytes!("model.json"),
        quire_spec_language::Limits::default().source_bytes,
    )?;
    Ok(model_source::read(
        formal(source, "ProtocolHandoffModel")?,
        model_source::FORMAT_V2,
        ModelSourceLimits::default(),
    )?
    .admit(ModelLimits::default())?)
}

fn selected_definitions() -> Vec<R> {
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
    while let Some(next) = pending.pop() {
        if selected.insert(next) {
            pending.extend(next.requirements());
        }
    }
    selected.into_iter().collect()
}

fn source(model: &NativeModel, recipe: &UnitRecipe) -> Result<Source, Error> {
    let mut text = "language \"ix:native\" edition \"1-draft\";\n".to_owned();
    for (alias, definition) in [
        ("G", R::StateGraph),
        ("T", R::EventPosition),
        ("P", R::Protocol),
    ] {
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

#[derive(Serialize)]
struct SelectedDependency {
    artifact: w::ArtifactRef,
    file: String,
    requires: Vec<w::ArtifactRef>,
}

#[derive(Serialize)]
struct SelectedDeclaration {
    name: String,
    span: w::Span,
    requirement: w::Requirement,
    clause: String,
    execution: w::Execution,
}

#[derive(Serialize)]
struct SelectedSource {
    file: &'static str,
    source: w::Source,
    declarations: Vec<SelectedDeclaration>,
}

#[derive(Clone, Serialize)]
struct SelectedModel {
    artifact: w::ArtifactRef,
    source: w::Source,
    source_file: &'static str,
    source_format: &'static str,
}

/// Serialized fields from the existing Expected input, confined to this example.
#[derive(Serialize)]
struct Selection {
    artifact: w::ArtifactRef,
    contract: w::ArtifactRef,
    baseline: w::ArtifactRef,
    producer: w::Producer,
    language: w::Language,
    sources: Vec<SelectedSource>,
    dependencies: Vec<SelectedDependency>,
    model: SelectedModel,
}

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
        let model = model()?;
        let operation = OperationSelection::new(&model)?;
        let units = UNITS
            .iter()
            .map(|recipe| UnitInput::new(recipe, &model, &operation))
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(Self {
            model,
            units,
            operation,
            definitions: DefinitionInputs::new(),
        })
    }
}

struct DefinitionInputs {
    selected: Vec<R>,
    artifacts: Vec<definitions::Artifact<'static>>,
    rules: Vec<definitions::RuleInput<'static>>,
}

impl DefinitionInputs {
    fn new() -> Self {
        let selected = selected_definitions();
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
    fn new(inputs: &Inputs, binary: Vec<u8>) -> Result<Self, Error> {
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
        let binary_ref = reference(
            AUTHORITY,
            w::ArtifactKind::GeneratedArtifact,
            "native_protocol_handoff",
            revision("crate-version", env!("CARGO_PKG_VERSION")),
            "ELF",
            "1",
            &binary,
        );
        let producer = w::Producer {
            implementation: "quire-spec-language/native_protocol_handoff".into(),
            revision: revision("crate-version", env!("CARGO_PKG_VERSION")),
            binary: binary_ref.clone(),
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

        let dependencies =
            selected_dependencies(inputs, binary, &contract, binary_ref, &model_ref)?;
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
                source_file: "model-source.json",
                source_format: model_source::FORMAT_V2,
            },
            dependencies,
        })
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
    binary: Vec<u8>,
    contract: &w::ArtifactRef,
    binary_ref: w::ArtifactRef,
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
    dependencies.extend([
        Dependency {
            artifact: contract.clone(),
            bytes: Cow::Borrowed(CONTRACT),
            requires: Vec::new(),
            file: String::new(),
        },
        Dependency {
            artifact: binary_ref,
            bytes: Cow::Owned(binary),
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

fn compile(inputs: &Inputs, selected: &SelectedInputs) -> Result<Output, Error> {
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
                file: unit.file,
                source: wire_source(unit.artifact.clone(), &unit.mapping.source),
                declarations: declaration_selection(namespace, unit, &inputs.operation)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    emit_and_read(inputs, selected, &proofs, original)
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
    let binary = current_binary()?;
    let inputs = Inputs::new()?;
    let selected = SelectedInputs::new(&inputs, binary)?;
    let output = compile(&inputs, &selected)?;
    write_files(
        directory,
        &output.selection,
        &selected.dependencies,
        output.emitted.bytes(),
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
            &directory.join(selected.file),
            selected.source.text.as_bytes(),
        )?;
    }
    write_file(
        &directory.join(selection.model.source_file),
        selection.model.source.text.as_bytes(),
    )?;
    write_file(&directory.join("expected.json"), &selected_bytes)?;
    write_file(
        &directory.join("compiled-protocol.ref.json"),
        &reference_bytes,
    )?;
    write_file(&directory.join("compiled-protocol.json"), bytes)
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    fs::write(path, bytes).map_err(|error| io_at(path, error))
}
