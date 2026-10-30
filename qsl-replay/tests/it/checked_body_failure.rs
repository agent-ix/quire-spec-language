// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-485-AC-15: owner-authored QSV failures cross the public application boundary.

use ix_trace_rs::trace;
use qsl_replay::spine::{self, ParseRequest, SpineLimits};
use qsl_replay::{
    acquire_checked_closure, ApplicationEnvironment, CallableSelection, CheckedClosureCompilation,
    CheckedClosureHandle, DependencyInput, NodeKey, NominalDeclaration, NominalDeclarationForm,
    NominalDeclarationMember, ReplayLimits, SourceIdentity, StageLimits,
};
use quire_eval::{
    Admission, Body, BodyFactory, BodyFailure, CallFailure, Evaluation, FamilyOutcome, Resume,
    Transition,
};
use quire_exact::{
    Cancel, CancelCause, ChargePoint, Integer, LimitKind, Meter, NodeKey, Outcome, ScalarLimits,
    UnionMember, Value, ValueType,
};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, EnvironmentFailure, ReservationDecision, StorageRequest,
    TypeEnvironment, TypeEnvironmentLimits, UnionMemberDeclaration,
};
use quire_semantic_value::object_closure::ObjectClosure;
use std::sync::{Arc, Mutex};

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.value.complete/v1\";\n\
     union Payload { One(Integer) }\n\
     union Other { Other(Integer) }\n\
     function f using v(x: Payload): Integer pure { 1 }\n\
     function g using v(x: Other): Integer pure { 2 }\n";

#[derive(Default)]
struct Capture {
    failure: Option<EnvironmentFailure>,
    request: Option<StorageRequest>,
}

type CaptureRef = Arc<Mutex<Capture>>;

#[derive(Clone)]
enum Action {
    MembershipFalse {
        types: Box<TypeEnvironment>,
        expected: ValueType,
        value: Box<Value>,
    },
    SealRefused {
        types: Box<TypeEnvironment>,
        handle: NodeKey,
        final_key: NodeKey,
        bindings: Vec<UnionMember>,
        cancel: Cancel,
        capture: CaptureRef,
    },
    Reservation {
        declarations: Vec<CompositeDeclaration>,
        decision: ReservationDecision,
        cancel: Cancel,
        capture: CaptureRef,
    },
    WorkLimit {
        declarations: Vec<CompositeDeclaration>,
        cancel: Cancel,
        trip_cancel: bool,
        capture: CaptureRef,
    },
    Fault,
}

struct ActionFactory(Action);

impl BodyFactory for ActionFactory {
    fn begin(&self) -> Box<dyn Body> {
        Box::new(ActionBody(self.0.clone()))
    }
}

struct ActionBody(Action);

impl Body for ActionBody {
    fn resume(&mut self, input: Resume, _meter: &mut Meter) -> Result<Transition, BodyFailure> {
        assert!(matches!(input, Resume::Start(_)));
        match &self.0 {
            Action::MembershipFalse {
                types,
                expected,
                value,
            } => match types.admits(expected, value) {
                Ok(false) => Ok(Transition::Done(Evaluation {
                    outcome: FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(
                        Integer::from(7_u64),
                    ))),
                    location: None,
                    losses: vec![],
                })),
                Ok(true) => panic!("the mismatched value was admitted"),
                Err(failure) => Err(BodyFailure::qsv_environment(failure, None)),
            },
            Action::SealRefused {
                types,
                handle,
                final_key,
                bindings,
                cancel,
                capture,
            } => match types
                .clone()
                .seal_union_verified(*handle, *final_key, bindings.clone())
            {
                Err(failure) => {
                    capture.lock().expect("capture mutex").failure = Some(failure.clone());
                    Err(BodyFailure::qsv_environment(failure, Some(cancel)))
                }
                Ok(()) => panic!("mismatched checked union bindings unexpectedly sealed"),
            },
            Action::Reservation {
                declarations,
                decision,
                cancel,
                capture,
            } => {
                let admission =
                    TypeEnvironment::new_with_reservation(declarations.clone(), [], &|request| {
                        capture.lock().expect("capture mutex").request = Some(request);
                        *decision
                    });
                match admission {
                    Err(failure) => {
                        capture.lock().expect("capture mutex").failure = Some(failure.clone());
                        Err(BodyFailure::qsv_environment(failure, Some(cancel)))
                    }
                    Ok(_) => panic!("reservation control did not produce a failure"),
                }
            }
            Action::WorkLimit {
                declarations,
                cancel,
                trip_cancel,
                capture,
            } => {
                if *trip_cancel {
                    cancel.cancel(CancelCause::Requested);
                }
                let limits = if *trip_cancel {
                    TypeEnvironmentLimits::default()
                } else {
                    TypeEnvironmentLimits {
                        ancestor_steps: u64::MAX,
                        work_units: 0,
                    }
                };
                match TypeEnvironment::bounded_with_cancel(declarations.clone(), [], limits, cancel)
                {
                    Err(failure) => {
                        capture.lock().expect("capture mutex").failure = Some(failure.clone());
                        Err(BodyFailure::qsv_environment(failure, Some(cancel)))
                    }
                    Ok(_) => panic!("work limit did not stop type-environment admission"),
                }
            }
            Action::Fault => Err(BodyFailure::Fault(InternalFault::new(
                "generated-body",
                "broken-invariant",
            ))),
        }
    }
}

struct Fixture {
    _acquired: CheckedClosureCompilation,
    handle: CheckedClosureHandle,
    selection: CallableSelection,
    payload: CompositeDeclaration,
    payload_environment: TypeEnvironment,
    refusal_environment: TypeEnvironment,
    other_bindings: Vec<UnionMember>,
    argument: Value,
}

fn fixture() -> Fixture {
    let source = SourceIdentity::new("a", "body-failure", "git", "1");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "body-failure.native",
            bytes: UNIT.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("fixture parses")
    .into_value();
    let models = spine::select(&parsed, &Default::default(), limits.model, &cancel)
        .expect("fixture selects")
        .into_value();
    let checked = spine::check(
        &parsed,
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .expect("fixture checks")
    .into_value();
    let graph = checked.package().graph();
    let function_node = |name: &str| {
        NodeKey::from_digest(
            *graph
                .function_identity(name)
                .expect("checked function identity")
                .as_bytes(),
        )
    };
    let composite_node = |name: &str| {
        let node = graph
            .semantic_graph()
            .nodes()
            .find(|node| {
                node.declaration()
                    .and_then(|name| name.last())
                    .is_some_and(|segment| segment.as_str() == name)
            })
            .expect("checked composite declaration");
        node.key().clone()
    };
    let f_node = function_node("f");
    let g_node = function_node("g");
    let payload_node = composite_node("Payload");
    let other_node = composite_node("Other");
    drop(checked);

    let acquired = acquire_checked_closure(
        source,
        "body-failure.native",
        UNIT.as_bytes(),
        [],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("closure acquired");
    let handle = acquired.handle().clone();
    let selection = handle
        .lookup_callable(handle.package_id(), f_node)
        .expect("f selected");
    let other_selection = handle
        .lookup_callable(handle.package_id(), g_node)
        .expect("g selected");
    let payload_view = handle
        .nominal_declaration(&selection, payload_node)
        .expect("Payload declaration is reachable from f");
    let other_view = handle
        .nominal_declaration(&other_selection, other_node)
        .expect("Other declaration is reachable from g");
    let payload = composite(&payload_view);
    let payload_environment = environment(&payload_view, payload.clone());
    let refusal_environment = TypeEnvironment::new([payload.clone()], [])
        .expect("owner-authored Payload declaration admits before binding");
    let union_variant = |view: &NominalDeclaration| {
        let [NominalDeclarationMember::UnionMember { variant, .. }] = view.members() else {
            panic!("expected the checked union declaration to have one member")
        };
        *variant
    };
    let argument = payload_environment
        .union(
            payload.key(),
            union_variant(&payload_view),
            vec![Value::Integer(Integer::from(1_u64))],
        )
        .expect("Payload value admits");
    Fixture {
        _acquired: acquired,
        handle,
        selection,
        payload,
        payload_environment,
        refusal_environment,
        other_bindings: verified_union_members(&other_view),
        argument,
    }
}

fn composite(view: &NominalDeclaration) -> CompositeDeclaration {
    assert_eq!(view.form(), NominalDeclarationForm::Union);
    let members = view
        .members()
        .iter()
        .map(|member| match member {
            NominalDeclarationMember::UnionMember {
                member, payload, ..
            } => UnionMemberDeclaration::resolved(member.identifier().clone(), payload.clone()),
            other => panic!("unexpected member in union declaration: {other:?}"),
        })
        .collect();
    let name = view
        .qualified_name()
        .iter()
        .map(|part| part.as_str())
        .collect::<Vec<_>>()
        .join("::");
    CompositeDeclaration::new(view.key(), name, CompositeShape::Union(members))
}

fn verified_union_members(view: &NominalDeclaration) -> Vec<UnionMember> {
    view.members()
        .iter()
        .map(|member| match member {
            NominalDeclarationMember::UnionMember { member, .. } => member.clone(),
            other => panic!("unexpected member in union declaration: {other:?}"),
        })
        .collect()
}

fn environment(view: &NominalDeclaration, declaration: CompositeDeclaration) -> TypeEnvironment {
    let mut environment =
        TypeEnvironment::new([declaration], []).expect("owner-authored union declaration admits");
    environment
        .seal_union_verified(view.key(), view.node(), verified_union_members(view))
        .expect("checked union member bindings seal the declaration");
    environment
}

fn run(fixture: &Fixture, action: Action) -> (Result<Evaluation, CallFailure>, Meter) {
    let mut registry = fixture.handle.registry();
    let admission = registry
        .register(
            &fixture.selection,
            fixture.selection.signature(),
            ActionFactory(action),
        )
        .expect("selection belongs to registry acquisition");
    assert!(matches!(admission, Admission::Accepted));
    let environment = ApplicationEnvironment::new(ObjectClosure::default());
    let mut meter = Meter::new(ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units: 1_000,
        result_units: u64::MAX,
    });
    let result = registry
        .begin_call(
            &fixture.selection,
            vec![fixture.argument.clone()],
            &environment,
            &mut meter,
        )
        .expect("valid root argument begins")
        .run();
    (result, meter)
}

fn assert_one_root_charge(meter: &Meter) {
    assert_eq!(meter.admitted_charges().len(), 1);
    assert_eq!(meter.admitted_charges()[0].point, ChargePoint::FunctionCall);
    assert!(meter.consumed(LimitKind::WorkUnits) > 0);
}

fn expected_failure(capture: &CaptureRef) -> EnvironmentFailure {
    capture
        .lock()
        .expect("capture mutex")
        .failure
        .clone()
        .expect("the public QSV operation returned a failure")
}

fn assert_qsv_failure(
    result: Result<Evaluation, CallFailure>,
    expected: EnvironmentFailure,
    cancellation: Option<CancelCause>,
) {
    let Err(CallFailure::Body(body_failure)) = result else {
        panic!("body-side QSV failure did not remain typed: {result:?}");
    };
    let BodyFailure::QsvEnvironment {
        failure,
        cancellation: observed_cancel,
    } = body_failure.as_ref()
    else {
        panic!("body-side QSV failure did not retain its environment variant");
    };
    assert_eq!(failure, &expected);
    assert_eq!(*observed_cancel, cancellation);
}

/// Exercise the real QSV operations from a public QSL body and preserve their typed results.
#[trace("FR-485-AC-15", "FR-401-AC-12", "FR-401-AC-13")]
#[test]
fn qsv_body_failures_cross_checked_application_without_losing_charge_or_shape() {
    let fixture = fixture();
    let payload_type = ValueType::Composite(fixture.payload.key());
    let payload_environment = fixture.payload_environment.clone();

    let (result, meter) = run(
        &fixture,
        Action::MembershipFalse {
            types: Box::new(payload_environment.clone()),
            expected: payload_type,
            value: Box::new(Value::Integer(Integer::from(99_u64))),
        },
    );
    let evaluation = result.expect("Ok(false) is not an error");
    assert!(matches!(
        evaluation.outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) if value == Integer::from(7_u64)
    ));
    assert_one_root_charge(&meter);

    let limit_capture = Arc::new(Mutex::new(Capture::default()));
    let (result, meter) = run(
        &fixture,
        Action::WorkLimit {
            declarations: vec![fixture.payload.clone()],
            cancel: Cancel::new(),
            trip_cancel: false,
            capture: Arc::clone(&limit_capture),
        },
    );
    let limit = expected_failure(&limit_capture);
    assert!(matches!(limit, EnvironmentFailure::Limit(_)));
    assert_qsv_failure(result, limit, None);
    assert_one_root_charge(&meter);

    let cancelled_capture = Arc::new(Mutex::new(Capture::default()));
    let (result, meter) = run(
        &fixture,
        Action::WorkLimit {
            declarations: vec![fixture.payload.clone()],
            cancel: Cancel::new(),
            trip_cancel: true,
            capture: Arc::clone(&cancelled_capture),
        },
    );
    let cancelled_limit = expected_failure(&cancelled_capture);
    assert!(matches!(cancelled_limit, EnvironmentFailure::Limit(_)));
    assert_qsv_failure(result, cancelled_limit, Some(CancelCause::Requested));
    assert_one_root_charge(&meter);

    for (decision, expected_variant) in [
        (ReservationDecision::AllocationDenied, "allocation"),
        (ReservationDecision::CapacityExceeded, "capacity"),
    ] {
        let capture = Arc::new(Mutex::new(Capture::default()));
        let (result, meter) = run(
            &fixture,
            Action::Reservation {
                declarations: vec![fixture.payload.clone()],
                decision,
                cancel: Cancel::new(),
                capture: Arc::clone(&capture),
            },
        );
        let observed = expected_failure(&capture);
        let request = capture
            .lock()
            .expect("capture mutex")
            .request
            .expect("the actual reservation boundary ran");
        assert!(request.amount() > 0);
        match (&observed, expected_variant) {
            (EnvironmentFailure::Allocation(actual), "allocation")
            | (EnvironmentFailure::Capacity(actual), "capacity") => assert_eq!(*actual, request),
            _ => panic!("wrong QSV failure for {decision:?}: {observed:?}"),
        }
        assert_qsv_failure(result, observed, None);
        assert_one_root_charge(&meter);
    }

    let refused_capture = Arc::new(Mutex::new(Capture::default()));
    let (result, meter) = run(
        &fixture,
        Action::SealRefused {
            types: Box::new(fixture.refusal_environment.clone()),
            handle: fixture.payload.key(),
            final_key: fixture
                .payload_environment
                .union_key(fixture.payload.key())
                .unwrap(),
            bindings: fixture.other_bindings.clone(),
            cancel: Cancel::new(),
            capture: Arc::clone(&refused_capture),
        },
    );
    let refused = expected_failure(&refused_capture);
    assert!(matches!(refused, EnvironmentFailure::Refused(_)));
    assert_qsv_failure(result, refused, None);
    assert_one_root_charge(&meter);

    let (result, meter) = run(&fixture, Action::Fault);
    let Err(CallFailure::Body(body_failure)) = result else {
        panic!("body fault did not remain a typed call failure: {result:?}");
    };
    let BodyFailure::Fault(fault) = body_failure.as_ref() else {
        panic!("the generated body invariant did not remain a fault");
    };
    assert_eq!(fault.stage(), "generated-body");
    assert_eq!(fault.invariant(), "broken-invariant");
    assert_one_root_charge(&meter);
}
