// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-138: strict compiled-protocol v2 production, admission and L5 handoff.

use crate::support::native_protocol as setup;

use ix_trace_rs::trace;
use qsl_foundation::Source;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::number::{NumberComponent, NumberError, NumberWire};
use quire_spec_language::protocol_artifact::{
    self as artifact, native, v2, v3, wire as w, Dimension as WorkDimension, Error, Invalid,
    Limits, Unsupported,
};
use setup::{Inputs, TemporalDefinitionExpectation, TemporalExpectation, Unit};

const DECLARATIONS: [(&str, R); 3] = [
    ("ByEvent", R::EventPosition),
    ("BySample", R::FixedSample),
    ("ByTimestamp", R::TimestampedWindow),
];

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
                  retry (earlier: M::Plain, later: M::Plain) {
                    earlier.ready = later.ready and target >= 0 and not activated
                  };
                  commit never;
                  recover (recovered: M::Node) {
                    recovered.n = 1 and target >= 0 and not activated
                  };
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
                    and partialTarget >= 0 and not partialActivated
                  };
                  commit never;
                  recover (partialRecovered: M::Node) {
                    partialRecovered.n = 1
                    and partialTarget >= 0 and not partialActivated
                  };
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
    with_v2_inputs_and_source_limits(inputs, quire_spec_language::Limits::default(), test);
}

fn with_v2_inputs_and_source_limits(
    inputs: Inputs,
    source_limits: quire_spec_language::Limits,
    test: impl FnOnce(
        &Inputs,
        &proofs::ProofReport<'_, '_, '_>,
        &native::Selections<'_>,
        &TemporalTables<'_>,
        &native::AdmissionV2,
    ),
) {
    let mut type_limits = TypeLimits::default();
    type_limits.expressions = type_limits.expressions.max(source_limits.tokens);
    let mut artifact_limits = Limits::default();
    artifact_limits.entries = artifact_limits
        .entries
        .max(source_limits.tokens.saturating_mul(20));
    artifact_limits.depth = artifact_limits.depth.max(source_limits.tokens);
    artifact_limits.content_bytes = artifact_limits
        .content_bytes
        .max(source_limits.source_bytes.saturating_mul(20));
    artifact_limits.payload_bytes = artifact_limits
        .payload_bytes
        .max(source_limits.source_bytes.saturating_mul(20));
    artifact_limits.output_bytes = artifact_limits
        .output_bytes
        .max(source_limits.source_bytes.saturating_mul(20));
    artifact_limits.byte_work = artifact_limits
        .byte_work
        .max(source_limits.source_bytes.saturating_mul(200));
    inputs.with_proofs_and_source_limits(
        type_limits,
        proofs::ProofLimits::default(),
        source_limits,
        |proofs, selected| {
            for entry in proofs.declarations() {
                let id = entry.declaration();
                assert_eq!(
                    proofs.types().disposition(id),
                    Some(TypeDisposition::Typed),
                    "{id:?}: {:?}; exhaustion: {:?}",
                    proofs.types().declaration(id).map(|typed| typed.causes()),
                    proofs.types().exhaustion()
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
            let report = native::admit_v2(proofs, selected, &temporal.producer, artifact_limits);
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

fn checked_fr050_inputs() -> Inputs {
    checked_fr050_inputs_with_event_body(
        "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { eventually[0,0] holds(view.code > 0) }",
    )
}

fn checked_fr050_inputs_with_event_body(event_body: &str) -> Inputs {
    let units = [
        Unit {
            name: "event-time",
            body: event_body,
            declarations: &["ByEvent"],
        },
        Unit {
            name: "sample-time",
            body: "temporal BySample using F over (view: M::Widget) clock \"sample-clock\" on origin { eventually[0,0] holds(view.code > 0) }",
            declarations: &["BySample"],
        },
        Unit {
            name: "timestamp-time",
            body: "temporal ByTimestamp using W over (view: M::Widget) clock \"timestamp-clock\" on origin { eventually[0,0] holds(view.code > 0) }",
            declarations: &["ByTimestamp"],
        },
        Unit {
            name: "consumer",
            body: "protocol Flow using P over (view: M::Plain) on origin { role Service on M::Node; requires temporal ByEvent; requires temporal BySample; requires temporal ByTimestamp; run sequence Main { event Happened by Service as (happened: M::Plain) { happened.ready }; } finish Closed as (closed: M::Plain) { closed.ready }; }",
            declarations: &["Flow"],
        },
    ];
    let mut model: serde_json::Value =
        serde_json::from_str(crate::support::native_rule_model::fixture())
            .expect("native model fixture JSON");
    model["package"] = serde_json::json!("acme/orders");
    model["records"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "name": "Widget",
            "fields": [{"name": "code", "type": {"kind": "scalar", "name": "Version"}}]
        }));
    model["records"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "name": "Plain",
            "fields": [{"name": "ready", "type": {"kind": "boolean"}}]
        }));
    let model = crate::support::native_rule_model::from_text(
        &serde_json::to_string(&model).expect("serialize test model"),
        "fr050-native-model.json",
    )
    .expect("native model fixture with the same checked model identity")
    .model();
    let mut inputs = Inputs::with_model(&units, model);
    for (index, retained) in ["T", "F", "W"].into_iter().enumerate() {
        let source = &inputs.sources[index];
        let text = source
            .text()
            .split_inclusive('\n')
            .map(|line| {
                let unused_temporal_profile = ["T", "F", "W"].into_iter().any(|alias| {
                    alias != retained && line.starts_with(&format!("profile {alias} ="))
                });
                if unused_temporal_profile {
                    let content_len = line.strip_suffix('\n').unwrap_or(line).len();
                    format!(
                        "{}{}",
                        " ".repeat(content_len),
                        if line.ends_with('\n') { "\n" } else { "" }
                    )
                } else {
                    line.to_owned()
                }
            })
            .collect::<String>();
        let replaced = Source::read(
            source.identity().clone(),
            source.path(),
            text.as_bytes(),
            quire_spec_language::Limits::default().source_bytes,
        )
        .expect("one checked temporal profile remains in each source");
        inputs.sources[index] = replaced.clone();
        inputs.formal[index] = quire_spec_language::formal_source::FormalSource::new(
            replaced,
            inputs.formal[index].identity().clone(),
        );
        inputs.mappings[index].source = inputs.formal[index].clone();
    }
    // The checked graph's RawSourceRef uses the source identity's authority;
    // the admitted artifact selection must retain that exact pair too.
    for source in &mut inputs.source_references {
        source.authority = "test".into();
    }
    inputs
}

fn derive_v3_bytes(
    emitted: &native::AdmissionV2,
    activations: &[v3::ExpectedActivation],
) -> Vec<u8> {
    let v2_package = emitted.admitted().package();
    let mut inherited = v2_package.inherited.clone();
    inherited.wire = v3::WIRE.into();
    inherited.media = v3::MEDIA.into();
    inherited.schema = v3::SCHEMA.into();
    let package = v3::wire::Package {
        inherited,
        temporal_bindings: v2_package.temporal_bindings.clone(),
        activation_mappings: activations
            .iter()
            .map(|selection| v3::wire::ActivationMapping {
                control: selection.control.clone(),
                temporal_declaration: selection.temporal_declaration,
            })
            .collect(),
    };
    v3::encode_candidate(&package, Limits::default())
        .into_result()
        .expect("derived v3 package encodes canonically")
        .bytes()
        .to_vec()
}

fn declaration(package: &v2::AdmittedPackage, name: &str) -> usize {
    package
        .inherited()
        .declarations
        .iter()
        .position(|declaration| declaration.name == name)
        .expect("authored temporal declaration")
}

#[test]
fn initialized_compensation_capture_graph_refuses_foreign_and_recursive_initializers() {
    with_v2_inputs(
        compensation_inputs(),
        |inputs, proofs, _, temporal, emitted| {
            let base = emitted.admitted().package();
            let (owner, compensation) = base
                .inherited
                .declarations
                .iter()
                .enumerate()
                .find_map(|(owner, declaration)| match &declaration.body {
                    w::Body::Protocol { compensations, .. } => {
                        Some((owner, compensations.first().expect("one compensation")))
                    }
                    _ => None,
                })
                .expect("compensation declaration");
            let capture = &compensation.activation_captures[0];
            let capture_index = usize::try_from(capture.index).expect("bounded capture index");
            let initializer = base.inherited.declarations[owner].binders[capture_index]
                .initializer
                .0
                .as_ref()
                .expect("capture initializer")
                .clone();

            let mut foreign = base.clone();
            foreign.inherited.declarations[owner].binders[capture_index]
                .initializer
                .0
                .as_mut()
                .expect("capture initializer")
                .declaration = capture.declaration.saturating_sub(1);
            let bytes = serde_json::to_vec(&foreign).expect("canonical adverse wire shape");
            assert_error(
                &inputs.read_v2_bytes(proofs, &bytes, &temporal.expected, Limits::default()),
                Error::Invalid(Invalid::Owner),
            );

            let mut recursive = base.clone();
            let initializer_index =
                usize::try_from(initializer.index).expect("bounded initializer index");
            recursive.inherited.declarations[owner].values[initializer_index].operation =
                w::ValueOperation::Group {
                    value: compensation.retry.clone(),
                };
            let bytes = serde_json::to_vec(&recursive).expect("canonical adverse wire shape");
            assert_error(
                &inputs.read_v2_bytes(proofs, &bytes, &temporal.expected, Limits::default()),
                Error::Invalid(Invalid::Cycle),
            );
        },
    );
}

fn assert_error<T>(report: &artifact::Report<T>, expected: Error) {
    assert_eq!(report.result().err(), Some(&expected));
}

/// TC-143: FR-054-AC-1 through FR-054-AC-4.
#[trace("TC-143", "FR-054-AC-1", "FR-054-AC-2", "FR-054-AC-3", "FR-054-AC-4")]
#[test]
fn v3_control_temporal_activation_mapping_is_strict_and_non_inferential() {
    with_v2(|inputs, proofs, _, temporal, emitted| {
        let bytes = include_bytes!("../fixtures/compiled-protocol-v3-control-map.json");
        let package: v3::wire::Package =
            serde_json::from_slice(bytes).expect("authored canonical v3 wire fixture");
        let inherited = emitted.admitted().package().inherited.clone();
        let flow = u32::try_from(declaration(emitted.admitted(), "Flow")).expect("flow index");
        let controls = match &inherited.declarations[flow as usize].body {
            w::Body::Protocol { controls, .. } => controls,
            _ => panic!("fixture flow is protocol"),
        };
        let event = u32::try_from(
            controls
                .iter()
                .position(|control| matches!(control.operation, w::ControlOperation::Event { .. }))
                .expect("fixture event control"),
        )
        .expect("control index");
        let activation = v3::ExpectedActivation {
            control: w::Handle {
                declaration: flow,
                index: event,
            },
            temporal_declaration: u32::try_from(declaration(emitted.admitted(), "ByEvent"))
                .expect("temporal index"),
        };
        let out_of_range = w::Handle {
            declaration: flow,
            index: u32::try_from(controls.len())
                .expect("control count fits the handle")
                .checked_add(1)
                .expect("fixture control index remains representable"),
        };
        let non_event = u32::try_from(
            controls
                .iter()
                .position(|control| !matches!(control.operation, w::ControlOperation::Event { .. }))
                .expect("fixture non-event control"),
        )
        .expect("non-event control index");
        let non_event = w::Handle {
            declaration: flow,
            index: non_event,
        };
        assert_eq!(
            derive_v3_bytes(emitted, std::slice::from_ref(&activation)),
            bytes,
            "the control-map fixture is the canonical /3 derivation of the /2 admission"
        );
        let admitted = inputs
            .read_v3_bytes(
                proofs,
                bytes,
                &temporal.expected,
                std::slice::from_ref(&activation),
                Limits::default(),
            )
            .into_result()
            .expect("strict v3 reader");
        assert_eq!(
            admitted.activation_mappings(),
            package.activation_mappings.as_slice()
        );
        assert!(inputs
            .read_v3_bytes(
                proofs,
                emitted.bytes(),
                &temporal.expected,
                std::slice::from_ref(&activation),
                Limits::default(),
            )
            .result()
            .is_err());
        assert!(inputs
            .read_v2_bytes(proofs, bytes, &temporal.expected, Limits::default(),)
            .result()
            .is_err());
        assert!(inputs.read_bytes(proofs, bytes).result().is_err());
        for control in [out_of_range, non_event] {
            let mut invalid_control = package.clone();
            invalid_control.activation_mappings[0].control = control;
            let candidate = v3::encode_candidate(&invalid_control, Limits::default())
                .into_result()
                .expect("control mutations remain encodable candidates");
            assert_error(
                &inputs.read_v3_bytes(
                    proofs,
                    candidate.bytes(),
                    &temporal.expected,
                    std::slice::from_ref(&activation),
                    Limits::default(),
                ),
                Error::V3(v3::Refusal::Control),
            );
        }
        let mut non_temporal = package.clone();
        non_temporal.activation_mappings[0].temporal_declaration = flow;
        let non_temporal = v3::encode_candidate(&non_temporal, Limits::default())
            .into_result()
            .expect("non-temporal mutation remains encodable");
        assert_error(
            &inputs.read_v3_bytes(
                proofs,
                non_temporal.bytes(),
                &temporal.expected,
                std::slice::from_ref(&activation),
                Limits::default(),
            ),
            Error::V3(v3::Refusal::TemporalDeclaration),
        );
        let mut reordered = package.clone();
        let second = match &mut reordered.inherited.declarations[flow as usize].body {
            w::Body::Protocol { controls, .. } => {
                let cloned = controls[event as usize].clone();
                controls.push(cloned);
                w::Handle {
                    declaration: flow,
                    index: u32::try_from(controls.len() - 1).expect("appended control index"),
                }
            }
            _ => panic!("fixture flow remains protocol"),
        };
        let second = v3::ExpectedActivation {
            control: second,
            temporal_declaration: activation.temporal_declaration,
        };
        reordered.activation_mappings = vec![
            v3::wire::ActivationMapping {
                control: second.control.clone(),
                temporal_declaration: second.temporal_declaration,
            },
            v3::wire::ActivationMapping {
                control: activation.control.clone(),
                temporal_declaration: activation.temporal_declaration,
            },
        ];
        let reordered = v3::encode_candidate(&reordered, Limits::default())
            .into_result()
            .expect("canonical reordered mapping candidate");
        assert_error(
            &inputs.read_v3_bytes(
                proofs,
                reordered.bytes(),
                &temporal.expected,
                &[activation.clone(), second],
                Limits::default(),
            ),
            Error::V3(v3::Refusal::OfferOrder),
        );
        let mut substituted = package.clone();
        substituted.activation_mappings[0].temporal_declaration =
            u32::try_from(declaration(emitted.admitted(), "BySample"))
                .expect("other temporal index");
        let substituted = v3::encode_candidate(&substituted, Limits::default())
            .into_result()
            .expect("canonical substitute");
        assert_error(
            &inputs.read_v3_bytes(
                proofs,
                substituted.bytes(),
                &temporal.expected,
                std::slice::from_ref(&activation),
                Limits::default(),
            ),
            Error::V3(v3::Refusal::Expected(v3::MappingField::TemporalDeclaration)),
        );
        let mut missing = package.clone();
        missing.activation_mappings.clear();
        let missing = v3::encode_candidate(&missing, Limits::default())
            .into_result()
            .expect("canonical missing mapping");
        assert_error(
            &inputs.read_v3_bytes(
                proofs,
                missing.bytes(),
                &temporal.expected,
                std::slice::from_ref(&activation),
                Limits::default(),
            ),
            Error::V3(v3::Refusal::Mapping {
                side: v3::InventorySide::Offer,
                cause: v3::MappingCause::Missing,
            }),
        );
        let mut duplicate = package.clone();
        duplicate
            .activation_mappings
            .push(duplicate.activation_mappings[0].clone());
        let duplicate = v3::encode_candidate(&duplicate, Limits::default())
            .into_result()
            .expect("canonical duplicate mapping");
        assert_error(
            &inputs.read_v3_bytes(
                proofs,
                duplicate.bytes(),
                &temporal.expected,
                std::slice::from_ref(&activation),
                Limits::default(),
            ),
            Error::V3(v3::Refusal::Mapping {
                side: v3::InventorySide::Offer,
                cause: v3::MappingCause::Duplicate,
            }),
        );
    });
}

/// Checked protocol temporal evaluation over the retired native route
/// consumes the admitted control mapping and the observation owner's exact
/// trigger identities. The spine carries TC-144 and TC-149 in
/// `qsl-eval/tests/it/{checked_route_owner,checked_route_trigger}.rs` and in
/// `qsl-replay`'s `checked_route_owner`; this test is the legacy behavior
/// QSL-858 deletes with `src/temporal`.
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
    errors.extend(
        [
            Invalid::Selection,
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
    // A version-2 refusal keeps its own established spelling.
    assert_eq!(
        Error::V2(v2::Refusal::Header(v2::HeaderField::Wire)).code(),
        v2::Refusal::Header(v2::HeaderField::Wire).code()
    );
}

#[trace("TC-138", "TC-150", "TC-1277", "FR-050-AC-1", "FR-050-AC-4")]
#[test]
fn native_v2_emission_reaches_the_strict_reader_and_authenticated_l5_adapter() {
    with_v2(|inputs, proofs, selected, temporal, emitted| {
        let package = emitted.admitted().package();
        assert_eq!(
            emitted
                .admitted()
                .schema_model(0)
                .map(|model| model.artifact_bytes()),
            Some(selected.models[0].model.artifact_bytes())
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
        assert_eq!(admitted.package(), emitted.admitted().package());
        assert_eq!(
            admitted.schema_model(0).map(|model| model.artifact_bytes()),
            Some(selected.models[0].model.artifact_bytes())
        );
        // Emission produces bytes before any caller publishes an identity for
        // them; the reader retains the identity it independently admitted.
        assert!(emitted.admitted().artifact().is_none());
        assert!(admitted.artifact().is_some());

        let v1_before = native::admit(proofs, selected, Limits::default())
            .into_result()
            .expect("v1 admission");
        let v1_before = native::emit(&v1_before, Limits::default())
            .into_result()
            .expect("v1 emission");
        // FR-050-AC-4: `/1` stays readable by the strict `/1` reader. Its
        // bytes are not frozen: FR-042 changes the shared `Model` in place.
        assert!(inputs.read(proofs, &v1_before).result().is_ok());

        assert!(inputs.read_bytes(proofs, emitted.bytes()).result().is_err());
        assert!(inputs
            .read_v2_bytes(
                proofs,
                v1_before.bytes(),
                &temporal.expected,
                Limits::default(),
            )
            .result()
            .is_err());
    });
}

#[trace("TC-138", "TC-150", "FR-050-AC-2", "FR-050-AC-3")]
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

#[trace("TC-138", "TC-150", "FR-050-AC-2", "FR-050-AC-3", "FR-050-AC-4")]
#[test]
fn strict_reader_rejects_structural_and_identity_substitutions() {
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
            let mut out_of_range_declaration = base.clone();
            out_of_range_declaration.temporal_bindings[2].declaration = 9_999;
            let mut wrong_clock = base.clone();
            wrong_clock.temporal_bindings[0].clock =
                v2::wire::ClockConfiguration::TimestampedEvent {
                    timestamp_unit: "millisecond".into(),
                };
            let mut foreign_source = base.clone();
            let source_index = foreign_source.inherited.declarations
                [foreign_source.temporal_bindings[0].declaration as usize]
                .locus
                .source as usize;
            foreign_source.inherited.sources[source_index]
                .artifact
                .identity
                .push_str("-foreign");
            let mut unnamed_clock = base.clone();
            unnamed_clock.temporal_bindings[0].clock =
                v2::wire::ClockConfiguration::EventPosition {
                    sequence_authority: String::new(),
                };
            let mut changed_definition = base.clone();
            let index = changed_definition.temporal_bindings[0].definition as usize;
            changed_definition.inherited.definitions[index]
                .identity
                .push_str("-other");
            let mut changed_definition_revision = base.clone();
            let index = changed_definition_revision.temporal_bindings[0].definition as usize;
            changed_definition_revision.inherited.definitions[index]
                .revision
                .value
                .push_str("-other");
            let mut changed_definition_artifact = base.clone();
            let index = changed_definition_artifact.temporal_bindings[0].definition as usize;
            let artifact =
                changed_definition_artifact.inherited.definitions[index].artifact as usize;
            changed_definition_artifact.inherited.dependencies[artifact]
                .artifact
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
                    out_of_range_declaration,
                    v2_refusal(v2::Refusal::OfferIndex(v2::BindingIndex::Declaration)),
                ),
                (
                    wrong_clock,
                    v2_refusal(v2::Refusal::Clock(v2::ClockField::Alternative)),
                ),
                (
                    foreign_source,
                    v2_refusal(v2::Refusal::ForeignOwner(v2::SelectionSide::Expected)),
                ),
                (
                    unnamed_clock,
                    v2_refusal(v2::Refusal::Clock(v2::ClockField::SequenceAuthority)),
                ),
                (
                    changed_definition,
                    v2_refusal(v2::Refusal::Definition(v2::DefinitionField::Identity)),
                ),
                (
                    changed_definition_revision,
                    v2_refusal(v2::Refusal::Definition(v2::DefinitionField::Revision)),
                ),
                (
                    changed_definition_artifact,
                    v2_refusal(v2::Refusal::Definition(v2::DefinitionField::Artifact(
                        v2::ArtifactField::Identity,
                    ))),
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
                &inputs.read_v2_bytes(proofs, &bytes, temporal, Limits::default()),
                expected,
            );
        }

        assert_error(
            &inputs.read_v2_bytes(proofs, emitted.bytes(), &temporal[..2], Limits::default()),
            binding(v2::InventorySide::Expected, v2::BindingCause::Missing),
        );
        let mut surplus_expected = temporal.to_vec();
        surplus_expected.push(temporal[0].clone());
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
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
                &duplicate_expected,
                Limits::default(),
            ),
            binding(v2::InventorySide::Expected, v2::BindingCause::Duplicate),
        );
        let mut changed_span = temporal.to_vec();
        changed_span[0].declaration.span.start += 1;
        assert_error(
            &inputs.read_v2_bytes(proofs, emitted.bytes(), &changed_span, Limits::default()),
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
                &revision_expected,
                Limits::default(),
            ),
            v2_refusal(v2::Refusal::Definition(v2::DefinitionField::Revision)),
        );
        let mut foreign_owner = temporal.to_vec();
        foreign_owner[0].source = inputs.source_references[1].clone();
        assert_error(
            &inputs.read_v2_bytes(proofs, emitted.bytes(), &foreign_owner, Limits::default()),
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
            let mut wire_identity = original.clone();
            wire_identity.wire.identity.push_str("/canonical-json");
            let mut wire_version = original.clone();
            wire_version.wire.version.push_str("-other");
            vec![
                (v2::ArtifactField::RefVersion, ref_version),
                (v2::ArtifactField::Kind, kind),
                (v2::ArtifactField::Authority, authority),
                (v2::ArtifactField::Identity, identity),
                (v2::ArtifactField::WireIdentity, wire_identity),
                (v2::ArtifactField::WireVersion, wire_version),
            ]
        } {
            let mut changed = temporal.to_vec();
            changed[0].definition.artifact = artifact;
            assert_error(
                &inputs.read_v2_bytes(proofs, emitted.bytes(), &changed, Limits::default()),
                v2_refusal(v2::Refusal::Definition(v2::DefinitionField::Artifact(
                    field,
                ))),
            );
        }
    });
}

#[trace("TC-138", "TC-150", "FR-050-AC-2", "FR-050-AC-3", "FR-050-AC-4")]
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
                &inputs.read_v2_bytes(proofs, &bytes, temporal, Limits::default()),
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
                &inputs.read_v2_bytes(proofs, &bytes, temporal, Limits::default()),
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
                &inputs.read_v2_bytes(proofs, emitted.bytes(), &expected, Limits::default()),
                v2_refusal(v2::Refusal::Clock(*field)),
            );
        }
    });
}

#[trace("TC-138", "FR-050-AC-2", "FR-050-AC-3")]
#[test]
fn malformed_clock_objects_refuse() {
    with_v2(|inputs, proofs, _, temporal, emitted| {
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
                        .read_v2_bytes(proofs, &bytes, temporal, Limits::default(),)
                        .result(),
                    Err(Error::Json { .. })
                ));
            }
        }
    });
}

#[trace("TC-138", "TC-150", "FR-050-AC-5")]
#[test]
fn added_v2_work_is_exactly_bounded_and_a_fresh_retry_is_reproducible() {
    with_v2_inputs(
        checked_fr050_inputs(),
        |inputs, proofs, selected, temporal, emitted| {
            let tables = temporal;
            let temporal = &tables.expected;
            let complete =
                inputs.read_v2_bytes(proofs, emitted.bytes(), temporal, Limits::default());
            let usage = complete.usage();
            assert!(
                usage.byte_work >= emitted.bytes().len(),
                "byte work charges at least the emitted bytes"
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
                let report = inputs.read_v2_bytes(proofs, emitted.bytes(), temporal, limits);
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
                    .read_v2_bytes(proofs, emitted.bytes(), temporal, exact)
                    .result()
                    .is_ok());
            }
            let producer = native::admit_v2(proofs, selected, &tables.producer, Limits::default());
            let producer_usage = producer.usage();
            let producer_admitted = producer
                .result()
                .expect("producer path admits the exact selected artifact");
            let producer_dimensions = [
                (WorkDimension::PayloadBytes, producer_usage.payload_bytes),
                (WorkDimension::OutputBytes, producer_usage.output_bytes),
                (WorkDimension::SourceBytes, producer_usage.source_bytes),
                (WorkDimension::ContentBytes, producer_usage.content_bytes),
                (WorkDimension::Sources, producer_usage.sources),
                (WorkDimension::Dependencies, producer_usage.dependencies),
                (WorkDimension::Definitions, producer_usage.definitions),
                (WorkDimension::Models, producer_usage.models),
                (WorkDimension::Declarations, producer_usage.declarations),
                (WorkDimension::Entries, producer_usage.entries),
                (WorkDimension::References, producer_usage.references),
                (WorkDimension::ByteWork, producer_usage.byte_work),
                (WorkDimension::Depth, producer_usage.depth),
            ];
            for (dimension, amount) in producer_dimensions {
                if amount == 0 {
                    continue;
                }
                let set_limit = |limits: &mut Limits, value| match dimension {
                    WorkDimension::PayloadBytes => limits.payload_bytes = value,
                    WorkDimension::OutputBytes => limits.output_bytes = value,
                    WorkDimension::SourceBytes => limits.source_bytes = value,
                    WorkDimension::ContentBytes => limits.content_bytes = value,
                    WorkDimension::Sources => limits.sources = value,
                    WorkDimension::Dependencies => limits.dependencies = value,
                    WorkDimension::Definitions => limits.definitions = value,
                    WorkDimension::Models => limits.models = value,
                    WorkDimension::Declarations => limits.declarations = value,
                    WorkDimension::Entries => limits.entries = value,
                    WorkDimension::References => limits.references = value,
                    WorkDimension::ByteWork => limits.byte_work = value,
                    WorkDimension::Depth => limits.depth = value,
                };
                let mut one_short = Limits::default();
                set_limit(&mut one_short, amount - 1);
                let refused = native::admit_v2(proofs, selected, &tables.producer, one_short);
                let Error::Incomplete(exhaustion) = refused.result().unwrap_err() else {
                    panic!("producer {dimension:?}: expected typed exhaustion")
                };
                assert_eq!(exhaustion.dimension, dimension);

                let mut exact = Limits::default();
                set_limit(&mut exact, amount);
                assert!(
                    native::admit_v2(proofs, selected, &tables.producer, exact)
                        .result()
                        .is_ok(),
                    "producer {dimension:?} succeeds at its measured limit"
                );
            }
            assert!(
                producer_usage.content_bytes > 0,
                "producer charges strings/numbers"
            );
            assert_eq!(producer_usage.output_bytes, producer_admitted.bytes().len());
            assert!(producer_usage.byte_work >= producer_admitted.bytes().len());
            let producer_retry =
                native::admit_v2(proofs, selected, &tables.producer, Limits::default())
                    .into_result()
                    .expect("fresh producer retry");
            assert_eq!(producer_retry.bytes(), producer_admitted.bytes());
            assert_eq!(
                producer_retry.admitted().package(),
                producer_admitted.admitted().package()
            );
            assert_eq!(
                native::admit_v2(proofs, selected, &tables.producer, Limits::default()).usage(),
                producer_usage,
                "fresh producer retries start with reproducible usage"
            );

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
                ..Limits::default()
            };
            let clamped = inputs.read_v2_bytes(proofs, emitted.bytes(), temporal, above_hard);
            let hard_limits = Limits {
                payload_bytes: 64 * 1_048_576,
                output_bytes: 64 * 1_048_576,
                content_bytes: 64 * 1_048_576,
                entries: 10_000_000,
                byte_work: 512 * 1_048_576,
                depth: 1_000_000,
                ..Limits::default()
            };
            assert_eq!(clamped.limits(), hard_limits);
            assert!(clamped.result().is_ok());
            let reader_retry =
                inputs.read_v2_bytes(proofs, emitted.bytes(), temporal, Limits::default());
            assert_eq!(reader_retry.usage(), usage);
            assert_eq!(
                reader_retry.result().unwrap().package(),
                emitted.admitted().package()
            );
            let retry = inputs
                .read_v2(proofs, emitted, temporal)
                .into_result()
                .expect("fresh sufficient retry");
            assert_eq!(retry.package(), emitted.admitted().package());
        },
    );
}
