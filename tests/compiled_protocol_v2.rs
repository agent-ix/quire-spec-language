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

#[trace("TC-138", "FR-050-AC-1", "FR-050-AC-4", "FR-050-AC-6")]
#[test]
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

#[trace("TC-138", "FR-050-AC-2", "FR-050-AC-3")]
#[test]
fn producer_requires_one_exact_source_definition_and_clock_selection() {
    with_v2(|inputs, proofs, selected, temporal, _| {
        assert_error(
            &native::admit_v2(proofs, selected, &temporal[..2], Limits::default()),
            binding(v2::InventorySide::Producer, v2::BindingCause::Missing),
        );
        let mut surplus = temporal.to_vec();
        surplus.push(temporal[0]);
        assert_error(
            &native::admit_v2(proofs, selected, &surplus, Limits::default()),
            binding(v2::InventorySide::Producer, v2::BindingCause::Surplus),
        );
        let mut duplicate = temporal.to_vec();
        duplicate[2] = duplicate[1];
        assert_error(
            &native::admit_v2(proofs, selected, &duplicate, Limits::default()),
            binding(v2::InventorySide::Producer, v2::BindingCause::Duplicate),
        );
        let mut foreign = temporal.to_vec();
        foreign[0].source = &inputs.source_references[1];
        assert_error(
            &native::admit_v2(proofs, selected, &foreign, Limits::default()),
            v2_refusal(v2::Refusal::ForeignOwner(v2::SelectionSide::Producer)),
        );
        let wrong_clock = v2::wire::ClockConfiguration::TimestampedEvent {
            timestamp_unit: "millisecond".into(),
        };
        let mut wrong_profile = temporal.to_vec();
        wrong_profile[0].clock = &wrong_clock;
        assert_error(
            &native::admit_v2(proofs, selected, &wrong_profile, Limits::default()),
            v2_refusal(v2::Refusal::Clock(v2::ClockField::Alternative)),
        );
        let mut wrong_definition = temporal.to_vec();
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
            v2_refusal(v2::Refusal::Clock(v2::ClockField::SequenceAuthority)),
        );
        let oversized = v2::wire::ClockConfiguration::TimestampedEvent {
            timestamp_unit: "x".repeat(4_097),
        };
        let mut overlong = temporal.to_vec();
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
            v2_refusal(v2::Refusal::Clock(v2::ClockField::SequenceAuthority)),
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
            v2_refusal(v2::Refusal::Definition(v2::DefinitionField::Revision)),
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
            v2_refusal(v2::Refusal::ForeignOwner(v2::SelectionSide::Expected)),
        );

        for (field, artifact) in {
            let original = temporal[0].definition_artifact;
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
            changed[0].definition_artifact = &artifact;
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
            expected[*index].clock = clock;
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

        let definition = temporal[0].definition_artifact;
        let at = selected
            .dependencies
            .iter()
            .position(|dependency| dependency.artifact == definition)
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
            &native::admit_v2(proofs, &changed, temporal, Limits::default()),
            Error::Invalid(Invalid::Seal),
        );
    });
}

#[trace("TC-138", "FR-050-AC-5")]
#[test]
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
            (4_813, 2_586, 2_311_843, 85_188)
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

#[trace("TC-138", "FR-050-AC-6")]
#[test]
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
            temporal_selections,
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
            &native::admit_v2(proofs, &changed, temporal_selections, Limits::default()),
            Error::Invalid(Invalid::Seal),
        );
    });
}
