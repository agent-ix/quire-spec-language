// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real native inputs and fixture-authored external selections for emission tests.

#[path = "../composed_types/mod.rs"]
mod composed_inputs;

use std::collections::BTreeMap;

use quire_contract_ir as ir;
use quire_spec_language::checking::composed::{self, proofs, TypeLimits};
use quire_spec_language::checking::{CheckBindings, ClauseBinding};
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::linking::composed::binding_work::Limits as BindingLimits;
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::protocol_artifact::{self as artifact, native, v2, v3, wire as w};
use quire_spec_language::{ByteDigest, Source};

pub struct Unit<'a> {
    pub name: &'a str,
    pub body: &'a str,
    pub declarations: &'a [&'a str],
}

pub struct Inputs {
    pub model: NativeModel,
    pub sources: Vec<Source>,
    pub formal: Vec<FormalSource>,
    pub mappings: Vec<CheckBindings>,
    executions: BTreeMap<String, w::Execution>,
    pub source_references: Vec<w::ArtifactRef>,
    pub dependencies: Vec<(w::ArtifactRef, Vec<u8>)>,
    pub contract: w::ArtifactRef,
    pub baseline: w::ArtifactRef,
    pub producer: w::Producer,
    pub model_reference: w::ArtifactRef,
    foreign_source: w::ArtifactRef,
    foreign_formal: w::Formal,
}

/// Independently authored strict-v2 reader selection used by integration tests.
///
/// This deliberately owns its values rather than borrowing the native producer's
/// `TemporalSelection`, so a producer-side selection error cannot appoint the
/// reader's expectation by construction.
#[derive(Clone, Debug)]
pub struct TemporalExpectation {
    pub source: w::ArtifactRef,
    pub declaration: TemporalDeclarationExpectation,
    pub definition: TemporalDefinitionExpectation,
}

#[derive(Clone, Debug)]
pub struct TemporalDeclarationExpectation {
    pub name: String,
    pub span: w::Span,
    pub requirement: w::Requirement,
    pub clause: String,
    pub execution: w::Execution,
}

#[derive(Clone, Debug)]
pub struct TemporalDefinitionExpectation {
    pub identity: String,
    pub revision: w::Revision,
    pub artifact: w::ArtifactRef,
    pub clock: v2::wire::ClockConfiguration,
}

fn revision(namespace: &str, value: &str) -> w::Revision {
    w::Revision {
        namespace: namespace.into(),
        value: value.into(),
    }
}

fn reference(
    kind: w::ArtifactKind,
    identity: &str,
    wire: &str,
    version: &str,
    bytes: &[u8],
) -> w::ArtifactRef {
    w::ArtifactRef {
        ref_version: "ix.artifact-ref/3-draft".into(),
        kind,
        authority: "test:native-emission".into(),
        identity: identity.into(),
        revision: revision("test:fixture-revision", "1"),
        digest: ByteDigest::of(bytes),
        wire: w::Wire {
            identity: wire.into(),
            version: version.into(),
        },
    }
}

fn owner() -> ir::RequirementRef {
    ir::RequirementRef::new(
        ir::PackageId::new("test/native-emission").unwrap(),
        ir::RequirementId::new("NativeEmission").unwrap(),
        ir::RequirementRevision::new(7).unwrap(),
    )
}

impl Inputs {
    /// Re-derive one strict-v2 expectation from the authored fixture inventory.
    #[allow(
        dead_code,
        reason = "Only strict-v2 integration tests use this selector"
    )]
    pub fn temporal_expectation(
        &self,
        proofs: &proofs::ProofReport<'_, '_, '_>,
        source: usize,
        name: &str,
        definition: TemporalDefinitionExpectation,
    ) -> TemporalExpectation {
        let mapping = &self.mappings[source];
        let clause = mapping
            .clauses
            .iter()
            .find(|clause| clause.name == name)
            .expect("independently selected authored declaration");
        let [id] = proofs.types().binding().namespace().lookup(name) else {
            panic!("one independently selected authored declaration")
        };
        let span = proofs
            .types()
            .binding()
            .namespace()
            .syntax(*id)
            .expect("independently selected authored syntax")
            .span;
        TemporalExpectation {
            source: self.source_references[source].clone(),
            declaration: TemporalDeclarationExpectation {
                name: clause.name.clone(),
                span: w::Span {
                    start: span.start as u32,
                    end: span.end as u32,
                },
                requirement: w::Requirement {
                    package: clause.requirement.package().as_str().into(),
                    identity: clause.requirement.requirement().as_str().into(),
                    revision: revision(
                        "test:requirement-revision",
                        &clause.requirement.revision().get().to_string(),
                    ),
                },
                clause: clause.clause.as_str().into(),
                execution: self.executions[name].clone(),
            },
            definition,
        }
    }

    #[allow(
        dead_code,
        reason = "Shared fixture module; each test binary selects its own constructor"
    )]
    pub fn new(units: &[Unit<'_>]) -> Self {
        Self::with_model(units, composed_inputs::model("NativeEmission"))
    }

    /// The shared fixture already carries the full signed-64 integer scalar;
    /// this variant also carries the exact-rational domain at its ceiling.
    #[allow(
        dead_code,
        reason = "Only numeric-boundary consumers need this shared fixture variant"
    )]
    pub fn with_signed64_domains(units: &[Unit<'_>]) -> Self {
        Self::with_model(
            units,
            composed_inputs::model_with_signed64_domains("NativeEmission"),
        )
    }

    #[allow(
        dead_code,
        reason = "Only admitted state-evaluation fixtures need this compact query record"
    )]
    pub fn with_query_input(units: &[Unit<'_>], maximum: u32) -> Self {
        Self::with_model(
            units,
            composed_inputs::model_with_query_input("NativeEmission", maximum),
        )
    }

    #[allow(
        dead_code,
        reason = "Only admitted state-evaluation fixtures need this compact graph role"
    )]
    pub fn with_graph_input(units: &[Unit<'_>]) -> Self {
        Self::with_model(
            units,
            composed_inputs::model_with_graph_input("NativeEmission"),
        )
    }

    #[allow(
        dead_code,
        reason = "Only admitted state-evaluation fixtures need this optional graph role"
    )]
    pub fn with_optional_graph_input(units: &[Unit<'_>]) -> Self {
        Self::with_model(
            units,
            composed_inputs::model_with_optional_graph_input("NativeEmission"),
        )
    }

    pub fn with_model(units: &[Unit<'_>], model: NativeModel) -> Self {
        let sources: Vec<_> = units
            .iter()
            .map(|unit| composed_inputs::source(unit.name, &model, unit.body))
            .collect();
        let formal = composed_inputs::formal_sources(&sources);
        let mappings: Vec<CheckBindings> = formal
            .iter()
            .zip(units)
            .map(|(source, unit)| CheckBindings {
                source: source.clone(),
                clauses: unit
                    .declarations
                    .iter()
                    .map(|name| ClauseBinding {
                        name: (*name).into(),
                        requirement: owner(),
                        clause: ir::ClauseId::new(name.to_lowercase()).unwrap(),
                        execution_point: ir::ExecutionPoint::Handler {
                            name: ir::AnchorName::new("validate").unwrap(),
                        },
                    })
                    .collect(),
            })
            .collect();
        let executions = mappings
            .iter()
            .flat_map(|mapping| &mapping.clauses)
            .map(|clause| {
                (
                    clause.name.clone(),
                    w::Execution::Handler {
                        name: "validate".into(),
                    },
                )
            })
            .collect();
        let source_references = sources
            .iter()
            .map(|source| {
                let mut selected = reference(
                    w::ArtifactKind::Source,
                    &source.identity().identity,
                    "ix:native",
                    "1-draft",
                    source.text().as_bytes(),
                );
                selected.revision =
                    revision("test:native-source-revision", &source.identity().revision);
                selected
            })
            .collect();
        let mut dependencies = Vec::new();
        for definition in R::all() {
            let mut selected = reference(
                w::ArtifactKind::Source,
                definition.identity(),
                "text/markdown",
                "1",
                definition.bytes(),
            );
            selected.revision = revision("test:definition-revision", definition.revision());
            dependencies.push((selected, definition.bytes().to_vec()));
        }
        let rules: BTreeMap<_, _> = R::all()
            .iter()
            .flat_map(|definition| definition.rules())
            .map(|rule| (rule.path, rule.path.as_bytes()))
            .collect();
        for (path, bytes) in rules {
            dependencies.push((
                reference(w::ArtifactKind::Source, path, "text/markdown", "1", bytes),
                bytes.to_vec(),
            ));
        }
        let contract_bytes = include_bytes!("../../../docs/compiled-protocol-v1.md");
        let contract = reference(
            w::ArtifactKind::Source,
            "fixture-contract",
            "text/markdown",
            "1",
            contract_bytes,
        );
        let baseline_bytes = b"test-only accepted baseline; no ecosystem adoption claim";
        let baseline = reference(
            w::ArtifactKind::Source,
            "fixture-baseline",
            "test:baseline",
            "1",
            baseline_bytes,
        );
        let binary_bytes = b"test-only selected producer implementation";
        let binary = reference(
            w::ArtifactKind::GeneratedArtifact,
            "fixture-producer",
            "test:binary",
            "1",
            binary_bytes,
        );
        let producer = w::Producer {
            implementation: "test:native-emission-producer".into(),
            revision: revision("test:producer-revision", "1"),
            binary: binary.clone(),
        };
        let model_reference = reference(
            w::ArtifactKind::ModelPackage,
            "fixture-model",
            "native-state-model",
            "2",
            model.artifact_bytes(),
        );
        dependencies.extend([
            (contract.clone(), contract_bytes.to_vec()),
            (baseline.clone(), baseline_bytes.to_vec()),
            (binary, binary_bytes.to_vec()),
            (model_reference.clone(), model.artifact_bytes().to_vec()),
        ]);
        dependencies.sort_by(|a, b| {
            (a.0.kind.as_str(), &a.0.identity).cmp(&(b.0.kind.as_str(), &b.0.identity))
        });
        let foreign_source = reference(
            w::ArtifactKind::Source,
            "fixture-model-source",
            "native-rule-model",
            "2",
            model.source().source().text().as_bytes(),
        );
        let foreign_formal = w::Formal {
            document: model.source().identity().document().as_str().into(),
            revision: revision(
                "test:formal-revision",
                &model.source().identity().revision().get().to_string(),
            ),
        };
        Self {
            model,
            sources,
            formal,
            mappings,
            executions,
            source_references,
            dependencies,
            contract,
            baseline,
            producer,
            model_reference,
            foreign_source,
            foreign_formal,
        }
    }

    /// The fixture has one Node::step operation; derive its ordered export index
    /// from the original IR declarations, before any native emission exists.
    // Shared fixtures also serve event-only tests without operation contracts.
    #[allow(dead_code)]
    pub fn step_contracts(&mut self, pre: &str, post: &str) -> w::ExportRef {
        let [operation] = self.model.roles().operations.as_slice() else {
            panic!("one original fixture operation")
        };
        assert_eq!(operation.context.as_str(), "Node");
        assert_eq!(operation.name.as_str(), "step");
        // Export kinds preceding operation are enum, field and object.
        let preceding: usize = self
            .model
            .environment()
            .types()
            .iter()
            .map(|declaration| match declaration {
                ir::TypeDeclaration::Record { declaration } => declaration.fields().len(),
                ir::TypeDeclaration::Enum { .. } => 1,
            })
            .sum::<usize>()
            + self.model.roles().objects.len();
        let export = w::ExportRef {
            model: 0,
            export: preceding as u32,
        };
        for (name, execution, expected) in [
            (
                pre,
                ir::ExecutionPoint::Pre {
                    operation: operation.anchor.clone(),
                },
                w::Execution::Pre {
                    operation: export.clone(),
                },
            ),
            (
                post,
                ir::ExecutionPoint::Post {
                    operation: operation.anchor.clone(),
                },
                w::Execution::Post {
                    operation: export.clone(),
                },
            ),
        ] {
            let clause = self
                .mappings
                .iter_mut()
                .flat_map(|mapping| &mut mapping.clauses)
                .find(|clause| clause.name == name)
                .expect("explicit authored contract name");
            clause.execution_point = execution;
            *self.executions.get_mut(name).unwrap() = expected;
        }
        export
    }

    pub fn with_proofs(
        &self,
        type_limits: TypeLimits,
        proof_limits: proofs::ProofLimits,
        test: impl FnOnce(&proofs::ProofReport<'_, '_, '_>, &native::Selections<'_>),
    ) {
        composed_inputs::with_binding(
            &self.sources,
            &[&self.model],
            BindingLimits::default(),
            |binding| {
                assert!(
                    binding.complete(),
                    "binding exhaustion: {:?}",
                    binding.exhaustion()
                );
                let typed = composed::admit_types(binding, &self.formal, type_limits);
                let proved = proofs::discharge(&typed, &self.mappings, proof_limits);
                self.with_selections(|selected| test(&proved, selected));
            },
        );
    }

    fn with_selections<T>(&self, test: impl FnOnce(&native::Selections<'_>) -> T) -> T {
        let dependencies: Vec<_> = self
            .dependencies
            .iter()
            .map(|(artifact, bytes)| artifact::SuppliedDependency {
                artifact,
                bytes,
                requires: &[],
            })
            .collect();
        let foreign = artifact::ExpectedForeignSource {
            artifact: &self.foreign_source,
            formal: &self.foreign_formal,
            bytes: self.model.source().source().text().as_bytes(),
        };
        let models = [artifact::AdmittedModel {
            artifact: &self.model_reference,
            model: &self.model,
            source: &foreign,
        }];
        let sources: Vec<_> = self
            .source_references
            .iter()
            .zip(&self.formal)
            .map(|(artifact, source)| native::SourceSelection {
                artifact,
                source,
                revision_namespace: "test:formal-revision",
            })
            .collect();
        test(&native::Selections {
            contract: &self.contract,
            baseline: &self.baseline,
            producer: &self.producer,
            sources: &sources,
            dependencies: &dependencies,
            models: &models,
            definition_revision_namespace: "test:semantic-definition-revision",
            requirement_revision_namespace: "test:requirement-revision",
        })
    }

    /// Independently reconstruct only expectation records from original parser
    /// spans and authored mappings; no selection comes from the emitted payload.
    pub fn read(
        &self,
        proofs: &proofs::ProofReport<'_, '_, '_>,
        emitted: &native::EmittedPackage,
    ) -> artifact::Report<artifact::AdmittedPackage> {
        self.read_bytes(proofs, emitted.bytes(), emitted.digest())
    }

    /// Check transport against the original selections and an independently
    /// selected seal. Adverse bytes never acquire native emission authority.
    pub fn read_bytes(
        &self,
        proofs: &proofs::ProofReport<'_, '_, '_>,
        bytes: &[u8],
        digest: ByteDigest,
    ) -> artifact::Report<artifact::AdmittedPackage> {
        self.with_expected(proofs, bytes, digest, "1", |expected, _| {
            artifact::read(bytes, &expected, artifact::Limits::default())
        })
    }

    /// Read version-2 bytes against an independently authored temporal expectation.
    #[allow(dead_code, reason = "Only version-2 artifact controls use this helper")]
    pub fn read_v2(
        &self,
        proofs: &proofs::ProofReport<'_, '_, '_>,
        emitted: &native::AdmissionV2,
        temporal: &[TemporalExpectation],
    ) -> artifact::Report<v2::AdmittedPackage> {
        self.read_v2_bytes(
            proofs,
            emitted.bytes(),
            emitted.digest(),
            temporal,
            artifact::Limits::default(),
        )
    }

    /// Check version-2 transport against independently retained selections.
    #[allow(dead_code, reason = "Only version-2 artifact controls use this helper")]
    pub fn read_v2_bytes(
        &self,
        proofs: &proofs::ProofReport<'_, '_, '_>,
        bytes: &[u8],
        digest: ByteDigest,
        temporal: &[TemporalExpectation],
        limits: artifact::Limits,
    ) -> artifact::Report<v2::AdmittedPackage> {
        self.with_expected(proofs, bytes, digest, "2", |inherited, _| {
            let declarations: Vec<_> = temporal
                .iter()
                .map(|selection| artifact::ExpectedDeclaration {
                    name: &selection.declaration.name,
                    span: &selection.declaration.span,
                    requirement: &selection.declaration.requirement,
                    clause: &selection.declaration.clause,
                    execution: &selection.declaration.execution,
                })
                .collect();
            let expected_temporal: Vec<_> = temporal
                .iter()
                .zip(&declarations)
                .map(|(selection, declaration)| v2::ExpectedTemporal {
                    source: &selection.source,
                    declaration,
                    definition: v2::ExpectedDefinition {
                        identity: &selection.definition.identity,
                        revision: &selection.definition.revision,
                        artifact: &selection.definition.artifact,
                    },
                    clock: &selection.definition.clock,
                })
                .collect();
            v2::read(
                bytes,
                &v2::Expected {
                    inherited,
                    temporal: &expected_temporal,
                },
                limits,
            )
        })
    }

    /// Check version-3 transport against independent temporal and activation selections.
    #[allow(dead_code)]
    pub fn read_v3_bytes(
        &self,
        proofs: &proofs::ProofReport<'_, '_, '_>,
        bytes: &[u8],
        digest: ByteDigest,
        temporal: &[TemporalExpectation],
        activations: &[v3::ExpectedActivation],
        limits: artifact::Limits,
    ) -> artifact::Report<v3::AdmittedPackage> {
        self.with_expected(proofs, bytes, digest, "3", |inherited, _| {
            let declarations: Vec<_> = temporal
                .iter()
                .map(|selection| artifact::ExpectedDeclaration {
                    name: &selection.declaration.name,
                    span: &selection.declaration.span,
                    requirement: &selection.declaration.requirement,
                    clause: &selection.declaration.clause,
                    execution: &selection.declaration.execution,
                })
                .collect();
            let expected_temporal: Vec<_> = temporal
                .iter()
                .zip(&declarations)
                .map(|(selection, declaration)| v2::ExpectedTemporal {
                    source: &selection.source,
                    declaration,
                    definition: v2::ExpectedDefinition {
                        identity: &selection.definition.identity,
                        revision: &selection.definition.revision,
                        artifact: &selection.definition.artifact,
                    },
                    clock: &selection.definition.clock,
                })
                .collect();
            v3::read(
                bytes,
                &v3::Expected {
                    inherited: v2::Expected {
                        inherited,
                        temporal: &expected_temporal,
                    },
                    activations,
                },
                limits,
            )
        })
    }

    fn with_expected<T>(
        &self,
        proofs: &proofs::ProofReport<'_, '_, '_>,
        bytes: &[u8],
        digest: ByteDigest,
        version: &str,
        test: impl FnOnce(artifact::Expected<'_>, &[artifact::ExpectedSource<'_>]) -> T,
    ) -> T {
        let namespace = proofs.types().binding().namespace();
        let spans: Vec<Vec<_>> = self
            .mappings
            .iter()
            .map(|mapping| {
                mapping
                    .clauses
                    .iter()
                    .map(|clause| {
                        let [id] = namespace.lookup(&clause.name) else {
                            panic!("one authored declaration")
                        };
                        let span = namespace.syntax(*id).unwrap().span;
                        w::Span {
                            start: span.start as u32,
                            end: span.end as u32,
                        }
                    })
                    .collect()
            })
            .collect();
        let requirements: Vec<Vec<_>> = self
            .mappings
            .iter()
            .map(|mapping| {
                mapping
                    .clauses
                    .iter()
                    .map(|clause| w::Requirement {
                        package: clause.requirement.package().as_str().into(),
                        identity: clause.requirement.requirement().as_str().into(),
                        revision: revision(
                            "test:requirement-revision",
                            &clause.requirement.revision().get().to_string(),
                        ),
                    })
                    .collect()
            })
            .collect();
        let declarations: Vec<Vec<_>> = self
            .mappings
            .iter()
            .zip(&spans)
            .zip(&requirements)
            .map(|((mapping, spans), requirements)| {
                mapping
                    .clauses
                    .iter()
                    .zip(spans)
                    .zip(requirements)
                    .map(
                        |((clause, span), requirement)| artifact::ExpectedDeclaration {
                            name: &clause.name,
                            span,
                            requirement,
                            clause: clause.clause.as_str(),
                            execution: &self.executions[&clause.name],
                        },
                    )
                    .collect()
            })
            .collect();
        let native: Vec<_> = self
            .sources
            .iter()
            .map(|source| w::NativeSource {
                identity: source.identity().identity.clone(),
                revision: source.identity().revision.clone(),
            })
            .collect();
        let formal: Vec<_> = self
            .formal
            .iter()
            .map(|source| w::Formal {
                document: source.identity().document().as_str().into(),
                revision: revision(
                    "test:formal-revision",
                    &source.identity().revision().get().to_string(),
                ),
            })
            .collect();
        let sources: Vec<_> = self
            .sources
            .iter()
            .enumerate()
            .map(|(index, source)| artifact::ExpectedSource {
                artifact: &self.source_references[index],
                native: &native[index],
                path: source.path(),
                formal: &formal[index],
                text: source.text(),
                declarations: &declarations[index],
            })
            .collect();
        let mut artifact = reference(
            w::ArtifactKind::LinkedPackage,
            "fixture-native-output",
            "quire.compiled-protocol",
            version,
            bytes,
        );
        artifact.digest = digest;
        let language = w::Language {
            identity: "ix:native".into(),
            edition: "1-draft".into(),
        };
        self.with_selections(|selected| {
            test(
                artifact::Expected {
                    artifact: &artifact,
                    contract: &self.contract,
                    baseline: &self.baseline,
                    producer: &self.producer,
                    language: &language,
                    sources: &sources,
                    dependencies: selected.dependencies,
                    models: selected.models,
                },
                &sources,
            )
        })
    }
}
