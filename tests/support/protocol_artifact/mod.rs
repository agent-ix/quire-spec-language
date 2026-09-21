// SPDX-License-Identifier: AGPL-3.0-or-later
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

/// Synthetic, deliberately-not-the-real-standard-text bytes for a registered
/// definition's supplied artifact. QSL recognizes a definition by identity,
/// never by comparing its bytes to any particular snapshot (PLAT-887).
fn definition_bytes(definition: R) -> &'static [u8] {
    definition.identity().as_bytes()
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
        let mut fixture = Self::with_definitions(&[]);
        // Authored input and finish binders retain distinct observations.
        fixture.population_requirements(&[(0, 0), (0, 2)]);
        fixture
    }

    fn with_definitions(additional: &[R]) -> Self {
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
        let mut registry = vec![
            R::Edition,
            R::StateCore,
            R::StateQueries,
            R::StateGraph,
            R::TemporalFacet,
            R::Protocol,
            R::Package,
            R::Range,
            R::ObservationBinding,
            R::Progress,
        ];
        registry.extend_from_slice(additional);
        registry.sort_by_key(|definition| definition.identity());
        let mut inputs = BTreeMap::<String, (w::ArtifactRef, Vec<u8>)>::new();
        for definition in &registry {
            let mut selected = reference(
                w::ArtifactKind::Source,
                definition.path(),
                "text/markdown",
                "1",
                definition_bytes(*definition),
            );
            selected.revision = revision(definition.revision());
            inputs.insert(
                definition.path().into(),
                (selected, definition_bytes(*definition).to_vec()),
            );
            for rule in definition.rules() {
                inputs.entry(rule.path.into()).or_insert_with(|| {
                    let bytes = rule.path.as_bytes();
                    (
                        reference(
                            w::ArtifactKind::Source,
                            rule.path,
                            "text/markdown",
                            "1",
                            bytes,
                        ),
                        bytes.to_vec(),
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
                ByteDigest::of(definition_bytes(selected))
            ));
        }
        if !additional.is_empty() {
            for (alias, selected) in [("Q", R::StateQueries), ("T", R::EventPosition)] {
                text.push_str(&format!(
                    "profile {alias} = \"{}\" version \"{}\" digest \"{}\";\n",
                    selected.identity(),
                    selected.revision(),
                    ByteDigest::of(definition_bytes(selected))
                ));
            }
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
                exports: vec![
                    w::Export {
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
                    },
                    w::Export {
                        kind: w::ExportKind::Population,
                        path: vec!["Node".into(), "nodes".into()],
                        locus: w::ForeignLocus {
                            source: foreign_source.clone(),
                            formal: foreign_formal.clone(),
                            span: w::Span {
                                start: object_span.start as u32,
                                end: object_span.end as u32,
                            },
                        },
                    },
                ],
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

    pub fn definition(&self, selected: R) -> u32 {
        self.package
            .definitions
            .iter()
            .position(|value| value.identity == selected.identity())
            .unwrap() as u32
    }

    pub fn export(&self, kind: w::ExportKind, path: &[&str]) -> w::ExportRef {
        w::ExportRef {
            model: 0,
            export: self.package.models[0]
                .exports
                .iter()
                .position(|value| {
                    value.kind == kind
                        && value
                            .path
                            .iter()
                            .map(String::as_str)
                            .eq(path.iter().copied())
                })
                .unwrap() as u32,
        }
    }

    /// Explicit expected Node populations for this fixture's authored input sites.
    fn population_requirements(&mut self, observations: &[(u32, u32)]) {
        let population = self.export(w::ExportKind::Population, &["Node", "nodes"]);
        let observation =
            self.package.definitions[self.definition(R::ObservationBinding) as usize].artifact;
        let progress = self.package.definitions[self.definition(R::Progress) as usize].artifact;
        for &(owner, anchor) in observations {
            let declaration = &mut self.package.declarations[owner as usize];
            let required = declaration.anchors[anchor as usize]
                .binding
                .0
                .into_iter()
                .collect();
            let population_index = declaration.bindings.len() as u32;
            for (kind, contract, requires) in [
                (w::BindingKind::Population, observation, required),
                (w::BindingKind::Closure, progress, vec![population_index]),
            ] {
                declaration.bindings.push(w::BindingRequirement {
                    name: format!("Node:nodes:{anchor}:{}", kind.as_str()),
                    kind,
                    value_type: w::Nullable(Some(0)),
                    authority: self.package.dependencies[contract as usize]
                        .artifact
                        .clone(),
                    contract,
                    model: w::Nullable(Some(population.clone())),
                    subject: w::Subject::Declaration { declaration: owner },
                    anchor: owned(owner, anchor),
                    scope: owned(owner, 0),
                    relation: w::Nullable(None),
                    requires,
                    locus: declaration.locus.clone(),
                });
            }
        }
    }

    fn enrich_exports(&mut self) {
        let foreign = |source: &ir::SourceSpan| {
            let span = self.model.source().to_native(source).unwrap();
            w::ForeignLocus {
                source: self.foreign_source.clone(),
                formal: self.foreign_formal.clone(),
                span: w::Span {
                    start: span.start as u32,
                    end: span.end as u32,
                },
            }
        };
        let exports = &mut self.package.models[0].exports;
        for scalar in &self.model.roles().scalars {
            if ["Signed", "Count", "Version", "ObjectId"].contains(&scalar.name.as_str()) {
                exports.push(w::Export {
                    kind: w::ExportKind::Scalar,
                    path: vec![scalar.name.as_str().to_owned()],
                    locus: foreign(&scalar.source),
                });
            }
        }
        for declaration in self.model.environment().types() {
            if let ir::TypeDeclaration::Record { declaration } = declaration {
                if declaration.name().as_str() == "Node" {
                    for field in declaration.fields() {
                        if ["signed", "items"].contains(&field.name().as_str()) {
                            exports.push(w::Export {
                                kind: w::ExportKind::Field,
                                path: vec!["Node".into(), field.name().as_str().to_owned()],
                                locus: foreign(field.source()),
                            });
                        }
                    }
                }
            }
        }
        let operation = &self.model.roles().operations[0];
        exports.push(w::Export {
            kind: w::ExportKind::Operation,
            path: vec!["Node".into(), "step".into()],
            locus: foreign(&operation.source),
        });
        exports.sort_by(|a, b| (a.kind.as_str(), &a.path).cmp(&(b.kind.as_str(), &b.path)));
        let object = self.export(w::ExportKind::Object, &["Node"]);
        self.package.types[0] = w::Type::Object {
            export: object.clone(),
        };
        for binding in &mut self.package.declarations[0].bindings {
            binding.model = w::Nullable(Some(object.clone()));
        }
    }

    fn append(&mut self, name: &str, text: &str, profile: R) -> w::Declaration {
        let mut declaration = self.package.declarations[0].clone();
        let source = &mut self.package.sources[0];
        let start = source.text.len();
        source.text.push_str(text);
        source.text.push('\n');
        source.artifact.digest = ByteDigest::of(source.text.as_bytes());
        declaration.name = name.into();
        declaration.requirement.identity = name.into();
        declaration.locus.span = w::Span {
            start: start as u32,
            end: (start + text.len()) as u32,
        };
        declaration.profile = self.definition(profile);
        declaration.requires.clear();
        declaration.scopes = vec![w::Scope {
            parent: w::Nullable(None),
            locus: declaration.locus.clone(),
        }];
        declaration.anchors = vec![w::Anchor {
            kind: w::AnchorKind::Current,
            owner: w::Nullable(None),
            binding: w::Nullable(None),
            locus: declaration.locus.clone(),
        }];
        declaration.binders.clear();
        declaration.values.clear();
        declaration.temporal.clear();
        declaration.bindings.clear();
        declaration
    }

    /// Predicate arithmetic/lexical values, an exact field/callee state clause,
    /// and a concrete temporal profile all use the same real producer exports.
    pub fn families() -> Self {
        let mut fixture = Self::with_definitions(&[R::EventPosition]);
        fixture.enrich_exports();
        fixture.package.types.push(w::Type::Scalar {
            export: fixture.export(w::ExportKind::Scalar, &["Signed"]),
            unit: w::Nullable(None),
            representation: w::Representation::Integer {
                minimum: integer(-10),
                maximum: integer(10),
            },
        });
        const PREDICATE: &str = "predicate Advance using Q (x: M::Signed): Boolean { let prior: M::Signed = 1 in if true then (prior + 1 <= 10) else -x <= 10 }";
        let mut predicate = fixture.append("Advance", PREDICATE, R::StateQueries);
        predicate.anchors[0].kind = w::AnchorKind::Predicate;
        predicate.binders = vec![
            binder(1, &predicate, "x", w::BinderKind::Parameter, 2),
            binder(1, &predicate, "prior", w::BinderKind::Let, 2),
        ];
        predicate.binders[1].initializer = w::Nullable(Some(owned(1, 1)));
        let base = predicate.locus.span.start as usize;
        let at = |text: &str| base + PREDICATE.find(text).unwrap();
        let h = |index| owned(1, index);
        let body = "let prior: M::Signed = 1 in if true then (prior + 1 <= 10) else -x <= 10";
        let conditional = "if true then (prior + 1 <= 10) else -x <= 10";
        predicate.values = vec![
            value(
                1,
                &predicate,
                (at(body), body.len()),
                1,
                w::ValueOperation::Let {
                    binder: h(1),
                    initializer: h(1),
                    body: h(2),
                },
            ),
            value(
                1,
                &predicate,
                (at("= 1") + 2, 1),
                2,
                w::ValueOperation::Number { value: number(1) },
            ),
            value(
                1,
                &predicate,
                (at(conditional), conditional.len()),
                1,
                w::ValueOperation::If {
                    condition: h(3),
                    then_value: h(4),
                    else_value: h(11),
                },
            ),
            value(
                1,
                &predicate,
                (at("true"), 4),
                1,
                w::ValueOperation::Boolean { value: true },
            ),
            value(
                1,
                &predicate,
                (at("(prior + 1 <= 10)"), 17),
                1,
                w::ValueOperation::Group { value: h(7) },
            ),
            value(
                1,
                &predicate,
                (at("prior + 1"), 5),
                2,
                w::ValueOperation::Read { binder: h(1) },
            ),
            value(
                1,
                &predicate,
                (at("prior + 1"), 9),
                2,
                w::ValueOperation::Binary {
                    operator: w::Binary::Add,
                    left: h(5),
                    right: h(8),
                },
            ),
            value(
                1,
                &predicate,
                (at("prior + 1 <= 10"), 15),
                1,
                w::ValueOperation::Binary {
                    operator: w::Binary::LessEqual,
                    left: h(6),
                    right: h(9),
                },
            ),
            value(
                1,
                &predicate,
                (at("prior + 1") + 8, 1),
                2,
                w::ValueOperation::Number { value: number(1) },
            ),
            value(
                1,
                &predicate,
                (at("prior + 1 <= 10") + 13, 2),
                2,
                w::ValueOperation::Number { value: number(10) },
            ),
            value(
                1,
                &predicate,
                (at("-x"), 2),
                2,
                w::ValueOperation::Unary {
                    operator: w::Unary::Negate,
                    value: h(12),
                },
            ),
            value(
                1,
                &predicate,
                (at("-x <= 10"), 8),
                1,
                w::ValueOperation::Binary {
                    operator: w::Binary::LessEqual,
                    left: h(10),
                    right: h(13),
                },
            ),
            value(
                1,
                &predicate,
                (at("-x") + 1, 1),
                2,
                w::ValueOperation::Read { binder: h(0) },
            ),
            value(
                1,
                &predicate,
                (at("-x <= 10") + 6, 2),
                2,
                w::ValueOperation::Number { value: number(10) },
            ),
        ];
        identify(&mut predicate.values);
        predicate.body = w::Body::Predicate {
            parameters: vec![h(0)],
            result: 1,
            root: h(0),
        };
        fixture.package.declarations.push(predicate);
        const STATE: &str =
            "invariant Bounded using Q on M::Node at current { Advance(self.signed) }";
        let mut state = fixture.append("Bounded", STATE, R::StateQueries);
        state
            .binders
            .push(binder(2, &state, "self", w::BinderKind::SelfValue, 0));
        state.requires = vec![1];
        state.values = fixture.predicate_call(2, &state, STATE, "self");
        state.body = w::Body::State {
            clause_kind: w::ClauseKind::Invariant,
            context: fixture.export(w::ExportKind::Object, &["Node"]),
            operation: w::Nullable(None),
            root: owned(2, 0),
        };
        fixture.package.declarations.push(state);
        const TEMPORAL: &str = "temporal Due using T over (view: M::Node) clock \"ticks\" on origin { eventually[0,1] holds(Advance(view.signed)) }";
        let mut temporal = fixture.append("Due", TEMPORAL, R::EventPosition);
        temporal.anchors[0].kind = w::AnchorKind::TemporalInstant;
        temporal.anchors.push(w::Anchor {
            kind: w::AnchorKind::Activation,
            owner: w::Nullable(None),
            binding: w::Nullable(None),
            locus: temporal.locus.clone(),
        });
        temporal
            .binders
            .push(binder(3, &temporal, "view", w::BinderKind::Input, 0));
        temporal.requires = vec![1];
        temporal.values = fixture.predicate_call(3, &temporal, TEMPORAL, "view");
        let mut clock = fixture.package.declarations[0].bindings[0].clone();
        clock.name = "ticks".into();
        clock.kind = w::BindingKind::Clock;
        clock.value_type = w::Nullable(None);
        clock.model = w::Nullable(None);
        clock.subject = w::Subject::Declaration { declaration: 3 };
        clock.anchor = owned(3, 0);
        clock.scope = owned(3, 0);
        clock.locus = temporal.locus.clone();
        temporal.bindings.push(clock);
        let start = temporal.locus.span.start as usize;
        let node = |text: &str| w::Locus {
            source: 0,
            span: w::Span {
                start: (start + TEMPORAL.find(text).unwrap()) as u32,
                end: (start + TEMPORAL.len() - 2) as u32,
            },
        };
        temporal.temporal = vec![
            w::Temporal {
                original_node: 1,
                locus: node("eventually"),
                operation: w::TemporalOperation::Unary {
                    operator: w::TemporalUnary::Eventually,
                    interval: w::Nullable(Some(w::Interval {
                        lower: integer(0),
                        upper: integer(1),
                    })),
                    value: owned(3, 1),
                },
            },
            w::Temporal {
                original_node: 0,
                locus: node("holds"),
                operation: w::TemporalOperation::Holds { value: owned(3, 0) },
            },
        ];
        temporal.body = w::Body::Temporal {
            input: owned(3, 0),
            clock: 0,
            activation: w::Activation::Origin {
                anchor: owned(3, 1),
            },
            captures: vec![],
            root: owned(3, 0),
        };
        fixture.package.declarations.push(temporal);
        fixture.package.types.push(w::Type::Scalar {
            export: fixture.export(w::ExportKind::Scalar, &["ObjectId"]),
            unit: w::Nullable(None),
            representation: w::Representation::Text {
                maximum_scalars: integer(256),
            },
        });
        const TEXT: &str =
            "predicate Label using Q (label: M::ObjectId): Boolean { label = \"é\" }";
        let mut text = fixture.append("Label", TEXT, R::StateQueries);
        text.anchors[0].kind = w::AnchorKind::Predicate;
        text.binders
            .push(binder(4, &text, "label", w::BinderKind::Parameter, 3));
        let base = text.locus.span.start as usize;
        let at = base + TEXT.find("label =").unwrap();
        text.values = vec![
            value(
                4,
                &text,
                (at, 5),
                3,
                w::ValueOperation::Read {
                    binder: owned(4, 0),
                },
            ),
            value(
                4,
                &text,
                (at, "label = \"é\"".len()),
                1,
                w::ValueOperation::Binary {
                    operator: w::Binary::Equal,
                    left: owned(4, 0),
                    right: owned(4, 2),
                },
            ),
            value(
                4,
                &text,
                (base + TEXT.find("\"é\"").unwrap(), 4),
                3,
                w::ValueOperation::Text { value: "é".into() },
            ),
        ];
        identify(&mut text.values);
        text.body = w::Body::Predicate {
            parameters: vec![owned(4, 0)],
            result: 1,
            root: owned(4, 1),
        };
        fixture.package.declarations.push(text);
        fixture.package.features.declarations = [
            "declaration.predicate",
            "family.protocol",
            "family.state",
            "family.temporal",
        ]
        .map(str::to_owned)
        .to_vec();
        fixture
            .package
            .features
            .required
            .push("quire.protocol.temporal/1".into());
        fixture.package.features.required.sort();
        // Protocol input/finish, current state self, and temporal input.
        fixture.population_requirements(&[(0, 0), (0, 2), (2, 0), (3, 0)]);
        fixture
    }

    fn predicate_call(
        &self,
        owner: u32,
        declaration: &w::Declaration,
        text: &str,
        input: &str,
    ) -> Vec<w::Value> {
        let start = declaration.locus.span.start as usize;
        let call = format!("Advance({input}.signed)");
        let selected = format!("{input}.signed");
        let at = |needle: &str| start + text.find(needle).unwrap();
        let h = |index| owned(owner, index);
        let mut values = vec![
            value(
                owner,
                declaration,
                (at(&call), call.len()),
                1,
                w::ValueOperation::Call {
                    predicate: 1,
                    arguments: vec![h(2)],
                },
            ),
            value(
                owner,
                declaration,
                (at(&selected), input.len()),
                0,
                w::ValueOperation::Read { binder: h(0) },
            ),
            value(
                owner,
                declaration,
                (at(&selected), selected.len()),
                2,
                w::ValueOperation::Field {
                    base: h(1),
                    field: self.export(w::ExportKind::Field, &["Node", "signed"]),
                },
            ),
        ];
        identify(&mut values);
        values
    }

    /// Finite control wire records using an actual Node operation/role view.
    /// This fixture does not establish choice totality or runtime observations.
    pub fn controlled() -> Self {
        const ATTEMPT: &str =
            "attempt Tried by Service on M::Node::step contracts [] as (tried: M::Node) { true };";
        const EFFECT: &str = "effect Applied of Tried as (applied: M::Node) { true };";
        const ACCEPT: &str = "check Accepted using S { true };";
        const DECLINE: &str = "check Declined using S { true };";
        const CHOICE: &str = "choice Select by Service visible (true) { case yes when { true } check Accepted using S { true }; case no when { false } check Declined using S { true }; }";
        const PULSE: &str = "event Pulse by Service as (pulse: M::Node) { true };";
        const LIMIT: &str = "check Limit using S { true };";
        const REPEAT: &str = "repeat Retry by Service visible (false) max 0 while { false } event Pulse by Service as (pulse: M::Node) { true }; exhausted check Limit using S { true };";
        const MATCHED: &str = "event Matched by Service as (matched: M::Node) { true };";
        const GOT: &str = "check Got using S { true };";
        const EXPIRED: &str = "check Expired using S { true };";
        const AWAIT: &str = "await Wait after Applied using T clock \"ticks\" within [0,1] match event Matched by Service as (matched: M::Node) { true }; then check Got using S { true }; timeout check Expired using S { true };";
        const COMMIT: &str = "commit Committed by Service as (committed: M::Node) { true };";
        const FINISH: &str = "finish Closed as (closed: M::Node) { true };";
        let mut fixture = Self::with_definitions(&[R::EventPosition]);
        fixture.enrich_exports();
        let state = fixture.definition(R::StateCore);
        let temporal = fixture.definition(R::EventPosition);
        let object = fixture.export(w::ExportKind::Object, &["Node"]);
        let operation = fixture.export(w::ExportKind::Operation, &["Node", "step"]);
        let run = format!(
            "sequence Main {{\n{ATTEMPT}\n{EFFECT}\n{CHOICE}\n{REPEAT}\n{AWAIT}\n{COMMIT}\n}}"
        );
        let declaration = &mut fixture.package.declarations[0];
        let start = declaration.locus.span.start as usize;
        let source = &mut fixture.package.sources[0];
        source.text.truncate(start);
        source.text.push_str(&format!("protocol Demo using P over (view: M::Node) on origin {{\nrole Service on M::Node;\nrun {run}\n{FINISH}\n}}\n"));
        source.artifact.digest = ByteDigest::of(source.text.as_bytes());
        declaration.locus.span.end = source.text.len() as u32;
        let whole = declaration.locus.clone();
        let locus = |text: &str| {
            let at = source.text.find(text).unwrap();
            w::Locus {
                source: 0,
                span: w::Span {
                    start: at as u32,
                    end: (at + text.len()) as u32,
                },
            }
        };
        declaration.scopes[0].locus = whole.clone();
        for anchor in &mut declaration.anchors {
            anchor.locus = whole.clone();
        }
        for binding in &mut declaration.bindings {
            binding.locus = whole.clone();
        }
        let mut boolean_sites: Vec<_> = ["true", "false"]
            .into_iter()
            .flat_map(|token| {
                source.text[start..]
                    .match_indices(token)
                    .map(move |(at, _)| (start + at, token))
            })
            .collect();
        boolean_sites.sort_unstable();
        declaration.values = boolean_sites
            .iter()
            .map(|(at, token)| {
                let mut value = value(
                    0,
                    declaration,
                    (*at, token.len()),
                    1,
                    w::ValueOperation::Boolean {
                        value: *token == "true",
                    },
                );
                value.profile = state;
                value.anchor = handle(1);
                value
            })
            .collect();
        identify(&mut declaration.values);
        let boolean = |within: &str, token: &str| {
            let outer = source.text.find(within).unwrap();
            let at = outer + within.find(token).unwrap();
            handle(
                boolean_sites
                    .iter()
                    .position(|(site, _)| *site == at)
                    .unwrap() as u32,
            )
        };
        declaration.binders.clear();
        declaration
            .binders
            .push(binder(0, declaration, "view", w::BinderKind::Input, 0));
        for (name, snippet) in [
            ("tried", ATTEMPT),
            ("applied", EFFECT),
            ("pulse", PULSE),
            ("matched", MATCHED),
            ("committed", COMMIT),
        ] {
            let mut selected = binder(0, declaration, name, w::BinderKind::Event, 0);
            selected.locus = locus(snippet);
            selected.anchor = handle(1);
            declaration.binders.push(selected);
        }
        let mut finish_binder = binder(0, declaration, "closed", w::BinderKind::Finish, 0);
        finish_binder.locus = locus(FINISH);
        finish_binder.anchor = handle(2);
        declaration.binders.push(finish_binder);
        let template = declaration.bindings[0].clone();
        for (name, kind, subject) in [
            (
                "service",
                w::BindingKind::RoleInstance,
                w::Subject::Role { role: handle(0) },
            ),
            (
                "ticks",
                w::BindingKind::Clock,
                w::Subject::Declaration { declaration: 0 },
            ),
            (
                "attempt",
                w::BindingKind::Attempt,
                w::Subject::Control { control: handle(1) },
            ),
            (
                "effect",
                w::BindingKind::Effect,
                w::Subject::Control { control: handle(2) },
            ),
            (
                "pulse",
                w::BindingKind::Invocation,
                w::Subject::Control { control: handle(7) },
            ),
            (
                "matched",
                w::BindingKind::Invocation,
                w::Subject::Control {
                    control: handle(10),
                },
            ),
            (
                "commit",
                w::BindingKind::Commit,
                w::Subject::Control {
                    control: handle(13),
                },
            ),
        ] {
            let mut binding = template.clone();
            binding.name = name.into();
            binding.kind = kind;
            binding.subject = subject;
            if kind == w::BindingKind::Clock {
                binding.model = w::Nullable(None);
                binding.value_type = w::Nullable(None);
            }
            declaration.bindings.push(binding);
        }
        let mut deadline = template.clone();
        deadline.name = "await-completeness".into();
        deadline.kind = w::BindingKind::Progress;
        deadline.value_type = w::Nullable(None);
        deadline.model = w::Nullable(None);
        deadline.subject = w::Subject::Control { control: handle(9) };
        deadline.requires = vec![4];
        declaration.bindings.push(deadline);
        let control = |name: &str, text: &str, original_node, operation| w::Control {
            name: name.into(),
            original_node,
            locus: locus(text),
            operation,
        };
        let check = |text| w::ControlOperation::Check {
            profile: state,
            value: boolean(text, "true"),
        };
        let event = |event, binder, text| w::ControlOperation::Event {
            event,
            binder: handle(binder),
            related: vec![],
            constraint: boolean(text, "true"),
        };
        let controls = vec![
            control(
                "Main",
                &run,
                13,
                w::ControlOperation::Sequence {
                    children: [1, 2, 3, 6, 9, 13].map(handle).to_vec(),
                },
            ),
            control(
                "Tried",
                ATTEMPT,
                0,
                event(
                    w::Event::Attempt {
                        owner: handle(0),
                        operation,
                        contracts: vec![],
                        instance: 5,
                    },
                    1,
                    ATTEMPT,
                ),
            ),
            control(
                "Applied",
                EFFECT,
                1,
                event(
                    w::Event::Effect {
                        attempt: handle(1),
                        instance: 6,
                    },
                    2,
                    EFFECT,
                ),
            ),
            control(
                "Select",
                CHOICE,
                4,
                w::ControlOperation::Choice {
                    owner: handle(0),
                    visible: vec![boolean("visible (true)", "true")],
                    cases: vec![
                        w::ChoiceCase {
                            label: "yes".into(),
                            guard: boolean("case yes when { true }", "true"),
                            body: handle(4),
                            locus: locus(ACCEPT),
                        },
                        w::ChoiceCase {
                            label: "no".into(),
                            guard: boolean("case no when { false }", "false"),
                            body: handle(5),
                            locus: locus(DECLINE),
                        },
                    ],
                },
            ),
            control("Accepted", ACCEPT, 2, check(ACCEPT)),
            control("Declined", DECLINE, 3, check(DECLINE)),
            control(
                "Retry",
                REPEAT,
                7,
                w::ControlOperation::Repeat {
                    owner: handle(0),
                    visible: vec![boolean("visible (false)", "false")],
                    maximum: integer(0),
                    guard: boolean("while { false }", "false"),
                    body: handle(7),
                    exhausted: handle(8),
                },
            ),
            control(
                "Pulse",
                PULSE,
                5,
                event(
                    w::Event::Event {
                        owner: handle(0),
                        compensation: w::Nullable(None),
                        instance: 7,
                    },
                    3,
                    PULSE,
                ),
            ),
            control("Limit", LIMIT, 6, check(LIMIT)),
            control(
                "Wait",
                AWAIT,
                11,
                w::ControlOperation::Await {
                    after: w::AwaitAnchor::Event { node: handle(2) },
                    profile: temporal,
                    clock: 4,
                    within: w::Interval {
                        lower: integer(0),
                        upper: integer(1),
                    },
                    event: handle(10),
                    then_body: handle(11),
                    timeout: handle(12),
                },
            ),
            control(
                "Matched",
                MATCHED,
                8,
                event(
                    w::Event::Event {
                        owner: handle(0),
                        compensation: w::Nullable(None),
                        instance: 8,
                    },
                    4,
                    MATCHED,
                ),
            ),
            control("Got", GOT, 9, check(GOT)),
            control("Expired", EXPIRED, 10, check(EXPIRED)),
            control(
                "Committed",
                COMMIT,
                12,
                w::ControlOperation::Commit {
                    owner: handle(0),
                    binder: handle(5),
                    constraint: boolean(COMMIT, "true"),
                    instance: 9,
                },
            ),
        ];
        use w::{
            EdgeKind::{AwaitSuccess, AwaitTimeout, Branch, Join, RepeatProgress, Sequence},
            Port::{Enter, Exit},
        };
        // Literal edge vectors, independently expanded from the five contract rows.
        let edges = [
            (0, Sequence, 0, Enter, 1, Enter),
            (0, Sequence, 1, Exit, 2, Enter),
            (0, Sequence, 2, Exit, 3, Enter),
            (0, Sequence, 3, Exit, 6, Enter),
            (0, Sequence, 6, Exit, 9, Enter),
            (0, Sequence, 9, Exit, 13, Enter),
            (0, Sequence, 13, Exit, 0, Exit),
            (1, Sequence, 1, Enter, 1, Exit),
            (2, Sequence, 1, Exit, 2, Enter),
            (2, Sequence, 2, Enter, 2, Exit),
            (3, Branch, 3, Enter, 4, Enter),
            (3, Branch, 3, Enter, 5, Enter),
            (3, Join, 4, Exit, 3, Exit),
            (3, Join, 5, Exit, 3, Exit),
            (4, Sequence, 4, Enter, 4, Exit),
            (5, Sequence, 5, Enter, 5, Exit),
            (6, Branch, 6, Enter, 6, Exit),
            (6, Branch, 6, Enter, 7, Enter),
            (6, Branch, 6, Enter, 8, Enter),
            (6, Join, 8, Exit, 6, Exit),
            (6, RepeatProgress, 7, Exit, 6, Enter),
            (7, Sequence, 7, Enter, 7, Exit),
            (8, Sequence, 8, Enter, 8, Exit),
            (9, AwaitSuccess, 9, Enter, 10, Enter),
            (9, AwaitSuccess, 10, Exit, 11, Enter),
            (9, AwaitTimeout, 9, Enter, 12, Enter),
            (9, Join, 11, Exit, 9, Exit),
            (9, Join, 12, Exit, 9, Exit),
            (9, Sequence, 2, Exit, 9, Enter),
            (10, Sequence, 10, Enter, 10, Exit),
            (11, Sequence, 11, Enter, 11, Exit),
            (12, Sequence, 12, Enter, 12, Exit),
            (13, Sequence, 13, Enter, 13, Exit),
        ];
        let mut causal_edges: Vec<_> = edges
            .into_iter()
            .map(
                |(owner, kind, from, from_port, to, to_port)| w::CausalEdge {
                    owner: handle(owner),
                    kind,
                    from: w::Endpoint {
                        node: handle(from),
                        port: from_port,
                    },
                    to: w::Endpoint {
                        node: handle(to),
                        port: to_port,
                    },
                    maximum: w::Nullable((kind == RepeatProgress).then(|| integer(0))),
                },
            )
            .collect();
        causal_edges.sort_by(|a, b| {
            (
                a.owner.index,
                a.kind.as_str(),
                a.from.node.index,
                a.from.port.as_str(),
                a.to.node.index,
                a.to.port.as_str(),
            )
                .cmp(&(
                    b.owner.index,
                    b.kind.as_str(),
                    b.from.node.index,
                    b.from.port.as_str(),
                    b.to.node.index,
                    b.to.port.as_str(),
                ))
        });
        declaration.values[boolean(FINISH, "true").index as usize].anchor = handle(2);
        declaration.body = w::Body::Protocol {
            input: handle(0),
            activation: w::Activation::Origin { anchor: handle(0) },
            captures: vec![],
            roles: vec![w::Role {
                name: "Service".into(),
                model: object,
                instance: 3,
                locus: locus("role Service on M::Node;"),
            }],
            relationships: vec![],
            channels: vec![],
            compensations: vec![],
            temporal_requirements: vec![],
            controls,
            causal_edges,
            run: handle(0),
            finish: Box::new(w::Finish {
                name: "Closed".into(),
                binder: handle(6),
                constraint: boolean(FINISH, "true"),
                closure: 2,
                locus: locus(FINISH),
            }),
        };
        // Input, all event-record views, and finish retain three observations.
        fixture.population_requirements(&[(0, 0), (0, 1), (0, 2)]);
        fixture
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
        let declarations: Vec<_> = self
            .package
            .declarations
            .iter()
            .map(|declaration| artifact::ExpectedDeclaration {
                name: &declaration.name,
                span: &declaration.locus.span,
                requirement: &declaration.requirement,
                clause: &declaration.clause,
                execution: &declaration.execution,
            })
            .collect();
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

pub fn owned(declaration: u32, index: u32) -> w::Handle {
    w::Handle { declaration, index }
}
pub fn integer(value: i64) -> w::Integer {
    w::Integer(artifact::NumberWire::Integer {
        decimal: value.to_string(),
    })
}
fn number(value: i64) -> w::Number {
    w::Number(artifact::NumberWire::Integer {
        decimal: value.to_string(),
    })
}
fn binder(
    owner: u32,
    declaration: &w::Declaration,
    name: &str,
    kind: w::BinderKind,
    value_type: u32,
) -> w::Binder {
    w::Binder {
        name: name.into(),
        kind,
        value_type,
        scope: owned(owner, 0),
        anchor: owned(owner, 0),
        initializer: w::Nullable(None),
        locus: declaration.locus.clone(),
    }
}
fn value(
    owner: u32,
    declaration: &w::Declaration,
    (start, length): (usize, usize),
    value_type: u32,
    operation: w::ValueOperation,
) -> w::Value {
    w::Value {
        original_expression: 0,
        locus: w::Locus {
            source: 0,
            span: w::Span {
                start: start as u32,
                end: (start + length) as u32,
            },
        },
        operator_locus: w::Nullable(None),
        value_type,
        profile: declaration.profile,
        scope: owned(owner, 0),
        anchor: owned(owner, 0),
        origin: w::Origin::Independent {},
        operation,
    }
}
fn identify(values: &mut [w::Value]) {
    for (index, value) in values.iter_mut().enumerate() {
        value.original_expression = index as u32;
    }
}
