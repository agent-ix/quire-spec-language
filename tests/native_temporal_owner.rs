// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-140: canonical formula-wide native temporal owner requests and results.

#[allow(
    dead_code,
    reason = "shared fixture exposes version-1 paths unused by this v2 owner suite"
)]
#[path = "support/native_protocol/mod.rs"]
mod setup;

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quire_spec_language::{
    checking::composed::{proofs, TypeDisposition, TypeLimits},
    linking::composed::definition_source::RegisteredDefinition,
    protocol_artifact::{
        self as artifact, native,
        native_temporal::{self, request, result, v2 as temporal_v2},
        v2, wire as w,
    },
    temporal, ByteDigest,
};
use serde_json::Value;
use setup::{Inputs, TemporalDefinitionExpectation, Unit};

#[derive(Clone, Copy)]
enum FixtureProfile {
    Event,
    FixedSample,
}

fn with_package(
    profile: FixtureProfile,
    formula: &str,
    test: impl FnOnce(&v2::AdmittedPackage, u32),
) {
    with_package_activation(profile, "on origin", formula, test);
}

fn with_package_activation(
    profile: FixtureProfile,
    activation: &str,
    formula: &str,
    test: impl FnOnce(&v2::AdmittedPackage, u32),
) {
    let profile_alias = match profile {
        FixtureProfile::Event => "T",
        FixtureProfile::FixedSample => "F",
    };
    let temporal_source = format!(
        "temporal ByTime using {profile_alias} over (view: M::Plain) clock \"orders\" {activation} {{ {formula} }}"
    );
    let inputs = Inputs::new(&[
        Unit {
            name: "temporal-evaluation",
            body: &temporal_source,
            declarations: &["ByTime"],
        },
        Unit {
            name: "temporal-consumer",
            body: "protocol Flow using P over (view: M::Plain) on origin {
                role Service on M::Node;
                requires temporal ByTime;
                run sequence Main {
                    event Happened by Service as (happened: M::Plain) { happened.ready };
                }
                finish Closed as (closed: M::Plain) { closed.ready };
            }",
            declarations: &["Flow"],
        },
    ]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            for entry in proofs.declarations() {
                let id = entry.declaration();
                assert_eq!(proofs.types().disposition(id), Some(TypeDisposition::Typed));
            }
            let [id] = proofs.types().binding().namespace().lookup("ByTime") else {
                panic!("one authored ByTime declaration")
            };
            let source_span = proofs
                .types()
                .binding()
                .namespace()
                .syntax(*id)
                .expect("authored ByTime syntax")
                .span;
            let span = w::Span {
                start: u32::try_from(source_span.start).expect("bounded source start"),
                end: u32::try_from(source_span.end).expect("bounded source end"),
            };
            let definition = match profile {
                FixtureProfile::Event => RegisteredDefinition::EventPosition,
                FixtureProfile::FixedSample => RegisteredDefinition::FixedSample,
            };
            let definition_artifact = selected
                .dependencies
                .iter()
                .find(|dependency| {
                    dependency.artifact.identity == definition.identity()
                        && dependency.bytes == definition.bytes()
                })
                .expect("selected event-position definition")
                .artifact;
            let revision = w::Revision {
                namespace: selected.definition_revision_namespace.into(),
                value: definition.revision().into(),
            };
            let clock = match profile {
                FixtureProfile::Event => v2::wire::ClockConfiguration::EventPosition {
                    sequence_authority: "orders".into(),
                },
                FixtureProfile::FixedSample => v2::wire::ClockConfiguration::FixedSample {
                    epoch: w::Number(artifact::NumberWire::Integer {
                        decimal: "0".into(),
                    }),
                    period: w::Number(artifact::NumberWire::Rational {
                        numerator: "1".into(),
                        denominator: "2".into(),
                    }),
                    unit: "second".into(),
                },
            };
            let producer = [native::TemporalSelection {
                source: &inputs.source_references[0],
                span: &span,
                definition_identity: definition.identity(),
                definition_revision: &revision,
                definition_artifact,
                clock: &clock,
            }];
            let expected = [inputs.temporal_expectation(
                proofs,
                0,
                "ByTime",
                TemporalDefinitionExpectation {
                    identity: definition.identity().into(),
                    revision: revision.clone(),
                    artifact: definition_artifact.clone(),
                    clock: clock.clone(),
                },
            )];
            let emission =
                native::admit_v2(proofs, selected, &producer, artifact::Limits::default())
                    .into_result()
                    .expect("admit strict version-2 package");
            let package = inputs
                .read_v2(proofs, &emission, &expected)
                .into_result()
                .expect("strict-read version-2 package");
            let declaration = package
                .inherited()
                .declarations
                .iter()
                .position(|entry| entry.name == "ByTime")
                .and_then(|value| u32::try_from(value).ok())
                .expect("ByTime declaration index");
            test(&package, declaration);
        },
    );
}

fn digest(label: &str) -> String {
    format!("{:x}", ByteDigest::of(label.as_bytes()))
}

fn evidence(scope: &str, identity: &str, population: u64) -> native_temporal::EvidenceRef {
    native_temporal::EvidenceRef::new(
        "quire.owner-evidence/v1",
        digest(&format!("schema:{scope}")),
        identity,
        digest(&format!("document:{identity}")),
        format!("authority:{scope}"),
        "1",
        digest(&format!("authority:{scope}")),
        scope,
        population,
    )
}

fn input(leaf: u32, value: bool, correspondence: &str) -> request::Input {
    let position = temporal::Position {
        coordinate: 0,
        order: Some(temporal::OrderKey {
            authority: "orders".into(),
            key: 0,
        }),
        valuations: BTreeMap::from([(leaf, value)]),
    };
    request::Input {
        instance: "execution:1".into(),
        correspondence: evidence("correspondence", correspondence, 1),
        positions: vec![request::ObservedPosition {
            observation: evidence("observation", "observation:0", 1),
            position,
        }],
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
        decision_progress: request::ProgressInput {
            reference: evidence("decision-progress", "decision-progress:1", 1),
            watermark: 0,
        },
        decision_closure: request::ClosureInput {
            reference: evidence("decision-closure", "decision-closure:1", 1),
            state: temporal::Closure::Closed,
        },
        surrounding_progress: request::ProgressInput {
            reference: evidence("surrounding-progress", "surrounding-progress:1", 1),
            watermark: 0,
        },
        surrounding_closure: request::ClosureInput {
            reference: evidence("surrounding-closure", "surrounding-closure:1", 1),
            state: temporal::Closure::Open,
        },
        execution: temporal::Execution::Completed,
        completeness: request::CompletenessInput {
            reference: evidence("completeness", "completeness:1", 1),
            state: temporal::Completeness::Complete,
            facts: vec![evidence("completeness-fact", "fact:observation:0", 1)],
        },
        authoritative_origin: true,
        evicted: Vec::new(),
    }
}

fn input_v2(leaf: u32, trigger: Vec<u8>) -> temporal_v2::Input {
    let request::Input {
        correspondence,
        positions,
        anchor,
        triggers,
        trigger_evidence,
        trigger_scope,
        decision_progress,
        decision_closure,
        surrounding_progress,
        surrounding_closure,
        execution,
        completeness,
        authoritative_origin,
        evicted,
        ..
    } = input(leaf, true, "correspondence:opaque-v2");
    temporal_v2::Input {
        trigger: temporal_v2::SemanticTriggerIdentity::new(trigger).expect("nonempty trigger"),
        correspondence,
        positions,
        anchor,
        triggers: triggers
            .into_iter()
            .map(|value| temporal_v2::TriggerInput {
                receipt: value.receipt,
                anchor: value.anchor,
                payload: value.payload,
                guard: value.guard,
                captures: value.captures,
            })
            .collect(),
        trigger_evidence,
        trigger_scope,
        decision_progress,
        decision_closure,
        surrounding_progress,
        surrounding_closure,
        execution,
        completeness,
        authoritative_origin,
        evicted,
    }
}

fn input_rows(nodes: &[u32], rows: &[Vec<bool>], correspondence: &str) -> request::Input {
    assert!(!nodes.is_empty());
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|row| row.len() == nodes.len()));
    let mut request = input(nodes[0], rows[0][0], correspondence);
    request.positions = rows
        .iter()
        .enumerate()
        .map(|(index, row)| request::ObservedPosition {
            observation: evidence(
                "observation",
                &format!("observation:{index}"),
                u64::try_from(nodes.len()).expect("small test leaf population"),
            ),
            position: temporal::Position {
                coordinate: i64::try_from(index).expect("small test coordinate"),
                order: Some(temporal::OrderKey {
                    authority: "orders".into(),
                    key: i64::try_from(index).expect("small test order key"),
                }),
                valuations: nodes.iter().copied().zip(row.iter().copied()).collect(),
            },
        })
        .collect();
    let watermark = i64::try_from(rows.len() - 1).expect("small test row population");
    request.decision_progress.watermark = watermark;
    request.surrounding_progress.watermark = watermark;
    request.completeness.facts = (0..rows.len())
        .map(|index| {
            evidence(
                "completeness-fact",
                &format!("fact:observation:{index}"),
                u64::try_from(nodes.len()).expect("small test leaf population"),
            )
        })
        .collect();
    request.completeness.reference = evidence(
        "completeness",
        "completeness:1",
        u64::try_from(rows.len()).expect("small test position population"),
    );
    request
}

fn checked_subject(
    package: &v2::AdmittedPackage,
    declaration: u32,
) -> artifact::temporal_subject::ValidatedTemporalSubject {
    let selection = artifact::temporal_subject::DeclarationSelection::new(declaration);
    let document = artifact::temporal_subject::derive(
        package,
        selection,
        artifact::temporal_subject::Limits::default(),
    )
    .into_result()
    .expect("derive temporal subject");
    artifact::temporal_subject::read(
        document.bytes(),
        package,
        selection,
        artifact::temporal_subject::Limits::default(),
    )
    .into_result()
    .expect("strict-read temporal subject")
}

fn leaf(package: &v2::AdmittedPackage, declaration: u32) -> u32 {
    package.inherited().declarations[usize::try_from(declaration).expect("test declaration index")]
        .temporal
        .iter()
        .position(|node| matches!(node.operation, w::TemporalOperation::Holds { .. }))
        .and_then(|value| u32::try_from(value).ok())
        .expect("one holds node")
}

fn leaves(package: &v2::AdmittedPackage, declaration: u32) -> Vec<u32> {
    package.inherited().declarations[usize::try_from(declaration).expect("test declaration index")]
        .temporal
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node.operation, w::TemporalOperation::Holds { .. }))
        .map(|(index, _)| u32::try_from(index).expect("bounded temporal arena"))
        .collect()
}

fn assert_schema(schema: &[u8], expected_digest: &str, bytes: &[u8]) {
    assert_eq!(format!("{:x}", ByteDigest::of(schema)), expected_digest);
    let schema: Value = serde_json::from_slice(schema).expect("schema JSON");
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .expect("schema compiles");
    let value: Value = serde_json::from_slice(bytes).expect("canonical document JSON");
    assert!(validator.is_valid(&value));
}

#[trace(
    "TC-140",
    "FR-052-AC-1",
    "FR-052-AC-2",
    "FR-052-AC-4",
    "FR-052-AC-5",
    "FR-052-AC-8"
)]
#[test]
fn canonical_request_and_formula_result_round_trip_without_leaf_coercion() {
    with_package(
        FixtureProfile::Event,
        "holds(view.ready)",
        |package, declaration| {
            let subject = checked_subject(package, declaration);
            let leaf = leaf(package, declaration);
            let document = request::produce(
                &subject,
                input(leaf, true, "correspondence:1"),
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("produce request");
            assert_schema(
                request::SCHEMA_BYTES,
                request::SCHEMA_SHA256,
                document.bytes(),
            );
            let request = request::read(
                document.bytes(),
                &subject,
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("strict-read request");
            assert_eq!(request.positions().len(), 1);
            assert_eq!(
                request
                    .positions()
                    .next()
                    .expect("position")
                    .valuations()
                    .len(),
                1
            );
            assert_eq!(
                request.decision_closure().state(),
                temporal::Closure::Closed
            );
            assert_eq!(
                request.surrounding_closure().state(),
                temporal::Closure::Open
            );

            let document = result::evaluate(
                &request,
                result::Relation::Original,
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("evaluate formula-wide result");
            assert_schema(
                result::SCHEMA_BYTES,
                result::SCHEMA_SHA256,
                document.bytes(),
            );
            let result = result::read(
                document.bytes(),
                &request,
                result::Relation::Original,
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("strict-read result by re-evaluation");
            assert_eq!(result.truth(), Some(result::Truth::True));
            assert_eq!(result.settlement(), Some(result::Settlement::ClosedScope));
            assert_eq!(result.support().count(), 1);
            assert_eq!(result.request_identity(), request.document().identity());
            assert_eq!(result.decision_closure().state(), temporal::Closure::Closed);
            assert_eq!(
                result.surrounding_closure().state(),
                temporal::Closure::Open
            );
            assert_eq!(
                result.completeness().state(),
                temporal::Completeness::Complete
            );
            assert_eq!(result.limits(), native_temporal::Limits::default());
        },
    );
}

#[trace("TC-140", "FR-052-AC-8")]
#[test]
fn public_owner_surface_is_closed_native_and_structurally_readable() {
    let surfaces = [
        include_str!("../src/protocol_artifact/native_temporal/mod.rs"),
        include_str!("../src/protocol_artifact/native_temporal/request.rs"),
        include_str!("../src/protocol_artifact/native_temporal/result.rs"),
    ];
    for surface in surfaces {
        for forbidden in [
            "quire_contract_ir",
            "tl_syntax",
            "Box<dyn",
            "dyn Fn",
            "parser_callback",
            "evaluator_callback",
            "network_client",
            "trust_override",
        ] {
            assert!(!surface.contains(forbidden), "forbidden API: {forbidden}");
        }
    }

    with_package(
        FixtureProfile::Event,
        "holds(view.ready)",
        |package, declaration| {
            let subject = checked_subject(package, declaration);
            let leaf = leaf(package, declaration);
            let request_document = request::produce(
                &subject,
                input(leaf, true, "correspondence:public-surface"),
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("produce request");
            let admitted_request = request::read(
                request_document.bytes(),
                &subject,
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("read request");

            assert_eq!(
                admitted_request.document().bytes(),
                request_document.bytes()
            );
            assert_eq!(
                admitted_request.subject_identity(),
                subject.document().identity()
            );
            assert_eq!(admitted_request.declaration(), declaration);
            assert_eq!(admitted_request.root(), subject.root());
            assert_eq!(admitted_request.instance(), "execution:1");
            assert_eq!(admitted_request.anchor(), "origin");
            assert!(!admitted_request.subject_digest().is_empty());
            assert!(!admitted_request.package_digest().is_empty());
            assert!(!admitted_request.profile_identity().is_empty());
            assert!(!admitted_request.profile_revision().is_empty());
            assert!(!admitted_request.clock_name().is_empty());
            let _clock = admitted_request.clock_configuration();
            assert_eq!(admitted_request.correspondence().population(), 1);
            let position = admitted_request.positions().next().expect("one position");
            assert!(!position.identity().is_empty());
            assert_eq!(position.observation().population(), 1);
            assert_eq!(position.coordinate(), 0);
            assert_eq!(position.order(), Some(("orders", 0)));
            assert_eq!(position.valuations().count(), 1);
            let trigger = admitted_request.triggers().next().expect("one trigger");
            assert_eq!(trigger.identity(), admitted_request.instance());
            assert_eq!(trigger.receipt(), "receipt:1");
            assert_eq!(trigger.anchor(), admitted_request.anchor());
            assert_eq!(trigger.payload(), "");
            assert_eq!(trigger.guard(), None);
            assert_eq!(trigger.captures().count(), 0);
            assert_eq!(
                admitted_request.trigger_evidence(),
                temporal::Evidence::Admitted
            );
            assert_eq!(admitted_request.trigger_scope(), temporal::Closure::Closed);
            assert_eq!(admitted_request.decision_progress().watermark(), 0);
            assert_eq!(
                admitted_request.decision_closure().state(),
                temporal::Closure::Closed
            );
            assert_eq!(admitted_request.surrounding_progress().watermark(), 0);
            assert_eq!(
                admitted_request.surrounding_closure().state(),
                temporal::Closure::Open
            );
            assert_eq!(admitted_request.completeness().facts().count(), 1);
            assert_eq!(admitted_request.execution(), temporal::Execution::Completed);
            assert!(admitted_request.authoritative_origin());

            let result_document = result::evaluate(
                &admitted_request,
                result::Relation::Original,
                admitted_request.limits(),
            )
            .into_result()
            .expect("evaluate result");
            let admitted_result = result::read(
                result_document.bytes(),
                &admitted_request,
                result::Relation::Original,
                admitted_request.limits(),
            )
            .into_result()
            .expect("read result");
            assert_eq!(admitted_result.document().bytes(), result_document.bytes());
            assert_eq!(admitted_result.relation(), result::RelationKind::Original);
            assert_eq!(
                admitted_result.request_identity(),
                admitted_request.document().identity()
            );
            assert_eq!(
                admitted_result.subject_identity(),
                admitted_request.subject_identity()
            );
            assert_eq!(admitted_result.instance(), admitted_request.instance());
            assert_eq!(
                admitted_result.correspondence(),
                admitted_request.correspondence()
            );
            assert_eq!(
                admitted_result.activation(),
                result::ActivationState::Active
            );
            assert_eq!(admitted_result.execution(), temporal::Execution::Completed);
            assert_eq!(admitted_result.truth(), Some(result::Truth::True));
            assert!(admitted_result.non_value().is_none());
            assert!(admitted_result.settlement().is_some());
            assert_eq!(admitted_result.support().count(), 1);
            assert_eq!(admitted_result.decision_progress().watermark(), 0);
            assert_eq!(
                admitted_result.decision_closure().state(),
                temporal::Closure::Closed
            );
            assert_eq!(admitted_result.surrounding_progress().watermark(), 0);
            assert_eq!(
                admitted_result.surrounding_closure().state(),
                temporal::Closure::Open
            );
            assert_eq!(admitted_result.completeness().facts().count(), 1);
            assert_eq!(admitted_result.limits(), admitted_request.limits());
            assert_eq!(admitted_result.predecessor(), None);
            assert_eq!(admitted_result.lineage(), 0);
        },
    );
}

#[trace("TC-140", "FR-052-AC-6")]
#[test]
fn corrections_are_direct_immutable_and_require_changed_input() {
    with_package(
        FixtureProfile::Event,
        "holds(view.ready)",
        |package, declaration| {
            let subject = checked_subject(package, declaration);
            let leaf = leaf(package, declaration);
            let original_request_document = request::produce(
                &subject,
                input(leaf, true, "correspondence:1"),
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            let original_request = request::read(
                original_request_document.bytes(),
                &subject,
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            let original_document = result::evaluate(
                &original_request,
                result::Relation::Original,
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            let original = result::read(
                original_document.bytes(),
                &original_request,
                result::Relation::Original,
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();

            assert_eq!(
                result::evaluate(
                    &original_request,
                    result::Relation::Superseding(&original),
                    native_temporal::Limits::default()
                )
                .result()
                .expect_err("unchanged request must refuse")
                .code(),
                native_temporal::ErrorCode::InvalidRelation,
            );

            let corrected_request_document = request::produce(
                &subject,
                input(leaf, false, "correspondence:1"),
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            let corrected_request = request::read(
                corrected_request_document.bytes(),
                &subject,
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            let corrected_document = result::evaluate(
                &corrected_request,
                result::Relation::Superseding(&original),
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            let corrected = result::read(
                corrected_document.bytes(),
                &corrected_request,
                result::Relation::Superseding(&original),
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            assert_eq!(corrected.document().revision(), 2);
            assert_eq!(corrected.truth(), Some(result::Truth::False));
            assert_eq!(
                corrected.predecessor().map(|value| value.0),
                Some(original.document().identity())
            );
            assert_eq!(original.document().bytes(), original_document.bytes());
        },
    );
}

#[trace("TC-140", "FR-052-AC-7")]
#[test]
fn strict_readers_reject_noncanonical_duplicate_trailing_and_bounded_inputs() {
    with_package(
        FixtureProfile::Event,
        "holds(view.ready)",
        |package, declaration| {
            let subject = checked_subject(package, declaration);
            let leaf = leaf(package, declaration);
            let document = request::produce(
                &subject,
                input(leaf, true, "correspondence:1"),
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            let mut trailing = document.bytes().to_vec();
            trailing.extend_from_slice(b" ");
            assert!(
                request::read(&trailing, &subject, native_temporal::Limits::default())
                    .result()
                    .is_err()
            );
            let mut duplicate = b"{\"contract\":\"quire.native-temporal-request/v1\",".to_vec();
            duplicate.extend_from_slice(&document.bytes()[1..]);
            assert!(
                request::read(&duplicate, &subject, native_temporal::Limits::default())
                    .result()
                    .is_err()
            );
            let limited = native_temporal::Limits {
                positions: 0,
                ..native_temporal::Limits::default()
            };
            assert_eq!(
                request::produce(&subject, input(leaf, true, "correspondence:1"), limited)
                    .result()
                    .expect_err("one over position limit")
                    .code(),
                native_temporal::ErrorCode::ResourceIncomplete,
            );
        },
    );
}

fn owner_truth(profile: FixtureProfile, formula: &str, rows: &[Vec<bool>]) -> result::Truth {
    let mut observed = None;
    with_package(profile, formula, |package, declaration| {
        let subject = checked_subject(package, declaration);
        let nodes = leaves(package, declaration);
        let request_document = request::produce(
            &subject,
            input_rows(&nodes, rows, "correspondence:semantic-vector"),
            native_temporal::Limits::default(),
        )
        .into_result()
        .expect("produce semantic-vector request");
        let request = request::read(
            request_document.bytes(),
            &subject,
            native_temporal::Limits::default(),
        )
        .into_result()
        .expect("read semantic-vector request");
        let result_document = result::evaluate(
            &request,
            result::Relation::Original,
            native_temporal::Limits::default(),
        )
        .into_result()
        .expect("evaluate semantic vector");
        let result = result::read(
            result_document.bytes(),
            &request,
            result::Relation::Original,
            native_temporal::Limits::default(),
        )
        .into_result()
        .expect("read semantic-vector result");
        observed = result.truth();
    });
    observed.expect("semantic vector returns truth")
}

#[trace("TC-140", "FR-052-AC-3")]
#[test]
fn future_past_and_fixed_sample_vectors_preserve_native_semantics() {
    assert_eq!(
        owner_truth(
            FixtureProfile::Event,
            "eventually[0,2] holds(view.ready)",
            &[vec![false], vec![true], vec![false]],
        ),
        result::Truth::True,
    );
    assert_eq!(
        owner_truth(
            FixtureProfile::FixedSample,
            "eventually[0,2] holds(view.ready)",
            &[vec![false], vec![true], vec![false]],
        ),
        result::Truth::True,
    );
    for (formula, rows, expected) in [
        (
            "once[0,0] holds(view.ready)",
            vec![vec![true]],
            result::Truth::True,
        ),
        (
            "historically[0,0] holds(view.ready)",
            vec![vec![false]],
            result::Truth::False,
        ),
        (
            "once[1,1] holds(view.ready)",
            vec![vec![true]],
            result::Truth::False,
        ),
        (
            "holds(view.ready) since[0,0] holds(view.ready)",
            vec![vec![false, true]],
            result::Truth::True,
        ),
        (
            "holds(view.ready) triggered[0,0] holds(view.ready)",
            vec![vec![false, false]],
            result::Truth::False,
        ),
    ] {
        assert_eq!(
            owner_truth(FixtureProfile::Event, formula, &rows),
            expected,
            "{formula}"
        );
    }
}

#[trace("TC-140", "FR-052-AC-5")]
#[test]
fn axes_and_completeness_remain_independent_in_request_and_result_identities() {
    with_package(
        FixtureProfile::Event,
        "always[0,1] holds(view.ready)",
        |package, declaration| {
            let subject = checked_subject(package, declaration);
            let leaf = leaf(package, declaration);
            let mut identities = Vec::new();
            for decision in [temporal::Closure::Open, temporal::Closure::Closed] {
                for surrounding in [temporal::Closure::Open, temporal::Closure::Closed] {
                    for execution in [temporal::Execution::Completed, temporal::Execution::Failed] {
                        for completeness in [
                            temporal::Completeness::Complete,
                            temporal::Completeness::Incomplete,
                        ] {
                            let mut supplied = input(leaf, true, "correspondence:axes");
                            supplied.decision_closure.state = decision;
                            supplied.surrounding_closure.state = surrounding;
                            supplied.execution = execution;
                            supplied.completeness.state = completeness;
                            let document = request::produce(
                                &subject,
                                supplied,
                                native_temporal::Limits::default(),
                            )
                            .into_result()
                            .unwrap();
                            let request = request::read(
                                document.bytes(),
                                &subject,
                                native_temporal::Limits::default(),
                            )
                            .into_result()
                            .unwrap();
                            assert_eq!(request.decision_closure().state(), decision);
                            assert_eq!(request.surrounding_closure().state(), surrounding);
                            assert_eq!(request.execution(), execution);
                            assert_eq!(request.completeness().state(), completeness);
                            let result = result::evaluate(
                                &request,
                                result::Relation::Original,
                                native_temporal::Limits::default(),
                            )
                            .into_result()
                            .unwrap();
                            identities.push((
                                document.identity().to_owned(),
                                result.identity().to_owned(),
                            ));
                        }
                    }
                }
            }
            identities.sort();
            identities.dedup();
            assert_eq!(identities.len(), 16);
        },
    );
}

#[trace("TC-140", "FR-052-AC-2", "FR-052-AC-3", "FR-052-AC-7")]
#[test]
fn missing_native_support_is_a_typed_non_value_and_result_mutation_refuses() {
    with_package(
        FixtureProfile::Event,
        "holds(view.ready)",
        |package, declaration| {
            let subject = checked_subject(package, declaration);
            let leaf = leaf(package, declaration);
            let mut supplied = input(leaf, true, "correspondence:non-value");
            supplied.evicted.push(temporal::Eviction::Valuation {
                node: leaf,
                coordinate: 0,
            });
            let request_document =
                request::produce(&subject, supplied, native_temporal::Limits::default())
                    .into_result()
                    .unwrap();
            let request = request::read(
                request_document.bytes(),
                &subject,
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            let result_document = result::evaluate(
                &request,
                result::Relation::Original,
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            let result = result::read(
                result_document.bytes(),
                &request,
                result::Relation::Original,
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            assert_eq!(result.truth(), None);
            let non_value = result.non_value().expect("typed missing support");
            assert_eq!(non_value.kind(), result::NonValueKind::Missing);
            assert_eq!(non_value.dimension(), Some("valuation"));

            let mut changed: Value = serde_json::from_slice(result_document.bytes()).unwrap();
            changed["truth"] = Value::String("false".into());
            let changed = serde_json::to_vec(&changed).unwrap();
            assert!(result::read(
                &changed,
                &request,
                result::Relation::Original,
                native_temporal::Limits::default()
            )
            .result()
            .is_err());
        },
    );
}

#[trace("TC-140", "FR-052-AC-1", "FR-052-AC-4", "FR-052-AC-7")]
#[test]
fn request_canonicalization_is_order_invariant_and_cross_wiring_refuses() {
    with_package(
        FixtureProfile::Event,
        "eventually[0,1] holds(view.ready)",
        |package, declaration| {
            let subject = checked_subject(package, declaration);
            let nodes = leaves(package, declaration);
            let supplied = input_rows(&nodes, &[vec![false], vec![true]], "correspondence:order");
            let forward = request::produce(
                &subject,
                supplied.clone(),
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            let mut reversed = supplied.clone();
            reversed.positions.reverse();
            let reversed = request::produce(&subject, reversed, native_temporal::Limits::default())
                .into_result()
                .unwrap();
            assert_eq!(forward.bytes(), reversed.bytes());

            let mut cross_wired = supplied.clone();
            std::mem::swap(
                &mut cross_wired.decision_progress.reference,
                &mut cross_wired.surrounding_progress.reference,
            );
            assert_eq!(
                request::produce(&subject, cross_wired, native_temporal::Limits::default(),)
                    .result()
                    .expect_err("axis scope substitution must refuse")
                    .code(),
                native_temporal::ErrorCode::InvalidInput,
            );

            let mut missing = supplied;
            missing.positions[0].position.valuations.clear();
            assert_eq!(
                request::produce(&subject, missing, native_temporal::Limits::default())
                    .result()
                    .expect_err("missing explicit leaf valuation must refuse")
                    .code(),
                native_temporal::ErrorCode::InvalidInput,
            );
        },
    );
}

#[trace("TC-140", "FR-052-AC-2", "FR-052-AC-5")]
#[test]
fn inactive_unknown_and_pending_are_distinct_non_boolean_outcomes() {
    with_package(
        FixtureProfile::Event,
        "eventually[0,1] holds(view.ready)",
        |package, declaration| {
            let subject = checked_subject(package, declaration);
            let leaf = leaf(package, declaration);
            for (
                name,
                evidence_state,
                trigger_scope,
                completeness,
                expected_activation,
                expected_non_value,
            ) in [
                (
                    "inactive",
                    temporal::Evidence::Admitted,
                    temporal::Closure::Closed,
                    temporal::Completeness::Complete,
                    result::ActivationState::Inactive,
                    result::NonValueKind::Inactive,
                ),
                (
                    "unknown",
                    temporal::Evidence::Missing,
                    temporal::Closure::Open,
                    temporal::Completeness::Incomplete,
                    result::ActivationState::Unknown,
                    result::NonValueKind::ActivationUnknown,
                ),
            ] {
                let mut supplied = input(leaf, false, &format!("correspondence:{name}"));
                supplied.triggers.clear();
                supplied.trigger_evidence = evidence_state;
                supplied.trigger_scope = trigger_scope;
                supplied.completeness.state = completeness;
                let request_document =
                    request::produce(&subject, supplied, native_temporal::Limits::default())
                        .into_result()
                        .unwrap();
                let request = request::read(
                    request_document.bytes(),
                    &subject,
                    native_temporal::Limits::default(),
                )
                .into_result()
                .unwrap();
                let result_document = result::evaluate(
                    &request,
                    result::Relation::Original,
                    native_temporal::Limits::default(),
                )
                .into_result()
                .unwrap();
                let result = result::read(
                    result_document.bytes(),
                    &request,
                    result::Relation::Original,
                    native_temporal::Limits::default(),
                )
                .into_result()
                .unwrap();
                assert_eq!(result.activation(), expected_activation, "{name}");
                assert_eq!(result.truth(), None, "{name}");
                assert_eq!(
                    result.non_value().map(|value| value.kind()),
                    Some(expected_non_value),
                    "{name}",
                );
            }

            let mut supplied = input(leaf, false, "correspondence:pending");
            supplied.decision_closure.state = temporal::Closure::Open;
            supplied.completeness.state = temporal::Completeness::Incomplete;
            let request_document =
                request::produce(&subject, supplied, native_temporal::Limits::default())
                    .into_result()
                    .unwrap();
            let request = request::read(
                request_document.bytes(),
                &subject,
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            let result_document = result::evaluate(
                &request,
                result::Relation::Original,
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            let result = result::read(
                result_document.bytes(),
                &request,
                result::Relation::Original,
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            assert_eq!(result.activation(), result::ActivationState::Active);
            assert_eq!(result.truth(), Some(result::Truth::Pending));
            assert!(result.non_value().is_none());
            assert_eq!(result.settlement(), Some(result::Settlement::Unsettled));
        },
    );
}

#[trace("TC-140", "FR-052-AC-6", "FR-052-AC-7")]
#[test]
fn owner_limits_are_effective_and_correction_lineage_is_bounded() {
    with_package(
        FixtureProfile::Event,
        "holds(view.ready)",
        |package, declaration| {
            let subject = checked_subject(package, declaration);
            let leaf = leaf(package, declaration);
            let default = native_temporal::Limits::default();
            let document = request::produce(
                &subject,
                input(leaf, true, "correspondence:limits"),
                default,
            )
            .into_result()
            .unwrap();

            let exact_input = native_temporal::Limits {
                input_bytes: document.bytes().len(),
                ..default
            };
            assert!(request::read(document.bytes(), &subject, exact_input)
                .result()
                .is_ok());
            let one_short_input = native_temporal::Limits {
                input_bytes: document.bytes().len() - 1,
                ..default
            };
            assert_eq!(
                request::read(document.bytes(), &subject, one_short_input)
                    .result()
                    .expect_err("one byte below exact input must stop")
                    .code(),
                native_temporal::ErrorCode::ResourceIncomplete,
            );
            let shallow = native_temporal::Limits {
                json_depth: 1,
                ..default
            };
            assert_eq!(
                request::read(document.bytes(), &subject, shallow)
                    .result()
                    .expect_err("JSON depth limit must stop")
                    .code(),
                native_temporal::ErrorCode::ResourceIncomplete,
            );

            for limits in [
                native_temporal::Limits {
                    output_bytes: 1,
                    ..default
                },
                native_temporal::Limits {
                    string_bytes: 4,
                    ..default
                },
                native_temporal::Limits {
                    formula_nodes: 0,
                    ..default
                },
                native_temporal::Limits {
                    formula_depth: 0,
                    ..default
                },
                native_temporal::Limits {
                    positions: 0,
                    ..default
                },
                native_temporal::Limits {
                    valuations: 0,
                    ..default
                },
                native_temporal::Limits {
                    visited: 1,
                    ..default
                },
            ] {
                assert_eq!(
                    request::produce(&subject, input(leaf, true, "correspondence:limits"), limits,)
                        .result()
                        .expect_err("lowered request limit must stop")
                        .code(),
                    native_temporal::ErrorCode::ResourceIncomplete,
                );
            }

            for limits in [
                native_temporal::Limits {
                    evaluation_steps: 0,
                    ..default
                },
                native_temporal::Limits {
                    support: 0,
                    ..default
                },
            ] {
                let limited_document = request::produce(
                    &subject,
                    input(leaf, true, "correspondence:limited-evaluation"),
                    limits,
                )
                .into_result()
                .unwrap();
                let limited = request::read(limited_document.bytes(), &subject, limits)
                    .into_result()
                    .unwrap();
                assert_eq!(
                    result::evaluate(&limited, result::Relation::Original, limits)
                        .result()
                        .expect_err("lowered evaluator limit must stop")
                        .code(),
                    native_temporal::ErrorCode::ResourceIncomplete,
                );
            }

            let request = request::read(document.bytes(), &subject, default)
                .into_result()
                .unwrap();
            let original_document = result::evaluate(&request, result::Relation::Original, default)
                .into_result()
                .unwrap();
            let original = result::read(
                original_document.bytes(),
                &request,
                result::Relation::Original,
                default,
            )
            .into_result()
            .unwrap();
            let no_lineage = native_temporal::Limits {
                lineage: 0,
                ..default
            };
            let corrected_document = request::produce(
                &subject,
                input(leaf, false, "correspondence:limits"),
                no_lineage,
            )
            .into_result()
            .unwrap();
            let corrected = request::read(corrected_document.bytes(), &subject, no_lineage)
                .into_result()
                .unwrap();
            assert_eq!(
                result::evaluate(
                    &corrected,
                    result::Relation::Invalidating(&original),
                    no_lineage,
                )
                .result()
                .expect_err("correction must honor lineage ceiling")
                .code(),
                native_temporal::ErrorCode::ResourceIncomplete,
            );
        },
    );

    with_package(
        FixtureProfile::Event,
        "eventually[0,2] holds(view.ready)",
        |package, declaration| {
            let subject = checked_subject(package, declaration);
            let leaf = leaf(package, declaration);
            let limits = native_temporal::Limits {
                history_span: 1,
                ..native_temporal::Limits::default()
            };
            assert_eq!(
                request::produce(
                    &subject,
                    input(leaf, true, "correspondence:history"),
                    limits,
                )
                .result()
                .expect_err("history span below interval must stop")
                .code(),
                native_temporal::ErrorCode::ResourceIncomplete,
            );
        },
    );
}

#[trace("TC-140", "FR-052-AC-1", "FR-052-AC-4", "FR-052-AC-7")]
#[test]
fn activation_captures_are_immutable_identity_inputs_and_bounded() {
    with_package_activation(
        FixtureProfile::Event,
        "on each (started: M::Node)",
        "capture saved: M::Version = started.n; holds(view.ready)",
        |package, declaration| {
            let subject = checked_subject(package, declaration);
            let leaf = leaf(package, declaration);
            let mut first = input(leaf, true, "correspondence:capture");
            first.authoritative_origin = false;
            first.triggers[0].captures = vec![temporal::CaptureInput::Value {
                anchor: "origin".into(),
                value: "version:1".into(),
            }];
            let first_document =
                request::produce(&subject, first.clone(), native_temporal::Limits::default())
                    .into_result()
                    .unwrap();
            let first_request = request::read(
                first_document.bytes(),
                &subject,
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            let result = result::evaluate(
                &first_request,
                result::Relation::Original,
                native_temporal::Limits::default(),
            )
            .into_result()
            .unwrap();
            assert_ne!(result.identity(), first_document.identity());

            let mut changed = first;
            changed.triggers[0].captures[0] = temporal::CaptureInput::Value {
                anchor: "origin".into(),
                value: "version:2".into(),
            };
            let changed = request::produce(&subject, changed, native_temporal::Limits::default())
                .into_result()
                .unwrap();
            assert_ne!(first_document.identity(), changed.identity());

            let mut bounded = input(leaf, true, "correspondence:capture");
            bounded.authoritative_origin = false;
            bounded.triggers[0].captures = vec![temporal::CaptureInput::Value {
                anchor: "origin".into(),
                value: "version:1".into(),
            }];
            let limits = native_temporal::Limits {
                captures: 0,
                ..native_temporal::Limits::default()
            };
            assert_eq!(
                request::produce(&subject, bounded, limits)
                    .result()
                    .expect_err("capture one over limit must stop")
                    .code(),
                native_temporal::ErrorCode::ResourceIncomplete,
            );
        },
    );
}

#[trace("TC-141", "FR-053-AC-1", "FR-053-AC-2", "FR-053-AC-3")]
#[trace("FR-053-AC-4", "FR-053-AC-5", "FR-053-AC-6")]
#[test]
fn opaque_trigger_v2_round_trips_and_rejects_substitution_and_cross_version_documents() {
    with_package(
        FixtureProfile::Event,
        "holds(view.ready)",
        |package, declaration| {
            let subject = checked_subject(package, declaration);
            let leaf = leaf(package, declaration);
            let mut first_input = input_v2(leaf, vec![0, 0xff, b'/', 0x80]);
            first_input.decision_progress.watermark = 17;
            first_input.decision_closure.state = temporal::Closure::Open;
            first_input.surrounding_progress.watermark = 23;
            first_input.surrounding_closure.state = temporal::Closure::Closed;
            first_input.execution = temporal::Execution::Failed;
            first_input.completeness.state = temporal::Completeness::Incomplete;
            let expected_decision_progress = first_input.decision_progress.clone();
            let expected_decision_closure = first_input.decision_closure.clone();
            let expected_surrounding_progress = first_input.surrounding_progress.clone();
            let expected_surrounding_closure = first_input.surrounding_closure.clone();
            let expected_execution = first_input.execution;
            let expected_completeness = first_input.completeness.clone();
            let first = temporal_v2::produce(
                &subject,
                first_input.clone(),
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("produce v2 request");
            assert_schema(
                temporal_v2::REQUEST_SCHEMA_BYTES,
                temporal_v2::REQUEST_SCHEMA_SHA256,
                first.bytes(),
            );
            let request =
                temporal_v2::read(first.bytes(), &subject, native_temporal::Limits::default())
                    .into_result()
                    .expect("strict read v2 request");
            assert_eq!(request.semantic_trigger(), &[0, 0xff, b'/', 0x80]);
            assert_eq!(request.evaluation_anchor(), "origin");
            assert!(request.activation_captures().next().is_none());
            assert_eq!(
                request.decision_progress().reference(),
                &expected_decision_progress.reference
            );
            assert_eq!(
                request.decision_progress().watermark(),
                expected_decision_progress.watermark
            );
            assert_eq!(
                request.decision_closure().reference(),
                &expected_decision_closure.reference
            );
            assert_eq!(
                request.decision_closure().state(),
                expected_decision_closure.state
            );
            assert_eq!(
                request.surrounding_progress().reference(),
                &expected_surrounding_progress.reference
            );
            assert_eq!(
                request.surrounding_progress().watermark(),
                expected_surrounding_progress.watermark
            );
            assert_eq!(
                request.surrounding_closure().reference(),
                &expected_surrounding_closure.reference
            );
            assert_eq!(
                request.surrounding_closure().state(),
                expected_surrounding_closure.state
            );
            assert_eq!(request.execution(), expected_execution);
            assert_eq!(
                request.completeness().reference(),
                &expected_completeness.reference
            );
            assert_eq!(request.completeness().state(), expected_completeness.state);
            assert_eq!(
                request.completeness().facts().collect::<Vec<_>>(),
                expected_completeness.facts.iter().collect::<Vec<_>>()
            );

            let mut changed_axis_inputs = Vec::new();
            let mut changed = first_input.clone();
            changed.decision_progress.watermark = 19;
            changed_axis_inputs.push((
                changed,
                19,
                temporal::Closure::Open,
                23,
                temporal::Closure::Closed,
                temporal::Execution::Failed,
                temporal::Completeness::Incomplete,
            ));
            let mut changed = first_input.clone();
            changed.decision_closure.state = temporal::Closure::Closed;
            changed_axis_inputs.push((
                changed,
                17,
                temporal::Closure::Closed,
                23,
                temporal::Closure::Closed,
                temporal::Execution::Failed,
                temporal::Completeness::Incomplete,
            ));
            let mut changed = first_input.clone();
            changed.surrounding_progress.watermark = 29;
            changed_axis_inputs.push((
                changed,
                17,
                temporal::Closure::Open,
                29,
                temporal::Closure::Closed,
                temporal::Execution::Failed,
                temporal::Completeness::Incomplete,
            ));
            let mut changed = first_input.clone();
            changed.surrounding_closure.state = temporal::Closure::Open;
            changed_axis_inputs.push((
                changed,
                17,
                temporal::Closure::Open,
                23,
                temporal::Closure::Open,
                temporal::Execution::Failed,
                temporal::Completeness::Incomplete,
            ));
            let mut changed = first_input.clone();
            changed.execution = temporal::Execution::Completed;
            changed_axis_inputs.push((
                changed,
                17,
                temporal::Closure::Open,
                23,
                temporal::Closure::Closed,
                temporal::Execution::Completed,
                temporal::Completeness::Incomplete,
            ));
            let mut changed = first_input;
            changed.completeness.state = temporal::Completeness::Complete;
            changed_axis_inputs.push((
                changed,
                17,
                temporal::Closure::Open,
                23,
                temporal::Closure::Closed,
                temporal::Execution::Failed,
                temporal::Completeness::Complete,
            ));

            let mut axis_identities = vec![first.identity().to_owned()];
            for (
                changed,
                decision_watermark,
                decision_closure,
                surrounding_watermark,
                surrounding_closure,
                execution,
                completeness,
            ) in changed_axis_inputs
            {
                let changed_document =
                    temporal_v2::produce(&subject, changed, native_temporal::Limits::default())
                        .into_result()
                        .expect("one changed axis produces a distinct v2 request");
                assert_ne!(changed_document.identity(), first.identity());
                let changed_request = temporal_v2::read(
                    changed_document.bytes(),
                    &subject,
                    native_temporal::Limits::default(),
                )
                .into_result()
                .expect("strict-read one changed v2 request axis");
                assert_eq!(
                    changed_request.decision_progress().watermark(),
                    decision_watermark
                );
                assert_eq!(changed_request.decision_closure().state(), decision_closure);
                assert_eq!(
                    changed_request.surrounding_progress().watermark(),
                    surrounding_watermark
                );
                assert_eq!(
                    changed_request.surrounding_closure().state(),
                    surrounding_closure
                );
                assert_eq!(changed_request.execution(), execution);
                assert_eq!(changed_request.completeness().state(), completeness);
                axis_identities.push(changed_document.identity().to_owned());
            }
            axis_identities.sort();
            axis_identities.dedup();
            assert_eq!(axis_identities.len(), 7);

            let output = temporal_v2::evaluate(
                &request,
                temporal_v2::Relation::Original,
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("evaluate v2 result");
            assert_schema(
                temporal_v2::RESULT_SCHEMA_BYTES,
                temporal_v2::RESULT_SCHEMA_SHA256,
                output.bytes(),
            );
            let result = temporal_v2::read_result(
                output.bytes(),
                &request,
                temporal_v2::Relation::Original,
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("strict read v2 result");
            assert_eq!(result.semantic_trigger(), &[0, 0xff, b'/', 0x80]);
            assert_eq!(
                result.activation(),
                native_temporal::result::ActivationState::Active
            );

            let second = temporal_v2::produce(
                &subject,
                input_v2(leaf, vec![0, 0xfe, b'/', 0x80]),
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("one-byte mutation remains a distinct v2 request");
            assert_ne!(first.identity(), second.identity());
            let other_request =
                temporal_v2::read(second.bytes(), &subject, native_temporal::Limits::default())
                    .into_result()
                    .expect("read changed request");
            assert!(temporal_v2::read_result(
                output.bytes(),
                &other_request,
                temporal_v2::Relation::Original,
                native_temporal::Limits::default(),
            )
            .into_result()
            .is_err());
            assert!(temporal_v2::evaluate(
                &other_request,
                temporal_v2::Relation::Superseding(&result),
                native_temporal::Limits::default(),
            )
            .into_result()
            .is_err());
            let mut correction_input = input_v2(leaf, vec![0, 0xff, b'/', 0x80]);
            correction_input.triggers[0].receipt = "receipt:2".into();
            let correction_document = temporal_v2::produce(
                &subject,
                correction_input,
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("changed delivery creates a new request for the same trigger");
            let correction_request = temporal_v2::read(
                correction_document.bytes(),
                &subject,
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("strict read correction request");
            let correction = temporal_v2::evaluate(
                &correction_request,
                temporal_v2::Relation::Superseding(&result),
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("same-trigger correction is admitted");
            assert!(temporal_v2::read_result(
                correction.bytes(),
                &correction_request,
                temporal_v2::Relation::Superseding(&result),
                native_temporal::Limits::default(),
            )
            .into_result()
            .is_ok());
            let mut malformed: Value = serde_json::from_slice(first.bytes()).expect("v2 JSON");
            malformed["trigger"] = Value::String("00FF2f80".into());
            let malformed = serde_json::to_vec(&malformed).expect("mutated JSON");
            assert!(
                temporal_v2::read(&malformed, &subject, native_temporal::Limits::default(),)
                    .into_result()
                    .is_err()
            );
            assert!(temporal_v2::SemanticTriggerIdentity::new(Vec::new()).is_err());
            assert!(
                request::read(first.bytes(), &subject, native_temporal::Limits::default())
                    .into_result()
                    .is_err()
            );
            let v1 = request::produce(
                &subject,
                input(leaf, true, "correspondence:v1-cross-version"),
                native_temporal::Limits::default(),
            )
            .into_result()
            .expect("produce v1 request");
            assert!(
                temporal_v2::read(v1.bytes(), &subject, native_temporal::Limits::default())
                    .into_result()
                    .is_err()
            );
        },
    );
}
