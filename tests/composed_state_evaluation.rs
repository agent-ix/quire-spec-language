// SPDX-License-Identifier: AGPL-3.0-only
//! Public admitted-artifact state evaluation controls.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::checking::composed::{proofs, TypeLimits};
use quire_spec_language::protocol_artifact::{
    native, wire as w, ExactInteger, Limits as ArtifactLimits, ProtocolNumber,
};
use quire_spec_language::state::{
    self, AssessmentAuthority, AuthorityAdapter, AuthorityEvidence, BinderInput, CanonicalDigest,
    Dimension, EvaluationOutcome, EvaluationRequest, ExhaustionCause, FieldInput, InputSlot,
    Limits, ObjectInput, ObjectKey, ObservationDigest, ObservationIdentity, ObservationKey,
    PopulationInput, Refusal, StateView, StaticAuthority, Value, ValueKind,
    OBSERVATION_CONTRACT_REVISION, PRODUCER_CONTRACT_REVISION,
};
use quire_spec_language::ByteDigest;
use setup::{Inputs, Unit};

fn admitted(test: impl FnOnce(&quire_spec_language::protocol_artifact::AdmittedPackage)) {
    let inputs = Inputs::new(&[
        Unit {
            name: "state-evaluation",
            body: "predicate Constant using S (): Boolean { true }
                predicate Caller using S (): Boolean { Constant() }",
            declarations: &["Constant", "Caller"],
        },
        Unit {
            name: "state-evaluation-protocol",
            body: "protocol Flow using P over (view: M::Node) on origin {
                role Service on M::Node;
                run check Ready using S { true };
                finish Closed as (closed: M::Node) { true };
            }",
            declarations: &["Flow"],
        },
    ]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selections| {
            let admitted = native::admit(proofs, selections, ArtifactLimits::default())
                .into_result()
                .expect("discharged constant predicate");
            let emitted = native::emit(&admitted, ArtifactLimits::default())
                .into_result()
                .expect("authenticated native emission");
            let reread = inputs
                .read(proofs, &emitted)
                .into_result()
                .expect("independently read admitted artifact");
            test(&reread);
        },
    );
}

fn admitted_queries(test: impl FnOnce(&quire_spec_language::protocol_artifact::AdmittedPackage)) {
    let inputs = Inputs::with_query_input(
        &[
            Unit {
                name: "state-queries",
                body: "predicate Queries using S (input: M::QueryInput): Boolean {
                    size<M::Tally>(input.amounts) = 3
                    and contains(input.amounts, 2)
                    and forall(allItem in input.amounts: allItem >= 1)
                    and exists(someItem in input.amounts: someItem = 3)
                    and contains(filter(kept in input.amounts: kept = 2), 2)
                    and size<M::Tally>(map(projected in input.amounts: projected)) = 3
                    and count<M::Tally>(counted in input.amounts: counted = 2) = 2
                    and sum<M::Total>(summand in input.amounts: summand) = 7
                }",
                declarations: &["Queries"],
            },
            Unit {
                name: "state-query-protocol",
                body: "protocol Flow using P over (view: M::Node) on origin {
                    role Service on M::Node;
                    run check Ready using S { true };
                    finish Closed as (closed: M::Node) { true };
                }",
                declarations: &["Flow"],
            },
        ],
        5,
    );
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selections| {
            let admitted = native::admit(proofs, selections, ArtifactLimits::default())
                .into_result()
                .expect("discharged ordered query fixture");
            let emitted = native::emit(&admitted, ArtifactLimits::default())
                .into_result()
                .expect("authenticated query emission");
            let reread = inputs
                .read(proofs, &emitted)
                .into_result()
                .expect("independently read query artifact");
            test(&reread);
        },
    );
}

fn admitted_graph(test: impl FnOnce(&quire_spec_language::protocol_artifact::AdmittedPackage)) {
    let inputs = Inputs::with_graph_input(&[Unit {
        name: "state-graph",
        body: "invariant Graph using G on M::GraphNode at current {
                size<M::Tally>(self.links) >= 0 and reaches(self,self,links)
            }
            protocol Flow using P over (view: M::Node) on origin {
                role Service on M::Node;
                run check Ready using S { true };
                finish Closed as (closed: M::Node) { true };
            }",
        declarations: &["Graph", "Flow"],
    }]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selections| {
            let admitted = native::admit(proofs, selections, ArtifactLimits::default())
                .into_result()
                .expect("discharged finite graph fixture");
            let emitted = native::emit(&admitted, ArtifactLimits::default())
                .into_result()
                .expect("authenticated graph emission");
            let reread = inputs
                .read(proofs, &emitted)
                .into_result()
                .expect("independently read graph artifact");
            test(&reread);
        },
    );
}

fn admitted_text(test: impl FnOnce(&quire_spec_language::protocol_artifact::AdmittedPackage)) {
    let inputs = Inputs::new(&[
        Unit {
            name: "state-text",
            body: "predicate Text using S (input: M::Label): Boolean { input = \"Ω\" }",
            declarations: &["Text"],
        },
        Unit {
            name: "state-text-protocol",
            body: "protocol Flow using P over (view: M::Node) on origin {
                role Service on M::Node;
                run check Ready using S { true };
                finish Closed as (closed: M::Node) { true };
            }",
            declarations: &["Flow"],
        },
    ]);
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selections| {
            let admitted = native::admit(proofs, selections, ArtifactLimits::default())
                .into_result()
                .expect("discharged text fixture");
            let emitted = native::emit(&admitted, ArtifactLimits::default())
                .into_result()
                .expect("authenticated text emission");
            let reread = inputs
                .read(proofs, &emitted)
                .into_result()
                .expect("independently read text artifact");
            test(&reread);
        },
    );
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

fn authority(
    package: &quire_spec_language::protocol_artifact::AdmittedPackage,
    owner: u32,
    requirement_index: u32,
) -> AuthorityEvidence {
    let requirement =
        &package.package().declarations[owner as usize].bindings[requirement_index as usize];
    let population = requirement
        .model
        .0
        .as_ref()
        .expect("population model export");
    let producer = package.package().dependencies
        [package.package().models[population.model as usize].artifact as usize]
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
    let compiled = package.artifact().clone();
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

fn graph_view(
    package: &quire_spec_language::protocol_artifact::AdmittedPackage,
) -> (u32, w::Handle, StateView) {
    let owner = package
        .package()
        .declarations
        .iter()
        .position(|declaration| declaration.name == "Graph")
        .expect("graph declaration") as u32;
    let declaration = &package.package().declarations[owner as usize];
    let (requirement_index, requirement) = declaration
        .bindings
        .iter()
        .enumerate()
        .find(|(_, binding)| binding.kind == w::BindingKind::Population)
        .expect("population requirement");
    let closure_index = declaration
        .bindings
        .iter()
        .position(|binding| {
            binding.kind == w::BindingKind::Closure
                && binding.requires == [requirement_index as u32]
        })
        .expect("closure requirement");
    let (binder_index, binder) = declaration
        .binders
        .iter()
        .enumerate()
        .find(|(_, binder)| binder.name == "self")
        .expect("self binder");
    let (root_index, reaches) = declaration
        .values
        .iter()
        .enumerate()
        .find(|(_, value)| matches!(value.operation, w::ValueOperation::Reaches { .. }))
        .expect("reachability operation");
    let w::ValueOperation::Reaches { edge, .. } = &reaches.operation else {
        unreachable!("selected reachability")
    };
    let field_type = declaration
        .values
        .iter()
        .find(|value| matches!(&value.operation, w::ValueOperation::Field { field, .. } if field == edge))
        .expect("graph edge field")
        .value_type;
    let w::Type::Sequence { element, .. } = package
        .package()
        .types
        .get(field_type as usize)
        .expect("edge sequence type")
    else {
        unreachable!("edge sequence")
    };
    let object_type = match &package.package().types[binder.value_type as usize] {
        w::Type::Object { export } => export.clone(),
        _ => unreachable!("self object type"),
    };
    let universe = requirement.model.0.clone().expect("population export");
    let occurrence = |record: &str| ObservationKey {
        anchor: requirement.anchor.clone(),
        snapshot: ObservationIdentity("snapshot:selected".into()),
        window: None,
        record: ObservationIdentity(record.into()),
    };
    let key = |record: &str| ObjectKey {
        observation: occurrence(record),
        model: object_type.model,
        universe: universe.clone(),
        object_type: object_type.clone(),
        identifier: "logical-a".into(),
    };
    let first = key("record:first");
    let second = key("record:second");
    let links = |target: ObjectKey| FieldInput {
        field: edge.clone(),
        value: InputSlot::Available(Value::new(
            field_type,
            ValueKind::Sequence(vec![InputSlot::Available(Value::new(
                *element,
                ValueKind::Reference(target),
            ))]),
        )),
    };
    let evidence = authority(package, owner, requirement_index as u32);
    let objects = vec![
        ObjectInput {
            key: first.clone(),
            fields: vec![links(first.clone())],
        },
        ObjectInput {
            key: second.clone(),
            fields: vec![links(second)],
        },
    ];
    (
        owner,
        w::Handle {
            declaration: owner,
            index: root_index as u32,
        },
        StateView {
            binders: vec![BinderInput {
                binder: w::Handle {
                    declaration: owner,
                    index: binder_index as u32,
                },
                requirement: Some(requirement_index as u32),
                authority: Some(evidence.clone()),
                value: InputSlot::Available(Value::new(
                    binder.value_type,
                    ValueKind::Object(first),
                )),
            }],
            populations: vec![PopulationInput {
                requirement: w::Handle {
                    declaration: owner,
                    index: requirement_index as u32,
                },
                closure_requirement: w::Handle {
                    declaration: owner,
                    index: closure_index as u32,
                },
                authority: evidence,
                membership: Ok(()),
                closure: Ok(()),
                objects,
            }],
        },
    )
}

fn query_view(
    package: &quire_spec_language::protocol_artifact::AdmittedPackage,
) -> (u32, StateView) {
    let owner = package
        .package()
        .declarations
        .iter()
        .position(|declaration| declaration.name == "Queries")
        .expect("query declaration") as u32;
    let declaration = &package.package().declarations[owner as usize];
    let (binder_index, binder) = declaration
        .binders
        .iter()
        .enumerate()
        .find(|(_, binder)| binder.name == "input")
        .expect("query input binder");
    let field_node = declaration
        .values
        .iter()
        .find(|value| matches!(value.operation, w::ValueOperation::Field { .. }))
        .expect("amounts field");
    let w::ValueOperation::Field { field, .. } = &field_node.operation else {
        unreachable!("selected field operation")
    };
    let w::Type::Sequence { element, .. } = package
        .package()
        .types
        .get(field_node.value_type as usize)
        .expect("sequence type")
    else {
        unreachable!("amounts sequence type")
    };
    let amount = |number| {
        InputSlot::Available(Value::new(
            *element,
            ValueKind::Number(ProtocolNumber::Integer(ExactInteger::new(number))),
        ))
    };
    let amounts = Value::new(
        field_node.value_type,
        ValueKind::Sequence(vec![amount(2), amount(2), amount(3)]),
    );
    let record = Value::new(
        binder.value_type,
        ValueKind::Record(vec![FieldInput {
            field: field.clone(),
            value: InputSlot::Available(amounts),
        }]),
    );
    (
        owner,
        StateView {
            binders: vec![BinderInput {
                binder: w::Handle {
                    declaration: owner,
                    index: binder_index as u32,
                },
                requirement: None,
                authority: None,
                value: InputSlot::Available(record),
            }],
            populations: Vec::new(),
        },
    )
}

fn operation(
    package: &quire_spec_language::protocol_artifact::AdmittedPackage,
    owner: u32,
    predicate: impl Fn(&w::ValueOperation) -> bool,
) -> w::Handle {
    let index = package.package().declarations[owner as usize]
        .values
        .iter()
        .position(|value| predicate(&value.operation))
        .expect("selected authored operation");
    w::Handle {
        declaration: owner,
        index: index as u32,
    }
}

fn completed<'a>(
    package: &'a quire_spec_language::protocol_artifact::AdmittedPackage,
    owner: u32,
    handle: w::Handle,
    view: &'a StateView,
) -> &'static str {
    let report = state::evaluate(
        package,
        EvaluationRequest {
            declaration: owner,
            value: handle,
        },
        view,
        Limits::default(),
    );
    match report.outcome() {
        EvaluationOutcome::Completed(value) => match value.kind() {
            ValueKind::Boolean(true) => "true",
            ValueKind::Boolean(false) => "false",
            ValueKind::Number(ProtocolNumber::Integer(value)) if value.value() == 2 => "2",
            ValueKind::Number(ProtocolNumber::Integer(value)) if value.value() == 3 => "3",
            ValueKind::Number(ProtocolNumber::Integer(value)) if value.value() == 7 => "7",
            ValueKind::Sequence(values) if sequence_is(values, &[2, 2]) => "[2,2]",
            ValueKind::Sequence(values) if sequence_is(values, &[2, 2, 3]) => "[2,2,3]",
            _ => "unexpected-completed",
        },
        _ => "non-complete",
    }
}

fn sequence_is(values: &[InputSlot], expected: &[i64]) -> bool {
    values.len() == expected.len()
        && values.iter().zip(expected).all(|(slot, expected)| {
            matches!(slot,
                InputSlot::Available(value)
                    if matches!(value.kind(),
                        ValueKind::Number(ProtocolNumber::Integer(actual))
                            if actual.value() == *expected))
        })
}

fn request(package: &quire_spec_language::protocol_artifact::AdmittedPackage) -> EvaluationRequest {
    let (declaration_index, declaration) = package
        .package()
        .declarations
        .iter()
        .enumerate()
        .find(|(_, declaration)| declaration.name == "Constant")
        .expect("fixture predicate");
    let root = match &declaration.body {
        quire_spec_language::protocol_artifact::wire::Body::Predicate { root, .. } => root.clone(),
        _ => panic!("fixture predicate"),
    };
    EvaluationRequest {
        declaration: declaration_index as u32,
        value: root,
    }
}

/// Tracing: TC-126, TC-136.
#[test]
#[trace(
    "TC-126",
    "TC-136",
    "FR-046-AC-1",
    "FR-046-AC-8",
    "FR-049-AC-1",
    "FR-049-AC-6"
)]
fn tc_126_136_admitted_value_selection_returns_one_typed_outcome() {
    admitted(|package| {
        let report = state::evaluate(
            package,
            request(package),
            &StateView::default(),
            Limits::default(),
        );
        assert_eq!(report.accounting_version(), "quire.state.evaluation-work/1");
        assert!(matches!(
            report.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(true))
        ));

        let mut crossed = request(package);
        crossed.value.declaration = 1;
        assert!(matches!(
            state::evaluate(package, crossed, &StateView::default(), Limits::default()).outcome(),
            EvaluationOutcome::Refused(_)
        ));
    });
}

/// Tracing: TC-137.
#[test]
#[trace("TC-137", "FR-049-AC-6", "FR-049-AC-7", "FR-049-AC-8", "NFR-009")]
fn tc_137_expression_and_output_limits_are_charge_before_work_and_fresh() {
    admitted(|package| {
        let complete = state::evaluate(
            package,
            request(package),
            &StateView::default(),
            Limits::default(),
        );
        assert_eq!(complete.usage().expression_work, 1);
        assert_eq!(complete.usage().active_expression_depth, 1);
        assert_eq!(complete.usage().retained_output, 1);

        let stopped = state::evaluate(
            package,
            request(package),
            &StateView::default(),
            Limits {
                expression_work: 0,
                ..Limits::default()
            },
        );
        assert!(matches!(
            stopped.outcome(),
            EvaluationOutcome::Exhausted(exhaustion)
                if exhaustion.dimension == Dimension::ExpressionWork
                    && exhaustion.cause == ExhaustionCause::Limit
                    && exhaustion.used == 0
        ));

        let output_stopped = state::evaluate(
            package,
            request(package),
            &StateView::default(),
            Limits {
                retained_output: 0,
                ..Limits::default()
            },
        );
        assert!(matches!(
            output_stopped.outcome(),
            EvaluationOutcome::Exhausted(exhaustion)
                if exhaustion.dimension == Dimension::RetainedOutput
                    && exhaustion.used == 0
        ));

        let retry = state::evaluate(
            package,
            request(package),
            &StateView::default(),
            Limits::default(),
        );
        assert_eq!(retry.outcome(), complete.outcome());
        assert_eq!(retry.usage(), complete.usage());
    });
}

/// Tracing: TC-137.
#[test]
#[trace("TC-137", "NFR-009", "FR-046-AC-7", "FR-049-AC-7")]
fn tc_137_query_input_sequence_and_retention_dimensions_have_exact_boundaries() {
    admitted_queries(|package| {
        let (owner, view) = query_view(package);
        let value = operation(package, owner, |op| {
            matches!(
                op,
                w::ValueOperation::Query {
                    operator: w::Query::Map,
                    ..
                }
            )
        });
        let request = EvaluationRequest {
            declaration: owner,
            value,
        };
        let exact = Limits {
            input_value_nodes: 5,
            input_aggregate_entries: 5,
            input_structural_depth: 3,
            expression_work: 6,
            active_expression_depth: 3,
            sequence_work: 3,
            retained_output: 4,
            ..Limits::default()
        };
        let complete = state::evaluate(package, request.clone(), &view, exact);
        assert!(matches!(
            complete.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Sequence(values)
                    if sequence_is(values, &[2, 2, 3]))
        ));
        assert_eq!(complete.usage().input_value_nodes, 5);
        assert_eq!(complete.usage().input_aggregate_entries, 5);
        assert_eq!(complete.usage().input_structural_depth, 3);
        assert_eq!(complete.usage().expression_work, 6);
        assert_eq!(complete.usage().active_expression_depth, 3);
        assert_eq!(complete.usage().sequence_work, 3);
        assert_eq!(complete.usage().retained_output, 4);

        for (limits, dimension) in [
            (
                Limits {
                    input_value_nodes: 4,
                    ..exact
                },
                Dimension::InputValueNodes,
            ),
            (
                Limits {
                    input_aggregate_entries: 4,
                    ..exact
                },
                Dimension::InputAggregateEntries,
            ),
            (
                Limits {
                    input_structural_depth: 2,
                    ..exact
                },
                Dimension::InputStructuralDepth,
            ),
            (
                Limits {
                    expression_work: 5,
                    ..exact
                },
                Dimension::ExpressionWork,
            ),
            (
                Limits {
                    active_expression_depth: 2,
                    ..exact
                },
                Dimension::ActiveExpressionDepth,
            ),
            (
                Limits {
                    sequence_work: 2,
                    ..exact
                },
                Dimension::SequenceWork,
            ),
            (
                Limits {
                    retained_output: 3,
                    ..exact
                },
                Dimension::RetainedOutput,
            ),
        ] {
            assert!(matches!(
                state::evaluate(package, request.clone(), &view, limits).outcome(),
                EvaluationOutcome::Exhausted(exhaustion)
                    if exhaustion.dimension == dimension
                        && exhaustion.cause == ExhaustionCause::Limit
            ));
        }
    });
}

/// Tracing: TC-128, TC-137.
#[test]
#[trace("TC-128", "TC-137", "NFR-009", "FR-046-AC-7")]
fn tc_128_137_predicate_call_depth_has_an_independent_exact_boundary() {
    admitted(|package| {
        let owner = package
            .package()
            .declarations
            .iter()
            .position(|declaration| declaration.name == "Caller")
            .expect("caller declaration") as u32;
        let call = operation(package, owner, |op| {
            matches!(op, w::ValueOperation::Call { .. })
        });
        let request = EvaluationRequest {
            declaration: owner,
            value: call,
        };
        let exact = state::evaluate(
            package,
            request.clone(),
            &StateView::default(),
            Limits {
                predicate_call_depth: 1,
                ..Limits::default()
            },
        );
        assert!(matches!(
            exact.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(true))
        ));
        assert_eq!(exact.usage().predicate_call_depth, 1);
        assert!(matches!(
            state::evaluate(
                package,
                request,
                &StateView::default(),
                Limits {
                    predicate_call_depth: 0,
                    ..Limits::default()
                }
            )
            .outcome(),
            EvaluationOutcome::Exhausted(exhaustion)
                if exhaustion.dimension == Dimension::PredicateCallDepth
                    && exhaustion.cause == ExhaustionCause::Limit
        ));
    });
}

/// Tracing: TC-136, TC-137.
#[test]
#[trace("TC-136", "TC-137", "NFR-009", "FR-049-AC-3", "FR-049-AC-7")]
fn tc_136_137_utf8_input_content_has_a_byte_exact_boundary() {
    admitted_text(|package| {
        let owner = package
            .package()
            .declarations
            .iter()
            .position(|declaration| declaration.name == "Text")
            .expect("text declaration") as u32;
        let declaration = &package.package().declarations[owner as usize];
        let (binder_index, binder) = declaration
            .binders
            .iter()
            .enumerate()
            .find(|(_, binder)| binder.name == "input")
            .expect("text binder");
        let root = match &declaration.body {
            w::Body::Predicate { root, .. } => root.clone(),
            _ => unreachable!("text predicate"),
        };
        let view = StateView {
            binders: vec![BinderInput {
                binder: w::Handle {
                    declaration: owner,
                    index: binder_index as u32,
                },
                requirement: None,
                authority: None,
                value: InputSlot::Available(Value::new(
                    binder.value_type,
                    ValueKind::Text("Ω".into()),
                )),
            }],
            populations: Vec::new(),
        };
        let request = EvaluationRequest {
            declaration: owner,
            value: root,
        };
        let exact = state::evaluate(
            package,
            request.clone(),
            &view,
            Limits {
                input_text_bytes: 2,
                ..Limits::default()
            },
        );
        assert!(matches!(
            exact.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(true))
        ));
        assert_eq!(exact.usage().input_text_bytes, 2);
        assert!(matches!(
            state::evaluate(
                package,
                request,
                &view,
                Limits {
                    input_text_bytes: 1,
                    ..Limits::default()
                }
            )
            .outcome(),
            EvaluationOutcome::Exhausted(exhaustion)
                if exhaustion.dimension == Dimension::InputTextBytes
                    && exhaustion.cause == ExhaustionCause::Limit
        ));
    });
}

/// Tracing: TC-127.
#[test]
#[trace("TC-127", "FR-046-AC-3", "FR-046-AC-4", "FR-046-AC-8")]
fn tc_127_all_eight_queries_preserve_order_duplicates_and_independent_results() {
    admitted_queries(|package| {
        let (owner, view) = query_view(package);
        let selected = [
            (
                operation(
                    package,
                    owner,
                    |op| matches!(op, w::ValueOperation::Size { collection, .. } if matches!(package.package().declarations[owner as usize].values[collection.index as usize].operation, w::ValueOperation::Field { .. })),
                ),
                "3",
            ),
            (
                operation(
                    package,
                    owner,
                    |op| matches!(op, w::ValueOperation::Contains { collection, .. } if matches!(package.package().declarations[owner as usize].values[collection.index as usize].operation, w::ValueOperation::Field { .. })),
                ),
                "true",
            ),
            (
                operation(package, owner, |op| {
                    matches!(
                        op,
                        w::ValueOperation::Query {
                            operator: w::Query::ForAll,
                            ..
                        }
                    )
                }),
                "true",
            ),
            (
                operation(package, owner, |op| {
                    matches!(
                        op,
                        w::ValueOperation::Query {
                            operator: w::Query::Exists,
                            ..
                        }
                    )
                }),
                "true",
            ),
            (
                operation(package, owner, |op| {
                    matches!(
                        op,
                        w::ValueOperation::Query {
                            operator: w::Query::Filter,
                            ..
                        }
                    )
                }),
                "[2,2]",
            ),
            (
                operation(package, owner, |op| {
                    matches!(
                        op,
                        w::ValueOperation::Query {
                            operator: w::Query::Map,
                            ..
                        }
                    )
                }),
                "[2,2,3]",
            ),
            (
                operation(package, owner, |op| {
                    matches!(
                        op,
                        w::ValueOperation::Query {
                            operator: w::Query::Count,
                            ..
                        }
                    )
                }),
                "2",
            ),
            (
                operation(package, owner, |op| {
                    matches!(
                        op,
                        w::ValueOperation::Query {
                            operator: w::Query::Sum,
                            ..
                        }
                    )
                }),
                "7",
            ),
        ];
        for (case, (handle, expected)) in selected.into_iter().enumerate() {
            assert_eq!(
                completed(package, owner, handle, &view),
                expected,
                "query case {case}"
            );
        }
    });
}

/// Tracing: TC-129, TC-130, TC-131, TC-136.
#[test]
#[trace(
    "TC-129",
    "TC-130",
    "TC-131",
    "TC-136",
    "FR-047-AC-1",
    "FR-047-AC-2",
    "FR-047-AC-4",
    "FR-047-AC-5",
    "FR-049-AC-5"
)]
fn tc_129_130_131_full_occurrences_authority_and_graph_limits_are_exact() {
    admitted_graph(|package| {
        let (owner, root, view) = graph_view(package);
        let request = EvaluationRequest {
            declaration: owner,
            value: root.clone(),
        };
        let complete = state::evaluate(
            package,
            request.clone(),
            &view,
            Limits {
                graph_expansion: 1,
                graph_edges: 1,
                active_graph_depth: 1,
                value_comparison: 1,
                ..Limits::default()
            },
        );
        assert!(matches!(
            complete.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(true))
        ));
        assert_eq!(complete.usage().graph_expansion, 1);
        assert_eq!(complete.usage().graph_edges, 1);
        assert_eq!(complete.usage().active_graph_depth, 1);
        assert_eq!(complete.usage().value_comparison, 1);
        assert_eq!(view.populations[0].objects.len(), 2);
        assert_ne!(
            view.populations[0].objects[0].key.observation.record,
            view.populations[0].objects[1].key.observation.record
        );

        let mut duplicate = view.clone();
        let repeated = duplicate.populations[0].objects[0].clone();
        duplicate.populations[0].objects.push(repeated);
        assert!(matches!(
            state::evaluate(package, request.clone(), &duplicate, Limits::default()).outcome(),
            EvaluationOutcome::Refused(Refusal::DuplicateObject(_))
        ));

        let mut crossed_compiled = view.clone();
        crossed_compiled.binders[0]
            .authority
            .as_mut()
            .expect("binder authority")
            .compiled
            .digest = ByteDigest::of(b"different compiled artifact");
        assert!(matches!(
            state::evaluate(
                package,
                request.clone(),
                &crossed_compiled,
                Limits::default()
            )
            .outcome(),
            EvaluationOutcome::Refused(Refusal::Authority(_))
        ));

        let mut raw_digest_in_canonical_domain = view.clone();
        raw_digest_in_canonical_domain.binders[0]
            .authority
            .as_mut()
            .expect("binder authority")
            .static_selection
            .model_digest
            .value = package.digest().to_string();
        assert!(matches!(
            state::evaluate(
                package,
                request.clone(),
                &raw_digest_in_canonical_domain,
                Limits::default()
            )
            .outcome(),
            EvaluationOutcome::Refused(Refusal::UngroundedAuthorityMapping(_))
        ));

        let mut ungrounded = view.clone();
        ungrounded.binders[0]
            .authority
            .as_mut()
            .expect("binder authority")
            .adapter = None;
        assert!(matches!(
            state::evaluate(package, request.clone(), &ungrounded, Limits::default()).outcome(),
            EvaluationOutcome::Refused(Refusal::UngroundedAuthorityMapping(_))
        ));

        for (limits, dimension) in [
            (
                Limits {
                    graph_expansion: 0,
                    ..Limits::default()
                },
                Dimension::GraphExpansion,
            ),
            (
                Limits {
                    graph_edges: 0,
                    ..Limits::default()
                },
                Dimension::GraphEdges,
            ),
            (
                Limits {
                    active_graph_depth: 0,
                    ..Limits::default()
                },
                Dimension::ActiveGraphDepth,
            ),
            (
                Limits {
                    value_comparison: 0,
                    ..Limits::default()
                },
                Dimension::ValueComparison,
            ),
        ] {
            assert!(matches!(
                state::evaluate(package, request.clone(), &view, limits).outcome(),
                EvaluationOutcome::Exhausted(exhaustion)
                    if exhaustion.dimension == dimension
                        && exhaustion.cause == ExhaustionCause::Limit
            ));
        }
    });
}
