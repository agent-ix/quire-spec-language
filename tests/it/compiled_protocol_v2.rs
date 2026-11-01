// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-138: strict compiled-protocol v2 production, admission and L5 handoff.

use crate::support::native_protocol as setup;

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::{Source, SourceIdentity};
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::protocol_artifact::{
    self as artifact, native, v2, v3, wire as w, Dimension as WorkDimension, Error, Invalid,
    Limits, NumberComponent, NumberError, NumberWire, Unsupported,
};
use quire_spec_language::temporal;
use setup::{Inputs, TemporalDefinitionExpectation, TemporalExpectation, Unit};

const DECLARATIONS: [(&str, R); 3] = [
    ("ByEvent", R::EventPosition),
    ("BySample", R::FixedSample),
    ("ByTimestamp", R::TimestampedWindow),
];

#[cfg(feature = "handoff-writer")]
#[trace("TC-138", "TC-150", "FR-050-AC-1", "FR-050-AC-4", "FR-050-AC-7")]
#[test]
fn written_handoff_interchange_records_are_complete() {
    use artifact::handoff::{
        MUTATION_MANIFEST_FORMAT, PUBLISHED_MUTATION_MANIFEST_FILE, PUBLISHED_SELECTION_FILE,
    };
    use std::fs;
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path().join("handoff");
    artifact::handoff::write_v2(&root).expect("write_v2 emits a complete handoff");
    let root = root.as_path();
    let selection: artifact::handoff::SelectionV2 = serde_json::from_slice(
        &fs::read(root.join(PUBLISHED_SELECTION_FILE)).expect("written reader selection"),
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
        &fs::read(root.join(PUBLISHED_MUTATION_MANIFEST_FILE)).expect("written mutation manifest"),
    )
    .expect("decode mutations/manifest.json with the published type");
    assert_eq!(manifest.format, MUTATION_MANIFEST_FORMAT);
    assert!(!manifest.cases.is_empty(), "the adverse corpus has cases");
    let mut case_ids = manifest
        .cases
        .iter()
        .map(|case| case.identity.as_str())
        .collect::<Vec<_>>();
    case_ids.sort();
    case_ids.dedup();
    assert_eq!(case_ids.len(), manifest.cases.len(), "no duplicate case");
    let case_files = fs::read_dir(root.join("mutations"))
        .expect("written mutations directory")
        .map(|entry| entry.expect("mutation entry").file_name())
        .filter(|name| name.to_str() != Some("manifest.json"))
        .count();
    assert_eq!(
        case_files,
        manifest.cases.len(),
        "one written case file per manifest case"
    );
    let manifest_bytes =
        serde_json::to_vec(&manifest).expect("encode published mutation-manifest type");
    assert_eq!(
        serde_json::from_slice::<artifact::handoff::MutationManifest>(&manifest_bytes)
            .expect("round-trip published mutation-manifest type"),
        manifest
    );
    for path in [manifest.base_offer, manifest.independent_selection] {
        assert!(
            root.join(&path).is_file(),
            "manifest path {path} is not a published file"
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

fn checked_fr050_inputs() -> Inputs {
    let units = [
        Unit {
            name: "event-time",
            body: "temporal ByEvent using T over (view: M::Widget) clock \"event-clock\" on origin { eventually[0,0] holds(view.code > 0) }",
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
        "1",
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

fn checked_fr050_unit(inputs: &Inputs, index: usize) -> qsl_replay::spine::CheckedUnit {
    checked_fr050_unit_with_identity(
        inputs,
        index,
        SourceIdentity::new(
            inputs.sources[index].identity().authority.clone(),
            inputs.sources[index].identity().identity.clone(),
            inputs.sources[index].identity().revision_namespace.clone(),
            inputs.sources[index].identity().revision.clone(),
        ),
    )
}

fn checked_fr050_unit_with_identity(
    inputs: &Inputs,
    index: usize,
    identity: SourceIdentity,
) -> qsl_replay::spine::CheckedUnit {
    let model = include_bytes!("../fixtures/spine-model.semantic-ir.json");
    let packages = qsl_semantics::model::intake::package_input([model.as_slice()])
        .expect("admit the checked package model fixture");
    let source = &inputs.sources[index];
    crate::support::front_end::checked(
        identity,
        "fr050-checked-successor.native",
        source.text().as_bytes(),
        &packages,
        &qsl_replay::spine::DependencyInput::default(),
    )
    .expect("checked route admits the emitted native source")
}

#[trace("TC-150", "FR-050-AC-1", "FR-050-AC-6")]
#[test]
fn checked_successor_authenticates_each_emitted_profile_and_clock_before_evaluation() {
    with_v2_inputs(
        checked_fr050_inputs(),
        |inputs, proofs, _, tables, emitted| {
            let expected = inputs.read_v2(proofs, emitted, &tables.expected);
            let admitted = expected
                .into_result()
                .expect("strict reader admits the emitted v2 bytes");
            for (index, (clock_name, profile, parameters)) in [
                (
                    "event-clock",
                    temporal::EVENT_POSITION,
                    vec![("sequence_authority", "orders")],
                ),
                (
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
                    "timestamp-clock",
                    temporal::TIMESTAMPED_WINDOW,
                    vec![("timestamp_unit", "millisecond")],
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let checked = checked_fr050_unit(&inputs, index);
                let mut trace = trace_input(clock_name, profile, &parameters);
                let clause = checked.package().graph().temporal_clauses()[0].clause();
                let hold = clause
                    .formula()
                    .nodes()
                    .iter()
                    .position(|node| {
                        matches!(node, qsl_semantics::check::FormulaNode::Holds { .. })
                    })
                    .and_then(|index| u32::try_from(index).ok())
                    .expect("checked hold atom index");
                trace.positions.push(temporal::Position {
                    coordinate: 0,
                    order: None,
                    valuations: BTreeMap::from([(hold, true)]),
                    instance_valuations: BTreeMap::new(),
                });
                let raw = temporal::evaluate_checked(
                    checked.package(),
                    0,
                    &trace,
                    temporal::Limits::default(),
                );
                assert!(
                    raw.result().is_ok(),
                    "raw checked evaluation: {:?}",
                    raw.result()
                );
                let report = temporal::evaluate_checked_v2(
                    checked.package(),
                    &admitted,
                    0,
                    &trace,
                    temporal::Limits::default(),
                );
                assert!(
                    report.result().is_ok(),
                    "matching checked selection evaluates: {:?}",
                    report.result()
                );
                let binding = report.authenticated().expect("selection is retained");
                assert_eq!(
                    binding.declaration,
                    declaration(&admitted, DECLARATIONS[index].0)
                );
                assert_eq!(binding.clock, clock_name);
                assert_eq!(
                    binding.parameters,
                    parameters
                        .iter()
                        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                        .collect()
                );
                assert_eq!(
                    binding.definition_identity,
                    [R::EventPosition, R::FixedSample, R::TimestampedWindow][index].identity()
                );
                assert_eq!(
                    binding.definition_revision,
                    tables.expected[index].definition.revision.value
                );
                let binding_row = &admitted.package().temporal_bindings[index];
                let definition = &admitted.inherited().definitions
                    [usize::try_from(binding_row.definition).unwrap()];
                let definition_artifact = &admitted.inherited().dependencies
                    [usize::try_from(definition.artifact).unwrap()]
                .artifact;
                assert_eq!(
                    definition_artifact,
                    &tables.expected[index].definition.artifact
                );
                assert!(
                    report.usage().positions > 0,
                    "matching path evaluates formula positions"
                );
                let classification = temporal::mapping_support_checked_v2(
                    checked.package(),
                    &admitted,
                    0,
                    temporal::Closure::Open,
                )
                .expect("checked mapping retains the admitted selection");
                assert_eq!(
                    classification.authenticated.definition_identity(),
                    tables.expected[index].definition.identity
                );
                assert_eq!(
                    classification.authenticated.definition_revision(),
                    &tables.expected[index].definition.revision
                );
                assert_eq!(classification.clock_name, clock_name);
                assert_eq!(
                    classification.clock,
                    tables.expected[index].definition.clock
                );
                let mut ledger = temporal::Ledger::default();
                let mut established_trace = trace.clone();
                established_trace.watermark = 2;
                let progress_report = temporal::evaluate_checked_with_progress_v2(
                    checked.package(),
                    &admitted,
                    0,
                    &established_trace,
                    temporal::Limits::default(),
                    &mut ledger,
                );
                assert!(
                    progress_report.result().is_ok(),
                    "checked progress route evaluates with authenticated selection"
                );
                assert_eq!(progress_report.authenticated(), Some(binding));

                let mut wrong_profile = trace.clone();
                wrong_profile.clock.profile_identity = "quire.temporal.other/v1".into();
                let mut wrong_clock = trace.clone();
                wrong_clock.clock.name = "other-clock".into();
                let mut missing_parameter = trace.clone();
                missing_parameter.clock.parameters.remove(parameters[0].0);
                let mut extra_parameter = trace.clone();
                extra_parameter
                    .clock
                    .parameters
                    .insert("extra".into(), "unexpected".into());
                let substitutions: Vec<(&str, temporal::Trace, temporal::Dimension)> = match index {
                    0 => vec![(
                        "sequence authority",
                        {
                            let mut changed = trace.clone();
                            changed
                                .clock
                                .parameters
                                .insert("sequence_authority".into(), "other-orders".into());
                            changed
                        },
                        temporal::Dimension::SequenceAuthority,
                    )],
                    1 => vec![
                        (
                            "epoch",
                            {
                                let mut changed = trace.clone();
                                changed.clock.parameters.insert(
                                    "epoch".into(),
                                    "{\"kind\":\"integer\",\"decimal\":\"1\"}".into(),
                                );
                                changed
                            },
                            temporal::Dimension::Epoch,
                        ),
                        (
                            "period",
                            {
                                let mut changed = trace.clone();
                                changed.clock.parameters.insert(
                                        "period".into(),
                                        "{\"kind\":\"rational\",\"numerator\":\"1\",\"denominator\":\"3\"}".into(),
                                    );
                                changed
                            },
                            temporal::Dimension::SamplePeriod,
                        ),
                        (
                            "unit",
                            {
                                let mut changed = trace.clone();
                                changed
                                    .clock
                                    .parameters
                                    .insert("unit".into(), "minute".into());
                                changed
                            },
                            temporal::Dimension::ClockUnit,
                        ),
                    ],
                    2 => vec![(
                        "timestamp unit",
                        {
                            let mut changed = trace.clone();
                            changed
                                .clock
                                .parameters
                                .insert("timestamp_unit".into(), "nanosecond".into());
                            changed
                        },
                        temporal::Dimension::TimestampUnit,
                    )],
                    _ => unreachable!("three selected temporal profiles"),
                };
                let mut refusals = vec![
                    ("profile", wrong_profile, temporal::Dimension::Profile),
                    ("clock", wrong_clock, temporal::Dimension::Clock),
                    (
                        "missing parameter",
                        missing_parameter,
                        temporal::Dimension::ClockParameters,
                    ),
                    (
                        "extra parameter",
                        extra_parameter,
                        temporal::Dimension::ClockParameters,
                    ),
                ];
                refusals.extend(substitutions);
                for (axis, changed, expected_dimension) in refusals {
                    let refused = temporal::evaluate_checked_v2(
                        checked.package(),
                        &admitted,
                        0,
                        &changed,
                        temporal::Limits::default(),
                    );
                    assert!(
                        matches!(
                            refused.result(),
                            Err(temporal::Error::Refused(temporal::Refusal::Binding {
                                dimension,
                                ..
                            })) if *dimension == expected_dimension
                        ),
                        "{axis} mismatch has its exact typed refusal: {:?}",
                        refused.result()
                    );
                    assert_eq!(refused.usage().positions, 0, "{axis} refusal precedes work");
                    assert!(refused.authenticated().is_none(), "{axis} is not retained");

                    let mut changed_progress = changed.clone();
                    changed_progress.watermark = 3;
                    let progress_refused = temporal::evaluate_checked_with_progress_v2(
                        checked.package(),
                        &admitted,
                        0,
                        &changed_progress,
                        temporal::Limits::default(),
                        &mut ledger,
                    );
                    assert!(matches!(
                        progress_refused.result(),
                        Err(temporal::Error::Refused(temporal::Refusal::Binding {
                            dimension,
                            ..
                        })) if *dimension == expected_dimension
                    ));
                    assert_eq!(progress_refused.usage().positions, 0);
                    assert!(progress_refused.authenticated().is_none());

                    let mut regressed = established_trace.clone();
                    regressed.watermark = 1;
                    let retained = temporal::evaluate_checked_with_progress_v2(
                        checked.package(),
                        &admitted,
                        0,
                        &regressed,
                        temporal::Limits::default(),
                        &mut ledger,
                    );
                    assert!(matches!(
                        retained.result(),
                        Err(temporal::Error::Refused(temporal::Refusal::Progress {
                            dimension: temporal::Dimension::Watermark,
                            ..
                        }))
                    ));
                }
            }

            let wrong_source = checked_fr050_unit_with_identity(
                &inputs,
                0,
                SourceIdentity::new("other-authority", "unit:event-time", "test", "authored"),
            );
            let mut wrong_trace = trace_input(
                "event-clock",
                temporal::EVENT_POSITION,
                &[("sequence_authority", "orders")],
            );
            wrong_trace.positions.push(temporal::Position {
                coordinate: 0,
                order: None,
                valuations: BTreeMap::new(),
                instance_valuations: BTreeMap::new(),
            });
            let refused = temporal::evaluate_checked_v2(
                wrong_source.package(),
                &admitted,
                0,
                &wrong_trace,
                temporal::Limits::default(),
            );
            assert!(matches!(
                refused.result(),
                Err(temporal::Error::Refused(
                    temporal::Refusal::Reference { .. }
                ))
            ));
            assert_eq!(refused.usage().positions, 0);
            assert!(refused.authenticated().is_none());
        },
    );
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

/// TC-143: FR-054-AC-1 through FR-054-AC-4.
#[trace("TC-143", "FR-054-AC-1", "FR-054-AC-2", "FR-054-AC-3", "FR-054-AC-4")]
#[test]
fn v3_control_temporal_activation_mapping_is_strict_and_non_inferential() {
    with_v2(|inputs, proofs, _, temporal, emitted| {
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
        let produced = native::admit_v3(
            emitted,
            &[native::ActivationSelection {
                control: &activation.control,
                temporal_declaration: activation.temporal_declaration,
            }],
            Limits::default(),
        )
        .into_result()
        .expect("source-authorized canonical v3 emission");
        let package = produced.admitted().package().clone();
        let out_of_range = w::Handle {
            declaration: u32::MAX,
            index: 0,
        };
        assert_error(
            &native::admit_v3(
                emitted,
                &[native::ActivationSelection {
                    control: &out_of_range,
                    temporal_declaration: activation.temporal_declaration,
                }],
                Limits::default(),
            ),
            Error::V3(v3::Refusal::Control),
        );
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
        assert_error(
            &native::admit_v3(
                emitted,
                &[native::ActivationSelection {
                    control: &non_event,
                    temporal_declaration: activation.temporal_declaration,
                }],
                Limits::default(),
            ),
            Error::V3(v3::Refusal::Control),
        );
        assert_error(
            &native::admit_v3(
                emitted,
                &[native::ActivationSelection {
                    control: &activation.control,
                    temporal_declaration: flow,
                }],
                Limits::default(),
            ),
            Error::V3(v3::Refusal::TemporalDeclaration),
        );
        assert_error(
            &native::admit_v3(
                emitted,
                &[
                    native::ActivationSelection {
                        control: &activation.control,
                        temporal_declaration: activation.temporal_declaration,
                    },
                    native::ActivationSelection {
                        control: &activation.control,
                        temporal_declaration: activation.temporal_declaration,
                    },
                ],
                Limits::default(),
            ),
            Error::V3(v3::Refusal::Mapping {
                side: v3::InventorySide::Offer,
                cause: v3::MappingCause::Duplicate,
            }),
        );
        let admitted = inputs
            .read_v3_bytes(
                proofs,
                produced.bytes(),
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
            .read_v2_bytes(
                proofs,
                produced.bytes(),
                &temporal.expected,
                Limits::default(),
            )
            .result()
            .is_err());
        assert!(inputs
            .read_bytes(proofs, produced.bytes())
            .result()
            .is_err());
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
        let mut non_temporal = package;
        non_temporal.activation_mappings[0].temporal_declaration = flow;
        let non_temporal = v3::encode_candidate(&non_temporal, Limits::default())
            .into_result()
            .expect("canonical non-temporal mapping");
        assert_error(
            &inputs.read_v3_bytes(
                proofs,
                non_temporal.bytes(),
                &temporal.expected,
                &[activation],
                Limits::default(),
            ),
            Error::V3(v3::Refusal::TemporalDeclaration),
        );
    });
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

#[trace(
    "TC-138",
    "TC-150",
    "TC-1277",
    "FR-050-AC-1",
    "FR-050-AC-4",
    "FR-050-AC-6",
    "FR-043-AC-14"
)]
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
        |inputs, proofs, _, temporal, emitted| {
            let temporal = &temporal.expected;
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
            assert_eq!(clamped.limits(), Limits::default());
            assert!(clamped.result().is_ok());
            let retry = inputs
                .read_v2(proofs, emitted, temporal)
                .into_result()
                .expect("fresh sufficient retry");
            assert_eq!(retry.package(), emitted.admitted().package());
            let checked = checked_fr050_unit(inputs, 0);
            let mut trace = trace_input(
                "event-clock",
                temporal::EVENT_POSITION,
                &[("sequence_authority", "orders")],
            );
            let clause = checked.package().graph().temporal_clauses()[0].clause();
            let hold = clause
                .formula()
                .nodes()
                .iter()
                .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
                .and_then(|index| u32::try_from(index).ok())
                .expect("checked hold atom index");
            trace.positions.push(temporal::Position {
                coordinate: 0,
                order: None,
                valuations: BTreeMap::from([(hold, true)]),
                instance_valuations: BTreeMap::new(),
            });
            let checked_result = temporal::evaluate_checked_v2(
                checked.package(),
                &retry,
                0,
                &trace,
                temporal::Limits::default(),
            );
            assert!(
                checked_result.result().is_ok(),
                "fresh strict-read retry reaches its checked successor"
            );
            assert_eq!(checked_result.authenticated().unwrap().clock, "event-clock");
        },
    );
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

        for dimension in [temporal::Dimension::Profile, temporal::Dimension::Clock] {
            let mut supplied = trace_input("sample-clock", temporal::FIXED_SAMPLE, &matching);
            match dimension {
                temporal::Dimension::Profile => {
                    supplied.clock.profile_identity = temporal::EVENT_POSITION.into();
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

#[trace("TC-138", "TC-136", "FR-050-AC-6", "FR-049-AC-10")]
#[test]
fn progress_is_partitioned_by_definition_and_exact_clock_configuration() {
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
        assert_ne!(emitted.bytes(), changed_emission.bytes());
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
        assert_ne!(emitted.bytes(), other_emission.bytes());
        let mut other_trace = original_trace.clone();
        other_trace.watermark = 5;
        let same_selection_regression = temporal::evaluate_with_progress_v2(
            other_emission.admitted(),
            declaration(other_emission.admitted(), "BySample"),
            &other_trace,
            temporal::Limits::default(),
            &mut ledger,
        );
        assert!(matches!(
            same_selection_regression.result(),
            Err(temporal::Error::Refused(temporal::Refusal::Progress {
                dimension: temporal::Dimension::Watermark,
                ..
            }))
        ));

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
