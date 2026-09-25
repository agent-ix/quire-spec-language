// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-251 (FR-042-AC-15, FR-050-AC-8): `protocol_artifact::handoff::{write_v1,
//! write_v2}` write a complete, deterministic handoff that a consumer admits
//! from the files on disk alone, through QSL's own strict readers.
//!
//! Each test reads back only what the writer put in its output directory:
//! the `expected*.json` selection, the offer, its reference, the original
//! sources, the model source and every `dependencies/` file. Nothing is taken
//! from the writer's in-memory state.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use ix_trace_rs::trace;
use qsl_foundation::{ByteDigest, Source, SourceIdentity};
use quire_contract_ir as ir;
use quire_spec_language::{
    formal_source::FormalSource,
    model_source::{self, ModelSourceLimits},
    native_model::{ModelLimits, NativeModel},
    protocol_artifact::{
        self as artifact,
        handoff::{
            self, writer, Selection, SelectionV2, PUBLISHED_ARTIFACT_REFERENCE_FILE,
            PUBLISHED_CHECKSUMS_FILE, PUBLISHED_OFFER_FILE, PUBLISHED_SELECTION_FILE,
            PUBLISHED_V1_ARTIFACT_REFERENCE_FILE, PUBLISHED_V1_CHECKSUMS_FILE,
            PUBLISHED_V1_OFFER_FILE, PUBLISHED_V1_SELECTION_FILE,
        },
        v2, wire as w,
    },
};

/// Every regular file below `root`, keyed by its root-relative path.
fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(directory).expect("read handoff directory") {
            let entry = entry.expect("read handoff entry");
            let path = entry.path();
            let file_type = entry.file_type().expect("handoff entry type");
            if file_type.is_dir() {
                walk(root, &path, files);
            } else {
                assert!(file_type.is_file(), "handoff entries are plain files");
                let relative = path.strip_prefix(root).expect("below root").to_owned();
                files.insert(relative, fs::read(&path).expect("read handoff file"));
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

/// `SHA256SUMS` lists every other file exactly once with its digest.
fn assert_checksums_complete(root: &Path, checksums: &str) {
    let mut files = tree(root);
    let sums = String::from_utf8(
        files
            .remove(Path::new(checksums))
            .expect("the handoff carries its checksum inventory"),
    )
    .expect("checksum inventory is UTF-8");
    let mut listed = BTreeMap::new();
    for line in sums.lines() {
        let (digest, relative) = line.split_once("  ./").expect("sha256sum line");
        assert!(
            listed
                .insert(PathBuf::from(relative), digest.to_owned())
                .is_none(),
            "duplicate checksum path {relative}"
        );
    }
    assert_eq!(
        listed.keys().collect::<Vec<_>>(),
        files.keys().collect::<Vec<_>>(),
        "SHA256SUMS covers every handoff file"
    );
    for (relative, bytes) in &files {
        assert_eq!(
            format!("{:x}", ByteDigest::of(bytes)),
            listed[relative],
            "{relative:?}"
        );
    }
}

/// A consumer's view of one written handoff, loaded from its files only.
struct Loaded {
    selection: Selection,
    dependency_bytes: Vec<Vec<u8>>,
    model: NativeModel,
}

impl Loaded {
    fn new(root: &Path, selection: Selection) -> Self {
        for source in &selection.sources {
            let bytes = fs::read(root.join(&source.file)).expect("written original source");
            assert_eq!(ByteDigest::of(&bytes), source.source.artifact.digest);
            assert_eq!(bytes, source.source.text.as_bytes());
        }

        let selected = &selection.model.source;
        let bytes = fs::read(root.join(&selection.model.source_file)).expect("model source");
        assert_eq!(ByteDigest::of(&bytes), selected.artifact.digest);
        let source = Source::read(
            SourceIdentity {
                authority: selected.native.authority.clone(),
                identity: selected.native.identity.clone(),
                revision_namespace: selected.native.revision_namespace.clone(),
                revision: selected.native.revision.clone(),
            },
            &selected.path,
            &bytes,
            quire_spec_language::Limits::default().source_bytes,
        )
        .expect("read the selected model source");
        let formal = FormalSource::new(
            source,
            ir::SourceIdentity::new(
                ir::SourceDocumentId::new(&selected.formal.document).expect("formal document"),
                ir::SourceRevision::new(
                    selected
                        .formal
                        .revision
                        .value
                        .parse()
                        .expect("formal revision integer"),
                )
                .expect("formal revision"),
            ),
        );
        let model = model_source::read(
            formal,
            &selection.model.source_format,
            ModelSourceLimits::default(),
        )
        .expect("read the selected model")
        .admit(ModelLimits::default())
        .expect("admit the selected model");

        assert!(
            !selection.dependencies.is_empty(),
            "the written handoff selects its dependencies"
        );
        let dependency_bytes = selection
            .dependencies
            .iter()
            .map(|dependency| {
                let bytes = fs::read(root.join(&dependency.file)).expect("written dependency");
                assert_eq!(
                    ByteDigest::of(&bytes),
                    dependency.artifact.digest,
                    "{}",
                    dependency.file
                );
                bytes
            })
            .collect::<Vec<_>>();
        let model_bytes = selection
            .dependencies
            .iter()
            .zip(&dependency_bytes)
            .find(|(dependency, _)| dependency.artifact == selection.model.artifact)
            .map(|(_, bytes)| bytes.as_slice())
            .expect("the model artifact is a selected dependency");
        assert_eq!(model.artifact_bytes(), model_bytes);

        Self {
            selection,
            dependency_bytes,
            model,
        }
    }

    /// Build the strict reader's borrowed `Expected` and hand it to `read`.
    fn with_expected<T>(&self, read: impl FnOnce(artifact::Expected<'_>) -> T) -> T {
        let selection = &self.selection;
        let dependencies = selection
            .dependencies
            .iter()
            .zip(&self.dependency_bytes)
            .map(|(dependency, bytes)| artifact::SuppliedDependency {
                artifact: &dependency.artifact,
                bytes,
                requires: &dependency.requires,
            })
            .collect::<Vec<_>>();
        let foreign = artifact::ExpectedForeignSource {
            artifact: &selection.model.source.artifact,
            formal: &selection.model.source.formal,
            bytes: self.model.source().source().text().as_bytes(),
        };
        let models = [artifact::AdmittedModel {
            artifact: &selection.model.artifact,
            model: &self.model,
            source: &foreign,
        }];
        let declarations = selection
            .sources
            .iter()
            .map(|source| {
                source
                    .declarations
                    .iter()
                    .map(expected_declaration)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let sources = selection
            .sources
            .iter()
            .zip(&declarations)
            .map(|(source, declarations)| artifact::ExpectedSource {
                artifact: &source.source.artifact,
                native: &source.source.native,
                path: &source.source.path,
                formal: &source.source.formal,
                text: &source.source.text,
                declarations,
            })
            .collect::<Vec<_>>();
        read(artifact::Expected {
            artifact: &selection.artifact,
            contract: &selection.contract,
            baseline: &selection.baseline,
            producer: &selection.producer,
            language: &selection.language,
            sources: &sources,
            dependencies: &dependencies,
            models: &models,
            domain_packages: &[],
        })
    }
}

fn expected_declaration(
    declaration: &handoff::SelectedDeclaration,
) -> artifact::ExpectedDeclaration<'_> {
    artifact::ExpectedDeclaration {
        name: &declaration.name,
        span: &declaration.span,
        requirement: &declaration.requirement,
        clause: &declaration.clause,
        execution: &declaration.execution,
    }
}

fn written(write: fn(&Path) -> Result<(), writer::Error>) -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path().join("handoff");
    write(&root).expect("the writer emits a complete handoff");
    (directory, root)
}

#[test]
#[trace("TC-121", "FR-042-AC-15", "FR-042-AC-7")]
fn write_v1_handoff_admits_from_its_files_through_the_strict_v1_reader() {
    let (_directory, root) = written(handoff::write_v1);
    assert_checksums_complete(&root, PUBLISHED_V1_CHECKSUMS_FILE);

    let selection: Selection = serde_json::from_slice(
        &fs::read(root.join(PUBLISHED_V1_SELECTION_FILE)).expect("written v1 selection"),
    )
    .expect("decode expected.json through the published type");
    let offer = fs::read(root.join(PUBLISHED_V1_OFFER_FILE)).expect("written v1 offer");
    let reference: w::ArtifactRef = serde_json::from_slice(
        &fs::read(root.join(PUBLISHED_V1_ARTIFACT_REFERENCE_FILE)).expect("written v1 reference"),
    )
    .expect("decode the written v1 reference");
    assert_eq!(reference, selection.artifact);
    assert_eq!(ByteDigest::of(&offer), selection.artifact.digest);

    let loaded = Loaded::new(&root, selection);
    let admitted = loaded
        .with_expected(|expected| artifact::read(&offer, &expected, artifact::Limits::default()))
        .into_result()
        .expect("the strict v1 reader admits the written handoff");
    assert_eq!(admitted.digest(), loaded.selection.artifact.digest);
}

#[test]
#[trace("TC-138", "FR-050-AC-8", "FR-050-AC-1")]
fn write_v2_handoff_admits_from_its_files_through_the_strict_v2_reader() {
    let (_directory, root) = written(handoff::write_v2);
    assert_checksums_complete(&root, PUBLISHED_CHECKSUMS_FILE);

    let selection: SelectionV2 = serde_json::from_slice(
        &fs::read(root.join(PUBLISHED_SELECTION_FILE)).expect("written v2 selection"),
    )
    .expect("decode expected-v2.json through the published type");
    let offer = fs::read(root.join(PUBLISHED_OFFER_FILE)).expect("written v2 offer");
    let reference: w::ArtifactRef = serde_json::from_slice(
        &fs::read(root.join(PUBLISHED_ARTIFACT_REFERENCE_FILE)).expect("written v2 reference"),
    )
    .expect("decode the written v2 reference");
    assert_eq!(reference, selection.inherited.artifact);
    assert_eq!(ByteDigest::of(&offer), selection.inherited.artifact.digest);
    for temporal in &selection.temporal {
        let clock = fs::read(root.join(&temporal.clock_input.file)).expect("written clock input");
        assert_eq!(
            ByteDigest::of(&clock).to_string(),
            temporal.clock_input.digest
        );
    }

    let temporal = selection.temporal.clone();
    let declarations = temporal
        .iter()
        .map(|selected| expected_declaration(&selected.declaration))
        .collect::<Vec<_>>();
    let expected_temporal = temporal
        .iter()
        .zip(&declarations)
        .map(|(selected, declaration)| v2::ExpectedTemporal {
            source: &selected.source,
            declaration,
            definition: v2::ExpectedDefinition {
                identity: &selected.definition_identity,
                revision: &selected.definition_revision,
                artifact: &selected.definition_artifact,
            },
            clock: &selected.clock_input.configuration,
        })
        .collect::<Vec<_>>();
    let loaded = Loaded::new(&root, selection.inherited);
    let admitted = loaded
        .with_expected(|inherited| {
            v2::read(
                &offer,
                &v2::Expected {
                    inherited,
                    temporal: &expected_temporal,
                },
                artifact::Limits::default(),
            )
        })
        .into_result()
        .expect("the strict v2 reader admits the written handoff");
    assert_eq!(
        admitted.package().temporal_bindings.len(),
        expected_temporal.len()
    );

    let manifest: handoff::MutationManifest = serde_json::from_slice(
        &fs::read(root.join(handoff::PUBLISHED_MUTATION_MANIFEST_FILE))
            .expect("written mutation manifest"),
    )
    .expect("decode the written mutation manifest");
    assert!(
        !manifest.cases.is_empty(),
        "the writer emits a nonempty mutation corpus"
    );
    for case in &manifest.cases {
        match &case.input {
            handoff::MutationInput::Offer { file, .. } => assert!(
                root.join(file).is_file(),
                "{}: mutation offer file {file} exists",
                case.identity
            ),
            handoff::MutationInput::ReplaceOriginal { target, file } => {
                assert!(
                    root.join(target).is_file(),
                    "{}: replacement target {target} exists",
                    case.identity
                );
                assert!(
                    root.join(file).is_file(),
                    "{}: replacement file {file} exists",
                    case.identity
                );
            }
        }
    }

    // Replay one case's exact bytes against the strict reader and confirm it
    // refuses with the manifest's own recorded code, rather than trusting the
    // manifest's claim unverified.
    let (case, file, mutated_artifact) = manifest
        .cases
        .iter()
        .find_map(|case| match &case.input {
            handoff::MutationInput::Offer { file, artifact } => Some((case, file, artifact)),
            handoff::MutationInput::ReplaceOriginal { .. } => None,
        })
        .expect("the corpus carries at least one offer-mutation case");
    let mutated_bytes = fs::read(root.join(file)).expect("written mutation offer bytes");
    let refusal = loaded
        .with_expected(|inherited| {
            v2::read(
                &mutated_bytes,
                &v2::Expected {
                    inherited: artifact::Expected {
                        artifact: mutated_artifact,
                        ..inherited
                    },
                    temporal: &expected_temporal,
                },
                artifact::Limits::default(),
            )
        })
        .into_result()
        .expect_err("a mutation case must not admit");
    assert_eq!(
        refusal.code(),
        case.expected_refusal_code.as_str(),
        "{}: replayed refusal code",
        case.identity
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-15")]
fn write_v1_is_deterministic_across_runs() {
    let (_first, first) = written(handoff::write_v1);
    let (_second, second) = written(handoff::write_v1);
    assert_eq!(tree(&first), tree(&second));
}

#[test]
#[trace("TC-138", "FR-050-AC-8")]
fn write_v2_is_deterministic_across_runs() {
    let (_first, first) = written(handoff::write_v2);
    let (_second, second) = written(handoff::write_v2);
    assert_eq!(tree(&first), tree(&second));
}

#[test]
#[trace("TC-121", "FR-042-AC-10", "FR-042-AC-15")]
fn written_producer_identity_is_the_writer_source_digest() {
    let (_v1, v1) = written(handoff::write_v1);
    let (_v2, v2) = written(handoff::write_v2);
    let v1: Selection =
        serde_json::from_slice(&fs::read(v1.join(PUBLISHED_V1_SELECTION_FILE)).expect("v1"))
            .expect("v1 selection");
    let v2: SelectionV2 =
        serde_json::from_slice(&fs::read(v2.join(PUBLISHED_SELECTION_FILE)).expect("v2"))
            .expect("v2 selection");
    for selection in [v1, v2.inherited] {
        assert_eq!(
            selection.producer.binary.digest,
            ByteDigest::of(include_bytes!(
                "../../src/protocol_artifact/handoff/writer.rs"
            )),
            "FR-042-AC-10: producer identity is this repository's writer source"
        );
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-15")]
fn write_refuses_an_existing_directory() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let sentinel = directory.path().join("sentinel.txt");
    fs::write(&sentinel, b"do not touch").expect("write sentinel");
    for write in [handoff::write_v1, handoff::write_v2] {
        let error = write(directory.path()).expect_err("an existing directory is refused");
        assert!(
            matches!(&error, writer::Error::Io { path, .. } if path == directory.path()),
            "{error}"
        );
    }
    assert_eq!(
        fs::read_dir(directory.path()).expect("read").count(),
        1,
        "a refused write leaves the existing directory untouched"
    );
    assert_eq!(
        fs::read(&sentinel).expect("read sentinel"),
        b"do not touch",
        "a refused write does not modify a file already in the existing directory"
    );
}
