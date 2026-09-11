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
const NATIVE_ID: &str = "ix://agent-ix/quire-spec-language/examples/protocol-handoff/workflow";
const SOURCE_PATH: &str = "examples/protocol-handoff/workflow.native";
const FORMAL_NAMESPACE: &str = "quire-contract-ir/source-revision";
const REQUIREMENT_NAMESPACE: &str = "quire-contract-ir/requirement-revision";
const DEFINITION_NAMESPACE: &str = "quire/native-definition-revision";
const BINARY_BYTES: usize = 16 * 1_048_576;
const NAMES: &[&str] = &["Allowed", "Healthy", "Due", "Flow"];
const CONTRACT: &[u8] = include_bytes!("../../docs/compiled-protocol-v1.md");

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
    #[error("{stage} did not complete: {details}")]
    Stage {
        stage: &'static str,
        details: String,
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
    #[error("an authored declaration or supplied dependency is missing or ambiguous")]
    Inventory,
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

fn source(model: &NativeModel) -> Result<Source, Error> {
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
    text.push_str(include_str!("workflow.body.native"));
    Ok(Source::read(
        SourceIdentity {
            identity: NATIVE_ID.into(),
            revision: "1".into(),
        },
        SOURCE_PATH,
        text.as_bytes(),
        quire_spec_language::Limits::default().source_bytes,
    )?)
}

struct Dependency {
    artifact: w::ArtifactRef,
    bytes: Cow<'static, [u8]>,
    requires: Vec<w::ArtifactRef>,
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
    source: w::Source,
    declarations: Vec<SelectedDeclaration>,
}

#[derive(Serialize)]
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
    clauses: &[ClauseBinding],
) -> Result<Vec<SelectedDeclaration>, Error> {
    clauses
        .iter()
        .map(|clause| {
            let [id] = namespace.lookup(&clause.name) else {
                return Err(Error::Inventory);
            };
            let syntax = namespace.syntax(*id).ok_or(Error::Inventory)?;
            Ok(SelectedDeclaration {
                name: clause.name.clone(),
                span: w::Span {
                    start: u32::try_from(syntax.span.start).map_err(|_| Error::Inventory)?,
                    end: u32::try_from(syntax.span.end).map_err(|_| Error::Inventory)?,
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
                    ir::ExecutionPoint::Initialization { .. }
                    | ir::ExecutionPoint::Pre { .. }
                    | ir::ExecutionPoint::Post { .. } => return Err(Error::Inventory),
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

pub fn write(directory: &Path) -> Result<(), Error> {
    let binary = current_binary()?;
    let model = model()?;
    let sources = [source(&model)?];
    let formal_sources = [formal(sources[0].clone(), "ProtocolHandoff")?];
    let owner = ir::RequirementRef::new(
        ir::PackageId::new("agent-ix/quire-spec-language").map_err(Error::Identifier)?,
        ir::RequirementId::new("ProtocolHandoff").map_err(Error::Identifier)?,
        ir::RequirementRevision::new(1).map_err(Error::Identifier)?,
    );
    let clauses = NAMES
        .iter()
        .map(|name| {
            Ok(ClauseBinding {
                name: (*name).into(),
                requirement: owner.clone(),
                clause: ir::ClauseId::new(name.to_lowercase()).map_err(Error::Identifier)?,
                execution_point: ir::ExecutionPoint::Handler {
                    name: ir::AnchorName::new("validate").map_err(Error::Identifier)?,
                },
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let mappings = [CheckBindings {
        source: formal_sources[0].clone(),
        clauses,
    }];
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
    let inventory = linking::SourceInventory {
        language: "ix:native".into(),
        edition: "1-draft".into(),
        units: vec![linking::ExpectedSource {
            authority: NATIVE_ID.into(),
            identity: sources[0].identity().clone(),
            digest: sources[0].digest(),
        }],
    };
    let namespace = linking::admit_namespace(
        &inventory,
        &sources,
        linking::WorkLimits::default(),
        quire_spec_language::Limits::default(),
    );
    let namespace = namespace.namespace().ok_or_else(|| Error::Stage {
        stage: "namespace",
        details: format!("{namespace:?}"),
    })?;
    let definition_inputs = definitions::Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rule_inputs,
    };
    let model_inputs = [ModelInput::Native(&model)];
    let binding = binding::bind(
        namespace,
        &definition_inputs,
        &model_inputs,
        linking::binding_work::Limits::default(),
    );
    let typed = composed::admit_types(&binding, &formal_sources, composed::TypeLimits::default());
    let proofs = proofs::discharge(&typed, &mappings, proofs::ProofLimits::default());
    if proofs.declarations().len() != NAMES.len()
        || proofs
            .declarations()
            .iter()
            .any(|declaration| declaration.disposition() != proofs::ProofDisposition::Discharged)
    {
        return Err(Error::Stage {
            stage: "type/definedness",
            details: format!("{proofs:?}"),
        });
    }
    let source_reference = reference(
        AUTHORITY,
        w::ArtifactKind::Source,
        NATIVE_ID,
        revision("native-source-revision", "1"),
        "ix:native",
        "1-draft",
        sources[0].text().as_bytes(),
    );
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
        revision(REQUIREMENT_NAMESPACE, "1"),
        "native-state-model",
        "2",
        model.artifact_bytes(),
    );
    let foreign_ref = reference(
        AUTHORITY,
        w::ArtifactKind::Source,
        &model.source().source().identity().identity,
        revision("native-source-revision", "1"),
        "native-rule-model",
        "2",
        model.source().source().text().as_bytes(),
    );
    let foreign_formal = w::Formal {
        document: "ProtocolHandoffModel".into(),
        revision: revision(FORMAL_NAMESPACE, "1"),
    };
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
        })
        .collect::<Vec<_>>();
    let baseline = dependencies
        .iter()
        .find(|dependency| dependency.artifact.identity == R::Edition.identity())
        .ok_or(Error::Inventory)?
        .artifact
        .clone();
    for rule in rules.values() {
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
        });
    }
    dependencies.extend([
        Dependency {
            artifact: contract.clone(),
            bytes: Cow::Borrowed(CONTRACT),
            requires: Vec::new(),
        },
        Dependency {
            artifact: binary_ref,
            bytes: Cow::Owned(binary),
            requires: Vec::new(),
        },
        Dependency {
            artifact: model_ref.clone(),
            bytes: Cow::Owned(model.artifact_bytes().to_vec()),
            requires: Vec::new(),
        },
    ]);
    // The registry owns definition/rule prerequisites. This fixture supplies
    // those exact direct edges instead of discovering dependencies from output.
    for definition in &selected {
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
            .ok_or(Error::Inventory)?;
        dependency.requires = requires;
    }
    dependencies.sort_by(|a, b| dependency_key(&a.artifact).cmp(&dependency_key(&b.artifact)));

    // All source/declaration expectations come from the original input and
    // authored mappings before native admission or any offered package exists.
    let selected_source = SelectedSource {
        source: w::Source {
            artifact: source_reference.clone(),
            native: w::NativeSource {
                identity: sources[0].identity().identity.clone(),
                revision: sources[0].identity().revision.clone(),
            },
            path: sources[0].path().into(),
            formal: w::Formal {
                document: formal_sources[0].identity().document().as_str().into(),
                revision: revision(
                    FORMAL_NAMESPACE,
                    &formal_sources[0].identity().revision().get().to_string(),
                ),
            },
            text: sources[0].text().into(),
        },
        declarations: declaration_selection(namespace, &mappings[0].clauses)?,
    };
    let language = w::Language {
        identity: inventory.language.clone(),
        edition: inventory.edition.clone(),
    };
    let supplied = dependency_inputs(&dependencies);
    let foreign = artifact::ExpectedForeignSource {
        artifact: &foreign_ref,
        formal: &foreign_formal,
        bytes: model.source().source().text().as_bytes(),
    };
    let models = [artifact::AdmittedModel {
        artifact: &model_ref,
        model: &model,
        source: &foreign,
    }];
    let native_sources = [native::SourceSelection {
        artifact: &source_reference,
        source: &formal_sources[0],
        revision_namespace: FORMAL_NAMESPACE,
    }];
    let selections = native::Selections {
        contract: &contract,
        baseline: &baseline,
        producer: &producer,
        sources: &native_sources,
        dependencies: &supplied,
        models: &models,
        definition_revision_namespace: DEFINITION_NAMESPACE,
        requirement_revision_namespace: REQUIREMENT_NAMESPACE,
    };
    let admitted = artifact_result(
        "native family admission",
        native::admit(&proofs, &selections, artifact::Limits::default()),
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
    let expected_declarations = selected_source
        .declarations
        .iter()
        .map(|declaration| artifact::ExpectedDeclaration {
            name: &declaration.name,
            span: &declaration.span,
            requirement: &declaration.requirement,
            clause: &declaration.clause,
            execution: &declaration.execution,
        })
        .collect::<Vec<_>>();
    let original = &selected_source.source;
    let expected_sources = [artifact::ExpectedSource {
        artifact: &original.artifact,
        native: &original.native,
        path: &original.path,
        formal: &original.formal,
        text: &original.text,
        declarations: &expected_declarations,
    }];
    let read = artifact_result(
        "independent selection reader",
        artifact::read(
            emitted.bytes(),
            &artifact::Expected {
                artifact: &output,
                contract: &contract,
                baseline: &baseline,
                producer: &producer,
                language: &language,
                sources: &expected_sources,
                dependencies: &supplied,
                models: &models,
            },
            artifact::Limits::default(),
        ),
    )?;
    if read.package() != admitted.package() || read.digest() != emitted.digest() {
        return Err(Error::RoundTrip);
    }
    let selection = Selection {
        artifact: output,
        contract,
        baseline,
        producer,
        language,
        sources: vec![selected_source],
        dependencies: dependencies
            .iter()
            .enumerate()
            .map(|(index, dependency)| SelectedDependency {
                artifact: dependency.artifact.clone(),
                file: format!("dependencies/{index}.bin"),
                requires: dependency.requires.clone(),
            })
            .collect(),
        model: SelectedModel {
            artifact: model_ref,
            source: w::Source {
                artifact: foreign_ref,
                formal: foreign_formal,
                native: w::NativeSource {
                    identity: model.source().source().identity().identity.clone(),
                    revision: model.source().source().identity().revision.clone(),
                },
                path: model.source().source().path().into(),
                text: model.source().source().text().into(),
            },
            source_file: "model-source.json",
            source_format: model_source::FORMAT_V2,
        },
    };
    write_files(directory, &selection, &dependencies, emitted.bytes())
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
        _ => Err(Error::Inventory),
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
    for (index, dependency) in dependencies.iter().enumerate() {
        write_file(
            &dependency_directory.join(format!("{index}.bin")),
            &dependency.bytes,
        )?;
    }
    write_file(
        &directory.join("workflow.native"),
        selection.sources[0].source.text.as_bytes(),
    )?;
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
