// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-401 (FR-262): the std reference adapter. A `quire-eval` registry over a
//! checked package admits exactly the package's callables by their minted
//! identity and signature, and a session over it runs interpreted and
//! registered host bodies through one scheduler with the results
//! `CheckedPackage::call` gives.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, PackageClosure, QualifiedName};
use qsl_forms::{
    DeclarationSpans, DeclaredClauseKind, Expression, ExpressionSpans, FunctionDeclaration,
};
use qsl_foundation::{SourceIdentity, Span};
use qsl_package::{CheckedPackage, Import};
use qsl_semantics::check::{CheckedExpression, PackageDeclarations};
use qsl_semantics::library::LibraryName;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::{
    Admission, Body, BodyFactory, CallFailure, Evaluation, FamilyOutcome, Progress, Registry,
    Resume, Signature, Transition,
};
use quire_exact::{
    Cancel, CancelCause, ChargePoint, Integer, IntegerInterval, LimitKind, Meter, NodeKey, Outcome,
    ScalarLimits, Value, ValueType,
};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::location::{Location, Origin};

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

type Closure<'p> = PackageClosure<&'p CheckedPackage, &'p CheckedExpression>;

fn form(value_type: &ValueType) -> qsl_forms::TypeForm {
    crate::support::type_form::type_form(value_type)
}

fn int(n: i64) -> Value {
    Value::Integer(Integer::from(n))
}

fn range(lower: i64, upper: i64) -> ValueType {
    ValueType::Int(IntegerInterval::new(Integer::from(lower), Integer::from(upper)).unwrap())
}

fn declarations(functions: Vec<FunctionDeclaration>) -> PackageDeclarations {
    PackageDeclarations {
        functions,
        ..PackageDeclarations::new(
            qsl_semantics::check::fixture_source(),
            qsl_foundation::IdentityLimits::default(),
        )
    }
}

/// `keep(a: Integer): Integer { a }`.
fn keep() -> FunctionDeclaration {
    FunctionDeclaration::new(
        "keep",
        vec![("a".to_owned(), form(&ValueType::Integer))],
        form(&ValueType::Integer),
        None,
        Expression::name("a".to_owned()),
    )
}

/// `f(x: Int[0, 9]): Boolean { true }`.
fn f() -> FunctionDeclaration {
    FunctionDeclaration::new(
        "f",
        vec![("x".to_owned(), form(&range(0, 9)))],
        form(&ValueType::Boolean),
        None,
        Expression::boolean(true),
    )
}

/// A host-only callable: `<name>(x: Int[0, 9]): Boolean { true }`, whose
/// registered body stands in for it.
fn stand_in(name: &str) -> FunctionDeclaration {
    FunctionDeclaration::new(
        name,
        vec![("x".to_owned(), form(&range(0, 9)))],
        form(&ValueType::Boolean),
        None,
        Expression::boolean(true),
    )
}

fn package(functions: Vec<FunctionDeclaration>) -> CheckedPackage {
    CheckedPackage::link(
        declarations(functions)
            .check(CheckingLimits::default())
            .unwrap(),
    )
}

fn identity(package: &CheckedPackage, name: &str) -> NodeKey {
    package.graph().callable(name).expect("declared").identity
}

fn completed(evaluation: &Evaluation) -> Option<&Value> {
    match &evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => Some(value),
        _ => None,
    }
}

fn completed_int(evaluation: &Evaluation) -> Option<i64> {
    match completed(evaluation) {
        Some(Value::Integer(n)) => n.to_u64().and_then(|n| i64::try_from(n).ok()),
        _ => None,
    }
}

fn work(meter: &Meter) -> u64 {
    meter.consumed(LimitKind::WorkUnits)
}

fn call_site() -> Location {
    Location::root(Origin::Expression)
}

/// A host body that requests `callee` with `arguments` and returns the
/// child's outcome unchanged. `starts` counts its transitions.
struct Request {
    callee: NodeKey,
    arguments: Vec<Value>,
    starts: Arc<AtomicUsize>,
}

impl BodyFactory for Request {
    fn begin(&self) -> Box<dyn Body> {
        Box::new(RequestBody {
            callee: self.callee,
            arguments: self.arguments.clone(),
            transitions: Arc::clone(&self.starts),
        })
    }
}

struct RequestBody {
    callee: NodeKey,
    arguments: Vec<Value>,
    transitions: Arc<AtomicUsize>,
}

impl Body for RequestBody {
    fn resume(&mut self, input: Resume, _meter: &mut Meter) -> Result<Transition, InternalFault> {
        self.transitions.fetch_add(1, Ordering::SeqCst);
        match input {
            Resume::Start(_) | Resume::Again => Ok(Transition::Call(quire_eval::ChildRequest {
                callee: self.callee,
                arguments: self.arguments.clone(),
                site: call_site(),
            })),
            Resume::Child(child) => Ok(Transition::Done(child)),
        }
    }
}

/// A host body that returns a constant.
struct Constant(i64);

impl BodyFactory for Constant {
    fn begin(&self) -> Box<dyn Body> {
        Box::new(ConstantBody(self.0))
    }
}

struct ConstantBody(i64);

impl Body for ConstantBody {
    fn resume(&mut self, _input: Resume, _meter: &mut Meter) -> Result<Transition, InternalFault> {
        Ok(Transition::Done(Evaluation::completed(int(self.0))))
    }
}

fn register_request(
    registry: &mut Registry<Closure<'_>>,
    host: NodeKey,
    callee: NodeKey,
    arguments: Vec<Value>,
) -> Arc<AtomicUsize> {
    let transitions = Arc::new(AtomicUsize::new(0));
    let signature = Signature::new(vec![range(0, 9)], ValueType::Boolean);
    assert_eq!(
        registry.register(
            host,
            &signature,
            Request {
                callee,
                arguments,
                starts: Arc::clone(&transitions),
            },
        ),
        Admission::Accepted
    );
    transitions
}

const DEPENDENCY_TEXT: &[u8] = b"function g(): Boolean pure { true }";

/// `g(): Boolean { true }` read from [`DEPENDENCY_TEXT`]. A dependency is
/// emitted when it is linked, which needs every occurrence located, so the
/// declaration carries its spans.
fn located_dependency() -> CheckedPackage {
    let text = std::str::from_utf8(DEPENDENCY_TEXT).unwrap();
    let literal_start = text.rfind("true").unwrap();
    let literal = Span {
        start: literal_start,
        end: literal_start + "true".len(),
    };
    let declaration = FunctionDeclaration::new(
        "g",
        Vec::new(),
        form(&ValueType::Boolean),
        None,
        Expression::boolean(true),
    )
    .with_spans(DeclarationSpans {
        declaration: Span {
            start: 0,
            end: text.len(),
        },
        body: ExpressionSpans::new(literal).unwrap(),
        measure: None,
    })
    .expect("the spans fit g");
    let source = qsl_semantics::check::admitted_source(
        SourceIdentity::new("a", "u", "git", "1"),
        DEPENDENCY_TEXT,
    );
    CheckedPackage::link(
        PackageDeclarations {
            functions: vec![declaration],
            ..PackageDeclarations::new(source, qsl_foundation::IdentityLimits::default())
        }
        .check(CheckingLimits::default())
        .unwrap(),
    )
}

/// FR-401-AC-5: registration is decided from the package's admitted signature
/// alone, over `f(x: Int[0, 9]): Boolean`, a dependency package's
/// `g(): Boolean`, a function that is not callable by name (the mechanism an
/// FR-151 synthesized dispatch candidate has) and a key no function has.
#[trace("FR-401-AC-5")]
#[test]
fn registration_is_decided_from_the_packages_admitted_signature() {
    let dependency = located_dependency();
    let g = identity(&dependency, "g");
    let guard = FunctionDeclaration::clause(
        "internal_guard",
        Vec::new(),
        form(&ValueType::Boolean),
        None,
        Expression::boolean(true),
        DeclaredClauseKind::Body,
    );
    let graph = declarations(vec![f(), guard])
        .check(CheckingLimits::default())
        .unwrap();
    let guard_key = graph.function_identity("internal_guard").expect("declared");
    let root = CheckedPackage::link_with(
        graph,
        vec![Import {
            identity: LibraryName::new("lib.g").unwrap(),
            package: Arc::new(dependency),
        }],
    )
    .expect("the dependency links");
    let f_key = identity(&root, "f");
    let unknown = NodeKey::from_digest([7; 32]);
    let fresh = || Registry::new(Closure::new(&root));

    let admitted = Signature::new(vec![range(0, 9)], ValueType::Boolean);
    let wider = Signature::new(vec![range(0, 10)], ValueType::Boolean);
    let generator = Signature::new(Vec::new(), ValueType::Boolean);
    let guard_signature = Signature::new(Vec::new(), ValueType::Boolean);

    assert_eq!(
        fresh().register(f_key, &admitted, Constant(0)),
        Admission::Accepted
    );
    assert_eq!(
        fresh().register(f_key, &wider, Constant(0)),
        Admission::SignatureMismatch
    );
    assert_eq!(
        fresh().register(g, &generator, Constant(0)),
        Admission::Accepted
    );
    assert_eq!(
        fresh().register(guard_key, &guard_signature, Constant(0)),
        Admission::CallableNotAdmitted
    );
    assert_eq!(
        fresh().register(unknown, &admitted, Constant(0)),
        Admission::CallableNotAdmitted
    );
    let mut twice = fresh();
    assert_eq!(
        twice.register(f_key, &admitted, Constant(0)),
        Admission::Accepted
    );
    assert_eq!(
        twice.register(f_key, &admitted, Constant(0)),
        Admission::DuplicateBinding
    );
}

/// A session over the adapter and `CheckedPackage::call` charge exactly one
/// `function.call` for an application whose body charges nothing, and agree
/// on the value.
#[test]
fn a_session_over_the_adapter_charges_one_call_for_a_chargeless_body() {
    let package = package(vec![keep()]);
    let objects = ObjectEnvironment::default();
    let key = identity(&package, "keep");

    let mut called = Meter::new(UNLIMITED);
    let by_call = package
        .call(
            &QualifiedName::unqualified("keep").unwrap(),
            vec![int(5)],
            &objects,
            &mut called,
        )
        .unwrap();
    let mut driven = Meter::new(UNLIMITED);
    let mut registry = Registry::new(Closure::new(&package));
    let by_session = registry
        .begin_call(key, vec![int(5)], &objects, &mut driven)
        .unwrap()
        .run()
        .unwrap();

    assert_eq!(completed_int(&by_call), Some(5));
    assert_eq!(completed_int(&by_session), Some(5));
    for meter in [&called, &driven] {
        assert_eq!(work(meter), 1);
    }
}

/// A registered host body runs in place of the interpreted body for a root
/// application, under one `function.call`, once `Begin` has sealed the
/// registry.
#[test]
fn a_registered_host_body_runs_for_its_root_callable() {
    let package = package(vec![keep()]);
    let objects = ObjectEnvironment::default();
    let mut registry = Registry::new(Closure::new(&package));
    let admitted = Signature::new(vec![ValueType::Integer], ValueType::Integer);
    assert_eq!(
        registry.register(identity(&package, "keep"), &admitted, Constant(42)),
        Admission::Accepted
    );
    let mut meter = Meter::new(UNLIMITED);
    let result = registry
        .begin_call(
            identity(&package, "keep"),
            vec![int(5)],
            &objects,
            &mut meter,
        )
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(completed_int(&result), Some(42));
    assert_eq!(work(&meter), 1);
    assert!(registry.is_sealed());
}

/// `Begin` over a closure that admits its own arguments refuses a wrong
/// argument count before any charge, and leaves the registry open.
#[test]
fn begin_refuses_a_wrong_argument_count_before_any_charge() {
    let package = package(vec![keep()]);
    let objects = ObjectEnvironment::default();
    let mut registry = Registry::new(Closure::new(&package));
    let mut meter = Meter::new(UNLIMITED);
    let refused = registry.begin_call(identity(&package, "keep"), vec![], &objects, &mut meter);
    assert!(matches!(
        refused,
        Err(CallFailure::Input(InputRefusal::Arity {
            declared: 1,
            supplied: 0
        }))
    ));
    assert!(!registry.is_sealed());
    assert_eq!(meter.admission_count(), 0);
}

/// FR-262: `Begin`'s supplied-value run polls the meter's installed handle,
/// so a handle cancelled before `Begin` ends it with `Cancelled`, before any
/// charge.
#[trace("FR-262-AC-10")]
#[test]
fn begin_observes_the_meters_cancelled_handle() {
    let package = package(vec![keep()]);
    let objects = ObjectEnvironment::default();
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Deadline);
    let mut meter = Meter::new(UNLIMITED).with_cancel(cancel);
    let mut registry = Registry::new(Closure::new(&package));
    let refused = registry.begin_call(
        identity(&package, "keep"),
        vec![int(1)],
        &objects,
        &mut meter,
    );
    assert!(matches!(
        refused,
        Err(CallFailure::Cancelled(CancelCause::Deadline))
    ));
    assert!(!registry.is_sealed());
    assert_eq!(meter.admission_count(), 0);
}

/// FR-262-AC-11 over the adapter: a body requesting a `NodeKey` no function
/// has, `f` with two arguments, or `f` with `10` ends the session as an
/// internal fault, with no child charge, no callee body and the requesting
/// body not resumed. A valid request, `f(9)`, is applied with one
/// `function.call`. (The two `Population` runs and the storage run need an
/// object model and an exhausted reservation; they are `quire-eval`'s toy
/// closure tests.)
#[trace("FR-262-AC-11")]
#[test]
fn invalid_child_requests_fault_and_a_valid_one_is_applied() {
    let package = package(vec![f(), stand_in("host")]);
    let objects = ObjectEnvironment::default();
    let (f_key, host) = (identity(&package, "f"), identity(&package, "host"));
    let cases: [(&str, NodeKey, Vec<Value>); 3] = [
        ("unknown key", NodeKey::from_digest([9; 32]), vec![int(1)]),
        ("two arguments", f_key, vec![int(1), int(2)]),
        ("out of range", f_key, vec![int(10)]),
    ];
    for (what, callee, arguments) in cases {
        let mut registry = Registry::new(Closure::new(&package));
        let transitions = register_request(&mut registry, host, callee, arguments);
        let mut meter = Meter::new(UNLIMITED);
        let result = registry
            .begin_call(host, vec![int(1)], &objects, &mut meter)
            .unwrap()
            .run();
        match result {
            Err(CallFailure::Fault(fault)) => assert_eq!(
                (fault.stage(), fault.invariant()),
                ("quire-eval", "child-request-not-admitted"),
                "{what}"
            ),
            other => panic!("{what}: expected an internal fault, got {other:?}"),
        }
        assert_eq!(work(&meter), 1, "{what}: only the root call was charged");
        assert_eq!(transitions.load(Ordering::SeqCst), 1, "{what}: not resumed");
    }
    let mut registry = Registry::new(Closure::new(&package));
    let transitions = register_request(&mut registry, host, f_key, vec![int(9)]);
    let mut meter = Meter::new(UNLIMITED);
    let result = registry
        .begin_call(host, vec![int(1)], &objects, &mut meter)
        .unwrap()
        .run()
        .unwrap();
    assert!(matches!(completed(&result), Some(Value::Boolean(true))));
    assert_eq!(work(&meter), 2, "one call charge for the child");
    assert_eq!(transitions.load(Ordering::SeqCst), 2);
}

/// FR-262-AC-4 and AC-6 over the adapter: a host body calls a second host
/// body that calls an interpreted function. Stepping between every handoff
/// gives the synchronous result and charges; the limit `w` completes, and
/// `w - 1` returns the located incomplete result at the denied call, keeping
/// earlier spend and leaving the denied charge unrecorded.
#[trace("FR-262-AC-4", "FR-262-AC-6")]
#[test]
fn a_mixed_chain_steps_to_the_synchronous_result_and_stops_at_its_fuel() {
    let package = package(vec![f(), stand_in("first"), stand_in("second")]);
    let objects = ObjectEnvironment::default();
    let (f_key, first, second) = (
        identity(&package, "f"),
        identity(&package, "first"),
        identity(&package, "second"),
    );
    let chain = |limit: u64| {
        let mut registry = Registry::new(Closure::new(&package));
        let a = register_request(&mut registry, first, second, vec![int(3)]);
        let b = register_request(&mut registry, second, f_key, vec![int(3)]);
        let mut meter = Meter::new(ScalarLimits {
            work_units: limit,
            ..UNLIMITED
        });
        let mut session = registry
            .begin_call(first, vec![int(3)], &objects, &mut meter)
            .unwrap();
        let mut steps = 0;
        let result = loop {
            steps += 1;
            match session.step() {
                Progress::Runnable(next) => session = next,
                Progress::Terminal(result) => break result,
            }
        };
        let charges: Vec<_> = meter.admitted_charges().iter().map(|c| c.point).collect();
        (
            result,
            work(&meter),
            charges,
            steps,
            (a.load(Ordering::SeqCst), b.load(Ordering::SeqCst)),
        )
    };
    let (done, w, charges, steps, resumed) = chain(u64::MAX);
    assert!(matches!(
        completed(&done.unwrap()),
        Some(Value::Boolean(true))
    ));
    assert_eq!(w, 3);
    assert_eq!(charges, [ChargePoint::FunctionCall; 3]);
    assert!(steps > 3);
    assert_eq!(resumed, (2, 2), "each caller resumed once per handoff");

    let (exact, spent_exact, ..) = chain(w);
    assert!(exact.is_ok());
    assert_eq!(spent_exact, w);
    let (denied, spent_short, _, _, resumed_short) = chain(w - 1);
    let denied = denied.expect("a denied charge is an evaluation");
    assert!(matches!(
        &denied.outcome,
        FamilyOutcome::Evaluated(Outcome::Incomplete(record))
            if record.charge_point == ChargePoint::FunctionCall
                && record.limit_kind == LimitKind::WorkUnits
    ));
    assert_eq!(denied.location, Some(call_site()));
    assert_eq!(
        spent_short,
        w - 1,
        "earlier spend kept, denied charge unrecorded"
    );
    assert_eq!(resumed_short, (1, 1), "no caller resumed with a child");
}

/// FR-262-AC-7 over the adapter: the meter's handle cancelled after a child
/// request is saved ends the session at the child's `function.call` charge
/// with the located incomplete result, no child body and no later charge.
#[trace("FR-262-AC-7")]
#[test]
fn cancelling_after_a_request_is_saved_ends_at_the_childs_call_charge() {
    let package = package(vec![f(), stand_in("host")]);
    let objects = ObjectEnvironment::default();
    let (f_key, host) = (identity(&package, "f"), identity(&package, "host"));
    let mut registry = Registry::new(Closure::new(&package));
    let transitions = register_request(&mut registry, host, f_key, vec![int(2)]);
    let cancel = Cancel::new();
    let mut meter = Meter::new(UNLIMITED).with_cancel(cancel.clone());
    let session = registry
        .begin_call(host, vec![int(1)], &objects, &mut meter)
        .unwrap();
    let Progress::Runnable(session) = session.step() else {
        panic!("entering the host body is not terminal");
    };
    let Progress::Runnable(session) = session.step() else {
        panic!("the host body's request is saved, not terminal");
    };
    cancel.cancel(CancelCause::Requested);
    let Progress::Terminal(Ok(denied)) = session.step() else {
        panic!("expected the located incomplete result at the child's call charge");
    };
    assert!(matches!(
        &denied.outcome,
        FamilyOutcome::Evaluated(Outcome::Incomplete(record))
            if record.charge_point == ChargePoint::FunctionCall
                && record.limit == record.consumed
    ));
    assert_eq!(denied.location, Some(call_site()));
    assert_eq!(cancel.tripped(), Some(CancelCause::Requested));
    assert_eq!(work(&meter), 1);
    assert_eq!(transitions.load(Ordering::SeqCst), 1);
}
