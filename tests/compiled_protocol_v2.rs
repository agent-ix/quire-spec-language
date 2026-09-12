// SPDX-License-Identifier: AGPL-3.0-only
//! TC-138: strict compiled-protocol v2 production, admission and L5 handoff.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeDisposition, TypeLimits};
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
use quire_spec_language::protocol_artifact::{
    self as artifact, native, v2, wire as w, Dimension as WorkDimension, Error, Invalid, Limits,
    NumberComponent, NumberError, NumberWire,
};
use quire_spec_language::temporal;
use quire_spec_language::ByteDigest;
use setup::{Inputs, Unit};

const DECLARATIONS: [(&str, R); 3] = [
    ("ByEvent", R::EventPosition),
    ("BySample", R::FixedSample),
    ("ByTimestamp", R::TimestampedWindow),
];

fn inputs() -> Inputs {
    Inputs::new(&[
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

fn with_v2(
    test: impl FnOnce(
        &Inputs,
        &proofs::ProofReport<'_, '_, '_>,
        &native::Selections<'_>,
        &[native::TemporalSelection<'_>],
        &native::AdmissionV2,
    ),
) {
    let inputs = inputs();
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
            let report = native::admit_v2(proofs, selected, &temporal, Limits::default());
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

fn assert_error<T>(report: &artifact::Report<T>, expected: Error) {
    assert_eq!(report.result().err(), Some(&expected));
}

#[test]
#[trace("TC-138", "FR-050-AC-1", "FR-050-AC-4", "FR-050-AC-6")]
fn native_v2_emission_reaches_the_strict_reader_and_authenticated_l5_adapter() {
    with_v2(|inputs, proofs, selected, temporal_selections, emitted| {
        let package = emitted.admitted().package();
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

        let read = inputs.read_v2(proofs, emitted, temporal_selections);
        let admitted = read.result().expect("strict v2 reader");
        assert_eq!(admitted.digest(), emitted.digest());
        assert_eq!(admitted.package(), emitted.admitted().package());

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
            assert!(
                temporal::evaluate_v2(admitted, at, &supplied, temporal::Limits::default())
                    .result()
                    .is_ok(),
                "{name} reaches evaluation"
            );
            assert!(temporal::mapping_support_v2(admitted, at, temporal::Closure::Open).is_ok());
        }

        let v1_before = native::admit(proofs, selected, Limits::default())
            .into_result()
            .expect("v1 admission");
        let v1_before = native::emit(&v1_before, Limits::default())
            .into_result()
            .expect("v1 emission");
        assert!(inputs.read(proofs, &v1_before).result().is_ok());
        let v1_after = native::admit(proofs, selected, Limits::default())
            .into_result()
            .expect("unchanged v1 admission");
        let v1_after = native::emit(&v1_after, Limits::default())
            .into_result()
            .expect("unchanged v1 emission");
        assert_eq!(v1_before.bytes(), v1_after.bytes());

        assert!(inputs
            .read_bytes(proofs, emitted.bytes(), emitted.digest())
            .result()
            .is_err());
        assert!(inputs
            .read_v2_bytes(
                proofs,
                v1_before.bytes(),
                v1_before.digest(),
                temporal_selections,
                Limits::default(),
            )
            .result()
            .is_err());
    });
}

#[test]
#[trace("TC-138", "FR-050-AC-2", "FR-050-AC-3")]
fn producer_requires_one_exact_source_definition_and_clock_selection() {
    with_v2(|inputs, proofs, selected, temporal, _| {
        assert_error(
            &native::admit_v2(proofs, selected, &temporal[..2], Limits::default()),
            Error::Invalid(Invalid::Inventory),
        );
        let mut duplicate = temporal.to_vec();
        duplicate[2] = duplicate[1];
        assert_error(
            &native::admit_v2(proofs, selected, &duplicate, Limits::default()),
            Error::Invalid(Invalid::Duplicate),
        );
        let mut foreign = temporal.to_vec();
        foreign[0].source = &inputs.source_references[1];
        assert_error(
            &native::admit_v2(proofs, selected, &foreign, Limits::default()),
            Error::Invalid(Invalid::Owner),
        );
        let wrong_clock = v2::wire::ClockConfiguration::TimestampedEvent {
            timestamp_unit: "millisecond".into(),
        };
        let mut wrong_profile = temporal.to_vec();
        wrong_profile[0].clock = &wrong_clock;
        assert_error(
            &native::admit_v2(proofs, selected, &wrong_profile, Limits::default()),
            Error::Invalid(Invalid::Profile),
        );
        let mut wrong_definition = temporal.to_vec();
        wrong_definition[0].definition_identity = R::FixedSample.identity();
        assert_error(
            &native::admit_v2(proofs, selected, &wrong_definition, Limits::default()),
            Error::Invalid(Invalid::Selection),
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
            let mut invalid = temporal.to_vec();
            invalid[1].clock = clock;
            assert_error(
                &native::admit_v2(proofs, selected, &invalid, Limits::default()),
                expected.clone(),
            );
        }
        let empty = v2::wire::ClockConfiguration::EventPosition {
            sequence_authority: String::new(),
        };
        let mut unnamed = temporal.to_vec();
        unnamed[0].clock = &empty;
        assert_error(
            &native::admit_v2(proofs, selected, &unnamed, Limits::default()),
            Error::Invalid(Invalid::Name),
        );
        let oversized = v2::wire::ClockConfiguration::TimestampedEvent {
            timestamp_unit: "x".repeat(4_097),
        };
        let mut overlong = temporal.to_vec();
        overlong[2].clock = &oversized;
        assert_error(
            &native::admit_v2(proofs, selected, &overlong, Limits::default()),
            Error::Invalid(Invalid::Name),
        );
    });
}

#[test]
#[trace("TC-138", "FR-050-AC-2", "FR-050-AC-3")]
fn strict_reader_rejects_resealed_structural_and_identity_substitutions() {
    with_v2(|inputs, proofs, _, temporal, emitted| {
        let base = emitted.admitted().package();
        let cases: Vec<(v2::wire::Package, Error)> = {
            let mut missing = base.clone();
            missing.temporal_bindings.pop();
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
            vec![
                (missing, Error::Invalid(Invalid::Inventory)),
                (duplicate, Error::Invalid(Invalid::Order)),
                (reordered, Error::Invalid(Invalid::Order)),
                (wrong_definition, Error::Invalid(Invalid::Profile)),
                (wrong_clock, Error::Invalid(Invalid::Profile)),
                (
                    changed_definition,
                    Error::Unsupported(artifact::Unsupported::Definition),
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
            Error::Invalid(Invalid::Inventory),
        );
        let changed_clock = v2::wire::ClockConfiguration::EventPosition {
            sequence_authority: "other-orders".into(),
        };
        let mut changed_expected = temporal.to_vec();
        changed_expected[0].clock = &changed_clock;
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                emitted.digest(),
                &changed_expected,
                Limits::default(),
            ),
            Error::Invalid(Invalid::Selection),
        );
        let mut substituted_revision = temporal[0].definition_revision.clone();
        substituted_revision.value.push_str("-other");
        let mut revision_expected = temporal.to_vec();
        revision_expected[0].definition_revision = &substituted_revision;
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                emitted.digest(),
                &revision_expected,
                Limits::default(),
            ),
            Error::Invalid(Invalid::Selection),
        );
        let mut foreign_owner = temporal.to_vec();
        foreign_owner[0].source = &inputs.source_references[1];
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                emitted.digest(),
                &foreign_owner,
                Limits::default(),
            ),
            Error::Invalid(Invalid::Owner),
        );
    });
}

#[test]
#[trace("TC-138", "FR-050-AC-5")]
fn added_v2_work_is_exactly_bounded_and_a_fresh_retry_is_reproducible() {
    with_v2(|inputs, proofs, _, temporal, emitted| {
        let complete = inputs.read_v2_bytes(
            proofs,
            emitted.bytes(),
            emitted.digest(),
            temporal,
            Limits::default(),
        );
        let usage = complete.usage();
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
        }
        let retry = inputs
            .read_v2(proofs, emitted, temporal)
            .into_result()
            .expect("fresh sufficient retry");
        assert_eq!(retry.package(), emitted.admitted().package());
    });
}

#[test]
#[trace("TC-138", "FR-050-AC-6")]
fn l5_refuses_each_parameter_axis_before_temporal_evaluation() {
    with_v2(|inputs, proofs, _, temporal_selections, emitted| {
        let admitted = inputs
            .read_v2(proofs, emitted, temporal_selections)
            .into_result()
            .unwrap();
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
            let report =
                temporal::evaluate_v2(&admitted, at, &supplied, temporal::Limits::default());
            assert!(matches!(
                report.result(),
                Err(temporal::Error::Refused(temporal::Refusal::Binding {
                    dimension: actual,
                    ..
                })) if *actual == dimension
            ));
            assert_eq!(report.usage().positions, 0);
        }
        let mut extra = trace_input("sample-clock", temporal::FIXED_SAMPLE, &matching);
        extra.clock.parameters.insert("extra".into(), "x".into());
        let report = temporal::evaluate_v2(&admitted, at, &extra, temporal::Limits::default());
        assert!(matches!(
            report.result(),
            Err(temporal::Error::Refused(temporal::Refusal::Binding {
                dimension: temporal::Dimension::ClockParameters,
                ..
            }))
        ));
        assert_eq!(report.usage().positions, 0);
    });
}

#[test]
#[trace("TC-138", "FR-050-AC-6")]
fn progress_is_partitioned_by_package_definition_and_exact_clock_configuration() {
    with_v2(|inputs, proofs, selected, temporal_selections, emitted| {
        let original = inputs
            .read_v2(proofs, emitted, temporal_selections)
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
        let mut changed_selections = temporal_selections.to_vec();
        changed_selections[1].clock = &changed_clock;
        let changed_emission =
            native::admit_v2(proofs, selected, &changed_selections, Limits::default())
                .into_result()
                .expect("changed exact configuration emission");
        assert_ne!(emitted.digest(), changed_emission.digest());
        let changed = inputs
            .read_v2(proofs, &changed_emission, &changed_selections)
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

#[test]
#[trace("TC-138", "FR-050-AC-3", "FR-050-AC-4")]
fn digest_domains_and_original_producer_bytes_cannot_be_reinterpreted() {
    with_v2(|inputs, proofs, selected, temporal_selections, emitted| {
        let definition_digest = temporal_selections[0].definition_artifact.digest;
        assert_ne!(definition_digest, emitted.digest());
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                definition_digest,
                temporal_selections,
                Limits::default(),
            ),
            Error::Invalid(Invalid::Seal),
        );

        let mut substituted_artifact = temporal_selections[0].definition_artifact.clone();
        substituted_artifact.digest = emitted.digest();
        let mut substituted_expected = temporal_selections.to_vec();
        substituted_expected[0].definition_artifact = &substituted_artifact;
        assert_error(
            &inputs.read_v2_bytes(
                proofs,
                emitted.bytes(),
                emitted.digest(),
                &substituted_expected,
                Limits::default(),
            ),
            Error::Invalid(Invalid::Selection),
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
            &native::admit_v2(proofs, &changed, temporal_selections, Limits::default()),
            Error::Invalid(Invalid::Seal),
        );
    });
}
