// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-401 (FR-262): the std reference adapter. A `quire-eval` registry over a
//! checked package admits exactly the package's callables by their minted
//! identity and signature, and a session over it runs interpreted and
//! registered host bodies through one scheduler with the results
//! `CheckedPackage::call` gives.

use qsl_eval::value::{CheckedPackageEvaluation, PackageClosure, QualifiedName};
use qsl_forms::{Expression, FunctionDeclaration};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedExpression, PackageDeclarations};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::{
    Admission, Body, BodyFactory, CallFailure, Evaluation, FamilyOutcome, Registry, Resume,
    Signature, Transition,
};
use quire_exact::{Integer, Meter, NodeKey, Outcome, ScalarLimits, Value, ValueType};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::checking::CheckingLimits;
use sha2::{Digest, Sha256};

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

fn identity_function() -> FunctionDeclaration {
    FunctionDeclaration::new(
        "keep",
        vec![(
            "a".to_owned(),
            crate::support::type_form::type_form(&ValueType::Integer),
        )],
        crate::support::type_form::type_form(&ValueType::Integer),
        None,
        Expression::name("a".to_owned()),
    )
}

fn package() -> CheckedPackage {
    let declarations = PackageDeclarations {
        functions: vec![identity_function()],
        ..PackageDeclarations::new(
            qsl_semantics::check::fixture_source(),
            qsl_foundation::IdentityLimits::default(),
        )
    };
    CheckedPackage::link(declarations.check(CheckingLimits::default()).unwrap())
}

fn keep(package: &CheckedPackage) -> NodeKey {
    package.graph().callable("keep").expect("declared").identity
}

fn int(n: i64) -> Value {
    Value::Integer(Integer::from(n))
}

fn completed_int(evaluation: &Evaluation) -> Option<i64> {
    match &evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(n))) => {
            n.to_u64().and_then(|n| i64::try_from(n).ok())
        }
        _ => None,
    }
}

struct ConstantBody(i64);

impl BodyFactory for ConstantBody {
    fn begin(&self) -> Box<dyn Body> {
        Box::new(Constant(self.0))
    }
}

struct Constant(i64);

impl Body for Constant {
    fn resume(&mut self, _input: Resume, _meter: &mut Meter) -> Result<Transition, InternalFault> {
        Ok(Transition::Done(Evaluation::completed(int(self.0))))
    }
}

/// FR-401-AC-5: registration decides from the package's admitted signature
/// alone.
#[test]
fn registration_is_decided_from_the_packages_admitted_signature() {
    let package = package();
    let mut registry = Registry::new(Closure::new(&package));
    let admitted = Signature::new(vec![ValueType::Integer], ValueType::Integer);
    let wrong = Signature::new(vec![ValueType::Boolean], ValueType::Integer);
    let unknown = NodeKey::from_digest(Sha256::digest(b"no function has this key").into());

    assert_eq!(
        registry.register(unknown, &admitted, ConstantBody(0)),
        Admission::CallableNotAdmitted
    );
    assert_eq!(
        registry.register(keep(&package), &wrong, ConstantBody(0)),
        Admission::SignatureMismatch
    );
    assert_eq!(
        registry.register(keep(&package), &admitted, ConstantBody(0)),
        Admission::Accepted
    );
    assert_eq!(
        registry.register(keep(&package), &admitted, ConstantBody(0)),
        Admission::DuplicateBinding
    );
}

/// A session over the adapter gives `CheckedPackage::call`'s result and
/// charges the same `function.call`.
#[test]
fn a_session_over_the_adapter_matches_call() {
    let package = package();
    let objects = ObjectEnvironment::default();
    let mut called = Meter::new(UNLIMITED);
    let by_call = package
        .call(
            &QualifiedName::unqualified("keep").unwrap(),
            vec![int(5)],
            &objects,
            &mut called,
        )
        .unwrap();

    let mut registry = Registry::new(Closure::new(&package));
    let mut driven = Meter::new(UNLIMITED);
    let by_session = registry
        .begin_call(keep(&package), vec![int(5)], &objects, &mut driven)
        .unwrap()
        .run()
        .unwrap();

    assert_eq!(completed_int(&by_call), Some(5));
    assert_eq!(completed_int(&by_session), Some(5));
    assert_eq!(
        called.consumed(quire_exact::LimitKind::WorkUnits),
        driven.consumed(quire_exact::LimitKind::WorkUnits)
    );
}

/// A registered host body runs in place of the interpreted body, under one
/// `function.call`, once `Begin` has sealed the registry.
#[test]
fn a_registered_host_body_runs_for_its_callable() {
    let package = package();
    let objects = ObjectEnvironment::default();
    let mut registry = Registry::new(Closure::new(&package));
    let admitted = Signature::new(vec![ValueType::Integer], ValueType::Integer);
    assert_eq!(
        registry.register(keep(&package), &admitted, ConstantBody(42)),
        Admission::Accepted
    );
    let mut meter = Meter::new(UNLIMITED);
    let result = registry
        .begin_call(keep(&package), vec![int(5)], &objects, &mut meter)
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(completed_int(&result), Some(42));
    assert_eq!(meter.consumed(quire_exact::LimitKind::WorkUnits), 1);
    assert!(registry.is_sealed());
}

/// `Begin` over a closure that admits its own arguments refuses a wrong
/// argument count before any charge, and leaves the registry open.
#[test]
fn begin_refuses_a_wrong_argument_count_before_any_charge() {
    let package = package();
    let objects = ObjectEnvironment::default();
    let mut registry = Registry::new(Closure::new(&package));
    let mut meter = Meter::new(UNLIMITED);
    let refused = registry.begin_call(keep(&package), vec![], &objects, &mut meter);
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
