// SPDX-License-Identifier: AGPL-3.0-or-later
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
    ContextualSlot, ContextualValue, ContextualValueKind, Dimension, EvaluationOutcome,
    EvaluationRequest, ExhaustionCause, FieldInput, FieldValue, InputSlot, Limits, MissingInput,
    ObjectInput, ObjectKey, ObservationDigest, ObservationIdentity, ObservationKey,
    PopulationInput, Refusal, StateView, StaticAuthority, Value, ValueKind, ValuePathSegment,
    OBSERVATION_CONTRACT_REVISION,
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
                reaches(self,self,links)
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

fn admitted_optional_graph(
    test: impl FnOnce(&quire_spec_language::protocol_artifact::AdmittedPackage),
) {
    let inputs = Inputs::with_optional_graph_input(&[Unit {
        name: "state-optional-graph",
        body: "invariant Graph using G on M::GraphNode at current {
                reaches(self,self,links)
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
                .expect("discharged optional graph fixture");
            let emitted = native::emit(&admitted, ArtifactLimits::default())
                .into_result()
                .expect("authenticated optional graph emission");
            let reread = inputs
                .read(proofs, &emitted)
                .into_result()
                .expect("independently read optional graph artifact");
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

fn admitted_postcondition(
    test: impl FnOnce(&quire_spec_language::protocol_artifact::AdmittedPackage),
) {
    let mut inputs = Inputs::new(&[
        Unit {
            name: "state-observations",
            body: "pre Before using S on M::Node::step { self.n >= 0 }
                post Changed using S on M::Node::step {
                    pre(self.n) = 1 and self.n = 2 and result
                }",
            declarations: &["Before", "Changed"],
        },
        Unit {
            name: "state-observation-protocol",
            body: "protocol Flow using P over (view: M::Node) on origin {
                role Service on M::Node;
                run attempt Tried by Service on M::Node::step contracts [Before,Changed]
                    as (attempted: M::Plain) { attempted.ready };
                finish Closed as (closed: M::Node) { true };
            }",
            declarations: &["Flow"],
        },
    ]);
    inputs.step_contracts("Before", "Changed");
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selections| {
            let admitted = native::admit(proofs, selections, ArtifactLimits::default())
                .into_result()
                .expect("discharged pre/post fixture");
            let emitted = native::emit(&admitted, ArtifactLimits::default())
                .into_result()
                .expect("authenticated pre/post emission");
            let reread = inputs
                .read(proofs, &emitted)
                .into_result()
                .expect("independently read pre/post artifact");
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
        observation_contract_revision: OBSERVATION_CONTRACT_REVISION.into(),
        static_selection: static_selection.clone(),
        assessment_selection: assessment_selection.clone(),
    };
    AuthorityEvidence {
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
    let binder_requirement_index = declaration.anchors[binder.anchor.index as usize]
        .binding
        .0
        .expect("self anchor requirement");
    let (root_index, reaches) = declaration
        .values
        .iter()
        .enumerate()
        .find(|(_, value)| matches!(value.operation, w::ValueOperation::Reaches { .. }))
        .expect("reachability operation");
    let w::ValueOperation::Reaches { edge, .. } = &reaches.operation else {
        unreachable!("selected reachability")
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
        value: FieldValue::Contextual(ContextualSlot::Available(ContextualValue::new(
            ContextualValueKind::Sequence(vec![ContextualSlot::Available(ContextualValue::new(
                ContextualValueKind::Reference(target),
            ))]),
        ))),
    };
    let evidence = authority(package, owner, requirement_index as u32);
    let binder_evidence = authority(package, owner, binder_requirement_index);
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
                requirement: Some(binder_requirement_index),
                authority: Some(binder_evidence),
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

fn graph_link(targets: impl IntoIterator<Item = ObjectKey>, field: w::ExportRef) -> FieldInput {
    FieldInput {
        field,
        value: FieldValue::Contextual(ContextualSlot::Available(ContextualValue::new(
            ContextualValueKind::Sequence(
                targets
                    .into_iter()
                    .map(|target| {
                        ContextualSlot::Available(ContextualValue::new(
                            ContextualValueKind::Reference(target),
                        ))
                    })
                    .collect(),
            ),
        ))),
    }
}

fn replace_graph_links(object: &mut ObjectInput, targets: impl IntoIterator<Item = ObjectKey>) {
    let field = object.fields[0].field.clone();
    object.fields = vec![graph_link(targets, field)];
}

fn postcondition_view(
    package: &quire_spec_language::protocol_artifact::AdmittedPackage,
) -> (u32, w::Handle, StateView) {
    let owner = package
        .package()
        .declarations
        .iter()
        .position(|declaration| declaration.name == "Changed")
        .expect("postcondition declaration") as u32;
    let declaration = &package.package().declarations[owner as usize];
    let root = match &declaration.body {
        w::Body::State { root, .. } => root.clone(),
        _ => unreachable!("Changed is a state clause"),
    };
    let mut binders = Vec::new();
    let mut populations = Vec::new();
    for (binder_index, binder) in declaration.binders.iter().enumerate() {
        if !matches!(
            binder.kind,
            w::BinderKind::SelfValue | w::BinderKind::Result
        ) {
            continue;
        }
        let binder_handle = w::Handle {
            declaration: owner,
            index: binder_index as u32,
        };
        let requirement_index = declaration.anchors[binder.anchor.index as usize]
            .binding
            .0
            .expect("anchored state binder requirement");
        let evidence = authority(package, owner, requirement_index);
        if binder.kind == w::BinderKind::Result {
            binders.push(BinderInput {
                binder: binder_handle,
                requirement: Some(requirement_index),
                authority: Some(evidence),
                value: InputSlot::Available(Value::new(
                    binder.value_type,
                    ValueKind::Boolean(true),
                )),
            });
            continue;
        }

        let population_index = declaration
            .bindings
            .iter()
            .position(|requirement| {
                requirement.kind == w::BindingKind::Population
                    && requirement.anchor == binder.anchor
            })
            .expect("anchor-local population requirement") as u32;
        let population = &declaration.bindings[population_index as usize];
        let closure_index = declaration
            .bindings
            .iter()
            .position(|requirement| {
                requirement.kind == w::BindingKind::Closure
                    && requirement.requires == [population_index]
            })
            .expect("anchor-local closure requirement") as u32;
        let object_type = match &package.package().types[binder.value_type as usize] {
            w::Type::Object { export } => export.clone(),
            _ => unreachable!("self has object type"),
        };
        let anchor_kind = declaration.anchors[binder.anchor.index as usize].kind;
        let number = match anchor_kind {
            w::AnchorKind::InvocationPre => 1,
            w::AnchorKind::InvocationPost => 2,
            _ => unreachable!("postcondition self anchor"),
        };
        let key = ObjectKey {
            observation: ObservationKey {
                anchor: binder.anchor.clone(),
                snapshot: ObservationIdentity("snapshot:selected".into()),
                window: None,
                record: ObservationIdentity(format!("record:{}", anchor_kind.as_str())),
            },
            model: object_type.model,
            universe: population.model.0.clone().expect("population export"),
            object_type: object_type.clone(),
            identifier: "logical-self".into(),
        };
        let fields = package.package().models[object_type.model as usize]
            .exports
            .iter()
            .enumerate()
            .filter(|(_, export)| {
                export.kind == w::ExportKind::Field
                    && export.path.len() == 2
                    && export.path[0] == "Node"
            })
            .map(|(field_index, export)| {
                let field = w::ExportRef {
                    model: object_type.model,
                    export: field_index as u32,
                };
                let value = if export.path[1] == "n" {
                    let value_type = declaration
                        .values
                        .iter()
                        .find_map(|value| match &value.operation {
                            w::ValueOperation::Field {
                                field: selected, ..
                            } if selected == &field => Some(value.value_type),
                            _ => None,
                        })
                        .expect("selected n field value type");
                    return FieldInput {
                        field,
                        value: FieldValue::Compiled(InputSlot::Available(Value::new(
                            value_type,
                            ValueKind::Number(ProtocolNumber::Integer(ExactInteger::new(number))),
                        ))),
                    };
                } else {
                    ContextualSlot::Unavailable(state::MissingInput::Field {
                        object: key.clone(),
                        field: field.clone(),
                    })
                };
                FieldInput {
                    field,
                    value: FieldValue::Contextual(value),
                }
            })
            .collect();
        binders.push(BinderInput {
            binder: binder_handle,
            requirement: Some(requirement_index),
            authority: Some(evidence),
            value: InputSlot::Available(Value::new(
                binder.value_type,
                ValueKind::Object(key.clone()),
            )),
        });
        populations.push(PopulationInput {
            requirement: w::Handle {
                declaration: owner,
                index: population_index,
            },
            closure_requirement: w::Handle {
                declaration: owner,
                index: closure_index,
            },
            authority: authority(package, owner, population_index),
            membership: Ok(()),
            closure: Ok(()),
            objects: vec![ObjectInput { key, fields }],
        });
    }
    (
        owner,
        root,
        StateView {
            binders,
            populations,
        },
    )
}

#[derive(Clone, Copy)]
enum AmountInput {
    Available(i64),
    Missing { claimed_index: usize },
}

fn query_view(
    package: &quire_spec_language::protocol_artifact::AdmittedPackage,
) -> (u32, StateView) {
    query_view_with_amounts(
        package,
        &[
            AmountInput::Available(2),
            AmountInput::Available(2),
            AmountInput::Available(3),
        ],
    )
}

fn query_view_with_amounts(
    package: &quire_spec_language::protocol_artifact::AdmittedPackage,
    inputs: &[AmountInput],
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
    let binder_handle = w::Handle {
        declaration: owner,
        index: binder_index as u32,
    };
    let amount = |input| match input {
        AmountInput::Available(number) => InputSlot::Available(Value::new(
            *element,
            ValueKind::Number(ProtocolNumber::Integer(ExactInteger::new(number))),
        )),
        AmountInput::Missing { claimed_index } => {
            InputSlot::Unavailable(state::MissingInput::Member {
                requirement: binder_handle.clone(),
                path: vec![
                    ValuePathSegment::Field {
                        object: None,
                        field: field.clone(),
                    },
                    ValuePathSegment::Member(claimed_index),
                ],
            })
        }
    };
    let amounts = Value::new(
        field_node.value_type,
        ValueKind::Sequence(inputs.iter().copied().map(amount).collect()),
    );
    let record = Value::new(
        binder.value_type,
        ValueKind::Record(vec![FieldInput {
            field: field.clone(),
            value: FieldValue::Compiled(InputSlot::Available(amounts)),
        }]),
    );
    (
        owner,
        StateView {
            binders: vec![BinderInput {
                binder: binder_handle,
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
            ValueKind::Number(ProtocolNumber::Integer(value)) if value.value() == 0 => "0",
            ValueKind::Sequence(values) if values.is_empty() => "[]",
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

fn first_query_missing(view: &StateView) -> MissingInput {
    let InputSlot::Available(record) = &view.binders[0].value else {
        unreachable!("query input is available")
    };
    let ValueKind::Record(fields) = record.kind() else {
        unreachable!("query input is a record")
    };
    let FieldValue::Compiled(InputSlot::Available(amounts)) = &fields[0].value else {
        unreachable!("amounts is available")
    };
    let ValueKind::Sequence(values) = amounts.kind() else {
        unreachable!("amounts is a sequence")
    };
    let InputSlot::Unavailable(missing) = &values[0] else {
        unreachable!("first member is unavailable")
    };
    missing.clone()
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
#[trace(
    "TC-126",
    "TC-136",
    "FR-046-AC-1",
    "FR-046-AC-8",
    "FR-049-AC-1",
    "FR-049-AC-6"
)]
#[test]
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

/// Tracing: TC-136. Also FR-049-AC-2's proof that `StaticAuthority` needs no
/// interface version (#139: the field was vestigial, pinned to a producer
/// interface FR-150-AC-7 refuses outright, and was deleted rather than
/// re-pinned) -- `postcondition_view`'s authority below never sets one, and
/// evaluation still completes.
#[trace("TC-136", "FR-049-AC-2", "FR-049-AC-4", "FR-049-AC-5")]
#[test]
fn tc_136_pre_reads_the_exact_invocation_pre_observation_and_binding() {
    admitted_postcondition(|package| {
        let (owner, root, view) = postcondition_view(package);
        let request = EvaluationRequest {
            declaration: owner,
            value: root,
        };
        let report = state::evaluate(package, request.clone(), &view, Limits::default());
        assert!(
            matches!(
                report.outcome(),
                EvaluationOutcome::Completed(value)
                    if matches!(value.kind(), ValueKind::Boolean(true))
            ),
            "{:?}",
            report.outcome()
        );

        let declaration = &package.package().declarations[owner as usize];
        let pre = view
            .binders
            .iter()
            .position(|input| {
                declaration.anchors[declaration.binders[input.binder.index as usize]
                    .anchor
                    .index as usize]
                    .kind
                    == w::AnchorKind::InvocationPre
            })
            .expect("pre self input");
        let post = view
            .binders
            .iter()
            .position(|input| {
                let binder = &declaration.binders[input.binder.index as usize];
                binder.kind == w::BinderKind::SelfValue
                    && declaration.anchors[binder.anchor.index as usize].kind
                        == w::AnchorKind::InvocationPost
            })
            .expect("post self input");

        let mut crossed_observation = view.clone();
        crossed_observation.binders[pre].value = view.binders[post].value.clone();
        assert!(matches!(
            state::evaluate(
                package,
                request.clone(),
                &crossed_observation,
                Limits::default()
            )
            .outcome(),
            EvaluationOutcome::Refused(Refusal::Authority(handle))
                if handle == &view.binders[pre].binder
        ));

        let mut laundered_requirement = view.clone();
        laundered_requirement.binders[pre].requirement = view.binders[post].requirement;
        laundered_requirement.binders[pre].authority = view.binders[post].authority.clone();
        assert!(matches!(
            state::evaluate(
                package,
                request,
                &laundered_requirement,
                Limits::default()
            )
            .outcome(),
            EvaluationOutcome::Refused(Refusal::Authority(handle))
                if handle == &view.binders[pre].binder
        ));
    });
}

/// Tracing: TC-137.
#[trace(
    "TC-137",
    "FR-049-AC-6",
    "FR-049-AC-7",
    "FR-049-AC-8",
    "NFR-009",
    "NFR-009-AC-4"
)]
#[test]
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

/// Tracing: TC-131, TC-137.
#[trace("TC-131", "TC-137", "FR-047-AC-7", "FR-049-AC-7", "NFR-009-AC-1")]
#[test]
fn tc_137_all_dimensions_clamp_and_unused_zero_limits_remain_effective() {
    admitted(|package| {
        let above_hard = Limits {
            input_value_nodes: usize::MAX,
            input_aggregate_entries: usize::MAX,
            input_text_bytes: usize::MAX,
            input_structural_depth: usize::MAX,
            expression_work: usize::MAX,
            active_expression_depth: usize::MAX,
            predicate_call_depth: usize::MAX,
            sequence_work: usize::MAX,
            retained_output: usize::MAX,
            graph_expansion: usize::MAX,
            graph_edges: usize::MAX,
            active_graph_depth: usize::MAX,
            value_comparison: usize::MAX,
        };
        let clamped = state::evaluate(package, request(package), &StateView::default(), above_hard);
        assert_eq!(clamped.limits(), Limits::default());

        let zero_unused = Limits {
            input_value_nodes: 0,
            input_text_bytes: 0,
            input_structural_depth: 0,
            predicate_call_depth: 0,
            sequence_work: 0,
            graph_expansion: 0,
            graph_edges: 0,
            active_graph_depth: 0,
            value_comparison: 0,
            ..Limits::default()
        };
        let report = state::evaluate(
            package,
            request(package),
            &StateView::default(),
            zero_unused,
        );
        assert!(matches!(report.outcome(), EvaluationOutcome::Completed(_)));
        assert_eq!(report.usage().input_value_nodes, 0);
        assert_eq!(report.usage().input_text_bytes, 0);
        assert_eq!(report.usage().input_structural_depth, 0);
        assert_eq!(report.usage().predicate_call_depth, 0);
        assert_eq!(report.usage().sequence_work, 0);
        assert_eq!(report.usage().graph_expansion, 0);
        assert_eq!(report.usage().graph_edges, 0);
        assert_eq!(report.usage().active_graph_depth, 0);
        assert_eq!(report.usage().value_comparison, 0);
    });
}

/// Tracing: TC-137.
#[trace("TC-137", "NFR-009", "FR-046-AC-7", "FR-049-AC-7")]
#[test]
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
            // Eleven required-input discovery visits plus six supplied-input
            // entries (binder/table, field and three sequence members).
            input_aggregate_entries: 17,
            input_structural_depth: 3,
            expression_work: 6,
            active_expression_depth: 3,
            sequence_work: 3,
            // Three active query bindings, three map values and the root.
            retained_output: 7,
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
        assert_eq!(complete.usage().input_aggregate_entries, 17);
        assert_eq!(complete.usage().input_structural_depth, 3);
        assert_eq!(complete.usage().expression_work, 6);
        assert_eq!(complete.usage().active_expression_depth, 3);
        assert_eq!(complete.usage().sequence_work, 3);
        assert_eq!(complete.usage().retained_output, 7);

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
                    input_aggregate_entries: 16,
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
                    retained_output: 6,
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
#[trace("TC-128", "TC-137", "NFR-009", "FR-046-AC-7")]
#[test]
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
#[trace("TC-136", "TC-137", "NFR-009", "FR-049-AC-3", "FR-049-AC-7")]
#[test]
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
                // One pair plus the two equal-length UTF-8 bytes inspected.
                value_comparison: 3,
                ..Limits::default()
            },
        );
        assert!(matches!(
            exact.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(true))
        ));
        assert_eq!(exact.usage().input_text_bytes, 2);
        assert_eq!(exact.usage().value_comparison, 3);
        assert!(matches!(
            state::evaluate(
                package,
                request.clone(),
                &view,
                Limits {
                    input_text_bytes: 2,
                    value_comparison: 2,
                    ..Limits::default()
                }
            )
            .outcome(),
            EvaluationOutcome::Exhausted(exhaustion)
                if exhaustion.dimension == Dimension::ValueComparison
                    && exhaustion.cause == ExhaustionCause::Limit
                    && exhaustion.used == 1
                    && exhaustion.requested == 2
        ));
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
#[trace("TC-127", "FR-046-AC-3", "FR-046-AC-4", "FR-046-AC-8")]
#[test]
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

/// Tracing: TC-127.
#[trace("TC-127", "FR-046-AC-4")]
#[test]
fn tc_127_all_eight_queries_have_their_exact_empty_runtime_results() {
    admitted_queries(|package| {
        let (owner, view) = query_view_with_amounts(package, &[]);
        let selected = [
            (
                operation(
                    package,
                    owner,
                    |op| matches!(op, w::ValueOperation::Size { collection, .. } if matches!(package.package().declarations[owner as usize].values[collection.index as usize].operation, w::ValueOperation::Field { .. })),
                ),
                "0",
            ),
            (
                operation(
                    package,
                    owner,
                    |op| matches!(op, w::ValueOperation::Contains { collection, .. } if matches!(package.package().declarations[owner as usize].values[collection.index as usize].operation, w::ValueOperation::Field { .. })),
                ),
                "false",
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
                "false",
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
                "[]",
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
                "[]",
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
                "0",
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
                "0",
            ),
        ];
        for (case, (handle, expected)) in selected.into_iter().enumerate() {
            assert_eq!(
                completed(package, owner, handle, &view),
                expected,
                "empty query case {case}"
            );
        }
    });
}

/// Tracing: TC-127, TC-128, TC-136.
#[trace(
    "TC-127",
    "TC-128",
    "TC-136",
    "FR-046-AC-4",
    "FR-046-AC-6",
    "FR-049-AC-4"
)]
#[test]
fn tc_127_128_full_traversal_discards_partial_results_and_validation_precedes_decision() {
    admitted_queries(|package| {
        let (owner, middle) = query_view_with_amounts(
            package,
            &[
                AmountInput::Available(2),
                AmountInput::Missing { claimed_index: 1 },
                AmountInput::Available(3),
            ],
        );
        let missing = match &middle.binders[0].value {
            InputSlot::Available(record) => match record.kind() {
                ValueKind::Record(fields) => match &fields[0].value {
                    FieldValue::Compiled(InputSlot::Available(amounts)) => match amounts.kind() {
                        ValueKind::Sequence(values) => match &values[1] {
                            InputSlot::Unavailable(missing) => missing.clone(),
                            _ => unreachable!("middle member is unavailable"),
                        },
                        _ => unreachable!("amounts is a sequence"),
                    },
                    _ => unreachable!("amounts is available"),
                },
                _ => unreachable!("query input is a record"),
            },
            _ => unreachable!("query input is available"),
        };
        for operator in [
            w::Query::Filter,
            w::Query::Map,
            w::Query::Count,
            w::Query::Sum,
        ] {
            let value = operation(
                package,
                owner,
                |op| matches!(op, w::ValueOperation::Query { operator: actual, .. } if *actual == operator),
            );
            let report = state::evaluate(
                package,
                EvaluationRequest {
                    declaration: owner,
                    value,
                },
                &middle,
                Limits::default(),
            );
            assert!(
                matches!(report.outcome(), EvaluationOutcome::Incomplete(actual) if actual == &missing)
            );
        }

        let (_, corrupt_after_decision) = query_view_with_amounts(
            package,
            &[AmountInput::Available(2), AmountInput::Available(21)],
        );
        let contains = operation(
            package,
            owner,
            |op| matches!(op, w::ValueOperation::Contains { collection, .. } if matches!(package.package().declarations[owner as usize].values[collection.index as usize].operation, w::ValueOperation::Field { .. })),
        );
        let report = state::evaluate(
            package,
            EvaluationRequest {
                declaration: owner,
                value: contains,
            },
            &corrupt_after_decision,
            Limits::default(),
        );
        assert!(matches!(
            report.outcome(),
            EvaluationOutcome::Refused(Refusal::Bounds(_))
        ));
        assert_eq!(report.usage().expression_work, 0);
    });
}

/// Tracing: TC-127, TC-128, TC-136.
#[trace(
    "TC-127",
    "TC-128",
    "TC-136",
    "FR-046-AC-4",
    "FR-046-AC-6",
    "FR-049-AC-4"
)]
#[test]
fn tc_127_128_unavailable_members_obey_decisive_order_and_exact_position() {
    admitted_queries(|package| {
        let (owner, before) = query_view_with_amounts(
            package,
            &[
                AmountInput::Missing { claimed_index: 0 },
                AmountInput::Available(2),
            ],
        );
        let (_, after) = query_view_with_amounts(
            package,
            &[
                AmountInput::Available(2),
                AmountInput::Missing { claimed_index: 1 },
            ],
        );
        let contains = operation(package, owner, |op| {
            matches!(op, w::ValueOperation::Contains { collection, .. }
                if matches!(package.package().declarations[owner as usize].values[collection.index as usize].operation,
                    w::ValueOperation::Field { .. }))
        });
        let missing = first_query_missing(&before);
        let selected = EvaluationRequest {
            declaration: owner,
            value: contains.clone(),
        };
        assert!(matches!(
            state::evaluate(package, selected.clone(), &before, Limits::default()).outcome(),
            EvaluationOutcome::Incomplete(actual) if actual == &missing
        ));
        assert!(matches!(
            state::evaluate(package, selected.clone(), &after, Limits::default()).outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(true))
        ));

        let (_, wrong_position) = query_view_with_amounts(
            package,
            &[
                AmountInput::Available(2),
                AmountInput::Missing { claimed_index: 0 },
            ],
        );
        assert!(matches!(
            state::evaluate(
                package,
                selected.clone(),
                &wrong_position,
                Limits::default()
            )
            .outcome(),
            EvaluationOutcome::Refused(Refusal::Authority(_))
        ));

        let mut detached_path = before.clone();
        let InputSlot::Available(record) = &detached_path.binders[0].value else {
            unreachable!("query input is available")
        };
        let ValueKind::Record(original_fields) = record.kind() else {
            unreachable!("query input is a record")
        };
        let mut fields = original_fields.clone();
        let FieldValue::Compiled(InputSlot::Available(amounts)) = &fields[0].value else {
            unreachable!("amounts is available")
        };
        let ValueKind::Sequence(original_values) = amounts.kind() else {
            unreachable!("amounts is a sequence")
        };
        let mut values = original_values.clone();
        let InputSlot::Unavailable(MissingInput::Member { path, .. }) = &mut values[0] else {
            unreachable!("first member is unavailable")
        };
        path.remove(0);
        fields[0].value = FieldValue::Compiled(InputSlot::Available(Value::new(
            amounts.value_type,
            ValueKind::Sequence(values),
        )));
        detached_path.binders[0].value =
            InputSlot::Available(Value::new(record.value_type, ValueKind::Record(fields)));
        assert!(matches!(
            state::evaluate(
                package,
                EvaluationRequest {
                    declaration: owner,
                    value: contains,
                },
                &detached_path,
                Limits::default(),
            )
            .outcome(),
            EvaluationOutcome::Refused(Refusal::Authority(_))
        ));
    });
}

/// Tracing: TC-136, TC-137.
#[trace("TC-136", "TC-137", "FR-049-AC-4", "FR-049-AC-7", "NFR-009")]
#[test]
fn tc_136_nested_unavailable_precedes_root_retention_exhaustion() {
    admitted_queries(|package| {
        let (owner, view) =
            query_view_with_amounts(package, &[AmountInput::Missing { claimed_index: 0 }]);
        let missing = first_query_missing(&view);
        let input = view.binders[0].binder.clone();
        let read = operation(
            package,
            owner,
            |op| matches!(op, w::ValueOperation::Read { binder } if binder == &input),
        );
        let report = state::evaluate(
            package,
            EvaluationRequest {
                declaration: owner,
                value: read,
            },
            &view,
            Limits {
                retained_output: 0,
                ..Limits::default()
            },
        );
        assert!(
            matches!(report.outcome(), EvaluationOutcome::Incomplete(actual) if actual == &missing)
        );
        assert_eq!(report.usage().retained_output, 0);
    });
}

/// Tracing: TC-129, TC-130, TC-131, TC-136.
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
#[test]
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

/// Tracing: TC-129.
#[trace("TC-129", "FR-047-AC-1", "FR-047-AC-2", "FR-047-AC-3")]
#[test]
fn tc_129_complete_domain_validation_precedes_decisive_graph_execution() {
    admitted_graph(|package| {
        let (owner, root, view) = graph_view(package);
        let request = EvaluationRequest {
            declaration: owner,
            value: root,
        };
        let first = view.populations[0].objects[0].key.clone();
        let mut dangling = first.clone();
        dangling.identifier = "absent-target".into();

        let mut hidden_after_target = view.clone();
        replace_graph_links(
            &mut hidden_after_target.populations[0].objects[0],
            [first.clone(), dangling.clone()],
        );
        assert!(matches!(
            state::evaluate(
                package,
                request.clone(),
                &hidden_after_target,
                Limits::default()
            )
            .outcome(),
            EvaluationOutcome::Refused(Refusal::Dangling(key)) if key == &dangling
        ));

        let mut incomplete = hidden_after_target.clone();
        incomplete.populations[0].closure = Err(MissingInput::Closure(
            incomplete.populations[0].closure_requirement.clone(),
        ));
        assert!(matches!(
            state::evaluate(package, request.clone(), &incomplete, Limits::default()).outcome(),
            EvaluationOutcome::Incomplete(MissingInput::Closure(_))
        ));

        let mut foreign_observation = incomplete;
        let mut foreign = dangling;
        foreign.observation.snapshot = ObservationIdentity("snapshot:foreign".into());
        replace_graph_links(
            &mut foreign_observation.populations[0].objects[0],
            [first, foreign.clone()],
        );
        assert!(matches!(
            state::evaluate(
                package,
                request,
                &foreign_observation,
                Limits::default()
            )
            .outcome(),
            EvaluationOutcome::Refused(Refusal::PopulationDomain(key)) if key == &foreign
        ));
    });
}

/// Tracing: TC-130.
#[trace("TC-130", "FR-047-AC-4", "FR-047-AC-5")]
#[test]
fn tc_130_multi_record_cycle_uses_full_keys_and_positive_length_paths() {
    admitted_graph(|package| {
        let (owner, root, mut view) = graph_view(package);
        let first = view.populations[0].objects[0].key.clone();
        let second = view.populations[0].objects[1].key.clone();
        assert_ne!(first.observation.record, second.observation.record);
        replace_graph_links(&mut view.populations[0].objects[0], [second.clone()]);
        replace_graph_links(&mut view.populations[0].objects[1], [first]);
        let report = state::evaluate(
            package,
            EvaluationRequest {
                declaration: owner,
                value: root.clone(),
            },
            &view,
            Limits::default(),
        );
        assert!(matches!(
            report.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(true))
        ));
        assert_eq!(report.usage().graph_expansion, 2);
        assert_eq!(report.usage().graph_edges, 2);
        assert_eq!(report.usage().active_graph_depth, 2);
        assert_eq!(report.usage().value_comparison, 2);

        replace_graph_links(&mut view.populations[0].objects[0], []);
        replace_graph_links(&mut view.populations[0].objects[1], []);
        let isolated = state::evaluate(
            package,
            EvaluationRequest {
                declaration: owner,
                value: root.clone(),
            },
            &view,
            Limits::default(),
        );
        assert!(matches!(
            isolated.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(false))
        ));
        assert_eq!(isolated.usage().graph_expansion, 1);
        assert_eq!(isolated.usage().graph_edges, 0);

        let second = view.populations[0].objects[1].key.clone();
        replace_graph_links(
            &mut view.populations[0].objects[0],
            [second.clone(), second],
        );
        let duplicate_edge = state::evaluate(
            package,
            EvaluationRequest {
                declaration: owner,
                value: root,
            },
            &view,
            Limits::default(),
        );
        assert!(matches!(
            duplicate_edge.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(false))
        ));
        assert_eq!(duplicate_edge.usage().graph_expansion, 2);
        assert_eq!(duplicate_edge.usage().graph_edges, 2);
        assert_eq!(duplicate_edge.usage().value_comparison, 2);
    });
}

/// Tracing: TC-130.
#[trace("TC-130", "FR-047-AC-6")]
#[test]
fn tc_130_absent_optional_reference_has_no_outgoing_edge() {
    admitted_optional_graph(|package| {
        let (owner, root, mut view) = graph_view(package);
        for object in &mut view.populations[0].objects {
            object.fields[0].value = FieldValue::Contextual(ContextualSlot::Available(
                ContextualValue::new(ContextualValueKind::Option(None)),
            ));
        }
        let report = state::evaluate(
            package,
            EvaluationRequest {
                declaration: owner,
                value: root,
            },
            &view,
            Limits::default(),
        );
        assert!(matches!(
            report.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(false))
        ));
        assert_eq!(report.usage().graph_expansion, 1);
        assert_eq!(report.usage().graph_edges, 0);
        assert_eq!(report.usage().value_comparison, 0);
    });
}

/// Tracing: TC-130.
#[trace("TC-130", "FR-047-AC-4", "FR-047-AC-5")]
#[test]
fn tc_130_authored_sibling_order_selects_depth_first_work_before_the_target() {
    admitted_graph(|package| {
        let (owner, root, mut early) = graph_view(package);
        let first = early.populations[0].objects[0].key.clone();
        let second = early.populations[0].objects[1].key.clone();
        replace_graph_links(
            &mut early.populations[0].objects[0],
            [first.clone(), second.clone()],
        );
        replace_graph_links(&mut early.populations[0].objects[1], []);

        let mut late = early.clone();
        replace_graph_links(&mut late.populations[0].objects[0], [second.clone(), first]);
        let request = EvaluationRequest {
            declaration: owner,
            value: root,
        };
        let early_report = state::evaluate(package, request.clone(), &early, Limits::default());
        let late_report = state::evaluate(package, request.clone(), &late, Limits::default());
        assert!(matches!(
            early_report.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(true))
        ));
        assert!(matches!(
            late_report.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(true))
        ));
        assert_eq!(
            (
                early_report.usage().graph_expansion,
                early_report.usage().graph_edges,
                early_report.usage().value_comparison,
            ),
            (1, 1, 1)
        );
        assert_eq!(
            (
                late_report.usage().graph_expansion,
                late_report.usage().graph_edges,
                late_report.usage().value_comparison,
            ),
            (2, 2, 2)
        );

        let mut diamond = early;
        let mut third = diamond.populations[0].objects[1].clone();
        third.key.identifier = "logical-c".into();
        third.key.observation.record = ObservationIdentity("record:third".into());
        let third_key = third.key.clone();
        let mut fourth = third.clone();
        fourth.key.identifier = "logical-d".into();
        fourth.key.observation.record = ObservationIdentity("record:fourth".into());
        let fourth_key = fourth.key.clone();
        diamond.populations[0].objects.extend([third, fourth]);
        replace_graph_links(
            &mut diamond.populations[0].objects[0],
            [second, third_key.clone()],
        );
        replace_graph_links(&mut diamond.populations[0].objects[1], [fourth_key.clone()]);
        replace_graph_links(&mut diamond.populations[0].objects[2], [fourth_key]);
        replace_graph_links(&mut diamond.populations[0].objects[3], []);
        let diamond_report = state::evaluate(package, request, &diamond, Limits::default());
        assert!(matches!(
            diamond_report.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(false))
        ));
        assert_eq!(
            (
                diamond_report.usage().graph_expansion,
                diamond_report.usage().graph_edges,
                diamond_report.usage().active_graph_depth,
                diamond_report.usage().value_comparison,
            ),
            (4, 4, 3, 4)
        );
    });
}

/// Tracing: TC-131.
#[trace("TC-131", "FR-047-AC-7", "NFR-009-AC-4")]
#[test]
fn tc_131_multi_record_graph_limits_are_exact_and_retry_is_fresh() {
    admitted_graph(|package| {
        let (owner, root, mut view) = graph_view(package);
        let first = view.populations[0].objects[0].key.clone();
        let second = view.populations[0].objects[1].key.clone();
        replace_graph_links(&mut view.populations[0].objects[0], [second]);
        replace_graph_links(&mut view.populations[0].objects[1], [first]);
        let request = EvaluationRequest {
            declaration: owner,
            value: root,
        };
        let exact = Limits {
            graph_expansion: 2,
            graph_edges: 2,
            active_graph_depth: 2,
            value_comparison: 2,
            ..Limits::default()
        };
        let complete = state::evaluate(package, request.clone(), &view, exact);
        assert!(matches!(
            complete.outcome(),
            EvaluationOutcome::Completed(value)
                if matches!(value.kind(), ValueKind::Boolean(true))
        ));
        for (limits, dimension) in [
            (
                Limits {
                    graph_expansion: 1,
                    ..exact
                },
                Dimension::GraphExpansion,
            ),
            (
                Limits {
                    graph_edges: 1,
                    ..exact
                },
                Dimension::GraphEdges,
            ),
            (
                Limits {
                    active_graph_depth: 1,
                    ..exact
                },
                Dimension::ActiveGraphDepth,
            ),
            (
                Limits {
                    value_comparison: 1,
                    ..exact
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
        let retry = state::evaluate(package, request, &view, exact);
        assert_eq!(retry.outcome(), complete.outcome());
        assert_eq!(retry.usage(), complete.usage());
    });
}

/// Tracing: TC-137.
#[trace("TC-137", "FR-049-AC-7", "NFR-009-AC-1", "NFR-009-AC-2")]
#[test]
fn tc_137_object_field_value_roots_start_at_structural_depth_one() {
    admitted_graph(|package| {
        let (owner, root, view) = graph_view(package);
        let request = EvaluationRequest {
            declaration: owner,
            value: root,
        };
        let exact = state::evaluate(
            package,
            request.clone(),
            &view,
            Limits {
                input_structural_depth: 2,
                ..Limits::default()
            },
        );
        assert!(matches!(exact.outcome(), EvaluationOutcome::Completed(_)));
        assert_eq!(exact.usage().input_structural_depth, 2);
        assert!(matches!(
            state::evaluate(
                package,
                request,
                &view,
                Limits {
                    input_structural_depth: 1,
                    ..Limits::default()
                },
            )
            .outcome(),
            EvaluationOutcome::Exhausted(exhaustion)
                if exhaustion.dimension == Dimension::InputStructuralDepth
                    && exhaustion.used == 1
        ));
    });
}

/// Tracing: TC-136.
#[trace("TC-136", "FR-049-AC-2", "FR-049-AC-5")]
#[test]
fn tc_136_population_inputs_are_exact_not_best_effort_search_domains() {
    admitted_graph(|package| {
        let (owner, root, view) = graph_view(package);
        let request = EvaluationRequest {
            declaration: owner,
            value: root,
        };
        let mut missing = view.clone();
        let population = missing.populations.remove(0);
        assert!(matches!(
            state::evaluate(package, request.clone(), &missing, Limits::default()).outcome(),
            EvaluationOutcome::Refused(Refusal::MissingBinding(handle))
                if handle == &population.requirement
        ));

        let mut duplicate = view.clone();
        duplicate.populations.push(population);
        assert!(matches!(
            state::evaluate(package, request.clone(), &duplicate, Limits::default()).outcome(),
            EvaluationOutcome::Refused(Refusal::DuplicateBinding(_))
        ));

        let flow = package
            .package()
            .declarations
            .iter()
            .position(|declaration| declaration.name == "Flow")
            .expect("unrelated protocol declaration") as u32;
        let declaration = &package.package().declarations[flow as usize];
        let population_index = declaration
            .bindings
            .iter()
            .position(|binding| binding.kind == w::BindingKind::Population)
            .expect("unrelated population requirement");
        let closure_index = declaration
            .bindings
            .iter()
            .position(|binding| {
                binding.kind == w::BindingKind::Closure
                    && binding.requires == [population_index as u32]
            })
            .expect("unrelated closure requirement");
        let unrelated = PopulationInput {
            requirement: w::Handle {
                declaration: flow,
                index: population_index as u32,
            },
            closure_requirement: w::Handle {
                declaration: flow,
                index: closure_index as u32,
            },
            authority: authority(package, flow, population_index as u32),
            membership: Err(MissingInput::Membership(w::Handle {
                declaration: flow,
                index: population_index as u32,
            })),
            closure: Ok(()),
            objects: Vec::new(),
        };
        let mut surplus = view;
        surplus.populations.insert(0, unrelated);
        assert!(matches!(
            state::evaluate(package, request, &surplus, Limits::default()).outcome(),
            EvaluationOutcome::Refused(Refusal::SurplusBinding(handle))
                if handle.declaration == flow
        ));
    });
}
