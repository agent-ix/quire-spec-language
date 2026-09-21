// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-121: the committed version-1 handoff admits through the public reader.

use std::{fs, path::Path};

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::{
    formal_source::FormalSource,
    model_source::{self, ModelSourceLimits},
    native_model::ModelLimits,
    protocol_artifact::{
        self as artifact,
        handoff::{
            Selection, PUBLISHED_V1_ARTIFACT_REFERENCE_FILE, PUBLISHED_V1_HANDOFF,
            PUBLISHED_V1_OFFER_FILE, PUBLISHED_V1_SELECTION_FILE,
        },
        wire as w,
    },
    ByteDigest, Source, SourceIdentity,
};

struct PublishedFixture {
    selection: Selection,
    dependency_bytes: Vec<Vec<u8>>,
    model: quire_spec_language::native_model::NativeModel,
}

impl PublishedFixture {
    fn load(root: &Path) -> Self {
        let selection: Selection = serde_json::from_slice(
            &fs::read(root.join(PUBLISHED_V1_SELECTION_FILE)).expect("published v1 selection"),
        )
        .expect("decode selection through the published record type");

        for selected in &selection.sources {
            let bytes = fs::read(root.join(&selected.file)).expect("published original source");
            assert_eq!(ByteDigest::of(&bytes), selected.source.artifact.digest);
            assert_eq!(bytes, selected.source.text.as_bytes());
        }

        let selected_source = &selection.model.source;
        let model_source_bytes =
            fs::read(root.join(&selection.model.source_file)).expect("published model source");
        assert_eq!(
            ByteDigest::of(&model_source_bytes),
            selected_source.artifact.digest
        );
        let source = Source::read(
            SourceIdentity {
                identity: selected_source.native.identity.clone(),
                revision: selected_source.native.revision.clone(),
            },
            &selected_source.path,
            &model_source_bytes,
            quire_spec_language::Limits::default().source_bytes,
        )
        .expect("read selected model source");
        let formal = FormalSource::new(
            source,
            ir::SourceIdentity::new(
                ir::SourceDocumentId::new(&selected_source.formal.document)
                    .expect("selected formal document"),
                ir::SourceRevision::new(
                    selected_source
                        .formal
                        .revision
                        .value
                        .parse()
                        .expect("selected formal revision integer"),
                )
                .expect("selected formal revision"),
            ),
        );
        let model = model_source::read(
            formal,
            &selection.model.source_format,
            ModelSourceLimits::default(),
        )
        .expect("read selected model")
        .admit(ModelLimits::default())
        .expect("admit selected model");

        let dependency_bytes = selection
            .dependencies
            .iter()
            .map(|dependency| {
                let bytes = fs::read(root.join(&dependency.file)).expect("published dependency");
                assert_eq!(ByteDigest::of(&bytes), dependency.artifact.digest);
                bytes
            })
            .collect::<Vec<_>>();
        let selected_model_bytes = selection
            .dependencies
            .iter()
            .zip(&dependency_bytes)
            .find(|(dependency, _)| dependency.artifact == selection.model.artifact)
            .map(|(_, bytes)| bytes.as_slice())
            .expect("selected model dependency");
        assert_eq!(model.artifact_bytes(), selected_model_bytes);

        Self {
            selection,
            dependency_bytes,
            model,
        }
    }

    fn read(&self, bytes: &[u8]) -> artifact::Report<artifact::AdmittedPackage> {
        let dependencies = self
            .selection
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
            artifact: &self.selection.model.source.artifact,
            formal: &self.selection.model.source.formal,
            bytes: self.model.source().source().text().as_bytes(),
        };
        let models = [artifact::AdmittedModel {
            artifact: &self.selection.model.artifact,
            model: &self.model,
            source: &foreign,
        }];
        let selected_sources = self
            .selection
            .sources
            .iter()
            .map(|source| {
                let declarations = source
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
                (source, declarations)
            })
            .collect::<Vec<_>>();
        let sources = selected_sources
            .iter()
            .map(|(source, declarations)| artifact::ExpectedSource {
                artifact: &source.source.artifact,
                native: &source.source.native,
                path: &source.source.path,
                formal: &source.source.formal,
                text: &source.source.text,
                declarations,
            })
            .collect::<Vec<_>>();

        artifact::read(
            bytes,
            &artifact::Expected {
                artifact: &self.selection.artifact,
                contract: &self.selection.contract,
                baseline: &self.selection.baseline,
                producer: &self.selection.producer,
                language: &self.selection.language,
                sources: &sources,
                dependencies: &dependencies,
                models: &models,
            },
            artifact::Limits::default(),
        )
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-7", "FR-042-AC-10")]
fn committed_v1_handoff_admits_through_the_public_reader() {
    let root = Path::new(PUBLISHED_V1_HANDOFF);
    let fixture = PublishedFixture::load(root);
    let bytes = fs::read(root.join(PUBLISHED_V1_OFFER_FILE)).expect("published v1 offer");
    let selected_reference: w::ArtifactRef = serde_json::from_slice(
        &fs::read(root.join(PUBLISHED_V1_ARTIFACT_REFERENCE_FILE))
            .expect("published v1 artifact reference"),
    )
    .expect("decode published v1 artifact reference");

    assert_eq!(selected_reference, fixture.selection.artifact);
    assert_eq!(ByteDigest::of(&bytes), fixture.selection.artifact.digest);
    let admitted = fixture
        .read(&bytes)
        .into_result()
        .expect("public strict v1 reader admits the committed handoff");
    assert_eq!(admitted.digest(), fixture.selection.artifact.digest);

    let flow = admitted
        .package()
        .declarations
        .iter()
        .find(|declaration| declaration.name == "Flow")
        .expect("published Flow declaration");
    let w::Body::Protocol { roles, .. } = &flow.body else {
        panic!("published Flow retains its protocol body")
    };
    let [service, provider] = roles.as_slice() else {
        panic!("published Flow retains its two authored roles")
    };
    assert_eq!(
        (service.name.as_str(), provider.name.as_str()),
        ("Service", "Provider")
    );
    assert_ne!(
        service.model, provider.model,
        "the published handoff must not assign two roles one authority"
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-10")]
fn published_v1_producer_digest_matches_this_repositorys_producer_source() {
    let root = Path::new(PUBLISHED_V1_HANDOFF);
    let fixture = PublishedFixture::load(root);
    assert_eq!(
        ByteDigest::of(include_bytes!("../examples/protocol-handoff/producer.rs")),
        fixture.selection.producer.binary.digest,
        "FR-042-AC-10: published producer identity is this repository's producer source",
    );
}
