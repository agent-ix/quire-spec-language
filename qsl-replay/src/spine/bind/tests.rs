// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1053: FR-367's binding of a call's named values to a scalar
//! application occurrence.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_contract_model::CheckedOccurrence;
use quire_exact::{Cancel, Integer, IntegerInterval, Origin, Role, Value};

use super::*;
use crate::execute::scalar_site::{check_operands, equality_operator};
use crate::scalar::{ScalarOperand, ScalarOperation};
use crate::spine::call::tests::{lookup_driver_context, lookup_driver_fixture};
use crate::spine::{
    check, default_accounting, package, parse, run, select as select_models, CallArgument,
    CallOutcome, DependencyInput, LockEvidence, ParseRequest, SpineLimits,
};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"quire.value.complete/v1\";\n";

/// One compiled and emitted unit, as `run` holds it after its package stage.
struct Unit {
    source: String,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
    models: AdmittedModels,
    checked: CheckedUnit,
    emitted: EmittedUnit,
}

fn identity() -> qsl_foundation::SourceIdentity {
    qsl_foundation::SourceIdentity::new("agent-ix", "bind-fixture", "git", "1")
}

fn compile_source(source: String, packages: BTreeMap<[u8; 32], Vec<u8>>) -> Unit {
    let cancel = Cancel::new();
    let identity = identity();
    let limits = SpineLimits::default();
    let parsed = parse(
        &ParseRequest {
            source: &identity,
            path: "bind.native",
            bytes: source.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("the unit parses")
    .into_value();
    let models = select_models(&parsed, &packages, limits.model, &cancel)
        .expect("the models select")
        .into_value();
    let checked = check(
        &parsed,
        &models,
        &DependencyInput::default(),
        &LockEvidence::default(),
        limits,
        &cancel,
    )
    .expect("the unit checks")
    .into_value();
    let emitted = package(&checked, Default::default(), &cancel)
        .expect("the unit emits")
        .into_value();
    Unit {
        source,
        packages,
        models,
        checked,
        emitted,
    }
}

fn compile(functions: &str) -> Unit {
    compile_source(format!("{HEADER}{functions}\n"), BTreeMap::new())
}

fn read(unit: &Unit) -> BindPackage<'_> {
    BindPackage::read(&unit.models, &unit.checked, &unit.emitted).expect("the package reads")
}

fn arg(parameter: &str, value: i64) -> CallArgument {
    CallArgument {
        parameter: parameter.to_owned(),
        value: Value::Integer(Integer::from(value)),
    }
}

fn call(function: &str, arguments: Vec<CallArgument>) -> Call {
    Call {
        function: function.to_owned(),
        arguments,
        accounting: default_accounting(1_000_000),
        supplied: Default::default(),
    }
}

fn bind_with(
    unit: &Unit,
    call: &Call,
    occurrence: &ScalarOccurrence,
    objects: &ObjectEnvironment,
) -> Result<BindOutcome, Box<RunRefusal>> {
    bind_scalar_operands(&read(unit), call, occurrence, objects, &Cancel::new())
}

fn bind_call(
    unit: &Unit,
    call: &Call,
    occurrence: &ScalarOccurrence,
) -> Result<BindOutcome, Box<RunRefusal>> {
    bind_with(unit, call, occurrence, &ObjectEnvironment::default())
}

/// The call bound, or a test failure naming what else came back.
fn bound(unit: &Unit, call: &Call, occurrence: &ScalarOccurrence) -> BoundScalarOperands {
    match bind_call(unit, call, occurrence) {
        Ok(BindOutcome::Bound(bound)) => bound,
        other => panic!("expected a binding, got {other:?}"),
    }
}

fn refused(unit: &Unit, call: &Call, occurrence: &ScalarOccurrence) -> BindRefusal {
    match bind_call(unit, call, occurrence) {
        Ok(BindOutcome::Refused(refusal)) => refusal,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn wire(key: quire_exact::NodeKey) -> WireNodeId {
    WireNodeId::from_digest(*key.as_bytes())
}

/// Every occurrence in `function`'s body of a node `keep` accepts.
fn occurrences_in(
    unit: &Unit,
    function: &str,
    keep: impl Fn(&qsl_semantics::check::SemanticNode) -> bool,
) -> Vec<ScalarOccurrence> {
    let graph = unit.checked.package().graph();
    graph
        .occurrences()
        .filter(|(_, _, location)| {
            matches!(&location.origin, BodyOrigin::Body { function: f, .. } if f == function)
        })
        .filter(|(key, _, _)| {
            graph
                .semantic_graph()
                .node(*key)
                .is_some_and(&keep)
        })
        .map(|(key, origin, _)| ScalarOccurrence {
            function: function.to_owned(),
            application: wire(key),
            occurrence: OccurrenceKey::new(wire(key), origin),
        })
        .collect()
}

/// The one occurrence of `operator` in `function`'s body.
fn the_application(unit: &Unit, function: &str, operator: ScalarOperator) -> ScalarOccurrence {
    let mut found = occurrences_in(unit, function, |node| applies(node, operator));
    assert_eq!(found.len(), 1, "one {operator:?} in {function}");
    found.remove(0)
}

fn parameter_node(unit: &Unit, function: &str, name: &str) -> WireNodeId {
    let package = unit.checked.package();
    let callable = package.graph().callable(function).expect("declared");
    let keys = callable_parameter_keys(package, &callable).expect("parameter nodes");
    let position = callable
        .parameters
        .iter()
        .position(|(parameter, _)| parameter == name)
        .expect("declared parameter");
    wire(keys[position])
}

fn bound_range(lower: i128, upper: i128) -> CheckedScalarOperandRange {
    CheckedScalarOperandRange { lower, upper }
}

/// The source of the argument at `index` for parameter `parameter`: a
/// parameter's level is its declared position.
fn argument_source(index: usize, parameter: &str, node: WireNodeId) -> OperandSource {
    OperandSource::Argument {
        argument: index,
        parameter: parameter.to_owned(),
        node,
        level: if parameter == "b" { "1" } else { "0" }.to_owned(),
    }
}

const SUB: &str = "function sub using v(a: Int[0, 9], b: Int[0, 9]): Int[-9, 9] pure { a - b }";

/// What `run` returns for `call` over `unit`'s source, as the refusal.
fn run_refusal(unit: &Unit, call: &Call, objects: &ObjectEnvironment) -> Box<RunRefusal> {
    run(
        identity(),
        "bind.native",
        unit.source.as_bytes(),
        &unit.packages,
        &DependencyInput::default(),
        SpineLimits::default(),
        call,
        objects,
        &Cancel::new(),
    )
    .map(|(_, outcome)| outcome)
    .expect_err("run refuses the call")
}

/// FR-367-AC-1: the operand order is the application's, never the call's
/// argument order.
#[trace("TC-1053", "FR-367-AC-1")]
#[test]
fn tc_1053_step_1_operands_bind_in_application_order() {
    let unit = compile(SUB);
    let occurrence = the_application(&unit, "sub", ScalarOperator::Subtract);
    let (a, b) = (
        parameter_node(&unit, "sub", "a"),
        parameter_node(&unit, "sub", "b"),
    );

    let reversed = bound(
        &unit,
        &call("sub", vec![arg("b", 2), arg("a", 7)]),
        &occurrence,
    );
    assert_eq!(reversed.values(), [7, 2]);
    assert_eq!(
        reversed.operands(),
        [
            BoundOperand {
                ordinal: 0,
                value: 7,
                range: bound_range(0, 9),
                source: argument_source(1, "a", a),
            },
            BoundOperand {
                ordinal: 1,
                value: 2,
                range: bound_range(0, 9),
                source: argument_source(0, "b", b),
            },
        ]
    );
    assert_eq!(reversed.occurrence(), &occurrence);
    assert_eq!(reversed.package_id(), unit.emitted.package().package_id());

    let listed = bound(
        &unit,
        &call("sub", vec![arg("a", 7), arg("b", 2)]),
        &occurrence,
    );
    assert_eq!(listed.values(), [7, 2]);
    assert_eq!(
        listed
            .operands()
            .iter()
            .map(|o| o.source.clone())
            .collect::<Vec<_>>(),
        [argument_source(0, "a", a), argument_source(1, "b", b)]
    );

    let swapped =
        compile("function sub using v(a: Int[0, 9], b: Int[0, 9]): Int[-9, 9] pure { b - a }");
    let occurrence = the_application(&swapped, "sub", ScalarOperator::Subtract);
    let swapped = bound(
        &swapped,
        &call("sub", vec![arg("b", 2), arg("a", 7)]),
        &occurrence,
    );
    assert_eq!(swapped.values(), [2, 7]);
}

/// FR-367-AC-2: literals, repeated parameters and unary operators.
#[trace("TC-1053", "FR-367-AC-2")]
#[test]
fn tc_1053_step_2_literals_repeats_and_unary_operands() {
    let unit = compile(
        "function plus using v(a: Int[0, 9]): Int[1, 10] pure { a + 1 }\n\
         function lead using v(a: Int[0, 9]): Int[1, 10] pure { 1 + a }\n\
         function twice using v(a: Int[0, 9]): Int[-9, 9] pure { a - a }\n\
         function neg using v(a: Int[0, 9]): Int[-9, 0] pure { -a }",
    );
    let four = || vec![arg("a", 4)];

    let occurrence = the_application(&unit, "plus", ScalarOperator::Add);
    let plus = bound(&unit, &call("plus", four()), &occurrence);
    assert_eq!(plus.values(), [4, 1]);
    assert!(matches!(
        plus.operands()[0].source,
        OperandSource::Argument { argument: 0, .. }
    ));
    let literal = &plus.operands()[1];
    assert_eq!(literal.range, bound_range(1, 1));
    let OperandSource::InlineLiteral { node: Some(node) } = literal.source else {
        panic!("a source literal is a graph node, got {:?}", literal.source);
    };
    let graph = unit.checked.package().graph().semantic_graph();
    let literal_node = graph.node(graph.resolve_wire(node).unwrap()).unwrap();
    assert_eq!(literal_node.semantic_form(), "literal");

    let occurrence = the_application(&unit, "lead", ScalarOperator::Add);
    assert_eq!(
        bound(&unit, &call("lead", four()), &occurrence).values(),
        [1, 4]
    );

    let occurrence = the_application(&unit, "twice", ScalarOperator::Subtract);
    let twice = bound(&unit, &call("twice", four()), &occurrence);
    let a = parameter_node(&unit, "twice", "a");
    assert_eq!(twice.values(), [4, 4]);
    assert_eq!(
        twice
            .operands()
            .iter()
            .map(|o| (o.ordinal, o.source.clone()))
            .collect::<Vec<_>>(),
        [
            (0, argument_source(0, "a", a)),
            (1, argument_source(0, "a", a))
        ]
    );

    let occurrence = the_application(&unit, "neg", ScalarOperator::Negate);
    let neg = bound(&unit, &call("neg", four()), &occurrence);
    assert_eq!(neg.values(), [4]);
    assert_eq!(neg.operands().len(), 1);
}

/// FR-367-AC-2: a Contract IR inline literal term binds with no node. Source
/// compilation never yields one, so the operand list is Contract IR's shape
/// built by hand and resolved against a compiled unit.
#[trace("TC-1053", "FR-367-AC-2")]
#[test]
fn tc_1053_step_2_inline_literal_term_binds_without_a_node() {
    let unit = compile("function plus using v(a: Int[0, 9]): Int[1, 10] pure { a + 1 }");
    let package = read(&unit);
    let occurrence = the_application(&unit, "plus", ScalarOperator::Add);
    let selected = select(unit.checked.package(), "plus").unwrap();
    let arguments = [Value::Integer(Integer::from(4_i64))];
    let call = call("plus", vec![arg("a", 4)]);
    let resolver = Resolver::new(&package, &call, selected.parameters, &arguments).unwrap();
    let operands = [CheckedScalarOperand {
        ordinal: 0,
        child: CheckedScalarOperandChild::InlineLiteral {
            application: checked_node_id(occurrence.application),
            occurrence: CheckedOccurrence {
                role: quire_contract_model::CheckedOccurrenceRole::Expression,
                ordinal: 0,
            },
            ordinal: 0,
        },
        range: bound_range(7, 7),
    }];
    let Ok(bound) = resolver.bind(&operands) else {
        panic!("an inline literal term binds");
    };
    assert_eq!(
        bound,
        [BoundOperand {
            ordinal: 0,
            value: 7,
            range: bound_range(7, 7),
            source: OperandSource::InlineLiteral { node: None },
        }]
    );
}

/// FR-367-AC-3: binding refusals are `run`'s own, and carry no operand.
#[trace("TC-1053", "FR-367-AC-3")]
#[test]
fn tc_1053_step_3_binding_refusals_equal_runs() {
    let unit = compile(SUB);
    let occurrence = the_application(&unit, "sub", ScalarOperator::Subtract);
    let objects = ObjectEnvironment::default();

    let missing = call("sub", vec![arg("a", 7)]);
    let refusal = bind_call(&unit, &missing, &occurrence).unwrap_err();
    assert!(matches!(&*refusal, RunRefusal::UnboundParameter { parameter } if parameter == "b"));
    assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
    assert_eq!(
        format!("{refusal:?}"),
        format!("{:?}", run_refusal(&unit, &missing, &objects))
    );

    let extra = call("sub", vec![arg("a", 7), arg("b", 2), arg("c", 1)]);
    let refusal = bind_call(&unit, &extra, &occurrence).unwrap_err();
    assert!(matches!(&*refusal, RunRefusal::UnknownParameter { parameter } if parameter == "c"));
    assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
    assert_eq!(
        format!("{refusal:?}"),
        format!("{:?}", run_refusal(&unit, &extra, &objects))
    );

    let twice = call("sub", vec![arg("a", 7), arg("a", 6), arg("b", 2)]);
    let refusal = bind_call(&unit, &twice, &occurrence).unwrap_err();
    assert!(matches!(&*refusal, RunRefusal::DuplicateArgument { parameter } if parameter == "a"));
    assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
    assert_eq!(
        format!("{refusal:?}"),
        format!("{:?}", run_refusal(&unit, &twice, &objects))
    );
}

/// FR-367-AC-4: a mistyped value, and FR-100's order among refusals; no
/// occurrence refusal precedes an admission refusal.
#[trace("TC-1053", "FR-367-AC-4")]
#[test]
fn tc_1053_step_3_mistyped_arguments_equal_runs() {
    let unit = compile(SUB);
    let occurrence = the_application(&unit, "sub", ScalarOperator::Subtract);
    let objects = ObjectEnvironment::default();
    let boolean = CallArgument {
        parameter: "a".to_owned(),
        value: Value::Boolean(true),
    };
    let cases = [
        call("sub", vec![boolean.clone(), arg("b", 2)]),
        call("sub", vec![arg("a", 10), arg("b", 2)]),
    ];
    for case in &cases {
        let refusal = bind_call(&unit, case, &occurrence).unwrap_err();
        assert!(
            matches!(*refusal, RunRefusal::WrongValueKind { position: 0 }),
            "{refusal:?}"
        );
        assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
        assert_eq!(
            format!("{refusal:?}"),
            format!("{:?}", run_refusal(&unit, case, &objects))
        );
    }

    let both = call("sub", vec![boolean]);
    let refusal = bind_call(&unit, &both, &occurrence).unwrap_err();
    assert!(matches!(&*refusal, RunRefusal::UnboundParameter { parameter } if parameter == "b"));

    // An occurrence refusal never precedes an admission refusal.
    let mut stale = occurrence.clone();
    stale.function = "other".to_owned();
    let refusal = bind_call(&unit, &cases[1], &stale).unwrap_err();
    assert!(matches!(
        *refusal,
        RunRefusal::WrongValueKind { position: 0 }
    ));
}

/// FR-367-AC-4: a reference of a foreign universe, and one absent from the
/// complete population, keep `run`'s model-aware refusals.
#[trace("TC-1053", "FR-367-AC-4")]
#[test]
fn tc_1053_step_3_reference_admission_equals_runs() {
    let fixture = lookup_driver_fixture(
        "function mk using v(r: M::Gadget, a: Int[0, 9]): Int[1, 10] pure { a + 1 }",
    );
    let unit = compile_source(fixture.source.clone(), fixture.packages.clone());
    let (objects, _, present, _) = lookup_driver_context(&fixture);
    let occurrence = the_application(&unit, "mk", ScalarOperator::Add);
    let reference = |reference: quire_exact::ObjectReference| {
        call(
            "mk",
            vec![
                CallArgument {
                    parameter: "r".to_owned(),
                    value: Value::Reference(reference),
                },
                arg("a", 4),
            ],
        )
    };

    let good = call_with_reference(&unit, &objects, &occurrence, reference(present.clone()));
    assert_eq!(good.values(), [4, 1]);

    let foreign = quire_exact::ObjectReference::new(
        quire_exact::UniverseId::from_digest([0xa5; 32]),
        present.object_type(),
        present.object().clone(),
    );
    let foreign_call = reference(foreign);
    let refusal = bind_with(&unit, &foreign_call, &occurrence, &objects).unwrap_err();
    assert!(
        matches!(&*refusal, RunRefusal::Admission { position: 0, refusal }
        if matches!(refusal.cause, ModelRefusalCause::ForeignUniverse { .. }))
    );
    assert_eq!(refusal.code(), Code::ForeignReference);
    assert_eq!(
        format!("{refusal:?}"),
        format!("{:?}", run_refusal(&unit, &foreign_call, &objects))
    );

    let dangling = quire_exact::ObjectReference::new(
        present.universe(),
        present.object_type(),
        quire_exact::ObjectId::new("absent-from-the-population").unwrap(),
    );
    let dangling_call = reference(dangling);
    let refusal = bind_with(&unit, &dangling_call, &occurrence, &objects).unwrap_err();
    assert!(matches!(
        *refusal,
        RunRefusal::DanglingReference { position: 0 }
    ));
    assert_eq!(refusal.code(), Code::DanglingReference);
    assert_eq!(
        format!("{refusal:?}"),
        format!("{:?}", run_refusal(&unit, &dangling_call, &objects))
    );
}

fn call_with_reference(
    unit: &Unit,
    objects: &ObjectEnvironment,
    occurrence: &ScalarOccurrence,
    call: Call,
) -> BoundScalarOperands {
    match bind_with(unit, &call, occurrence, objects) {
        Ok(BindOutcome::Bound(bound)) => bound,
        other => panic!("expected a binding, got {other:?}"),
    }
}

use qsl_semantics::model::refusal::ModelRefusalCause;

/// FR-367-AC-5: each way the occurrence is not the claim's.
#[trace("TC-1053", "FR-367-AC-5")]
#[test]
fn tc_1053_step_4_occurrence_identity_refusals() {
    let unit = compile(&format!(
        "{SUB}\n\
         function other using v(a: Int[0, 9], b: Int[0, 9]): Int[0, 81] pure {{ a * b }}\n\
         function eq using v(a: Int[0, 9], b: Int[0, 9]): Boolean pure {{ a = b }}"
    ));
    let values = || call("sub", vec![arg("a", 7), arg("b", 2)]);
    let sub = the_application(&unit, "sub", ScalarOperator::Subtract);
    let node = sub.application;

    // An occurrence key the node lacks, while the node has another.
    let mut stale = sub.clone();
    stale.occurrence = OccurrenceKey::new(
        node,
        Origin::new(Role::new(sub.occurrence.origin().role().as_str()), 99),
    );
    assert_eq!(
        refused(&unit, &values(), &stale),
        BindRefusal::ScalarIdentity(BindIdentityCause::Occurrence {
            occurrence: stale.occurrence.clone()
        })
    );
    assert_eq!(bound(&unit, &values(), &sub).values(), [7, 2]);

    // The node of another function, which shares sub's parameter nodes.
    let mut foreign = the_application(&unit, "other", ScalarOperator::Multiply);
    foreign.function = "sub".to_owned();
    assert_eq!(
        refused(&unit, &values(), &foreign),
        BindRefusal::ScalarIdentity(BindIdentityCause::Node {
            node: foreign.application
        })
    );
    let other = the_application(&unit, "other", ScalarOperator::Multiply);
    let under_other = call("other", vec![arg("a", 7), arg("b", 2)]);
    assert_eq!(bound(&unit, &under_other, &other).values(), [7, 2]);

    // A node outside the called function's body.
    let equality = occurrences_in(&unit, "eq", |node| equality_operator(node).is_some());
    assert_eq!(equality.len(), 1);
    let outside = ScalarOccurrence {
        function: "sub".to_owned(),
        ..equality[0].clone()
    };
    assert_eq!(
        refused(&unit, &values(), &outside),
        BindRefusal::ScalarIdentity(BindIdentityCause::Node {
            node: outside.application
        })
    );

    // A node absent from the package.
    let absent = WireNodeId::from_digest([0xee; 32]);
    let absent = ScalarOccurrence {
        application: absent,
        occurrence: OccurrenceKey::new(absent, sub.occurrence.origin().clone()),
        function: "sub".to_owned(),
    };
    assert_eq!(
        refused(&unit, &values(), &absent),
        BindRefusal::ScalarIdentity(BindIdentityCause::Node {
            node: absent.application
        })
    );

    // A function name that differs from the call's.
    let mut renamed = sub.clone();
    renamed.function = "other".to_owned();
    assert_eq!(
        refused(&unit, &values(), &renamed),
        BindRefusal::ScalarIdentity(BindIdentityCause::Function {
            claimed: "other".to_owned(),
            called: "sub".to_owned()
        })
    );

    // An equality application.
    let under_eq = call("eq", vec![arg("a", 7), arg("b", 2)]);
    assert_eq!(
        refused(&unit, &under_eq, &equality[0]),
        BindRefusal::ScalarIdentity(BindIdentityCause::Operator {
            node: equality[0].application
        })
    );
    assert_eq!(
        BindRefusal::ScalarIdentity(BindIdentityCause::Operator { node }).code(),
        Code::StaleDependency
    );
    assert_eq!(
        BindRefusal::ScalarIdentity(BindIdentityCause::Operator { node }).cause(),
        Some("content-mismatch")
    );
}

/// FR-367-AC-6: an unbounded operand and a nested application.
#[trace("TC-1053", "FR-367-AC-6")]
#[test]
fn tc_1053_step_5_unbounded_and_nested_operands() {
    let unit = compile(
        "function w using v(a: Integer): Integer pure { a + 1 }\n\
         function n using v(a: Int[0, 9]): Int[0, 12] pure { (a + 1) + 2 }",
    );
    let occurrence = the_application(&unit, "w", ScalarOperator::Add);
    assert_eq!(
        refused(&unit, &call("w", vec![arg("a", 4)]), &occurrence),
        BindRefusal::Operands(CheckedScalarOperandError::UnboundedRange)
    );
    let refusal = BindRefusal::Operands(CheckedScalarOperandError::UnboundedRange);
    assert_eq!(refusal.code(), Code::UnsupportedConstruct);
    assert_eq!(refusal.cause(), Some("expression-form"));

    // Source compilation types the inner `a + 1` as an unbounded `Integer`,
    // so Contract IR refuses the outer's operand list before any operand is
    // classified; the inner binds.
    let applications = occurrences_in(&unit, "n", |node| applies(node, ScalarOperator::Add));
    assert_eq!(applications.len(), 2);
    let n = call("n", vec![arg("a", 4)]);
    let outcomes: Vec<_> = applications
        .iter()
        .map(|occurrence| bind_call(&unit, &n, occurrence).unwrap())
        .collect();
    let inner: Vec<_> = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            BindOutcome::Bound(bound) => Some(bound.values()),
            _ => None,
        })
        .collect();
    assert_eq!(inner, [vec![4, 1]]);

    // An operand that is a nested application (not a literal, not a
    // parameter) refuses `OperandNotParameter` and binds no partial list.
    let package = read(&unit);
    let selected = select(unit.checked.package(), "n").unwrap();
    let arguments = [Value::Integer(Integer::from(4_i64))];
    let request = call("n", vec![arg("a", 4)]);
    let resolver = Resolver::new(&package, &request, selected.parameters, &arguments).unwrap();
    let nested = applications
        .iter()
        .find(|occurrence| {
            matches!(
                bind_call(&unit, &request, occurrence),
                Ok(BindOutcome::Refused(_))
            )
        })
        .expect("the outer application refuses");
    let operands = [CheckedScalarOperand {
        ordinal: 0,
        child: CheckedScalarOperandChild::GraphChild(checked_node_id(nested.application)),
        range: bound_range(0, 12),
    }];
    let Err(Halt::Refused(refusal)) = resolver.bind(&operands) else {
        panic!("a nested application operand refuses");
    };
    assert_eq!(refusal, BindRefusal::OperandNotParameter { ordinal: 0 });
    assert_eq!(refusal.code(), Code::UnsupportedConstruct);
    assert_eq!(refusal.cause(), Some("expression-form"));
}

/// FR-367-AC-7: a value past `i64` refuses; a range endpoint past it does not.
#[trace("TC-1053", "FR-367-AC-7")]
#[test]
fn tc_1053_step_6_values_past_i64_refuse() {
    let unit = compile(
        "function big using v(a: Int[0, 100000000000000000000]): Int[0, 100000000000000000001] pure { a + 1 }\n\
         function lit using v(a: Int[0, 9]): Int[0, 9223372036854775817] pure { a + 9223372036854775808 }",
    );
    let big = the_application(&unit, "big", ScalarOperator::Add);
    let fits = Integer::from(i64::MAX);
    let at_max = CallArgument {
        parameter: "a".to_owned(),
        value: Value::Integer(fits),
    };
    let bound = bound(&unit, &call("big", vec![at_max]), &big);
    assert_eq!(bound.values(), [i64::MAX, 1]);
    assert_eq!(
        bound.operands()[0].range,
        bound_range(0, 100_000_000_000_000_000_000)
    );

    let past = CallArgument {
        parameter: "a".to_owned(),
        value: Value::Integer(Integer::from(i128::from(i64::MAX) + 1)),
    };
    let refusal = refused(&unit, &call("big", vec![past]), &big);
    assert_eq!(
        refusal,
        BindRefusal::OperandNotNative {
            ordinal: 0,
            value: "9223372036854775808".to_owned()
        }
    );
    assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
    assert_eq!(refusal.cause(), None);

    let lit = the_application(&unit, "lit", ScalarOperator::Add);
    assert_eq!(
        refused(&unit, &call("lit", vec![arg("a", 4)]), &lit),
        BindRefusal::OperandNotNative {
            ordinal: 1,
            value: "9223372036854775808".to_owned()
        }
    );
}

/// FR-367-AC-8: binding is repeatable and evaluates nothing.
#[trace("TC-1053", "FR-367-AC-8")]
#[test]
fn tc_1053_step_7_binding_does_not_evaluate() {
    let unit = compile("function plus using v(a: Int[0, 9]): Int[1, 10] pure { a + 1 }");
    let occurrence = the_application(&unit, "plus", ScalarOperator::Add);
    let request = call("plus", vec![arg("a", 4)]);
    assert_eq!(
        bound(&unit, &request, &occurrence),
        bound(&unit, &request, &occurrence)
    );

    // The call needs more work units than it is given: `run` is incomplete
    // at `work_units`, and the binding evaluates nothing.
    let occurrence = the_application(&unit, "plus", ScalarOperator::Add);
    let mut starved = call("plus", vec![arg("a", 4)]);
    starved.accounting = default_accounting(0);
    let evaluated = run(
        identity(),
        "bind.native",
        unit.source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        &starved,
        &ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .unwrap()
    .1;
    let CallOutcome::Incomplete(incomplete) = evaluated else {
        panic!("run is incomplete at work_units, got {evaluated:?}");
    };
    assert_eq!(incomplete.limits_field(), "work_units");
    assert_eq!(bound(&unit, &starved, &occurrence).values(), [4, 1]);

    // A conversion past `value_occurrences` is incomplete and binds nothing.
    let occurrence = the_application(&unit, "plus", ScalarOperator::Add);
    let mut limited = call("plus", vec![arg("a", 4)]);
    limited.accounting.value_occurrences = 0;
    let Ok(BindOutcome::Incomplete(incomplete)) = bind_call(&unit, &limited, &occurrence) else {
        panic!("a conversion past value_occurrences is incomplete");
    };
    assert_eq!(incomplete.limits_field(), "value_occurrences");
    assert_eq!(incomplete.record.limit, 0);
    assert_eq!(incomplete.record.consumed, 0);
    assert_eq!(incomplete.record.next_charge, Integer::from(1_i64));
}

/// FR-367-AC-9: the binding is FR-357's operator-level operand list, and its
/// identities are the ones FR-357 checks against the application.
#[trace("TC-1053", "FR-367-AC-9")]
#[test]
fn tc_1053_step_8_binding_is_the_operator_claims_operand_list() {
    let unit = compile(
        "function plus using v(a: Int[0, 9]): Int[1, 10] pure { a + 1 }\n\
         function sub using v(a: Int[0, 9], b: Int[0, 9]): Int[-9, 9] pure { a - b }",
    );
    let graph = unit.checked.package().graph().semantic_graph();
    let claimed = |function: &str, operator, request: Call| {
        let occurrence = the_application(&unit, function, operator);
        let bound = bound(&unit, &request, &occurrence);
        let operands: Vec<ScalarOperand> = bound
            .operands()
            .iter()
            .map(|operand| {
                let range = IntegerInterval::spanning(
                    Integer::from(operand.range.lower),
                    Integer::from(operand.range.upper),
                );
                match operand.source {
                    OperandSource::InlineLiteral { node: None } => {
                        ScalarOperand::literal(operand.value)
                    }
                    OperandSource::Argument { node, .. }
                    | OperandSource::InlineLiteral { node: Some(node) } => {
                        ScalarOperand::graph_child(operand.value, range, node)
                    }
                }
            })
            .collect();
        let preimage = {
            let node = graph.node(graph.resolve_wire(occurrence.application).unwrap());
            serde_json::from_slice::<serde_json::Value>(node.unwrap().preimage()).unwrap()
        };
        (bound, operands, preimage, occurrence)
    };

    let (bound, mut operands, preimage, occurrence) = claimed(
        "sub",
        ScalarOperator::Subtract,
        call("sub", vec![arg("b", 2), arg("a", 7)]),
    );
    let (right, left) = (operands.pop().unwrap(), operands.pop().unwrap());
    assert_eq!(
        [left.value, right.value],
        <[i64; 2]>::try_from(bound.values()).unwrap()
    );
    let operation = ScalarOperation::Subtract { left, right };
    assert!(check_operands(&preimage, occurrence.application, &operation).is_ok());

    let (bound, mut operands, preimage, occurrence) =
        claimed("plus", ScalarOperator::Add, call("plus", vec![arg("a", 4)]));
    let (right, left) = (operands.pop().unwrap(), operands.pop().unwrap());
    assert!(matches!(
        right.identity,
        crate::scalar::OperandIdentity::GraphChild(_)
    ));
    let operation = ScalarOperation::Add { left, right };
    assert!(check_operands(&preimage, occurrence.application, &operation).is_ok());
    assert_eq!(bound.values(), [4, 1]);
}

/// FR-367-AC-10: a `let` binder operand is a parameter node no argument
/// supplies.
#[trace("TC-1053", "FR-367-AC-10")]
#[test]
fn tc_1053_step_5_binder_operand_is_not_an_argument() {
    let unit = compile("function lt using v(a: Int[0, 9]): Int[0, 11] pure { let x = a in x + 1 }");
    let adds = occurrences_in(&unit, "lt", |node| applies(node, ScalarOperator::Add));
    // `x + 1`; the binding `a` is no application.
    assert_eq!(adds.len(), 1);
    let refusal = refused(&unit, &call("lt", vec![arg("a", 4)]), &adds[0]);
    assert_eq!(
        refusal,
        BindRefusal::OperandNotArgument {
            ordinal: 0,
            parameter: "x".to_owned(),
            level: "1".to_owned()
        }
    );
    assert_eq!(refusal.code(), Code::UnsupportedConstruct);
    assert_eq!(refusal.cause(), Some("expression-form"));
}
