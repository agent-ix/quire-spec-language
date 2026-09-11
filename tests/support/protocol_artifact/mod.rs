// SPDX-License-Identifier: AGPL-3.0-only
//! Authored reader data and independent selections, not native compiler emission.

use std::collections::BTreeMap;

use quire_contract_ir as ir;
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::model_source::{self, ModelSourceLimits};
use quire_spec_language::native_model::{ModelLimits, NativeModel};
use quire_spec_language::protocol_artifact::{self as artifact, wire as w};
use quire_spec_language::{ByteDigest, Source, SourceIdentity};
use serde_json::json;

pub struct Fixture {
    pub package: w::Package,
    pub model: NativeModel,
    pub dependency_bytes: Vec<Vec<u8>>,
    pub foreign_source: w::ArtifactRef,
    pub foreign_formal: w::Formal,
}

fn revision(value: &str) -> w::Revision {
    w::Revision {
        namespace: "test-authored-revision".into(),
        value: value.into(),
    }
}

pub fn reference(
    kind: w::ArtifactKind,
    identity: &str,
    wire: &str,
    version: &str,
    bytes: &[u8],
) -> w::ArtifactRef {
    w::ArtifactRef {
        ref_version: "ix.artifact-ref/3-draft".into(),
        kind,
        authority: "test:protocol-reader".into(),
        identity: identity.into(),
        revision: revision("fixture-1"),
        digest: ByteDigest::of(bytes),
        wire: w::Wire {
            identity: wire.into(),
            version: version.into(),
        },
    }
}

pub fn handle(index: u32) -> w::Handle {
    w::Handle {
        declaration: 0,
        index,
    }
}

impl Fixture {
    pub fn new() -> Self {
        let model_source = Source::read(
            SourceIdentity {
                identity: "test:rule-model".into(),
                revision: "draft:1".into(),
            },
            "native-rule-model.json",
            include_bytes!("../../fixtures/native-rule-model.json"),
            quire_spec_language::source::MAX_SOURCE_BYTES,
        )
        .unwrap();
        let model_source = FormalSource::new(
            model_source,
            ir::SourceIdentity::new(
                ir::SourceDocumentId::new("RuleModelSource").unwrap(),
                ir::SourceRevision::new(1).unwrap(),
            ),
        );
        let model = model_source::read(
            model_source,
            model_source::FORMAT,
            ModelSourceLimits::default(),
        )
        .unwrap()
        .admit(ModelLimits::default())
        .unwrap();
        let mut registry = [
            R::Edition,
            R::StateCore,
            R::StateQueries,
            R::StateGraph,
            R::TemporalFacet,
            R::Protocol,
            R::Package,
        ];
        registry.sort_by_key(|definition| definition.identity());
        let mut inputs = BTreeMap::<String, (w::ArtifactRef, Vec<u8>)>::new();
        for definition in &registry {
            let mut selected = reference(
                w::ArtifactKind::Source,
                definition.path(),
                "text/markdown",
                "1",
                definition.bytes(),
            );
            selected.revision = revision(definition.revision());
            inputs.insert(
                definition.path().into(),
                (selected, definition.bytes().to_vec()),
            );
            for rule in definition.rules() {
                inputs.entry(rule.path.into()).or_insert_with(|| {
                    (
                        reference(
                            w::ArtifactKind::Source,
                            rule.path,
                            "text/markdown",
                            "1",
                            rule.bytes,
                        ),
                        rule.bytes.to_vec(),
                    )
                });
            }
        }
        let contract_bytes = include_bytes!("../../../docs/compiled-protocol-v1.md");
        let contract = reference(
            w::ArtifactKind::Source,
            "compiled-protocol-contract",
            "text/markdown",
            "1",
            contract_bytes,
        );
        let baseline_bytes = b"independently selected protocol reader baseline";
        let baseline = reference(
            w::ArtifactKind::Source,
            "accepted-baseline",
            "test:baseline",
            "1",
            baseline_bytes,
        );
        let binary_bytes = b"independently selected reader fixture producer binary";
        let binary = reference(
            w::ArtifactKind::GeneratedArtifact,
            "producer-binary",
            "test:binary",
            "1",
            binary_bytes,
        );
        let model_ref = reference(
            w::ArtifactKind::ModelPackage,
            "rule-model",
            model.profile().as_str(),
            "1",
            model.artifact_bytes(),
        );
        for (key, selected, bytes) in [
            ("contract", contract.clone(), contract_bytes.as_slice()),
            ("baseline", baseline.clone(), baseline_bytes.as_slice()),
            ("binary", binary.clone(), binary_bytes.as_slice()),
            ("model", model_ref.clone(), model.artifact_bytes()),
        ] {
            inputs.insert(key.into(), (selected, bytes.to_vec()));
        }
        let mut inputs: Vec<_> = inputs.into_iter().collect();
        // All references share authority and have distinct kind/identity pairs.
        inputs.sort_by(|a, b| {
            (a.1 .0.kind.as_str(), &a.1 .0.identity).cmp(&(b.1 .0.kind.as_str(), &b.1 .0.identity))
        });
        let dependency = |key: &str| inputs.iter().position(|input| input.0 == key).unwrap() as u32;
        let definition = |wanted: R| {
            registry
                .iter()
                .position(|offered| *offered == wanted)
                .unwrap() as u32
        };
        let dependencies = inputs
            .iter()
            .map(|(_, (artifact, _))| w::Dependency {
                artifact: artifact.clone(),
                requires: vec![],
            })
            .collect();
        let definitions = registry
            .iter()
            .map(|selected| {
                let mut rules: Vec<_> = selected
                    .rules()
                    .iter()
                    .map(|rule| dependency(rule.path))
                    .collect();
                rules.sort_unstable();
                let mut requires: Vec<_> = selected
                    .requirements()
                    .iter()
                    .map(|required| definition(*required))
                    .collect();
                requires.sort_unstable();
                w::Definition {
                    identity: selected.identity().into(),
                    revision: revision(selected.revision()),
                    artifact: dependency(selected.path()),
                    rules,
                    requires,
                }
            })
            .collect();
        let mut text = "language \"ix:native\" edition \"1-draft\";\n".to_owned();
        for (alias, selected) in [("P", R::Protocol), ("S", R::StateCore)] {
            text.push_str(&format!(
                "profile {alias} = \"{}\" version \"{}\" digest \"{}\";\n",
                selected.identity(),
                selected.revision(),
                selected.selection().digest
            ));
        }
        text.push_str(&format!(
            "model M = \"{}\" version \"{}\" digest \"{}\";\n",
            model.environment().owner().package().as_str(),
            model.environment().owner().revision().get(),
            model.digest()
        ));
        text.push_str("// Unicode café / and escaped control: \t\nprotocol Demo using P over (view: M::Node) on origin {\n  run check Good using S { true };\n  finish Closed as (closed: M::Node) { true };\n}\n");
        let source = w::Source {
            artifact: reference(
                w::ArtifactKind::Source,
                "native-protocol",
                "ix:native",
                "1-draft",
                text.as_bytes(),
            ),
            native: w::NativeSource {
                identity: "test:protocol-source".into(),
                revision: "editable:7".into(),
            },
            path: "protocol.quire".into(),
            formal: w::Formal {
                document: "ProtocolReaderSource".into(),
                revision: revision("7"),
            },
            text,
        };
        let start = source.text.find("protocol Demo").unwrap() as u32;
        let end = source.text.len() as u32;
        let locus = w::Locus {
            source: 0,
            span: w::Span { start, end },
        };
        let first = source.text.find("true").unwrap() as u32;
        let last = source.text.rfind("true").unwrap() as u32;
        let boolean = |original_expression, at, anchor| w::Value {
            original_expression,
            locus: w::Locus {
                source: 0,
                span: w::Span {
                    start: at,
                    end: at + 4,
                },
            },
            operator_locus: w::Nullable(None),
            value_type: 1,
            profile: definition(R::StateCore),
            scope: handle(0),
            anchor: handle(anchor),
            origin: w::Origin::Independent {},
            operation: w::ValueOperation::Boolean { value: true },
        };
        let foreign_source = reference(
            w::ArtifactKind::Source,
            "rule-model-source",
            "native-rule-model",
            "1",
            model.source().source().text().as_bytes(),
        );
        let foreign_formal = w::Formal {
            document: "RuleModelSource".into(),
            revision: revision("1"),
        };
        let object_span = model
            .source()
            .to_native(&model.roles().objects[0].source)
            .unwrap();
        let object = w::ExportRef {
            model: 0,
            export: 0,
        };
        let bindings = [
            ("workflow", w::BindingKind::WorkflowInstance, 0),
            ("current", w::BindingKind::Snapshot, 1),
            ("closed", w::BindingKind::Closure, 2),
        ]
        .into_iter()
        .map(|(name, kind, anchor)| w::BindingRequirement {
            name: name.into(),
            kind,
            value_type: w::Nullable(Some(0)),
            authority: model_ref.clone(),
            contract: dependency("proposals/quire-v1/protocol-contract.md"),
            model: w::Nullable(Some(object.clone())),
            subject: w::Subject::Declaration { declaration: 0 },
            anchor: handle(anchor),
            scope: handle(0),
            relation: w::Nullable(None),
            requires: vec![],
            locus: locus.clone(),
        })
        .collect();
        let declaration = w::Declaration {
            name: "Demo".into(),
            locus: locus.clone(),
            requirement: w::Requirement {
                package: "test/protocol-reader".into(),
                identity: "Demo".into(),
                revision: revision("7"),
            },
            clause: "protocol-body".into(),
            execution: w::Execution::Handler {
                name: "workflow".into(),
            },
            profile: definition(R::Protocol),
            requires: vec![],
            scopes: vec![w::Scope {
                parent: w::Nullable(None),
                locus: locus.clone(),
            }],
            anchors: [
                w::AnchorKind::Activation,
                w::AnchorKind::ProtocolInstant,
                w::AnchorKind::Finish,
            ]
            .into_iter()
            .enumerate()
            .map(|(index, kind)| w::Anchor {
                kind,
                owner: w::Nullable(None),
                binding: w::Nullable(Some(index as u32)),
                locus: locus.clone(),
            })
            .collect(),
            binders: [
                ("view", w::BinderKind::Input, 0),
                ("closed", w::BinderKind::Finish, 2),
            ]
            .into_iter()
            .map(|(name, kind, anchor)| w::Binder {
                name: name.into(),
                kind,
                value_type: 0,
                scope: handle(0),
                anchor: handle(anchor),
                initializer: w::Nullable(None),
                locus: locus.clone(),
            })
            .collect(),
            values: vec![boolean(0, first, 1), boolean(1, last, 2)],
            temporal: vec![],
            bindings,
            body: w::Body::Protocol {
                input: handle(0),
                activation: w::Activation::Origin { anchor: handle(0) },
                captures: vec![],
                roles: vec![],
                relationships: vec![],
                channels: vec![],
                compensations: vec![],
                temporal_requirements: vec![],
                controls: vec![w::Control {
                    name: "Good".into(),
                    original_node: 0,
                    locus: locus.clone(),
                    operation: w::ControlOperation::Check {
                        profile: definition(R::StateCore),
                        value: handle(0),
                    },
                }],
                causal_edges: vec![w::CausalEdge {
                    kind: w::EdgeKind::Sequence,
                    owner: handle(0),
                    from: w::Endpoint {
                        node: handle(0),
                        port: w::Port::Enter,
                    },
                    to: w::Endpoint {
                        node: handle(0),
                        port: w::Port::Exit,
                    },
                    maximum: w::Nullable(None),
                }],
                run: handle(0),
                finish: Box::new(w::Finish {
                    name: "Closed".into(),
                    binder: handle(1),
                    constraint: handle(1),
                    closure: 2,
                    locus,
                }),
            },
        };
        let package = w::Package {
            wire: artifact::WIRE.into(),
            media: artifact::MEDIA.into(),
            schema: artifact::SCHEMA.into(),
            package_type: artifact::PACKAGE_TYPE.into(),
            encoding: artifact::ENCODING.into(),
            numeric: artifact::NUMERIC_PROFILE.into(),
            contract,
            baseline,
            producer: w::Producer {
                implementation: "test:reader-fixture".into(),
                revision: revision("1"),
                binary,
            },
            language: w::Language {
                identity: "ix:native".into(),
                edition: "1-draft".into(),
            },
            package_definition: definition(R::Package),
            features: w::Features {
                declarations: vec!["family.protocol".into()],
                required: vec![
                    "quire.protocol.bindings/1".into(),
                    "quire.protocol.control/1".into(),
                    "quire.protocol.numeric/1".into(),
                    "quire.protocol.values/1".into(),
                ],
                optional: vec![],
            },
            sources: vec![source],
            dependencies,
            definitions,
            models: vec![w::Model {
                artifact: dependency("model"),
                profile: model.profile().as_str().into(),
                exports: vec![w::Export {
                    kind: w::ExportKind::Object,
                    path: vec!["Node".into()],
                    locus: w::ForeignLocus {
                        source: foreign_source.clone(),
                        formal: foreign_formal.clone(),
                        span: w::Span {
                            start: object_span.start as u32,
                            end: object_span.end as u32,
                        },
                    },
                }],
                correspondence: w::Nullable(None),
            }],
            types: vec![w::Type::Object { export: object }, w::Type::Boolean {}],
            declarations: vec![declaration],
        };
        Self {
            package,
            model,
            dependency_bytes: inputs.into_iter().map(|(_, (_, bytes))| bytes).collect(),
            foreign_source,
            foreign_formal,
        }
    }

    pub fn candidate(&self) -> artifact::Candidate {
        artifact::encode_candidate(&self.package, artifact::Limits::default())
            .into_result()
            .expect("valid untrusted wire candidate")
    }

    /// Two separate branch-label scopes retain repeated nested control names.
    /// The records and original text are authored reader inputs, not emission.
    pub fn branched() -> Self {
        const CHECK: &str = "check Same using S { true };";
        const SEQUENCE: &str = "sequence Nested { check Same using S { true }; }";
        const PARALLEL: &str = "parallel Both {\n    branch left sequence Nested { check Same using S { true }; }\n    branch right sequence Nested { check Same using S { true }; }\n  } join all [left,right];";
        const FINISH: &str = "finish Closed as (closed: M::Node) { true };";
        let mut fixture = Self::new();
        let start = fixture.package.declarations[0].locus.span.start as usize;
        let source = &mut fixture.package.sources[0];
        source.text.truncate(start);
        source.text.push_str(&format!("protocol Demo using P over (view: M::Node) on origin {{\n  run {PARALLEL}\n  {FINISH}\n}}\n"));
        source.artifact.digest = ByteDigest::of(source.text.as_bytes());
        let locus = |at: usize, length: usize| w::Locus {
            source: 0,
            span: w::Span {
                start: at as u32,
                end: (at + length) as u32,
            },
        };
        let original = locus(start, source.text.len() - start);
        let sequences: Vec<_> = source
            .text
            .match_indices(SEQUENCE)
            .map(|(at, _)| locus(at, SEQUENCE.len()))
            .collect();
        let checks: Vec<_> = source
            .text
            .match_indices(CHECK)
            .map(|(at, _)| locus(at, CHECK.len()))
            .collect();
        assert_eq!((sequences.len(), checks.len()), (2, 2));
        let declaration = &mut fixture.package.declarations[0];
        declaration.locus = original.clone();
        declaration.scopes[0].locus = original.clone();
        for anchor in &mut declaration.anchors {
            anchor.locus = original.clone();
        }
        for binder in &mut declaration.binders {
            binder.locus = original.clone();
        }
        for binding in &mut declaration.bindings {
            binding.locus = original.clone();
        }
        let boolean = declaration.values[0].clone();
        let profile = boolean.profile;
        declaration.values = source
            .text
            .match_indices("true")
            .enumerate()
            .map(|(index, (at, _))| {
                let mut value = boolean.clone();
                value.original_expression = index as u32;
                value.locus = locus(at, 4);
                value.anchor = handle(if index == 2 { 2 } else { 1 });
                value
            })
            .collect();
        assert_eq!(declaration.values.len(), 3);
        let w::Body::Protocol {
            controls,
            causal_edges,
            finish,
            ..
        } = &mut declaration.body
        else {
            unreachable!()
        };
        finish.locus = locus(source.text.find(FINISH).unwrap(), FINISH.len());
        finish.constraint = handle(2);
        let branches = [("left", 1, &sequences[0]), ("right", 3, &sequences[1])]
            .into_iter()
            .map(|(label, index, selected)| {
                let at = source.text.find(&format!("branch {label} ")).unwrap();
                w::Branch {
                    label: label.into(),
                    body: handle(index),
                    locus: locus(at, selected.span.end as usize - at),
                }
            })
            .collect();
        // Source order is outer-before-inner; the original parser arena allocates
        // each completed child before its enclosing sequence/parallel node.
        *controls = vec![
            w::Control {
                name: "Both".into(),
                original_node: 4,
                locus: locus(source.text.find(PARALLEL).unwrap(), PARALLEL.len()),
                operation: w::ControlOperation::Parallel {
                    branches,
                    join: vec![0, 1],
                },
            },
            w::Control {
                name: "Nested".into(),
                original_node: 1,
                locus: sequences[0].clone(),
                operation: w::ControlOperation::Sequence {
                    children: vec![handle(2)],
                },
            },
            w::Control {
                name: "Same".into(),
                original_node: 0,
                locus: checks[0].clone(),
                operation: w::ControlOperation::Check {
                    profile,
                    value: handle(0),
                },
            },
            w::Control {
                name: "Nested".into(),
                original_node: 3,
                locus: sequences[1].clone(),
                operation: w::ControlOperation::Sequence {
                    children: vec![handle(4)],
                },
            },
            w::Control {
                name: "Same".into(),
                original_node: 2,
                locus: checks[1].clone(),
                operation: w::ControlOperation::Check {
                    profile,
                    value: handle(1),
                },
            },
        ];
        let edge = |owner, kind, from, from_port, to, to_port| w::CausalEdge {
            kind,
            owner: handle(owner),
            from: w::Endpoint {
                node: handle(from),
                port: from_port,
            },
            to: w::Endpoint {
                node: handle(to),
                port: to_port,
            },
            maximum: w::Nullable(None),
        };
        use w::{
            EdgeKind::{Branch, Join, Sequence},
            Port::{Enter, Exit},
        };
        // Independently expanded from the normative Parallel/Sequence/Check rows,
        // in (owner,kind,from-node,from-port,to-node,to-port) order.
        *causal_edges = vec![
            edge(0, Branch, 0, Enter, 1, Enter),
            edge(0, Branch, 0, Enter, 3, Enter),
            edge(0, Join, 1, Exit, 0, Exit),
            edge(0, Join, 3, Exit, 0, Exit),
            edge(1, Sequence, 1, Enter, 2, Enter),
            edge(1, Sequence, 2, Exit, 1, Exit),
            edge(2, Sequence, 2, Enter, 2, Exit),
            edge(3, Sequence, 3, Enter, 4, Enter),
            edge(3, Sequence, 4, Exit, 3, Exit),
            edge(4, Sequence, 4, Enter, 4, Exit),
        ];
        fixture
    }

    pub fn seal(bytes: &[u8]) -> w::ArtifactRef {
        reference(
            w::ArtifactKind::LinkedPackage,
            "compiled-reader-fixture",
            "quire.compiled-protocol",
            "1",
            bytes,
        )
    }

    pub fn read(
        &self,
        bytes: &[u8],
        selected: &w::ArtifactRef,
        limits: artifact::Limits,
    ) -> artifact::Report<artifact::AdmittedPackage> {
        let declaration = &self.package.declarations[0];
        let declarations = [artifact::ExpectedDeclaration {
            name: &declaration.name,
            span: &declaration.locus.span,
            requirement: &declaration.requirement,
            clause: &declaration.clause,
            execution: &declaration.execution,
        }];
        let source = &self.package.sources[0];
        let sources = [artifact::ExpectedSource {
            artifact: &source.artifact,
            native: &source.native,
            path: &source.path,
            formal: &source.formal,
            text: &source.text,
            declarations: &declarations,
        }];
        let closure: Vec<Vec<_>> = self
            .package
            .dependencies
            .iter()
            .map(|dependency| {
                dependency
                    .requires
                    .iter()
                    .map(|index| self.package.dependencies[*index as usize].artifact.clone())
                    .collect()
            })
            .collect();
        let dependencies: Vec<_> = self
            .package
            .dependencies
            .iter()
            .zip(&self.dependency_bytes)
            .zip(&closure)
            .map(
                |((dependency, bytes), requires)| artifact::SuppliedDependency {
                    artifact: &dependency.artifact,
                    bytes,
                    requires,
                },
            )
            .collect();
        let foreign = artifact::ExpectedForeignSource {
            artifact: &self.foreign_source,
            formal: &self.foreign_formal,
            bytes: self.model.source().source().text().as_bytes(),
        };
        let models = [artifact::AdmittedModel {
            artifact: &self.package.dependencies[self.package.models[0].artifact as usize].artifact,
            model: &self.model,
            source: &foreign,
        }];
        let expected = artifact::Expected {
            artifact: selected,
            contract: &self.package.contract,
            baseline: &self.package.baseline,
            producer: &self.package.producer,
            language: &self.package.language,
            sources: &sources,
            dependencies: &dependencies,
            models: &models,
        };
        artifact::read(bytes, &expected, limits)
    }

    /// Select adverse bytes explicitly so tests reach validation past the seal;
    /// all producer/source/dependency selections still come from the baseline.
    pub fn offered(&self, package: &w::Package) -> artifact::Report<artifact::AdmittedPackage> {
        let bytes = serde_json::to_vec(package).unwrap();
        self.read(&bytes, &Self::seal(&bytes), artifact::Limits::default())
    }

    pub fn json(&self) -> serde_json::Value {
        serde_json::to_value(&self.package).unwrap()
    }

    pub fn raw(&self, value: &serde_json::Value) -> artifact::Report<artifact::AdmittedPackage> {
        let bytes = serde_json::to_vec(value).unwrap();
        self.read(&bytes, &Self::seal(&bytes), artifact::Limits::default())
    }
}

pub fn rational(numerator: &str, denominator: &str) -> serde_json::Value {
    json!({"kind":"rational", "numerator":numerator, "denominator":denominator})
}
