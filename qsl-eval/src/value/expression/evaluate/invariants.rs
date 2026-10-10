// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-090 producer probes and FR-096 received-kernel provenance.

use super::*;
use ix_trace_rs::trace;
use qsl_semantics::check::CheckedBody;
use quire_semantic_value::location::Origin;

macro_rules! probe {
    ($identifier:expr, $machine:ident, $charges:expr, $action:block) => {{
        probe_environment!(
            $identifier,
            $machine,
            ObjectEnvironment::default(),
            $charges,
            $action
        )
    }};
}

macro_rules! probe_environment {
    ($identifier:expr, $machine:ident, $objects:expr, $charges:expr, $action:block) => {{
        let (package, _) = super::tests::population_function_package();
        probe_package!($identifier, $machine, package, $objects, $charges, $action)
    }};
}

macro_rules! probe_package {
    ($identifier:expr, $machine:ident, $package:expr, $objects:expr, $charges:expr, $action:block) => {{
        let package = $package;
        let graph = package.graph();
        let objects = $objects;
        let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let fault = {
            let mut __probe_machine_storage = Machine::new(
                graph.scope(),
                graph,
                &objects,
                &mut meter,
                graph.dispatch_tables(),
            );
            let $machine = &mut __probe_machine_storage;
            let result: Result<(), Halt> = $action;
            let fault = match result {
                Err(Halt::Fault(fault)) => fault,
                Err(_) => panic!("producer returned an ordinary halt: {}", $identifier),
                Ok(()) => panic!("producer was not reached: {}", $identifier),
            };
            assert_eq!(fault.stage(), "S6a");
            assert_eq!(fault.invariant(), $identifier);
            assert_eq!(fault.kernel_cause(), None);
            assert_eq!(fault.category(), qsl_foundation::Category::InternalFailure);
            fault
        };
        let expected: &[ChargePoint] = &$charges;
        assert_eq!(meter.admitted_charges(), expected, "{}", $identifier);
    }};
}

fn body(kind: impl FnOnce(&[qsl_semantics::check::NodeId]) -> NodeKind) -> CheckedBody {
    CheckedBody::fixture(
        vec![ValueType::Boolean, ValueType::Boolean],
        kind,
        ValueType::Boolean,
    )
}

fn collection(elements: Vec<Value>) -> Arc<CollectionValue> {
    let Value::Collection(value) = quire_exact::from_admitted(
        quire_exact::CollectionType::new(CollectionKind::Sequence, ValueType::Boolean, None),
        elements,
    ) else {
        panic!("collection fixture");
    };
    value
}

fn iteration<'a>(node: CheckedNode<'a>) -> Box<Iteration<'a>> {
    Box::new(Iteration {
        node,
        source: collection(vec![Value::Boolean(true)]),
        next: 0,
        awaiting: false,
        results: vec![],
        sources: vec![],
        accumulator: None,
        count: Integer::zero(),
    })
}

fn reference() -> ObjectReference {
    let (package, _) = super::tests::population_function_package();
    let object_type = package
        .graph()
        .scope()
        .types()
        .object_types()
        .next()
        .unwrap()
        .key();
    ObjectReference::new(
        quire_exact::UniverseId::from_digest([0; 32]),
        object_type,
        quire_exact::ObjectId::new("receiver").unwrap(),
    )
}

fn claim_level(machine: &mut Machine<'_, '_>) {
    machine.frames = vec![vec![]];
    machine.iterations = 0;
    machine.trail = Some(Trail::new(
        super::super::s6a::separation::ObservationIdentity {
            authority: "test".to_owned(),
            identity: "producer-probe".to_owned(),
            revision_namespace: "test".to_owned(),
            revision: "1".to_owned(),
        },
        None,
    ));
}

fn earlier_charge_controls<'a>(
    package: &'a qsl_package::CheckedPackage,
    objects: &'a ObjectEnvironment,
    point: ChargePoint,
    mut action: impl for<'meter> FnMut(&mut Machine<'a, 'meter>) -> Result<(), Halt>,
) {
    let graph = package.graph();
    for cause in [
        None,
        Some(quire_exact::CancelCause::Requested),
        Some(quire_exact::CancelCause::Deadline),
    ] {
        let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let cancel = cause.map(|cause| {
            let cancel = quire_exact::Cancel::new();
            cancel.cancel(cause);
            cancel
        });
        if let Some(cancel) = &cancel {
            meter = meter.with_cancel(cancel.clone());
        } else {
            meter = meter.with_injected_denial(quire_exact::InjectedDenial {
                point,
                occurrence: std::num::NonZeroU64::new(1).unwrap(),
            });
        }
        let mut machine = Machine::new(
            graph.scope(),
            graph,
            objects,
            &mut meter,
            graph.dispatch_tables(),
        );
        let stop = match action(&mut machine) {
            Err(Halt::Stop(Stop::Incomplete(record))) => record,
            Err(Halt::Located(located)) => match located.stop {
                Stop::Incomplete(record) => record,
                _ => panic!("earlier charge did not stay Incomplete"),
            },
            _ => panic!("earlier charge failed to precede the producer"),
        };
        assert_eq!(stop.charge_point, point);
        drop(machine);
        assert!(meter.admitted_charges().is_empty());
        if let Some(cancel) = cancel {
            assert_eq!(cancel.tripped(), cause);
        }
    }
}

fn finish_tasks(machine: &mut Machine<'_, '_>) -> Result<(), Halt> {
    while let Some(task) = machine.tasks.pop() {
        machine.step(task)?;
    }
    Ok(())
}

fn checked_dispatch_package() -> qsl_package::CheckedPackage {
    use qsl_forms::{BuiltinType, Expression, FunctionDeclaration, TypeForm};
    use qsl_semantics::check::{DispatchCandidate, DispatchOperation, PackageDeclarations};
    let span = qsl_foundation::Span { start: 0, end: 0 };
    let owner = reference().object_type();
    let receiver = || {
        TypeForm::builtin(BuiltinType::Reference, span)
            .with_arguments(vec![TypeForm::name("Owner", span)])
    };
    let function = |name, holds| {
        FunctionDeclaration::new(
            name,
            vec![("receiver".to_owned(), receiver())],
            TypeForm::builtin(BuiltinType::Boolean, span),
            None,
            Expression::boolean(holds),
        )
    };
    let declarations = PackageDeclarations {
        types: quire_semantic_value::declaration::TypeEnvironment::new(
            [],
            [
                quire_semantic_value::declaration::ObjectTypeDeclaration::new(
                    owner,
                    "Owner",
                    vec![],
                ),
            ],
        )
        .unwrap(),
        functions: vec![function("Body", true), function("Guard", true)],
        dispatch_operations: vec![DispatchOperation {
            receiver_type: owner,
            member: "step".to_owned(),
            parameters: vec![ValueType::Reference(owner)],
            result: ValueType::Boolean,
            table: 0,
        }],
        dispatch_tables: vec![DispatchTable::new(
            vec![(
                owner,
                DispatchCandidate {
                    body: 0,
                    precondition: Some(1),
                    precondition_clauses: vec![1],
                },
            )],
            1,
        )],
        ..PackageDeclarations::new(
            qsl_semantics::check::fixture_source(),
            qsl_foundation::IdentityLimits::default(),
        )
    };
    qsl_package::CheckedPackage::link(
        declarations
            .check(quire_semantic_value::checking::CheckingLimits::default())
            .unwrap(),
    )
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn dispatch_table_candidate_and_each_callable_consumer_fault_at_its_own_site() {
    use qsl_semantics::check::DispatchCandidate;
    let node = body(|ids| NodeKind::Dispatch {
        receiver: ids[0],
        operation: 0,
        table: usize::MAX,
        arguments: vec![],
    });
    probe_package!(
        "dispatch-table-unresolved",
        m,
        checked_dispatch_package(),
        ObjectEnvironment::default(),
        [],
        {
            m.values.push(Value::Reference(reference()));
            m.apply(node.root())
        }
    );
    let node = body(|ids| NodeKind::Dispatch {
        receiver: ids[0],
        operation: 0,
        table: 0,
        arguments: vec![],
    });
    let foreign = ObjectReference::new(
        quire_exact::UniverseId::from_digest([0; 32]),
        quire_exact::EffectiveId::from_digest([0xfe; 32]),
        quire_exact::ObjectId::new("foreign-subtype").unwrap(),
    );
    probe_package!(
        "dispatch-candidate-unresolved",
        m,
        checked_dispatch_package(),
        ObjectEnvironment::default(),
        [ChargePoint::DispatchSelect],
        {
            m.values.push(Value::Reference(foreign.clone()));
            m.apply(node.root())
        }
    );
    let package = checked_dispatch_package();
    earlier_charge_controls(
        &package,
        &ObjectEnvironment::default(),
        ChargePoint::DispatchSelect,
        |m| {
            m.values.push(Value::Reference(foreign.clone()));
            m.apply(node.root())
        },
    );
    // Mutate exactly one callable index in an otherwise linked table:
    // precondition lookup, selected body after precondition lookup, bare body.
    for (precondition, selected) in [
        (Some(usize::MAX), 0),
        (Some(1), usize::MAX),
        (None, usize::MAX),
    ] {
        let tables = [DispatchTable::new(
            vec![(
                reference().object_type(),
                DispatchCandidate {
                    body: selected,
                    precondition,
                    precondition_clauses: vec![],
                },
            )],
            1,
        )];
        probe_package!(
            "evaluation-callable-unresolved",
            m,
            checked_dispatch_package(),
            ObjectEnvironment::default(),
            [ChargePoint::DispatchSelect],
            {
                m.dispatch_tables = &tables;
                m.values.push(Value::Reference(reference()));
                m.apply(node.root())
            }
        );
        earlier_charge_controls(
            &package,
            &ObjectEnvironment::default(),
            ChargePoint::DispatchSelect,
            |m| {
                m.dispatch_tables = &tables;
                m.values.push(Value::Reference(reference()));
                m.apply(node.root())
            },
        );
    }
    probe_package!(
        "evaluation-callable-unresolved",
        m,
        checked_dispatch_package(),
        ObjectEnvironment::default(),
        [],
        {
            m.frames.push(vec![]);
            m.values.push(Value::Boolean(true));
            m.step(Task::DispatchGuard(Box::new(DispatchGuard {
                body_function: usize::MAX,
                arguments: vec![Value::Reference(reference())],
                failure: PreconditionFailure {
                    operation: "step".to_owned(),
                    selected: "Body".to_owned(),
                    receiver: reference(),
                },
                node: node.root(),
            })))
        }
    );
}

#[test]
#[trace("TC-917", "FR-090-AC-16")]
fn resolved_dispatch_and_callable_consumers_keep_the_matching_valid_prefixes() {
    use qsl_semantics::check::DispatchCandidate;
    let package = checked_dispatch_package();
    let graph = package.graph();
    let objects = ObjectEnvironment::default();
    let receiver = reference();
    let node = CheckedBody::fixture(
        vec![ValueType::Reference(receiver.object_type())],
        |ids| NodeKind::Dispatch {
            receiver: ids[0],
            operation: 0,
            table: 0,
            arguments: vec![],
        },
        ValueType::Boolean,
    );
    for guarded in [true, false] {
        let tables = [DispatchTable::new(
            vec![(
                receiver.object_type(),
                DispatchCandidate {
                    body: 0,
                    precondition: guarded.then_some(1),
                    precondition_clauses: vec![],
                },
            )],
            1,
        )];
        let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let mut machine = Machine::new(graph.scope(), graph, &objects, &mut meter, &tables);
        machine.values.push(Value::Reference(receiver.clone()));
        machine
            .apply(node.root())
            .unwrap_or_else(|_| panic!("linked dispatch"));
        finish_tasks(&mut machine).unwrap_or_else(|_| panic!("linked callable bodies"));
        assert!(matches!(machine.values.as_slice(), [Value::Boolean(true)]));
        assert!(machine.frames.is_empty());
        drop(machine);
        let mut expected = vec![ChargePoint::DispatchSelect, ChargePoint::FunctionCall];
        if guarded {
            expected.push(ChargePoint::FunctionCall);
        }
        assert_eq!(meter.admitted_charges(), expected);
    }
    let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
    let mut machine = Machine::new(
        graph.scope(),
        graph,
        &objects,
        &mut meter,
        graph.dispatch_tables(),
    );
    machine
        .frames
        .push(vec![Some(Value::Reference(receiver.clone()))]);
    machine.values.push(Value::Boolean(true));
    machine
        .step(Task::DispatchGuard(Box::new(DispatchGuard {
            body_function: 0,
            arguments: vec![Value::Reference(receiver.clone())],
            failure: PreconditionFailure {
                operation: "step".to_owned(),
                selected: "Body".to_owned(),
                receiver: receiver.clone(),
            },
            node: node.root(),
        })))
        .unwrap_or_else(|_| panic!("linked guard body"));
    finish_tasks(&mut machine).unwrap_or_else(|_| panic!("guard result"));
    assert!(matches!(machine.values.as_slice(), [Value::Boolean(true)]));
    drop(machine);
    assert_eq!(meter.admitted_charges(), [ChargePoint::FunctionCall]);
    let node = CheckedBody::fixture(
        vec![ValueType::Reference(receiver.object_type())],
        |ids| NodeKind::Call {
            function: 0,
            arguments: vec![ids[0]],
        },
        ValueType::Boolean,
    );
    let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
    let mut machine = Machine::new(
        graph.scope(),
        graph,
        &objects,
        &mut meter,
        graph.dispatch_tables(),
    );
    machine.values.push(Value::Reference(receiver));
    machine
        .apply(node.root())
        .unwrap_or_else(|_| panic!("local callable"));
    finish_tasks(&mut machine).unwrap_or_else(|_| panic!("local call result"));
    assert!(matches!(machine.values.as_slice(), [Value::Boolean(true)]));
    drop(machine);
    assert_eq!(meter.admitted_charges(), [ChargePoint::FunctionCall]);
}

fn quantity_units() -> quire_semantic_value::quantity::UnitTable {
    use quire_semantic_value::unit::{DimensionNode, NominalDeclaration, UnitGraph, UnitNode};
    let dimension_a = quire_exact::NodeKey::from_digest([0xa1; 32]);
    let dimension_b = quire_exact::NodeKey::from_digest([0xb1; 32]);
    let unit_a = quire_exact::NodeKey::from_digest([0xa2; 32]);
    let unit_b = quire_exact::NodeKey::from_digest([0xb2; 32]);
    let nominal = |name: &str| NominalDeclaration {
        qualified_declaration: vec![name.to_owned()],
        preimage: vec![],
    };
    let dimensions = std::collections::BTreeMap::from([
        (
            dimension_a,
            DimensionNode::checked(vec![], nominal("Length")).unwrap(),
        ),
        (
            dimension_b,
            DimensionNode::checked(vec![], nominal("Time")).unwrap(),
        ),
    ]);
    let units = std::collections::BTreeMap::from([
        (
            unit_a,
            UnitNode::checked(
                *dimension_a.as_bytes(),
                None,
                Rational::from_integer(Integer::one()),
                Rational::from_integer(Integer::zero()),
                nominal("Metre"),
            )
            .unwrap(),
        ),
        (
            unit_b,
            UnitNode::checked(
                *dimension_b.as_bytes(),
                None,
                Rational::from_integer(Integer::one()),
                Rational::from_integer(Integer::zero()),
                nominal("Second"),
            )
            .unwrap(),
        ),
    ]);
    let graph = UnitGraph::from_checked_nodes(&dimensions, &units).unwrap();
    quire_semantic_value::quantity::UnitTable::declared(&graph)
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn imported_callable_consumer_uses_a_real_emitted_and_admitted_library() {
    use qsl_forms::{BuiltinType, Expression, FunctionDeclaration, TypeForm};
    use qsl_semantics::check::{AdmittedImport, PackageDeclarations};
    use qsl_semantics::library::{LibraryName, PackageNodeKey};
    let span = qsl_foundation::Span { start: 0, end: 0 };
    let mut declarations = PackageDeclarations::new(
        qsl_semantics::check::fixture_source(),
        qsl_foundation::IdentityLimits::default(),
    );
    declarations.functions.push(FunctionDeclaration::new(
        "Body",
        vec![],
        TypeForm::builtin(BuiltinType::Boolean, span),
        None,
        Expression::boolean(true),
    ));
    let library = qsl_package::CheckedPackage::link(
        declarations
            .check(quire_semantic_value::checking::CheckingLimits::default())
            .unwrap(),
    );
    let identity = LibraryName::new("fixture:callable").unwrap();
    let emission = qsl_package::emit_checked(&library).unwrap();
    let view = qsl_package::read_import_view(
        &library,
        &emission,
        identity.clone(),
        &std::collections::BTreeMap::new(),
        &mut qsl_package::AdmittedPackages::default(),
        qsl_package::V2ReadLimits::default(),
    )
    .unwrap();
    let callee = PackageNodeKey::new(
        emission.package().package_id(),
        qsl_foundation::digest::WireNodeId::from_digest(
            *library
                .graph()
                .callable("Body")
                .unwrap()
                .identity
                .as_bytes(),
        ),
    );
    let mut declarations = PackageDeclarations::new(
        qsl_semantics::check::fixture_source(),
        qsl_foundation::IdentityLimits::default(),
    );
    declarations.imports.insert(
        "lib".to_owned(),
        AdmittedImport {
            identity,
            view,
            graph: library.shared_graph(),
        },
    );
    let package = qsl_package::CheckedPackage::link(
        declarations
            .check(quire_semantic_value::checking::CheckingLimits::default())
            .unwrap(),
    );
    let node = body(|_| NodeKind::ImportedCall {
        callee,
        function: usize::MAX,
        arguments: vec![],
    });
    probe_package!(
        "evaluation-callable-unresolved",
        m,
        &package,
        ObjectEnvironment::default(),
        [],
        { m.apply(node.root()) }
    );
    let node = body(|_| NodeKind::ImportedCall {
        callee,
        function: 0,
        arguments: vec![],
    });
    let graph = package.graph();
    let objects = ObjectEnvironment::default();
    let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
    let mut machine = Machine::new(
        graph.scope(),
        graph,
        &objects,
        &mut meter,
        graph.dispatch_tables(),
    );
    machine
        .apply(node.root())
        .unwrap_or_else(|_| panic!("admitted imported callable"));
    finish_tasks(&mut machine).unwrap_or_else(|_| panic!("imported body result"));
    assert!(matches!(machine.values.as_slice(), [Value::Boolean(true)]));
    assert!(machine.imported.is_empty());
    drop(machine);
    assert_eq!(meter.admitted_charges(), [ChargePoint::FunctionCall]);
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn quantity_operation_and_ordering_rejections_reach_the_real_typed_operators() {
    let units = quantity_units();
    let mut ids = units.ids();
    let left = Value::Quantity(quire_exact::Quantity::new(
        Rational::from_integer(Integer::one()),
        ids.next().unwrap(),
    ));
    let right = Value::Quantity(quire_exact::Quantity::new(
        Rational::from_integer(Integer::one()),
        ids.next().unwrap(),
    ));
    let node = body(|ids| NodeKind::Quantity(ArithmeticOperator::Add, ids[0], ids[1]));
    probe!("quantity-operation-rejected", m, [], {
        m.units = UnitScope::new(&units);
        m.values = vec![left.clone(), right.clone()];
        m.apply(node.root())
    });
    probe!("ordered-quantity-comparison-rejected", m, [], {
        m.units = UnitScope::new(&units);
        m.order(
            OrderingOperator::Less,
            OrderedKind::Quantities,
            &left,
            &right,
        )
        .map(|_| ())
    });
    let (package, _) = super::tests::population_function_package();
    let objects = ObjectEnvironment::default();
    let graph = package.graph();
    let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
    let mut machine = Machine::new(
        graph.scope(),
        graph,
        &objects,
        &mut meter,
        graph.dispatch_tables(),
    );
    machine.units = UnitScope::new(&units);
    assert!(!machine
        .order(
            OrderingOperator::Less,
            OrderedKind::Quantities,
            &left,
            &left
        )
        .unwrap_or_else(|_| panic!("same-unit comparison")));
    machine.values = vec![left.clone(), left.clone()];
    machine
        .apply(node.root())
        .unwrap_or_else(|_| panic!("same-unit addition"));
    let Value::Quantity(sum) = machine.values.pop().unwrap() else {
        panic!("quantity sum");
    };
    let Value::Quantity(left) = left else {
        unreachable!()
    };
    assert_eq!(
        sum.magnitude(),
        &Rational::from_integer(Integer::from(2_i64))
    );
    assert_eq!(sum.unit(), left.unit());
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn query_stop_provenance_rejects_a_missing_selected_position_at_the_real_quantifier() {
    use super::super::s6a::separation::{ObservationIdentity, Provenance};
    use qsl_forms::{BinderQuery, BuiltinType, Expression, FunctionDeclaration, TypeForm};
    use qsl_semantics::check::PackageDeclarations;
    let span = qsl_foundation::Span { start: 0, end: 0 };
    let mut declarations = PackageDeclarations::new(
        qsl_semantics::check::fixture_source(),
        qsl_foundation::IdentityLimits::default(),
    );
    declarations.functions.push(FunctionDeclaration::new(
        "Quantified",
        vec![],
        TypeForm::builtin(BuiltinType::Boolean, span),
        None,
        Expression::query(
            BinderQuery::Forall,
            "element",
            Expression::collection(CollectionKind::Sequence, vec![Expression::boolean(true)]),
            Expression::name("element"),
        ),
    ));
    let package = qsl_package::CheckedPackage::link(
        declarations
            .check(quire_semantic_value::checking::CheckingLimits::default())
            .unwrap(),
    );
    let graph = package.graph();
    let node = graph.function_state(0).unwrap().body;
    let objects = ObjectEnvironment::default();
    for missing_position in [true, false] {
        let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let mut machine = Machine::new(
            graph.scope(),
            graph,
            &objects,
            &mut meter,
            graph.dispatch_tables(),
        );
        claim_level(&mut machine);
        let mut i = iteration(node);
        i.next = 1;
        let provenance = Provenance::member(
            ObservationIdentity {
                authority: "test".to_owned(),
                identity: "producer-probe".to_owned(),
                revision_namespace: "test".to_owned(),
                revision: "1".to_owned(),
            },
            reference(),
            "items".to_owned(),
        );
        let provenance = if missing_position {
            provenance.select(&[]).unwrap()
        } else {
            provenance
        };
        machine
            .trail
            .as_mut()
            .unwrap()
            .record(&Value::Collection(i.source.clone()), provenance);
        let result = machine.note_stop(&i);
        if missing_position {
            let Err(Halt::Fault(fault)) = result else {
                panic!("provenance producer");
            };
            assert_eq!(fault.invariant(), "query-stop-provenance-position-invalid");
            assert_eq!(fault.stage(), "S6a");
            assert_eq!(fault.kernel_cause(), None);
        } else {
            result.unwrap_or_else(|_| panic!("valid provenance position"));
            assert_eq!(
                machine
                    .trail
                    .as_ref()
                    .unwrap()
                    .provenance(&Value::Collection(i.source.clone()))
                    .unwrap()
                    .position(0),
                Some(0)
            );
        }
        drop(machine);
        assert!(meter.admitted_charges().is_empty());
    }
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn result_stack_reference_and_import_shapes_reach_their_actual_producers() {
    let literal = body(|_| NodeKind::Literal(CheckedLiteral(Value::Boolean(true))));
    probe!("evaluation-result-stack-invalid", m, [], {
        m.values.push(Value::Boolean(false));
        m.drive(literal.root()).map(|_| ()).map_err(Halt::Fault)
    });
    let field = FieldRef::new(reference().object_type(), "missing");
    let reaches = body(|ids| NodeKind::Reaches {
        source: ids[0],
        target: ids[1],
        edge: field.clone(),
    });
    for values in [
        vec![Value::Boolean(true), Value::Boolean(true)],
        vec![Value::Boolean(true), Value::Reference(reference())],
    ] {
        probe!("reaches-reference-value-expected", m, [], {
            m.values = values;
            m.apply(reaches.root())
        });
    }
    let attribute = body(|ids| NodeKind::Attribute {
        reference: ids[0],
        field: field.clone(),
        optional: false,
        derefed: false,
    });
    probe!("attribute-reference-value-expected", m, [], {
        m.values.push(Value::Boolean(true));
        m.apply(attribute.root())
    });
    probe!("attribute-slot-unresolved", m, [], {
        m.values.push(Value::Reference(reference()));
        m.apply(attribute.root())
    });
    let dispatch = body(|ids| NodeKind::Dispatch {
        receiver: ids[0],
        operation: usize::MAX,
        table: 0,
        arguments: vec![],
    });
    probe!("dispatch-operation-unresolved", m, [], {
        m.values.push(Value::Reference(reference()));
        m.apply(dispatch.root())
    });
    let callee = qsl_semantics::library::PackageNodeKey::new(
        qsl_semantics::library::PackageId::of_preimage(b"unresolved fixture package"),
        qsl_foundation::digest::WireNodeId::from_digest([0xfe; 32]),
    );
    let imported = body(|_| NodeKind::ImportedCall {
        callee,
        function: 0,
        arguments: vec![],
    });
    probe!("evaluation-imported-graph-unresolved", m, [], {
        m.apply(imported.root())
    });
    let edge = FieldRef::new(reference().object_type(), "missing");
    probe!("edge-attribute-unresolved", m, [], {
        m.edge_targets(literal.root(), &reference(), &edge)
            .map(|_| ())
    });
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn query_type_consumers_are_exercised_individually_after_population_resolution() {
    let binding = super::tests::population_binding(Some(3));
    let population = Value::Population(binding.population_id());
    let all = CheckedBody::fixture(
        vec![ValueType::Population(Some(3))],
        |ids| NodeKind::AllInstances { population: ids[0] },
        ValueType::Boolean,
    );
    probe!("all-instances-collection-result-expected", m, [], {
        m.values.push(population.clone());
        m.apply(all.root())
    });
    let lookup = body(|ids| NodeKind::Lookup {
        population: ids[0],
        reference: ids[1],
        absence: qsl_foundation::absence::AbsenceMode::Empty,
    });
    probe!("query-population-type-expected", m, [], {
        m.values = vec![population.clone(), Value::Boolean(true)];
        m.apply(lookup.root())
    });
    let lookup = CheckedBody::fixture(
        vec![ValueType::Population(Some(3)), ValueType::Boolean],
        |ids| NodeKind::Lookup {
            population: ids[0],
            reference: ids[1],
            absence: qsl_foundation::absence::AbsenceMode::Empty,
        },
        ValueType::Boolean,
    );
    probe_environment!(
        "lookup-reference-type-expected",
        m,
        ObjectEnvironment::default()
            .with_population(binding.clone())
            .unwrap(),
        [],
        {
            m.values = vec![population.clone(), Value::Boolean(true)];
            m.apply(lookup.root())
        }
    );
    for result_type in [ValueType::Boolean, ValueType::option(ValueType::Boolean)] {
        let lookup = CheckedBody::fixture(
            vec![
                ValueType::Population(Some(3)),
                ValueType::Reference(reference().object_type()),
            ],
            |ids| NodeKind::Lookup {
                population: ids[0],
                reference: ids[1],
                absence: qsl_foundation::absence::AbsenceMode::Empty,
            },
            result_type,
        );
        probe_environment!(
            "lookup-reference-result-expected",
            m,
            ObjectEnvironment::default()
                .with_population(binding.clone())
                .unwrap(),
            [],
            {
                m.values = vec![population.clone(), Value::Reference(reference())];
                m.apply(lookup.root())
            }
        );
    }
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn both_ieee_profile_checks_and_conversion_operand_check_are_real_producers() {
    let ieee = body(|ids| {
        NodeKind::Ieee(
            ArithmeticOperator::Add,
            quire_exact::RoundingMode::Exact,
            ids[0],
            ids[1],
        )
    });
    probe!("ieee-profile-unresolved", m, [], {
        m.values = vec![
            Value::Float(quire_exact::IeeeValue::binary32(0)),
            Value::Float(quire_exact::IeeeValue::binary32(0)),
        ];
        m.apply(ieee.root())
    });
    let domain = quire_exact::RationalDomain::new(
        IntegerInterval::spanning(Integer::zero(), Integer::one()),
        IntegerInterval::spanning(Integer::one(), Integer::one()),
    )
    .unwrap();
    let convert = body(|ids| NodeKind::IeeeToRational(ids[0], domain));
    probe!("ieee-conversion-float-value-expected", m, [], {
        m.values.push(Value::Boolean(true));
        m.apply(convert.root())
    });
    probe!("ieee-profile-unresolved", m, [], {
        m.values
            .push(Value::Float(quire_exact::IeeeValue::binary32(0)));
        m.apply(convert.root())
    });
    let lock = qsl_semantics::value::DefinitionLock::pinned();
    let entry = lock.entry(qsl_semantics::value::CatalogRole::IeeeProfile);
    let profile = lock
        .admit_ieee_profile(
            &[qsl_semantics::value::DefinitionReference {
                authority: entry.authority.to_owned(),
                identity: entry.identity.to_owned(),
            }],
            &[],
        )
        .unwrap();
    let graph = qsl_semantics::check::PackageDeclarations {
        ieee_profile: Some(profile),
        ..qsl_semantics::check::PackageDeclarations::new(
            qsl_semantics::check::fixture_source(),
            qsl_foundation::IdentityLimits::default(),
        )
    }
    .check(quire_semantic_value::checking::CheckingLimits::default())
    .unwrap();
    let package = qsl_package::CheckedPackage::link(graph);
    probe_package!(
        "ieee-operation-rejected",
        m,
        package,
        ObjectEnvironment::default(),
        [],
        {
            m.values = vec![
                Value::Float(quire_exact::IeeeValue::binary32(0)),
                Value::Float(quire_exact::IeeeValue::binary64(0)),
            ];
            m.apply(ieee.root())
        }
    );
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn field_slot_and_record_constructor_fail_at_the_actual_application_sites() {
    let key = quire_exact::NodeKey::from_digest([4; 32]);
    let field = body(|ids| NodeKind::Field {
        operand: ids[0],
        index: 1,
        optional: false,
    });
    probe!("field-slot-unresolved", m, [], {
        m.values.push(quire_exact::from_admitted_slots(
            key,
            vec![FieldValue::Present(Value::Boolean(true))].into_boxed_slice(),
        ));
        m.apply(field.root())
    });
    let declaration = quire_semantic_value::declaration::CompositeDeclaration::new(
        key,
        "Holder",
        CompositeShape::Record(vec![
            quire_semantic_value::declaration::FieldDeclaration::new(
                "required",
                ValueType::Boolean,
                quire_exact::Presence::Required,
            ),
        ]),
    );
    let graph = qsl_semantics::check::PackageDeclarations {
        types: quire_semantic_value::declaration::TypeEnvironment::new([declaration], []).unwrap(),
        ..qsl_semantics::check::PackageDeclarations::new(
            qsl_semantics::check::fixture_source(),
            qsl_foundation::IdentityLimits::default(),
        )
    }
    .check(quire_semantic_value::checking::CheckingLimits::default())
    .unwrap();
    let package = qsl_package::CheckedPackage::link(graph);
    let record = body(|_| NodeKind::Record {
        declaration: key,
        slots: vec![RecordSlot::Absent],
    });
    probe_package!(
        "record-construction-rejected",
        m,
        package,
        ObjectEnvironment::default(),
        [],
        { m.apply(record.root()) }
    );
}

fn object_package(
    value_type: ValueType,
    presence: quire_exact::Presence,
) -> qsl_package::CheckedPackage {
    let declaration = quire_semantic_value::declaration::ObjectTypeDeclaration::new(
        reference().object_type(),
        "Owner",
        vec![quire_semantic_value::declaration::FieldDeclaration::new(
            "edge", value_type, presence,
        )],
    );
    let graph = qsl_semantics::check::PackageDeclarations {
        types: quire_semantic_value::declaration::TypeEnvironment::new([], [declaration]).unwrap(),
        ..qsl_semantics::check::PackageDeclarations::new(
            qsl_semantics::check::fixture_source(),
            qsl_foundation::IdentityLimits::default(),
        )
    }
    .check(quire_semantic_value::checking::CheckingLimits::default())
    .unwrap();
    qsl_package::CheckedPackage::link(graph)
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn edge_traversal_shape_probes_reach_all_seven_detecting_sites() {
    let literal = body(|_| NodeKind::Literal(CheckedLiteral(Value::Boolean(true))));
    let field = FieldRef::new(reference().object_type(), "edge");
    let unknown = ObjectReference::new(
        quire_exact::UniverseId::from_digest([0; 32]),
        quire_exact::EffectiveId::from_digest([0xfe; 32]),
        quire_exact::ObjectId::new("unknown").unwrap(),
    );
    probe!("edge-attribute-declarations-unresolved", m, [], {
        m.edge_targets(literal.root(), &unknown, &field).map(|_| ())
    });
    let package = object_package(
        ValueType::Reference(reference().object_type()),
        quire_exact::Presence::Required,
    );
    probe_package!(
        "edge-slot-unresolved",
        m,
        package,
        ObjectEnvironment::default(),
        [],
        {
            m.edge_targets(literal.root(), &reference(), &field)
                .map(|_| ())
        }
    );
    let reference_type = ValueType::Reference(reference().object_type());
    let sequence = ValueType::collection(quire_exact::CollectionType::new(
        CollectionKind::Sequence,
        reference_type.clone(),
        None,
    ));
    for (expected, stored_type, stored, identifier) in [
        (
            reference_type.clone(),
            ValueType::Boolean,
            FieldValue::Present(Value::Boolean(true)),
            "edge-reference-slot-invalid",
        ),
        (
            sequence.clone(),
            ValueType::Boolean,
            FieldValue::Absent,
            "edge-sequence-slot-invalid",
        ),
        (
            sequence,
            ValueType::collection(quire_exact::CollectionType::new(
                CollectionKind::Sequence,
                ValueType::Boolean,
                None,
            )),
            FieldValue::Present(Value::Collection(collection(vec![Value::Boolean(true)]))),
            "edge-reference-element-expected",
        ),
        (
            ValueType::Boolean,
            ValueType::Boolean,
            FieldValue::Present(Value::Boolean(true)),
            "edge-field-type-invalid",
        ),
    ] {
        // Each closure is genuinely admitted against its storage environment.
        // Only the checked machine's field declaration is deliberately mismatched.
        let storage = object_package(stored_type, quire_exact::Presence::Optional);
        let closure = quire_semantic_value::object_closure::ObjectClosure::new(
            storage.graph().scope().types(),
            [(reference(), vec![("edge", stored)])],
            &[],
        )
        .unwrap();
        let package = object_package(expected, quire_exact::Presence::Required);
        probe_package!(
            identifier,
            m,
            package,
            ObjectEnvironment::new(closure),
            [],
            {
                m.edge_targets(literal.root(), &reference(), &field)
                    .map(|_| ())
            }
        );
    }
    for stored in [FieldValue::Absent, FieldValue::Null] {
        let package = object_package(reference_type.clone(), quire_exact::Presence::Optional);
        let closure = quire_semantic_value::object_closure::ObjectClosure::new(
            package.graph().scope().types(),
            [(reference(), vec![("edge", stored)])],
            &[],
        )
        .unwrap();
        let objects = ObjectEnvironment::new(closure);
        let graph = package.graph();
        let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let m = Machine::new(
            graph.scope(),
            graph,
            &objects,
            &mut meter,
            graph.dispatch_tables(),
        );
        assert!(m
            .edge_targets(literal.root(), &reference(), &field)
            .unwrap_or_else(|_| panic!("optional empty edge"))
            .is_empty());
        assert!(meter.admitted_charges().is_empty());
    }
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn ordering_lookup_and_profile_rejections_are_not_kernel_refusals() {
    let member = Value::Enum(quire_exact::EnumMember::new(
        quire_exact::VariantId::from_digest([0xfd; 32]),
        0,
    ));
    probe!("ordered-enumeration-variant-unresolved", m, [], {
        m.order(OrderingOperator::Less, OrderedKind::Enums, &member, &member)
            .map(|_| ())
    });
    for distinct in [false, true] {
        let declaration = quire_semantic_value::enumeration::EnumDeclaration::new(
            quire_exact::NodeKey::from_digest([0xfa; 32]),
            distinct,
            vec!["A".to_owned()],
        )
        .unwrap();
        let other = if distinct {
            quire_semantic_value::enumeration::EnumDeclaration::new(
                quire_exact::NodeKey::from_digest([0xfb; 32]),
                true,
                vec!["A".to_owned()],
            )
            .unwrap()
        } else {
            declaration.clone()
        };
        let left = declaration
            .member("A", quire_exact::NodeKey::from_digest([0xfc; 32]))
            .unwrap();
        let right = other
            .member("A", quire_exact::NodeKey::from_digest([0xfd; 32]))
            .unwrap();
        let left_value = Value::Enum(quire_exact::EnumMember::new(left.variant(), 0));
        let right_value = Value::Enum(quire_exact::EnumMember::new(right.variant(), 0));
        let mut index = EnumMemberIndex::default();
        index.record(left);
        index.record(right);
        probe!("ordered-enumeration-comparison-rejected", m, [], {
            m.enum_members = &index;
            m.order(
                OrderingOperator::Less,
                OrderedKind::Enums,
                &left_value,
                &right_value,
            )
            .map(|_| ())
        });
    }
    let payload = quire_exact::TextPayload::from_utf8(b"a").unwrap();
    let mut admission = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
    let mut text = |profile| {
        let kind = quire_exact::TextType::new(0, 1, profile).unwrap();
        let Outcome::Completed(text) = quire_exact::admit_text(&payload, &kind, &mut admission)
        else {
            panic!("text fixture admission");
        };
        Value::Text(text)
    };
    let left = text(quire_exact::TextProfile::UnicodeScalars);
    let right = text(quire_exact::TextProfile::BinaryUtf8);
    probe!("ordered-text-comparison-rejected", m, [], {
        m.order(OrderingOperator::Less, OrderedKind::Texts, &left, &right)
            .map(|_| ())
    });
}

#[test]
#[trace("TC-917", "FR-090-AC-16")]
fn real_required_reference_and_sequence_edges_have_the_uncharged_valid_prefix() {
    let receiver = reference();
    let reference_type = ValueType::Reference(receiver.object_type());
    let sequence_type =
        quire_exact::CollectionType::new(CollectionKind::Sequence, reference_type.clone(), None);
    let sequence = quire_exact::from_admitted(
        sequence_type.clone(),
        vec![Value::Reference(receiver.clone())],
    );
    let literal = body(|_| NodeKind::Literal(CheckedLiteral(Value::Boolean(true))));
    let field = FieldRef::new(receiver.object_type(), "edge");
    for (value_type, stored) in [
        (reference_type.clone(), Value::Reference(receiver.clone())),
        (ValueType::collection(sequence_type), sequence),
    ] {
        let package = object_package(value_type.clone(), quire_exact::Presence::Required);
        let graph = package.graph();
        let closure = quire_semantic_value::object_closure::ObjectClosure::new(
            graph.scope().types(),
            [(
                receiver.clone(),
                vec![("edge", FieldValue::Present(stored))],
            )],
            &[],
        )
        .unwrap();
        let objects = ObjectEnvironment::new(closure);
        let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let mut machine = Machine::new(
            graph.scope(),
            graph,
            &objects,
            &mut meter,
            graph.dispatch_tables(),
        );
        let targets = machine
            .edge_targets(literal.root(), &receiver, &field)
            .unwrap_or_else(|_| panic!("admitted required edge"));
        assert_eq!(targets, vec![receiver.clone()]);
        let attribute = CheckedBody::fixture(
            vec![reference_type.clone()],
            |ids| NodeKind::Attribute {
                reference: ids[0],
                field: field.clone(),
                optional: false,
                derefed: false,
            },
            value_type,
        );
        machine.values.push(Value::Reference(receiver.clone()));
        machine
            .apply(attribute.root())
            .unwrap_or_else(|_| panic!("admitted attribute read"));
        match machine.pop().unwrap_or_else(|_| panic!("attribute result")) {
            Value::Reference(value) => assert_eq!(value, receiver),
            Value::Collection(value) => {
                assert!(matches!(value.elements(), [Value::Reference(value)] if value == &receiver))
            }
            _ => panic!("attribute changed its admitted value kind"),
        }
        drop(machine);
        assert!(meter.admitted_charges().is_empty());
    }
}

#[test]
#[trace("TC-918", "FR-096-AC-19", "FR-090-AC-17")]
fn received_equality_source_cause_crosses_actual_public_call_and_evaluate_unchanged() {
    use super::super::{CallFailure, CheckedPackageEvaluation, QualifiedName};
    use qsl_forms::{BinaryOperator, BuiltinType, Expression, FunctionDeclaration, TypeForm};
    use qsl_semantics::check::PackageDeclarations;
    let span = qsl_foundation::Span { start: 0, end: 0 };
    let expected =
        ValueType::Int(IntegerInterval::new(Integer::zero(), Integer::from(9_i64)).unwrap());
    let base = object_package(expected, quire_exact::Presence::Required);
    let operand = || Expression::field(Expression::name("receiver"), "edge");
    let expression = Expression::binary(BinaryOperator::Equal, operand(), operand());
    let mut declarations = PackageDeclarations::new(
        qsl_semantics::check::fixture_source(),
        qsl_foundation::IdentityLimits::default(),
    );
    declarations.types = base.graph().scope().types().clone();
    declarations.functions.push(FunctionDeclaration::new(
        "Compare",
        vec![(
            "receiver".to_owned(),
            TypeForm::builtin(BuiltinType::Reference, span)
                .with_arguments(vec![TypeForm::name("Owner", span)]),
        )],
        TypeForm::builtin(BuiltinType::Boolean, span),
        None,
        expression.clone(),
    ));
    let package = qsl_package::CheckedPackage::link(
        declarations
            .check(quire_semantic_value::checking::CheckingLimits::default())
            .unwrap(),
    );
    let checked = package
        .graph()
        .check_expression(
            vec![(
                "receiver".to_owned(),
                ValueType::Reference(reference().object_type()),
            )],
            &expression,
            None,
            quire_semantic_value::checking::CheckMode::Kernel,
            quire_semantic_value::checking::CheckingLimits::default(),
        )
        .unwrap();
    // Admit real storage against Integer, then read it against the separately
    // checked Int[0,9] declaration. No fault/result/cause is injected.
    let storage = object_package(ValueType::Integer, quire_exact::Presence::Required);
    for stored in [-1_i64, 1_i64] {
        let closure = quire_semantic_value::object_closure::ObjectClosure::new(
            storage.graph().scope().types(),
            [(
                reference(),
                vec![(
                    "edge",
                    FieldValue::Present(Value::Integer(Integer::from(stored))),
                )],
            )],
            &[],
        )
        .unwrap();
        let objects = ObjectEnvironment::new(closure);
        for call in [false, true] {
            let mut input_meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
            let input = if call {
                package.call(
                    &QualifiedName::unqualified("Compare").unwrap(),
                    vec![Value::Boolean(true)],
                    &objects,
                    &mut input_meter,
                )
            } else {
                package.evaluate(
                    &checked,
                    vec![Value::Boolean(true)],
                    &objects,
                    &mut input_meter,
                )
            };
            assert!(matches!(
                input,
                Err(CallFailure::Input(
                    quire_semantic_value::call::InputRefusal::WrongValueKind { .. }
                ))
            ));
            assert!(input_meter.admitted_charges().is_empty());
            let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
            let arguments = vec![Value::Reference(reference())];
            let result = if call {
                package.call(
                    &QualifiedName::unqualified("Compare").unwrap(),
                    arguments,
                    &objects,
                    &mut meter,
                )
            } else {
                package.evaluate(&checked, arguments, &objects, &mut meter)
            };
            if stored < 0 {
                let Err(CallFailure::Fault(fault)) = result else {
                    panic!("received typed kernel fault");
                };
                assert_eq!(fault.stage(), "S6a");
                assert_eq!(fault.invariant(), "checked-program-invariant");
                assert_eq!(
                    fault.kernel_cause(),
                    Some(quire_exact::CheckedInvariantCause::EqualityOperandSourceNotAdmitted)
                );
                assert_eq!(fault.category(), qsl_foundation::Category::InternalFailure);
                let expected: &[ChargePoint] = if call {
                    &[ChargePoint::FunctionCall]
                } else {
                    &[]
                };
                assert_eq!(meter.admitted_charges(), expected);
                if call {
                    let mut denied = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED)
                        .with_injected_denial(quire_exact::InjectedDenial {
                            point: ChargePoint::FunctionCall,
                            occurrence: std::num::NonZeroU64::new(1).unwrap(),
                        });
                    let outcome = package
                        .call(
                            &QualifiedName::unqualified("Compare").unwrap(),
                            vec![Value::Reference(reference())],
                            &objects,
                            &mut denied,
                        )
                        .unwrap();
                    assert!(
                        matches!(outcome.outcome, FamilyOutcome::Evaluated(Outcome::Incomplete(record))
                        if record.charge_point == ChargePoint::FunctionCall)
                    );
                    assert!(denied.admitted_charges().is_empty());
                    for cause in [
                        quire_exact::CancelCause::Requested,
                        quire_exact::CancelCause::Deadline,
                    ] {
                        let cancel = quire_exact::Cancel::new();
                        cancel.cancel(cause);
                        let mut stopped = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED)
                            .with_cancel(cancel.clone());
                        let outcome = package
                            .call(
                                &QualifiedName::unqualified("Compare").unwrap(),
                                vec![Value::Reference(reference())],
                                &objects,
                                &mut stopped,
                            )
                            .unwrap();
                        assert!(
                            matches!(outcome.outcome, FamilyOutcome::Evaluated(Outcome::Incomplete(record))
                            if record.charge_point == ChargePoint::FunctionCall)
                        );
                        assert_eq!(cancel.tripped(), Some(cause));
                        assert!(stopped.admitted_charges().is_empty());
                    }
                }
            } else {
                assert!(matches!(
                    result.unwrap().outcome,
                    qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(
                        Value::Boolean(true)
                    ))
                ));
            }
        }
    }
}

#[test]
#[trace("TC-918", "FR-096-AC-19", "FR-090-AC-17")]
fn every_received_kernel_payload_survives_stop_and_public_fault_conversion() {
    let location = Location::root(Origin::Expression);
    for cause in qsl_semantics::check::received_kernel_causes() {
        let refusal = Refusal::CheckedInvariant { cause };
        assert_eq!(refusal.code(), None);
        assert_eq!(refusal.cause(), None);
        assert_eq!(kernel_refusal_record(&refusal, None), None);
        let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        charge_call(&mut meter).unwrap();
        let before = meter.admitted_charges().to_vec();
        for location in [None, Some(&location)] {
            let fault = Machine::stopped(Stop::Refused(refusal.clone()), location).unwrap_err();
            assert_eq!(fault.stage(), "S6a");
            assert_eq!(fault.invariant(), "checked-program-invariant");
            assert_eq!(fault.kernel_cause(), Some(cause));
            assert_eq!(fault.category(), qsl_foundation::Category::InternalFailure);
            assert_eq!(fault.catalog_code().code(), "runtime_invariant");
            // Both public entries use map_err(CallFailure::Fault) unchanged.
            let super::super::CallFailure::Fault(carried) = super::super::CallFailure::Fault(fault)
            else {
                panic!("public call/evaluate fault conversion changed category");
            };
            assert_eq!(carried, fault);
            assert_eq!(meter.admitted_charges(), before);
        }
    }
    let evaluation =
        Machine::stopped(Stop::Undefined(Undefined::NoneValue), Some(&location)).unwrap();
    assert!(matches!(
        evaluation.outcome,
        FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::NoneValue))
    ));
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn stack_extraction_and_frame_producers_report_their_own_invariants() {
    probe!("evaluation-value-stack-underflow", m, [], {
        m.pop().map(|_| ())
    });
    probe!("evaluation-value-stack-underflow", m, [], {
        m.pop_many(2).map(|_| ())
    });
    probe!("integer-value-expected", m, [], {
        m.values.push(Value::Boolean(true));
        m.pop_integer().map(|_| ())
    });
    probe!("boolean-value-expected", m, [], {
        m.values.push(Value::Integer(Integer::one()));
        m.pop_boolean().map(|_| ())
    });
    probe!("collection-value-expected", m, [], {
        m.values.push(Value::Boolean(true));
        m.pop_collection().map(|_| ())
    });
    probe!("rational-value-expected", m, [], {
        m.values.push(Value::Boolean(true));
        m.pop_rational().map(|_| ())
    });
    probe!("decimal-value-expected", m, [], {
        m.values.push(Value::Boolean(true));
        m.pop_decimal().map(|_| ())
    });
    for frame in [vec![], vec![vec![]]] {
        probe!("evaluation-local-slot-unresolved", m, [], {
            m.frames = frame;
            m.slot(0).map(|_| ())
        });
    }
    let local = body(|_| NodeKind::Local(0));
    probe!("evaluation-local-value-unbound", m, [], {
        m.frames.push(vec![None]);
        m.eval(local.root())
    });
    probe!("evaluation-return-frame-missing", m, [], {
        m.step(Task::Return)
    });
    let root = body(|_| NodeKind::Literal(CheckedLiteral(Value::Boolean(true))));
    probe!("evaluation-return-frame-missing", m, [], {
        m.values.push(Value::Boolean(true));
        m.step(Task::DispatchGuard(Box::new(DispatchGuard {
            body_function: 0,
            arguments: vec![],
            failure: PreconditionFailure {
                operation: "probe".to_owned(),
                selected: "probe".to_owned(),
                receiver: ObjectReference::new(
                    quire_exact::UniverseId::from_digest([0; 32]),
                    m.scope.types().object_types().next().unwrap().key(),
                    quire_exact::ObjectId::new("receiver").unwrap(),
                ),
            },
            node: root.root(),
        })))
    });
    probe!("evaluation-branch-node-expected", m, [], {
        m.step(Task::Branch(root.root()))
    });
    probe!("evaluation-connective-node-expected", m, [], {
        m.step(Task::Short(root.root()))
    });
    for root in [
        body(|_| NodeKind::Literal(CheckedLiteral(Value::Boolean(true)))),
        body(|_| NodeKind::Local(0)),
        body(|ids| NodeKind::Let {
            slot: 0,
            value: ids[0],
            body: ids[1],
        }),
        body(|ids| NodeKind::If {
            condition: ids[0],
            then: ids[1],
            otherwise: ids[1],
        }),
        body(|ids| NodeKind::Connective(Connective::And, ids[0], ids[1])),
        body(|ids| NodeKind::Pre(ids[0])),
    ] {
        probe!("evaluation-apply-node-invalid", m, [], {
            m.apply(root.root())
        });
    }
}

#[test]
#[trace("TC-917", "FR-090-AC-16")]
fn admitted_stack_values_bound_locals_and_branch_keep_the_uncharged_valid_prefix() {
    let (package, _) = super::tests::population_function_package();
    let graph = package.graph();
    let objects = ObjectEnvironment::default();
    let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
    let mut machine = Machine::new(
        graph.scope(),
        graph,
        &objects,
        &mut meter,
        graph.dispatch_tables(),
    );
    machine.values.push(Value::Boolean(true));
    assert!(matches!(
        machine.pop().unwrap_or_else(|_| panic!("pop")),
        Value::Boolean(true)
    ));
    machine.values = vec![
        Value::Boolean(false),
        Value::Integer(Integer::one()),
        Value::Boolean(true),
    ];
    let values = machine.pop_many(2).unwrap_or_else(|_| panic!("pop_many"));
    assert!(
        matches!(values.as_slice(), [Value::Integer(value), Value::Boolean(true)] if value == &Integer::one())
    );
    assert!(matches!(machine.values.as_slice(), [Value::Boolean(false)]));
    assert!(!machine
        .pop_boolean()
        .unwrap_or_else(|_| panic!("Boolean extraction")));
    machine.values.push(Value::Integer(Integer::one()));
    assert_eq!(
        machine
            .pop_integer()
            .unwrap_or_else(|_| panic!("Integer extraction")),
        Integer::one()
    );
    let rational = Rational::new(Integer::one(), Integer::from(2_i64)).unwrap();
    machine.values.push(Value::Rational(rational.clone()));
    assert_eq!(
        machine
            .pop_rational()
            .unwrap_or_else(|_| panic!("Rational extraction")),
        rational
    );
    let decimal = Decimal::new(Integer::from(125_i64), 2);
    machine.values.push(Value::Decimal(decimal.clone()));
    let extracted = machine
        .pop_decimal()
        .unwrap_or_else(|_| panic!("Decimal extraction"));
    assert_eq!(
        extracted.normalized().coefficient(),
        &Integer::from(125_i64)
    );
    assert_eq!(extracted.normalized().scale(), 2);
    let source = collection(vec![Value::Boolean(true)]);
    machine.values.push(Value::Collection(source.clone()));
    assert!(Arc::ptr_eq(
        &machine
            .pop_collection()
            .unwrap_or_else(|_| panic!("collection extraction")),
        &source
    ));
    machine.frames.push(vec![
        Some(Value::Boolean(true)),
        Some(Value::Boolean(false)),
    ]);
    assert!(matches!(
        machine.slot(0).unwrap_or_else(|_| panic!("bound slot")),
        Some(Value::Boolean(true))
    ));
    let local = body(|_| NodeKind::Local(0));
    machine
        .eval(local.root())
        .unwrap_or_else(|_| panic!("bound local"));
    assert!(matches!(
        machine.pop().unwrap_or_else(|_| panic!("local result")),
        Value::Boolean(true)
    ));
    let branch = body(|ids| NodeKind::If {
        condition: ids[0],
        then: ids[0],
        otherwise: ids[1],
    });
    machine.values.push(Value::Boolean(false));
    machine
        .step(Task::Branch(branch.root()))
        .unwrap_or_else(|_| panic!("checked branch"));
    finish_tasks(&mut machine).unwrap_or_else(|_| panic!("branch result"));
    assert!(matches!(
        machine.pop().unwrap_or_else(|_| panic!("branch pop")),
        Value::Boolean(false)
    ));
    machine
        .step(Task::Return)
        .unwrap_or_else(|_| panic!("existing return frame"));
    assert!(machine.frames.is_empty());
    assert!(machine.values.is_empty());
    let literal = body(|_| NodeKind::Literal(CheckedLiteral(Value::Boolean(true))));
    let result = machine.drive(literal.root()).unwrap();
    assert!(matches!(
        result.outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    drop(machine);
    assert!(meter.admitted_charges().is_empty());
}

#[test]
#[trace("TC-917", "FR-090-AC-16")]
fn valid_required_and_optional_projection_preserves_values_without_a_charge() {
    let slot = FieldValue::Present(Value::Boolean(true));
    let required = Machine::project(&slot, false, &ValueType::Boolean, false)
        .unwrap_or_else(|_| panic!("required projection"));
    assert!(matches!(required, Value::Boolean(true)));
    let optional = Machine::project(&slot, true, &ValueType::option(ValueType::Boolean), false)
        .unwrap_or_else(|_| panic!("optional projection"));
    assert!(
        matches!(optional, Value::Option(value) if matches!(value.payload(), Some(Value::Boolean(true))))
    );
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn application_shapes_and_constructor_rejections_reach_their_detecting_sites() {
    let bad_key = quire_exact::NodeKey::from_digest([0xfe; 32]);
    let target = DecimalType::new(
        Integer::from(-10_i64),
        Integer::from(10_i64),
        0,
        2,
        quire_exact::RoundingMode::Exact,
    )
    .unwrap();
    for (root, values, expected) in [
        (
            body(|ids| NodeKind::ConvertDecimal(ids[0], target.clone())),
            vec![Value::Boolean(true)],
            "decimal-conversion-operand-invalid",
        ),
        (
            body(|ids| {
                NodeKind::Ieee(
                    ArithmeticOperator::Add,
                    quire_exact::RoundingMode::Exact,
                    ids[0],
                    ids[1],
                )
            }),
            vec![Value::Boolean(true), Value::Boolean(true)],
            "ieee-float-value-expected",
        ),
        (
            body(|ids| NodeKind::Quantity(ArithmeticOperator::Add, ids[0], ids[1])),
            vec![Value::Boolean(true), Value::Boolean(true)],
            "quantity-value-expected",
        ),
        (
            body(|ids| NodeKind::Field {
                operand: ids[0],
                index: 0,
                optional: false,
            }),
            vec![Value::Boolean(true)],
            "field-composite-value-expected",
        ),
        (
            body(|ids| NodeKind::Present(ids[0])),
            vec![Value::Boolean(true)],
            "option-value-expected",
        ),
        (
            body(|ids| NodeKind::Value(ids[0])),
            vec![Value::Boolean(true)],
            "option-value-expected",
        ),
        (
            body(|_| NodeKind::Call {
                function: usize::MAX,
                arguments: vec![],
            }),
            vec![],
            "evaluation-callable-unresolved",
        ),
        (
            body(|ids| NodeKind::Tuple {
                declaration: bad_key,
                arguments: vec![ids[0]],
            }),
            vec![Value::Boolean(true)],
            "tuple-construction-rejected",
        ),
        (
            body(|_| NodeKind::Record {
                declaration: bad_key,
                slots: vec![],
            }),
            vec![],
            "record-declaration-unresolved",
        ),
        (
            body(|ids| NodeKind::Flatten(ids[0])),
            vec![],
            "flatten-collection-result-expected",
        ),
        (
            body(|ids| NodeKind::AllInstances { population: ids[0] }),
            vec![Value::Boolean(true)],
            "query-population-value-expected",
        ),
        (
            body(|ids| NodeKind::Lookup {
                population: ids[0],
                reference: ids[1],
                absence: qsl_foundation::absence::AbsenceMode::Empty,
            }),
            vec![Value::Boolean(true), Value::Boolean(true)],
            "query-population-value-expected",
        ),
        (
            body(|ids| NodeKind::Dispatch {
                receiver: ids[0],
                operation: 0,
                table: 0,
                arguments: vec![],
            }),
            vec![Value::Boolean(true)],
            "dispatch-reference-value-expected",
        ),
    ] {
        probe!(expected, m, [], {
            m.values = values;
            m.apply(root.root())
        });
    }
    let root = CheckedBody::fixture(
        vec![ValueType::Boolean],
        |ids| NodeKind::Flatten(ids[0]),
        ValueType::collection(quire_exact::CollectionType::new(
            CollectionKind::Sequence,
            ValueType::Boolean,
            None,
        )),
    );
    probe!(
        "flatten-inner-collection-expected",
        m,
        [ChargePoint::CollectionVisit],
        {
            m.values
                .push(Value::Collection(collection(vec![Value::Boolean(true)])));
            m.apply(root.root())
        }
    );
    let (package, _) = super::tests::population_function_package();
    earlier_charge_controls(
        &package,
        &ObjectEnvironment::default(),
        ChargePoint::CollectionVisit,
        |m| {
            m.values
                .push(Value::Collection(collection(vec![Value::Boolean(true)])));
            m.apply(root.root())
        },
    );
    let root = body(|ids| NodeKind::AllInstances { population: ids[0] });
    probe!("query-population-type-expected", m, [], {
        // Use a genuine binding identity; failure precedes population resolution.
        let binding = super::tests::population_binding(Some(3));
        m.values.push(Value::Population(binding.population_id()));
        m.apply(root.root())
    });
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn projection_and_ordering_shape_faults_keep_their_original_channels() {
    let slot = FieldValue::Present(Value::Integer(Integer::one()));
    probe!("field-option-payload-rejected", m, [], {
        let _ = &mut m;
        Machine::project(&slot, true, &ValueType::option(ValueType::Boolean), false).map(|_| ())
    });
    for slot in [FieldValue::Absent, FieldValue::Null] {
        probe!("field-projection-shape-invalid", m, [], {
            let _ = &mut m;
            Machine::project(&slot, false, &ValueType::Boolean, false).map(|_| ())
        });
        let control = Machine::project(&slot, true, &ValueType::option(ValueType::Boolean), false);
        assert!(matches!(control, Ok(Value::Option(option)) if option.payload().is_none()));
    }
    probe!("ordered-operand-kind-invalid", m, [], {
        m.order(
            OrderingOperator::Less,
            OrderedKind::Integers,
            &Value::Boolean(true),
            &Value::Boolean(false),
        )
        .map(|_| ())
    });
}

#[test]
#[trace("TC-917", "FR-090-AC-15", "FR-090-AC-16")]
fn iteration_resume_finish_and_stop_reporting_reach_distinct_producers() {
    let literal = body(|_| NodeKind::Literal(CheckedLiteral(Value::Boolean(true))));
    probe!("iteration-node-invalid", m, [], {
        m.start_iteration(literal.root())
    });
    probe!("iteration-node-invalid", m, [], {
        let mut i = iteration(literal.root());
        i.awaiting = true;
        i.next = 1;
        m.values.push(Value::Boolean(true));
        m.iterate(i)
    });
    probe!(
        "iteration-node-invalid",
        m,
        [ChargePoint::CollectionVisit],
        { m.iterate(iteration(literal.root())) }
    );
    let (package, _) = super::tests::population_function_package();
    earlier_charge_controls(
        &package,
        &ObjectEnvironment::default(),
        ChargePoint::CollectionVisit,
        |m| m.iterate(iteration(literal.root())),
    );
    probe!("iteration-node-invalid", m, [], {
        m.finish(*iteration(literal.root()))
    });
    probe!("query-stop-node-expected", m, [], {
        claim_level(&mut m);
        m.note_stop(&iteration(literal.root()))
    });
    for visit in [
        Visit::Filter,
        Visit::Forall,
        Visit::Exists,
        Visit::Count,
        Visit::Sum,
    ] {
        let root = body(|ids| NodeKind::Query {
            visit,
            slot: 0,
            source: ids[0],
            body: ids[1],
        });
        let wrong = if visit == Visit::Sum {
            Value::Boolean(true)
        } else {
            Value::Integer(Integer::one())
        };
        probe!("query-body-value-invalid", m, [], {
            let mut i = iteration(root.root());
            i.awaiting = true;
            i.next = 1;
            m.values.push(wrong);
            m.iterate(i)
        });
    }
    let sum = body(|ids| NodeKind::Query {
        visit: Visit::Sum,
        slot: 0,
        source: ids[0],
        body: ids[1],
    });
    probe!("sum-accumulator-value-invalid", m, [], {
        let mut i = iteration(sum.root());
        i.awaiting = true;
        i.next = 1;
        i.accumulator = Some(Value::Boolean(true));
        m.values.push(Value::Integer(Integer::one()));
        m.iterate(i)
    });
    probe!("sum-accumulator-value-invalid", m, [], {
        let mut i = iteration(sum.root());
        i.accumulator = Some(Value::Boolean(true));
        m.finish(*i)
    });
    let fold = body(|ids| NodeKind::Fold {
        accumulator: 0,
        binder: 1,
        source: ids[0],
        step: ids[1],
        identity: Some(ids[0]),
    });
    probe!(
        "fold-accumulator-missing",
        m,
        [ChargePoint::CollectionVisit],
        { m.iterate(iteration(fold.root())) }
    );
    earlier_charge_controls(
        &package,
        &ObjectEnvironment::default(),
        ChargePoint::CollectionVisit,
        |m| m.iterate(iteration(fold.root())),
    );
    probe!("fold-accumulator-missing", m, [], {
        m.finish(*iteration(fold.root()))
    });
    let query = body(|ids| NodeKind::Query {
        visit: Visit::Map,
        slot: 0,
        source: ids[0],
        body: ids[1],
    });
    for next in [0, 2] {
        probe!("iteration-source-position-invalid", m, [], {
            let mut i = iteration(query.root());
            i.awaiting = true;
            i.next = next;
            m.values.push(Value::Boolean(true));
            m.iterate(i)
        });
        probe!("query-stop-position-invalid", m, [], {
            claim_level(&mut m);
            let mut i = iteration(query.root());
            i.next = next;
            m.note_stop(&i)
        });
    }
    for visit in [Visit::Map, Visit::Filter] {
        let root = body(|ids| NodeKind::Query {
            visit,
            slot: 0,
            source: ids[0],
            body: ids[1],
        });
        probe!("query-collection-result-expected", m, [], {
            m.finish(*iteration(root.root()))
        });
    }
}

#[test]
#[trace("TC-917", "FR-090-AC-16")]
fn scalar_query_resume_retains_the_valid_result_and_preserves_earlier_stops() {
    let (package, _) = super::tests::population_function_package();
    let graph = package.graph();
    let objects = ObjectEnvironment::default();
    for (visit, result) in [
        (Visit::Forall, Value::Boolean(false)),
        (Visit::Exists, Value::Boolean(true)),
        (Visit::Count, Value::Boolean(true)),
        (Visit::Sum, Value::Integer(Integer::one())),
    ] {
        let source_type = ValueType::collection(quire_exact::CollectionType::new(
            CollectionKind::Sequence,
            ValueType::Boolean,
            None,
        ));
        let result_type = match visit {
            Visit::Count | Visit::Sum => ValueType::Integer,
            _ => ValueType::Boolean,
        };
        let operand_type = if visit == Visit::Sum {
            ValueType::Integer
        } else {
            ValueType::Boolean
        };
        let root = CheckedBody::fixture(
            vec![source_type, operand_type],
            |ids| NodeKind::Query {
                visit,
                slot: 0,
                source: ids[0],
                body: ids[1],
            },
            result_type,
        );
        let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let mut machine = Machine::new(
            graph.scope(),
            graph,
            &objects,
            &mut meter,
            graph.dispatch_tables(),
        );
        let mut pending = iteration(root.root());
        pending.awaiting = true;
        pending.next = 1;
        machine.iterations = 1;
        machine.values.push(result.clone());
        machine
            .iterate(pending)
            .unwrap_or_else(|_| panic!("valid scalar query resume"));
        assert_eq!(machine.iterations, 0);
        assert!(machine.tasks.is_empty());
        match visit {
            Visit::Forall => assert!(matches!(machine.values.as_slice(), [Value::Boolean(false)])),
            Visit::Exists => assert!(matches!(machine.values.as_slice(), [Value::Boolean(true)])),
            Visit::Count | Visit::Sum => assert!(matches!(machine.values.as_slice(),
                [Value::Integer(value)] if value == &Integer::one())),
            _ => panic!("unselected query visit"),
        }
        drop(machine);
        assert_eq!(
            meter.admitted_charges(),
            [ChargePoint::CollectionResultRetain]
        );
        earlier_charge_controls(
            &package,
            &objects,
            ChargePoint::CollectionResultRetain,
            |machine| {
                let mut pending = iteration(root.root());
                pending.awaiting = true;
                pending.next = 1;
                machine.iterations = 1;
                machine.values.push(result.clone());
                machine.iterate(pending)
            },
        );
    }
}

#[test]
#[trace("TC-917", "FR-090-AC-16")]
fn fold_resume_and_finish_retain_the_real_accumulator_and_earlier_stop() {
    fn run<'a>(
        machine: &mut Machine<'a, '_>,
        node: CheckedNode<'a>,
        resume: bool,
    ) -> Result<(), Halt> {
        let mut pending = iteration(node);
        pending.next = 1;
        pending.accumulator = Some(Value::Boolean(!resume));
        machine.iterations = 1;
        if resume {
            pending.awaiting = true;
            machine.values.push(Value::Boolean(true));
            machine.iterate(pending)
        } else {
            machine.finish(*pending)
        }
    }
    let (package, _) = super::tests::population_function_package();
    let graph = package.graph();
    let objects = ObjectEnvironment::default();
    let source_type = ValueType::collection(quire_exact::CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Boolean,
        None,
    ));
    let root = CheckedBody::fixture(
        vec![source_type, ValueType::Boolean, ValueType::Boolean],
        |ids| NodeKind::Fold {
            accumulator: 0,
            binder: 1,
            source: ids[0],
            step: ids[1],
            identity: Some(ids[2]),
        },
        ValueType::Boolean,
    );
    for resume in [false, true] {
        let mut meter = Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED);
        let mut machine = Machine::new(
            graph.scope(),
            graph,
            &objects,
            &mut meter,
            graph.dispatch_tables(),
        );
        run(&mut machine, root.root(), resume).unwrap_or_else(|_| panic!("valid fold accumulator"));
        assert!(matches!(machine.values.as_slice(), [Value::Boolean(true)]));
        assert_eq!(machine.iterations, 0);
        assert!(machine.tasks.is_empty());
        drop(machine);
        assert_eq!(
            meter.admitted_charges(),
            [ChargePoint::CollectionResultRetain]
        );
        assert_eq!(meter.consumed(quire_exact::LimitKind::ResultUnits), 1);
        earlier_charge_controls(
            &package,
            &objects,
            ChargePoint::CollectionResultRetain,
            |machine| run(machine, root.root(), resume),
        );
    }
}

#[test]
#[trace("TC-917", "FR-090-AC-18")]
fn record_present_value_branch_has_a_local_count_and_zip_proof() {
    let source: String = include_str!("../evaluate.rs")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let record = source
        .split("NodeKind::Record{declaration,slots}=>")
        .nth(1)
        .expect("record detecting producer")
        .split("NodeKind::Collection{")
        .next()
        .unwrap();
    assert!(record.contains(".filter(|slot|matches!(slot,RecordSlot::Present(_)))"));
    assert!(record.contains(".count()"));
    assert!(record.contains("self.pop_many(present)?.into_iter()"));
    assert!(record.contains("slots.iter().zip(declared)"));
    assert!(
        record.contains("values.next().ok_or_else(||invariant(\"record-present-value-missing\"))")
    );
    let pop_many = source
        .split("fnpop_many")
        .nth(1)
        .unwrap()
        .split("fnpop_integer")
        .next()
        .unwrap();
    assert!(pop_many.contains("checked_sub(count)"));
    assert!(pop_many.contains("self.values.split_off(start)"));
    // pop_many supplies exactly present values or stops earlier; zip visits
    // only a prefix of slots and consumes one value per Present slot.
    // Thus the defensive values.next() failure has no executable fixture.
}
