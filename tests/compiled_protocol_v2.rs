// SPDX-License-Identifier: AGPL-3.0-only
//! TC-138: strict compiled-protocol v2 production, admission and L5 handoff.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::linking::composed::binding_work::{
    Dimension as BindingDimension, Exhaustion as BindingExhaustion,
};
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::linking::composed::producer::ProducerModelRefusal;
use quire_spec_language::protocol_artifact::{
    self as artifact,
    handoff::{
        MUTATION_MANIFEST_FORMAT, PUBLISHED_CHECKSUMS_FILE, PUBLISHED_HANDOFF,
        PUBLISHED_MUTATION_MANIFEST_FILE, PUBLISHED_SELECTION_FILE,
    },
    native, v2, wire as w, Dimension as WorkDimension, Error, ExactInteger, Invalid, Limits,
    NumberComponent, NumberError, NumberWire, ProtocolNumber, Unsupported,
};
use quire_spec_language::state::{
    self, AssessmentAuthority, AuthorityAdapter, AuthorityEvidence, BinderInput, CanonicalDigest,
    ContextualSlot, Dimension as StateDimension, EvaluationOutcome, EvaluationRequest, FieldInput,
    FieldValue, InputSlot, Limits as StateLimits, MissingInput, ObjectInput, ObjectKey,
    ObservationDigest, ObservationIdentity, ObservationKey, PopulationInput,
    Refusal as StateRefusal, StateView, StaticAuthority, Value as StateValue,
    ValueKind as StateValueKind, OBSERVATION_CONTRACT_REVISION, PRODUCER_CONTRACT_REVISION,
};
use quire_spec_language::temporal;
use quire_spec_language::ByteDigest;
use serde_json::Value;
use setup::{Inputs, TemporalDefinitionExpectation, TemporalExpectation, Unit};
use sha2::{Digest as _, Sha256};

const DECLARATIONS: [(&str, R); 3] = [
    ("ByEvent", R::EventPosition),
    ("BySample", R::FixedSample),
    ("ByTimestamp", R::TimestampedWindow),
];

fn handoff_files(root: &Path, directory: &Path, files: &mut BTreeSet<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read committed handoff directory") {
        let entry = entry.expect("read committed handoff entry");
        let path = entry.path();
        if entry
            .file_type()
            .expect("read committed handoff file type")
            .is_dir()
        {
            handoff_files(root, &path, files);
        } else {
            let relative = path
                .strip_prefix(root)
                .expect("handoff entry remains below its root")
                .to_owned();
            if relative != Path::new(PUBLISHED_CHECKSUMS_FILE) {
                assert!(files.insert(relative), "duplicate handoff file");
            }
        }
    }
}

#[trace("TC-138", "FR-050-AC-1", "FR-050-AC-4")]
#[test]
fn committed_handoff_checksums_and_interchange_records_are_complete() {
    let root = Path::new(PUBLISHED_HANDOFF);
    let sums =
        fs::read_to_string(root.join(PUBLISHED_CHECKSUMS_FILE)).expect("committed SHA256SUMS");
    let mut listed = BTreeSet::new();

    for (line_index, line) in sums.lines().enumerate() {
        let (expected_digest, relative) = line
            .split_once("  ./")
            .unwrap_or_else(|| panic!("malformed SHA256SUMS line {}", line_index + 1));
        assert_eq!(expected_digest.len(), 64, "SHA-256 width");
        assert!(
            expected_digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "SHA-256 must use lowercase hexadecimal"
        );
        let relative = PathBuf::from(relative);
        assert!(
            relative
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
            "checksum path must remain relative and normalized"
        );
        assert!(
            listed.insert(relative.clone()),
            "duplicate checksum path {relative:?}"
        );
        let bytes = fs::read(root.join(&relative)).expect("read checksummed handoff file");
        assert_eq!(format!("{:x}", ByteDigest::of(&bytes)), expected_digest);
    }

    let mut actual = BTreeSet::new();
    handoff_files(root, root, &mut actual);
    assert_eq!(listed, actual, "SHA256SUMS must cover every handoff file");

    let selection: artifact::handoff::SelectionV2 = serde_json::from_slice(
        &fs::read(root.join(PUBLISHED_SELECTION_FILE)).expect("committed reader selection"),
    )
    .expect("decode expected-v2.json with the published type");
    assert_eq!(selection.temporal.len(), DECLARATIONS.len());
    let selection_bytes = serde_json::to_vec(&selection).expect("encode published selection type");
    assert_eq!(
        serde_json::from_slice::<artifact::handoff::SelectionV2>(&selection_bytes)
            .expect("round-trip published selection type"),
        selection
    );

    let manifest: artifact::handoff::MutationManifest = serde_json::from_slice(
        &fs::read(root.join(PUBLISHED_MUTATION_MANIFEST_FILE))
            .expect("committed mutation manifest"),
    )
    .expect("decode mutations/manifest.json with the published type");
    assert_eq!(manifest.format, MUTATION_MANIFEST_FORMAT);
    assert_eq!(manifest.cases.len(), 28);
    let manifest_bytes =
        serde_json::to_vec(&manifest).expect("encode published mutation-manifest type");
    assert_eq!(
        serde_json::from_slice::<artifact::handoff::MutationManifest>(&manifest_bytes)
            .expect("round-trip published mutation-manifest type"),
        manifest
    );
    for path in [
        manifest.base_offer,
        manifest.base_artifact,
        manifest.independent_selection,
    ] {
        assert!(
            listed.contains(Path::new(&path)),
            "unlisted manifest path {path}"
        );
    }
}

fn inputs() -> Inputs {
    inputs_with_event_body(
        "temporal ByEvent using T over (view: M::Plain) clock \"event-clock\" on origin { true }",
    )
}

fn compensation_inputs() -> Inputs {
    let mut inputs = Inputs::new(&[
        Unit {
            name: "event-time",
            body: "temporal ByEvent using T over (view: M::Plain) clock \"event-clock\" on origin { true }",
            declarations: &["ByEvent"],
        },
        Unit {
            name: "sample-time",
            body: "temporal BySample using F over (view: M::Plain) clock \"sample-clock\" on origin { true }",
            declarations: &["BySample"],
        },
        Unit {
            name: "timestamp-time",
            body: "temporal ByTimestamp using W over (view: M::Plain) clock \"timestamp-clock\" on origin { true }",
            declarations: &["ByTimestamp"],
        },
        Unit {
            name: "recovery-contracts",
            body: "pre Before using S on M::Node::step { delta >= 0 }
                   post After using S on M::Node::step { result }",
            declarations: &["Before", "After"],
        },
        Unit {
            name: "recovery-flow",
            body: "protocol RecoveryFlow using P over (view: M::Node) on origin {
                role Service on M::Node;
                compensate Full for Main::Applied as (forward: M::Node)
                    by Service on M::Node::step using T clock \"event-clock\" {
                  capture target: M::Total = forward.total;
                  activate first (trigger: M::Plain) when { trigger.ready } {
                    capture activated: Boolean = trigger.ready;
                  }
                  within [0,30]; attempts 3 of M::Plain;
                  retry (earlier: M::Plain, later: M::Plain) { earlier.ready = later.ready };
                  commit never;
                  recover (recovered: M::Node) { recovered.n = 1 };
                }
                compensate Partial for Main::Applied as (partialForward: M::Node)
                    by Service on M::Node::step using T clock \"event-clock\" {
                  capture partialTarget: M::Total = partialForward.total;
                  activate first (partialTrigger: M::Plain) when { partialTrigger.ready } {
                    capture partialActivated: Boolean = partialTrigger.ready;
                  }
                  within [0,30]; attempts 3 of M::Plain;
                  retry (partialEarlier: M::Plain, partialLater: M::Plain) {
                    partialEarlier.ready = partialLater.ready
                  };
                  commit never;
                  recover (partialRecovered: M::Node) { partialRecovered.n = 1 };
                }
                requires temporal ByEvent;
                run sequence Main {
                  attempt Tried by Service on M::Node::step contracts [Before,After]
                      as (attempted: M::Plain) { attempted.ready };
                  effect Applied of Main::Tried as (applied: M::Node) { true };
                }
                finish Closed as (closed: M::Node) { true };
            }",
            declarations: &["RecoveryFlow"],
        },
    ]);
    let _ = inputs.step_contracts("Before", "After");
    inputs
}

fn canonical(value: char) -> CanonicalDigest {
    CanonicalDigest {
        algorithm: "sha256".into(),
        domain: "filament-canonical-json-1".into(),
        value: format!("sha256:{}", value.to_string().repeat(64)),
    }
}

fn observation_digest(value: char) -> ObservationDigest {
    ObservationDigest(format!("sha256:{}", value.to_string().repeat(64)))
}

fn state_authority(
    package: &v2::AdmittedPackage,
    owner: u32,
    requirement_index: u32,
) -> AuthorityEvidence {
    let inherited = package.inherited();
    let requirement = &inherited.declarations[owner as usize].bindings[requirement_index as usize];
    let selection = requirement
        .model
        .0
        .as_ref()
        .expect("model-bound requirement");
    let producer = inherited.dependencies
        [inherited.models[selection.model as usize].artifact as usize]
        .artifact
        .clone();
    let static_selection = StaticAuthority {
        interface_version: "1.2.0".into(),
        document_identity: "document:selected".into(),
        document_digest: canonical('1'),
        model_identity: "model:selected".into(),
        model_digest: canonical('2'),
        profile_identity: "profile:selected".into(),
        profile_digest: canonical('3'),
        configuration_identity: "configuration:selected".into(),
        configuration_digest: canonical('4'),
    };
    let assessment_selection = AssessmentAuthority {
        population_identity: "population:selected".into(),
        membership_digest: observation_digest('5'),
        membership_complete: true,
        snapshot_identity: "snapshot:selected".into(),
        snapshot_digest: observation_digest('6'),
        window_identity: None,
        window_digest: None,
        closure_identity: "closure:selected".into(),
        closure_digest: observation_digest('7'),
    };
    let mut adapter_artifact = requirement.authority.clone();
    adapter_artifact.kind = w::ArtifactKind::Binding;
    adapter_artifact.identity = "explicit-d-f-compatibility".into();
    adapter_artifact.digest = ByteDigest::of(b"explicit compatibility mapping fixture");
    adapter_artifact.wire.identity = "quire.state.authority-adapter".into();
    adapter_artifact.wire.version = "1".into();
    let compiled = package.artifact().expect("strict package artifact").clone();
    let observation = requirement.authority.clone();
    let adapter = AuthorityAdapter {
        artifact: adapter_artifact,
        compiled: compiled.clone(),
        requirement: observation.clone(),
        producer: producer.clone(),
        observation: observation.clone(),
        producer_contract_revision: PRODUCER_CONTRACT_REVISION.into(),
        observation_contract_revision: OBSERVATION_CONTRACT_REVISION.into(),
        static_selection: static_selection.clone(),
        assessment_selection: assessment_selection.clone(),
    };
    AuthorityEvidence {
        producer_contract_revision: PRODUCER_CONTRACT_REVISION.into(),
        observation_contract_revision: OBSERVATION_CONTRACT_REVISION.into(),
        producer,
        observation,
        compiled,
        adapter: Some(adapter),
        static_selection,
        assessment_selection,
    }
}

fn binder_requirement(package: &v2::AdmittedPackage, binder: &w::Handle) -> u32 {
    let declaration = &package.inherited().declarations[binder.declaration as usize];
    declaration.anchors[declaration.binders[binder.index as usize].anchor.index as usize]
        .binding
        .0
        .expect("runtime-bound compensation binder")
}

fn field_export(
    package: &v2::AdmittedPackage,
    model: u32,
    record: &str,
    field: &str,
) -> w::ExportRef {
    let export = package.inherited().models[model as usize]
        .exports
        .iter()
        .position(|export| export.kind == w::ExportKind::Field && export.path == [record, field])
        .expect("selected model field");
    w::ExportRef {
        model,
        export: u32::try_from(export).expect("bounded fixture export"),
    }
}

fn plain_binder(package: &v2::AdmittedPackage, binder: w::Handle, ready: bool) -> BinderInput {
    let declaration = &package.inherited().declarations[binder.declaration as usize];
    let binding = &declaration.binders[binder.index as usize];
    let w::Type::Record { export } = &package.inherited().types[binding.value_type as usize] else {
        panic!("Plain binder record type")
    };
    let boolean_type = package
        .inherited()
        .types
        .iter()
        .position(|value| matches!(value, w::Type::Boolean {}))
        .and_then(|index| u32::try_from(index).ok())
        .expect("Boolean wire type");
    let requirement = binder_requirement(package, &binder);
    BinderInput {
        binder,
        requirement: Some(requirement),
        authority: Some(state_authority(
            package,
            binding.anchor.declaration,
            requirement,
        )),
        value: InputSlot::Available(StateValue::new(
            binding.value_type,
            StateValueKind::Record(vec![FieldInput {
                field: field_export(package, export.model, "Plain", "ready"),
                value: FieldValue::Compiled(InputSlot::Available(StateValue::new(
                    boolean_type,
                    StateValueKind::Boolean(ready),
                ))),
            }]),
        )),
    }
}

fn recovery_view(package: &v2::AdmittedPackage, binder: w::Handle, n: i64) -> StateView {
    let inherited = package.inherited();
    let declaration = &inherited.declarations[binder.declaration as usize];
    let binding = &declaration.binders[binder.index as usize];
    let requirement_index = binder_requirement(package, &binder);
    let w::Type::Object {
        export: object_type,
    } = &inherited.types[binding.value_type as usize]
    else {
        panic!("recovery object type")
    };
    let population_index = declaration
        .bindings
        .iter()
        .position(|requirement| {
            requirement.kind == w::BindingKind::Population
                && requirement.anchor == binding.anchor
                && requirement
                    .model
                    .0
                    .as_ref()
                    .is_some_and(|model| model.model == object_type.model)
        })
        .and_then(|index| u32::try_from(index).ok())
        .expect("recovery population requirement");
    let closure_index = declaration
        .bindings
        .iter()
        .position(|requirement| {
            requirement.kind == w::BindingKind::Closure
                && requirement.requires == [population_index]
        })
        .and_then(|index| u32::try_from(index).ok())
        .expect("recovery population closure");
    let population = declaration.bindings[population_index as usize]
        .model
        .0
        .clone()
        .expect("population export");
    let key = ObjectKey {
        observation: ObservationKey {
            anchor: binding.anchor.clone(),
            snapshot: ObservationIdentity("snapshot:selected".into()),
            window: None,
            record: ObservationIdentity("record:recovery".into()),
        },
        model: object_type.model,
        universe: population,
        object_type: object_type.clone(),
        identifier: "logical-recovery".into(),
    };
    let n_field = field_export(package, object_type.model, "Node", "n");
    let n_type = declaration
        .values
        .iter()
        .find_map(|value| match &value.operation {
            w::ValueOperation::Field { field, .. } if field == &n_field => Some(value.value_type),
            _ => None,
        })
        .expect("selected recovery field type");
    let fields = inherited.models[object_type.model as usize]
        .exports
        .iter()
        .enumerate()
        .filter(|(_, export)| {
            export.kind == w::ExportKind::Field
                && export.path.len() == 2
                && export.path[0] == "Node"
        })
        .map(|(index, _)| w::ExportRef {
            model: object_type.model,
            export: u32::try_from(index).expect("bounded fixture field"),
        })
        .map(|field| FieldInput {
            value: if field == n_field {
                FieldValue::Compiled(InputSlot::Available(StateValue::new(
                    n_type,
                    StateValueKind::Number(ProtocolNumber::Integer(ExactInteger::new(n))),
                )))
            } else {
                FieldValue::Contextual(ContextualSlot::Unavailable(MissingInput::Field {
                    object: key.clone(),
                    field: field.clone(),
                }))
            },
            field,
        })
        .collect();
    StateView {
        binders: vec![BinderInput {
            binder: binder.clone(),
            requirement: Some(requirement_index),
            authority: Some(state_authority(
                package,
                binder.declaration,
                requirement_index,
            )),
            value: InputSlot::Available(StateValue::new(
                binding.value_type,
                StateValueKind::Object(key.clone()),
            )),
        }],
        populations: vec![PopulationInput {
            requirement: w::Handle {
                declaration: binder.declaration,
                index: population_index,
            },
            closure_requirement: w::Handle {
                declaration: binder.declaration,
                index: closure_index,
            },
            authority: state_authority(package, binder.declaration, population_index),
            membership: Ok(()),
            closure: Ok(()),
            objects: vec![ObjectInput { key, fields }],
        }],
    }
}

fn inputs_with_event_body(event_body: &str) -> Inputs {
    Inputs::new(&[
        Unit {
            name: "event-time",
            body: event_body,
            declarations: &["ByEvent"],
        },
        Unit {
            name: "sample-time",
            body: "temporal BySample using F over (view: M::Plain) clock \"sample-clock\" on origin { true }",
            declarations: &["BySample"],
        },
        Unit {
            name: "timestamp-time",
            body: "temporal ByTimestamp using W over (view: M::Plain) clock \"timestamp-clock\" on origin { true }",
            declarations: &["ByTimestamp"],
        },
        Unit {
            name: "consumer",
            body: "protocol Flow using P over (view: M::Plain) on origin {
                role Service on M::Node;
                requires temporal ByEvent;
                requires temporal BySample;
                requires temporal ByTimestamp;
                run sequence Main {
                    event Happened by Service as (happened: M::Plain) { happened.ready };
                }
                finish Closed as (closed: M::Plain) { closed.ready };
            }",
            declarations: &["Flow"],
        },
    ])
}

fn definition_artifact<'a>(
    selected: &'a native::Selections<'a>,
    definition: R,
) -> &'a w::ArtifactRef {
    selected
        .dependencies
        .iter()
        .find(|dependency| {
            dependency.artifact.identity == definition.identity()
                && dependency.bytes == definition.bytes()
        })
        .expect("selected original definition bytes")
        .artifact
}

struct TemporalTables<'a> {
    producer: Vec<native::TemporalSelection<'a>>,
    expected: Vec<TemporalExpectation>,
}

fn with_v2(
    test: impl FnOnce(
        &Inputs,
        &proofs::ProofReport<'_, '_, '_>,
        &native::Selections<'_>,
        &TemporalTables<'_>,
        &native::AdmissionV2,
    ),
) {
    with_v2_inputs(inputs(), test);
}

fn with_v2_inputs(
    inputs: Inputs,
    test: impl FnOnce(
        &Inputs,
        &proofs::ProofReport<'_, '_, '_>,
        &native::Selections<'_>,
        &TemporalTables<'_>,
        &native::AdmissionV2,
    ),
) {
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            for entry in proofs.declarations() {
                let id = entry.declaration();
                assert_eq!(
                    proofs.types().disposition(id),
                    Some(TypeDisposition::Typed),
                    "{id:?}: {:?}",
                    proofs.types().declaration(id).map(|typed| typed.causes())
                );
            }
            let namespace = proofs.types().binding().namespace();
            let spans: Vec<_> = DECLARATIONS
                .iter()
                .map(|(name, _)| {
                    let [id] = namespace.lookup(name) else {
                        panic!("one authored declaration named {name}")
                    };
                    let span = namespace.syntax(*id).expect("authored syntax").span;
                    w::Span {
                        start: span.start as u32,
                        end: span.end as u32,
                    }
                })
                .collect();
            let revisions: Vec<_> = DECLARATIONS
                .iter()
                .map(|(_, definition)| w::Revision {
                    namespace: selected.definition_revision_namespace.into(),
                    value: definition.revision().into(),
                })
                .collect();
            let clocks = [
                v2::wire::ClockConfiguration::EventPosition {
                    sequence_authority: "orders".into(),
                },
                v2::wire::ClockConfiguration::FixedSample {
                    epoch: w::Number(NumberWire::Integer {
                        decimal: "0".into(),
                    }),
                    period: w::Number(NumberWire::Rational {
                        numerator: "1".into(),
                        denominator: "2".into(),
                    }),
                    unit: "second".into(),
                },
                v2::wire::ClockConfiguration::TimestampedEvent {
                    timestamp_unit: "millisecond".into(),
                },
            ];
            let temporal: Vec<_> = DECLARATIONS
                .iter()
                .enumerate()
                .map(|(index, (_, definition))| native::TemporalSelection {
                    source: &inputs.source_references[index],
                    span: &spans[index],
                    definition_identity: definition.identity(),
                    definition_revision: &revisions[index],
                    definition_artifact: definition_artifact(selected, *definition),
                    clock: &clocks[index],
                })
                .collect();
            // Re-derive the reader selection from the authored sources and
            // registered definition catalog. Do not clone or project the
            // producer's `TemporalSelection` table: producer and reader must be
            // capable of disagreeing independently.
            let expected_clocks = [
                v2::wire::ClockConfiguration::EventPosition {
                    sequence_authority: "orders".into(),
                },
                v2::wire::ClockConfiguration::FixedSample {
                    epoch: w::Number(NumberWire::Integer {
                        decimal: "0".into(),
                    }),
                    period: w::Number(NumberWire::Rational {
                        numerator: "1".into(),
                        denominator: "2".into(),
                    }),
                    unit: "second".into(),
                },
                v2::wire::ClockConfiguration::TimestampedEvent {
                    timestamp_unit: "millisecond".into(),
                },
            ];
            let expected: Vec<_> = DECLARATIONS
                .iter()
                .enumerate()
                .map(|(index, (name, definition))| {
                    inputs.temporal_expectation(
                        proofs,
                        index,
                        name,
                        TemporalDefinitionExpectation {
                            identity: definition.identity().into(),
                            revision: w::Revision {
                                namespace: selected.definition_revision_namespace.into(),
                                value: definition.revision().into(),
                            },
                            artifact: definition_artifact(selected, *definition).clone(),
                            clock: expected_clocks[index].clone(),
                        },
                    )
                })
                .collect();
            let temporal = TemporalTables {
                producer: temporal,
                expected,
            };
            let report = native::admit_v2(proofs, selected, &temporal.producer, Limits::default());
            assert!(
                report.result().is_ok(),
                "v2 admission: {:?}; locus {:?}",
                report.result().err(),
                report.locus()
            );
            let admission = report.into_result().unwrap();
            test(&inputs, proofs, selected, &temporal, &admission);
        },
    );
}

fn trace_input(name: &str, profile_identity: &str, parameters: &[(&str, &str)]) -> temporal::Trace {
    temporal::Trace {
        clock: temporal::ClockBinding {
            name: name.into(),
            profile_identity: profile_identity.into(),
            profile_revision: "1-draft.3".into(),
            parameters: parameters
                .iter()
                .map(|(name, value)| ((*name).into(), (*value).into()))
                .collect::<BTreeMap<_, _>>(),
        },
        positions: Vec::new(),
        anchor: "origin".into(),
        triggers: vec![temporal::Trigger {
            identity: "execution:1".into(),
            receipt: "receipt:1".into(),
            anchor: "origin".into(),
            payload: String::new(),
            guard: None,
            captures: Vec::new(),
        }],
        trigger_evidence: temporal::Evidence::Admitted,
        trigger_scope: temporal::Closure::Closed,
        decision_scope: temporal::Closure::Closed,
        surrounding_execution: temporal::Closure::Open,
        execution: temporal::Execution::Completed,
        completeness: temporal::Completeness::Complete,
        authoritative_origin: true,
        watermark: 0,
        evicted: Vec::new(),
    }
}

fn declaration(package: &v2::AdmittedPackage, name: &str) -> usize {
    package
        .inherited()
        .declarations
        .iter()
        .position(|declaration| declaration.name == name)
        .expect("authored temporal declaration")
}

fn checked_leaf(package: &v2::AdmittedPackage) -> (u32, w::Handle) {
    package
        .inherited()
        .declarations
        .iter()
        .enumerate()
        .find_map(|(index, declaration)| {
            let root = match &declaration.body {
                w::Body::Predicate { root, .. } | w::Body::State { root, .. } => Some(root.clone()),
                w::Body::Temporal { .. } => declaration.temporal.iter().find_map(|node| {
                    if let w::TemporalOperation::Holds { value } = &node.operation {
                        Some(value.clone())
                    } else {
                        None
                    }
                }),
                w::Body::Protocol {
                    controls, finish, ..
                } => controls
                    .iter()
                    .find_map(|control| {
                        if let w::ControlOperation::Check { value, .. } = &control.operation {
                            Some(value.clone())
                        } else {
                            None
                        }
                    })
                    .or_else(|| Some(finish.constraint.clone())),
            }?;
            Some((u32::try_from(index).ok()?, root))
        })
        .expect("fixture contains a checked Boolean clause")
}

fn assert_handoff_identity(document: &[u8], domain: &str) -> Value {
    let value: Value = serde_json::from_slice(document).expect("canonical handoff JSON");
    let marker = b",\"identity\":\"";
    let start = document
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("canonical identity member");
    let value_start = start + marker.len();
    let value_end = value_start + 64;
    assert_eq!(document.get(value_end), Some(&b'"'));
    let mut preimage = Vec::with_capacity(document.len() - marker.len() - 65);
    preimage.extend_from_slice(&document[..start]);
    preimage.extend_from_slice(&document[value_end + 1..]);
    let mut digest = Sha256::new();
    digest.update(domain.as_bytes());
    digest.update([0]);
    digest.update(preimage);
    assert_eq!(value["identity"], format!("{:x}", digest.finalize()));
    value
}

fn assert_schema(schema: &[u8], digest: &str, document: &[u8]) {
    assert_eq!(format!("{:x}", ByteDigest::of(schema)), digest);
    let schema: Value = serde_json::from_slice(schema).expect("published schema JSON");
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .expect("published handoff schema");
    let document: Value = serde_json::from_slice(document).expect("handoff JSON");
    assert!(validator.is_valid(&document));
}

/// Tracing: TC-141.
#[trace("TC-141", "FR-049-AC-9", "NFR-009-AC-4")]
#[test]
fn admitted_v2_evaluates_exact_compensation_expressions_with_shared_accounting() {
    with_v2_inputs(
        compensation_inputs(),
        |inputs, proofs, _, temporal, emitted| {
            let unpublished = emitted.admitted();
            let (owner, compensation) = unpublished
                .inherited()
                .declarations
                .iter()
                .enumerate()
                .find_map(|(owner, declaration)| match &declaration.body {
                    w::Body::Protocol { compensations, .. } => Some((
                        u32::try_from(owner).expect("bounded owner"),
                        &compensations[0],
                    )),
                    _ => None,
                })
                .expect("one compensation");
            let request = |value: w::Handle| EvaluationRequest {
                declaration: owner,
                value,
            };
            assert_eq!(
                state::evaluate_v2(
                    unpublished,
                    request(compensation.guard.clone()),
                    &StateView::default(),
                    StateLimits::default(),
                )
                .outcome(),
                &EvaluationOutcome::Refused(StateRefusal::UnpublishedArtifact)
            );

            let admitted = inputs
                .read_v2(proofs, emitted, &temporal.expected)
                .into_result()
                .expect("strict v2 package");
            let compensation = match &admitted.inherited().declarations[owner as usize].body {
                w::Body::Protocol { compensations, .. } => &compensations[0],
                _ => panic!("protocol"),
            };
            let cases = [
                (
                    compensation.guard.clone(),
                    StateView {
                        binders: vec![plain_binder(&admitted, compensation.trigger.clone(), true)],
                        populations: Vec::new(),
                    },
                ),
                (
                    compensation.retry.clone(),
                    StateView {
                        binders: vec![
                            plain_binder(&admitted, compensation.earlier.clone(), true),
                            plain_binder(&admitted, compensation.later.clone(), true),
                        ],
                        populations: Vec::new(),
                    },
                ),
                (
                    compensation.recover.clone(),
                    recovery_view(&admitted, compensation.recovery.clone(), 1),
                ),
            ];
            for (value, view) in cases {
                let selected = request(value);
                assert!(matches!(
                    state::evaluate_v2(
                        &admitted,
                        selected.clone(),
                        &StateView::default(),
                        StateLimits::default(),
                    )
                    .outcome(),
                    EvaluationOutcome::Refused(StateRefusal::MissingBinding(_))
                ));

                let mut missing_source = view.clone();
                let missing_binder = &missing_source.binders[0];
                let selected_authority = missing_binder
                    .authority
                    .as_ref()
                    .expect("selected source authority");
                let binding = &admitted.inherited().declarations
                    [missing_binder.binder.declaration as usize]
                    .binders[missing_binder.binder.index as usize];
                let missing = MissingInput::Observation(ObservationKey {
                    anchor: binding.anchor.clone(),
                    snapshot: ObservationIdentity(
                        selected_authority
                            .assessment_selection
                            .snapshot_identity
                            .clone(),
                    ),
                    window: selected_authority
                        .assessment_selection
                        .window_identity
                        .clone()
                        .map(ObservationIdentity),
                    record: ObservationIdentity("record:missing".into()),
                });
                missing_source.binders[0].value = InputSlot::Unavailable(missing.clone());
                assert_eq!(
                    state::evaluate_v2(
                        &admitted,
                        selected.clone(),
                        &missing_source,
                        StateLimits::default(),
                    )
                    .outcome(),
                    &EvaluationOutcome::Incomplete(missing)
                );

                let first =
                    state::evaluate_v2(&admitted, selected.clone(), &view, StateLimits::default());
                assert!(matches!(
                    first.outcome(),
                    EvaluationOutcome::Completed(value)
                        if matches!(value.kind(), StateValueKind::Boolean(true))
                ));
                let replay =
                    state::evaluate_v2(&admitted, selected.clone(), &view, StateLimits::default());
                assert_eq!(replay.outcome(), first.outcome());
                assert_eq!(replay.usage(), first.usage());

                let usage = first.usage();
                let exact_limits = StateLimits {
                    input_value_nodes: usage.input_value_nodes,
                    input_aggregate_entries: usage.input_aggregate_entries,
                    input_text_bytes: usage.input_text_bytes,
                    input_structural_depth: usage.input_structural_depth,
                    expression_work: usage.expression_work,
                    active_expression_depth: usage.active_expression_depth,
                    predicate_call_depth: usage.predicate_call_depth,
                    sequence_work: usage.sequence_work,
                    retained_output: usage.retained_output,
                    graph_expansion: usage.graph_expansion,
                    graph_edges: usage.graph_edges,
                    active_graph_depth: usage.active_graph_depth,
                    value_comparison: usage.value_comparison,
                };
                assert!(matches!(
                    state::evaluate_v2(&admitted, selected.clone(), &view, exact_limits).outcome(),
                    EvaluationOutcome::Completed(value)
                        if matches!(value.kind(), StateValueKind::Boolean(true))
                ));

                let one_short = StateLimits {
                    expression_work: usage.expression_work - 1,
                    ..StateLimits::default()
                };
                assert!(matches!(
                    state::evaluate_v2(&admitted, selected.clone(), &view, one_short).outcome(),
                    EvaluationOutcome::Exhausted(exhaustion)
                        if exhaustion.dimension == StateDimension::ExpressionWork
                ));

                let zero = state::evaluate_v2(
                    &admitted,
                    selected.clone(),
                    &view,
                    StateLimits {
                        expression_work: 0,
                        ..StateLimits::default()
                    },
                );
                assert!(matches!(
                    zero.outcome(),
                    EvaluationOutcome::Exhausted(exhaustion)
                        if exhaustion.dimension == StateDimension::ExpressionWork
                ));
                assert!(matches!(
                    state::evaluate_v2(
                        &admitted,
                        selected.clone(),
                        &view,
                        StateLimits::default(),
                    )
                    .outcome(),
                    EvaluationOutcome::Completed(value)
                        if matches!(value.kind(), StateValueKind::Boolean(true))
                ));

                let mut crossed_authority = view.clone();
                crossed_authority.binders[0]
                    .authority
                    .as_mut()
                    .expect("binder authority")
                    .compiled = admitted.inherited().producer.binary.clone();
                assert!(matches!(
                    state::evaluate_v2(
                        &admitted,
                        selected,
                        &crossed_authority,
                        StateLimits::default(),
                    )
                    .outcome(),
                    EvaluationOutcome::Refused(StateRefusal::Authority(_))
                ));
            }

            let mut crossed = request(compensation.guard.clone());
            crossed.value.declaration = owner.saturating_add(1);
            assert!(matches!(
                state::evaluate_v2(
                    &admitted,
                    crossed,
                    &StateView::default(),
                    StateLimits::default(),
                )
                .outcome(),
                EvaluationOutcome::Refused(StateRefusal::Owner(_))
            ));

            let compensations = match &admitted.inherited().declarations[owner as usize].body {
                w::Body::Protocol { compensations, .. } => compensations,
                _ => panic!("protocol"),
            };
            let other = &compensations[1];
            let crossed_compensation_view = StateView {
                binders: vec![plain_binder(&admitted, other.trigger.clone(), true)],
                populations: Vec::new(),
            };
            assert!(matches!(
                state::evaluate_v2(
                    &admitted,
                    request(compensation.guard.clone()),
                    &crossed_compensation_view,
                    StateLimits::default(),
                )
                .outcome(),
                EvaluationOutcome::Refused(StateRefusal::SurplusBinding(_))
            ));

            let mut wrong_type = StateView {
                binders: vec![plain_binder(&admitted, compensation.trigger.clone(), true)],
                populations: Vec::new(),
            };
            let InputSlot::Available(value) = &mut wrong_type.binders[0].value else {
                unreachable!("available fixture value")
            };
            value.value_type = u32::MAX;
            assert!(matches!(
                state::evaluate_v2(
                    &admitted,
                    request(compensation.guard.clone()),
                    &wrong_type,
                    StateLimits::default(),
                )
                .outcome(),
                EvaluationOutcome::Refused(StateRefusal::Type { .. })
            ));

            let mut wrong_producer = StateView {
                binders: vec![plain_binder(&admitted, compensation.trigger.clone(), true)],
                populations: Vec::new(),
            };
            let authority = wrong_producer.binders[0]
                .authority
                .as_mut()
                .expect("binder authority");
            authority.producer = admitted.inherited().producer.binary.clone();
            authority
                .adapter
                .as_mut()
                .expect("explicit adapter")
                .producer = authority.producer.clone();
            assert!(matches!(
                state::evaluate_v2(
                    &admitted,
                    request(compensation.guard.clone()),
                    &wrong_producer,
                    StateLimits::default(),
                )
                .outcome(),
                EvaluationOutcome::Refused(StateRefusal::Authority(_))
            ));

            let mut wrong_anchor = StateView {
                binders: vec![plain_binder(&admitted, compensation.trigger.clone(), true)],
                populations: Vec::new(),
            };
            let mut observation = ObservationKey {
                anchor: admitted.inherited().declarations[owner as usize].binders
                    [compensation.trigger.index as usize]
                    .anchor
                    .clone(),
                snapshot: ObservationIdentity("snapshot:selected".into()),
                window: None,
                record: ObservationIdentity("record:wrong-anchor".into()),
            };
            observation.anchor.index = observation.anchor.index.saturating_add(1);
            wrong_anchor.binders[0].value =
                InputSlot::Unavailable(MissingInput::Observation(observation));
            assert!(matches!(
                state::evaluate_v2(
                    &admitted,
                    request(compensation.guard.clone()),
                    &wrong_anchor,
                    StateLimits::default(),
                )
                .outcome(),
                EvaluationOutcome::Refused(StateRefusal::Authority(_))
            ));

            let mut crossed_population = recovery_view(&admitted, compensation.recovery.clone(), 1);
            crossed_population.populations[0].requirement.index =
                binder_requirement(&admitted, &compensation.trigger);
            assert!(matches!(
                state::evaluate_v2(
                    &admitted,
                    request(compensation.recover.clone()),
                    &crossed_population,
                    StateLimits::default(),
                )
                .outcome(),
                EvaluationOutcome::Refused(StateRefusal::SurplusBinding(_))
            ));
        },
    );
}

#[trace("TC-139", "FR-051-AC-1", "FR-051-AC-4", "FR-051-AC-6")]
#[test]
fn checked_predicate_and_temporal_handoffs_derive_read_and_retain_static_authority() {
    with_v2(|inputs, proofs, _, temporal, emitted| {
        let admitted = inputs
            .read_v2(proofs, emitted, &temporal.expected)
            .into_result()
            .expect("strict v2 package");
        let (predicate_declaration, predicate_root) = checked_leaf(&admitted);
        let predicate_selection = artifact::checked_predicate::ClauseSelection::new(
            predicate_declaration,
            predicate_root,
        );
        let predicate = artifact::checked_predicate::derive(
            &admitted,
            predicate_selection.clone(),
            artifact::checked_predicate::Limits::default(),
        )
        .into_result()
        .expect("derive checked predicate handoff");
        assert_schema(
            artifact::checked_predicate::SCHEMA_BYTES,
            artifact::checked_predicate::SCHEMA_SHA256,
            predicate.bytes(),
        );
        let predicate_value =
            assert_handoff_identity(predicate.bytes(), "quire.checked-predicate/v1");
        let predicate_view = artifact::checked_predicate::read(
            predicate.bytes(),
            &admitted,
            predicate_selection,
            artifact::checked_predicate::Limits::default(),
        )
        .into_result()
        .expect("strict checked predicate reader");
        assert_eq!(predicate_view.document().identity(), predicate.identity());
        assert_eq!(predicate_view.package_digest(), admitted.digest());
        assert_eq!(predicate_view.declaration(), predicate_declaration);
        assert_eq!(predicate_view.parent_kind(), Some("protocol"));
        assert!(!predicate_view.type_indices().is_empty());
        assert!(!predicate_view.values().collect::<Vec<_>>().is_empty());
        assert!(predicate_value.get("truth").is_none());

        let temporal_declaration =
            u32::try_from(declaration(&admitted, "ByEvent")).expect("declaration index");
        let temporal_selection =
            artifact::temporal_subject::DeclarationSelection::new(temporal_declaration);
        let temporal_document = artifact::temporal_subject::derive(
            &admitted,
            temporal_selection,
            artifact::temporal_subject::Limits::default(),
        )
        .into_result()
        .expect("derive checked temporal subject");
        assert_schema(
            artifact::temporal_subject::SCHEMA_BYTES,
            artifact::temporal_subject::SCHEMA_SHA256,
            temporal_document.bytes(),
        );
        let temporal_value = assert_handoff_identity(
            temporal_document.bytes(),
            "quire.checked-temporal-subject/v1",
        );
        let temporal_view = artifact::temporal_subject::read(
            temporal_document.bytes(),
            &admitted,
            temporal_selection,
            artifact::temporal_subject::Limits::default(),
        )
        .into_result()
        .expect("strict checked temporal subject reader");
        assert_eq!(temporal_view.package_digest(), admitted.digest());
        assert_eq!(temporal_view.declaration(), temporal_declaration);
        assert_eq!(temporal_view.history_boundary(), Some("execution-origin"));
        assert_eq!(
            temporal_view.operators(),
            Some(["constant".into()].as_slice())
        );
        assert!(temporal_view
            .predicate_leaves()
            .is_some_and(<[_]>::is_empty));
        assert!(temporal_value.get("observation").is_none());
        assert!(temporal_value.get("result").is_none());
        assert!(temporal_value.get("progress").is_none());
        assert!(temporal_value.get("closure").is_none());
        assert!(temporal_value.get("completeness").is_none());
    });
}

#[trace("TC-139", "FR-051-AC-1", "FR-051-AC-6")]
#[test]
fn temporal_handoff_exports_only_reachable_checked_holds_leaves() {
    let inputs = inputs_with_event_body(
        "temporal ByEvent using T over (view: M::Plain) clock \"event-clock\" on origin { holds(view.ready) }",
    );
    with_v2_inputs(inputs, |inputs, proofs, _, temporal, emitted| {
        let admitted = inputs
            .read_v2(proofs, emitted, &temporal.expected)
            .into_result()
            .expect("strict v2 package with a holds leaf");
        let declaration =
            u32::try_from(declaration(&admitted, "ByEvent")).expect("temporal declaration index");
        let temporal = artifact::temporal_subject::derive(
            &admitted,
            artifact::temporal_subject::DeclarationSelection::new(declaration),
            artifact::temporal_subject::Limits::default(),
        )
        .into_result()
        .expect("derive temporal subject");
        let temporal = artifact::temporal_subject::read(
            temporal.bytes(),
            &admitted,
            artifact::temporal_subject::DeclarationSelection::new(declaration),
            artifact::temporal_subject::Limits::default(),
        )
        .into_result()
        .expect("read temporal subject");
        assert_eq!(temporal.operators(), Some(["holds".into()].as_slice()));
        let [leaf] = temporal
            .predicate_leaves()
            .expect("temporal subject predicate leaves")
        else {
            panic!("one reachable holds leaf")
        };

        let selection =
            artifact::checked_predicate::ClauseSelection::new(declaration, leaf.clone());
        let predicate = artifact::checked_predicate::derive(
            &admitted,
            selection.clone(),
            artifact::checked_predicate::Limits::default(),
        )
        .into_result()
        .expect("derive checked holds leaf");
        assert_eq!(
            artifact::checked_predicate::read(
                predicate.bytes(),
                &admitted,
                selection,
                artifact::checked_predicate::Limits::default(),
            )
            .into_result()
            .expect("read checked holds leaf")
            .parent_kind(),
            Some("temporal")
        );
    });
}

#[trace("TC-139", "FR-051-AC-1", "FR-051-AC-2", "FR-051-AC-6")]
#[test]
fn temporal_activation_guard_is_a_distinct_checked_predicate_selection() {
    let inputs = inputs_with_event_body(
        "temporal ByEvent using T over (view: M::Plain) clock \"event-clock\" on each (started: M::Node) when (started.n >= 0) { holds(view.ready) }",
    );
    with_v2_inputs(inputs, |inputs, proofs, _, temporal, emitted| {
        let admitted = inputs
            .read_v2(proofs, emitted, &temporal.expected)
            .into_result()
            .expect("strict guarded temporal package");
        let declaration =
            u32::try_from(declaration(&admitted, "ByEvent")).expect("temporal declaration index");
        let w::Body::Temporal {
            activation:
                w::Activation::Each {
                    trigger,
                    guard: w::Nullable(Some(guard)),
                    anchor,
                },
            ..
        } = &admitted.inherited().declarations
            [usize::try_from(declaration).expect("host declaration index")]
        .body
        else {
            panic!("authored guarded activation")
        };

        let temporal = artifact::temporal_subject::derive(
            &admitted,
            artifact::temporal_subject::DeclarationSelection::new(declaration),
            artifact::temporal_subject::Limits::default(),
        )
        .into_result()
        .expect("derive guarded temporal subject");
        let temporal = artifact::temporal_subject::read(
            temporal.bytes(),
            &admitted,
            artifact::temporal_subject::DeclarationSelection::new(declaration),
            artifact::temporal_subject::Limits::default(),
        )
        .into_result()
        .expect("read guarded temporal subject");
        let [formula_leaf] = temporal
            .predicate_leaves()
            .expect("temporal formula leaves")
        else {
            panic!("one reachable holds leaf")
        };
        assert_ne!(
            guard, formula_leaf,
            "activation guard is not a formula leaf"
        );

        let guard_selection =
            artifact::checked_predicate::ClauseSelection::new(declaration, guard.clone());
        let guard_document = artifact::checked_predicate::derive(
            &admitted,
            guard_selection.clone(),
            artifact::checked_predicate::Limits::default(),
        )
        .into_result()
        .expect("derive checked activation guard");
        let predicate = artifact::checked_predicate::read(
            guard_document.bytes(),
            &admitted,
            guard_selection.clone(),
            artifact::checked_predicate::Limits::default(),
        )
        .into_result()
        .expect("read checked activation guard");
        assert_eq!(predicate.parent_kind(), Some("temporal"));
        assert_eq!(predicate.leaf(), guard);

        let formula_selection =
            artifact::checked_predicate::ClauseSelection::new(declaration, formula_leaf.clone());
        let formula_document = artifact::checked_predicate::derive(
            &admitted,
            formula_selection.clone(),
            artifact::checked_predicate::Limits::default(),
        )
        .into_result()
        .expect("derive checked formula leaf");
        for (bytes, selection) in [
            (guard_document.bytes(), formula_selection),
            (formula_document.bytes(), guard_selection),
        ] {
            let report = artifact::checked_predicate::read(
                bytes,
                &admitted,
                selection,
                artifact::checked_predicate::Limits::default(),
            );
            assert_handoff_error(
                &report,
                artifact::checked_predicate::ErrorCode::NonCanonical,
            );
        }

        for invalid in [trigger, anchor] {
            let report = artifact::checked_predicate::derive(
                &admitted,
                artifact::checked_predicate::ClauseSelection::new(declaration, invalid.clone()),
                artifact::checked_predicate::Limits::default(),
            );
            assert_handoff_error(
                &report,
                artifact::checked_predicate::ErrorCode::InvalidSelection,
            );
        }
    });
}

fn assert_handoff_error<T>(
    report: &artifact::checked_predicate::Report<T>,
    code: artifact::checked_predicate::ErrorCode,
) {
    match report.result() {
        Ok(_) => panic!("handoff must refuse"),
        Err(error) => assert_eq!(error.code(), code),
    }
}

#[trace("TC-139", "FR-051-AC-2", "FR-051-AC-3", "FR-051-AC-5")]
#[test]
fn checked_handoff_readers_refuse_mutation_cross_wiring_and_resource_overrun() {
    with_v2(|inputs, proofs, _, temporal, emitted| {
        let admitted = inputs
            .read_v2(proofs, emitted, &temporal.expected)
            .into_result()
            .expect("strict v2 package");
        let (declaration_index, root) = checked_leaf(&admitted);
        let selection =
            artifact::checked_predicate::ClauseSelection::new(declaration_index, root.clone());
        let limits = artifact::checked_predicate::Limits::default();
        let document = artifact::checked_predicate::derive(&admitted, selection.clone(), limits)
            .into_result()
            .expect("control document");

        let mut unknown: Value = serde_json::from_slice(document.bytes()).unwrap();
        unknown["unknown"] = Value::Bool(true);
        let unknown = serde_json::to_vec(&unknown).unwrap();
        assert_handoff_error(
            &artifact::checked_predicate::read(&unknown, &admitted, selection.clone(), limits),
            artifact::checked_predicate::ErrorCode::NonCanonical,
        );

        let control: Value = serde_json::from_slice(document.bytes()).unwrap();
        for field in [
            "contract",
            "identity",
            "package",
            "subject",
            "source",
            "clause",
            "expression",
            "bindings",
            "type",
            "profiles",
            "limits",
        ] {
            let mut missing = control.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert_handoff_error(
                &artifact::checked_predicate::read(
                    &serde_json::to_vec(&missing).unwrap(),
                    &admitted,
                    selection.clone(),
                    limits,
                ),
                artifact::checked_predicate::ErrorCode::NonCanonical,
            );
            let mut changed = control.clone();
            changed[field] = Value::Null;
            assert_handoff_error(
                &artifact::checked_predicate::read(
                    &serde_json::to_vec(&changed).unwrap(),
                    &admitted,
                    selection.clone(),
                    limits,
                ),
                artifact::checked_predicate::ErrorCode::NonCanonical,
            );
        }

        let marker = b"{\"contract\":";
        assert!(document.bytes().starts_with(marker));
        let mut duplicate = b"{\"contract\":\"duplicate\",\"contract\":".to_vec();
        duplicate.extend_from_slice(&document.bytes()[marker.len()..]);
        assert_handoff_error(
            &artifact::checked_predicate::read(&duplicate, &admitted, selection.clone(), limits),
            artifact::checked_predicate::ErrorCode::NonCanonical,
        );

        let mut changed_identity: Value = serde_json::from_slice(document.bytes()).unwrap();
        changed_identity["identity"] = Value::String("0".repeat(64));
        let changed_identity = serde_json::to_vec(&changed_identity).unwrap();
        assert_handoff_error(
            &artifact::checked_predicate::read(
                &changed_identity,
                &admitted,
                selection.clone(),
                limits,
            ),
            artifact::checked_predicate::ErrorCode::NonCanonical,
        );

        let mut trailing = document.bytes().to_vec();
        trailing.push(b'\n');
        assert_handoff_error(
            &artifact::checked_predicate::read(&trailing, &admitted, selection.clone(), limits),
            artifact::checked_predicate::ErrorCode::NonCanonical,
        );

        let input_limited = artifact::checked_predicate::Limits {
            input_bytes: document.bytes().len().saturating_sub(1),
            ..limits
        };
        assert_handoff_error(
            &artifact::checked_predicate::read(
                document.bytes(),
                &admitted,
                selection.clone(),
                input_limited,
            ),
            artifact::checked_predicate::ErrorCode::ResourceIncomplete,
        );
        for limited in [
            artifact::checked_predicate::Limits {
                json_depth: 1,
                ..limits
            },
            artifact::checked_predicate::Limits {
                population: 1,
                ..limits
            },
            artifact::checked_predicate::Limits {
                string_bytes: 1,
                ..limits
            },
            artifact::checked_predicate::Limits {
                visited_fields: 1,
                ..limits
            },
        ] {
            let report = artifact::checked_predicate::read(
                document.bytes(),
                &admitted,
                selection.clone(),
                limited,
            );
            assert_handoff_error(
                &report,
                artifact::checked_predicate::ErrorCode::ResourceIncomplete,
            );
            assert_eq!(
                report.usage().output_bytes,
                0,
                "input census must refuse before canonical derivation"
            );
        }
        let output_limited = artifact::checked_predicate::Limits {
            output_bytes: 0,
            ..limits
        };
        assert_handoff_error(
            &artifact::checked_predicate::derive(&admitted, selection.clone(), output_limited),
            artifact::checked_predicate::ErrorCode::ResourceIncomplete,
        );
        for limited in [
            artifact::checked_predicate::Limits {
                json_depth: 1,
                ..limits
            },
            artifact::checked_predicate::Limits {
                population: 0,
                ..limits
            },
            artifact::checked_predicate::Limits {
                expression_depth: 0,
                ..limits
            },
            artifact::checked_predicate::Limits {
                string_bytes: 0,
                ..limits
            },
            artifact::checked_predicate::Limits {
                visited_fields: 0,
                ..limits
            },
        ] {
            assert_handoff_error(
                &artifact::checked_predicate::derive(&admitted, selection.clone(), limited),
                artifact::checked_predicate::ErrorCode::ResourceIncomplete,
            );
        }

        let foreign = artifact::checked_predicate::ClauseSelection::new(
            declaration_index.saturating_add(10_000),
            root,
        );
        assert_handoff_error(
            &artifact::checked_predicate::derive(&admitted, foreign, limits),
            artifact::checked_predicate::ErrorCode::InvalidSelection,
        );

        let temporal_declaration =
            u32::try_from(declaration(&admitted, "ByEvent")).expect("declaration index");
        let temporal_selection =
            artifact::temporal_subject::DeclarationSelection::new(temporal_declaration);
        let temporal_document = artifact::temporal_subject::derive(
            &admitted,
            temporal_selection,
            artifact::temporal_subject::Limits::default(),
        )
        .into_result()
        .expect("temporal document");
        assert_handoff_error(
            &artifact::checked_predicate::read(
                temporal_document.bytes(),
                &admitted,
                selection,
                limits,
            ),
            artifact::checked_predicate::ErrorCode::NonCanonical,
        );
    });
}

#[trace("TC-132", "FR-048-AC-1")]
#[test]
fn admitted_v2_exposes_the_inherited_occurrence_key_schema_without_translation() {
    with_v2(|_, _, _, _, emitted| {
        let declaration = u32::try_from(declaration(emitted.admitted(), "Flow")).unwrap();
        let report =
            artifact::occurrence_key_schema(emitted.admitted(), declaration, Limits::default());
        let schema = report.result().expect("v2 admitted protocol projection");
        assert_eq!(schema.declaration(), declaration);
        assert!(!schema.roles().is_empty());
        assert!(!schema.nodes().is_empty());
    });
}

fn assert_error<T>(report: &artifact::Report<T>, expected: Error) {
    assert_eq!(report.result().err(), Some(&expected));
}

fn v2_refusal(refusal: v2::Refusal) -> Error {
    Error::V2(refusal)
}

fn binding(side: v2::InventorySide, cause: v2::BindingCause) -> Error {
    v2_refusal(v2::Refusal::Binding { side, cause })
}

#[trace("TC-138", "FR-050-AC-2", "FR-050-AC-3")]
#[test]
fn public_v2_refusal_codes_are_injective_across_every_declared_axis() {
    let mut refusals = Vec::new();
    refusals.extend(
        [
            v2::HeaderField::Wire,
            v2::HeaderField::Media,
            v2::HeaderField::Schema,
            v2::HeaderField::PackageType,
            v2::HeaderField::Encoding,
            v2::HeaderField::Numeric,
            v2::HeaderField::ArtifactKind,
            v2::HeaderField::ArtifactWire,
            v2::HeaderField::ArtifactVersion,
        ]
        .map(v2::Refusal::Header),
    );
    for side in [
        v2::InventorySide::Offer,
        v2::InventorySide::Expected,
        v2::InventorySide::Producer,
    ] {
        for cause in [
            v2::BindingCause::Missing,
            v2::BindingCause::Surplus,
            v2::BindingCause::Duplicate,
        ] {
            refusals.push(v2::Refusal::Binding { side, cause });
        }
    }
    refusals.extend([
        v2::Refusal::OfferOrder,
        v2::Refusal::ForeignOwner(v2::SelectionSide::Expected),
        v2::Refusal::ForeignOwner(v2::SelectionSide::Producer),
        v2::Refusal::OfferIndex(v2::BindingIndex::Declaration),
        v2::Refusal::OfferIndex(v2::BindingIndex::Definition),
    ]);
    refusals.extend(
        [
            v2::DeclarationField::Name,
            v2::DeclarationField::Requirement,
            v2::DeclarationField::Clause,
            v2::DeclarationField::Execution,
        ]
        .map(v2::Refusal::Declaration),
    );
    refusals.extend([
        v2::Refusal::Definition(v2::DefinitionField::Identity),
        v2::Refusal::Definition(v2::DefinitionField::Revision),
    ]);
    refusals.extend(
        [
            v2::ArtifactField::RefVersion,
            v2::ArtifactField::Kind,
            v2::ArtifactField::Authority,
            v2::ArtifactField::Identity,
            v2::ArtifactField::RevisionNamespace,
            v2::ArtifactField::RevisionValue,
            v2::ArtifactField::Digest,
            v2::ArtifactField::WireIdentity,
            v2::ArtifactField::WireVersion,
        ]
        .map(|field| v2::Refusal::Definition(v2::DefinitionField::Artifact(field))),
    );
    refusals.extend(
        [
            v2::ClockField::Alternative,
            v2::ClockField::SequenceAuthority,
            v2::ClockField::Epoch,
            v2::ClockField::Period,
            v2::ClockField::Unit,
            v2::ClockField::TimestampUnit,
        ]
        .map(v2::Refusal::Clock),
    );

    let codes = refusals
        .iter()
        .map(|refusal| refusal.code())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(codes.len(), refusals.len());
}

/// Tracing: TC-121, TC-138.
#[trace("TC-121", "TC-138", "FR-042-AC-7", "FR-050-AC-2")]
#[test]
fn public_error_codes_are_injective_across_every_declared_axis() {
    let mut errors = vec![Error::Allocation, Error::Json { line: 1, column: 1 }];
    errors.extend(
        [
            NumberError::NonCanonicalDecimal {
                component: NumberComponent::Decimal,
            },
            NumberError::ComponentOutOfRange {
                component: NumberComponent::Decimal,
            },
            NumberError::NonPositiveDenominator,
            NumberError::UnreducedRational,
        ]
        .map(Error::Numeric),
    );
    errors.push(Error::Producer(ProducerModelRefusal::ResourceExhausted(
        BindingExhaustion {
            dimension: BindingDimension::Bindings,
            used: 0,
            requested: 1,
            limit: 0,
        },
    )));
    errors.extend(
        [
            ProducerModelRefusal::Interface,
            ProducerModelRefusal::Bundle,
            ProducerModelRefusal::Model,
            ProducerModelRefusal::Profile,
            ProducerModelRefusal::Configuration,
            ProducerModelRefusal::Correspondence,
            ProducerModelRefusal::DefinitionClosure,
            ProducerModelRefusal::Exports,
            ProducerModelRefusal::ProducerDigest,
            ProducerModelRefusal::NativeDigest,
            ProducerModelRefusal::NativeBytes,
        ]
        .map(Error::Producer),
    );
    errors.extend(
        [
            Invalid::Selection,
            Invalid::Seal,
            Invalid::Name,
            Invalid::StructuralInteger,
            Invalid::WrongNumericKind,
            Invalid::NumericDomain,
            Invalid::Inventory,
            Invalid::Order,
            Invalid::Duplicate,
            Invalid::Dependency,
            Invalid::Definition,
            Invalid::Model,
            Invalid::ForeignLocus,
            Invalid::Locus,
            Invalid::Reference,
            Invalid::Owner,
            Invalid::Scope,
            Invalid::Type,
            Invalid::Profile,
            Invalid::Call,
            Invalid::Binding,
            Invalid::Control,
            Invalid::Cycle,
            Invalid::Feature,
            Invalid::Canonical,
            Invalid::Encoding,
        ]
        .map(Error::Invalid),
    );
    errors.extend(
        [
            Unsupported::Wire,
            Unsupported::Feature,
            Unsupported::Definition,
            Unsupported::Profile,
            Unsupported::ProducerCorrespondence,
            Unsupported::FamilyProof,
            Unsupported::Export,
        ]
        .map(Error::Unsupported),
    );
    errors.extend(WorkDimension::ALL.map(|dimension| {
        Error::Incomplete(artifact::Exhaustion {
            dimension,
            used: 0,
            requested: 1,
            limit: 0,
            locus: None,
        })
    }));
    errors.push(Error::V2(v2::Refusal::Header(v2::HeaderField::Wire)));

    let codes = errors
        .iter()
        .map(Error::code)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(codes.len(), errors.len());

    // The two identities the downstream mutation corpus cannot express through
    // `v2::Refusal::code` alone.
    assert_eq!(Error::Json { line: 3, column: 9 }.code(), "json");
    assert_eq!(Error::Invalid(Invalid::Seal).code(), "invalid.seal");
    // A version-2 refusal keeps its own established spelling.
    assert_eq!(
        Error::V2(v2::Refusal::Header(v2::HeaderField::Wire)).code(),
        v2::Refusal::Header(v2::HeaderField::Wire).code()
    );
}

#[trace("TC-138", "FR-050-AC-1", "FR-050-AC-4", "FR-050-AC-6")]
#[test]
fn native_v2_emission_reaches_the_strict_reader_and_authenticated_l5_adapter() {
    with_v2(|inputs, proofs, selected, temporal, emitted| {
        let package = emitted.admitted().package();
        assert_eq!(
            emitted
                .admitted()
                .schema_model(0)
                .map(|model| model.digest()),
            Some(selected.models[0].model.digest())
        );
        assert_eq!(package.temporal_bindings.len(), 3);
        assert!(package
            .temporal_bindings
            .windows(2)
            .all(|pair| pair[0].declaration < pair[1].declaration));
        for binding in &package.temporal_bindings {
            assert_eq!(
                binding.definition,
                package.inherited.declarations[binding.declaration as usize].profile
            );
        }

        let read = inputs.read_v2(proofs, emitted, &temporal.expected);
        let admitted = read.result().expect("strict v2 reader");
        assert_eq!(admitted.digest(), emitted.digest());
        assert_eq!(admitted.package(), emitted.admitted().package());
        assert_eq!(
            admitted.schema_model(0).map(|model| model.digest()),
            Some(selected.models[0].model.digest())
        );
        // Emission produces bytes before any caller publishes an identity for
        // them; the reader retains the identity it independently admitted.
        assert!(emitted.admitted().artifact().is_none());
        assert_eq!(
            admitted.artifact().map(|artifact| artifact.digest),
            Some(emitted.digest())
        );

        for (name, clock, profile, parameters) in [
            (
                "ByEvent",
                "event-clock",
                temporal::EVENT_POSITION,
                vec![("sequence_authority", "orders")],
            ),
            (
                "BySample",
                "sample-clock",
                temporal::FIXED_SAMPLE,
                vec![
                    ("epoch", "{\"kind\":\"integer\",\"decimal\":\"0\"}"),
                    (
                        "period",
                        "{\"kind\":\"rational\",\"numerator\":\"1\",\"denominator\":\"2\"}",
                    ),
                    ("unit", "second"),
                ],
            ),
            (
                "ByTimestamp",
                "timestamp-clock",
                temporal::TIMESTAMPED_WINDOW,
                vec![("timestamp_unit", "millisecond")],
            ),
        ] {
            let at = declaration(admitted, name);
            let supplied = trace_input(clock, profile, &parameters);
            let report =
                temporal::evaluate_v2(admitted, at, &supplied, temporal::Limits::default());
            assert!(report.result().is_ok(), "{name} reaches evaluation");
            // FR-050-AC-6: the authenticated clock identity is retained in the
            // result, so a version-2 report is not mistakable for a version-1 one.
            let authenticated = report
                .authenticated()
                .unwrap_or_else(|| panic!("{name} retains its authenticated binding"));
            assert_eq!(authenticated.declaration, at);
            assert_eq!(authenticated.clock, clock);
            assert_eq!(authenticated.definition_identity, profile);
            assert_eq!(authenticated.package_digest, admitted.digest().to_string());
            assert_eq!(
                authenticated.parameters,
                parameters
                    .iter()
                    .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                    .collect::<BTreeMap<_, _>>()
            );
            assert!(temporal::mapping_support_v2(admitted, at, temporal::Closure::Open).is_ok());
        }

        let v1_before = native::admit(proofs, selected, Limits::default())
            .into_result()
            .expect("v1 admission");
        let v1_before = native::emit(&v1_before, Limits::default())
            .into_result()
            .expect("v1 emission");
        let frozen_v1_digest =
            include_str!("fixtures/compiled-protocol-v1-v2-fixture.sha256").trim();
        assert_eq!(v1_before.bytes().len(), 84_780);
        assert_eq!(v1_before.digest().to_string(), frozen_v1_digest);
        assert!(inputs.read(proofs, &v1_before).result().is_ok());

        assert!(inputs
            .read_bytes(proofs, emitted.bytes(), emitted.digest())
            .result()
            .is_err());
        assert!(inputs
            .read_v2_bytes(
                proofs,
                v1_before.bytes(),
                v1_before.digest(),
                &temporal.expected,
                Limits::default(),
            )
            .result()
            .is_err());
    });
}

#[trace("TC-138", "FR-050-AC-2", "FR-050-AC-3")]
#[test]
fn producer_requires_one_exact_source_definition_and_clock_selection() {
    with_v2(|inputs, proofs, selected, temporal, _| {
        assert_error(
            &native::admit_v2(proofs, selected, &temporal.producer[..2], Limits::default()),
            binding(v2::InventorySide::Producer, v2::BindingCause::Missing),
        );
        let mut surplus = temporal.producer.to_vec();
        surplus.push(temporal.producer[0]);
        assert_error(
            &native::admit_v2(proofs, selected, &surplus, Limits::default()),
            binding(v2::InventorySide::Producer, v2::BindingCause::Surplus),
        );
        let mut duplicate = temporal.producer.to_vec();
        duplicate[2] = duplicate[1];
        assert_error(
            &native::admit_v2(proofs, selected, &duplicate, Limits::default()),
            binding(v2::InventorySide::Producer, v2::BindingCause::Duplicate),
        );
        let mut foreign = temporal.producer.to_vec();
        foreign[0].source = &inputs.source_references[1];
        assert_error(
            &native::admit_v2(proofs, selected, &foreign, Limits::default()),
            v2_refusal(v2::Refusal::ForeignOwner(v2::SelectionSide::Producer)),
        );
        let wrong_clock = v2::wire::ClockConfiguration::TimestampedEvent {
            timestamp_unit: "millisecond".into(),
        };
        let mut wrong_profile = temporal.producer.to_vec();
        wrong_profile[0].clock = &wrong_clock;
        assert_error(
            &native::admit_v2(proofs, selected, &wrong_profile, Limits::default()),
            v2_refusal(v2::Refusal::Clock(v2::ClockField::Alternative)),
        );
        let mut wrong_definition = temporal.producer.to_vec();
        wrong_definition[0].definition_identity = R::FixedSample.identity();
        assert_error(
            &native::admit_v2(proofs, selected, &wrong_definition, Limits::default()),
            v2_refusal(v2::Refusal::Definition(v2::DefinitionField::Identity)),
        );

        let invalid_clocks = [
            (
                v2::wire::ClockConfiguration::FixedSample {
                    epoch: w::Number(NumberWire::Integer {
                        decimal: "0".into(),
                    }),
                    period: w::Number(NumberWire::Integer {
                        decimal: "0".into(),
                    }),
                    unit: "second".into(),
                },
                Error::Invalid(Invalid::NumericDomain),
            ),
            (
                v2::wire::ClockConfiguration::FixedSample {
                    epoch: w::Number(NumberWire::Integer {
                        decimal: "0".into(),
                    }),
                    period: w::Number(NumberWire::Integer {
                        decimal: "-1".into(),
                    }),
                    unit: "second".into(),
                },
                Error::Invalid(Invalid::NumericDomain),
            ),
            (
                v2::wire::ClockConfiguration::FixedSample {
                    epoch: w::Number(NumberWire::Integer {
                        decimal: "0".into(),
                    }),
                    period: w::Number(NumberWire::Rational {
                        numerator: "2".into(),
                        denominator: "4".into(),
                    }),
                    unit: "second".into(),
                },
                Error::Numeric(NumberError::UnreducedRational),
            ),
            (
                v2::wire::ClockConfiguration::FixedSample {
                    epoch: w::Number(NumberWire::Integer {
                        decimal: "0".into(),
                    }),
                    period: w::Number(NumberWire::Rational {
                        numerator: "9223372036854775808".into(),
                        denominator: "1".into(),
                    }),
                    unit: "second".into(),
                },
                Error::Numeric(NumberError::ComponentOutOfRange {
                    component: NumberComponent::Numerator,
                }),
            ),
        ];
        for (clock, expected) in &invalid_clocks {
            let mut invalid = temporal.producer.to_vec();
            invalid[1].clock = clock;
            assert_error(
                &native::admit_v2(proofs, selected, &invalid, Limits::default()),
                expected.clone(),
            );
        }
        let empty = v2::wire::ClockConfiguration::EventPosition {
            sequence_authority: String::new(),
        };
        let mut unnamed = temporal.producer.to_vec();
        unnamed[0].clock = &empty;
        assert_error(
            &native::admit_v2(proofs, selected, &unnamed, Limits::default()),
            v2_refusal(v2::Refusal::Clock(v2::ClockField::SequenceAuthority)),
        );
        let oversized = v2::wire::ClockConfiguration::TimestampedEvent {
            timestamp_unit: "x".repeat(4_097),
        };
        let mut overlong = temporal.producer.to_vec();
        overlong[2].clock = &oversized;
        assert_error(
            &native::admit_v2(proofs, selected, &overlong, Limits::default()),
            v2_refusal(v2::Refusal::Clock(v2::ClockField::TimestampUnit)),
        );
    });
}

#[trace("TC-138", "FR-050-AC-2", "FR-050-AC-3")]
#[test]
fn strict_reader_rejects_resealed_structural_and_identity_substitutions() {
    with_v2(|inputs, proofs, _, temporal, emitted| {
        let temporal = &temporal.expected;
        let base = emitted.admitted().package();
        let cases: Vec<(v2::wire::Package, Error)> = {
            let mut missing = base.clone();
            missing.temporal_bindings.pop();
            let mut surplus = base.clone();
            surplus
                .temporal_bindings
                .push(base.temporal_bindings[0].clone());
            let mut duplicate = base.clone();
            duplicate.temporal_bindings[2] = duplicate.temporal_bindings[1].clone();
            let mut reordered = base.clone();
            reordered.temporal_bindings.swap(0, 1);
            let mut wrong_definition = base.clone();
            wrong_definition.temporal_bindings[0].definition =
                wrong_definition.temporal_bindings[1].definition;
            let mut wrong_clock = base.clone();
            wrong_clock.temporal_bindings[0].clock =
                v2::wire::ClockConfiguration::TimestampedEvent {
                    timestamp_unit: "millisecond".into(),
                };
            let mut changed_definition = base.clone();
            let index = changed_definition.temporal_bindings[0].definition as usize;
            changed_definition.inherited.definitions[index]
                .identity
                .push_str("-other");
            let declaration = base.temporal_bindings[0].declaration as usize;
            let mut changed_name = base.clone();
            changed_name.inherited.declarations[declaration]
                .name
                .push_str("Other");
            let mut changed_requirement = base.clone();
            changed_requirement.inherited.declarations[declaration]
                .requirement
                .identity
                .push_str("Other");
            let mut changed_clause = base.clone();
            changed_clause.inherited.declarations[declaration]
                .clause
                .push_str("-other");
            let mut changed_execution = base.clone();
            changed_execution.inherited.declarations[declaration].execution =
                w::Execution::Handler {
                    name: "other".into(),
                };
            vec![
                (
                    missing,
                    binding(v2::InventorySide::Offer, v2::BindingCause::Missing),
                ),
                (
                    surplus,
                    binding(v2::InventorySide::Offer, v2::BindingCause::Surplus),
                ),
                (
                    duplicate,
                    binding(v2::InventorySide::Offer, v2::BindingCause::Duplicate),
                ),
                (reordered, v2_refusal(v2::Refusal::OfferOrder)),
                (
                    wrong_definition,
                    v2_refusal(v2::Refusal::OfferIndex(v2::BindingIndex::Definition)),
                ),
                (
                    wrong_clock,
                    v2_refusal(v2::Refusal::Clock(v2::ClockField::Alternative)),
                ),
                (
                    changed_definition,
                    v2_refusal(v2::Refusal::Definition(v2::DefinitionField::Identity)),
                ),
                (
                    changed_name,
                    v2_refusal(v2::Refusal::Declaration(v2::DeclarationField::Name)),
                ),
                (
                    changed_requirement,
                    v2_refusal(v2::Refusal::Declaration(v2::DeclarationField::Requirement)),
                ),
                (
                    changed_clause,
                    v2_refusal(v2::Refusal::Declaration(v2::DeclarationField::Clause)),
                ),
                (
                    changed_execution,
                    v2_refusal(v2::Refusal::Declaration(v2::DeclarationField::Execution)),
                ),
            ]
        };
        for (offered, expected) in cases {
            let bytes = serde_json::to_vec(&offered).expect("canonical adverse wire shape");
            assert_error(
                &inputs.read_v2_bytes(
                    proofs,
                    &bytes,
                    ByteDigest::of(&bytes),
                    temporal,
                    Limits::default(),
                ),
                expected,
            );
        }

        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                emitted.digest(),
                &temporal[..2],
                Limits::default(),
            ),
            binding(v2::InventorySide::Expected, v2::BindingCause::Missing),
        );
        let mut surplus_expected = temporal.to_vec();
        surplus_expected.push(temporal[0].clone());
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                emitted.digest(),
                &surplus_expected,
                Limits::default(),
            ),
            binding(v2::InventorySide::Expected, v2::BindingCause::Surplus),
        );
        let mut duplicate_expected = temporal.to_vec();
        duplicate_expected[2] = duplicate_expected[1].clone();
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                emitted.digest(),
                &duplicate_expected,
                Limits::default(),
            ),
            binding(v2::InventorySide::Expected, v2::BindingCause::Duplicate),
        );
        let mut changed_span = temporal.to_vec();
        changed_span[0].declaration.span.start += 1;
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                emitted.digest(),
                &changed_span,
                Limits::default(),
            ),
            binding(v2::InventorySide::Expected, v2::BindingCause::Missing),
        );
        let changed_clock = v2::wire::ClockConfiguration::EventPosition {
            sequence_authority: "other-orders".into(),
        };
        let mut changed_expected = temporal.to_vec();
        changed_expected[0].definition.clock = changed_clock;
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                emitted.digest(),
                &changed_expected,
                Limits::default(),
            ),
            v2_refusal(v2::Refusal::Clock(v2::ClockField::SequenceAuthority)),
        );
        let mut revision_expected = temporal.to_vec();
        revision_expected[0]
            .definition
            .revision
            .value
            .push_str("-other");
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                emitted.digest(),
                &revision_expected,
                Limits::default(),
            ),
            v2_refusal(v2::Refusal::Definition(v2::DefinitionField::Revision)),
        );
        let mut foreign_owner = temporal.to_vec();
        foreign_owner[0].source = inputs.source_references[1].clone();
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                emitted.digest(),
                &foreign_owner,
                Limits::default(),
            ),
            v2_refusal(v2::Refusal::ForeignOwner(v2::SelectionSide::Expected)),
        );

        for (field, artifact) in {
            let original = &temporal[0].definition.artifact;
            let mut ref_version = original.clone();
            ref_version.ref_version.push_str("-other");
            let mut kind = original.clone();
            kind.kind = w::ArtifactKind::GeneratedArtifact;
            let mut authority = original.clone();
            authority.authority.push_str("/other");
            let mut identity = original.clone();
            identity.identity.push_str("/other");
            let mut revision_namespace = original.clone();
            revision_namespace.revision.namespace.push_str("/other");
            let mut revision_value = original.clone();
            revision_value.revision.value.push_str("-other");
            let mut digest = original.clone();
            digest.digest = emitted.digest();
            let mut wire_identity = original.clone();
            wire_identity.wire.identity.push_str("/canonical-json");
            let mut wire_version = original.clone();
            wire_version.wire.version.push_str("-other");
            vec![
                (v2::ArtifactField::RefVersion, ref_version),
                (v2::ArtifactField::Kind, kind),
                (v2::ArtifactField::Authority, authority),
                (v2::ArtifactField::Identity, identity),
                (v2::ArtifactField::RevisionNamespace, revision_namespace),
                (v2::ArtifactField::RevisionValue, revision_value),
                (v2::ArtifactField::Digest, digest),
                (v2::ArtifactField::WireIdentity, wire_identity),
                (v2::ArtifactField::WireVersion, wire_version),
            ]
        } {
            let mut changed = temporal.to_vec();
            changed[0].definition.artifact = artifact;
            assert_error(
                &inputs.read_v2_bytes(
                    proofs,
                    emitted.bytes(),
                    emitted.digest(),
                    &changed,
                    Limits::default(),
                ),
                v2_refusal(v2::Refusal::Definition(v2::DefinitionField::Artifact(
                    field,
                ))),
            );
        }
    });
}

#[trace("TC-138", "FR-050-AC-2", "FR-050-AC-3", "FR-050-AC-4")]
#[test]
fn headers_indices_and_each_clock_member_have_stable_v2_refusals() {
    with_v2(|inputs, proofs, _, temporal, emitted| {
        let temporal = &temporal.expected;
        let base = emitted.admitted().package();
        for (field, offered) in {
            let mut wire = base.clone();
            wire.inherited.wire = artifact::WIRE.into();
            let mut media = base.clone();
            media.inherited.media = artifact::MEDIA.into();
            let mut schema = base.clone();
            schema.inherited.schema = artifact::SCHEMA.into();
            let mut package_type = base.clone();
            package_type.inherited.package_type.push_str("Other");
            let mut encoding = base.clone();
            encoding.inherited.encoding.push_str("-other");
            let mut numeric = base.clone();
            numeric.inherited.numeric.push_str("-other");
            vec![
                (v2::HeaderField::Wire, wire),
                (v2::HeaderField::Media, media),
                (v2::HeaderField::Schema, schema),
                (v2::HeaderField::PackageType, package_type),
                (v2::HeaderField::Encoding, encoding),
                (v2::HeaderField::Numeric, numeric),
            ]
        } {
            let bytes = serde_json::to_vec(&offered).unwrap();
            assert_error(
                &inputs.read_v2_bytes(
                    proofs,
                    &bytes,
                    ByteDigest::of(&bytes),
                    temporal,
                    Limits::default(),
                ),
                v2_refusal(v2::Refusal::Header(field)),
            );
        }

        for (index, offered) in {
            let mut declaration = base.clone();
            declaration.temporal_bindings[2].declaration = 9_999;
            let mut definition = base.clone();
            definition.temporal_bindings[0].definition = 9_999;
            vec![
                (v2::BindingIndex::Declaration, declaration),
                (v2::BindingIndex::Definition, definition),
            ]
        } {
            let bytes = serde_json::to_vec(&offered).unwrap();
            assert_error(
                &inputs.read_v2_bytes(
                    proofs,
                    &bytes,
                    ByteDigest::of(&bytes),
                    temporal,
                    Limits::default(),
                ),
                v2_refusal(v2::Refusal::OfferIndex(index)),
            );
        }

        let clocks = [
            (
                0,
                v2::wire::ClockConfiguration::EventPosition {
                    sequence_authority: "other-orders".into(),
                },
                v2::ClockField::SequenceAuthority,
            ),
            (
                1,
                v2::wire::ClockConfiguration::FixedSample {
                    epoch: w::Number(NumberWire::Integer {
                        decimal: "1".into(),
                    }),
                    period: w::Number(NumberWire::Rational {
                        numerator: "1".into(),
                        denominator: "2".into(),
                    }),
                    unit: "second".into(),
                },
                v2::ClockField::Epoch,
            ),
            (
                1,
                v2::wire::ClockConfiguration::FixedSample {
                    epoch: w::Number(NumberWire::Integer {
                        decimal: "0".into(),
                    }),
                    period: w::Number(NumberWire::Integer {
                        decimal: "1".into(),
                    }),
                    unit: "second".into(),
                },
                v2::ClockField::Period,
            ),
            (
                1,
                v2::wire::ClockConfiguration::FixedSample {
                    epoch: w::Number(NumberWire::Integer {
                        decimal: "0".into(),
                    }),
                    period: w::Number(NumberWire::Rational {
                        numerator: "1".into(),
                        denominator: "2".into(),
                    }),
                    unit: "minute".into(),
                },
                v2::ClockField::Unit,
            ),
            (
                2,
                v2::wire::ClockConfiguration::TimestampedEvent {
                    timestamp_unit: "nanosecond".into(),
                },
                v2::ClockField::TimestampUnit,
            ),
        ];
        for (index, clock, field) in &clocks {
            let mut expected = temporal.to_vec();
            expected[*index].definition.clock = clock.clone();
            assert_error(
                &inputs.read_v2_bytes(
                    proofs,
                    emitted.bytes(),
                    emitted.digest(),
                    &expected,
                    Limits::default(),
                ),
                v2_refusal(v2::Refusal::Clock(*field)),
            );
        }
    });
}

#[trace("TC-138", "FR-050-AC-2", "FR-050-AC-3")]
#[test]
fn malformed_clock_objects_and_changed_original_definition_bytes_refuse() {
    with_v2(|inputs, proofs, selected, temporal, emitted| {
        let producer_temporal = &temporal.producer;
        let temporal = &temporal.expected;
        let original: serde_json::Value = serde_json::from_slice(emitted.bytes()).unwrap();
        for (binding_index, field, renamed) in [
            (0, "sequence_authority", "sequenceAuthority"),
            (1, "period", "sample_period"),
            (2, "timestamp_unit", "timestampUnit"),
        ] {
            for replacement in [None, Some(renamed)] {
                let mut changed = original.clone();
                let clock = changed["temporal_bindings"][binding_index]["clock"]
                    .as_object_mut()
                    .unwrap();
                let value = clock.remove(field).unwrap();
                if let Some(replacement) = replacement {
                    clock.insert(replacement.into(), value);
                }
                let bytes = serde_json::to_vec(&changed).unwrap();
                assert!(matches!(
                    inputs
                        .read_v2_bytes(
                            proofs,
                            &bytes,
                            ByteDigest::of(&bytes),
                            temporal,
                            Limits::default(),
                        )
                        .result(),
                    Err(Error::Json { .. })
                ));
            }
        }

        let definition = &producer_temporal[0].definition_artifact;
        let at = selected
            .dependencies
            .iter()
            .position(|dependency| &dependency.artifact == definition)
            .unwrap();
        let mut dependencies = selected.dependencies.to_vec();
        let mut changed_bytes = dependencies[at].bytes.to_vec();
        changed_bytes.push(b'\n');
        dependencies[at].bytes = &changed_bytes;
        let changed = native::Selections {
            dependencies: &dependencies,
            ..*selected
        };
        assert_error(
            &native::admit_v2(proofs, &changed, producer_temporal, Limits::default()),
            Error::Invalid(Invalid::Seal),
        );
    });
}

#[trace("TC-138", "FR-050-AC-5")]
#[test]
fn added_v2_work_is_exactly_bounded_and_a_fresh_retry_is_reproducible() {
    with_v2(|inputs, proofs, _, temporal, emitted| {
        let temporal = &temporal.expected;
        let complete = inputs.read_v2_bytes(
            proofs,
            emitted.bytes(),
            emitted.digest(),
            temporal,
            Limits::default(),
        );
        let usage = complete.usage();
        // This fixture-owned oracle is intentionally frozen independently of
        // each report; a shared accounting drift cannot move test and source
        // together unnoticed.
        assert_eq!(
            (
                usage.entries,
                usage.references,
                usage.byte_work,
                usage.output_bytes
            ),
            (4_814, 2_588, 2_311_843, 85_188)
        );
        for (dimension, amount) in [
            (WorkDimension::Entries, usage.entries),
            (WorkDimension::References, usage.references),
            (WorkDimension::ByteWork, usage.byte_work),
            (WorkDimension::OutputBytes, usage.output_bytes),
        ] {
            assert!(amount > 0);
            let mut limits = Limits::default();
            match dimension {
                WorkDimension::Entries => limits.entries = amount - 1,
                WorkDimension::References => limits.references = amount - 1,
                WorkDimension::ByteWork => limits.byte_work = amount - 1,
                WorkDimension::OutputBytes => limits.output_bytes = amount - 1,
                _ => unreachable!(),
            }
            let report =
                inputs.read_v2_bytes(proofs, emitted.bytes(), emitted.digest(), temporal, limits);
            let Error::Incomplete(exhaustion) = report.result().unwrap_err() else {
                panic!("{dimension:?}: expected typed exhaustion")
            };
            assert_eq!(exhaustion.dimension, dimension);

            let mut exact = Limits::default();
            match dimension {
                WorkDimension::Entries => exact.entries = amount,
                WorkDimension::References => exact.references = amount,
                WorkDimension::ByteWork => exact.byte_work = amount,
                WorkDimension::OutputBytes => exact.output_bytes = amount,
                _ => unreachable!(),
            }
            assert!(inputs
                .read_v2_bytes(proofs, emitted.bytes(), emitted.digest(), temporal, exact)
                .result()
                .is_ok());
        }
        let above_hard = Limits {
            payload_bytes: usize::MAX,
            output_bytes: usize::MAX,
            source_bytes: usize::MAX,
            content_bytes: usize::MAX,
            sources: usize::MAX,
            dependencies: usize::MAX,
            definitions: usize::MAX,
            models: usize::MAX,
            declarations: usize::MAX,
            entries: usize::MAX,
            references: usize::MAX,
            byte_work: usize::MAX,
            depth: usize::MAX,
        };
        let clamped = inputs.read_v2_bytes(
            proofs,
            emitted.bytes(),
            emitted.digest(),
            temporal,
            above_hard,
        );
        assert_eq!(clamped.limits(), Limits::default());
        assert!(clamped.result().is_ok());
        let retry = inputs
            .read_v2(proofs, emitted, temporal)
            .into_result()
            .expect("fresh sufficient retry");
        assert_eq!(retry.package(), emitted.admitted().package());
    });
}

#[trace("TC-138", "FR-050-AC-6")]
#[test]
fn l5_refuses_each_parameter_axis_before_temporal_evaluation() {
    with_v2(|inputs, proofs, _, temporal_selections, emitted| {
        let temporal_selections = &temporal_selections.expected;
        let admitted = inputs
            .read_v2(proofs, emitted, temporal_selections)
            .into_result()
            .unwrap();
        let assert_pre_position_refusal =
            |at: usize, supplied: &temporal::Trace, dimension: temporal::Dimension| {
                let report =
                    temporal::evaluate_v2(&admitted, at, supplied, temporal::Limits::default());
                assert!(matches!(
                    report.result(),
                    Err(temporal::Error::Refused(temporal::Refusal::Binding {
                        dimension: actual,
                        ..
                    })) if *actual == dimension
                ));
                assert_eq!(report.usage().positions, 0);
                // Authentication itself refused, so nothing was authenticated.
                assert!(report.authenticated().is_none());
            };
        let at = declaration(&admitted, "BySample");
        let matching = [
            ("epoch", "{\"kind\":\"integer\",\"decimal\":\"0\"}"),
            (
                "period",
                "{\"kind\":\"rational\",\"numerator\":\"1\",\"denominator\":\"2\"}",
            ),
            ("unit", "second"),
        ];
        for (dimension, mutate) in [
            (
                temporal::Dimension::Epoch,
                ("epoch", "{\"kind\":\"integer\",\"decimal\":\"1\"}"),
            ),
            (
                temporal::Dimension::SamplePeriod,
                (
                    "period",
                    "{\"kind\":\"rational\",\"numerator\":\"1\",\"denominator\":\"3\"}",
                ),
            ),
            (temporal::Dimension::ClockUnit, ("unit", "minute")),
        ] {
            let mut supplied = trace_input("sample-clock", temporal::FIXED_SAMPLE, &matching);
            supplied
                .clock
                .parameters
                .insert(mutate.0.into(), mutate.1.into());
            assert_pre_position_refusal(at, &supplied, dimension);
        }

        for dimension in [
            temporal::Dimension::Profile,
            temporal::Dimension::ProfileRevision,
            temporal::Dimension::Clock,
        ] {
            let mut supplied = trace_input("sample-clock", temporal::FIXED_SAMPLE, &matching);
            match dimension {
                temporal::Dimension::Profile => {
                    supplied.clock.profile_identity = temporal::EVENT_POSITION.into();
                }
                temporal::Dimension::ProfileRevision => {
                    supplied.clock.profile_revision = "other-revision".into();
                }
                temporal::Dimension::Clock => supplied.clock.name = "other-clock".into(),
                _ => unreachable!("closed local mutation table"),
            }
            assert_pre_position_refusal(at, &supplied, dimension);
        }

        let event_at = declaration(&admitted, "ByEvent");
        let changed_event = trace_input(
            "event-clock",
            temporal::EVENT_POSITION,
            &[("sequence_authority", "other-orders")],
        );
        assert_pre_position_refusal(
            event_at,
            &changed_event,
            temporal::Dimension::SequenceAuthority,
        );

        let timestamp_at = declaration(&admitted, "ByTimestamp");
        let changed_timestamp = trace_input(
            "timestamp-clock",
            temporal::TIMESTAMPED_WINDOW,
            &[("timestamp_unit", "nanosecond")],
        );
        assert_pre_position_refusal(
            timestamp_at,
            &changed_timestamp,
            temporal::Dimension::TimestampUnit,
        );

        for (remove, rename) in [("epoch", None), ("period", Some("sample_period"))] {
            let mut supplied = trace_input("sample-clock", temporal::FIXED_SAMPLE, &matching);
            let value = supplied
                .clock
                .parameters
                .remove(remove)
                .expect("matching parameter exists");
            if let Some(rename) = rename {
                supplied.clock.parameters.insert(rename.into(), value);
            }
            assert_pre_position_refusal(at, &supplied, temporal::Dimension::ClockParameters);
        }
        let mut extra = trace_input("sample-clock", temporal::FIXED_SAMPLE, &matching);
        extra.clock.parameters.insert("extra".into(), "x".into());
        assert_pre_position_refusal(at, &extra, temporal::Dimension::ClockParameters);
    });
}

#[trace("TC-138", "FR-050-AC-6")]
#[test]
fn progress_is_partitioned_by_package_definition_and_exact_clock_configuration() {
    with_v2(|inputs, proofs, selected, temporal, emitted| {
        let producer_temporal = &temporal.producer;
        let expected_temporal = &temporal.expected;
        let original = inputs
            .read_v2(proofs, emitted, expected_temporal)
            .into_result()
            .unwrap();
        let changed_clock = v2::wire::ClockConfiguration::FixedSample {
            epoch: w::Number(NumberWire::Integer {
                decimal: "0".into(),
            }),
            period: w::Number(NumberWire::Rational {
                numerator: "1".into(),
                denominator: "2".into(),
            }),
            unit: "minute".into(),
        };
        let mut changed_selections = producer_temporal.to_vec();
        changed_selections[1].clock = &changed_clock;
        let changed_emission =
            native::admit_v2(proofs, selected, &changed_selections, Limits::default())
                .into_result()
                .expect("changed exact configuration emission");
        assert_ne!(emitted.digest(), changed_emission.digest());
        let mut changed_expected = expected_temporal.to_vec();
        changed_expected[1].definition.clock = changed_clock;
        let changed = inputs
            .read_v2(proofs, &changed_emission, &changed_expected)
            .into_result()
            .expect("changed exact configuration admission");

        let original_at = declaration(&original, "BySample");
        let changed_at = declaration(&changed, "BySample");
        let mut original_trace = trace_input(
            "sample-clock",
            temporal::FIXED_SAMPLE,
            &[
                ("epoch", "{\"kind\":\"integer\",\"decimal\":\"0\"}"),
                (
                    "period",
                    "{\"kind\":\"rational\",\"numerator\":\"1\",\"denominator\":\"2\"}",
                ),
                ("unit", "second"),
            ],
        );
        original_trace.watermark = 10;
        let mut changed_trace = original_trace.clone();
        changed_trace
            .clock
            .parameters
            .insert("unit".into(), "minute".into());
        changed_trace.watermark = 5;

        let mut ledger = temporal::Ledger::new();
        assert!(temporal::evaluate_with_progress_v2(
            &original,
            original_at,
            &original_trace,
            temporal::Limits::default(),
            &mut ledger,
        )
        .result()
        .is_ok());
        assert!(
            temporal::mapping_support_v2(&original, usize::MAX, temporal::Closure::Open).is_err()
        );
        assert!(temporal::mapping_support_v2(
            &original,
            declaration(&original, "Flow"),
            temporal::Closure::Open
        )
        .is_err());

        let mut other_producer = selected.producer.clone();
        other_producer
            .implementation
            .push_str("/same-clock-other-artifact");
        let other_selection = native::Selections {
            producer: &other_producer,
            ..*selected
        };
        let other_emission = native::admit_v2(
            proofs,
            &other_selection,
            producer_temporal,
            Limits::default(),
        )
        .into_result()
        .expect("same clock under a different source-authorized artifact");
        assert_ne!(emitted.digest(), other_emission.digest());
        let mut other_trace = original_trace.clone();
        other_trace.watermark = 5;
        assert!(temporal::evaluate_with_progress_v2(
            other_emission.admitted(),
            declaration(other_emission.admitted(), "BySample"),
            &other_trace,
            temporal::Limits::default(),
            &mut ledger,
        )
        .result()
        .is_ok());

        assert!(temporal::evaluate_with_progress_v2(
            &changed,
            changed_at,
            &changed_trace,
            temporal::Limits::default(),
            &mut ledger,
        )
        .result()
        .is_ok());

        changed_trace.watermark = 4;
        let regressed = temporal::evaluate_with_progress_v2(
            &changed,
            changed_at,
            &changed_trace,
            temporal::Limits::default(),
            &mut ledger,
        );
        assert!(matches!(
            regressed.result(),
            Err(temporal::Error::Refused(temporal::Refusal::Progress {
                dimension: temporal::Dimension::Watermark,
                ..
            }))
        ));
    });
}

#[trace("TC-138", "FR-050-AC-3", "FR-050-AC-4")]
#[test]
fn digest_domains_and_original_producer_bytes_cannot_be_reinterpreted() {
    with_v2(|inputs, proofs, selected, temporal, emitted| {
        let producer_temporal = &temporal.producer;
        let expected_temporal = &temporal.expected;
        let definition_digest = expected_temporal[0].definition.artifact.digest;
        assert_ne!(definition_digest, emitted.digest());
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                definition_digest,
                expected_temporal,
                Limits::default(),
            ),
            Error::Invalid(Invalid::Seal),
        );

        let mut substituted_artifact = expected_temporal[0].definition.artifact.clone();
        substituted_artifact.digest = emitted.digest();
        let mut substituted_expected = expected_temporal.to_vec();
        substituted_expected[0].definition.artifact = substituted_artifact;
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                emitted.digest(),
                &substituted_expected,
                Limits::default(),
            ),
            v2_refusal(v2::Refusal::Definition(v2::DefinitionField::Artifact(
                v2::ArtifactField::Digest,
            ))),
        );

        let model_dependency = selected
            .dependencies
            .iter()
            .position(|dependency| dependency.artifact == &inputs.model_reference)
            .expect("selected original model package");
        let original = selected.dependencies[model_dependency].bytes;
        let value: serde_json::Value = serde_json::from_slice(original).expect("model JSON");
        let recanonicalized = serde_json::to_vec_pretty(&value).expect("recanonicalized model");
        assert_ne!(original, recanonicalized);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(original).unwrap(),
            serde_json::from_slice::<serde_json::Value>(&recanonicalized).unwrap()
        );
        let mut dependencies = selected.dependencies.to_vec();
        dependencies[model_dependency].bytes = &recanonicalized;
        let changed = native::Selections {
            dependencies: &dependencies,
            ..*selected
        };
        assert_error(
            &native::admit_v2(proofs, &changed, producer_temporal, Limits::default()),
            Error::Invalid(Invalid::Seal),
        );
    });
}
